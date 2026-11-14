//! Spike (M6, `docs/architecture/mic-chain.md` §6): records the **same take** twice — raw from
//! the microphone device and through Apple's Voice Processing I/O unit — so it can be compared
//! blind against our RNNoise chain (`ssdsp --mic` on the raw file).
//!
//! ```text
//! cargo run -p ss-audio-macos --example vpio_spike -- <out-dir> [seconds] [input-substring] [--duck-min]
//! ```
//!
//! Throw-away measurement code: kept small, prints everything the design note asks for.

// FFI adapter crate (AGENTS.md): unsafe allowed; every block has a SAFETY note.
#![allow(unsafe_code)]

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("macOS only");
}

#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    spike::run()
}

#[cfg(target_os = "macos")]
mod spike {
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::path::PathBuf;
    use std::ptr::NonNull;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    use anyhow::{Context, Result, bail};
    use block2::RcBlock;
    use dispatch2::DispatchQueue;
    use objc2_audio_toolbox::{
        AURenderCallbackStruct, AUVoiceIOOtherAudioDuckingConfiguration,
        AUVoiceIOOtherAudioDuckingLevel, AudioComponentDescription, AudioComponentFindNext,
        AudioComponentInstanceDispose, AudioComponentInstanceNew, AudioOutputUnitStart,
        AudioOutputUnitStop, AudioUnit, AudioUnitGetProperty, AudioUnitInitialize, AudioUnitRender,
        AudioUnitRenderActionFlags, AudioUnitSetProperty,
        kAUVoiceIOProperty_OtherAudioDuckingConfiguration, kAudioOutputUnitProperty_CurrentDevice,
        kAudioOutputUnitProperty_EnableIO, kAudioOutputUnitProperty_SetInputCallback,
        kAudioUnitManufacturer_Apple, kAudioUnitProperty_Latency, kAudioUnitProperty_StreamFormat,
        kAudioUnitScope_Global, kAudioUnitScope_Input, kAudioUnitScope_Output,
        kAudioUnitSubType_VoiceProcessingIO, kAudioUnitType_Output,
    };
    use objc2_core_audio::{
        AudioDeviceCreateIOProcIDWithBlock, AudioDeviceDestroyIOProcID, AudioDeviceIOProcID,
        AudioDeviceStart, AudioDeviceStop, AudioObjectID,
    };
    use objc2_core_audio_types::{
        AudioBuffer, AudioBufferList, AudioStreamBasicDescription, AudioTimeStamp,
        kAudioFormatFlagIsFloat, kAudioFormatFlagIsNonInterleaved, kAudioFormatFlagIsPacked,
        kAudioFormatLinearPCM,
    };
    use rtrb::{Consumer, Producer, RingBuffer};
    use ss_audio_macos::list_input_devices;

    const SAMPLE_RATE: f64 = 48_000.0;
    const RING: usize = 48_000 * 4;
    const MAX_FRAMES: usize = 4096;
    /// `AudioUnit` input element (bus 1 = microphone side of an I/O unit).
    const INPUT_BUS: u32 = 1;
    /// `AudioUnit` output element (bus 0 = speaker side).
    const OUTPUT_BUS: u32 = 0;

    struct Capture {
        producer: RefCell<Producer<f32>>,
        first_host_time: AtomicU64,
    }

    impl Capture {
        fn new(producer: Producer<f32>) -> Rc<Self> {
            Rc::new(Self {
                producer: RefCell::new(producer),
                first_host_time: AtomicU64::new(0),
            })
        }

        /// Real-time safe: pushes channel 0 of interleaved `samples`.
        fn push(&self, samples: &[f32], channels: usize, host_time: u64) {
            let _ = self.first_host_time.compare_exchange(
                0,
                host_time,
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
            if let Ok(mut producer) = self.producer.try_borrow_mut() {
                for frame in samples.chunks_exact(channels.max(1)) {
                    let _ = producer.push(frame[0]);
                }
            }
        }
    }

    struct VpioContext {
        unit: AudioUnit,
        capture: Rc<Capture>,
        scratch: RefCell<Vec<f32>>,
    }

    pub fn run() -> Result<()> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let out_dir = PathBuf::from(
            args.first()
                .context("usage: <out-dir> [seconds] [input] [--duck-min]")?,
        );
        let seconds: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);
        let input = args
            .get(2)
            .filter(|a| !a.starts_with("--"))
            .map_or("Arctis", String::as_str);
        let duck_min = args.iter().any(|a| a == "--duck-min");

        let device = list_input_devices()?
            .into_iter()
            .find(|d| d.name.to_lowercase().contains(&input.to_lowercase()))
            .with_context(|| format!("no input device matching {input}"))?;
        println!(
            "input device: {} (id {}, {} ch)",
            device.name, device.id, device.input_channels
        );

        let (raw_tx, raw_rx) = RingBuffer::<f32>::new(RING);
        let (vp_tx, vp_rx) = RingBuffer::<f32>::new(RING);
        let raw = Capture::new(raw_tx);
        let vp = Capture::new(vp_tx);

        let raw_proc = start_raw(device.id, Rc::clone(&raw))?;
        let vpio = start_vpio(device.id, Rc::clone(&vp), duck_min);

        let result = vpio.and_then(|(unit, context)| {
            let recorded = record(&out_dir, seconds, raw_rx, vp_rx, &raw, &vp);
            // SAFETY: unit created below, stopped and disposed exactly once.
            unsafe {
                AudioOutputUnitStop(unit);
                AudioComponentInstanceDispose(unit);
            }
            drop(context);
            recorded
        });
        // SAFETY: stops the IOProc created in `start_raw`.
        unsafe {
            AudioDeviceStop(device.id, raw_proc.0);
            AudioDeviceDestroyIOProcID(device.id, raw_proc.0);
        }
        drop(raw_proc);
        result
    }

    type RawProc = (
        AudioDeviceIOProcID,
        RcBlock<
            dyn Fn(
                NonNull<AudioTimeStamp>,
                NonNull<AudioBufferList>,
                NonNull<AudioTimeStamp>,
                NonNull<AudioBufferList>,
                NonNull<AudioTimeStamp>,
            ),
        >,
    );

    fn start_raw(device: AudioObjectID, capture: Rc<Capture>) -> Result<RawProc> {
        let block = RcBlock::new(
            move |_now: NonNull<AudioTimeStamp>,
                  input: NonNull<AudioBufferList>,
                  input_time: NonNull<AudioTimeStamp>,
                  _out: NonNull<AudioBufferList>,
                  _out_time: NonNull<AudioTimeStamp>| {
                // SAFETY: valid for the callback.
                let (list, time) = unsafe { (input.as_ref(), input_time.as_ref()) };
                if list.mNumberBuffers == 0 {
                    return;
                }
                let buffer = list.mBuffers[0];
                if buffer.mData.is_null() {
                    return;
                }
                // SAFETY: the buffer holds mDataByteSize bytes of f32 samples.
                let samples = unsafe {
                    std::slice::from_raw_parts(
                        buffer.mData.cast::<f32>(),
                        buffer.mDataByteSize as usize / 4,
                    )
                };
                capture.push(samples, buffer.mNumberChannels as usize, time.mHostTime);
            },
        );
        let queue = DispatchQueue::new("dev.stealseries.spike.raw", None);
        let mut proc_id: AudioDeviceIOProcID = None;
        // SAFETY: valid out-pointer, device, queue and block (block kept alive by the caller,
        // queue retained by CoreAudio for the IOProc's lifetime).
        let status = unsafe {
            AudioDeviceCreateIOProcIDWithBlock(
                NonNull::from(&mut proc_id),
                device,
                Some(&queue),
                RcBlock::as_ptr(&block).cast(),
            )
        };
        check(status, "raw IOProc")?;
        // SAFETY: created above.
        check(unsafe { AudioDeviceStart(device, proc_id) }, "raw start")?;
        Ok((proc_id, block))
    }

    unsafe extern "C-unwind" fn vpio_input(
        refcon: NonNull<c_void>,
        flags: NonNull<AudioUnitRenderActionFlags>,
        time: NonNull<AudioTimeStamp>,
        bus: u32,
        frames: u32,
        _data: *mut AudioBufferList,
    ) -> i32 {
        // SAFETY: refcon is the boxed VpioContext kept alive while the unit runs.
        let context = unsafe { refcon.cast::<VpioContext>().as_ref() };
        let Ok(mut scratch) = context.scratch.try_borrow_mut() else {
            return 0;
        };
        let frames_usize = (frames as usize).min(scratch.len());
        let mut list = AudioBufferList {
            mNumberBuffers: 1,
            mBuffers: [AudioBuffer {
                mNumberChannels: 1,
                mDataByteSize: u32::try_from(frames_usize * 4).unwrap_or(0),
                mData: scratch.as_mut_ptr().cast(),
            }],
        };
        // SAFETY: unit is valid; list points to our scratch buffer of `frames` samples.
        let status = unsafe {
            AudioUnitRender(
                context.unit,
                flags.as_ptr(),
                time,
                bus,
                frames,
                NonNull::from(&mut list),
            )
        };
        if status == 0 {
            // SAFETY: valid timestamp for the call.
            let host = unsafe { time.as_ref() }.mHostTime;
            context.capture.push(&scratch[..frames_usize], 1, host);
        }
        status
    }

    #[allow(clippy::too_many_lines)] // linear AudioUnit setup, spike code
    fn start_vpio(
        device: AudioObjectID,
        capture: Rc<Capture>,
        duck_min: bool,
    ) -> Result<(AudioUnit, Box<VpioContext>)> {
        let description = AudioComponentDescription {
            componentType: kAudioUnitType_Output,
            componentSubType: kAudioUnitSubType_VoiceProcessingIO,
            componentManufacturer: kAudioUnitManufacturer_Apple,
            componentFlags: 0,
            componentFlagsMask: 0,
        };
        // SAFETY: valid description pointer.
        let component =
            unsafe { AudioComponentFindNext(std::ptr::null_mut(), NonNull::from(&description)) };
        if component.is_null() {
            bail!("VoiceProcessingIO not available");
        }
        let mut unit: AudioUnit = std::ptr::null_mut();
        // SAFETY: valid component and out-pointer.
        check(
            unsafe { AudioComponentInstanceNew(component, NonNull::from(&mut unit)) },
            "instance",
        )?;

        let enable: u32 = 1;
        set(
            unit,
            kAudioOutputUnitProperty_EnableIO,
            kAudioUnitScope_Input,
            INPUT_BUS,
            &enable,
            "EnableIO input",
        )?;
        report(
            "set CurrentDevice (input bus)",
            set(
                unit,
                kAudioOutputUnitProperty_CurrentDevice,
                kAudioUnitScope_Global,
                INPUT_BUS,
                &device,
                "CurrentDevice",
            ),
        );

        let format = AudioStreamBasicDescription {
            mSampleRate: SAMPLE_RATE,
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kAudioFormatFlagIsFloat
                | kAudioFormatFlagIsPacked
                | kAudioFormatFlagIsNonInterleaved,
            mBytesPerPacket: 4,
            mFramesPerPacket: 1,
            mBytesPerFrame: 4,
            mChannelsPerFrame: 1,
            mBitsPerChannel: 32,
            mReserved: 0,
        };
        set(
            unit,
            kAudioUnitProperty_StreamFormat,
            kAudioUnitScope_Output,
            INPUT_BUS,
            &format,
            "input format",
        )?;
        report(
            "set speaker-side format",
            set(
                unit,
                kAudioUnitProperty_StreamFormat,
                kAudioUnitScope_Input,
                OUTPUT_BUS,
                &format,
                "output format",
            ),
        );

        if duck_min {
            let ducking = AUVoiceIOOtherAudioDuckingConfiguration {
                mEnableAdvancedDucking: 1,
                mDuckingLevel: AUVoiceIOOtherAudioDuckingLevel::Min,
            };
            report(
                "set ducking = advanced/min",
                set(
                    unit,
                    kAUVoiceIOProperty_OtherAudioDuckingConfiguration,
                    kAudioUnitScope_Global,
                    0,
                    &ducking,
                    "ducking",
                ),
            );
        }

        let context = Box::new(VpioContext {
            unit,
            capture,
            scratch: RefCell::new(vec![0.0; MAX_FRAMES]),
        });
        let callback = AURenderCallbackStruct {
            inputProc: Some(vpio_input),
            inputProcRefCon: std::ptr::from_ref::<VpioContext>(&context)
                .cast_mut()
                .cast(),
        };
        set(
            unit,
            kAudioOutputUnitProperty_SetInputCallback,
            kAudioUnitScope_Global,
            0,
            &callback,
            "input callback",
        )?;
        // SAFETY: configured unit.
        check(unsafe { AudioUnitInitialize(unit) }, "initialize")?;

        let mut current: AudioObjectID = 0;
        let got = get(
            unit,
            kAudioOutputUnitProperty_CurrentDevice,
            kAudioUnitScope_Global,
            INPUT_BUS,
            &mut current,
        );
        println!("VPIO input device after init: {current} (status {got}; wanted {device})");
        let mut latency = 0.0_f64;
        let got = get(
            unit,
            kAudioUnitProperty_Latency,
            kAudioUnitScope_Global,
            0,
            &mut latency,
        );
        println!(
            "VPIO reported latency: {:.1} ms (status {got})",
            latency * 1000.0
        );

        // SAFETY: initialised unit.
        check(unsafe { AudioOutputUnitStart(unit) }, "start")?;
        println!(
            "VPIO running{}",
            if duck_min {
                " (ducking min)"
            } else {
                " (default ducking)"
            }
        );
        Ok((unit, context))
    }

    fn record(
        dir: &PathBuf,
        seconds: u64,
        mut raw_rx: Consumer<f32>,
        mut vp_rx: Consumer<f32>,
        raw: &Capture,
        vp: &Capture,
    ) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut raw_wav = hound::WavWriter::create(dir.join("raw.wav"), spec)?;
        let mut vp_wav = hound::WavWriter::create(dir.join("vpio.wav"), spec)?;
        let start = Instant::now();
        let mut next_report = Duration::from_secs(1);
        let (mut raw_peak, mut vp_peak) = (0.0_f32, 0.0_f32);
        while start.elapsed() < Duration::from_secs(seconds) {
            std::thread::sleep(Duration::from_millis(50));
            while let Ok(s) = raw_rx.pop() {
                raw_peak = raw_peak.max(s.abs());
                raw_wav.write_sample(s)?;
            }
            while let Ok(s) = vp_rx.pop() {
                vp_peak = vp_peak.max(s.abs());
                vp_wav.write_sample(s)?;
            }
            if start.elapsed() >= next_report {
                println!(
                    "t={:>2}s raw {:>6.1} dBFS | vpio {:>6.1} dBFS",
                    next_report.as_secs(),
                    db(raw_peak),
                    db(vp_peak)
                );
                (raw_peak, vp_peak) = (0.0, 0.0);
                next_report += Duration::from_secs(1);
            }
        }
        raw_wav.finalize()?;
        vp_wav.finalize()?;
        let (r, v) = (
            raw.first_host_time.load(Ordering::Relaxed),
            vp.first_host_time.load(Ordering::Relaxed),
        );
        println!(
            "first host time raw {r}, vpio {v} (vpio - raw = {} ticks)",
            i128::from(v) - i128::from(r)
        );
        println!("wrote {}/raw.wav and vpio.wav", dir.display());
        Ok(())
    }

    fn db(peak: f32) -> f32 {
        if peak > 0.0 {
            20.0 * peak.log10()
        } else {
            -120.0
        }
    }

    fn set<T>(
        unit: AudioUnit,
        id: u32,
        scope: u32,
        element: u32,
        value: &T,
        what: &str,
    ) -> Result<()> {
        // SAFETY: `value` is a valid T of the stated size.
        let status = unsafe {
            AudioUnitSetProperty(
                unit,
                id,
                scope,
                element,
                std::ptr::from_ref(value).cast(),
                u32::try_from(size_of::<T>()).unwrap_or(0),
            )
        };
        check(status, what)
    }

    fn get<T>(unit: AudioUnit, id: u32, scope: u32, element: u32, out: &mut T) -> i32 {
        let mut size = u32::try_from(size_of::<T>()).unwrap_or(0);
        // SAFETY: `out` provides `size` writable bytes.
        unsafe {
            AudioUnitGetProperty(
                unit,
                id,
                scope,
                element,
                NonNull::from(out).cast(),
                NonNull::from(&mut size),
            )
        }
    }

    fn report(what: &str, result: Result<()>) {
        match result {
            Ok(()) => println!("{what}: ok"),
            Err(e) => println!("{what}: {e}"),
        }
    }

    fn check(status: i32, what: &str) -> Result<()> {
        if status == 0 {
            Ok(())
        } else {
            let code = status.to_be_bytes();
            bail!(
                "{what} failed: {status} ('{}')",
                String::from_utf8_lossy(&code)
            )
        }
    }
}

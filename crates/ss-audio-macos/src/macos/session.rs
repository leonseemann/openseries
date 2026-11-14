//! A running AudioMix session: taps → aggregate device → IOProc → [`MixEngine`].
//!
//! **Inferred (to verify on hardware):** the aggregate device delivers the main sub-device's
//! own input streams first, followed by one stream per tap in tap-list order. The headset's
//! input buffer count is therefore skipped before mapping buffers to channels.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::sync::Arc;

use block2::RcBlock;
use dispatch2::{DispatchQueue, DispatchRetained};
use objc2_core_audio::{
    AudioDeviceCreateIOProcIDWithBlock, AudioDeviceDestroyIOProcID, AudioDeviceIOProcID,
    AudioDeviceStart, AudioDeviceStop, AudioObjectID, AudioObjectPropertyScope,
    kAudioDevicePropertyStreamConfiguration, kAudioObjectPropertyScopeInput,
    kAudioObjectPropertyScopeOutput,
};
use objc2_core_audio_types::{AudioBuffer, AudioBufferList, AudioTimeStamp};
use ss_core::mixer::ChannelId;
use ss_engine::{ChannelInput, EngineController, MixEngine};

use super::aggregate::AggregateDevice;
use super::buffers;
use super::device::OutputDevice;
use super::probe::{CallbackLayout, Probe, ProbeSnapshot};
use super::property;
use super::tap::ProcessTap;
use crate::AudioError;
use crate::plan::TapSpec;

/// Largest callback the engine is prepared for; larger callbacks are truncated (silence).
const MAX_CALLBACK_FRAMES: usize = 4096;
/// Most taps a session can have (one per mixer channel, see `plan_taps`).
const MAX_TAPS: usize = ChannelId::COUNT;
/// Label of the dispatch queue running the IOProc.
const QUEUE_LABEL: &str = "dev.stealseries.audiomix.io";

type IoBlock = RcBlock<
    dyn Fn(
        NonNull<AudioTimeStamp>,
        NonNull<AudioBufferList>,
        NonNull<AudioTimeStamp>,
        NonNull<AudioBufferList>,
        NonNull<AudioTimeStamp>,
    ),
>;

/// Which aggregate input buffer feeds which mixer channel.
#[derive(Debug, Clone, Copy)]
struct InputLayout {
    /// Leading buffers belonging to the output device's own inputs.
    skip: usize,
    channels: [ChannelId; MAX_TAPS],
    count: usize,
}

/// Static stream configuration of the running aggregate device (channels per buffer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamLayout {
    /// Input buffers the output device itself contributes (skipped before the taps).
    pub device_inputs: usize,
    /// Channels per aggregate input buffer.
    pub inputs: Vec<usize>,
    /// Channels per aggregate output buffer.
    pub outputs: Vec<usize>,
}

/// A running mix. Dropping it stops audio and removes the taps (apps become audible again).
pub struct AudioMixSession {
    // Field order = drop order after `Drop::drop`: aggregate device before its taps.
    aggregate: AggregateDevice,
    taps: Vec<ProcessTap>,
    proc_id: AudioDeviceIOProcID,
    layout: InputLayout,
    probe: Arc<Probe>,
    _block: IoBlock,
    _queue: DispatchRetained<DispatchQueue>,
}

impl AudioMixSession {
    /// Creates the taps and the aggregate device and starts mixing onto `output`.
    ///
    /// Returns the session and the controller for changing mixer settings while running.
    ///
    /// # Errors
    /// [`AudioError`] if a tap, the aggregate device or the IOProc cannot be created
    /// (commonly: the app lacks the audio-capture permission).
    pub fn start(
        output: &OutputDevice,
        specs: &[TapSpec],
    ) -> Result<(Self, EngineController), AudioError> {
        let layout = input_layout(output, specs)?;
        let taps = specs
            .iter()
            .map(ProcessTap::create)
            .collect::<Result<Vec<_>, _>>()?;
        let aggregate = AggregateDevice::create(output, &taps)?;

        #[allow(clippy::cast_possible_truncation)] // sample rates fit easily into f32
        let sample_rate = output.sample_rate()? as f32;
        let (engine, controller) =
            ss_engine::create(sample_rate, output.output_channels, MAX_CALLBACK_FRAMES)
                .map_err(|e| AudioError::Unsupported(e.to_string()))?;

        let engine = RefCell::new(engine);
        let probe = Arc::new(Probe::default());
        let rt_probe = Arc::clone(&probe);
        let block: IoBlock = RcBlock::new(move |_now, input, _input_time, output, _output_time| {
            render(&engine, &layout, &rt_probe, input, output);
        });
        let queue = DispatchQueue::new(QUEUE_LABEL, None);

        let mut proc_id: AudioDeviceIOProcID = None;
        // SAFETY: valid out-pointer, live device, queue and block (both kept in `Self`).
        let status = unsafe {
            AudioDeviceCreateIOProcIDWithBlock(
                NonNull::from(&mut proc_id),
                aggregate.id(),
                Some(&queue),
                RcBlock::as_ptr(&block),
            )
        };
        AudioError::check(status, "AudioDeviceCreateIOProcIDWithBlock")?;

        let session = Self {
            aggregate,
            taps,
            proc_id,
            layout,
            probe,
            _block: block,
            _queue: queue,
        };
        // SAFETY: device and IOProc id were created above.
        let status = unsafe { AudioDeviceStart(session.aggregate.id(), session.proc_id) };
        AudioError::check(status, "AudioDeviceStart")?;
        Ok((session, controller))
    }

    /// Number of active taps.
    #[must_use]
    pub fn tap_count(&self) -> usize {
        self.taps.len()
    }

    /// Mixer channel fed by each tap, in tap-list order.
    #[must_use]
    pub fn tap_channels(&self) -> &[ChannelId] {
        &self.layout.channels[..self.layout.count]
    }

    /// Stream configuration of the aggregate device as reported by CoreAudio.
    ///
    /// # Errors
    /// [`AudioError`] if a stream configuration cannot be read.
    pub fn stream_layout(&self) -> Result<StreamLayout, AudioError> {
        Ok(StreamLayout {
            device_inputs: self.layout.skip,
            inputs: buffer_channels(self.aggregate.id(), kAudioObjectPropertyScopeInput)?,
            outputs: buffer_channels(self.aggregate.id(), kAudioObjectPropertyScopeOutput)?,
        })
    }

    /// Measurements of the IOProc since the previous call (peaks are reset).
    #[must_use]
    pub fn probe(&self) -> ProbeSnapshot {
        self.probe.take()
    }
}

impl Drop for AudioMixSession {
    fn drop(&mut self) {
        // SAFETY: stopping/destroying the IOProc we created; errors are ignored in drop.
        unsafe {
            let _ = AudioDeviceStop(self.aggregate.id(), self.proc_id);
            let _ = AudioDeviceDestroyIOProcID(self.aggregate.id(), self.proc_id);
        }
    }
}

fn input_layout(output: &OutputDevice, specs: &[TapSpec]) -> Result<InputLayout, AudioError> {
    if specs.len() > MAX_TAPS {
        return Err(AudioError::Unsupported(format!(
            "{} taps requested, at most {MAX_TAPS} supported",
            specs.len()
        )));
    }
    let mut channels = [ChannelId::Media; MAX_TAPS];
    for (slot, spec) in channels.iter_mut().zip(specs) {
        *slot = spec.channel;
    }
    Ok(InputLayout {
        skip: buffer_channels(output.id, kAudioObjectPropertyScopeInput)?.len(),
        channels,
        count: specs.len(),
    })
}

/// Channels of every stream (buffer) of `device` in `scope`.
fn buffer_channels(
    device: AudioObjectID,
    scope: AudioObjectPropertyScope,
) -> Result<Vec<usize>, AudioError> {
    let raw = property::get_raw(device, kAudioDevicePropertyStreamConfiguration, scope)?;
    let Some(list) = NonNull::new(raw.as_ptr().cast_mut().cast::<AudioBufferList>()) else {
        return Ok(Vec::new());
    };
    // SAFETY: `raw` holds the complete AudioBufferList and outlives this call.
    let streams = unsafe { buffers::buffers(list) };
    Ok(streams.iter().map(|b| b.mNumberChannels as usize).collect())
}

/// The IOProc body. Runs on the real-time thread: no allocation, no locks, no logging.
fn render(
    engine: &RefCell<MixEngine>,
    layout: &InputLayout,
    probe: &Probe,
    input: NonNull<AudioBufferList>,
    output: NonNull<AudioBufferList>,
) {
    // SAFETY: CoreAudio passes valid buffer lists that live for the duration of the callback.
    let (inputs, outputs) = unsafe { (buffers::buffers(input), buffers::buffers(output)) };
    record_probe(probe, inputs, outputs);
    let Some((main, rest)) = outputs.split_first() else {
        return;
    };
    for extra in rest {
        // SAFETY: output buffers are exclusively ours during the callback.
        unsafe { buffers::samples_mut(extra) }.fill(0.0);
    }
    // SAFETY: as above.
    let out = unsafe { buffers::samples_mut(main) };

    let Ok(mut engine) = engine.try_borrow_mut() else {
        out.fill(0.0);
        return;
    };
    if main.mNumberChannels as usize != engine.format().channels() {
        out.fill(0.0); // non-interleaved / changed layout: unsupported in v1
        return;
    }

    let mut channel_inputs = [ChannelInput {
        channel: ChannelId::Media,
        samples: &[],
        channels: 0,
    }; MAX_TAPS];
    let mut count = 0;
    let tap_buffers = inputs.iter().skip(layout.skip);
    for (buffer, channel) in tap_buffers.zip(&layout.channels[..layout.count]) {
        channel_inputs[count] = ChannelInput {
            channel: *channel,
            // SAFETY: input buffers are valid and read-only for the callback.
            samples: unsafe { buffers::samples(buffer) },
            channels: buffer.mNumberChannels as usize,
        };
        count += 1;
    }
    engine.process(&channel_inputs[..count], out);
}

/// Feeds the probe with this callback's buffer shapes and input peaks. Real-time safe.
fn record_probe(probe: &Probe, inputs: &[AudioBuffer], outputs: &[AudioBuffer]) {
    let output_channels = outputs.first().map_or(0, |b| b.mNumberChannels as usize);
    let frames = outputs.first().map_or(0, |b| {
        (b.mDataByteSize as usize / size_of::<f32>()) / output_channels.max(1)
    });
    probe.record_layout(CallbackLayout {
        input_buffers: inputs.len(),
        output_buffers: outputs.len(),
        output_channels,
        frames,
    });
    for (index, buffer) in inputs.iter().enumerate() {
        // SAFETY: input buffers are valid and read-only for the callback.
        let samples = unsafe { buffers::samples(buffer) };
        probe.record_input(index, buffer.mNumberChannels as usize, samples);
    }
}

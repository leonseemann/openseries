//! Feeds a physical microphone into the virtual "StealSeries Microphone"
//! (`docs/architecture/virtual-mic-driver.md` §7).
//!
//! A private aggregate device combines the microphone's device (main, clock) with the hidden
//! feed device of `ss-hal-driver-macos` (drift compensated). One IOProc copies the mic into the
//! feed; the mic chain (M6) will run at that point later.
//!
//! **Observed** (`docs/hardware/macos-virtual-mic.md`, check 5): output buffers appear in
//! sub-device order, i.e. the mic device's own output streams (if any) before the feed stream.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::sync::Arc;

use block2::RcBlock;
use dispatch2::{DispatchQueue, DispatchRetained};
use objc2_core_audio::{
    AudioDeviceCreateIOProcIDWithBlock, AudioDeviceDestroyIOProcID, AudioDeviceIOProcID,
    AudioDeviceStart, AudioDeviceStop, kAudioObjectPropertyScopeInput,
    kAudioObjectPropertyScopeOutput,
};
use objc2_core_audio_types::{AudioBuffer, AudioBufferList};

use super::aggregate::{AggregateDevice, SubDevice};
use super::buffers;
use super::device::{InputDevice, device_id_for_uid};
use super::probe::{CallbackLayout, Probe, ProbeSnapshot};
use super::session::{IoBlock, StreamLayout, buffer_channels};
use super::stream_usage::set_stream_usage;
use crate::AudioError;
use ss_dsp::{MicChain, MicChainSettings, Processor};

/// UID of the hidden feed device published by `ss-hal-driver-macos`
/// (`virtual-mic-driver.md` §2; checked against the driver's model in a test).
pub const FEED_DEVICE_UID: &str = "dev.stealseries.mic.feed";
/// UID of the visible virtual microphone (must never be used as the source).
pub const VIRTUAL_MIC_UID: &str = "dev.stealseries.mic";
/// Label of the dispatch queue running the IOProc.
const QUEUE_LABEL: &str = "dev.stealseries.micfeed.io";
/// Pending live chain changes; only the newest matters, a few slots absorb slider drags.
const SETTINGS_QUEUE: usize = 8;

/// Largest callback the mic chain is prepared for; larger callbacks are truncated (silence).
const MAX_CALLBACK_FRAMES: usize = 4096;

/// Where the mic and the feed sit in the aggregate's buffer lists.
#[derive(Debug, Clone, Copy)]
struct FeedLayout {
    /// Output buffers of the mic device itself, before the feed buffer.
    mic_device_outputs: usize,
}

/// State owned by the IOProc: the mic chain and a mono scratch buffer (allocated up front).
struct FeedProcessor {
    chain: MicChain,
    mono: Box<[f32]>,
    /// Live chain changes from [`MicFeedSession::set_chain`].
    settings: rtrb::Consumer<MicChainSettings>,
    /// Per-thread state primed (first callback; see [`MicChain::warm_up`]).
    warmed: bool,
}

/// A running mic → virtual-mic feed. Dropping it stops the feed (the virtual mic goes silent).
pub struct MicFeedSession {
    // Field order = drop order after `Drop::drop`.
    aggregate: AggregateDevice,
    proc_id: AudioDeviceIOProcID,
    layout: FeedLayout,
    probe: Arc<Probe>,
    settings: rtrb::Producer<MicChainSettings>,
    _block: IoBlock,
    _queue: DispatchRetained<DispatchQueue>,
}

impl MicFeedSession {
    /// Changes the mic chain while running (applied at the next callback, no restart).
    /// Returns `false` if the queue is full (call again later).
    pub fn set_chain(&mut self, settings: MicChainSettings) -> bool {
        self.settings.push(settings).is_ok()
    }

    /// Starts feeding `mic` (channels mixed to mono, then processed by `chain`) into the
    /// virtual microphone.
    ///
    /// # Errors
    /// [`AudioError::NotFound`] if the driver is not installed, [`AudioError::Unsupported`] if
    /// `mic` is the virtual microphone itself, [`AudioError::Os`] for CoreAudio failures.
    pub fn start(mic: &InputDevice, chain: MicChain) -> Result<Self, AudioError> {
        if mic.uid == VIRTUAL_MIC_UID {
            return Err(AudioError::Unsupported(
                "the virtual microphone cannot feed itself".to_owned(),
            ));
        }
        device_id_for_uid(FEED_DEVICE_UID).map_err(|_| {
            AudioError::NotFound(
                "StealSeries Microphone driver (run scripts/macos/hal-driver.sh install)"
                    .to_owned(),
            )
        })?;
        let layout = FeedLayout {
            mic_device_outputs: buffer_channels(mic.id, kAudioObjectPropertyScopeOutput)?.len(),
        };
        let (settings_tx, settings_rx) = rtrb::RingBuffer::new(SETTINGS_QUEUE);
        let processor = RefCell::new(FeedProcessor {
            chain,
            mono: vec![0.0; MAX_CALLBACK_FRAMES].into_boxed_slice(),
            settings: settings_rx,
            warmed: false,
        });
        let aggregate = AggregateDevice::create_with(
            &[
                SubDevice {
                    uid: &mic.uid,
                    drift_compensation: false,
                },
                SubDevice {
                    uid: FEED_DEVICE_UID,
                    drift_compensation: true,
                },
            ],
            &[],
        )?;

        let probe = Arc::new(Probe::default());
        let rt_probe = Arc::clone(&probe);
        let block: IoBlock = RcBlock::new(move |_now, input, _input_time, output, _output_time| {
            render(layout, &processor, &rt_probe, input, output);
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
            proc_id,
            layout,
            probe,
            settings: settings_tx,
            _block: block,
            _queue: queue,
        };
        session.close_mic_device_outputs()?;
        // SAFETY: device and IOProc id were created above.
        let status = unsafe { AudioDeviceStart(session.aggregate.id(), session.proc_id) };
        AudioError::check(status, "AudioDeviceStart")?;
        Ok(session)
    }

    /// Switches the mic device's own outputs (e.g. the headset speakers) off for our IOProc so
    /// the feed never touches playback.
    fn close_mic_device_outputs(&self) -> Result<(), AudioError> {
        let skip = self.layout.mic_device_outputs;
        if skip == 0 {
            return Ok(());
        }
        let streams = buffer_channels(self.aggregate.id(), kAudioObjectPropertyScopeOutput)?.len();
        set_stream_usage(
            self.aggregate.id(),
            self.proc_id,
            kAudioObjectPropertyScopeOutput,
            streams,
            |index| index >= skip,
        )
    }

    /// Stream configuration of the aggregate device. `device_inputs` is 0 here: every input
    /// buffer belongs to the mic.
    ///
    /// # Errors
    /// [`AudioError`] if a stream configuration cannot be read.
    pub fn stream_layout(&self) -> Result<StreamLayout, AudioError> {
        Ok(StreamLayout {
            device_inputs: 0,
            inputs: buffer_channels(self.aggregate.id(), kAudioObjectPropertyScopeInput)?,
            outputs: buffer_channels(self.aggregate.id(), kAudioObjectPropertyScopeOutput)?,
        })
    }

    /// Index of the output buffer that feeds the virtual microphone.
    #[must_use]
    pub fn feed_buffer_index(&self) -> usize {
        self.layout.mic_device_outputs
    }

    /// Measurements of the IOProc since the previous call (peaks are reset). Input peaks are
    /// the mic buffers; the feed's output peak is reported as the slot after the last input.
    #[must_use]
    pub fn probe(&self) -> ProbeSnapshot {
        self.probe.take()
    }
}

impl Drop for MicFeedSession {
    fn drop(&mut self) {
        // SAFETY: stopping/destroying the IOProc we created; errors are ignored in drop.
        unsafe {
            let _ = AudioDeviceStop(self.aggregate.id(), self.proc_id);
            let _ = AudioDeviceDestroyIOProcID(self.aggregate.id(), self.proc_id);
        }
    }
}

/// The IOProc body. Runs on the real-time thread: no allocation, no locks, no logging.
fn render(
    layout: FeedLayout,
    processor: &RefCell<FeedProcessor>,
    probe: &Probe,
    input: NonNull<AudioBufferList>,
    output: NonNull<AudioBufferList>,
) {
    // SAFETY: CoreAudio passes valid buffer lists that live for the duration of the callback.
    let (inputs, outputs) = unsafe { (buffers::buffers(input), buffers::buffers(output)) };
    for buffer in outputs {
        // SAFETY: output buffers are exclusively ours during the callback.
        unsafe { buffers::samples_mut(buffer) }.fill(0.0);
    }
    let (Some(mic), Some(feed)) = (inputs.first(), outputs.get(layout.mic_device_outputs)) else {
        return;
    };
    // SAFETY: as above; input buffers are read-only, the feed buffer is ours.
    let (mic_samples, feed_samples) =
        unsafe { (buffers::samples(mic), buffers::samples_mut(feed)) };
    let Ok(mut processor) = processor.try_borrow_mut() else {
        return;
    };
    let FeedProcessor {
        chain,
        mono,
        settings,
        warmed,
    } = &mut *processor;
    if !*warmed {
        // One-time allocation of thread-local FFT plans, before any live change (mic-chain.md §3).
        chain.warm_up();
        *warmed = true;
    }
    let mut latest = None;
    while let Ok(next) = settings.pop() {
        latest = Some(next);
    }
    if let Some(next) = latest {
        chain.set(&next);
    }
    let (mic_channels, feed_channels) = (channels(mic), channels(feed));
    let frames = (mic_samples.len() / mic_channels)
        .min(feed_samples.len() / feed_channels)
        .min(mono.len());
    let mono = &mut mono[..frames];
    mix_to_mono(mic_samples, mic_channels, mono);
    chain.process(mono);
    spread_mono(mono, feed_samples, feed_channels);

    record_probe(probe, inputs, outputs, feed_samples);
}

fn channels(buffer: &AudioBuffer) -> usize {
    (buffer.mNumberChannels as usize).max(1)
}

/// Averages interleaved `mic` frames into `mono` (one sample per frame). Real-time safe.
fn mix_to_mono(mic: &[f32], mic_channels: usize, mono: &mut [f32]) {
    #[allow(clippy::cast_precision_loss)] // channel counts are tiny
    let scale = 1.0 / mic_channels as f32;
    for (frame, out) in mic.chunks_exact(mic_channels).zip(mono.iter_mut()) {
        *out = frame.iter().sum::<f32>() * scale;
    }
}

/// Writes each mono sample to every channel of the interleaved `feed`. Real-time safe.
fn spread_mono(mono: &[f32], feed: &mut [f32], feed_channels: usize) {
    for (sample, frame) in mono.iter().zip(feed.chunks_exact_mut(feed_channels)) {
        frame.fill(*sample);
    }
}

fn record_probe(probe: &Probe, inputs: &[AudioBuffer], outputs: &[AudioBuffer], feed: &[f32]) {
    let output_channels = outputs.first().map_or(0, |b| b.mNumberChannels as usize);
    probe.record_layout(CallbackLayout {
        input_buffers: inputs.len() + 1,
        output_buffers: outputs.len(),
        output_channels,
        frames: feed.len(),
    });
    for (index, buffer) in inputs.iter().enumerate() {
        // SAFETY: input buffers are valid and read-only for the callback.
        let samples = unsafe { buffers::samples(buffer) };
        probe.record_input(index, buffer.mNumberChannels as usize, samples);
    }
    probe.record_input(inputs.len(), 1, feed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_mic_is_averaged_to_mono() {
        let mic = [0.2, 0.4, -0.2, -0.4];
        let mut mono = [9.0; 2];
        mix_to_mono(&mic, 2, &mut mono);
        assert!((mono[0] - 0.3).abs() < 1e-6);
        assert!((mono[1] + 0.3).abs() < 1e-6);
    }

    #[test]
    fn mono_mic_fills_every_feed_channel() {
        let mono = [0.5, 0.25];
        let mut feed = [0.0; 4];
        spread_mono(&mono, &mut feed, 2);
        assert_eq!(feed, [0.5, 0.5, 0.25, 0.25]);
    }

    #[test]
    fn uids_match_the_driver_model() {
        use ss_hal_driver_macos::model::DeviceKind;
        assert_eq!(FEED_DEVICE_UID, DeviceKind::Feed.uid());
        assert_eq!(VIRTUAL_MIC_UID, DeviceKind::Mic.uid());
    }
}

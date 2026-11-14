//! Platform-independent AudioMix engine (`docs/architecture/audio-engine.md`).
//!
//! [`MixEngine`] runs inside the audio callback of a platform backend; it is controlled from
//! other threads through an [`EngineController`] over a lock-free queue.

mod command;
mod controller;
mod drift;
mod engine;
mod limiter;
mod upmix;

pub use command::Command;
pub use controller::{EngineBusy, EngineController};
pub use drift::{DriftReader, DriftStats, DriftWriter, MAX_CORRECTION, drift_buffer};
pub use engine::{ChannelInput, MixEngine};

/// Capacity of the control queue (commands waiting for the next callback).
pub const COMMAND_QUEUE_CAPACITY: usize = 64;

/// Creates an engine for the given output format and largest callback size, plus its controller.
///
/// # Errors
/// Returns [`ss_dsp::DspError`] for an invalid format.
pub fn create(
    sample_rate: f32,
    output_channels: usize,
    max_frames: usize,
) -> Result<(MixEngine, EngineController), ss_dsp::DspError> {
    let format = ss_dsp::Format::new(sample_rate, output_channels)?;
    let (producer, consumer) = rtrb::RingBuffer::new(COMMAND_QUEUE_CAPACITY);
    Ok((
        MixEngine::new(format, max_frames, consumer),
        EngineController::new(producer),
    ))
}

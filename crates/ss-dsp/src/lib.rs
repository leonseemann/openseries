//! Real-time-safe audio processing blocks (`docs/architecture/dsp.md`).
//!
//! All processors operate in place on **interleaved `f32`** buffers with a channel count fixed
//! at construction (1..=[`MAX_CHANNELS`]). After construction, no method allocates, locks or
//! panics, so they can run on an audio callback thread.

mod biquad;
mod boost;
mod chain;
mod chatmix;
mod denoise;
mod dynamics;
mod error;
mod gain;
pub mod level;
mod mic_chain;
mod peq;

#[cfg(test)]
mod test_util;

pub use biquad::{Biquad, Coefficients};
pub use boost::{BoostSettings, Boosts};
pub use chain::{ChannelStrip, ChannelStripSettings};
pub use chatmix::{ChatMixGains, balance_from_slider, chatmix_gains};
pub use denoise::{DENOISE_FRAME, DENOISE_LATENCY, DENOISE_SAMPLE_RATE, Denoiser};
pub use dynamics::{Compressor, CompressorSettings, NoiseGate, NoiseGateSettings, SmartVolume};
pub use error::DspError;
pub use gain::Gain;
pub use mic_chain::{
    DEFAULT_AUTO_GATE_OFFSET_DB, DEFAULT_DENOISE_MIX, DEFAULT_GATE_THRESHOLD_DB, GateMode,
    MicChain, MicChainSettings, voice_stabilizer,
};
pub use peq::ParametricEq;

/// Largest supported channel count (7.1 surround).
pub const MAX_CHANNELS: usize = 8;

/// An in-place audio processor.
pub trait Processor {
    /// Processes one interleaved buffer in place. `buffer.len()` must be a multiple of the
    /// processor's channel count; trailing partial frames are left untouched.
    fn process(&mut self, buffer: &mut [f32]);

    /// Clears internal state (filter memory, envelopes) without changing parameters.
    fn reset(&mut self);
}

/// Audio stream format shared by all processors of a chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Format {
    sample_rate: f32,
    channels: usize,
}

impl Format {
    /// Validates and creates a format.
    ///
    /// # Errors
    /// [`DspError`] if the sample rate is not positive/finite or the channel count is
    /// outside `1..=MAX_CHANNELS`.
    pub fn new(sample_rate: f32, channels: usize) -> Result<Self, DspError> {
        if !(sample_rate.is_finite() && sample_rate > 0.0) {
            return Err(DspError::InvalidSampleRate(sample_rate));
        }
        if !(1..=MAX_CHANNELS).contains(&channels) {
            return Err(DspError::UnsupportedChannels(channels));
        }
        Ok(Self {
            sample_rate,
            channels,
        })
    }

    /// Samples per second.
    #[must_use]
    pub const fn sample_rate(self) -> f32 {
        self.sample_rate
    }

    /// Interleaved channel count.
    #[must_use]
    pub const fn channels(self) -> usize {
        self.channels
    }

    /// One-pole smoothing coefficient for a time constant in milliseconds.
    #[must_use]
    pub fn smoothing_coefficient(self, time_ms: f32) -> f32 {
        if time_ms <= 0.0 {
            return 0.0;
        }
        (-1.0 / (time_ms * 0.001 * self.sample_rate)).exp()
    }
}

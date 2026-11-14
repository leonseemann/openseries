//! Microphone chain: denoise → noise gate → volume stabilizer → mic EQ → gain
//! (`docs/architecture/mic-chain.md` §2, SON-19/21/23/25).

use ss_core::eq::ParametricEqSettings;

use crate::level::{SILENCE_DB, linear_to_db};
use crate::{
    Compressor, CompressorSettings, Denoiser, DspError, Format, Gain, NoiseGate, NoiseGateSettings,
    ParametricEq, Processor,
};

/// Sonar's default dry/wet mix for ClearCast (`noiseCancelingState.value`, sonar.md §4).
pub const DEFAULT_DENOISE_MIX: f32 = 0.9;
/// Sonar's default manual gate threshold (`noiseGateState.value`).
pub const DEFAULT_GATE_THRESHOLD_DB: f32 = -60.0;
/// Our default offset of the automatic threshold above the noise floor (original Unknown).
pub const DEFAULT_AUTO_GATE_OFFSET_DB: f32 = 10.0;
/// Range the automatic threshold is clamped to.
const AUTO_THRESHOLD_RANGE_DB: (f32, f32) = (-80.0, -20.0);
/// How fast the tracked noise floor may rise (dB per second); it falls immediately.
const NOISE_FLOOR_RISE_DB_PER_S: f32 = 3.0;

/// How the gate threshold is chosen (SON-21).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GateMode {
    /// Fixed threshold in dBFS.
    Manual {
        /// Opening level.
        threshold_db: f32,
    },
    /// Threshold follows the measured noise floor (after denoise) plus an offset.
    Automatic {
        /// Distance above the noise floor.
        offset_db: f32,
    },
}

/// Settings of a [`MicChain`]. `None` disables a block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MicChainSettings {
    /// Denoise dry/wet mix 0..=1 (SON-19).
    pub denoise: Option<f32>,
    /// Noise gate (SON-21).
    pub gate: Option<GateMode>,
    /// Volume stabilizer = compressor (SON-23).
    pub stabilizer: Option<CompressorSettings>,
    /// Mic EQ (SON-25); `enabled = false` bypasses it.
    pub eq: ParametricEqSettings,
    /// Output gain in dB.
    pub gain_db: f32,
}

impl Default for MicChainSettings {
    /// Everything off: transparent apart from the gain.
    fn default() -> Self {
        let mut eq = ParametricEqSettings::flat();
        eq.enabled = false;
        Self {
            denoise: None,
            gate: None,
            stabilizer: None,
            eq,
            gain_db: 0.0,
        }
    }
}

/// Voice preset of the volume stabilizer (our tuning; Sonar's `Level` curve is Unknown).
#[must_use]
pub const fn voice_stabilizer() -> CompressorSettings {
    CompressorSettings {
        threshold_db: -30.0,
        ratio: 3.0,
        knee_db: 6.0,
        attack_ms: 10.0,
        release_ms: 200.0,
        makeup_db: 9.0,
    }
}

/// Tracks the noise floor with "minimum statistics": follows quieter blocks immediately and
/// rises slowly, so speech does not pull it up.
#[derive(Debug, Clone)]
struct NoiseFloor {
    floor_db: f32,
    sample_rate: f32,
}

impl NoiseFloor {
    const fn new(sample_rate: f32) -> Self {
        Self {
            floor_db: SILENCE_DB,
            sample_rate,
        }
    }

    fn update(&mut self, block: &[f32]) -> f32 {
        if block.is_empty() {
            return self.floor_db;
        }
        #[allow(clippy::cast_precision_loss)] // block sizes are small
        let len = block.len() as f32;
        let rms = (block.iter().map(|s| s * s).sum::<f32>() / len).sqrt();
        let level = linear_to_db(rms);
        let rise = NOISE_FLOOR_RISE_DB_PER_S * len / self.sample_rate;
        self.floor_db = if self.floor_db <= SILENCE_DB {
            level // first block
        } else {
            level.min(self.floor_db + rise)
        };
        self.floor_db
    }
}

/// The complete mono mic chain. Real-time safe after [`MicChain::new`].
#[derive(Debug)]
pub struct MicChain {
    denoiser: Denoiser,
    gate: NoiseGate,
    stabilizer: Compressor,
    eq: ParametricEq,
    gain: Gain,
    noise_floor: NoiseFloor,
    settings: MicChainSettings,
}

impl MicChain {
    /// Creates a transparent chain for 48 kHz mono.
    ///
    /// # Errors
    /// [`DspError`] if `sample_rate` is not 48 kHz (required by the denoiser).
    pub fn new(sample_rate: f32) -> Result<Self, DspError> {
        let format = Format::new(sample_rate, 1)?;
        let mut chain = Self {
            denoiser: Denoiser::new(format)?,
            gate: NoiseGate::new(format, NoiseGateSettings::default()),
            stabilizer: Compressor::new(format, voice_stabilizer()),
            eq: ParametricEq::new(format),
            gain: Gain::new(format),
            noise_floor: NoiseFloor::new(sample_rate),
            settings: MicChainSettings::default(),
        };
        chain.set(&MicChainSettings::default());
        chain.gain.snap();
        Ok(chain)
    }

    /// Applies new settings (no allocation).
    pub fn set(&mut self, settings: &MicChainSettings) {
        self.settings = *settings;
        if let Some(mix) = settings.denoise {
            self.denoiser.set_mix(mix);
        }
        if let Some(GateMode::Manual { threshold_db }) = settings.gate {
            self.set_gate_threshold(threshold_db);
        }
        if let Some(stabilizer) = settings.stabilizer {
            self.stabilizer.set(stabilizer);
        }
        self.eq.set(&settings.eq);
        self.gain.set_db(settings.gain_db);
    }

    /// Current input-to-output delay in frames (only the denoiser adds latency).
    #[must_use]
    pub const fn latency_frames(&self) -> usize {
        if self.settings.denoise.is_some() {
            self.denoiser.latency_frames()
        } else {
            0
        }
    }

    /// The gate threshold currently in use (useful to display the automatic value).
    #[must_use]
    pub fn gate_threshold_db(&self) -> f32 {
        self.gate_settings().threshold_db
    }

    fn gate_settings(&self) -> NoiseGateSettings {
        self.gate.settings()
    }

    fn set_gate_threshold(&mut self, threshold_db: f32) {
        let settings = NoiseGateSettings {
            threshold_db,
            ..self.gate.settings()
        };
        self.gate.set(settings);
    }
}

impl Processor for MicChain {
    fn process(&mut self, buffer: &mut [f32]) {
        if self.settings.denoise.is_some() {
            self.denoiser.process(buffer);
        }
        match self.settings.gate {
            Some(GateMode::Automatic { offset_db }) => {
                let floor = self.noise_floor.update(buffer);
                let (low, high) = AUTO_THRESHOLD_RANGE_DB;
                self.set_gate_threshold((floor + offset_db).clamp(low, high));
                self.gate.process(buffer);
            }
            Some(GateMode::Manual { .. }) => self.gate.process(buffer),
            None => {}
        }
        if self.settings.stabilizer.is_some() {
            self.stabilizer.process(buffer);
        }
        self.eq.process(buffer);
        self.gain.process(buffer);
    }

    fn reset(&mut self) {
        self.denoiser.reset();
        self.gate.reset();
        self.stabilizer.reset();
        self.eq.reset();
        self.gain.reset();
        self.noise_floor = NoiseFloor::new(self.noise_floor.sample_rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DENOISE_LATENCY;
    use crate::test_util::{FS, sine};

    fn chain(settings: &MicChainSettings) -> MicChain {
        let mut chain = MicChain::new(FS).unwrap();
        chain.set(settings);
        chain
    }

    #[test]
    fn default_chain_is_transparent_without_latency() {
        let mut chain = chain(&MicChainSettings::default());
        let input = sine(440.0, 0.3, 0.1, 1);
        let mut output = input.clone();
        chain.process(&mut output);
        assert_eq!(chain.latency_frames(), 0);
        let max_error = input
            .iter()
            .zip(&output)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f32, f32::max);
        assert!(max_error < 1e-4, "max error {max_error}");
    }

    #[test]
    fn denoise_adds_its_latency() {
        let chain = chain(&MicChainSettings {
            denoise: Some(DEFAULT_DENOISE_MIX),
            ..MicChainSettings::default()
        });
        assert_eq!(chain.latency_frames(), DENOISE_LATENCY);
    }

    #[test]
    fn manual_gate_silences_quiet_input_and_passes_voice_level() {
        let mut chain = chain(&MicChainSettings {
            gate: Some(GateMode::Manual {
                threshold_db: -40.0,
            }),
            ..MicChainSettings::default()
        });
        let mut quiet = sine(200.0, 0.001, 0.5, 1); // −60 dBFS
        chain.process(&mut quiet);
        let tail_peak = quiet[quiet.len() / 2..]
            .iter()
            .fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(tail_peak < 1e-5, "quiet input leaked: {tail_peak}");

        let mut loud = sine(200.0, 0.3, 0.5, 1);
        chain.process(&mut loud);
        let tail_peak = loud[loud.len() / 2..]
            .iter()
            .fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(tail_peak > 0.29, "voice level was gated: {tail_peak}");
    }

    #[test]
    fn automatic_threshold_settles_above_the_noise_floor() {
        let mut chain = chain(&MicChainSettings {
            gate: Some(GateMode::Automatic {
                offset_db: DEFAULT_AUTO_GATE_OFFSET_DB,
            }),
            ..MicChainSettings::default()
        });
        // Constant floor at about −53 dBFS RMS (sine amplitude 0.00316 → RMS −53 dBFS).
        let mut floor = sine(150.0, 0.003_16, 2.0, 1);
        for block in floor.chunks_mut(512) {
            chain.process(block);
        }
        let threshold = chain.gate_threshold_db();
        assert!(
            (threshold - (-53.0 + DEFAULT_AUTO_GATE_OFFSET_DB)).abs() < 1.5,
            "threshold {threshold}"
        );
    }

    #[test]
    fn automatic_threshold_is_not_pulled_up_by_short_speech() {
        let mut chain = chain(&MicChainSettings {
            gate: Some(GateMode::Automatic {
                offset_db: DEFAULT_AUTO_GATE_OFFSET_DB,
            }),
            ..MicChainSettings::default()
        });
        let mut floor = sine(150.0, 0.003_16, 1.0, 1);
        for block in floor.chunks_mut(512) {
            chain.process(block);
        }
        let before = chain.gate_threshold_db();
        let mut speech = sine(300.0, 0.3, 0.5, 1);
        for block in speech.chunks_mut(512) {
            chain.process(block);
        }
        let after = chain.gate_threshold_db();
        assert!(
            after - before <= NOISE_FLOOR_RISE_DB_PER_S * 0.5 + 0.1,
            "threshold rose from {before} to {after}"
        );
    }

    #[test]
    fn stabilizer_raises_quiet_speech() {
        let mut chain = chain(&MicChainSettings {
            stabilizer: Some(voice_stabilizer()),
            ..MicChainSettings::default()
        });
        let mut quiet = sine(300.0, 0.02, 0.5, 1); // −34 dBFS peak
        chain.process(&mut quiet);
        let tail_peak = quiet[quiet.len() / 2..]
            .iter()
            .fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(tail_peak > 0.03, "makeup gain missing: {tail_peak}");
    }
}

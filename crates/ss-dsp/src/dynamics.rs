//! Dynamics: compressor (Smart Volume, mic volume stabilizer) and noise gate.

use crate::level::{db_to_linear, linear_to_db};
use crate::{Format, Processor};

/// Peak level of one interleaved frame.
fn frame_peak(frame: &[f32]) -> f32 {
    frame.iter().fold(0.0_f32, |peak, s| peak.max(s.abs()))
}

/// Compressor parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressorSettings {
    /// Level above which gain reduction starts (dBFS).
    pub threshold_db: f32,
    /// Input:output ratio above the threshold (≥ 1).
    pub ratio: f32,
    /// Width of the soft knee in dB (0 = hard knee).
    pub knee_db: f32,
    /// Time to apply gain reduction.
    pub attack_ms: f32,
    /// Time to release gain reduction.
    pub release_ms: f32,
    /// Gain added after compression.
    pub makeup_db: f32,
}

impl CompressorSettings {
    /// Static gain change in dB (≤ 0) for an input level in dBFS.
    #[must_use]
    pub fn gain_reduction_db(&self, level_db: f32) -> f32 {
        let slope = 1.0 / self.ratio.max(1.0) - 1.0;
        let over = level_db - self.threshold_db;
        let half_knee = self.knee_db.max(0.0) / 2.0;
        if over <= -half_knee {
            0.0
        } else if over < half_knee {
            slope * (over + half_knee).powi(2) / (2.0 * self.knee_db)
        } else {
            slope * over
        }
    }
}

/// Loudness presets of Smart Volume (SON-12). Values are our tuning; the original's are
/// Unknown (`docs/architecture/dsp.md` §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SmartVolume {
    /// Gentle levelling.
    Soft,
    /// Default.
    Balanced,
    /// Strong levelling, quiet sounds become much louder.
    Extreme,
}

impl SmartVolume {
    /// Compressor parameters for this preset.
    #[must_use]
    pub const fn settings(self) -> CompressorSettings {
        let (threshold_db, ratio, makeup_db) = match self {
            Self::Soft => (-18.0, 2.0, 4.0),
            Self::Balanced => (-24.0, 3.0, 8.0),
            Self::Extreme => (-32.0, 6.0, 14.0),
        };
        CompressorSettings {
            threshold_db,
            ratio,
            knee_db: 6.0,
            attack_ms: 5.0,
            release_ms: 250.0,
            makeup_db,
        }
    }
}

/// Feed-forward peak compressor with smoothed gain reduction, linked across channels.
#[derive(Debug, Clone)]
pub struct Compressor {
    format: Format,
    settings: CompressorSettings,
    attack: f32,
    release: f32,
    makeup: f32,
    reduction_db: f32,
}

impl Compressor {
    /// Creates a compressor.
    #[must_use]
    pub fn new(format: Format, settings: CompressorSettings) -> Self {
        let mut compressor = Self {
            format,
            settings,
            attack: 0.0,
            release: 0.0,
            makeup: 1.0,
            reduction_db: 0.0,
        };
        compressor.set(settings);
        compressor
    }

    /// Applies new parameters.
    pub fn set(&mut self, settings: CompressorSettings) {
        self.settings = settings;
        self.attack = self.format.smoothing_coefficient(settings.attack_ms);
        self.release = self.format.smoothing_coefficient(settings.release_ms);
        self.makeup = db_to_linear(settings.makeup_db);
    }
}

impl Processor for Compressor {
    fn process(&mut self, buffer: &mut [f32]) {
        for frame in buffer.chunks_exact_mut(self.format.channels()) {
            let target = self
                .settings
                .gain_reduction_db(linear_to_db(frame_peak(frame)));
            let coefficient = if target < self.reduction_db {
                self.attack
            } else {
                self.release
            };
            self.reduction_db = target + coefficient * (self.reduction_db - target);
            let gain = db_to_linear(self.reduction_db) * self.makeup;
            for sample in frame {
                *sample *= gain;
            }
        }
    }

    fn reset(&mut self) {
        self.reduction_db = 0.0;
    }
}

/// Noise-gate parameters (SON-21).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseGateSettings {
    /// Level above which the gate opens (dBFS). Sonar's default is −60.
    pub threshold_db: f32,
    /// The gate closes only below `threshold_db − hysteresis_db`.
    pub hysteresis_db: f32,
    /// Time the gate stays open after the level drops.
    pub hold_ms: f32,
    /// Fade-in time when opening.
    pub attack_ms: f32,
    /// Fade-out time when closing.
    pub release_ms: f32,
    /// Attenuation while closed (dB, negative).
    pub floor_db: f32,
}

impl Default for NoiseGateSettings {
    fn default() -> Self {
        Self {
            threshold_db: -60.0,
            hysteresis_db: 6.0,
            hold_ms: 80.0,
            attack_ms: 1.0,
            release_ms: 120.0,
            floor_db: -80.0,
        }
    }
}

/// Envelope release of the level detector.
const DETECTOR_RELEASE_MS: f32 = 10.0;

/// Gate with hysteresis and hold, linked across channels.
#[derive(Debug, Clone)]
pub struct NoiseGate {
    format: Format,
    settings: NoiseGateSettings,
    detector_release: f32,
    attack: f32,
    release: f32,
    hold_frames: u32,
    envelope: f32,
    open: bool,
    hold_left: u32,
    gain: f32,
}

impl NoiseGate {
    /// Creates a closed gate.
    #[must_use]
    pub fn new(format: Format, settings: NoiseGateSettings) -> Self {
        let mut gate = Self {
            format,
            settings,
            detector_release: format.smoothing_coefficient(DETECTOR_RELEASE_MS),
            attack: 0.0,
            release: 0.0,
            hold_frames: 0,
            envelope: 0.0,
            open: false,
            hold_left: 0,
            gain: 0.0,
        };
        gate.set(settings);
        gate.reset();
        gate
    }

    /// Current parameters.
    #[must_use]
    pub const fn settings(&self) -> NoiseGateSettings {
        self.settings
    }

    /// Applies new parameters.
    pub fn set(&mut self, settings: NoiseGateSettings) {
        self.settings = settings;
        self.attack = self.format.smoothing_coefficient(settings.attack_ms);
        self.release = self.format.smoothing_coefficient(settings.release_ms);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let hold = (settings.hold_ms.max(0.0) * 0.001 * self.format.sample_rate()) as u32;
        self.hold_frames = hold;
    }

    fn update_state(&mut self, level_db: f32) {
        if level_db > self.settings.threshold_db {
            self.open = true;
            self.hold_left = self.hold_frames;
        } else if level_db < self.settings.threshold_db - self.settings.hysteresis_db {
            if self.hold_left > 0 {
                self.hold_left -= 1;
            } else {
                self.open = false;
            }
        }
    }
}

impl Processor for NoiseGate {
    fn process(&mut self, buffer: &mut [f32]) {
        let floor = db_to_linear(self.settings.floor_db);
        for frame in buffer.chunks_exact_mut(self.format.channels()) {
            let peak = frame_peak(frame);
            self.envelope = peak.max(self.detector_release * self.envelope);
            self.update_state(linear_to_db(self.envelope));

            let (target, coefficient) = if self.open {
                (1.0, self.attack)
            } else {
                (floor, self.release)
            };
            self.gain = target + coefficient * (self.gain - target);
            for sample in frame {
                *sample *= self.gain;
            }
        }
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.open = false;
        self.hold_left = 0;
        self.gain = db_to_linear(self.settings.floor_db);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{FS, assert_db, gain_at};

    fn hard_knee(threshold_db: f32, ratio: f32) -> CompressorSettings {
        CompressorSettings {
            threshold_db,
            ratio,
            knee_db: 0.0,
            attack_ms: 1.0,
            release_ms: 200.0,
            makeup_db: 0.0,
        }
    }

    #[test]
    fn static_curve() {
        let s = hard_knee(-20.0, 4.0);
        assert!((s.gain_reduction_db(-30.0)).abs() < f32::EPSILON);
        assert!((s.gain_reduction_db(-12.0) + 6.0).abs() < 1e-4);
        let soft = CompressorSettings { knee_db: 6.0, ..s };
        let at_threshold = soft.gain_reduction_db(-20.0);
        assert!(
            at_threshold < 0.0 && at_threshold > -1.0,
            "soft knee eases in"
        );
    }

    #[test]
    fn compressor_reduces_loud_signal() {
        // 0.25 peak ≈ −12 dBFS → 8 dB over a −20 dB threshold at 4:1 → −6 dB.
        let mut c = Compressor::new(Format::new(FS, 2).unwrap(), hard_knee(-20.0, 4.0));
        assert_db(gain_at(&mut c, 1_000.0, 2), -6.0, 1.0);
        let mut quiet = Compressor::new(Format::new(FS, 2).unwrap(), hard_knee(0.0, 4.0));
        assert_db(gain_at(&mut quiet, 1_000.0, 2), 0.0, 0.1);
    }

    #[test]
    fn smart_volume_presets_get_stronger() {
        let ratios = [
            SmartVolume::Soft,
            SmartVolume::Balanced,
            SmartVolume::Extreme,
        ]
        .map(|p| p.settings().ratio);
        assert!(ratios.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn gate_passes_loud_and_mutes_quiet_signal() {
        let format = Format::new(FS, 1).unwrap();
        let settings = NoiseGateSettings {
            threshold_db: -40.0,
            ..NoiseGateSettings::default()
        };
        let mut gate = NoiseGate::new(format, settings);
        assert_db(gain_at(&mut gate, 300.0, 1), 0.0, 0.2); // 0.25 ≈ −12 dBFS
        let mut quiet = vec![0.001_f32; 48_000]; // −60 dBFS
        gate.reset();
        gate.process(&mut quiet);
        assert!(quiet[47_999].abs() < 1e-6);
    }
}

//! Per-channel processing chain of the playback path (Game/Chat/Media/Aux).

use ss_core::eq::ParametricEqSettings;

use crate::{
    BoostSettings, Boosts, Compressor, Format, Gain, ParametricEq, Processor, SmartVolume,
};

/// Settings of a [`ChannelStrip`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelStripSettings {
    /// Parametric EQ (SON-10).
    pub eq: ParametricEqSettings,
    /// Bass/treble/clarity boosts (SON-11).
    pub boosts: BoostSettings,
    /// Smart Volume preset, `None` = off (SON-12).
    pub smart_volume: Option<SmartVolume>,
    /// Final gain in dB, e.g. volume boost or channel fader (SON-13).
    pub gain_db: f32,
}

impl Default for ChannelStripSettings {
    fn default() -> Self {
        Self {
            eq: ParametricEqSettings::flat(),
            boosts: BoostSettings::default(),
            smart_volume: None,
            gain_db: 0.0,
        }
    }
}

/// EQ → boosts → Smart Volume → gain.
#[derive(Debug, Clone)]
pub struct ChannelStrip {
    eq: ParametricEq,
    boosts: Boosts,
    smart_volume: Compressor,
    smart_volume_enabled: bool,
    gain: Gain,
}

impl ChannelStrip {
    /// Creates a strip with default (transparent) settings.
    #[must_use]
    pub fn new(format: Format) -> Self {
        let mut strip = Self {
            eq: ParametricEq::new(format),
            boosts: Boosts::new(format),
            smart_volume: Compressor::new(format, SmartVolume::Balanced.settings()),
            smart_volume_enabled: false,
            gain: Gain::new(format),
        };
        strip.set(&ChannelStripSettings::default());
        strip.gain.snap();
        strip
    }

    /// Applies new settings without interrupting the audio.
    pub fn set(&mut self, settings: &ChannelStripSettings) {
        self.eq.set(&settings.eq);
        self.boosts.set(settings.boosts);
        self.smart_volume_enabled = settings.smart_volume.is_some();
        if let Some(preset) = settings.smart_volume {
            self.smart_volume.set(preset.settings());
        }
        self.gain.set_db(settings.gain_db);
    }
}

impl Processor for ChannelStrip {
    fn process(&mut self, buffer: &mut [f32]) {
        self.eq.process(buffer);
        self.boosts.process(buffer);
        if self.smart_volume_enabled {
            self.smart_volume.process(buffer);
        }
        self.gain.process(buffer);
    }

    fn reset(&mut self) {
        self.eq.reset();
        self.boosts.reset();
        self.smart_volume.reset();
        self.gain.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{FS, assert_db, gain_at};

    #[test]
    fn default_strip_is_transparent() {
        let mut strip = ChannelStrip::new(Format::new(FS, 2).unwrap());
        assert_db(gain_at(&mut strip, 1_000.0, 2), 0.0, 0.05);
    }

    #[test]
    fn stages_combine() {
        let mut strip = ChannelStrip::new(Format::new(FS, 2).unwrap());
        strip.set(&ChannelStripSettings {
            boosts: BoostSettings {
                bass_db: 6.0,
                ..BoostSettings::default()
            },
            gain_db: -6.0,
            ..ChannelStripSettings::default()
        });
        assert_db(gain_at(&mut strip, 30.0, 2), 0.0, 0.7);
        assert_db(gain_at(&mut strip, 5_000.0, 2), -6.0, 0.3);
    }
}

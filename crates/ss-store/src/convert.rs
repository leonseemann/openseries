//! Mapping from the file DTOs to the serde-free domain types.

use ss_core::RangeError;
use ss_core::eq::{
    BandGain, FilterType, Frequency, PEQ_BANDS, ParametricEqSettings, PeqBand, QFactor,
};
use ss_core::mixer::{ChannelId, Fader};
use ss_dsp::{
    BoostSettings, ChannelStripSettings, GateMode, MicChainSettings, SmartVolume, voice_stabilizer,
};
use thiserror::Error;

use crate::config::{
    BandConfig, Channel, ChannelConfig, EqConfig, FaderConfig, GateConfig, Loudness, MicConfig,
};

/// A stored value the domain rejects.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ConvertError {
    /// An EQ band value is out of range.
    #[error("EQ band {band}: {source}")]
    Band {
        /// 1-based band number.
        band: usize,
        /// The violated range.
        source: RangeError,
    },
    /// Unknown Sonar filter id.
    #[error("EQ band {band}: unknown filter type {id}")]
    FilterType {
        /// 1-based band number.
        band: usize,
        /// The stored id.
        id: u8,
    },
    /// More than ten bands.
    #[error("at most {PEQ_BANDS} EQ bands are supported, got {0}")]
    TooManyBands(usize),
}

impl From<Channel> for ChannelId {
    fn from(channel: Channel) -> Self {
        match channel {
            Channel::Game => Self::Game,
            Channel::Chat => Self::Chat,
            Channel::Media => Self::Media,
            Channel::Aux => Self::Aux,
        }
    }
}

impl From<ChannelId> for Channel {
    fn from(channel: ChannelId) -> Self {
        match channel {
            ChannelId::Game => Self::Game,
            ChannelId::Chat => Self::Chat,
            ChannelId::Media => Self::Media,
            ChannelId::Aux => Self::Aux,
        }
    }
}

impl From<FaderConfig> for Fader {
    fn from(fader: FaderConfig) -> Self {
        Self {
            gain: fader.volume,
            muted: fader.muted,
        }
    }
}

impl From<Loudness> for SmartVolume {
    fn from(loudness: Loudness) -> Self {
        match loudness {
            Loudness::Soft => Self::Soft,
            Loudness::Balanced => Self::Balanced,
            Loudness::Extreme => Self::Extreme,
        }
    }
}

impl From<GateConfig> for GateMode {
    fn from(gate: GateConfig) -> Self {
        match gate {
            GateConfig::Manual { threshold_db } => Self::Manual { threshold_db },
            GateConfig::Automatic { offset_db } => Self::Automatic { offset_db },
        }
    }
}

impl EqConfig {
    /// The EQ as domain settings: listed bands replace the default flat bands from the lowest
    /// slot up.
    ///
    /// # Errors
    /// [`ConvertError`] for out-of-range values, unknown filter ids or too many bands.
    pub fn to_domain(&self) -> Result<ParametricEqSettings, ConvertError> {
        if self.bands.len() > PEQ_BANDS {
            return Err(ConvertError::TooManyBands(self.bands.len()));
        }
        let mut settings = ParametricEqSettings::flat();
        settings.enabled = self.enabled;
        for (index, (slot, band)) in settings.bands.iter_mut().zip(&self.bands).enumerate() {
            *slot = band.into_band(index + 1)?;
        }
        Ok(settings)
    }
}

impl BandConfig {
    fn into_band(self, band: usize) -> Result<PeqBand, ConvertError> {
        let range = |source| ConvertError::Band { band, source };
        Ok(PeqBand {
            enabled: self.enabled,
            filter: FilterType::from_sonar_id(self.filter).ok_or(ConvertError::FilterType {
                band,
                id: self.filter,
            })?,
            frequency: Frequency::new(self.frequency).map_err(range)?,
            gain: BandGain::new(self.gain_db).map_err(range)?,
            q: QFactor::new(self.q).map_err(range)?,
        })
    }
}

impl ChannelConfig {
    /// The channel's DSP strip.
    ///
    /// # Errors
    /// [`ConvertError`] if the EQ is invalid.
    pub fn strip(&self) -> Result<ChannelStripSettings, ConvertError> {
        Ok(ChannelStripSettings {
            eq: self.eq.to_domain()?,
            boosts: BoostSettings {
                bass_db: self.boosts.bass_db,
                treble_db: self.boosts.treble_db,
                clarity_db: self.boosts.clarity_db,
            },
            smart_volume: self.smart_volume.map(SmartVolume::from),
            gain_db: self.gain_db,
        })
    }
}

impl MicConfig {
    /// The mic chain settings.
    ///
    /// # Errors
    /// [`ConvertError`] if the mic EQ is invalid.
    pub fn chain(&self) -> Result<MicChainSettings, ConvertError> {
        Ok(MicChainSettings {
            denoise: self.denoise.map(|mix| mix.clamp(0.0, 1.0)),
            gate: self.gate.map(GateMode::from),
            stabilizer: self.stabilizer.then(voice_stabilizer),
            eq: self.eq.to_domain()?,
            gain_db: self.gain_db,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn default_config_converts_to_transparent_settings() {
        let config = Config::default();
        for channel in [Channel::Game, Channel::Chat, Channel::Media, Channel::Aux] {
            let strip = config.audiomix.channels.get(channel).strip().unwrap();
            assert!(!strip.eq.enabled);
            assert!(strip.smart_volume.is_none());
        }
        let mic = config.mic.chain().unwrap();
        assert!(mic.denoise.is_none() && mic.gate.is_none() && mic.stabilizer.is_none());
        assert_eq!(Fader::from(config.audiomix.master), Fader::UNITY);
    }

    #[test]
    fn bands_are_validated() {
        let band = |gain_db| BandConfig {
            enabled: true,
            filter: 7,
            frequency: 1000.0,
            gain_db,
            q: 1.0,
        };
        let ok = EqConfig {
            enabled: true,
            bands: vec![band(3.0)],
        };
        assert!((ok.to_domain().unwrap().bands[0].gain.get() - 3.0).abs() < f32::EPSILON);
        let too_loud = EqConfig {
            enabled: true,
            bands: vec![band(3.0), band(40.0)],
        };
        assert!(matches!(
            too_loud.to_domain(),
            Err(ConvertError::Band { band: 2, .. })
        ));
        let bad_type = EqConfig {
            enabled: true,
            bands: vec![BandConfig {
                filter: 42,
                ..band(0.0)
            }],
        };
        assert!(matches!(
            bad_type.to_domain(),
            Err(ConvertError::FilterType { id: 42, .. })
        ));
    }

    #[test]
    fn channel_ids_round_trip() {
        for id in ChannelId::ALL {
            assert_eq!(ChannelId::from(Channel::from(id)), id);
        }
    }
}

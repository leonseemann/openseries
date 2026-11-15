//! Named sound presets and their file format (`docs/architecture/app.md` §3.1; SON-15, SON-17,
//! HS-01).
//!
//! Three kinds, each a map name → settings inside `config.json`:
//!
//! * **Headset EQ** — the on-device 10-band curve (dB per band).
//! * **Playback** — one AudioMix channel's sound (EQ, boosts, Smart Volume, volume boost); the
//!   channel's fader is not part of it.
//! * **Microphone** — the mic chain (denoise, gate, stabilizer, EQ, gain); the on/off switch and
//!   the source device are not part of it.
//!
//! Export writes one preset as a small JSON document ([`export_json`]). [`import_json`] reads that
//! format and, best effort, a Sonar config object (`docs/re/sonar.md` §4.2): the shape of the
//! config JSON is Observed at runtime, but whether GG's export file is exactly that object is
//! **Inferred**.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ss_core::eq::FilterType;
use ss_dsp::{DEFAULT_AUTO_GATE_OFFSET_DB, DEFAULT_DENOISE_MIX};
use thiserror::Error;

use crate::config::{
    BandConfig, BoostConfig, ChannelConfig, EqConfig, FaderConfig, GateConfig, Loudness, MicConfig,
};

/// Number of bands of the headset's on-device EQ.
pub const HEADSET_EQ_BANDS: usize = 10;
/// `format` value of an exported preset.
const FORMAT: &str = "openseries-preset";
/// Version of the export format.
const FORMAT_VERSION: u32 = 1;
/// Number of PEQ filters in a Sonar config (`filter1..filter10`).
const SONAR_FILTERS: usize = 10;

/// All saved presets.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresetsConfig {
    /// Headset EQ curves (dB per band, lowest band first).
    pub headset_eq: BTreeMap<String, Vec<f32>>,
    /// Playback channel sounds.
    pub playback: BTreeMap<String, ChannelConfig>,
    /// Mic chain sounds.
    pub mic: BTreeMap<String, MicConfig>,
}

/// The settings of one preset, by kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "settings", rename_all = "snake_case")]
pub enum PresetData {
    /// Headset EQ curve.
    HeadsetEq(Vec<f32>),
    /// Playback channel sound.
    Playback(ChannelConfig),
    /// Mic chain sound.
    Mic(MicConfig),
}

impl PresetData {
    /// The playback part of a channel (everything except the fader).
    #[must_use]
    pub fn playback_of(channel: &ChannelConfig) -> Self {
        Self::Playback(ChannelConfig {
            fader: FaderConfig::default(),
            ..channel.clone()
        })
    }

    /// The sound part of the mic config (without switch and source device).
    #[must_use]
    pub fn mic_of(mic: &MicConfig) -> Self {
        Self::Mic(MicConfig {
            enabled: false,
            input_device_uid: None,
            ..mic.clone()
        })
    }
}

impl PresetsConfig {
    /// Stores `data` under `name`, replacing a preset of the same kind and name.
    pub fn insert(&mut self, name: String, data: PresetData) {
        match data {
            PresetData::HeadsetEq(curve) => {
                self.headset_eq.insert(name, curve);
            }
            PresetData::Playback(channel) => {
                self.playback.insert(name, channel);
            }
            PresetData::Mic(mic) => {
                self.mic.insert(name, mic);
            }
        }
    }
}

/// Applies a playback preset to a channel, keeping its fader.
pub fn apply_playback(channel: &mut ChannelConfig, preset: &ChannelConfig) {
    let fader = channel.fader;
    *channel = preset.clone();
    channel.fader = fader;
}

/// Applies a mic preset, keeping the switch and the source device.
pub fn apply_mic(mic: &mut MicConfig, preset: &MicConfig) {
    let (enabled, input) = (mic.enabled, mic.input_device_uid.take());
    *mic = preset.clone();
    mic.enabled = enabled;
    mic.input_device_uid = input;
}

#[derive(Serialize, Deserialize)]
struct ExportedPreset {
    format: String,
    version: u32,
    name: String,
    #[serde(flatten)]
    data: PresetData,
}

/// A preset file could not be read.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImportError {
    /// Not JSON.
    #[error("not a preset file: {0}")]
    Json(String),
    /// JSON, but neither a OpenSeries preset nor a Sonar config.
    #[error("unknown preset format")]
    UnknownFormat,
    /// A OpenSeries preset from a newer version.
    #[error("preset format version {0} is newer than this app supports")]
    TooNew(u32),
    /// The content does not fit its kind (e.g. a headset curve without 10 bands).
    #[error("invalid preset: {0}")]
    Invalid(&'static str),
}

/// Serialises one preset for export.
///
/// # Errors
/// Only if serialisation fails (not expected for these types).
pub fn export_json(name: &str, data: &PresetData) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&ExportedPreset {
        format: FORMAT.to_owned(),
        version: FORMAT_VERSION,
        name: name.to_owned(),
        data: data.clone(),
    })
}

/// Reads a preset file: a OpenSeries export or a Sonar config object.
///
/// # Errors
/// [`ImportError`] if the text is not one of the supported formats.
pub fn import_json(text: &str) -> Result<(String, PresetData), ImportError> {
    let value: Value = serde_json::from_str(text).map_err(|e| ImportError::Json(e.to_string()))?;
    if value.get("format").and_then(Value::as_str) == Some(FORMAT) {
        let version = value
            .get("version")
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);
        if version > FORMAT_VERSION {
            return Err(ImportError::TooNew(version));
        }
        let preset: ExportedPreset =
            serde_json::from_value(value).map_err(|e| ImportError::Json(e.to_string()))?;
        if let PresetData::HeadsetEq(curve) = &preset.data {
            if curve.len() != HEADSET_EQ_BANDS {
                return Err(ImportError::Invalid("a headset EQ needs 10 bands"));
            }
        }
        return Ok((preset.name, preset.data));
    }
    sonar_config(&value)
}

/// Maps a Sonar config object (`docs/re/sonar.md` §4.2) to a preset.
fn sonar_config(value: &Value) -> Result<(String, PresetData), ImportError> {
    let (Some(device), Some(data)) = (
        value.get("virtualAudioDevice").and_then(Value::as_str),
        value.get("data"),
    ) else {
        return Err(ImportError::UnknownFormat);
    };
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Sonar preset")
        .to_owned();
    let eq = sonar_eq(data.get("parametricEQ"));
    let preset = match device {
        "chatCapture" => PresetData::Mic(MicConfig {
            denoise: state(data, "noiseCancelingState")
                .then(|| number(data, "noiseCancelingState").unwrap_or(DEFAULT_DENOISE_MIX)),
            gate: if state(data, "automaticNoiseGateState") {
                Some(GateConfig::Automatic {
                    offset_db: DEFAULT_AUTO_GATE_OFFSET_DB,
                })
            } else if state(data, "noiseGateState") {
                number(data, "noiseGateState")
                    .map(|threshold_db| GateConfig::Manual { threshold_db })
            } else {
                None
            },
            stabilizer: state(data, "volumeStabilizerState"),
            eq,
            ..MicConfig::default()
        }),
        "game" | "chatRender" | "media" | "aux" => {
            let boost = |key| {
                if state(data, key) {
                    number(data, key).unwrap_or(0.0)
                } else {
                    0.0
                }
            };
            let smart = data.get("smartVolume");
            let smart_on = smart
                .and_then(|s| s.get("enabled"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            PresetData::Playback(ChannelConfig {
                eq,
                boosts: BoostConfig {
                    bass_db: boost("bassBoostState"),
                    treble_db: boost("trebleBoostState"),
                    clarity_db: boost("voiceClarityState"),
                },
                smart_volume: smart_on.then(|| {
                    match smart
                        .and_then(|s| s.get("loudness"))
                        .and_then(Value::as_str)
                    {
                        Some("soft") => Loudness::Soft,
                        Some("extreme") => Loudness::Extreme,
                        _ => Loudness::Balanced,
                    }
                }),
                gain_db: data
                    .get("generalGain")
                    .and_then(Value::as_f64)
                    .map_or(0.0, as_f32),
                ..ChannelConfig::default()
            })
        }
        _ => return Err(ImportError::UnknownFormat),
    };
    Ok((name, preset))
}

/// `{"enabled": bool, …}` of a Sonar state object.
fn state(data: &Value, key: &str) -> bool {
    data.get(key)
        .and_then(|s| s.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// `{"value": number}` of a Sonar state object.
fn number(data: &Value, key: &str) -> Option<f32> {
    data.get(key)?.get("value")?.as_f64().map(as_f32)
}

#[allow(clippy::cast_possible_truncation)] // dB / Hz / Q values are far inside f32 range
const fn as_f32(value: f64) -> f32 {
    value as f32
}

fn sonar_eq(peq: Option<&Value>) -> EqConfig {
    let Some(peq) = peq else {
        return EqConfig::default();
    };
    let bands = (1..=SONAR_FILTERS)
        .filter_map(|n| {
            let filter = peq.get(format!("filter{n}"))?;
            let get = |key: &str| filter.get(key).and_then(Value::as_f64).map(as_f32);
            Some(BandConfig {
                enabled: filter
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                filter: sonar_filter_type(filter.get("type").and_then(Value::as_str)).sonar_id(),
                frequency: get("frequency")?,
                gain_db: get("gain").unwrap_or(0.0),
                q: get("qFactor").unwrap_or(std::f32::consts::FRAC_1_SQRT_2),
            })
        })
        .collect();
    EqConfig {
        enabled: peq.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        bands,
    }
}

/// Sonar's filter type strings (`docs/re/sonar.md` §3.2, §4.2).
fn sonar_filter_type(name: Option<&str>) -> FilterType {
    match name {
        Some("lowPass") => FilterType::LowPass,
        Some("highPass") => FilterType::HighPass,
        Some("bandPassPeakQ") => FilterType::BandPassPeakQ,
        Some("bandPassPeak0dB") => FilterType::BandPassPeak0Db,
        Some("notchFilter") => FilterType::Notch,
        Some("allPass") => FilterType::AllPass,
        Some("lowShelving") => FilterType::LowShelf,
        Some("highShelving") => FilterType::HighShelf,
        Some("byPass") => FilterType::Bypass,
        _ => FilterType::Peaking,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_import_round_trip_for_every_kind() {
        let mut channel = ChannelConfig::default();
        channel.boosts.bass_db = 4.0;
        channel.smart_volume = Some(Loudness::Soft);
        let mic = MicConfig {
            denoise: Some(0.8),
            stabilizer: true,
            ..MicConfig::default()
        };
        for data in [
            PresetData::HeadsetEq(vec![1.5; HEADSET_EQ_BANDS]),
            PresetData::playback_of(&channel),
            PresetData::mic_of(&mic),
        ] {
            let text = export_json("Mine", &data).unwrap();
            assert_eq!(import_json(&text), Ok(("Mine".to_owned(), data)));
        }
    }

    #[test]
    fn playback_presets_leave_the_fader_alone() {
        let mut channel = ChannelConfig::default();
        channel.fader.volume = 0.3;
        let preset = ChannelConfig {
            gain_db: 6.0,
            ..ChannelConfig::default()
        };
        apply_playback(&mut channel, &preset);
        assert!((channel.fader.volume - 0.3).abs() < f32::EPSILON);
        assert!((channel.gain_db - 6.0).abs() < f32::EPSILON);
    }

    #[test]
    fn mic_presets_keep_switch_and_source() {
        let mut mic = MicConfig {
            enabled: true,
            input_device_uid: Some("headset".to_owned()),
            ..MicConfig::default()
        };
        let preset = MicConfig {
            stabilizer: true,
            ..MicConfig::default()
        };
        apply_mic(&mut mic, &preset);
        assert!(mic.enabled && mic.stabilizer);
        assert_eq!(mic.input_device_uid.as_deref(), Some("headset"));
    }

    #[test]
    fn imports_a_sonar_render_config() {
        let text = r#"{ "id": "x", "name": "Footsteps", "virtualAudioDevice": "game",
            "data": { "generalGain": 2,
              "bassBoostState": {"enabled": true, "value": 3},
              "trebleBoostState": {"enabled": false, "value": 5},
              "voiceClarityState": {"enabled": true, "value": 1},
              "smartVolume": {"enabled": true, "volumeLevel": 0, "loudness": "extreme"},
              "parametricEQ": {"enabled": true,
                "filter1": {"enabled": true, "qFactor": 0.7071, "frequency": 35, "gain": -2, "type": "lowShelving"},
                "filter2": {"enabled": false, "qFactor": 1.0, "frequency": 1000, "gain": 4, "type": "peakingEQ"} } } }"#;
        let (name, data) = import_json(text).unwrap();
        assert_eq!(name, "Footsteps");
        let PresetData::Playback(channel) = data else {
            panic!("expected a playback preset");
        };
        assert!((channel.gain_db - 2.0).abs() < f32::EPSILON);
        assert!((channel.boosts.bass_db - 3.0).abs() < f32::EPSILON);
        assert!(channel.boosts.treble_db.abs() < f32::EPSILON); // disabled
        assert_eq!(channel.smart_volume, Some(Loudness::Extreme));
        assert_eq!(channel.eq.bands.len(), 2);
        assert_eq!(channel.eq.bands[0].filter, FilterType::LowShelf.sonar_id());
        assert!(!channel.eq.bands[1].enabled);
    }

    #[test]
    fn imports_a_sonar_capture_config() {
        let text = r#"{ "name": "Voice", "virtualAudioDevice": "chatCapture",
            "data": { "noiseCancelingState": {"enabled": true, "value": 0.9},
                      "noiseGateState": {"enabled": true, "value": -55},
                      "volumeStabilizerState": {"enabled": true} } }"#;
        let (_, data) = import_json(text).unwrap();
        let PresetData::Mic(mic) = data else {
            panic!("expected a mic preset");
        };
        assert_eq!(mic.denoise, Some(0.9));
        assert_eq!(
            mic.gate,
            Some(GateConfig::Manual {
                threshold_db: -55.0
            })
        );
        assert!(mic.stabilizer);
    }

    #[test]
    fn rejects_unknown_and_invalid_files() {
        assert_eq!(import_json("{}"), Err(ImportError::UnknownFormat));
        assert!(matches!(import_json("nope"), Err(ImportError::Json(_))));
        let short = r#"{"format":"openseries-preset","version":1,"name":"x","kind":"headset_eq","settings":[1,2]}"#;
        assert!(matches!(import_json(short), Err(ImportError::Invalid(_))));
        let newer = r#"{"format":"openseries-preset","version":99,"name":"x","kind":"headset_eq","settings":[]}"#;
        assert_eq!(import_json(newer), Err(ImportError::TooNew(99)));
    }
}

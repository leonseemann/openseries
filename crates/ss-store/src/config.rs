//! The persisted configuration (schema version 1, `docs/architecture/app.md` §3).
//!
//! These are plain serde DTOs; [`crate::convert`] maps them to the serde-free domain types.
//! Every field has a default so older or partial files still load.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Current schema version.
pub const CONFIG_VERSION: u32 = 1;

/// The whole configuration file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Schema version of the file.
    pub version: u32,
    /// Playback mixing (Sonar's AudioMix).
    pub audiomix: AudioMixConfig,
    /// Virtual microphone and mic chain.
    pub mic: MicConfig,
    /// App-level settings.
    pub app: AppConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            audiomix: AudioMixConfig::default(),
            mic: MicConfig::default(),
            app: AppConfig::default(),
        }
    }
}

/// A playback channel as written in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// Game.
    Game,
    /// Chat.
    Chat,
    /// Media (also everything unassigned).
    Media,
    /// Aux.
    Aux,
}

/// AudioMix settings. The derived default has the master switch **off**.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioMixConfig {
    /// Master switch: when `false` no taps or aggregate devices exist (owner requirement).
    pub enabled: bool,
    /// Output device UID; `None` = system default output.
    pub output_device_uid: Option<String>,
    /// App bundle id (prefix) → channel. Unlisted apps play on Media.
    pub assignments: BTreeMap<String, Channel>,
    /// Per-channel settings.
    pub channels: Channels,
    /// Volume/mute of the summed output.
    pub master: FaderConfig,
}

/// The four channels.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Channels {
    /// Game channel.
    pub game: ChannelConfig,
    /// Chat channel.
    pub chat: ChannelConfig,
    /// Media channel.
    pub media: ChannelConfig,
    /// Aux channel.
    pub aux: ChannelConfig,
}

impl Channels {
    /// Settings of `channel`.
    #[must_use]
    pub const fn get(&self, channel: Channel) -> &ChannelConfig {
        match channel {
            Channel::Game => &self.game,
            Channel::Chat => &self.chat,
            Channel::Media => &self.media,
            Channel::Aux => &self.aux,
        }
    }

    /// Mutable settings of `channel`.
    pub const fn get_mut(&mut self, channel: Channel) -> &mut ChannelConfig {
        match channel {
            Channel::Game => &mut self.game,
            Channel::Chat => &mut self.chat,
            Channel::Media => &mut self.media,
            Channel::Aux => &mut self.aux,
        }
    }
}

/// Volume and mute.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FaderConfig {
    /// Linear volume 0..=1.
    pub volume: f32,
    /// Muted.
    pub muted: bool,
}

impl Default for FaderConfig {
    fn default() -> Self {
        Self {
            volume: 1.0,
            muted: false,
        }
    }
}

/// One playback channel: fader plus DSP strip.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelConfig {
    /// Volume/mute.
    pub fader: FaderConfig,
    /// Parametric EQ.
    pub eq: EqConfig,
    /// Bass/treble/clarity boosts in dB.
    pub boosts: BoostConfig,
    /// Smart Volume preset, `None` = off.
    pub smart_volume: Option<Loudness>,
    /// Extra gain in dB.
    pub gain_db: f32,
}

/// Smart Volume presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loudness {
    /// Gentle.
    Soft,
    /// Default.
    Balanced,
    /// Strong.
    Extreme,
}

/// Boost amounts in dB.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BoostConfig {
    /// Bass boost.
    pub bass_db: f32,
    /// Treble boost.
    pub treble_db: f32,
    /// Voice clarity boost.
    pub clarity_db: f32,
}

/// Parametric EQ. An empty `bands` list means the default flat bands.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EqConfig {
    /// Master switch of the EQ.
    pub enabled: bool,
    /// Up to 10 bands, lowest slot first.
    pub bands: Vec<BandConfig>,
}

/// One EQ band.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BandConfig {
    /// Band active.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// Sonar filter id 0–9 (7 = peaking).
    pub filter: u8,
    /// Frequency in Hz.
    pub frequency: f32,
    /// Gain in dB.
    pub gain_db: f32,
    /// Quality factor.
    pub q: f32,
}

const fn enabled_default() -> bool {
    true
}

/// Mic chain and virtual microphone.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MicConfig {
    /// Feed the virtual microphone.
    pub enabled: bool,
    /// Source microphone UID; `None` = system default input.
    pub input_device_uid: Option<String>,
    /// Denoise dry/wet mix, `None` = off.
    pub denoise: Option<f32>,
    /// Noise gate, `None` = off.
    pub gate: Option<GateConfig>,
    /// Volume stabilizer on/off.
    pub stabilizer: bool,
    /// Mic EQ.
    pub eq: EqConfig,
    /// Output gain in dB.
    pub gain_db: f32,
}

/// Noise-gate mode.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum GateConfig {
    /// Fixed threshold.
    Manual {
        /// dBFS.
        threshold_db: f32,
    },
    /// Noise floor + offset.
    Automatic {
        /// dB above the floor.
        offset_db: f32,
    },
}

/// App-level settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Start at login.
    pub autostart: bool,
    /// Reset the base station's USB connection automatically when the OS reports bursts of USB
    /// underflows (the crackling signature). Off by default.
    pub auto_fix_crackling: bool,
    /// Global hotkeys (`None` = unbound), e.g. `"Ctrl+Alt+M"`.
    pub hotkeys: HotkeysConfig,
}

/// Global hotkeys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeysConfig {
    /// Move ChatMix one step towards Game.
    pub chatmix_game: Option<String>,
    /// Move ChatMix one step towards Chat.
    pub chatmix_chat: Option<String>,
    /// Mute/unmute the virtual microphone.
    pub mic_mute: Option<String>,
}

impl Default for HotkeysConfig {
    fn default() -> Self {
        Self {
            chatmix_game: Some("Ctrl+Alt+Left".to_owned()),
            chatmix_chat: Some("Ctrl+Alt+Right".to_owned()),
            mic_mute: Some("Ctrl+Alt+M".to_owned()),
        }
    }
}

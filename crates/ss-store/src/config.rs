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
    /// Saved sound presets.
    pub presets: crate::presets::PresetsConfig,
}

impl Config {
    /// Factory settings of the app (DEV-12, app part): everything back to defaults except the
    /// saved presets, which are the user's own data.
    #[must_use]
    pub fn reset_keeping_presets(&self) -> Self {
        Self {
            presets: self.presets.clone(),
            ..Self::default()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            audiomix: AudioMixConfig::default(),
            mic: MicConfig::default(),
            app: AppConfig::default(),
            presets: crate::presets::PresetsConfig::default(),
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
    /// Backup outputs in priority order, used when the selected one is missing (SON-27/28).
    pub output_fallback: Vec<String>,
    /// Treat the headset's own devices as missing while the headset is turned off, so output and
    /// mic switch to their backups (SON-28 "wireless trigger").
    pub fallback_when_headset_off: bool,
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
    /// Backup microphones in priority order (SON-27/28).
    pub input_fallback: Vec<String>,
    /// While the feed runs, make "StealSeries Microphone" the system default input and restore
    /// the previous default afterwards (SON-06).
    pub set_as_default_input: bool,
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
    /// Desktop notifications (CORE-15).
    pub notifications: NotificationsConfig,
    /// The "Getting started" window was closed with Done.
    pub onboarding_done: bool,
    /// In-app updates (ADR 0006).
    pub updates: UpdatesConfig,
}

/// In-app updates (ADR 0006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdatesConfig {
    /// Look for a new release shortly after start and every few hours.
    pub check_automatically: bool,
}

impl Default for UpdatesConfig {
    fn default() -> Self {
        Self {
            check_automatically: true,
        }
    }
}

/// Desktop notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationsConfig {
    /// Show OS notifications (headset/base station connection, low battery, audio errors).
    pub enabled: bool,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
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
    /// Reconnect the base station's USB connection (cures crackling).
    pub reconnect_usb: Option<String>,
}

impl Default for HotkeysConfig {
    fn default() -> Self {
        Self {
            chatmix_game: Some("Ctrl+Alt+Left".to_owned()),
            chatmix_chat: Some("Ctrl+Alt+Right".to_owned()),
            mic_mute: Some("Ctrl+Alt+M".to_owned()),
            reconnect_usb: Some("Ctrl+Alt+R".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_restores_defaults_but_keeps_presets() {
        let mut config = Config::default();
        config.audiomix.enabled = true;
        config.app.autostart = true;
        config.app.onboarding_done = true;
        config.mic.gain_db = 6.0;
        config
            .presets
            .headset_eq
            .insert("Mine".to_owned(), vec![1.0; 10]);
        let reset = config.reset_keeping_presets();
        assert_eq!(reset.presets, config.presets);
        assert_eq!(
            Config {
                presets: crate::presets::PresetsConfig::default(),
                ..reset
            },
            Config::default()
        );
    }
}

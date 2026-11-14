//! Command-line definitions and their mapping to domain values.

use clap::{Parser, Subcommand, ValueEnum};
use ss_core::headset::{AutoOff, EqPreset, NoiseControl, SidetoneLevel};

/// Control a SteelSeries Arctis Nova Pro Wireless without vendor software.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// What to do.
    #[command(subcommand)]
    pub action: Action,
}

/// Top-level actions.
#[derive(Debug, Subcommand)]
pub enum Action {
    /// List matching HID collections (no data is sent to the device).
    List,
    /// Show connection, battery, noise control and other status.
    Status,
    /// Show headset and spare battery levels.
    Battery,
    /// Change a setting.
    Set {
        /// Setting and value.
        #[command(subcommand)]
        setting: Setting,
        /// Persist the change to the base station's flash memory.
        #[arg(long)]
        save: bool,
    },
    /// Hex-dump the raw reply of a read-only query (protocol exploration).
    Raw {
        /// Which block to query.
        block: RawBlock,
    },
    /// Print device events (dial, buttons, battery) until Ctrl-C.
    Watch {
        /// Let the dial report ChatMix positions (restored to normal on exit).
        #[arg(long)]
        chatmix_dial: bool,
    },
}

/// Settings that can be changed.
#[derive(Debug, Subcommand)]
pub enum Setting {
    /// Noise-control mode.
    NoiseControl {
        /// Mode.
        mode: NoiseControlArg,
    },
    /// Transparency strength (1–10).
    Transparency {
        /// Level.
        level: u8,
    },
    /// Sidetone level.
    Sidetone {
        /// Level.
        level: SidetoneArg,
    },
    /// Hardware microphone volume (1–10).
    MicVolume {
        /// Level.
        level: u8,
    },
    /// Hardware EQ preset.
    EqPreset {
        /// Preset.
        preset: EqPresetArg,
    },
    /// Custom 10-band hardware EQ in dB (−10…10, 0.5 dB steps); also selects the custom preset.
    Eq {
        /// Ten band gains from low to high frequency.
        #[arg(num_args = 10, allow_negative_numbers = true)]
        gains: Vec<f32>,
    },
    /// Inactivity auto-off.
    AutoOff {
        /// Timeout.
        timeout: AutoOffArg,
    },
    /// Base-station OLED brightness (1–10).
    OledBrightness {
        /// Level.
        level: u8,
    },
    /// Mic-mute LED brightness (1–10).
    MicLed {
        /// Level.
        level: u8,
    },
    /// ChatMix position held by the base station (0–100 each; **Inferred** opcode, test only).
    Chatmix {
        /// Game level.
        game: u8,
        /// Chat level.
        chat: u8,
    },
}

/// Read-only query blocks that `ssctl raw` may request.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RawBlock {
    /// General status block (0xB0).
    Status,
    /// Audio settings block (0x20).
    Settings,
    /// Display settings block (0x80).
    Display,
    /// Battery block (0xB7).
    Battery,
}

/// CLI spelling of [`NoiseControl`].
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum NoiseControlArg {
    /// No processing.
    Off,
    /// Hear-through.
    Transparency,
    /// Active noise cancellation.
    Anc,
}

impl From<NoiseControlArg> for NoiseControl {
    fn from(arg: NoiseControlArg) -> Self {
        match arg {
            NoiseControlArg::Off => Self::Off,
            NoiseControlArg::Transparency => Self::Transparency,
            NoiseControlArg::Anc => Self::NoiseCancelling,
        }
    }
}

/// CLI spelling of [`SidetoneLevel`].
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SidetoneArg {
    /// Off.
    Off,
    /// Low.
    Low,
    /// Medium.
    Medium,
    /// High.
    High,
}

impl From<SidetoneArg> for SidetoneLevel {
    fn from(arg: SidetoneArg) -> Self {
        match arg {
            SidetoneArg::Off => Self::Off,
            SidetoneArg::Low => Self::Low,
            SidetoneArg::Medium => Self::Medium,
            SidetoneArg::High => Self::High,
        }
    }
}

/// CLI spelling of [`EqPreset`].
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum EqPresetArg {
    /// Flat.
    Flat,
    /// Bass boost.
    BassBoost,
    /// Focus.
    Focus,
    /// Smiley (V-shape).
    Smiley,
    /// Custom curve.
    Custom,
}

impl From<EqPresetArg> for EqPreset {
    fn from(arg: EqPresetArg) -> Self {
        match arg {
            EqPresetArg::Flat => Self::Flat,
            EqPresetArg::BassBoost => Self::BassBoost,
            EqPresetArg::Focus => Self::Focus,
            EqPresetArg::Smiley => Self::Smiley,
            EqPresetArg::Custom => Self::Custom,
        }
    }
}

/// CLI spelling of [`AutoOff`] (minutes).
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AutoOffArg {
    /// Never.
    Never,
    /// 1 minute.
    #[value(name = "1")]
    Min1,
    /// 5 minutes.
    #[value(name = "5")]
    Min5,
    /// 10 minutes.
    #[value(name = "10")]
    Min10,
    /// 15 minutes.
    #[value(name = "15")]
    Min15,
    /// 30 minutes.
    #[value(name = "30")]
    Min30,
    /// 60 minutes.
    #[value(name = "60")]
    Min60,
}

impl From<AutoOffArg> for AutoOff {
    fn from(arg: AutoOffArg) -> Self {
        match arg {
            AutoOffArg::Never => Self::Never,
            AutoOffArg::Min1 => Self::After1Min,
            AutoOffArg::Min5 => Self::After5Min,
            AutoOffArg::Min10 => Self::After10Min,
            AutoOffArg::Min15 => Self::After15Min,
            AutoOffArg::Min30 => Self::After30Min,
            AutoOffArg::Min60 => Self::After60Min,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_negative_eq_gains() {
        let cli = Cli::try_parse_from([
            "ssctl", "set", "eq", "-3", "-1.5", "0", "0", "0", "0", "0", "0", "2", "4",
        ])
        .unwrap();
        let Action::Set {
            setting: Setting::Eq { gains },
            save: false,
        } = cli.action
        else {
            panic!("unexpected parse result");
        };
        assert_eq!(gains.len(), 10);
        assert!((gains[1] + 1.5).abs() < f32::EPSILON);
    }
}

//! Command-line definitions.

use clap::{Parser, Subcommand};

/// AudioMix demo: per-app channels, DSP and the Nova Pro ChatMix dial.
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
    /// List output devices.
    Devices,
    /// List processes known to the audio system (with PID and bundle id).
    Apps,
    /// Start mixing until Ctrl-C.
    Run(RunArgs),
    /// List input devices (microphones).
    Inputs,
    /// Feed a microphone into the virtual "StealSeries Microphone" until Ctrl-C.
    Mic(MicArgs),
}

/// Options of `ssmix mic`.
#[derive(Debug, clap::Args)]
pub struct MicArgs {
    /// Source microphone (substring of its name); default: current system input.
    #[arg(long)]
    pub input: Option<String>,
    /// Gain applied to the microphone in dB (end of the mic chain).
    #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
    pub gain_db: f32,
    /// RNNoise denoise with this dry/wet mix 0–1 (flag alone = 0.9, Sonar's default).
    #[arg(long, value_name = "MIX", num_args = 0..=1, default_missing_value = "0.9")]
    pub denoise: Option<f32>,
    /// Noise gate with a fixed threshold in dBFS (e.g. -50).
    #[arg(long, value_name = "DBFS", allow_negative_numbers = true)]
    pub gate: Option<f32>,
    /// Noise gate with automatic threshold = noise floor + OFFSET dB (flag alone = 10).
    #[arg(long, value_name = "OFFSET", num_args = 0..=1, default_missing_value = "10", conflicts_with = "gate")]
    pub auto_gate: Option<f32>,
    /// Volume stabilizer (voice compressor).
    #[arg(long)]
    pub stabilizer: bool,
    /// Mic EQ band `FREQ:GAIN_DB:Q[:TYPE]` (TYPE = Sonar filter id, default 7). Repeatable, max 10.
    #[arg(long = "band", value_name = "SPEC")]
    pub bands: Vec<String>,
    /// Print the aggregate stream layout and the mic/feed peaks every second.
    #[arg(long)]
    pub probe: bool,
}

/// Options of `ssmix run`. Apps are given as PID or bundle id (e.g. `com.hnc.Discord`).
#[derive(Debug, clap::Args)]
pub struct RunArgs {
    /// Output device (substring of its name); default: current system output.
    #[arg(long)]
    pub output: Option<String>,
    /// Apps for the Game channel.
    #[arg(long, value_name = "APP")]
    pub game: Vec<String>,
    /// Apps for the Chat channel.
    #[arg(long, value_name = "APP")]
    pub chat: Vec<String>,
    /// Apps for the Aux channel. All other apps go to Media.
    #[arg(long, value_name = "APP")]
    pub aux: Vec<String>,
    /// Follow the Arctis Nova Pro ChatMix dial.
    #[arg(long)]
    pub dial: bool,
    /// Bass boost on the Game channel in dB (0–12).
    #[arg(long, default_value_t = 0.0)]
    pub game_bass: f32,
    /// Enable Smart Volume (balanced) on the Media channel.
    #[arg(long)]
    pub media_smart_volume: bool,
    /// Print the aggregate stream layout and per-buffer input peaks every second.
    #[arg(long)]
    pub probe: bool,
}

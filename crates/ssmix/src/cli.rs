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

//! `ssmix` — AudioMix demo for macOS 14.4+.
//!
//! Routes apps into Game/Chat/Media/Aux, applies the DSP chain and follows the Arctis Nova Pro
//! ChatMix dial. On other operating systems it only explains that it is macOS-only.

#[cfg(target_os = "macos")]
mod cli;
#[cfg(all(target_os = "macos", feature = "nova-pro-dial"))]
mod dial;
#[cfg(target_os = "macos")]
mod run;

#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    use clap::Parser;
    run::execute(cli::Cli::parse())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("ssmix currently supports macOS 14.4+ only (Windows backend: milestone M4).");
    std::process::exit(2);
}

//! M4 spike: can WASAPI process loopback replace macOS process taps on Windows?
//!
//! Throwaway measurement tool for [ADR 0004](../../../docs/adr/0004-windows-audio-strategy.md).
//! The key unknown: macOS taps can mute the tapped app ("muted when tapped"); WASAPI has no such
//! flag. If process loopback still delivers an app's audio while that app's *session* is muted or
//! at volume 0, AudioMix works on Windows without a driver. `capture` measures exactly that.
//!
//! ```text
//! winspike endpoints                 # render/capture endpoints (is there a Nova Pro "chat" output?)
//! winspike sessions                  # audio sessions: pid, exe, state, volume, mute, peak
//! winspike policy                    # per-app default render endpoint (read-only)
//! winspike capture --name spotify.exe --silence mute --seconds 30 [--wav out.wav]
//! ```
//!
//! `capture` runs three equal phases: normal → session silenced → restored. Every second it prints
//! the loopback peak (what we would get) and the session meter peak (what Windows plays).

#[cfg(windows)]
mod win;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    win::run(&std::env::args().skip(1).collect::<Vec<_>>())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("winspike only runs on Windows");
}

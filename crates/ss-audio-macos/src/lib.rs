//! macOS backend of AudioMix ([ADR 0002](../../../docs/adr/0002-macos-audio-strategy.md)).
//!
//! * [`plan`] — OS-independent: turns app→channel assignments into a list of process taps.
//! * `macos` (macOS only) — CoreAudio FFI: process taps, a private aggregate device and the
//!   IOProc that feeds [`ss_engine::MixEngine`].
//!
//! On other operating systems only the portable planning code is compiled, so the workspace
//! builds and tests everywhere.

// FFI adapter crate: `unsafe` is allowed here only (AGENTS.md). Every block has a SAFETY note.
#![allow(unsafe_code)]

mod error;
pub mod plan;

#[cfg(target_os = "macos")]
mod macos;

pub use error::AudioError;
#[cfg(target_os = "macos")]
pub use macos::{
    AudioMixSession, AudioProcess, MAX_PROBED_BUFFERS, OutputDevice, ProbeSnapshot, StreamLayout,
    default_output_device, list_audio_processes, list_output_devices,
};

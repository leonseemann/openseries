//! macOS backend of AudioMix ([ADR 0002](../../../docs/adr/0002-macos-audio-strategy.md)).
//!
//! * [`plan`] — OS-independent: turns app→channel assignments into a list of process taps.
//! * `macos` (macOS only) — CoreAudio FFI: process taps, a private aggregate device and the
//!   IOProc that feeds [`ss_engine::MixEngine`]; plus the mic feed into the virtual microphone
//!   of `ss-hal-driver-macos`.
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
    AudioMixSession, AudioProcess, FEED_DEVICE_UID, InputDevice, MAX_PROBED_BUFFERS,
    MicFeedSession, OutputDevice, ProbeSnapshot, StreamLayout, VIRTUAL_MIC_UID,
    default_input_device, default_output_device, device_id_for_uid, list_audio_processes,
    list_input_devices, list_output_devices, set_default_input_device,
};

//! Virtual microphone driver for macOS (M3b, [design](../../../docs/architecture/virtual-mic-driver.md)).
//!
//! A loopback `AudioServerPlugIn` with two devices sharing one ring buffer: the hidden
//! "OpenSeries Microphone Feed" (the daemon writes the processed mic into it) and the visible
//! "OpenSeries Microphone" (apps read from it). No DSP and no IPC live here.
//!
//! * Portable, tested everywhere: [`selectors`], [`ring`], [`clock`], [`model`], [`driver`].
//! * macOS only: `ffi` — the C ABI entry points loaded by `coreaudiod`.

// FFI adapter crate: `unsafe` is allowed here only (AGENTS.md). Every block has a SAFETY note.
#![allow(unsafe_code)]

pub mod clock;
pub mod driver;
pub mod model;
pub mod ring;
pub mod selectors;

#[cfg(target_os = "macos")]
mod ffi;

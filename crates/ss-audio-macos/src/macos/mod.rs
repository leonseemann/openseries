//! CoreAudio FFI (macOS 14.4+). See `docs/architecture/audio-engine.md` §4.

mod aggregate;
mod buffers;
mod device;
mod process;
mod property;
mod session;
mod tap;

pub use device::{OutputDevice, default_output_device, list_output_devices};
pub use process::{AudioProcess, list_audio_processes};
pub use session::AudioMixSession;

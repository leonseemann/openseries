//! CoreAudio FFI (macOS 14.4+). See `docs/architecture/audio-engine.md` §4.

mod aggregate;
mod buffers;
mod device;
mod probe;
mod process;
mod property;
mod session;
mod tap;

pub use device::{OutputDevice, default_output_device, list_output_devices};
pub use probe::{MAX_PROBED_BUFFERS, ProbeSnapshot};
pub use process::{AudioProcess, list_audio_processes};
pub use session::{AudioMixSession, StreamLayout};

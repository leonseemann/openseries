//! CoreAudio FFI (macOS 14.4+). See `docs/architecture/audio-engine.md` §4.

mod aggregate;
mod buffers;
mod device;
mod mic_feed;
mod probe;
mod process;
mod property;
mod session;
mod stream_usage;
mod tap;

pub use device::{
    InputDevice, OutputDevice, default_input_device, default_output_device, device_id_for_uid,
    list_input_devices, list_output_devices,
};
pub use mic_feed::{FEED_DEVICE_UID, MicFeedSession, VIRTUAL_MIC_UID};
pub use probe::{MAX_PROBED_BUFFERS, ProbeSnapshot};
pub use process::{AudioProcess, list_audio_processes};
pub use session::{AudioMixSession, StreamLayout};

//! Switches individual aggregate-device streams off for one IOProc.
//!
//! Used to keep the headset microphone closed during playback mixing: the aggregate device
//! contains the headset's input stream, and running it makes macOS ask for microphone access
//! although AudioMix playback never reads it (`docs/hardware/macos-audiomix.md`, check 6).

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_core_audio::{
    AudioDeviceIOProcID, AudioObjectID, AudioObjectPropertyAddress, AudioObjectPropertyScope,
    AudioObjectSetPropertyData, kAudioDevicePropertyIOProcStreamUsage,
    kAudioObjectPropertyElementMain,
};

use crate::AudioError;

/// Most streams per scope this module can describe (an aggregate has 1 mic + ≤ 4 taps).
const MAX_STREAMS: usize = 16;

/// `AudioHardwareIOProcStreamUsage` with a fixed-capacity stream array. CoreAudio reads only
/// `mNumberStreams` entries, so the size passed is trimmed accordingly.
#[repr(C)]
struct StreamUsage {
    io_proc: *mut c_void,
    number_streams: u32,
    stream_is_on: [u32; MAX_STREAMS],
}

/// Enables only the streams for which `enabled(index)` is true in `scope` of `device`
/// for the IOProc `proc_id`. Must be called before the IOProc is started.
///
/// # Errors
/// [`AudioError::Unsupported`] for more than [`MAX_STREAMS`] streams or a missing IOProc,
/// [`AudioError::Os`] if CoreAudio rejects the setting.
pub fn set_stream_usage(
    device: AudioObjectID,
    proc_id: AudioDeviceIOProcID,
    scope: AudioObjectPropertyScope,
    streams: usize,
    enabled: impl Fn(usize) -> bool,
) -> Result<(), AudioError> {
    if streams > MAX_STREAMS {
        return Err(AudioError::Unsupported(format!(
            "{streams} streams, at most {MAX_STREAMS} supported"
        )));
    }
    let io_proc = proc_id
        .map(|f| f as *mut c_void)
        .ok_or_else(|| AudioError::Unsupported("IOProc not created".to_owned()))?;
    let mut usage = StreamUsage {
        io_proc,
        number_streams: u32::try_from(streams).unwrap_or(u32::MAX),
        stream_is_on: [0; MAX_STREAMS],
    };
    for (index, on) in usage.stream_is_on.iter_mut().enumerate().take(streams) {
        *on = u32::from(enabled(index));
    }

    let size = std::mem::offset_of!(StreamUsage, stream_is_on) + streams * size_of::<u32>();
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyIOProcStreamUsage,
        mScope: scope,
        mElement: kAudioObjectPropertyElementMain,
    };
    // SAFETY: `usage` is a valid `AudioHardwareIOProcStreamUsage` prefix of `size` bytes
    // (`size` ≤ size_of::<StreamUsage>()); `address` lives for the call; no qualifier.
    let status = unsafe {
        AudioObjectSetPropertyData(
            device,
            NonNull::from(&address),
            0,
            std::ptr::null(),
            u32::try_from(size).unwrap_or(u32::MAX),
            NonNull::from(&usage).cast(),
        )
    };
    AudioError::check(status, "AudioObjectSetPropertyData(IOProcStreamUsage)")
}

//! Processes known to the audio system (for the per-app routing UI).

use objc2_core_audio::{
    AudioObjectID, kAudioHardwarePropertyProcessObjectList,
    kAudioHardwarePropertyTranslatePIDToProcessObject, kAudioProcessPropertyBundleID,
    kAudioProcessPropertyIsRunningOutput, kAudioProcessPropertyPID,
};

use super::property::{self, GLOBAL, SYSTEM_OBJECT};
use crate::AudioError;
use crate::plan::Pid;

/// "No object" sentinel returned by PID translation.
const UNKNOWN_OBJECT: AudioObjectID = 0;

/// A process that has opened audio I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProcess {
    /// CoreAudio process object.
    pub object: AudioObjectID,
    /// Unix process id.
    pub pid: Pid,
    /// Bundle identifier, if the process is an app bundle.
    pub bundle_id: Option<String>,
    /// Whether the process is currently playing audio.
    pub is_playing: bool,
}

/// All processes the audio system knows about.
///
/// # Errors
/// [`AudioError`] if the process list cannot be read.
pub fn list_audio_processes() -> Result<Vec<AudioProcess>, AudioError> {
    let objects: Vec<AudioObjectID> = property::get_vec(
        SYSTEM_OBJECT,
        kAudioHardwarePropertyProcessObjectList,
        GLOBAL,
    )?;
    Ok(objects
        .into_iter()
        .filter_map(|object| {
            let pid =
                property::get::<Pid, ()>(object, kAudioProcessPropertyPID, GLOBAL, None).ok()?;
            let playing = property::get::<u32, ()>(
                object,
                kAudioProcessPropertyIsRunningOutput,
                GLOBAL,
                None,
            )
            .is_ok_and(|v| v != 0);
            Some(AudioProcess {
                object,
                pid,
                bundle_id: property::get_string(object, kAudioProcessPropertyBundleID).ok(),
                is_playing: playing,
            })
        })
        .collect())
}

/// Translates a PID into its CoreAudio process object, `None` if it has no audio I/O.
///
/// # Errors
/// [`AudioError`] if the translation call fails.
pub(super) fn process_object(pid: Pid) -> Result<Option<AudioObjectID>, AudioError> {
    let object: AudioObjectID = property::get(
        SYSTEM_OBJECT,
        kAudioHardwarePropertyTranslatePIDToProcessObject,
        GLOBAL,
        Some(&pid),
    )?;
    Ok((object != UNKNOWN_OBJECT).then_some(object))
}

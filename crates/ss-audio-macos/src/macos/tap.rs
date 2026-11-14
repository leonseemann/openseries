//! Audio process taps (macOS 14.4+).

use objc2::AnyThread;
use objc2_core_audio::{
    AudioHardwareCreateProcessTap, AudioHardwareDestroyProcessTap, AudioObjectID, CATapDescription,
    CATapMuteBehavior,
};
use objc2_foundation::{NSArray, NSNumber};

use super::process::process_object;
use crate::AudioError;
use crate::plan::{Pid, TapSource, TapSpec};

/// A live process tap; destroyed on drop.
#[derive(Debug)]
pub struct ProcessTap {
    id: AudioObjectID,
    uid: String,
}

impl ProcessTap {
    /// Creates a private, stereo tap that mutes the tapped processes' own output.
    ///
    /// # Errors
    /// [`AudioError::NotFound`] if an `Include` tap has no process with audio I/O,
    /// [`AudioError::Os`] if CoreAudio refuses the tap (e.g. missing permission).
    pub fn create(spec: &TapSpec) -> Result<Self, AudioError> {
        let (pids, include) = match &spec.source {
            TapSource::Include(pids) => (pids, true),
            TapSource::AllExcept(pids) => (pids, false),
        };
        let objects = process_objects(pids)?;
        if include && objects.is_empty() {
            return Err(AudioError::NotFound(format!(
                "no audio process for {:?} ({pids:?})",
                spec.channel
            )));
        }
        let numbers: Vec<_> = objects
            .iter()
            .map(|&id| NSNumber::numberWithUnsignedInt(id))
            .collect();
        let processes = NSArray::from_retained_slice(&numbers);

        // SAFETY: standard Objective-C alloc/init of `CATapDescription` with a valid NSArray.
        let description = unsafe {
            if include {
                CATapDescription::initStereoMixdownOfProcesses(
                    CATapDescription::alloc(),
                    &processes,
                )
            } else {
                CATapDescription::initStereoGlobalTapButExcludeProcesses(
                    CATapDescription::alloc(),
                    &processes,
                )
            }
        };
        // SAFETY: plain property setters on a valid object.
        unsafe {
            description.setPrivate(true);
            description.setMuteBehavior(CATapMuteBehavior::MutedWhenTapped);
        }

        let mut id: AudioObjectID = 0;
        // SAFETY: `description` is valid and `id` is a valid out-pointer.
        let status = unsafe { AudioHardwareCreateProcessTap(Some(&description), &raw mut id) };
        AudioError::check(status, "AudioHardwareCreateProcessTap")?;
        // SAFETY: `UUID` is a plain getter on a valid object.
        let uid = unsafe { description.UUID() }.UUIDString().to_string();
        Ok(Self { id, uid })
    }

    /// Tap UID used in the aggregate device's tap list.
    #[must_use]
    pub fn uid(&self) -> &str {
        &self.uid
    }
}

impl Drop for ProcessTap {
    fn drop(&mut self) {
        // SAFETY: `id` was returned by AudioHardwareCreateProcessTap and is destroyed once.
        // Errors cannot be reported from drop; the HAL cleans up when the process exits anyway.
        let _ = unsafe { AudioHardwareDestroyProcessTap(self.id) };
    }
}

/// Resolves PIDs to process objects, skipping processes without audio I/O.
fn process_objects(pids: &[Pid]) -> Result<Vec<AudioObjectID>, AudioError> {
    let mut objects = Vec::with_capacity(pids.len());
    for &pid in pids {
        if let Some(object) = process_object(pid)? {
            objects.push(object);
        }
    }
    Ok(objects)
}

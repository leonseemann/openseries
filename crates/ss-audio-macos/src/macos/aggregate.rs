//! Private aggregate device combining the headset output with process taps.

use std::ffi::CStr;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_core_audio::{
    AudioHardwareCreateAggregateDevice, AudioHardwareDestroyAggregateDevice, AudioObjectID,
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceMainSubDeviceKey,
    kAudioAggregateDeviceNameKey, kAudioAggregateDeviceSubDeviceListKey,
    kAudioAggregateDeviceTapAutoStartKey, kAudioAggregateDeviceTapListKey,
    kAudioAggregateDeviceUIDKey, kAudioSubDeviceUIDKey, kAudioSubTapDriftCompensationKey,
    kAudioSubTapUIDKey,
};
use objc2_core_foundation::CFDictionary;
use objc2_foundation::{NSArray, NSMutableDictionary, NSNumber, NSString, NSUUID};

use super::device::OutputDevice;
use super::tap::ProcessTap;
use crate::AudioError;

/// Name shown for the (private, normally invisible) aggregate device.
const DEVICE_NAME: &str = "StealSeries AudioMix";
/// Prefix of the aggregate device UID; a random suffix keeps sessions unique.
const UID_PREFIX: &str = "dev.stealseries.audiomix.";

/// An aggregate device owned by this process; destroyed on drop.
#[derive(Debug)]
pub struct AggregateDevice {
    id: AudioObjectID,
}

impl AggregateDevice {
    /// Creates a private aggregate device: `output` as main sub-device plus all `taps`.
    ///
    /// # Errors
    /// [`AudioError::Os`] if CoreAudio rejects the description.
    pub fn create(output: &OutputDevice, taps: &[ProcessTap]) -> Result<Self, AudioError> {
        let description = Dictionary::new();
        description.set(
            kAudioAggregateDeviceNameKey,
            &NSString::from_str(DEVICE_NAME),
        );
        let uid = format!("{UID_PREFIX}{}", NSUUID::new().UUIDString());
        description.set(kAudioAggregateDeviceUIDKey, &NSString::from_str(&uid));
        description.set(
            kAudioAggregateDeviceIsPrivateKey,
            &NSNumber::numberWithBool(true),
        );
        description.set(
            kAudioAggregateDeviceTapAutoStartKey,
            &NSNumber::numberWithBool(true),
        );
        description.set(
            kAudioAggregateDeviceMainSubDeviceKey,
            &NSString::from_str(&output.uid),
        );

        let sub_device = Dictionary::new();
        sub_device.set(kAudioSubDeviceUIDKey, &NSString::from_str(&output.uid));
        description.set(
            kAudioAggregateDeviceSubDeviceListKey,
            &NSArray::from_retained_slice(&[sub_device.0]),
        );

        let tap_entries: Vec<Retained<NSMutableDictionary<NSString, AnyObject>>> = taps
            .iter()
            .map(|tap| {
                let entry = Dictionary::new();
                entry.set(kAudioSubTapUIDKey, &NSString::from_str(tap.uid()));
                entry.set(
                    kAudioSubTapDriftCompensationKey,
                    &NSNumber::numberWithBool(true),
                );
                entry.0
            })
            .collect();
        description.set(
            kAudioAggregateDeviceTapListKey,
            &NSArray::from_retained_slice(&tap_entries),
        );

        let mut id: AudioObjectID = 0;
        let cf_description = description.as_cf();
        // SAFETY: `cf_description` is a valid dictionary for the duration of the call and
        // `id` is a valid out-pointer.
        let status = unsafe {
            AudioHardwareCreateAggregateDevice(cf_description, std::ptr::NonNull::from(&mut id))
        };
        AudioError::check(status, "AudioHardwareCreateAggregateDevice")?;
        Ok(Self { id })
    }

    /// CoreAudio device id of the aggregate device.
    #[must_use]
    pub const fn id(&self) -> AudioObjectID {
        self.id
    }
}

impl Drop for AggregateDevice {
    fn drop(&mut self) {
        // SAFETY: `id` was created by this process and is destroyed exactly once.
        let _ = unsafe { AudioHardwareDestroyAggregateDevice(self.id) };
    }
}

/// Small wrapper for building heterogeneous `NSDictionary`s keyed by CoreAudio C-string keys.
struct Dictionary(Retained<NSMutableDictionary<NSString, AnyObject>>);

impl Dictionary {
    fn new() -> Self {
        Self(NSMutableDictionary::new())
    }

    fn set(&self, key: &CStr, value: &AnyObject) {
        let key = NSString::from_str(&key.to_string_lossy());
        self.0.insert(&*key, value);
    }

    /// Views the dictionary as a `CFDictionary` (toll-free bridged).
    fn as_cf(&self) -> &CFDictionary {
        // SAFETY: NSDictionary and CFDictionary are toll-free bridged; the reference lives
        // as long as `self`.
        unsafe { &*Retained::as_ptr(&self.0).cast::<CFDictionary>() }
    }
}

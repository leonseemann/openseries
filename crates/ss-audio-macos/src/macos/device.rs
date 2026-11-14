//! Physical output devices.

use std::ptr::NonNull;

use objc2_core_audio::{
    AudioObjectID, AudioObjectPropertyScope, kAudioDevicePropertyDeviceUID,
    kAudioDevicePropertyNominalSampleRate, kAudioDevicePropertyStreamConfiguration,
    kAudioHardwarePropertyDefaultOutputDevice, kAudioHardwarePropertyDevices,
    kAudioObjectPropertyName, kAudioObjectPropertyScopeInput, kAudioObjectPropertyScopeOutput,
};
use objc2_core_audio_types::AudioBufferList;

use super::buffers;
use super::property::{self, GLOBAL, SYSTEM_OBJECT};
use crate::AudioError;

/// An audio device that can play sound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputDevice {
    /// CoreAudio object id (valid while the device is connected).
    pub id: AudioObjectID,
    /// Persistent unique id.
    pub uid: String,
    /// Display name.
    pub name: String,
    /// Number of output channels.
    pub output_channels: usize,
}

impl OutputDevice {
    fn load(id: AudioObjectID) -> Result<Self, AudioError> {
        Ok(Self {
            id,
            uid: property::get_string(id, kAudioDevicePropertyDeviceUID)?,
            name: property::get_string(id, kAudioObjectPropertyName)?,
            output_channels: channel_count(id, kAudioObjectPropertyScopeOutput)?,
        })
    }

    /// Current sample rate in Hz.
    ///
    /// # Errors
    /// [`AudioError`] if the property cannot be read.
    pub fn sample_rate(&self) -> Result<f64, AudioError> {
        property::get::<f64, ()>(self.id, kAudioDevicePropertyNominalSampleRate, GLOBAL, None)
    }

    /// Number of input channels (e.g. the headset microphone).
    ///
    /// # Errors
    /// [`AudioError`] if the stream configuration cannot be read.
    pub fn input_channels(&self) -> Result<usize, AudioError> {
        channel_count(self.id, kAudioObjectPropertyScopeInput)
    }
}

/// Total channel count of all streams of `device` in `scope`.
pub(super) fn channel_count(
    device: AudioObjectID,
    scope: AudioObjectPropertyScope,
) -> Result<usize, AudioError> {
    let raw = property::get_raw(device, kAudioDevicePropertyStreamConfiguration, scope)?;
    let list = NonNull::new(raw.as_ptr().cast_mut().cast::<AudioBufferList>())
        .ok_or_else(|| AudioError::Unsupported("empty stream configuration".to_owned()))?;
    // SAFETY: `raw` holds the complete, aligned AudioBufferList returned by CoreAudio and
    // lives until the end of this function.
    let streams = unsafe { buffers::buffers(list) };
    Ok(streams.iter().map(|b| b.mNumberChannels as usize).sum())
}

/// All devices with at least one output channel.
///
/// # Errors
/// [`AudioError`] if the device list cannot be read.
pub fn list_output_devices() -> Result<Vec<OutputDevice>, AudioError> {
    let ids: Vec<AudioObjectID> =
        property::get_vec(SYSTEM_OBJECT, kAudioHardwarePropertyDevices, GLOBAL)?;
    Ok(ids
        .into_iter()
        .filter_map(|id| OutputDevice::load(id).ok())
        .filter(|device| device.output_channels > 0)
        .collect())
}

/// The system's current default output device.
///
/// # Errors
/// [`AudioError`] if no default output exists or it cannot be read.
pub fn default_output_device() -> Result<OutputDevice, AudioError> {
    let id: AudioObjectID = property::get::<AudioObjectID, ()>(
        SYSTEM_OBJECT,
        kAudioHardwarePropertyDefaultOutputDevice,
        GLOBAL,
        None,
    )?;
    OutputDevice::load(id)
}

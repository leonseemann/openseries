//! Typed access to CoreAudio object properties.

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_core_audio::{
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID,
    AudioObjectPropertyAddress, AudioObjectPropertyScope, AudioObjectPropertySelector,
    AudioObjectSetPropertyData, kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal,
    kAudioObjectSystemObject,
};
use objc2_core_foundation::{CFRetained, CFString};

use crate::AudioError;

/// The system object (`kAudioObjectSystemObject`).
#[allow(clippy::cast_sign_loss)] // the constant is 1
pub const SYSTEM_OBJECT: AudioObjectID = kAudioObjectSystemObject as AudioObjectID;

/// Global scope, main element.
pub const GLOBAL: AudioObjectPropertyScope = kAudioObjectPropertyScopeGlobal;

fn address(
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: kAudioObjectPropertyElementMain,
    }
}

/// Size in bytes of a property value.
pub fn size(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Result<u32, AudioError> {
    let address = address(selector, scope);
    let mut size = 0_u32;
    // SAFETY: `address` and `size` are valid for the duration of the call; no qualifier.
    let status = unsafe {
        AudioObjectGetPropertyDataSize(
            object,
            NonNull::from(&address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
        )
    };
    AudioError::check(status, "AudioObjectGetPropertyDataSize")?;
    Ok(size)
}

/// Reads a fixed-size property, optionally with a qualifier.
pub fn get<T: Copy + Default, Q>(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
    qualifier: Option<&Q>,
) -> Result<T, AudioError> {
    let address = address(selector, scope);
    let mut value = T::default();
    let mut size = u32::try_from(size_of::<T>()).unwrap_or(u32::MAX);
    let (qualifier_size, qualifier_ptr) = qualifier.map_or((0, std::ptr::null()), |q| {
        (
            u32::try_from(size_of::<Q>()).unwrap_or(0),
            std::ptr::from_ref(q).cast::<c_void>(),
        )
    });
    // SAFETY: `value` provides `size` writable bytes of a `Copy` type; the qualifier pointer is
    // either null with size 0 or points to a live `Q` of the stated size.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&address),
            qualifier_size,
            qualifier_ptr,
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast(),
        )
    };
    AudioError::check(status, "AudioObjectGetPropertyData")?;
    Ok(value)
}

/// Writes a fixed-size property (no qualifier).
pub fn set<T: Copy>(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
    value: T,
) -> Result<(), AudioError> {
    let address = address(selector, scope);
    let size = u32::try_from(size_of::<T>()).unwrap_or(u32::MAX);
    // SAFETY: `value` provides `size` readable bytes of a `Copy` type and `address` lives for the
    // call; no qualifier.
    let status = unsafe {
        AudioObjectSetPropertyData(
            object,
            NonNull::from(&address),
            0,
            std::ptr::null(),
            size,
            NonNull::from(&value).cast(),
        )
    };
    AudioError::check(status, "AudioObjectSetPropertyData")
}

/// Reads an array property (e.g. a list of object IDs).
pub fn get_vec<T: Copy + Default>(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Result<Vec<T>, AudioError> {
    let bytes = size(object, selector, scope)?;
    let count = bytes as usize / size_of::<T>();
    let mut values = vec![T::default(); count];
    if count == 0 {
        return Ok(values);
    }
    let address = address(selector, scope);
    let mut size = bytes;
    // SAFETY: `values` holds `count * size_of::<T>() == bytes` writable bytes.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::new_unchecked(values.as_mut_ptr()).cast(),
        )
    };
    AudioError::check(status, "AudioObjectGetPropertyData")?;
    values.truncate(size as usize / size_of::<T>());
    Ok(values)
}

/// Reads a raw, variable-length property into an 8-byte aligned buffer
/// (suitable for structures that contain pointers, such as `AudioBufferList`).
pub fn get_raw(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Result<Vec<u64>, AudioError> {
    let bytes = size(object, selector, scope)?;
    let mut buffer = vec![0_u64; (bytes as usize).div_ceil(size_of::<u64>()).max(1)];
    let address = address(selector, scope);
    let mut size = bytes;
    // SAFETY: `buffer` has at least `bytes` writable, 8-byte aligned bytes.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::new_unchecked(buffer.as_mut_ptr()).cast(),
        )
    };
    AudioError::check(status, "AudioObjectGetPropertyData")?;
    Ok(buffer)
}

/// Reads a `CFString` property (returned with +1 retain count).
pub fn get_string(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
) -> Result<String, AudioError> {
    let raw: usize = get::<usize, ()>(object, selector, GLOBAL, None)?;
    let pointer = NonNull::new(raw as *mut CFString)
        .ok_or_else(|| AudioError::NotFound("empty string property".to_owned()))?;
    // SAFETY: CoreAudio returns string properties as retained `CFStringRef`s; we take ownership.
    let string = unsafe { CFRetained::from_raw(pointer) };
    Ok(string.to_string())
}

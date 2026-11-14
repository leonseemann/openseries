//! Serialises [`PropertyValue`]s into the C layouts the HAL expects.

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_core_audio::AudioStreamRangedDescription;
use objc2_core_audio_types::{AudioStreamBasicDescription, AudioValueRange};
use objc2_core_foundation::{CFRetained, CFString};

use crate::model::{self, PropertyValue, StreamFormat};
use crate::selectors as sel;

/// Size in bytes of the complete value.
pub fn byte_size(value: &PropertyValue) -> usize {
    match value {
        PropertyValue::U32(_) | PropertyValue::TranslateUid => size_of::<u32>(),
        PropertyValue::F64(_) => size_of::<f64>(),
        PropertyValue::ObjectIds(ids) => size_of_val(*ids),
        PropertyValue::String(_) => size_of::<*const CFString>(),
        PropertyValue::Format(_) => size_of::<AudioStreamBasicDescription>(),
        PropertyValue::FormatList(_) => size_of::<AudioStreamRangedDescription>(),
        PropertyValue::Ranges(ranges) => ranges.len() * size_of::<AudioValueRange>(),
    }
}

/// Writes `value` into `out` (`capacity` bytes) and returns the bytes written.
///
/// Arrays are truncated to whole elements that fit; scalars need their full size.
///
/// # Safety
/// `out` must be valid for writes of `capacity` bytes and suitably aligned for the value
/// (the HAL passes buffers allocated for the property's C type). For
/// [`PropertyValue::TranslateUid`], `qualifier` must point to `qualifier_size` readable bytes.
pub unsafe fn write(
    value: &PropertyValue,
    out: NonNull<c_void>,
    capacity: usize,
    qualifier: Option<NonNull<c_void>>,
    qualifier_size: usize,
) -> Result<usize, i32> {
    // SAFETY (whole function): every write below stays within `capacity` bytes, as checked by
    // `scalar`/`array`, and the caller guarantees `out` is valid for them.
    unsafe {
        match value {
            PropertyValue::U32(v) => scalar(out, capacity, *v),
            PropertyValue::F64(v) => scalar(out, capacity, *v),
            PropertyValue::ObjectIds(ids) => Ok(array(out, capacity, ids.iter().copied())),
            PropertyValue::String(s) => {
                let string: CFRetained<CFString> = CFString::from_str(s);
                let size = scalar(out, capacity, CFRetained::into_raw(string).as_ptr())?;
                Ok(size)
            }
            PropertyValue::Format(format) => scalar(out, capacity, asbd(*format)),
            PropertyValue::FormatList(format) => scalar(
                out,
                capacity,
                AudioStreamRangedDescription {
                    mFormat: asbd(*format),
                    mSampleRateRange: AudioValueRange {
                        mMinimum: format.sample_rate,
                        mMaximum: format.sample_rate,
                    },
                },
            ),
            PropertyValue::Ranges(ranges) => Ok(array(
                out,
                capacity,
                ranges.iter().map(|&(min, max)| AudioValueRange {
                    mMinimum: min,
                    mMaximum: max,
                }),
            )),
            PropertyValue::TranslateUid => {
                let device = translate_uid(qualifier, qualifier_size).unwrap_or(0);
                scalar(out, capacity, device)
            }
        }
    }
}

/// Whether a `SetPropertyData` request is a no-op that can be accepted: setting the fixed
/// sample rate or format to the value it already has (aggregate devices do this).
///
/// # Safety
/// `data` must point to `size` readable bytes.
pub unsafe fn is_noop_set(selector: u32, data: NonNull<c_void>, size: usize) -> bool {
    match selector {
        sel::DEVICE_NOMINAL_SAMPLE_RATE if size >= size_of::<f64>() => {
            // SAFETY: at least 8 readable bytes; unaligned read is fine.
            let rate = unsafe { data.cast::<f64>().read_unaligned() };
            (rate - model::SAMPLE_RATE).abs() < f64::EPSILON
        }
        sel::STREAM_VIRTUAL_FORMAT | sel::STREAM_PHYSICAL_FORMAT
            if size >= size_of::<AudioStreamBasicDescription>() =>
        {
            // SAFETY: a complete ASBD is readable.
            let format = unsafe { data.cast::<AudioStreamBasicDescription>().read_unaligned() };
            let ours = asbd(model::FORMAT);
            (format.mSampleRate - ours.mSampleRate).abs() < f64::EPSILON
                && format.mFormatID == ours.mFormatID
                && format.mChannelsPerFrame == ours.mChannelsPerFrame
                && format.mBitsPerChannel == ours.mBitsPerChannel
        }
        _ => false,
    }
}

/// Reads the `CFStringRef` qualifier of `TranslateUIDToDevice` and looks the device up.
///
/// # Safety
/// `qualifier` must point to `size` readable bytes holding a `CFStringRef`.
unsafe fn translate_uid(qualifier: Option<NonNull<c_void>>, size: usize) -> Option<u32> {
    if size < size_of::<*const CFString>() {
        return None;
    }
    // SAFETY: the caller guarantees a readable CFStringRef.
    let string = unsafe { qualifier?.cast::<*const CFString>().read_unaligned() };
    // SAFETY: the HAL passes a valid CFString that outlives this call.
    let string = unsafe { string.as_ref()? };
    model::device_for_uid(&string.to_string())
}

const fn asbd(format: StreamFormat) -> AudioStreamBasicDescription {
    AudioStreamBasicDescription {
        mSampleRate: format.sample_rate,
        mFormatID: format.format_id,
        mFormatFlags: format.format_flags,
        mBytesPerPacket: format.bytes_per_packet,
        mFramesPerPacket: format.frames_per_packet,
        mBytesPerFrame: format.bytes_per_frame,
        mChannelsPerFrame: format.channels_per_frame,
        mBitsPerChannel: format.bits_per_channel,
        mReserved: 0,
    }
}

/// # Safety
/// `out` must be valid for `capacity` bytes.
unsafe fn scalar<T>(out: NonNull<c_void>, capacity: usize, value: T) -> Result<usize, i32> {
    if capacity < size_of::<T>() {
        return Err(sel::STATUS_BAD_PROPERTY_SIZE);
    }
    // SAFETY: room for one `T` was checked; unaligned write avoids alignment assumptions.
    unsafe { out.cast::<T>().write_unaligned(value) };
    Ok(size_of::<T>())
}

/// # Safety
/// `out` must be valid for `capacity` bytes.
unsafe fn array<T>(
    out: NonNull<c_void>,
    capacity: usize,
    items: impl ExactSizeIterator<Item = T>,
) -> usize {
    let count = items.len().min(capacity / size_of::<T>().max(1));
    for (index, item) in items.take(count).enumerate() {
        // SAFETY: `index < count` and `count * size_of::<T>() <= capacity`.
        unsafe { out.cast::<T>().add(index).write_unaligned(item) };
    }
    count * size_of::<T>()
}

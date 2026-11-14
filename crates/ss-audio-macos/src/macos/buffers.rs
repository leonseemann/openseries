//! Safe views on CoreAudio `AudioBufferList`s.

use std::ptr::NonNull;

use objc2_core_audio_types::{AudioBuffer, AudioBufferList};

/// Returns the buffers of a list as a slice.
///
/// # Safety
/// `list` must point to a valid `AudioBufferList` whose `mNumberBuffers` entries are
/// initialised and outlive `'a`.
pub unsafe fn buffers<'a>(list: NonNull<AudioBufferList>) -> &'a [AudioBuffer] {
    // SAFETY: guaranteed by the caller; `mBuffers` is a C flexible array of
    // `mNumberBuffers` elements.
    unsafe {
        let list = list.as_ptr();
        let count = (*list).mNumberBuffers as usize;
        let first = std::ptr::addr_of!((*list).mBuffers).cast::<AudioBuffer>();
        std::slice::from_raw_parts(first, count)
    }
}

/// Interprets a buffer as interleaved `f32` samples.
///
/// # Safety
/// `buffer.mData` must point to `mDataByteSize` bytes of `f32` samples valid for `'a`
/// and not aliased elsewhere for the lifetime of the returned slice.
pub unsafe fn samples<'a>(buffer: &AudioBuffer) -> &'a [f32] {
    if buffer.mData.is_null() {
        return &[];
    }
    let len = buffer.mDataByteSize as usize / size_of::<f32>();
    // SAFETY: guaranteed by the caller.
    unsafe { std::slice::from_raw_parts(buffer.mData.cast::<f32>(), len) }
}

/// Mutable variant of [`samples`].
///
/// # Safety
/// As for [`samples`], plus exclusive access to the memory.
pub unsafe fn samples_mut<'a>(buffer: &AudioBuffer) -> &'a mut [f32] {
    if buffer.mData.is_null() {
        return &mut [];
    }
    let len = buffer.mDataByteSize as usize / size_of::<f32>();
    // SAFETY: guaranteed by the caller.
    unsafe { std::slice::from_raw_parts_mut(buffer.mData.cast::<f32>(), len) }
}

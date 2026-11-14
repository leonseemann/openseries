//! Lock-free loopback ring shared by the feed (writer) and microphone (reader) devices
//! (`docs/architecture/virtual-mic-driver.md` §4).
//!
//! Samples are addressed by the shared device sample time, so writer and reader need no
//! read/write pointers: the writer stores the frames for instant `T`, the reader asks for `T`.

use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};

/// Mono ring of `f32` samples indexed by sample time. Real-time safe after construction.
#[derive(Debug)]
pub struct LoopbackRing {
    samples: Box<[AtomicU32]>,
    /// One past the newest written sample time; `NOTHING_WRITTEN` before the first write.
    written_until: AtomicI64,
}

const NOTHING_WRITTEN: i64 = i64::MIN;

impl LoopbackRing {
    /// Creates a silent ring holding `capacity` frames (`capacity > 0`).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: (0..capacity.max(1)).map(|_| AtomicU32::new(0)).collect(),
            written_until: AtomicI64::new(NOTHING_WRITTEN),
        }
    }

    /// Capacity in frames.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.samples.len()
    }

    /// Stores `frames` starting at `sample_time`.
    pub fn write(&self, sample_time: i64, frames: &[f32]) {
        for (offset, sample) in (0_i64..).zip(frames) {
            self.slot(sample_time + offset)
                .store(sample.to_bits(), Ordering::Relaxed);
        }
        let end = sample_time.saturating_add(i64::try_from(frames.len()).unwrap_or(i64::MAX));
        self.written_until.fetch_max(end, Ordering::Release);
    }

    /// Fills `out` with the frames starting at `sample_time`; frames that were never written or
    /// have already been overwritten by newer data read as silence.
    pub fn read(&self, sample_time: i64, out: &mut [f32]) {
        let written_until = self.written_until.load(Ordering::Acquire);
        let capacity = i64::try_from(self.capacity()).unwrap_or(i64::MAX);
        let oldest_valid = written_until.saturating_sub(capacity);
        for (offset, sample) in (0_i64..).zip(out.iter_mut()) {
            let time = sample_time + offset;
            *sample =
                if written_until != NOTHING_WRITTEN && time < written_until && time >= oldest_valid
                {
                    f32::from_bits(self.slot(time).load(Ordering::Relaxed))
                } else {
                    0.0
                };
        }
    }

    /// Forgets all data (e.g. when the feed restarts with a new clock anchor).
    pub fn clear(&self) {
        self.written_until.store(NOTHING_WRITTEN, Ordering::Release);
    }

    fn slot(&self, sample_time: i64) -> &AtomicU32 {
        let capacity = i64::try_from(self.capacity()).unwrap_or(i64::MAX);
        // `rem_euclid` is in 0..capacity, so the index is always in bounds.
        let index = usize::try_from(sample_time.rem_euclid(capacity)).unwrap_or(0);
        &self.samples[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_back_what_was_written_at_the_same_time() {
        let ring = LoopbackRing::new(8);
        ring.write(100, &[0.1, 0.2, 0.3]);
        let mut out = [9.0; 3];
        ring.read(100, &mut out);
        assert_eq!(out, [0.1, 0.2, 0.3]);
    }

    #[test]
    fn unwritten_and_future_frames_are_silent() {
        let ring = LoopbackRing::new(8);
        let mut out = [9.0; 2];
        ring.read(0, &mut out);
        assert_eq!(out, [0.0, 0.0], "nothing written yet");

        ring.write(10, &[0.5, 0.5]);
        let mut out = [9.0; 4];
        ring.read(10, &mut out);
        assert_eq!(out, [0.5, 0.5, 0.0, 0.0], "reader ahead of writer");
    }

    #[test]
    fn overwritten_frames_are_silent_not_stale() {
        let ring = LoopbackRing::new(4);
        ring.write(0, &[1.0; 4]);
        ring.write(4, &[2.0; 4]);
        let mut out = [9.0; 4];
        ring.read(0, &mut out);
        assert_eq!(out, [0.0; 4], "older than capacity");
        ring.read(4, &mut out);
        assert_eq!(out, [2.0; 4]);
    }

    #[test]
    fn wraps_around_and_handles_negative_times() {
        let ring = LoopbackRing::new(4);
        ring.write(-2, &[1.0, 2.0, 3.0, 4.0]);
        let mut out = [0.0; 4];
        ring.read(-2, &mut out);
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn clear_silences_the_ring() {
        let ring = LoopbackRing::new(4);
        ring.write(0, &[1.0; 2]);
        ring.clear();
        let mut out = [9.0; 2];
        ring.read(0, &mut out);
        assert_eq!(out, [0.0; 2]);
    }
}

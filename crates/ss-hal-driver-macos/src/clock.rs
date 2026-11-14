//! Device clock shared by both devices (`docs/architecture/virtual-mic-driver.md` §3).
//!
//! Zero time stamps are `(n · period, anchor + n · period · ticks_per_frame)` with
//! `n = ⌊(now − anchor) / (period · ticks_per_frame)⌋`: stateless apart from the anchor,
//! so the real-time `GetZeroTimeStamp` call needs no lock.

use std::sync::atomic::{AtomicU64, Ordering};

/// Seed reported with every time stamp; it never changes because the clock never jumps.
pub const TIMESTAMP_SEED: u64 = 1;

/// One zero time stamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZeroTimeStamp {
    /// Device sample time of the period start.
    pub sample_time: f64,
    /// Host time (`mach_absolute_time` ticks) of the period start.
    pub host_time: u64,
}

/// Anchored device clock.
#[derive(Debug)]
pub struct DeviceClock {
    period_frames: u32,
    host_ticks_per_frame: f64,
    anchor: AtomicU64,
}

impl DeviceClock {
    /// Creates a clock with `period_frames` per zero time stamp and the given host tick rate.
    #[must_use]
    pub fn new(period_frames: u32, sample_rate: f64, host_ticks_per_second: f64) -> Self {
        Self {
            period_frames,
            host_ticks_per_frame: host_ticks_per_second / sample_rate,
            anchor: AtomicU64::new(0),
        }
    }

    /// Starts counting at host time `now`.
    pub fn anchor_at(&self, now: u64) {
        self.anchor.store(now, Ordering::Release);
    }

    /// The most recent zero time stamp at host time `now`.
    #[must_use]
    pub fn zero_time_stamp(&self, now: u64) -> ZeroTimeStamp {
        let anchor = self.anchor.load(Ordering::Acquire);
        let ticks_per_period = f64::from(self.period_frames) * self.host_ticks_per_frame;
        #[allow(clippy::cast_precision_loss)] // tick differences stay far below 2^52
        let elapsed = now.saturating_sub(anchor) as f64;
        let periods = (elapsed / ticks_per_period).floor();
        // Non-negative and at most `elapsed`, so it fits into u64.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let host_offset = (periods * ticks_per_period) as u64;
        ZeroTimeStamp {
            sample_time: periods * f64::from(self.period_frames),
            host_time: anchor + host_offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 48 kHz with one host tick per nanosecond → 20833.3 ticks per frame.
    fn clock() -> DeviceClock {
        DeviceClock::new(1000, 48_000.0, 1e9)
    }

    #[test]
    fn starts_at_the_anchor() {
        let clock = clock();
        clock.anchor_at(5_000);
        let stamp = clock.zero_time_stamp(5_000);
        assert!(stamp.sample_time.abs() < f64::EPSILON);
        assert_eq!(stamp.host_time, 5_000);
    }

    #[test]
    fn advances_one_period_at_a_time() {
        let clock = clock();
        clock.anchor_at(0);
        let period_ns = 1e9 * 1000.0 / 48_000.0; // 20.83 ms
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let just_before = (period_ns * 2.0) as u64 - 1;
        let stamp = clock.zero_time_stamp(just_before);
        assert!((stamp.sample_time - 1000.0).abs() < f64::EPSILON);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let after_two = (period_ns * 2.0) as u64 + 1;
        assert!((clock.zero_time_stamp(after_two).sample_time - 2000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn time_before_the_anchor_reports_the_anchor() {
        let clock = clock();
        clock.anchor_at(1_000_000);
        assert_eq!(clock.zero_time_stamp(10).host_time, 1_000_000);
    }
}

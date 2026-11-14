//! "Fix crackling automatically" (`docs/hardware/macos-app.md` Findings): when the OS reports
//! bursts of USB underflows from the base station, reset its USB connection.
//!
//! The decision logic is portable and tested here; the underflow source and the reset come from
//! [`crate::platform`].

use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime};

/// Underflows within [`WINDOW`] that count as a crackling episode. Normal operation shows none;
/// observed episodes produce 20–50 per second.
pub const BURST_THRESHOLD: usize = 20;
/// Sliding window for [`BURST_THRESHOLD`].
pub const WINDOW: Duration = Duration::from_secs(3);
/// Minimum time between two automatic resets (the reset itself can log a few underflows).
pub const COOLDOWN: Duration = Duration::from_secs(30);

/// What the UI shows about the automatic fix.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrackleStatus {
    /// The watch is running.
    pub active: bool,
    /// Automatic resets since the app started.
    pub fixes: u32,
    /// Time of the last automatic reset.
    pub last_fix: Option<SystemTime>,
    /// Why the watch is not running although enabled.
    pub error: Option<String>,
}

/// Decides when a run of underflow reports is a crackling episode.
#[derive(Debug, Default)]
pub struct BurstDetector {
    recent: VecDeque<Instant>,
    last_trigger: Option<Instant>,
}

impl BurstDetector {
    /// Records one underflow at `now`; returns `true` if a reset should happen now.
    pub fn record(&mut self, now: Instant) -> bool {
        self.recent.push_back(now);
        while self
            .recent
            .front()
            .is_some_and(|&t| now.duration_since(t) > WINDOW)
        {
            self.recent.pop_front();
        }
        let cooling_down = self
            .last_trigger
            .is_some_and(|t| now.duration_since(t) < COOLDOWN);
        if self.recent.len() >= BURST_THRESHOLD && !cooling_down {
            self.last_trigger = Some(now);
            self.recent.clear();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(
        detector: &mut BurstDetector,
        start: Instant,
        count: usize,
        spacing: Duration,
    ) -> usize {
        (0..count)
            .filter(|&i| {
                let offset = spacing * u32::try_from(i).unwrap_or(u32::MAX);
                detector.record(start + offset)
            })
            .count()
    }

    #[test]
    fn a_burst_triggers_once() {
        let mut detector = BurstDetector::default();
        let start = Instant::now();
        // 40 underflows in 2 s, like an observed episode.
        assert_eq!(feed(&mut detector, start, 40, Duration::from_millis(50)), 1);
    }

    #[test]
    fn sparse_underflows_never_trigger() {
        let mut detector = BurstDetector::default();
        let start = Instant::now();
        // One per second for a minute: below threshold in any window.
        assert_eq!(feed(&mut detector, start, 60, Duration::from_secs(1)), 0);
    }

    #[test]
    fn cooldown_prevents_reset_loops_but_allows_later_episodes() {
        let mut detector = BurstDetector::default();
        let start = Instant::now();
        assert_eq!(feed(&mut detector, start, 25, Duration::from_millis(40)), 1);
        // The reset itself may log a few more; a second burst right after must not trigger.
        let soon = start + Duration::from_secs(5);
        assert_eq!(feed(&mut detector, soon, 25, Duration::from_millis(40)), 0);
        // A new episode after the cooldown does.
        let later = start + COOLDOWN + Duration::from_secs(5);
        assert_eq!(feed(&mut detector, later, 25, Duration::from_millis(40)), 1);
    }
}

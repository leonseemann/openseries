//! Driver state shared by all entry points: IO bookkeeping, the clock and the loopback ring.
//! Platform-free; the FFI layer only translates C calls into these methods.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::clock::{DeviceClock, ZeroTimeStamp};
use crate::model::{DeviceKind, PERIOD_FRAMES, RuntimeState, SAMPLE_RATE};
use crate::ring::LoopbackRing;

/// Why an IO request was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoError {
    /// `StopIO` without a matching `StartIO`.
    NotRunning,
}

/// The whole driver.
#[derive(Debug)]
pub struct Driver {
    clock: DeviceClock,
    ring: LoopbackRing,
    /// Number of `StartIO` calls without a matching `StopIO`, per device (mic, feed).
    clients: Mutex<[u32; 2]>,
    running: [AtomicBool; 2],
}

impl Driver {
    /// Creates the driver for a host clock running at `host_ticks_per_second`.
    #[must_use]
    pub fn new(host_ticks_per_second: f64) -> Self {
        Self {
            clock: DeviceClock::new(PERIOD_FRAMES, SAMPLE_RATE, host_ticks_per_second),
            ring: LoopbackRing::new(PERIOD_FRAMES as usize),
            clients: Mutex::new([0; 2]),
            running: [AtomicBool::new(false), AtomicBool::new(false)],
        }
    }

    /// Registers one more IO client on `kind`. The first client of either device anchors the
    /// shared clock at host time `now` and clears the ring (sample times restart at 0).
    pub fn start_io(&self, kind: DeviceKind, now: u64) {
        let mut clients = self.lock_clients();
        if clients.iter().all(|&c| c == 0) {
            self.clock.anchor_at(now);
            self.ring.clear();
        }
        let slot = &mut clients[index(kind)];
        *slot = slot.saturating_add(1);
        self.running[index(kind)].store(true, Ordering::Release);
    }

    /// Removes one IO client from `kind`.
    ///
    /// # Errors
    /// [`IoError::NotRunning`] if `kind` has no client.
    pub fn stop_io(&self, kind: DeviceKind) -> Result<(), IoError> {
        let mut clients = self.lock_clients();
        let slot = &mut clients[index(kind)];
        if *slot == 0 {
            return Err(IoError::NotRunning);
        }
        *slot -= 1;
        self.running[index(kind)].store(*slot > 0, Ordering::Release);
        Ok(())
    }

    /// Running flags for the property model.
    #[must_use]
    pub fn state(&self) -> RuntimeState {
        RuntimeState {
            mic_running: self.running[index(DeviceKind::Mic)].load(Ordering::Acquire),
            feed_running: self.running[index(DeviceKind::Feed)].load(Ordering::Acquire),
        }
    }

    /// Current zero time stamp (identical for both devices). Real-time safe.
    #[must_use]
    pub fn zero_time_stamp(&self, now: u64) -> ZeroTimeStamp {
        self.clock.zero_time_stamp(now)
    }

    /// Whether `kind` handles IO operation `operation` (always in place).
    #[must_use]
    pub const fn will_do(kind: DeviceKind, operation: u32) -> bool {
        kind.io_operation() == operation
    }

    /// Performs `operation` on `buffer` for the cycle at `sample_time`. Real-time safe.
    /// Operations the device does not handle are ignored.
    pub fn do_io(&self, kind: DeviceKind, operation: u32, sample_time: f64, buffer: &mut [f32]) {
        if !Self::will_do(kind, operation) {
            return;
        }
        #[allow(clippy::cast_possible_truncation)] // sample times are whole frames well below 2^63
        let time = sample_time.floor() as i64;
        match kind {
            DeviceKind::Feed => self.ring.write(time, buffer),
            DeviceKind::Mic => self.ring.read(time, buffer),
        }
    }

    fn lock_clients(&self) -> std::sync::MutexGuard<'_, [u32; 2]> {
        // A poisoned lock only means another entry point panicked mid-update; the counters are
        // still plain integers, so continue with them.
        self.clients
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

const fn index(kind: DeviceKind) -> usize {
    match kind {
        DeviceKind::Mic => 0,
        DeviceKind::Feed => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selectors as sel;

    #[test]
    fn feed_writes_reach_the_mic_at_the_same_sample_time() {
        let driver = Driver::new(1e9);
        driver.start_io(DeviceKind::Feed, 0);
        driver.start_io(DeviceKind::Mic, 0);
        let mut feed = [0.25_f32; 4];
        driver.do_io(DeviceKind::Feed, sel::IO_WRITE_MIX, 512.0, &mut feed);
        let mut mic = [0.0_f32; 4];
        driver.do_io(DeviceKind::Mic, sel::IO_READ_INPUT, 512.0, &mut mic);
        assert_eq!(mic, [0.25; 4]);
    }

    #[test]
    fn devices_ignore_operations_of_the_other_side() {
        let driver = Driver::new(1e9);
        let mut buffer = [0.5_f32; 2];
        driver.do_io(DeviceKind::Mic, sel::IO_WRITE_MIX, 0.0, &mut buffer);
        let mut mic = [9.0_f32; 2];
        driver.do_io(DeviceKind::Mic, sel::IO_READ_INPUT, 0.0, &mut mic);
        assert_eq!(mic, [0.0; 2], "the mic never writes");
        assert!(Driver::will_do(DeviceKind::Feed, sel::IO_WRITE_MIX));
        assert!(!Driver::will_do(DeviceKind::Feed, sel::IO_READ_INPUT));
    }

    #[test]
    fn restarting_all_io_clears_stale_audio() {
        let driver = Driver::new(1e9);
        driver.start_io(DeviceKind::Feed, 0);
        let mut feed = [1.0_f32; 4];
        driver.do_io(DeviceKind::Feed, sel::IO_WRITE_MIX, 0.0, &mut feed);
        assert_eq!(driver.stop_io(DeviceKind::Feed), Ok(()));

        driver.start_io(DeviceKind::Mic, 1_000);
        let mut mic = [9.0_f32; 4];
        driver.do_io(DeviceKind::Mic, sel::IO_READ_INPUT, 0.0, &mut mic);
        assert_eq!(mic, [0.0; 4]);
    }

    #[test]
    fn running_state_follows_client_counts() {
        let driver = Driver::new(1e9);
        driver.start_io(DeviceKind::Mic, 0);
        driver.start_io(DeviceKind::Mic, 0);
        assert!(driver.state().mic_running);
        assert_eq!(driver.stop_io(DeviceKind::Mic), Ok(()));
        assert!(driver.state().mic_running, "one client left");
        assert_eq!(driver.stop_io(DeviceKind::Mic), Ok(()));
        assert!(!driver.state().mic_running);
        assert_eq!(driver.stop_io(DeviceKind::Mic), Err(IoError::NotRunning));
    }
}

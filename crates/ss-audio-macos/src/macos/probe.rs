//! Real-time-safe measurements of a running session, used to verify the *Inferred* buffer
//! layout on hardware (`docs/hardware/macos-audiomix.md`, checks 6 and 7).
//!
//! The IOProc only stores into atomics; the control thread reads (and resets) them.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Most aggregate input buffers that are metered; further buffers are ignored.
pub const MAX_PROBED_BUFFERS: usize = 8;

/// Values written by the IOProc.
#[derive(Debug, Default)]
pub struct Probe {
    callbacks: AtomicU64,
    input_buffers: AtomicU32,
    output_buffers: AtomicU32,
    output_channels: AtomicU32,
    frames: AtomicU32,
    /// Peak magnitude per input buffer as `f32` bits. For non-negative floats the bit pattern
    /// orders like the value, so `fetch_max` on the bits is a max on the samples.
    peaks: [AtomicU32; MAX_PROBED_BUFFERS],
    channels: [AtomicU32; MAX_PROBED_BUFFERS],
}

/// One reading of the probe.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ProbeSnapshot {
    /// IOProc calls since the session started.
    pub callbacks: u64,
    /// Buffers in the aggregate input list of the last callback.
    pub input_buffers: usize,
    /// Buffers in the aggregate output list of the last callback.
    pub output_buffers: usize,
    /// Channels of the first output buffer (interleaved when equal to the device's channels).
    pub output_channels: usize,
    /// Frames of the last callback.
    pub frames: usize,
    /// Peak magnitude per input buffer since the previous snapshot.
    pub peaks: [f32; MAX_PROBED_BUFFERS],
    /// Channels per input buffer.
    pub channels: [usize; MAX_PROBED_BUFFERS],
}

/// Layout of one callback, as recorded by [`Probe::record_layout`].
#[derive(Debug, Clone, Copy)]
pub struct CallbackLayout {
    pub input_buffers: usize,
    pub output_buffers: usize,
    pub output_channels: usize,
    pub frames: usize,
}

impl Probe {
    /// Records the shape of one callback. Real-time safe.
    pub fn record_layout(&self, layout: CallbackLayout) {
        self.callbacks.fetch_add(1, Ordering::Relaxed);
        self.input_buffers
            .store(saturate(layout.input_buffers), Ordering::Relaxed);
        self.output_buffers
            .store(saturate(layout.output_buffers), Ordering::Relaxed);
        self.output_channels
            .store(saturate(layout.output_channels), Ordering::Relaxed);
        self.frames
            .store(saturate(layout.frames), Ordering::Relaxed);
    }

    /// Records the peak of input buffer `index`. Real-time safe.
    pub fn record_input(&self, index: usize, channels: usize, samples: &[f32]) {
        let (Some(peak_slot), Some(channel_slot)) =
            (self.peaks.get(index), self.channels.get(index))
        else {
            return;
        };
        let peak = samples.iter().fold(0.0_f32, |max, s| max.max(s.abs()));
        peak_slot.fetch_max(peak.to_bits(), Ordering::Relaxed);
        channel_slot.store(saturate(channels), Ordering::Relaxed);
    }

    /// Reads all values and resets the peaks.
    pub fn take(&self) -> ProbeSnapshot {
        let mut snapshot = ProbeSnapshot {
            callbacks: self.callbacks.load(Ordering::Relaxed),
            input_buffers: self.input_buffers.load(Ordering::Relaxed) as usize,
            output_buffers: self.output_buffers.load(Ordering::Relaxed) as usize,
            output_channels: self.output_channels.load(Ordering::Relaxed) as usize,
            frames: self.frames.load(Ordering::Relaxed) as usize,
            ..ProbeSnapshot::default()
        };
        for (i, (peak, channels)) in self.peaks.iter().zip(&self.channels).enumerate() {
            snapshot.peaks[i] = f32::from_bits(peak.swap(0, Ordering::Relaxed));
            snapshot.channels[i] = channels.load(Ordering::Relaxed) as usize;
        }
        snapshot
    }
}

fn saturate(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_largest_magnitude_until_taken() {
        let probe = Probe::default();
        probe.record_input(1, 2, &[0.1, -0.5]);
        probe.record_input(1, 2, &[0.3, 0.2]);
        let first = probe.take();
        assert!((first.peaks[1] - 0.5).abs() < f32::EPSILON);
        assert_eq!(first.channels[1], 2);
        assert!(
            probe.take().peaks[1].abs() < f32::EPSILON,
            "reset after take"
        );
    }

    #[test]
    fn ignores_buffers_beyond_capacity() {
        let probe = Probe::default();
        probe.record_input(MAX_PROBED_BUFFERS, 1, &[1.0]);
        assert!(probe.take().peaks.iter().all(|p| p.abs() < f32::EPSILON));
    }
}

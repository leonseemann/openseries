//! Clock-drift compensation between two audio devices (`docs/architecture/windows-virtual-driver.md`
//! §2.4).
//!
//! A capture stream (one clock) feeds a render stream (another clock). [`DriftWriter`] pushes
//! interleaved frames into a lock-free SPSC ring buffer on the capture thread; [`DriftReader`]
//! pulls them on the render thread and resamples by a tiny ratio so the buffer's fill level stays
//! at a target. The ratio comes from a PI controller on the (smoothed) fill level; samples are
//! interpolated with a Catmull-Rom cubic. Both sides are real-time safe: all memory is allocated
//! in [`drift_buffer`].

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Largest resampling correction (±0.2 % = 2000 ppm); real USB clocks differ by far less.
pub const MAX_CORRECTION: f64 = 0.002;
/// Proportional gain: correction per unit of normalised fill error.
///
/// Tuning (per read call, ≈ 100 calls/s): the fill level integrates the clock mismatch, so with
/// a period of half the target the loop is x'' + 0.5·KP·x' + 0.5·KI·x = 0, giving
/// ω ≈ 0.0032 rad/call (≈ 3 s time constant) and damping ζ ≈ 0.8. A fill ripple of 1 % then moves
/// the ratio by only 100 ppm (≈ 0.2 cent), far below audibility.
const KP: f64 = 0.01;
/// Integral gain per read call.
const KI: f64 = 0.000_02;
/// Smoothing of the measured fill level per read call (exponential moving average).
const FILL_SMOOTHING: f64 = 0.02;
/// Frames of history the cubic interpolator needs.
const WINDOW: usize = 4;

/// Counters shared by both ends (diagnostics).
#[derive(Debug, Default)]
pub struct DriftStats {
    underruns: AtomicU64,
    overruns: AtomicU64,
}

impl DriftStats {
    /// Times the reader ran dry and had to output silence while re-priming.
    #[must_use]
    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    /// Times the writer found the buffer full and dropped frames.
    #[must_use]
    pub fn overruns(&self) -> u64 {
        self.overruns.load(Ordering::Relaxed)
    }
}

/// Capture side.
pub struct DriftWriter {
    producer: rtrb::Producer<f32>,
    channels: usize,
    stats: Arc<DriftStats>,
}

/// Render side.
pub struct DriftReader {
    consumer: rtrb::Consumer<f32>,
    channels: usize,
    target_frames: f64,
    /// Last [`WINDOW`] frames, oldest first (interleaved).
    window: Box<[f32]>,
    /// Position between window frames 1 and 2, `0.0..1.0`.
    position: f64,
    smoothed_fill: f64,
    integral: f64,
    correction: f64,
    primed: bool,
    stats: Arc<DriftStats>,
}

/// Creates a connected writer/reader pair.
///
/// `capacity_frames` bounds the buffer; `target_frames` is the fill level (≈ latency) the reader
/// steers to. Panics never; a zero channel count is treated as one.
#[must_use]
pub fn drift_buffer(
    channels: usize,
    capacity_frames: usize,
    target_frames: usize,
) -> (DriftWriter, DriftReader, Arc<DriftStats>) {
    let channels = channels.max(1);
    let (producer, consumer) = rtrb::RingBuffer::new(capacity_frames.max(1) * channels);
    let stats = Arc::new(DriftStats::default());
    #[allow(clippy::cast_precision_loss)] // frame counts are small
    let target = target_frames.max(1) as f64;
    (
        DriftWriter {
            producer,
            channels,
            stats: Arc::clone(&stats),
        },
        DriftReader {
            consumer,
            channels,
            target_frames: target,
            window: vec![0.0; WINDOW * channels].into_boxed_slice(),
            position: 0.0,
            smoothed_fill: 0.0,
            integral: 0.0,
            correction: 0.0,
            primed: false,
            stats: Arc::clone(&stats),
        },
        stats,
    )
}

impl DriftWriter {
    /// Pushes interleaved frames; returns how many whole frames fit (the rest is dropped and
    /// counted as an overrun).
    pub fn write(&mut self, interleaved: &[f32]) -> usize {
        let frames = interleaved.len() / self.channels;
        let room = self.producer.slots() / self.channels;
        let accepted = frames.min(room);
        for &sample in &interleaved[..accepted * self.channels] {
            // Cannot fail: `room` slots were free and only this thread pushes.
            let _ = self.producer.push(sample);
        }
        if accepted < frames {
            self.stats.overruns.fetch_add(1, Ordering::Relaxed);
        }
        accepted
    }
}

impl DriftReader {
    /// Current resampling correction (e.g. `0.0001` = the reader consumes 100 ppm faster).
    #[must_use]
    pub const fn correction(&self) -> f64 {
        self.correction
    }

    /// Frames waiting in the buffer.
    #[must_use]
    pub fn fill_frames(&self) -> usize {
        self.consumer.slots() / self.channels
    }

    /// Fills `out` (interleaved) completely: resampled audio, or silence while (re-)priming.
    pub fn read(&mut self, out: &mut [f32]) {
        #[allow(clippy::cast_precision_loss)]
        let fill = self.fill_frames() as f64;
        if !self.primed {
            if fill < self.target_frames {
                out.fill(0.0);
                return;
            }
            self.primed = true;
            self.smoothed_fill = fill;
            self.integral = 0.0;
            self.position = 0.0;
            self.window.fill(0.0);
        }
        self.update_correction(fill);
        let step = 1.0 + self.correction;
        let channels = self.channels;
        let frames = out.len() / channels;
        for index in 0..frames {
            let frame = &mut out[index * channels..(index + 1) * channels];
            for (channel, sample) in frame.iter_mut().enumerate() {
                *sample = self.interpolate(channel);
            }
            self.position += step;
            while self.position >= 1.0 {
                self.position -= 1.0;
                if !self.advance() {
                    // Ran dry: silence for the rest of this call, then re-prime.
                    self.primed = false;
                    self.stats.underruns.fetch_add(1, Ordering::Relaxed);
                    out[(index + 1) * channels..].fill(0.0);
                    return;
                }
            }
        }
        // A trailing partial frame (malformed buffer) stays silent.
        out[frames * channels..].fill(0.0);
    }

    /// PI control of the fill level towards the target.
    fn update_correction(&mut self, fill: f64) {
        self.smoothed_fill += FILL_SMOOTHING * (fill - self.smoothed_fill);
        let error = (self.smoothed_fill - self.target_frames) / self.target_frames;
        self.integral = (self.integral + KI * error).clamp(-MAX_CORRECTION, MAX_CORRECTION);
        self.correction = (KP * error + self.integral).clamp(-MAX_CORRECTION, MAX_CORRECTION);
    }

    /// Catmull-Rom between window frames 1 and 2 at `position`.
    fn interpolate(&self, channel: usize) -> f32 {
        let c = self.channels;
        let p = |i: usize| f64::from(self.window[i * c + channel]);
        let (p0, p1, p2, p3) = (p(0), p(1), p(2), p(3));
        let t = self.position;
        let value = 0.5
            * (2.0 * p1
                + (-p0 + p2) * t
                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
                + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t);
        #[allow(clippy::cast_possible_truncation)] // back to the sample type
        let sample = value as f32;
        sample
    }

    /// Shifts the window by one frame, pulling the next frame from the buffer.
    fn advance(&mut self) -> bool {
        if self.consumer.slots() < self.channels {
            return false;
        }
        let c = self.channels;
        self.window.copy_within(c.., 0);
        for slot in &mut self.window[(WINDOW - 1) * c..] {
            // Cannot fail: a whole frame is available and only this thread pops.
            *slot = self.consumer.pop().unwrap_or(0.0);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;
    const PERIOD: usize = 480; // 10 ms at 48 kHz

    /// Simulates `seconds` of a capture clock running `producer_ppm` faster than the render
    /// clock; returns (max fill, min fill after settling, final correction).
    #[allow(clippy::cast_precision_loss)] // small frame counts in a test
    fn simulate(producer_ppm: f64, seconds: f64) -> (usize, usize, f64, Arc<DriftStats>) {
        let (mut writer, mut reader, stats) = drift_buffer(2, 8 * PERIOD, 2 * PERIOD);
        let mut out = vec![0.0_f32; PERIOD * 2];
        let mut produced = 0.0_f64;
        let mut phase = 0.0_f64;
        let (mut max_fill, mut min_fill) = (0, usize::MAX);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let calls = (seconds * RATE / PERIOD as f64) as usize;
        for call in 0..calls {
            // Producer: PERIOD × (1 + ppm) frames per render period, in whole frames.
            produced += PERIOD as f64 * (1.0 + producer_ppm * 1e-6);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let frames = produced as usize;
            produced -= frames as f64;
            let mut chunk = Vec::with_capacity(frames * 2);
            for _ in 0..frames {
                phase += 2.0 * std::f64::consts::PI * 440.0 / RATE;
                #[allow(clippy::cast_possible_truncation)]
                let s = (0.5 * phase.sin()) as f32;
                chunk.extend_from_slice(&[s, s]);
            }
            writer.write(&chunk);
            reader.read(&mut out);
            let fill = reader.fill_frames();
            max_fill = max_fill.max(fill);
            if call > calls / 3 {
                min_fill = min_fill.min(fill);
            }
        }
        (max_fill, min_fill, reader.correction(), stats)
    }

    #[test]
    fn equal_clocks_stay_near_the_target() {
        let (max, min, correction, stats) = simulate(0.0, 30.0);
        assert_eq!(stats.underruns() + stats.overruns(), 0);
        assert!(min >= PERIOD / 2 && max <= 4 * PERIOD, "fill {min}..{max}");
        assert!(correction.abs() < 50e-6, "correction {correction}");
    }

    #[test]
    fn faster_producer_is_absorbed() {
        let (max, min, correction, stats) = simulate(500.0, 120.0);
        assert_eq!(stats.underruns() + stats.overruns(), 0);
        assert!(min >= PERIOD / 2 && max <= 4 * PERIOD, "fill {min}..{max}");
        assert!(
            (correction - 500e-6).abs() < 100e-6,
            "correction {correction}"
        );
    }

    #[test]
    fn slower_producer_is_absorbed() {
        let (max, min, correction, stats) = simulate(-500.0, 120.0);
        assert_eq!(stats.underruns() + stats.overruns(), 0);
        assert!(min >= PERIOD / 2 && max <= 4 * PERIOD, "fill {min}..{max}");
        assert!(
            (correction + 500e-6).abs() < 100e-6,
            "correction {correction}"
        );
    }

    #[test]
    fn primes_with_silence_then_passes_audio_through() {
        let (mut writer, mut reader, stats) = drift_buffer(1, 4096, 960);
        let mut out = vec![1.0_f32; 480];
        reader.read(&mut out);
        assert!(out.iter().all(|&s| s == 0.0), "silence while priming");
        let ramp: Vec<f32> = (0..2000_u16).map(|i| f32::from(i) / 2000.0).collect();
        writer.write(&ramp);
        reader.read(&mut out);
        // A slow ramp survives cubic interpolation almost unchanged (one-sample steps).
        let rising = out.windows(2).filter(|w| w[1] > w[0]).count();
        assert!(rising > 470, "{rising} of 479 steps rising");
        assert_eq!(stats.underruns(), 0);
    }

    #[test]
    fn running_dry_counts_an_underrun_and_outputs_silence() {
        let (mut writer, mut reader, stats) = drift_buffer(1, 4096, 480);
        writer.write(&[0.5; 600]);
        let mut out = vec![1.0_f32; 480];
        reader.read(&mut out); // primed, consumes ~480
        reader.read(&mut out); // runs dry
        assert_eq!(stats.underruns(), 1);
        assert_eq!(*out.last().unwrap_or(&1.0), 0.0);
    }

    #[test]
    fn full_buffer_drops_and_counts_an_overrun() {
        let (mut writer, _reader, stats) = drift_buffer(2, 100, 50);
        assert_eq!(writer.write(&[0.0; 300]), 100);
        assert_eq!(stats.overruns(), 1);
    }
}

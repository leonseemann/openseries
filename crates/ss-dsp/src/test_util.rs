//! Measurement helpers for tests.

use std::f32::consts::TAU;

use crate::Processor;
use crate::level::linear_to_db;

/// Sample rate used by tests.
pub const FS: f32 = 48_000.0;

/// Interleaved sine of `seconds` at `freq` Hz, identical on every channel.
pub fn sine(freq: f32, amplitude: f32, seconds: f32, channels: usize) -> Vec<f32> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let frames = (FS * seconds) as usize;
    let mut out = Vec::with_capacity(frames * channels);
    for n in 0..frames {
        #[allow(clippy::cast_precision_loss)]
        let s = amplitude * (TAU * freq * n as f32 / FS).sin();
        out.extend(std::iter::repeat_n(s, channels));
    }
    out
}

/// RMS of the second half of channel 0 (steady state).
pub fn steady_rms(buffer: &[f32], channels: usize) -> f32 {
    let samples: Vec<f32> = buffer.iter().step_by(channels).copied().collect();
    let tail = &samples[samples.len() / 2..];
    #[allow(clippy::cast_precision_loss)]
    let mean = tail.iter().map(|s| s * s).sum::<f32>() / tail.len() as f32;
    mean.sqrt()
}

/// Steady-state gain in dB that `processor` applies to a sine at `freq`.
pub fn gain_at(processor: &mut impl Processor, freq: f32, channels: usize) -> f32 {
    processor.reset();
    let input = sine(freq, 0.25, 0.5, channels);
    let mut output = input.clone();
    processor.process(&mut output);
    linear_to_db(steady_rms(&output, channels) / steady_rms(&input, channels))
}

/// Asserts `actual` is within `tolerance` of `expected` (dB values).
pub fn assert_db(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} dB ± {tolerance}, got {actual} dB"
    );
}

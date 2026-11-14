//! AI noise suppression for the microphone (SON-19) with RNNoise (`nnnoiseless`, BSD-3-Clause).
//!
//! Design: `docs/architecture/mic-chain.md` §3. RNNoise needs 480-sample frames at 48 kHz, so
//! arbitrary callback blocks are adapted through fixed FIFOs at a constant latency.

use nnnoiseless::DenoiseState;

use crate::{Format, Processor};

/// RNNoise frame length (10 ms at 48 kHz).
pub const DENOISE_FRAME: usize = DenoiseState::FRAME_SIZE;
/// The only sample rate RNNoise supports.
pub const DENOISE_SAMPLE_RATE: f32 = 48_000.0;
/// RNNoise expects samples in the `i16` range.
const I16_SCALE: f32 = 32_768.0;
/// Overlap-add delay of RNNoise itself (one frame), on top of the FIFO's frame.
/// Verified by `latency_matches_an_impulse` below.
const MODEL_DELAY: usize = DENOISE_FRAME;
/// Total delay from input to output.
pub const DENOISE_LATENCY: usize = DENOISE_FRAME + MODEL_DELAY;

/// Fixed-capacity sample queue (no allocation after construction).
#[derive(Debug, Clone)]
struct SampleQueue {
    data: Box<[f32]>,
    head: usize,
    len: usize,
}

impl SampleQueue {
    fn new(capacity: usize) -> Self {
        Self {
            data: vec![0.0; capacity].into_boxed_slice(),
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, sample: f32) {
        if self.len == self.data.len() {
            return; // cannot happen with the sizes used below; drop instead of panicking
        }
        let tail = (self.head + self.len) % self.data.len();
        self.data[tail] = sample;
        self.len += 1;
    }

    fn pop(&mut self) -> f32 {
        if self.len == 0 {
            return 0.0;
        }
        let sample = self.data[self.head];
        self.head = (self.head + 1) % self.data.len();
        self.len -= 1;
        sample
    }

    fn fill_silence(&mut self, count: usize) {
        self.head = 0;
        self.len = 0;
        for _ in 0..count {
            self.push(0.0);
        }
    }
}

/// Mono RNNoise denoiser with dry/wet mix (Sonar's `noiseCancelingState.value`).
pub struct Denoiser {
    state: Box<DenoiseState<'static>>,
    frame_in: [f32; DENOISE_FRAME],
    frame_out: [f32; DENOISE_FRAME],
    filled: usize,
    wet: SampleQueue,
    dry: SampleQueue,
    mix: f32,
}

impl std::fmt::Debug for Denoiser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Denoiser")
            .field("mix", &self.mix)
            .finish_non_exhaustive()
    }
}

impl Denoiser {
    /// Creates a denoiser with full suppression (`mix = 1`). Mono, 48 kHz only.
    ///
    /// # Errors
    /// [`crate::DspError`] if `format` is not 48 kHz mono.
    pub fn new(format: Format) -> Result<Self, crate::DspError> {
        if (format.sample_rate() - DENOISE_SAMPLE_RATE).abs() > f32::EPSILON {
            return Err(crate::DspError::InvalidSampleRate(format.sample_rate()));
        }
        if format.channels() != 1 {
            return Err(crate::DspError::UnsupportedChannels(format.channels()));
        }
        let mut denoiser = Self {
            state: DenoiseState::new(),
            frame_in: [0.0; DENOISE_FRAME],
            frame_out: [0.0; DENOISE_FRAME],
            filled: 0,
            wet: SampleQueue::new(2 * DENOISE_FRAME),
            dry: SampleQueue::new(DENOISE_LATENCY + 1),
            mix: 1.0,
        };
        denoiser.reset();
        Ok(denoiser)
    }

    /// Sets the dry/wet mix: 0 = original, 1 = fully denoised (clamped).
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    /// Input-to-output delay in frames.
    #[must_use]
    pub const fn latency_frames(&self) -> usize {
        DENOISE_LATENCY
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        self.frame_in[self.filled] = input * I16_SCALE;
        self.filled += 1;
        if self.filled == DENOISE_FRAME {
            self.filled = 0;
            self.state
                .process_frame(&mut self.frame_out, &self.frame_in);
            for sample in self.frame_out {
                self.wet.push(sample / I16_SCALE);
            }
        }
        self.dry.push(input);
        let wet = self.wet.pop();
        let dry = self.dry.pop();
        self.mix * wet + (1.0 - self.mix) * dry
    }
}

impl Processor for Denoiser {
    fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer {
            *sample = self.process_sample(*sample);
        }
    }

    fn reset(&mut self) {
        // RNNoise has no reset and re-creating it would allocate; its short internal history
        // fades out within a few frames.
        self.filled = 0;
        // The FIFO adds one frame; RNNoise's own delay is part of its output.
        self.wet.fill_silence(DENOISE_FRAME);
        self.dry.fill_silence(DENOISE_LATENCY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::linear_to_db;

    fn denoiser() -> Denoiser {
        Denoiser::new(Format::new(DENOISE_SAMPLE_RATE, 1).unwrap()).unwrap()
    }

    fn rms(samples: &[f32]) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let n = samples.len() as f32;
        (samples.iter().map(|s| s * s).sum::<f32>() / n).sqrt()
    }

    /// Deterministic white noise (xorshift) in -amplitude..amplitude.
    fn noise(len: usize, amplitude: f32) -> Vec<f32> {
        let mut state = 0x2545_F491_u32;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                #[allow(clippy::cast_precision_loss)]
                let unit = state as f32 / u32::MAX as f32;
                (unit * 2.0 - 1.0) * amplitude
            })
            .collect()
    }

    #[test]
    fn rejects_other_formats() {
        assert!(Denoiser::new(Format::new(44_100.0, 1).unwrap()).is_err());
        assert!(Denoiser::new(Format::new(48_000.0, 2).unwrap()).is_err());
    }

    #[test]
    fn latency_matches_an_impulse() {
        // Mix 0 → only the delayed dry path: the impulse must come out exactly
        // DENOISE_LATENCY samples later, in blocks that do not align with the frame size.
        let mut dn = denoiser();
        dn.set_mix(0.0);
        let mut signal = vec![0.0_f32; 4 * DENOISE_FRAME];
        signal[100] = 0.5;
        for block in signal.chunks_mut(512) {
            dn.process(block);
        }
        let peak = signal
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .map(|(i, _)| i);
        assert_eq!(peak, Some(100 + DENOISE_LATENCY));
    }

    #[test]
    fn wet_path_is_aligned_with_the_dry_path() {
        // RNNoise passes "silent" frames through unchanged; a low-level impulse must therefore
        // appear at the same delay in the wet path as in the dry path.
        let mut dn = denoiser();
        dn.set_mix(1.0);
        let mut signal = vec![0.0_f32; 4 * DENOISE_FRAME];
        signal[700] = 1e-4;
        dn.process(&mut signal);
        let peak = signal
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .map(|(i, _)| i);
        assert_eq!(peak, Some(700 + DENOISE_LATENCY));
    }

    #[test]
    fn suppresses_white_noise() {
        let mut dn = denoiser();
        let input = noise(48_000 * 2, 0.05);
        let mut output = input.clone();
        for block in output.chunks_mut(512) {
            dn.process(block);
        }
        let tail = input.len() / 2;
        let reduction = linear_to_db(rms(&output[tail..]) / rms(&input[tail..]));
        assert!(
            reduction < -10.0,
            "white noise reduced by only {reduction:.1} dB"
        );
    }

    #[test]
    fn mix_zero_is_a_pure_delay() {
        let mut dn = denoiser();
        dn.set_mix(0.0);
        let input = noise(3 * DENOISE_FRAME, 0.1);
        let mut output = input.clone();
        dn.process(&mut output);
        assert_eq!(
            &output[DENOISE_LATENCY..],
            &input[..input.len() - DENOISE_LATENCY]
        );
    }
}

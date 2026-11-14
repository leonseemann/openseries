//! Channel-layout conversion of an input into the output layout (v1 rules).
//!
//! * mono → every output channel
//! * otherwise channel *i* → output channel *i*; extra input channels are dropped,
//!   missing ones stay silent.

/// Copies `frames` frames of `input` (`in_channels` interleaved) into `out`
/// (`out_channels` interleaved), overwriting `out`.
pub fn convert(
    input: &[f32],
    in_channels: usize,
    out: &mut [f32],
    out_channels: usize,
    frames: usize,
) {
    if in_channels == 0 || out_channels == 0 {
        return;
    }
    let out_frames = out.chunks_exact_mut(out_channels).take(frames);
    for (out_frame, in_frame) in out_frames.zip(input.chunks_exact(in_channels)) {
        if in_channels == 1 {
            out_frame.fill(in_frame[0]);
        } else {
            out_frame.fill(0.0);
            for (o, i) in out_frame.iter_mut().zip(in_frame) {
                *o = *i;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_is_spread_to_all_outputs() {
        let mut out = [9.0; 4];
        convert(&[0.5, -0.5], 1, &mut out, 2, 2);
        assert_eq!(out, [0.5, 0.5, -0.5, -0.5]);
    }

    #[test]
    fn surround_is_folded_and_stereo_padded() {
        let mut out = [9.0; 2];
        convert(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 6, &mut out, 2, 1);
        assert_eq!(out, [1.0, 2.0]);
        let mut wide = [9.0; 6];
        convert(&[1.0, 2.0], 2, &mut wide, 6, 1);
        assert_eq!(wide, [1.0, 2.0, 0.0, 0.0, 0.0, 0.0]);
    }
}

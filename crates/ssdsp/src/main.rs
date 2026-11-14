//! `ssdsp` — run a WAV file through the StealSeries playback channel strip or, with `--mic`,
//! the mic chain (denoise → gate → stabilizer → mic EQ) for listening tests.
//!
//! Audio is processed in small blocks, exactly as the real-time engine will do it.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use ss_core::eq::ParametricEqSettings;
use ss_dsp::{
    BoostSettings, ChannelStrip, ChannelStripSettings, DENOISE_SAMPLE_RATE, Format, GateMode,
    MicChain, MicChainSettings, NoiseGate, NoiseGateSettings, Processor, SmartVolume,
    voice_stabilizer,
};

/// Frames per processing block (typical audio callback size).
const BLOCK_FRAMES: usize = 512;

/// Process a WAV file with EQ, boosts, Smart Volume, gain and an optional noise gate.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Input WAV file (16/24/32-bit integer or 32-bit float).
    input: PathBuf,
    /// Output WAV file (32-bit float).
    output: PathBuf,
    /// EQ band `FREQ:GAIN_DB:Q[:TYPE]`, TYPE = Sonar filter id 0–9 (default 7 = peaking). Repeatable, max 10.
    #[arg(long = "band", value_name = "SPEC")]
    bands: Vec<String>,
    /// Bass boost in dB (0–12).
    #[arg(long, default_value_t = 0.0)]
    bass: f32,
    /// Treble boost in dB (0–12).
    #[arg(long, default_value_t = 0.0)]
    treble: f32,
    /// Voice-clarity boost in dB (0–12).
    #[arg(long, default_value_t = 0.0)]
    clarity: f32,
    /// Smart Volume preset.
    #[arg(long)]
    smart_volume: Option<Loudness>,
    /// Output gain in dB.
    #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
    gain: f32,
    /// Enable a noise gate with this threshold in dBFS (e.g. -50).
    #[arg(long, allow_negative_numbers = true)]
    gate: Option<f32>,
    /// Use the mic chain instead of the playback strip (48 kHz input; mixed to mono).
    #[arg(long)]
    mic: bool,
    /// Mic: RNNoise denoise with this dry/wet mix 0–1 (flag alone = 0.9, Sonar's default).
    #[arg(long, value_name = "MIX", num_args = 0..=1, default_missing_value = "0.9", requires = "mic")]
    denoise: Option<f32>,
    /// Mic: automatic gate threshold = noise floor + OFFSET dB (flag alone = 10).
    #[arg(long, value_name = "OFFSET", num_args = 0..=1, default_missing_value = "10", requires = "mic", conflicts_with = "gate")]
    auto_gate: Option<f32>,
    /// Mic: volume stabilizer (voice compressor).
    #[arg(long, requires = "mic")]
    stabilizer: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Loudness {
    Soft,
    Balanced,
    Extreme,
}

impl From<Loudness> for SmartVolume {
    fn from(value: Loudness) -> Self {
        match value {
            Loudness::Soft => Self::Soft,
            Loudness::Balanced => Self::Balanced,
            Loudness::Extreme => Self::Extreme,
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.mic {
        return run_mic(&args);
    }
    let (spec, mut samples) = read_wav(&args.input)?;
    #[allow(clippy::cast_precision_loss)]
    let format = Format::new(spec.sample_rate as f32, usize::from(spec.channels))?;

    let mut strip = ChannelStrip::new(format);
    strip.set(&strip_settings(&args)?);
    let mut gate = args.gate.map(|threshold_db| {
        NoiseGate::new(
            format,
            NoiseGateSettings {
                threshold_db,
                ..NoiseGateSettings::default()
            },
        )
    });

    for block in samples.chunks_mut(BLOCK_FRAMES * format.channels()) {
        if let Some(gate) = gate.as_mut() {
            gate.process(block);
        }
        strip.process(block);
    }

    write_wav(&args.output, spec, &samples)?;
    println!("wrote {}", args.output.display());
    Ok(())
}

/// Mic mode: mono mixdown → [`MicChain`] → mono 48 kHz output.
fn run_mic(args: &Args) -> Result<()> {
    if [args.bass, args.treble, args.clarity]
        .iter()
        .any(|b| *b != 0.0)
        || args.smart_volume.is_some()
    {
        bail!("--bass/--treble/--clarity/--smart-volume belong to the playback strip, not --mic");
    }
    let (spec, samples) = read_wav(&args.input)?;
    #[allow(clippy::cast_precision_loss)]
    let rate = spec.sample_rate as f32;
    if (rate - DENOISE_SAMPLE_RATE).abs() > f32::EPSILON {
        bail!(
            "--mic needs a 48 kHz recording, got {} Hz",
            spec.sample_rate
        );
    }
    let channels = usize::from(spec.channels).max(1);
    #[allow(clippy::cast_precision_loss)]
    let scale = 1.0 / channels as f32;
    let mut mono: Vec<f32> = samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() * scale)
        .collect();

    let mut chain = MicChain::new(rate)?;
    chain.set(&mic_settings(args)?);
    for block in mono.chunks_mut(BLOCK_FRAMES) {
        chain.process(block);
    }
    // Remove the chain's latency so the output lines up with the input for A/B comparison.
    let latency = chain.latency_frames().min(mono.len());
    mono.drain(..latency);
    mono.extend(std::iter::repeat_n(0.0, latency));

    let out_spec = hound::WavSpec {
        channels: 1,
        ..spec
    };
    write_wav(&args.output, out_spec, &mono)?;
    println!(
        "wrote {} (mono, mic chain latency {} frames compensated)",
        args.output.display(),
        latency
    );
    Ok(())
}

fn mic_settings(args: &Args) -> Result<MicChainSettings> {
    let gate = match (args.gate, args.auto_gate) {
        (Some(threshold_db), _) => Some(GateMode::Manual { threshold_db }),
        (None, Some(offset_db)) => Some(GateMode::Automatic { offset_db }),
        (None, None) => None,
    };
    let mut eq = eq_settings(&args.bands)?;
    eq.enabled = !args.bands.is_empty();
    Ok(MicChainSettings {
        denoise: args.denoise.map(|mix| mix.clamp(0.0, 1.0)),
        gate,
        stabilizer: args.stabilizer.then(voice_stabilizer),
        eq,
        gain_db: args.gain,
    })
}

fn strip_settings(args: &Args) -> Result<ChannelStripSettings> {
    Ok(ChannelStripSettings {
        eq: eq_settings(&args.bands)?,
        boosts: BoostSettings {
            bass_db: args.bass,
            treble_db: args.treble,
            clarity_db: args.clarity,
        },
        smart_volume: args.smart_volume.map(SmartVolume::from),
        gain_db: args.gain,
    })
}

/// Builds EQ settings: listed bands replace the default bands from the lowest slot up;
/// remaining slots stay flat.
fn eq_settings(specs: &[String]) -> Result<ParametricEqSettings> {
    Ok(ParametricEqSettings::from_band_specs(specs)?)
}

fn read_wav(path: &PathBuf) -> Result<(hound::WavSpec, Vec<f32>)> {
    let mut reader =
        hound::WavReader::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let spec = reader.spec();
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / f64::from(1_u32 << (spec.bits_per_sample - 1));
            reader
                .samples::<i32>()
                .map(|s| {
                    #[allow(clippy::cast_possible_truncation)]
                    s.map(|v| (f64::from(v) * scale) as f32)
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    Ok((spec, samples))
}

fn write_wav(path: &PathBuf, input: hound::WavSpec, samples: &[f32]) -> Result<()> {
    let spec = hound::WavSpec {
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
        ..input
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mic_flag_defaults_match_the_dsp_defaults() {
        let args = Args::parse_from([
            "ssdsp",
            "in.wav",
            "out.wav",
            "--mic",
            "--denoise",
            "--auto-gate",
        ]);
        let settings = mic_settings(&args).unwrap();
        assert_eq!(settings.denoise, Some(ss_dsp::DEFAULT_DENOISE_MIX));
        assert_eq!(
            settings.gate,
            Some(GateMode::Automatic {
                offset_db: ss_dsp::DEFAULT_AUTO_GATE_OFFSET_DB
            })
        );
        assert!(!settings.eq.enabled, "no bands → mic EQ off");
    }

    #[test]
    fn mic_options_require_mic_mode() {
        assert!(Args::try_parse_from(["ssdsp", "a.wav", "b.wav", "--denoise"]).is_err());
        assert!(
            Args::try_parse_from([
                "ssdsp",
                "a.wav",
                "b.wav",
                "--mic",
                "--gate",
                "-50",
                "--auto-gate"
            ])
            .is_err()
        );
    }
}

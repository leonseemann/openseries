//! `ssdsp` — run a WAV file through the StealSeries channel strip for listening tests.
//!
//! Audio is processed in small blocks, exactly as the real-time engine will do it.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use ss_core::eq::{BandGain, FilterType, Frequency, PEQ_BANDS, ParametricEqSettings, QFactor};
use ss_dsp::{
    BoostSettings, ChannelStrip, ChannelStripSettings, Format, NoiseGate, NoiseGateSettings,
    Processor, SmartVolume,
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
    if specs.len() > PEQ_BANDS {
        bail!("at most {PEQ_BANDS} bands are supported");
    }
    let mut settings = ParametricEqSettings::flat();
    for (slot, spec) in settings.bands.iter_mut().zip(specs) {
        let parts: Vec<&str> = spec.split(':').collect();
        let [freq, gain, q, rest @ ..] = parts.as_slice() else {
            bail!("band `{spec}` must be FREQ:GAIN_DB:Q[:TYPE]");
        };
        slot.frequency = Frequency::new(freq.parse()?)?;
        slot.gain = BandGain::new(gain.parse()?)?;
        slot.q = QFactor::new(q.parse()?)?;
        slot.filter = match rest.first() {
            Some(id) => FilterType::from_sonar_id(id.parse()?)
                .with_context(|| format!("unknown filter type `{id}`"))?,
            None => FilterType::Peaking,
        };
    }
    Ok(settings)
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
    fn parses_band_specs() {
        let eq = eq_settings(&["120:-3:0.7".into(), "8000:4:1:9".into()]).unwrap();
        assert!((eq.bands[0].gain.get() + 3.0).abs() < f32::EPSILON);
        assert_eq!(eq.bands[1].filter, FilterType::HighShelf);
        assert_eq!(eq.bands[2].gain.get(), 0.0);
        assert!(eq_settings(&["1000:3".into()]).is_err());
        assert!(eq_settings(&["1000:30:1".into()]).is_err());
    }
}

# DSP design (`ss-dsp`, milestone M2)

Scope: playback processing per channel (SON-10…13), ChatMix (SON-07 / HS-08) and the mic dynamics
(SON-21, SON-23). AI denoise (SON-19) and the mic EQ reuse these blocks and come in M6.

## 1. What we know about the original (from [sonar.md](../re/sonar.md) §3–4)

| Original | Evidence | Our decision |
|---|---|---|
| 10-band parametric EQ per channel, filter types `ByPass=0, LowPass=1, HighPass=2, BandPassPeakQ=3, BandPassPeak0dB=4, NotchFilter=5, AllPass=6, PeakingEQ=7, LowShelving=8, HighShelving=9` | Observed (enum + registry) | Same model and numeric values (preset import stays possible). Own RBJ "Audio EQ Cookbook" biquads (the `biquad` crate lacks the two distinct band-pass variants; the formulas are short and fully unit-tested). |
| Bass / treble / voice-clarity boost `{enabled, value}` | Observed (`kSet_BassBoostGainDB` …) | Implemented as low shelf / high shelf / peaking filter. **Corner frequencies are our choice** (original Unknown): bass 100 Hz, treble 8 kHz, clarity 2.5 kHz. |
| General gain (`kSet_GeneralGainDB`) | Observed | Smoothed gain stage. |
| Smart Volume `{volumeLevel, loudness: soft/balanced/extreme}`, multiband compressor | Observed (names only) | Single-band feed-forward compressor + makeup gain; three loudness presets map to threshold/ratio (our tuning). Multiband is a later refinement. |
| Mic noise gate `{threshold dB, default −60}`, volume stabilizer | Observed | Gate with hysteresis, attack/hold/release; stabilizer = compressor preset. |
| ChatMix: gain pair on Game + Chat buses; constant `AttenuationCurveCoefficient = 2.0` | Observed constant, curve Unknown | Power law: `gain = (level / 100)^2` on the attenuated side; full level keeps unity. |

## 2. Structure

```
ss-core::eq        PeqBand, FilterType, ParametricEqSettings  (domain, serialisable later)
ss-dsp
├── processor.rs   trait Processor { fn reset(); fn process(&mut self, frames: &mut [f32]) }
├── biquad_bank.rs per-channel biquad state, coefficient (re)calculation without allocation
├── peq.rs         ParametricEq: up to 10 bands → BiquadBank
├── boost.rs       Boosts (bass/treble/clarity) → BiquadBank
├── gain.rs        Gain with one-pole smoothing (no zipper noise)
├── dynamics.rs    Compressor (smart volume, mic stabilizer), NoiseGate
├── chatmix.rs     ChatMix law: dial / slider → (game gain, chat gain)
├── chain.rs       ChannelStrip = PEQ → boosts → smart volume → gain
└── level.rs       dB ↔ linear helpers
ssdsp (bin)        offline WAV processing for listening tests
```

Audio format inside the engine: interleaved `f32`, any channel count (up to 8 for 7.1).

## 3. Real-time rules

- All processors are allocated up front (`new(sample_rate, channels)`); `process` never allocates,
  locks, logs or panics.
- Parameter setters only recompute coefficients (pure arithmetic). In the daemon they are called on
  the audio thread after receiving a message from the control thread via an `rtrb` queue.
- Gains are smoothed to avoid clicks; filters keep their state across coefficient changes.

## 4. Testing

- Frequency-response tests: feed a sine, measure steady-state RMS, compare with the expected dB
  (e.g. peaking +6 dB at f0 → +6 dB ± 0.5).
- Dynamics tests: static gain curve at steady state, attack/release monotonicity.
- ChatMix tests: centre = unity, extremes, monotonic.
- `ssdsp` produces WAV files for subjective listening tests.

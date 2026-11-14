# macOS mic chain — test guide and verification log (M6)

Goal: verify the mic chain ([design](../architecture/mic-chain.md)) live on the Nova Pro mic,
through the virtual "StealSeries Microphone" ([M3b](macos-virtual-mic.md)).

## Setup

```bash
scripts/macos/hal-driver.sh install          # in a normal terminal tab (sudo)
cargo build --release -p ssmix -p ssdsp
./target/release/ssmix mic --input Arctis --denoise --auto-gate --stabilizer --probe
```

Offline A/B on a recording: `ssdsp in.wav out.wav --mic --denoise --auto-gate --stabilizer`
(48 kHz input; latency is compensated so the files line up).

## Checklist

| # | Check | Expected | Result |
|---|-------|----------|--------|
| 1 | fmt, clippy, tests incl. no-allocation test | green | ✅ Observed 2026-11-14 |
| 2 | Offline measurements (`ssdsp --mic`, synthetic noise) | noise reduced, voice kept | ✅ Observed, see [mic-chain.md §4](../architecture/mic-chain.md) |
| 3 | Apple Voice Processing spike | decision recorded | ✅ not offered, see [mic-chain.md §6](../architecture/mic-chain.md) |
| 4 | Live: `--denoise` in a QuickTime recording from StealSeries Microphone | fan/room noise clearly lower, voice natural | ⏳ |
| 5 | Live: `--auto-gate` | silence between sentences, first syllables not cut | ⏳ |
| 6 | Live: `--stabilizer` | quiet and loud speech closer in level, noise not pumped up | ⏳ |
| 7 | Full chain, no crackles / dropouts over 60 s | clean | ⏳ |
| 8 | Latency in a live call (Discord/Zoom test call) | not noticeable (chain adds 20 ms) | ⏳ |
| 9 | Music/game playback while the chain runs | not ducked, unaffected | ⏳ |

# StealSeries

Clean-room, cross-platform (Windows + macOS) reimplementation of the SteelSeries GG
feature set in Rust — with a working Sonar-style **AudioMix** on macOS.

> Not affiliated with SteelSeries ApS. No proprietary code or assets are copied;
> behaviour is reimplemented from observed interfaces for interoperability.

## Layout
- `docs/re/` — reverse-engineering findings with evidence (what GG does, how)
- `docs/architecture/` — design of the Rust rewrite

## Status

Milestone **M1 – headset CLI** (see `docs/architecture/overview.md` §9):

```bash
cargo run -p ssctl -- list                       # find the base station (read-only)
cargo run -p ssctl -- status                     # battery, ANC, connection …
cargo run -p ssctl -- set noise-control anc
cargo run -p ssctl -- set eq -- -2 0 1 2 0 0 0 1 2 3
cargo run -p ssctl -- watch --chatmix-dial       # print dial / battery events
```

Milestone **M2 – DSP** (`docs/architecture/dsp.md`):

```bash
cargo run -p ssdsp -- in.wav out.wav --band 1000:-6:1 --bass 6 --smart-volume balanced
```

Crates: `ss-core` (domain), `ss-nova-pro` (device driver), `ss-hid` (hidapi transport),
`ss-dsp` (audio processing), `ssctl` (headset CLI), `ssdsp` (offline DSP tool).

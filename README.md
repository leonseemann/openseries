# StealSeries

Clean-room, cross-platform (Windows + macOS) reimplementation of the SteelSeries GG
feature set in Rust — with a working Sonar-style **AudioMix** on macOS.

> Not affiliated with SteelSeries ApS. No proprietary code or assets are copied;
> behaviour is reimplemented from observed interfaces for interoperability.

## Install

Download the latest installer from [Releases](https://github.com/leonseemann/stealseries/releases):
`StealSeries-…-windows-x64-setup.exe` (Windows 10/11) or `StealSeries-…-macos-arm64.zip`
(Apple Silicon, macOS 14.4+). The builds are free and not signed by a paid certificate, so the OS
asks once before the first start; see [docs/architecture/release.md](docs/architecture/release.md)
§4. Afterwards the app updates itself on request (Settings → Updates).

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

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option ([ADR 0003](docs/adr/0003-license.md)). Unless you explicitly state otherwise, any
contribution intentionally submitted for inclusion in this project shall be dual licensed as above,
without any additional terms or conditions.

# OpenSeries

Clean-room, cross-platform (Windows + macOS) reimplementation of the SteelSeries GG
feature set in Rust — with a working Sonar-style **AudioMix** on macOS.

> Not affiliated with SteelSeries ApS. No proprietary code or assets are copied;
> behaviour is reimplemented from observed interfaces for interoperability.
> Formerly named StealSeries ([ADR 0007](docs/adr/0007-rename-to-openseries.md)).

## Install

Download the latest installer from [Releases](https://github.com/leonseemann/openseries/releases):
`OpenSeries-…-windows-x64-setup.exe` (Windows 10/11) or `OpenSeries-…-macos-arm64.zip`
(Apple Silicon, macOS 14.4+, signed and notarized). Free of charge. Windows shows a SmartScreen
prompt once (no paid Authenticode certificate); see
[docs/architecture/release.md](docs/architecture/release.md) §4. Afterwards the app updates itself
on request (Settings → Updates).

On macOS also via [Homebrew](https://github.com/leonseemann/homebrew-tap):

```bash
brew install --cask leonseemann/tap/openseries
```

## Layout
- `docs/re/` — reverse-engineering notes; kept in a private repository (see AGENTS.md)
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

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md); security issues go to [SECURITY.md](SECURITY.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option ([ADR 0003](docs/adr/0003-license.md)). Unless you explicitly state otherwise, any
contribution intentionally submitted for inclusion in this project shall be dual licensed as above,
without any additional terms or conditions.

Exception: `drivers/windows/OpenSeriesAudio/` is derived from Microsoft's SysVAD sample and stays
under the [Microsoft Public License](drivers/windows/OpenSeriesAudio/LICENSE-MS-PL). It is parked
and not part of the released app. The licences of all third-party crates and fonts in the app are
listed in [THIRD_PARTY_LICENSES.md](crates/ss-app/THIRD_PARTY_LICENSES.md) and shown on the app's
About page.

## Trademarks

SteelSeries, Arctis, Sonar, GameSense and ChatMix are trademarks of SteelSeries ApS. They are used
here only to describe which hardware and file formats this independent project works with. This
project is not affiliated with, endorsed by or sponsored by SteelSeries.

## Privacy

OpenSeries collects no data and has no telemetry. Its only network access is the update check
(`api.github.com`, `github.com`), which can be switched off in Settings → Updates; GitHub sees the
request like any website visit. Settings stay on your computer.

# Documentation index

| Area | Document | Status |
|------|----------|--------|
| Contributor / agent rules | [`../AGENTS.md`](../AGENTS.md) (Claude Code loads it via [`../CLAUDE.md`](../CLAUDE.md); maintainer notes are private) | ✅ |
| Reverse-engineering notes of the original GG app (overview, core, UI, engine, Sonar, feature inventory) | `re/` — **private** repository `leonseemann/openseries-re`, cloned here by maintainers ([AGENTS.md](../AGENTS.md)) | ✅ |
| Research: open-source prior art & licenses | [`research/prior-art.md`](research/prior-art.md) | ✅ |
| Architecture of the Rust rewrite | [`architecture/overview.md`](architecture/overview.md) | ✅ |
| DSP design (M2) | [`architecture/dsp.md`](architecture/dsp.md) | ✅ |
| Audio engine + platform backends (M3/M4) | [`architecture/audio-engine.md`](architecture/audio-engine.md) | ✅ |
| Mic chain: denoise, gate, stabilizer, mic EQ (M6) | [`architecture/mic-chain.md`](architecture/mic-chain.md) | ✅ (impact NR deferred) |
| The app: menu bar, window, services, config (M5) | [`architecture/app.md`](architecture/app.md) | 🔄 in progress |
| Windows virtual audio driver + full AudioMix (M4b) | [`architecture/windows-virtual-driver.md`](architecture/windows-virtual-driver.md) | 🔄 design, user-mode prep |
| Virtual microphone driver (M3b) | [`architecture/virtual-mic-driver.md`](architecture/virtual-mic-driver.md) | ✅ |
| Hardware test: Windows AudioMix (M4a session mixer) | [`hardware/windows-audiomix.md`](hardware/windows-audiomix.md) | ✅ verified 2026-11-14 |
| Hardware test: Windows app (before M4) | [`hardware/windows-app.md`](hardware/windows-app.md) | ✅ verified 2026-11-14 |
| Hardware test: macOS app (M5) | [`hardware/macos-app.md`](hardware/macos-app.md) | ⏳ |
| Hardware test: macOS mic chain (M6) | [`hardware/macos-mic-chain.md`](hardware/macos-mic-chain.md) | ✅ (typing noise + call latency deferred) |
| Hardware test: macOS virtual microphone (M3b) | [`hardware/macos-virtual-mic.md`](hardware/macos-virtual-mic.md) | ✅ verified 2026-11-14 |
| Hardware test: macOS AudioMix (M3a) | [`hardware/macos-audiomix.md`](hardware/macos-audiomix.md) | ✅ verified 2026-11-14 |
| Hardware verification: Nova Pro Wireless | [`hardware/nova-pro-wireless.md`](hardware/nova-pro-wireless.md) | 🔄 in progress |
| Continuous integration (fmt, clippy, tests on Windows/macOS) | [`architecture/ci.md`](architecture/ci.md) | ✅ |
| Devices: extensible platform for any device class and transport (ADR 0008) | [`architecture/devices.md`](architecture/devices.md) | ✅ Phase A (verified on Windows; macOS re-test pending) |
| Releases: installers, update signing, in-app updates | [`architecture/release.md`](architecture/release.md) | ✅ v0.1.0 and v0.2.0 released; in-app update verified on Windows |
| Decisions | [`adr/`](adr/) — [0001 v1 scope](adr/0001-v1-scope.md), [0002 macOS audio](adr/0002-macos-audio-strategy.md), [0003 license](adr/0003-license.md), [0004 Windows audio](adr/0004-windows-audio-strategy.md), [0005 one app, egui](adr/0005-single-app-pure-rust-ui.md), [0006 installers + updates](adr/0006-distribution-and-updates.md), [0007 rename to OpenSeries](adr/0007-rename-to-openseries.md), [0008 device platform (proposed)](adr/0008-multi-device-support.md) | ✅ |

## Evidence convention

Every finding in the (private) `re/` notes is tagged:

- **Observed** — directly seen in a shipped artifact; the source is cited.
- **Inferred** — a conclusion drawn from observations; the reasoning is given.
- **Unknown** — open question; what would resolve it is noted.

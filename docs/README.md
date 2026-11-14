# Documentation index

| Area | Document | Status |
|------|----------|--------|
| Contributor / agent rules | [`../AGENTS.md`](../AGENTS.md), [`../CLAUDE.md`](../CLAUDE.md) (current task) | ✅ |
| RE: overview of the original GG install | [`re/overview.md`](re/overview.md) | ✅ |
| RE: GG core host (.NET) | [`re/gg-core.md`](re/gg-core.md) | ✅ |
| RE: Desktop UI (Electron) | [`re/frontend.md`](re/frontend.md) | ✅ |
| RE: Device engine (HID, GameSense) | [`re/engine.md`](re/engine.md) | ✅ |
| RE: Sonar / AudioMix | [`re/sonar.md`](re/sonar.md) | ✅ |
| RE: Feature inventory (scope decided) | [`re/feature-inventory.md`](re/feature-inventory.md) | ✅ |
| Research: open-source prior art & licenses | [`research/prior-art.md`](research/prior-art.md) | ✅ |
| Architecture of the Rust rewrite | [`architecture/overview.md`](architecture/overview.md) | ✅ |
| DSP design (M2) | [`architecture/dsp.md`](architecture/dsp.md) | ✅ |
| Audio engine + platform backends (M3/M4) | [`architecture/audio-engine.md`](architecture/audio-engine.md) | ✅ |
| Virtual microphone driver (M3b) | [`architecture/virtual-mic-driver.md`](architecture/virtual-mic-driver.md) | 🔄 in progress |
| Hardware test: macOS virtual microphone (M3b) | [`hardware/macos-virtual-mic.md`](hardware/macos-virtual-mic.md) | ⏳ |
| Hardware test: macOS AudioMix (M3a) | [`hardware/macos-audiomix.md`](hardware/macos-audiomix.md) | ✅ verified 2026-11-14 |
| Hardware verification: Nova Pro Wireless | [`hardware/nova-pro-wireless.md`](hardware/nova-pro-wireless.md) | 🔄 in progress |
| Decisions | [`adr/`](adr/) — [0001 v1 scope](adr/0001-v1-scope.md), [0002 macOS audio](adr/0002-macos-audio-strategy.md), [0003 license](adr/0003-license.md) | ✅ |

## Evidence convention

Every finding in `re/` is tagged:

- **Observed** — directly seen in a shipped artifact; the source is cited.
- **Inferred** — a conclusion drawn from observations; the reasoning is given.
- **Unknown** — open question; what would resolve it is noted.

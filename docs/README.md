# Documentation index

| Area | Document | Status |
|------|----------|--------|
| Contributor / agent rules | [`../AGENTS.md`](../AGENTS.md) | ✅ |
| RE: overview of the original GG install | [`re/overview.md`](re/overview.md) | ✅ |
| RE: GG core host (.NET) | [`re/gg-core.md`](re/gg-core.md) | ✅ |
| RE: Desktop UI (Electron) | [`re/frontend.md`](re/frontend.md) | ✅ |
| RE: Device engine (HID, GameSense) | [`re/engine.md`](re/engine.md) | ✅ |
| RE: Sonar / AudioMix | [`re/sonar.md`](re/sonar.md) | ✅ |
| RE: Feature inventory (scope decided) | [`re/feature-inventory.md`](re/feature-inventory.md) | ✅ |
| Architecture of the Rust rewrite | [`architecture/overview.md`](architecture/overview.md) | ⏳ planned |
| Decisions | [`adr/`](adr/) — [0001 v1 scope](adr/0001-v1-scope.md) | ✅ |

## Evidence convention

Every finding in `re/` is tagged:

- **Observed** — directly seen in a shipped artifact; the source is cited.
- **Inferred** — a conclusion drawn from observations; the reasoning is given.
- **Unknown** — open question; what would resolve it is noted.

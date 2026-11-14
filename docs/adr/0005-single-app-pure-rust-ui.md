# ADR 0005 — One menu-bar app with a pure-Rust UI (egui)

- **Status:** Accepted (2026-11-14)
- **Deciders:** project owner
- **Supersedes:** the "daemon + Tauri UI + local API" plan in [overview.md §3–4, §7](../architecture/overview.md)

## Context

M5 turns the demos (`ssctl`, `ssmix`) into an app. The original plan had a long-running daemon
(`ss-daemon`) and a separate Tauri 2 UI (`ss-ui`) talking over a local HTTP/WS API.

Findings from M3/M6 on macOS: every capture feature is gated by TCC permissions (System Audio
Recording, Microphone) that macOS attaches to the **responsible app**. With a terminal-launched
binary the prompts did not even appear for system audio ([macos-audiomix.md](../hardware/macos-audiomix.md)).
Two processes would need two bundles and two sets of permission prompts.

Options:

| Option | Pros | Cons |
|---|---|---|
| A. Daemon (LaunchAgent) + Tauri UI + local API | UI crash cannot stop audio; matches GG | two bundles, two TCC identities, Node.js toolchain, API auth to secure |
| B. **One app, menu-bar icon, egui window** | one bundle and one TCC identity; no IPC; Rust only | a UI bug can take audio down (mitigated: UI on the main thread, audio on CoreAudio threads, panics caught at service boundaries) |
| C. One app with a Tauri window | GG-like look | Node.js toolchain; web-view memory |

UI toolkits checked: **eframe/egui 0.32** (MIT OR Apache-2.0) fits our licence; **Slint** is
GPL-3.0 or a Slint-specific royalty-free licence with attribution duties — rejected.

## Decision

- **Option B.** One binary/bundle **`StealSeries.app`** (crate `ss-app`), `LSUIElement` menu-bar app:
  tray icon always present, the egui window opens from the tray and can be closed without
  quitting.
- Crates: `eframe`/`egui` (window), `tray-icon` (menu bar), `global-hotkey` (hotkeys), `serde` +
  `serde_json` (config), `directories` (config dir) — all MIT/Apache.
- Services run **in-process** behind plain Rust APIs (command channels + shared state snapshots).
  The local HTTP API is dropped from v1; `ssctl`/`ssmix` remain developer tools.
- Autostart via a per-user LaunchAgent plist written by the app (no helper bundle needed).

## Consequences

- `ss-daemon` and `ss-ui` from overview.md §4 are replaced by `ss-app` (+ `ss-store` for config).
- `Info.plist` carries `NSAudioCaptureUsageDescription` and `NSMicrophoneUsageDescription`, so
  macOS prompts properly for the app (not the terminal).
- Windows (M4) reuses the same app crate; only the audio adapter and autostart differ.
- A Sonar-compatible local API (SON-35) stays a possible later add-on, opt-in.

# The app (M5): `StealSeries.app`

Decision: [ADR 0005](../adr/0005-single-app-pure-rust-ui.md) — one menu-bar app, egui window,
services in-process. Scope: the ✅ v1 rows of [ADR 0001](../adr/0001-v1-scope.md) that are not
DSP/driver work, plus the owner's AudioMix master switch ([overview.md §7](overview.md)).

## 1. Threads and ownership

```mermaid
flowchart LR
  subgraph main["main thread"]
    UI["egui window (view only)"]
    TRAY["tray menu + hotkeys"]
  end
  subgraph core["AppCore (Send + Sync handle)"]
    CMD["command senders"]
    SNAP["state snapshot (Mutex, copied out)"]
  end
  HS["headset thread<br/>single HID owner"]
  AU["audio-control thread<br/>owns AudioMixSession + MicFeedSession"]
  CA["CoreAudio IO threads<br/>(real time)"]
  STORE[("config.json")]
  UI & TRAY --> CMD
  UI & TRAY --> SNAP
  CMD --> HS & AU
  HS -->|state| SNAP
  AU -->|state| SNAP
  HS -->|ChatMix dial| AU
  AU -->|EngineController, rtrb| CA
  AU <--> STORE
```

- **Headset thread** — the only owner of the HID device (engine.md §10 lesson). Polls events,
  executes commands, reconnects on hot-plug, forwards dial events to the audio thread.
- **Audio-control thread** — CoreAudio sessions hold non-`Send` blocks, so one thread creates,
  updates and drops them. Live parameter changes go through `EngineController` (no rebuild);
  routing changes rebuild the taps. Every 2 s it re-scans audio processes and devices: an
  assigned app that starts later, or a reconnected headset, triggers a rebuild.
- **UI / tray / hotkeys** never touch devices: they send commands and render the latest snapshot.
  Tray and hotkey handlers act on `AppCore` directly, so they work while the window is hidden.

## 2. Features (v1)

| Area | In the app |
|---|---|
| AudioMix | **master switch** (off = no taps, no aggregate devices); output device; per-app channel assignment (by bundle id, persisted); per channel fader, mute, PEQ, boosts, Smart Volume; ChatMix from the dial; conflict warning for known tap tools |
| Mic | on/off; source mic; mic chain (denoise, gate, stabilizer, EQ, gain); status of the virtual-mic driver |
| Headset | connection, battery, ANC/transparency, sidetone, mic volume, hardware EQ preset and custom 10-band curve editor (HS-01; shown when "Custom" is selected, writes `06 33` on release, bands numbered because their centre frequencies are Unknown; kept in the base station's RAM — saving to flash `06 09` is untested), auto-off, OLED brightness (all existing `ss-nova-pro` commands) |
| App | menu-bar icon with quick toggles, window, autostart (LaunchAgent), global hotkeys (ChatMix, mute), settings saved automatically |

Notes on v1 behaviour:
- **Mic mute hotkey** toggles the mic feed (StealSeries Microphone goes silent); a click-free live
  mute needs the mic command queue (later).
- **Reconnect USB** is on the headset page, in the menu-bar menu and on a hotkey (default Ctrl+Alt+R): the manual cure for both kinds of crackling (`docs/hardware/macos-app.md` Findings).
- **ChatMix hotkeys** move a software balance in steps of 10 %; moving the headset dial overrides it.
- **Hotkey editor** (Settings → Hotkeys): "Change" records the next key combination (needs Ctrl,
  Alt or Cmd; Esc cancels; a combination used by another action is refused), "Clear" unbinds,
  "Reset to defaults". All global hotkeys are unregistered while recording so the OS does not
  swallow the combination, and re-registered live when the config changes (no restart).
- Changing mic processing restarts the mic feed briefly (no live control of the chain yet).

## 3. Configuration (`ss-store`)

`~/Library/Application Support/StealSeries/config.json`, versioned (`"version": 1`), written
atomically (temp file + rename) and debounced (500 ms). Domain types stay serde-free; `ss-store`
owns the DTOs and the mapping. Unknown/invalid files are moved aside to `config.invalid-<ts>.json`
and defaults are used (never crash on a bad file).

```jsonc
{
  "version": 1,
  "audiomix": {
    "enabled": false,                    // master switch, default OFF
    "output_device_uid": null,            // null = system default
    "assignments": { "com.spotify.client": "media", "com.hnc.Discord": "chat" },
    "channels": { "game": { "volume": 1.0, "muted": false, "strip": { /* eq, boosts, smart volume */ } } }
  },
  "mic": { "enabled": false, "input_device_uid": null, "chain": { /* MicChainSettings */ } },
  "app": { "autostart": false, "hotkeys": { "chatmix_game": "Ctrl+Alt+Left", "chatmix_chat": "Ctrl+Alt+Right", "mic_mute": "Ctrl+Alt+M" } }
}
```

## 4. Crates

| Crate | Responsibility |
|---|---|
| `ss-store` | config DTOs, load/save, migration, defaults (portable, unit-tested) |
| `ss-app` (bin) | composition root: `AppCore`, headset + audio threads, egui UI, tray, hotkeys, autostart; macOS bundle via `scripts/macos/app.sh` |

`AppCore`'s state machine (what to (re)build for which config/process/device change) is plain Rust
and unit-tested without CoreAudio; the CoreAudio calls sit behind a small trait so tests use a fake.

## 5. Bundle

`StealSeries.app/Contents/{Info.plist, MacOS/stealseries, Resources/}`; `LSUIElement = true`,
`NSAudioCaptureUsageDescription`, `NSMicrophoneUsageDescription`, `LSMinimumSystemVersion 14.4`,
ad-hoc signed for development (Developer ID + notarisation later). Built and installed to
`~/Applications` by `scripts/macos/app.sh build|install`. Ad-hoc signatures change with each build,
so macOS may ask for permissions again after an update (**Inferred**). Later: install the
virtual-mic driver from the app (admin password via the system dialog).

## 6. Platforms

The app crate is shared by macOS and Windows (ADR 0005). Everything OS-specific lives in
`crates/ss-app/src/platform/` behind the same names per OS; the rest of the app has no
`cfg(target_os)`.

| Piece | macOS | Windows |
|---|---|---|
| UI, tray, hotkeys (egui, `tray-icon`, `global-hotkey`) | ✅ | ✅ (same crates) |
| Headset control (`ss-nova-pro` over hidapi) | ✅ | ✅ (verified with `ssctl` in M1) |
| Config (`ss-store`), supervisor logic, DSP, mic chain | ✅ | ✅ (portable) |
| AudioMix + virtual mic | process taps + HAL driver | M4a session mixer (`ss-audio-windows`: per-app volumes, ChatMix; no channel DSP, no virtual mic until M4b) |
| Autostart | LaunchAgent | `HKCU\…\Run` via `reg.exe` (verified 2026-11-14) |
| Reconnect USB | IOUSBHost re-enumeration | not yet (button hidden) |
| Fix crackling automatically | `usbaudiod` underflow log stream | not yet (option disabled) |
| Show the hidden window from the tray | `ViewportCommand::Visible` | Win32 `ShowWindow` via `ss-window-windows` (see below) |

**Hidden window on Windows (Observed 2026-11-14):** eframe only runs UI frames on `WM_PAINT`, which
Windows never sends to a hidden window. After closing the window to the tray, the queued
`Visible(true)` command was therefore never applied and "Open StealSeries…" did nothing. The tray
handler now shows the window natively (`platform::NativeWindow`, HWND taken from the creation
context) before sending the viewport command.

**Known gap:** the Windows build of `ss-app` is not compiled on the owner's Mac (no Windows target
installed); it has to be checked on Windows or in CI.

## 7. Fix crackling automatically

Off by default (`app.auto_fix_crackling`). macOS: `/usr/bin/log stream` filtered to `usbaudiod`
"USB underflow" messages (no admin rights). `crackle::BurstDetector` triggers a USB reset when
≥ 20 underflows arrive within 3 s, at most once per 30 s (observed crackling episodes produce
20–50 per second; normal operation none). The Settings page shows how often it fixed crackling.

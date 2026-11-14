# macOS app — test guide and verification log (M5)

Goal: verify `StealSeries.app` ([design](../architecture/app.md), [ADR 0005](../adr/0005-single-app-pure-rust-ui.md))
on the owner's Mac with the Arctis Nova Pro Wireless.

## Setup

```bash
scripts/macos/app.sh install           # → ~/Applications/StealSeries.app, opens it
scripts/macos/hal-driver.sh install    # only for the mic checks (normal terminal tab, sudo)
```

The app lives in the menu bar (headphone icon); "Open StealSeries…" shows the window.

## Checklist

| # | Check | Expected | Result |
|---|-------|----------|--------|
| 1 | fmt, clippy, tests (store, supervisor, plan, hotkeys, autostart) | green | ✅ Observed 2026-11-14 |
| 2 | Start from `~/Applications` | menu-bar icon, no Dock icon, window opens from the menu | ✅ Observed (first `open` after reinstall failed with −600, fixed in `app.sh`) |
| 3 | Headset page | battery, ANC, sidetone, mic volume shown; changing ANC/sidetone works | ✅ Observed: values shown; ANC selection follows app changes and the headset button after the read-back fix (`2c0da0a`). Battery has only 9 hardware steps (12.5 %), shown as ≈ |
| 4 | AudioMix **off** (default) | no permission prompt, nothing routed; FineTune works normally | ⏳ |
| 5 | AudioMix **on** | macOS asks **StealSeries** (not the terminal) for audio capture; music keeps playing through the mix | ⏳ |
| 6 | Assign an app to Game/Chat | routed; ChatMix dial affects it; assignment survives an app restart and a StealSeries restart | ⏳ |
| 7 | FineTune started while AudioMix is on | AudioMix pauses with a hint; resumes after FineTune quits | ⏳ |
| 8 | Channel fader, mute, boosts, Smart Volume, EQ | audible, no rebuild glitch | ⏳ |
| 9 | Microphone page: feed on + processing | macOS asks StealSeries for the microphone; StealSeries Microphone works in QuickTime | ⏳ |
| 10 | Tray toggles AudioMix / mic with the window closed | state changes, check marks follow | ⏳ |
| 11 | Hotkeys Ctrl+Alt+←/→, Ctrl+Alt+M | ChatMix steps; mic feed toggles | ⏳ |
| 12 | Autostart on, log out/in | app starts in the menu bar | ⏳ |
| 13 | Quit from the menu | base station ChatMix returns to normal, apps play normally | ⏳ |
| 14 | Headset page → **Reconnect USB** (IOUSBHost re-enumeration, `ss-usb-macos`) | base station re-plugs in software; app reconnects; running AudioMix/mic rebuild on the new device | ⏳ in the app. Spike 2026-11-14 (scratch tool, same calls): works **without admin rights**, base back within ~1 s with a new CoreAudio device id (119 → 143), audio normal again; the base-station display did **not** restart (power stays on) — so it is a USB reset, not a power cycle. Whether it cures the 2.4 GHz crackles (owner's fix: unplug the USB cable) is Unknown until they recur |

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
| 5 | AudioMix **on** | macOS asks **StealSeries** (not the terminal) for audio capture; music keeps playing through the mix | ✅ Observed: the prompt named StealSeries; music played through the mix |
| 6 | Assign an app to Game/Chat | routed; ChatMix dial affects it; assignment survives an app restart and a StealSeries restart | 🟡 Observed: Chrome on Game gets quieter with the dial towards Chat, Spotify on Media unaffected; ChatMix slider ↔ dial ↔ display stay in sync. Restart persistence not tested yet |
| 7 | FineTune started while AudioMix is on | AudioMix pauses with a hint; resumes after FineTune quits | ⏳ |
| 8 | Channel fader, mute, boosts, Smart Volume, EQ | audible, no rebuild glitch | ⏳ |
| 9 | Microphone page: feed on + processing | macOS asks StealSeries for the microphone; StealSeries Microphone works in QuickTime | ⏳ |
| 10 | Tray toggles AudioMix / mic with the window closed | state changes, check marks follow | ⏳ |
| 11 | Hotkeys Ctrl+Alt+←/→, Ctrl+Alt+M | ChatMix steps; mic feed toggles | 🟡 Observed: ChatMix hotkeys step the mix, the slider and the base-station display; mic hotkey not tested yet |
| 12 | Autostart on, log out/in | app starts in the menu bar | ⏳ |
| 13 | Quit from the menu | base station ChatMix returns to normal, apps play normally | ⏳ |
| 14 | Headset page → **Reconnect USB** (IOUSBHost re-enumeration, `ss-usb-macos`) | base station re-plugs in software; app reconnects; running AudioMix/mic recover | ✅ Observed 2026-11-14 in the app with AudioMix on and music playing: short audio dropout, then music continued through the mix; headset page repopulated; no error. Works **without admin rights**. Power stays on (display did not restart in the spike), so it is a USB reset, not a power cycle; whether it cures the 2.4 GHz crackles (owner's fix: unplug the cable) is Unknown until they recur. The CoreAudio device id changed in the spike (119 → 143) but **not** in this run (stayed 143) — ids can be reused, so the supervisor's device-list check is a safety net, not a guarantee; here the aggregate device recovered on its own |
| 15 | Settings → **Fix crackling automatically** on; wait for a crackling episode | episode ends after ~1 s with a USB reconnect; counter increases; off = no automatic resets | ⏳ (filter verified: `log stream` with the predicate starts and stays running) |
| 16 | AudioMix → Backup outputs: add the Mac speakers, enable "Switch to the backups while the headset is turned off"; turn the headset off, then on (SON-27/28) | mix moves to the speakers within ~2 s and back to the headset; with the option off, unplugging the base station also moves it | ⏳ (logic unit-tested on Windows; macOS run pending) |
| 17 | Microphone → Backup microphones: add the MacBook mic; turn the headset off | virtual mic source switches to the MacBook mic and back | ⏳ |
| 18 | Microphone → "Make StealSeries Microphone the system default input" on, feed on, then off; also quit with it on (SON-06) | System Settings → Sound → Input shows StealSeries Microphone while the feed runs and the previous input afterwards; picking another input by hand while it runs is kept | ⏳ (supervisor logic unit-tested; CoreAudio setter compile-checked for macOS only, written on Windows) |
| 19 | Microphone page: toggle noise suppression, gate, stabilizer and drag the gain while recording in QuickTime | changes audible at once, no gap or click from a feed restart; "Started … times" counter of the mic does not grow | ⏳ (supervisor + no-allocation tests green; macOS feed compile-checked only) |

## Findings

- **Stutter with AudioMix on** (2026-11-14, owner: "komischer Sound, hackt"): caused by the
  device-change rule added for Reconnect USB — our own private aggregate device is visible in the
  app's device list, so every rebuild changed the list and triggered the next rebuild (every 2 s);
  iPhone mics coming and going did the same. Fixed in `b663c53` (own devices filtered, rebuild only
  when the device in use changes). **Observed** after the fix: "Started 1 time(s)" stays at 1,
  stutter gone (owner: "ist glaube ich weg"). Regression test
  `unrelated_devices_coming_and_going_do_not_interrupt_the_mix`.
- **Crackling = USB underflows from the base station** (2026-11-14). The owner hears crackling with
  and without StealSeries (also on Windows with the original app); "Reconnect USB" (USB reset)
  stops it. macOS's USB audio daemon logs it (`/usr/bin/log show --predicate 'process == "usbaudiod"'`;
  note: in zsh `log` is a builtin, use the full path). **Observed** around 13:45: 13:45:09–13:45:16
  continuous `AUAInputTransferManager completeBlock USB underflow -64` (~160) with an
  `AUAAudioDevice_updateTimeStamp ioDriftNS` correction every ~2 s and occasional zero-length
  transfers; 13:45:19 re-enumeration (Reconnect USB); afterwards no underflows, only sparse drift
  updates. Same bursts at 12:30, 13:30–13:32 and 13:39. **Inferred:** the base station's USB
  audio stream intermittently delivers too little data, the HAL keeps re-timing the device clock,
  which is audible as clicks; a USB reset re-synchronises it — a device firmware/USB issue, not
  StealSeries. Live watch: `/usr/bin/log stream --predicate 'process == "usbaudiod" AND eventMessage CONTAINS "underflow"'`.
- **A second kind of crackling leaves no trace on the Mac** (2026-11-14 ~13:56–13:58). With the
  automatic fix enabled and its `log stream` running, the owner heard crackling, but `usbaudiod`
  logged nothing after 13:55:51 (no underflows) and `coreaudiod` reported no overloads. Clicking
  Reconnect USB still cured it. **Inferred:** this kind originates between headset and base
  station (radio) or inside the base station, invisible to the USB host; a USB reset re-initialises
  the base station's audio path anyway. Consequence: the automatic fix covers only the underflow
  kind; Reconnect USB is now also in the menu bar and on a hotkey (Ctrl+Alt+R).
- **Zero-length transfers + `ioDriftNS` growth are not a crackling signal on their own:** after a
  stream (re)start the input endpoint reports a few zero-length transfers and the cumulative drift
  grows in 1 ms steps (e.g. 2 → 16 ms within a minute after the 13:45:19 reset) while the owner
  heard no crackling. Not used for detection.

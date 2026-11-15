# Windows — app test guide and verification log

Goal: the StealSeries app ([app.md](../architecture/app.md)) on Windows 10/11 with the Arctis Nova
Pro Wireless, before the Windows audio backend (M4) exists. CI already builds, lints and tests the
workspace on Windows ([ci.md](../architecture/ci.md)); this checks the running app.

## Setup (on the Windows PC)

```powershell
git pull
cargo build --release -p ss-app
.\target\release\stealseries.exe          # GUI, no console window
# or, for logs in the terminal:
cargo run -p ss-app                        # debug build keeps the console
```

Config file: `%APPDATA%\StealSeries\config.json`. SteelSeries GG may keep running (it shares the
HID device, see [nova-pro-wireless.md](nova-pro-wireless.md)); quit GG for the ChatMix test so it
does not take over the dial.

## Expected on Windows today

Headset control, window, tray, hotkeys, settings and autostart work; AudioMix and the virtual mic
report "not available … (milestone M4)"; Reconnect USB and the automatic crackling fix are hidden
or disabled (macOS only so far).

## Checklist

Run on 2026-11-14: Windows 11 Pro (build 26300), Nova Pro Wireless. **All checks passed** (check 3 after a fix).

| # | Check | Expected | Result |
|---|-------|----------|--------|
| 1 | CI: clippy + tests on windows-latest | green | ✅ Observed 2026-11-14 |
| 2 | Start `stealseries.exe` | window opens, tray icon in the notification area (no console) | ✅ Observed 2026-11-14 (Windows 11 Pro 26300): window and tray icon, no console |
| 3 | Close the window | app keeps running in the tray; "Open StealSeries…" brings it back | ✅ Observed after fix `0263624`: first run the tray could not reopen the hidden window (no `WM_PAINT` for hidden windows, see [app.md §6](../architecture/app.md)); now reopens and focuses |
| 4 | Headset page | battery (≈ %), ANC/transparency/off, sidetone, mic volume, EQ preset, auto-off, OLED brightness shown and changeable | ✅ Observed: all values shown and changeable |
| 5 | AudioMix / Microphone pages | switches show "not available … (milestone M4)", nothing crashes | ✅ Observed: "not available" shown, no crash |
| 6 | Hotkeys Ctrl+Alt+←/→, Ctrl+Alt+M | registered without error message in the window header | ✅ Observed: registered, no error message |
| 7 | Settings → Start at login, then sign out/in | app starts; registry value `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\StealSeries` exists | ✅ Observed: app started after sign-out/in |
| 8 | Settings → Third-party licences | list scrolls smoothly | ✅ Observed: scrolls smoothly |
| 9 | Quit from the tray | process ends; settings kept after restart | ✅ Observed: process ends, settings kept |
| 10 | Headset → EQ "Custom": custom curve editor (HS-01) | 10 sliders show the stored curve; moving one is audible and reads back after release; "Flat" resets | ✅ Observed 2026-11-14: curve written and read back (`ssctl status`); audible in an A/B test (treble cut, see [nova-pro-wireless.md](nova-pro-wireless.md)). The app now re-selects Custom after each write; confirmed audible from the app sliders after that change |
| 11 | Settings → Hotkeys: "Change" ChatMix → Game to e.g. Ctrl+Shift+G | new combination works at once (no restart), old one no longer does; Esc cancels; Ctrl+Alt+M on another action is refused; "Reset to defaults" restores | ✅ Observed 2026-11-14: all steps as expected, changes kept after restart |
| 12 | Headset → Custom EQ → Preset: "Save current" as "Test", change bands, pick "Test" | curve comes back (audible); Delete removes it; Export… writes a JSON file, Import… of that file brings it back; a file with garbage shows "import failed" | ✅ Observed 2026-11-14: save, apply (audible), delete, export, import and the error case as expected. Sonar import not yet tried with a real GG export file |
| 13 | Headset → Device | Firmware `0.3.82/1.31.0/2.2.1/2.1.0` (as in GG), USB PID 0x12e0, release 1.30 | ✅ Observed 2026-11-14 via `ssctl status` (same code path); app view not yet looked at by the owner |
| 14 | Settings → Desktop notifications → Test; then switch the base station to the other computer and back, turn the headset off and on | test toast appears; "Base station disconnected …" / "connected", "Headset disconnected" / "connected" toasts | ✅ Observed 2026-11-14 by the owner: test toast and the live headset / base-station toasts appear |
| 15 | First start after the update: "Getting started" window | checklist shows base station ✅, headset ✅/⬜, AudioMix with "Turn on", start at login; no mic/driver steps on Windows; Done closes it, Settings → "Show getting started" reopens it | ✅ Observed 2026-11-14 |
| 16 | Settings → "Reset StealSeries settings…" → Reset (with a saved headset EQ preset) | asks first; afterwards AudioMix off, hotkeys default, start at login off (Run value removed), Getting started shows again; the preset is still there; Cancel changes nothing | ✅ Observed 2026-11-14 |
| 17 | AudioMix on, then unplug the base station (or switch it to the other computer); plug it back (`app.md` §8) | while unplugged: window shows the "paused" banner, tray says "Paused", app volumes back to normal, Ctrl+Alt+←/→ do nothing; after plugging back AudioMix resumes by itself | ✅ 2026-11-15 (owner) |

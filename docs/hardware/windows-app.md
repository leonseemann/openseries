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

| # | Check | Expected | Result |
|---|-------|----------|--------|
| 1 | CI: clippy + tests on windows-latest | green | ✅ Observed 2026-11-14 |
| 2 | Start `stealseries.exe` | window opens, tray icon in the notification area (no console) | ⏳ |
| 3 | Close the window | app keeps running in the tray; "Open StealSeries…" brings it back | ⏳ |
| 4 | Headset page | battery (≈ %), ANC/transparency/off, sidetone, mic volume, EQ preset, auto-off, OLED brightness shown and changeable | ⏳ |
| 5 | AudioMix / Microphone pages | switches show "not available … (milestone M4)", nothing crashes | ⏳ |
| 6 | Hotkeys Ctrl+Alt+←/→, Ctrl+Alt+M | registered without error message in the window header | ⏳ |
| 7 | Settings → Start at login, then sign out/in | app starts; registry value `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\StealSeries` exists (untested code) | ⏳ |
| 8 | Settings → Third-party licences | list scrolls smoothly | ⏳ |
| 9 | Quit from the tray | process ends; settings kept after restart | ⏳ |

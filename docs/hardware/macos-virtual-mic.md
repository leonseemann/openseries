# macOS virtual microphone — test guide and verification log (M3b)

Goal: verify the loopback HAL plug-in (`ss-hal-driver-macos`,
[design](../architecture/virtual-mic-driver.md)) and the mic feed (`ssmix mic`) on a real Mac
with the Arctis Nova Pro Wireless.

## Install / remove

```bash
scripts/macos/hal-driver.sh install    # builds, ad-hoc signs, copies to /Library/Audio/Plug-Ins/HAL, restarts coreaudiod (sudo)
scripts/macos/hal-driver.sh uninstall  # removes it again (sudo)
```

Restarting `coreaudiod` interrupts all audio for about a second.

## Steps

```bash
cargo build --release -p ssmix
./target/release/ssmix inputs
./target/release/ssmix mic --input Arctis --probe
```

While `ssmix mic` runs, select **StealSeries Microphone** as input (System Settings → Sound →
Input, QuickTime, Discord …) and speak into the headset.

## Checklist

| # | Check | Expected | Result |
|---|-------|----------|--------|
| 1 | Build, fmt, clippy, tests incl. the HAL-style vtable smoke test | green | ✅ Observed 2026-11-14 |
| 2 | `hal-driver.sh install`; `coreaudiod` loads the plug-in | "StealSeries Microphone" listed under Sound → Input; no "Feed" device anywhere | ⏳ |
| 3 | `ssmix inputs` | StealSeries Microphone listed (1 ch); feed device hidden | ⏳ |
| 4 | `coreaudiod` stability | no crash / restart loop after install (`log show --predicate 'process == "coreaudiod"' --last 5m`) | ⏳ |
| 5 | `ssmix mic --probe`: aggregate layout (**Inferred**: mic device outputs first, then feed) | feed index matches; feed peak follows the mic peak (+ gain) | ⏳ |
| 6 | IO on the **hidden** feed device (**Inferred**) | works (input level meter of StealSeries Microphone moves while speaking) | ⏳ |
| 7 | Stop `ssmix mic` | StealSeries Microphone goes silent, no replay of old audio | ⏳ |
| 8 | Real app (Discord / Zoom / QuickTime recording) | voice clear, no crackles, no noticeable delay | ⏳ |
| 9 | `--gain-db 6` | louder in the app | ⏳ |
| 10 | `hal-driver.sh uninstall` | device gone, other audio unaffected | ⏳ |

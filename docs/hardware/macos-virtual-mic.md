# macOS virtual microphone — test guide and verification log (M3b)

Goal: verify the loopback HAL plug-in (`ss-hal-driver-macos`,
[design](../architecture/virtual-mic-driver.md)) and the mic feed (`ssmix mic`) on a real Mac
with the Arctis Nova Pro Wireless.

## Install / remove

```bash
scripts/macos/hal-driver.sh install    # builds, ad-hoc signs, copies to /Library/Audio/Plug-Ins/HAL, restarts coreaudiod (sudo)
scripts/macos/hal-driver.sh uninstall  # removes it again (sudo)
```

Restarting `coreaudiod` interrupts all audio for about a second. `sudo` needs a real terminal:
run the script in a normal terminal tab, not via Claude Code's `!` prefix (**Observed**).

Permission: the terminal running `ssmix mic` needs **Microphone** access. If it was denied once,
macOS does not ask again and the mic delivers exact digital silence (**Observed**); enable it
under *Privacy & Security → Microphone* and restart the terminal. `ssmix mic` warns after 3 s of
exact silence.

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
| 2 | `hal-driver.sh install`; `coreaudiod` loads the plug-in | "StealSeries Microphone" listed under Sound → Input; no "Feed" device anywhere | ✅ Observed: listed under Sound → Input; feed device appears in neither input nor output lists |
| 3 | `ssmix inputs` | StealSeries Microphone listed (1 ch); feed device hidden | ✅ Observed: `StealSeries Microphone 1 ch dev.stealseries.mic`; feed absent from `inputs` and `devices` |
| 4 | `coreaudiod` stability | no crash / restart loop after install (`log show --predicate 'process == "coreaudiod"' --last 5m`) | ✅ Observed: one `coreaudiod` process since the install restart, no driver errors in the log |
| 5 | `ssmix mic --probe`: aggregate layout (**Inferred**: mic device outputs first, then feed) | feed index matches; feed peak follows the mic peak (+ gain) | ✅ Observed: inputs `[1]`, outputs `[2, 1]` (headset 2 ch, then feed 1 ch), feed = #1; feed peak identical to mic peak at 0 dB (−17…−28 dBFS while speaking); 95 callbacks/s, 512 frames |
| 6 | IO on the **hidden** feed device (**Inferred**) | works (input level meter of StealSeries Microphone moves while speaking) | ✅ Observed: the input level meter of StealSeries Microphone in System Settings follows the voice |
| 7 | Stop `ssmix mic` | StealSeries Microphone goes silent, no replay of old audio | ✅ Observed: meter flat for 10 s after stopping while still speaking |
| 8 | Real app (Discord / Zoom / QuickTime recording) | voice clear, no crackles, no noticeable delay | ✅ Observed (QuickTime audio recording from StealSeries Microphone, 60 s): voice clear, no crackles or dropouts; restarting `ssmix mic` mid-recording left no audible gap. Latency not assessable from a recording; to judge in a live call later |
| 9 | `--gain-db 6` | louder in the app | ✅ Observed: second half (+6 dB) clearly louder in the same recording |
| 10 | `hal-driver.sh uninstall` | device gone, other audio unaffected | ⏳ |

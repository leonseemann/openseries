# macOS AudioMix — test guide and verification log (M3a)

Goal: verify the process-tap backend (`ss-audio-macos`, [ADR 0002](../adr/0002-macos-audio-strategy.md))
on a real Mac with the Arctis Nova Pro Wireless.

## Prerequisites

- macOS **14.4 or newer** (Apple silicon or Intel).
- Xcode Command Line Tools: `xcode-select --install`
- Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- The repository checked out on the Mac.
- Permission: on first start macOS asks to allow the terminal app to record system audio.
  If no prompt appears and output stays silent, enable the terminal under
  *System Settings → Privacy & Security → Screen & System Audio Recording → System Audio Recording Only*
  (**Inferred**, to verify).

## Steps

```bash
cargo build --release -p ssmix
./target/release/ssmix devices
./target/release/ssmix apps
./target/release/ssmix run --output "Arctis" --game com.valvesoftware.steam --chat com.hnc.Discord --dial
```

While running: play music (Media), a game (Game) and a Discord call (Chat); turn the ChatMix dial.
Ctrl-C stops the mix and all apps play normally again.

## Checklist

| # | Check | Expected | Result |
|---|-------|----------|--------|
| 1 | `cargo build` on macOS incl. hidapi and `dial.rs` (never compiled on macOS yet) | builds | ⏳ |
| 2 | `ssmix devices` | Nova Pro output listed with channel count | ⏳ |
| 3 | `ssmix apps` | running audio apps with PID / bundle id | ⏳ |
| 4 | `ssmix run` without assignments | music still audible through the headset (Media tap → engine → output) | ⏳ |
| 5 | Tapped apps are muted at the source | no doubled/echoed audio | ⏳ |
| 6 | **Input stream order** of the aggregate device (session.rs, Inferred) | headset mic buffers first, then taps in tap-list order | ⏳ |
| 7 | Output buffer is interleaved `f32` with the device's channel count | sound, not silence | ⏳ |
| 8 | `--game` / `--chat` assignment | each app ends up on its channel | ⏳ |
| 9 | `--dial` | turning the dial lowers game or chat; base station returns to normal after Ctrl-C | ⏳ |
| 10 | `--game-bass 6`, `--media-smart-volume` | audible effect | ⏳ |
| 11 | Latency / glitches under load | no crackles | ⏳ |
| 12 | Does the Nova Pro expose a separate "chat" output device on macOS? (ADR 0002, Unknown) | note device list | ⏳ |

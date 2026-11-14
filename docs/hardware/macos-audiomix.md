# macOS AudioMix — test guide and verification log (M3a)

Goal: verify the process-tap backend (`ss-audio-macos`, [ADR 0002](../adr/0002-macos-audio-strategy.md))
on a real Mac with the Arctis Nova Pro Wireless.

## Prerequisites

- macOS **14.4 or newer** (Apple silicon or Intel).
- Xcode Command Line Tools: `xcode-select --install`
- Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- The repository checked out on the Mac.
- Permission (**Observed** 2026-11-14, macOS 27.2.1, Ghostty): macOS does **not** prompt for
  system-audio capture when `ssmix` runs from a terminal. Without the permission the taps still
  mute the tapped apps but deliver exact digital silence, so nothing is audible. Add the terminal
  manually under *System Settings → Privacy & Security → Screen & System Audio Recording →
  System Audio Recording Only* and restart it. `ssmix` warns after 3 s of exact silence on all taps.
- macOS additionally asks for **microphone** access on first start, because the aggregate device
  contains the headset's input stream. The stream itself is kept closed (see Findings).

## Steps

```bash
cargo build --release -p ssmix
./target/release/ssmix devices
./target/release/ssmix apps
./target/release/ssmix run --output "Arctis" --game com.valvesoftware.steam --chat com.hnc.Discord --dial
```

Add `--probe` to print the aggregate stream layout and the peak of every input buffer once per
second (used for checks 6 and 7). Quit other tap-based tools (e.g. FineTune) before testing.

While running: play music (Media), a game (Game) and a Discord call (Chat); turn the ChatMix dial.
Ctrl-C stops the mix and all apps play normally again.

## Checklist

Run on 2026-11-14: macOS 27.2.1 (arm64), Ghostty, Nova Pro base station on USB-1. **All checks passed.**

| # | Check | Expected | Result |
|---|-------|----------|--------|
| 1 | `cargo build` on macOS incl. hidapi and `dial.rs` (never compiled on macOS yet) | builds | ✅ Observed: builds, fmt/clippy/tests green without changes (macOS 27.2.1, Apple silicon, Rust 1.99) |
| 2 | `ssmix devices` | Nova Pro output listed with channel count | ✅ Observed: `Arctis Nova Pro Wireless`, 2 ch out, 1 ch in, 48 kHz, UID `AppleUSBAudioEngine:SteelSeries:Arctis Nova Pro Wireless:2143000:1,2` (base station on USB-1) |
| 3 | `ssmix apps` | running audio apps with PID / bundle id | ✅ Observed: PIDs and bundle ids listed (e.g. `us.zoom.xos`, `com.google.Chrome.helper`); some daemons have no bundle id; `playing` flag shown |
| 4 | `ssmix run` without assignments | music still audible through the headset (Media tap → engine → output) | ✅ Observed: music audible through the Nova Pro, normal again after Ctrl-C. **Only after** granting System Audio Recording manually (first run: silent, see Prerequisites) |
| 5 | Tapped apps are muted at the source | no doubled/echoed audio | ✅ Observed: no doubling reported; without permission the source is muted and the tap is silent (total silence) |
| 6 | **Input stream order** of the aggregate device (session.rs, Inferred) | headset mic buffers first, then taps in tap-list order | ✅ Observed with `--probe`: input buffers `[1 ch, 2 ch]`; #0 rose from −50 to −22 dBFS only while speaking into the headset mic, #1 carried the music (−8…−27 dBFS). Mic first, tap second. Order among several taps: tap-list order (check 8) |
| 7 | Output buffer is interleaved `f32` with the device's channel count | sound, not silence | ✅ Observed: 1 output buffer × 2 ch (interleaved), 512 frames per callback, ≈ 95 callbacks/s at 48 kHz |
| 8 | `--game` / `--chat` assignment | each app ends up on its channel | ✅ Observed with `--probe` (`--game com.google.Chrome --chat com.spotify.client`, Zoom meeting unassigned): input buffers `[1, 2, 2, 2]` = mic, Game, Chat, Media. Chrome only on #1 (−16…−18 dBFS), Spotify only on #2 (−26 dBFS), Zoom only on #3; the Media catch-all tap correctly excludes assigned apps. Taps keep tap-list order. Sounded normal (crackles: see check 11) |
| 9 | `--dial` | turning the dial lowers game or chat; base station returns to normal after Ctrl-C | ✅ Observed: ChatMix is only reachable on the base station while a host enables it (`SetChatMixDial`); then **pressing the base-station knob** opens it. Events arrive without extra macOS permission (no Input Monitoring prompt): centre = game 100 % / chat 100 %, 12 steps of ≈ 8.3 % per side down to 0 %. Attenuation audible on the expected side; restore on Ctrl-C without errors |
| 10 | `--game-bass 6`, `--media-smart-volume` | audible effect | ✅ Observed (A/B: 10 s without ssmix, then 30 s with): bass boost clearly audible on Chrome (Game); Smart Volume raised the quiet Spotify stream (Media) to roughly the level of the other sources |
| 11 | Latency / glitches under load | no crackles | ✅ Observed after restarting the base station: no crackles in 30 s with 2 taps + DSP; IOProc steady at 95–96 callbacks/s (no gaps). Earlier crackles during check 9 attributed by the owner to a known 2.4 GHz issue of the headset (also on Windows, cured by a base-station restart) |
| 12 | Does the Nova Pro expose a separate "chat" output device on macOS? (ADR 0002, Unknown) | note device list | ✅ Observed: no — one output device (2 ch) and one input (1 ch mic) only; ChatMix must stay software-side |

## Findings and follow-ups

- **Silent taps without permission** (check 4): handled by the `ssmix` warning; a GUI app will
  need a proper permission flow (`NSAudioCaptureUsageDescription`, ADR 0002).
- **Microphone for playback-only mixing** (check 6): the headset input is part of the aggregate
  device. Fixed: `ss-audio-macos` switches the device's own input streams off for its IOProc
  (`kAudioDevicePropertyIOProcStreamUsage`). **Observed** after `tccutil reset Microphone
  com.mitchellh.ghostty`: buffer #0 stays empty (no data even while speaking; before: −47 dBFS
  noise floor), no orange microphone indicator, playback unchanged. The microphone **prompt still
  appears** — macOS asks as soon as a device with inputs is part of the aggregate. Avoiding it
  would mean removing the headset from the aggregate (separate output path with ring buffer and
  drift handling, against ADR 0002); not worth it, since the mic features (SON-19…25, M3b) need the
  permission anyway. Accepted.
- **Other tap-based tools (e.g. FineTune)** — **Inferred**, not measured: such tools also tap apps
  with "muted when tapped" and re-render them from their own process. Running both, an app
  assigned to Game would be tapped by both; the tool's re-rendered copy comes from an unassigned
  process and lands in our Media catch-all tap → doubled/echoed audio and wrong channel. Quitting
  `ssmix` removes our taps and the other tool works normally again. Recommendation: don't run them
  at the same time. Possible later work: detect known tap-based tools and warn, or offer per-app
  volume ourselves.

# ADR 0002 — macOS AudioMix: Process Taps for playback, HAL driver only for the virtual mic

- **Status:** Accepted (2026-11-14)
- **Context docs:** [sonar.md §2, §6](../re/sonar.md), [prior-art §2](../research/prior-art.md)

## Context

On Windows, Sonar uses a kernel virtual audio driver (VAD) plus an APO for DSP. Neither exists on
macOS, which is why AudioMix is Windows-only. We need, on macOS 14.4+:

1. Per-app routing of playback into Game / Chat / Media / Aux channels (SON-01, SON-05),
2. DSP + ChatMix per channel, mixed onto the physical headset (SON-02/07/10–13),
3. A processed virtual microphone that apps such as Discord can select (SON-01 mic, SON-19…25).

Options considered:

| Option | Playback channels | Pros | Cons |
|---|---|---|---|
| A. HAL driver with loopback devices (BlackHole-style) for every channel | User/OS sends apps to "StealSeries Game" etc. | Works on 12+; apps can pick a channel themselves | Driver needed for everything; per-app routing still needs a mechanism; 5 extra devices visible |
| **B. Audio Process Taps** (macOS 14.4+) | Tap selected processes, mute their original output, re-render processed mix | No driver for playback; true per-app routing; nothing to install for playback | macOS 14.4+; needs audio-capture permission (TCC) |
| C. Become the system default output (Background Music style) | One virtual default device, split by process in the driver | Works on older macOS | Complex driver, fragile, GPL references only |

## Decision

- **Playback (Game/Chat/Media/Aux): Option B.** The daemon creates one *private aggregate device*
  containing the physical output (headset) plus one process tap per channel. Taps use
  "muted when tapped", so the original app audio is replaced by our processed mix. One IOProc
  receives all tap streams and writes the output in the same callback → no ring buffers, no clock
  drift between channels (the aggregate device handles drift correction).
- **Unassigned apps** fall into the Media channel by default (a global tap excluding assigned
  processes), so "everything goes through StealSeries" when AudioMix is on.
- **Virtual microphone: a minimal AudioServerPlugIn (HAL driver)** exposing one loopback device
  "StealSeries Microphone". The daemon writes the processed mic into the device's output side; apps
  read its input side. The driver contains no DSP and no IPC (loopback inside the driver).
- **Minimum macOS: 14.4.** The playback engine requires the app to declare
  `NSAudioCaptureUsageDescription`.

## Consequences

- Milestone M3 is split: **M3a** playback via taps (no driver, can ship unsigned for development),
  **M3b** virtual-mic HAL driver (needs Developer-ID signing + notarisation for distribution; local
  development works with ad-hoc signing and `sudo` install into `/Library/Audio/Plug-Ins/HAL`).
- The mixing core is OS-independent (`ss-engine`): it takes N channel buffers and produces one
  output buffer. Windows (M4) reuses it with WASAPI process-loopback capture.
- Apps that bypass the default output (exclusive/hog mode) cannot be tapped; documented limitation.
- The Nova Pro base station exposes its own audio interface; the aggregate device uses it as the
  output sub-device. Whether its separate "chat" USB endpoint can serve as a hardware ChatMix target
  is **Unknown** and will be checked on the Mac.

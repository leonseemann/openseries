# ADR 0004 — Windows AudioMix: session mixer now, virtual audio driver for full parity

- **Status:** Accepted (2026-11-14): M4a session mixer first, then M4b own driver. Test-signing
  mode for M4b development (open question 2) is still to be confirmed by the owner before M4b starts.
- **Deciders:** project owner
- **Context docs:** [sonar.md §2.3, §6](../re/sonar.md), [prior-art §3](../research/prior-art.md),
  [ADR 0002](0002-macos-audio-strategy.md), [audio-engine.md](../architecture/audio-engine.md)

## Context

M4 brings AudioMix and the virtual microphone to Windows. On macOS (ADR 0002) a process tap
**captures an app and mutes its original output in one step** ("muted when tapped"), so
`ss-engine` can replace the app's sound with the processed mix. No driver is needed for playback.

Windows has a per-app capture API too: WASAPI **process loopback**
(`AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK`, Windows 10 2004+). It has no "mute when tapped"
flag. Before this spike the plan (sonar.md §6, option C) was: capture with process loopback, then
silence the original through the app's audio session (mute or volume 0).

## Spike: measurements (2026-11-14)

Tool: [`spikes/winaudio`](../../spikes/winaudio/src/win.rs) (`winspike`). Machine: Windows 11 Pro
build 26300, Nova Pro Wireless base station on USB, SteelSeries GG not running. Source: Chrome
playing YouTube music (process tree of 18–21 processes, audio session in pid 21716).

| # | Question | Result |
|---|---|---|
| S1 | Which endpoints does the Nova Pro expose on Windows? | **Observed:** one render endpoint `Kopfhörer (2- Arctis Nova Pro Wireless)`, 2 ch / 48 kHz, and one capture endpoint (mic, 1 ch / 48 kHz). No separate "chat" output (same as macOS, `macos-audiomix.md` check 12). ChatMix stays in software. No other render endpoint exists on this PC. |
| S2 | Does process loopback (include tree, root pid) capture the app? | **Observed:** yes. Peak matches the session meter (−10…−15 dBFS). 48 kHz f32 stereo via autoconvert, 10 ms buffer (480 frames), ~100 packets/s, no discontinuities, no stalls over 30 s. |
| S3 | Does loopback keep delivering when the session is **muted** (`ISimpleAudioVolume::SetMute`)? | **Observed: no.** Loopback went to exact digital silence (−inf) within one second of muting, while the session meter kept showing −10…−13 dBFS. It recovered right after unmuting. |
| S4 | Same with session **volume 0** (`SetMasterVolume(0)`)? | **Observed: no.** Same result: −30.5 dBFS during the volume ramp, then −inf for the rest of the phase. |

**Conclusion (Observed):** process loopback captures *after* the session volume and mute. On
Windows, an app's audio can be captured or silenced, but not both. With only the headset as a
render endpoint, nothing can be routed "somewhere inaudible" either. **Per-channel DSP on Windows
needs virtual render endpoints, i.e. a driver**, which is exactly why Sonar ships one
(sonar.md §2). The virtual microphone needs a driver anyway, as it does on macOS.

## Options

| Option | Per-channel volume, mute, ChatMix | Per-channel DSP (EQ, boosts, Smart Volume) | Virtual mic | Cost / risk |
|---|---|---|---|---|
| **A. Session mixer** (no driver): assign apps to Game/Chat/Media/Aux; drive each app session's volume = channel fader × mute × ChatMix gain | ✅ | ❌ | ❌ | Small. Documented APIs only (`IAudioSessionManager2`, `ISimpleAudioVolume`, session notifications). Changes the user's per-app mixer volumes (must restore on exit/crash). |
| **B. Own virtual audio driver**: SysVAD-derived WaveRT driver (MS-PL, C++) with Game/Chat/Media/Aux render endpoints + "OpenSeries Microphone" capture; apps routed with `IAudioPolicyConfig` (undocumented, sonar.md §2.3); our process captures the endpoints and renders the processed mix to the headset | ✅ | ✅ | ✅ | Large. Kernel C++ outside the Rust workspace. Dev/test needs **test-signing mode** (`bcdedit /set testsigning on`, usually requires Secure Boot off). Distribution needs an **EV code-signing certificate + Microsoft attestation signing** (yearly cost). A driver bug can blue-screen the PC. |
| **C. Third-party virtual cable** (e.g. VB-Audio Cable, installed by the user) as sink + routing + capture | ✅ | ✅ (one cable per channel) | ✅ (one more cable) | No driver work, but extra install, several paid cables for 4 channels + mic, licence/redistribution not ours, foreign device names. Good only as a **development stand-in** to build the B pipeline before the driver exists. |
| D. Process loopback + session mute | — | — | — | **Ruled out by S3/S4.** |

Common to B and C (**Inferred**, to measure in M4b): the virtual endpoints and the headset run on
different clocks, so the capture → render path needs a ring buffer with drift compensation
(resampling, e.g. `rubato`). macOS avoids this through the aggregate device. Per-app routing via
`IAudioPolicyConfig` is ignored by some apps (Sonar keeps a "routing-error-prone" list), so the
default-device fallback (SON-06) matters.

## Decision

1. **M4a — session mixer (option A) now.** `ss-audio-windows` implements the app's audio backend
   with what works without a driver: app list, channel assignment, channel faders, mute, ChatMix
   (slider, hotkeys and the base-station dial) applied as per-app session volumes. The app's
   AudioMix page shows the per-channel DSP controls as "needs the OpenSeries audio driver" on
   Windows. Session volumes are saved before changing them and restored on stop, quit and the
   next start after a crash.
2. **M4b — own virtual audio driver (option B)**, as a separate component (`drivers/windows/`, C++,
   MS-PL files kept MS-PL), developed in test-signing mode on the owner's PC. Until it exists,
   option C can feed the same capture → `MixEngine` → headset pipeline for development.
3. **Distribution signing** (EV certificate + attestation) is decided later, before any release
   that includes the driver.

## Update 2026-11-14: own driver parked, VB-Cable as the stand-in

The first driver build (`drivers/windows/OpenSeriesAudio`, test-signed) cannot be loaded on the
owner's PC: `bcdedit /set testsigning on` is blocked by Secure Boot, and Secure Boot must stay on
because kernel anti-cheats (Easy Anti-Cheat, Vanguard, FACEIT …) require it and refuse
test-signing mode. Testing in a Hyper-V VM was offered and declined by the owner.

Decision (owner): **develop M4b with a third-party virtual cable (VB-Audio "VB-Cable", installed
by the user, signed, loads with Secure Boot) as the channel endpoint.** The app is built so that
each channel can use *any* virtual render endpoint — VB-Cable now, our own driver later — and
channels without one keep using the M4a session mixer. Our driver stays in the repository and
resumes once it can be attestation-signed (EV certificate; cost decision before a release).

## Update 2026-11-14 (later): no per-channel DSP or virtual mic on Windows for now

The owner wants OpenSeries to stay **completely free**: no paid code-signing certificate (needed
for our own driver on PCs with Secure Boot) and no dependency on VB-Cable (donationware; bundling
needs a licence from VB-Audio). So the VB-Cable stand-in above is **not pursued**.

**Windows scope = M4a session mixer** (per-app volume by channel, mute, master, ChatMix, presets
of the headset EQ). Per-channel EQ/boosts/Smart Volume and the virtual microphone stay macOS-only;
the Windows UI says so. Kept for later: the driver in `drivers/windows/`, the `DriftBuffer`, the
verified `IAudioPolicyConfig` routing and `winspike` — they are the building blocks if a free
signing route or another approach appears.

## Consequences

- Windows gets a useful AudioMix (ChatMix and per-app volumes) quickly, but without per-channel
  EQ, boosts or Smart Volume and without the virtual mic until M4b. This is a visible difference
  from macOS.
- M4a touches the user's per-app volume settings in the Windows mixer, which persist across
  reboots. Restoring them reliably is a hard requirement, and crash recovery needs the saved
  state on disk.
- M4b adds a C++ kernel component and a signing process to the project. The Rust side
  (`ss-audio-windows`) stays a user-mode adapter, like `ss-audio-macos`.
- `spikes/winaudio` stays in the repo as the reproducible evidence for S1–S4 and can be deleted
  once `ss-audio-windows` has equivalent hardware tests.

## Open questions for the owner

1. Agree to the split M4a (session mixer) → M4b (own driver), or go straight to the driver?
2. M4b development needs test-signing mode on your PC (Secure Boot usually off while testing). Is
   that acceptable?

# Prior art — open-source projects relevant to OpenSeries

> Scope: **SteelSeries Arctis Nova Pro Wireless** (base station + headset) and the Sonar core
> (virtual channels, per-app routing, ChatMix, DSP, virtual mic) on **Windows and macOS**.
> Research date: **2026-11-14**. Repo facts (license, stars, last push, language) were read from the
> GitHub REST API (`/repos/{owner}/{repo}`), crates.io API and the repositories' `LICENSE` files on that
> date. Shallow read-only clones were inspected outside the repo (scratch directory); nothing was built,
> run, or copied.

## How to read this document

- **Verdicts**
  - **Use as dependency** — link it as a library (license compatible with MIT/Apache-2.0).
  - **Reference only** — read docs / protocol facts, re-derive them independently; **do not copy code**
    (usually because of GPL/AGPL/unclear license, or because it is another language).
  - **Avoid** — technical or legal reasons not to rely on it.
- Protocol facts taken from community projects are third-party observations. Under our evidence rules
  (`AGENTS.md`, rule 3) they count as **Inferred** for OpenSeries until confirmed against GG's own device
  specification files (`docs/re/engine.md` §4.2) or a user-approved hardware test. Facts (byte layouts,
  command IDs) are not copyrightable; the code that implements them is — we reimplement from the table
  below, not from the source files.

---

## 0. TL;DR

| Layer | Best prior art | Verdict |
|---|---|---|
| Nova Pro Wireless HID protocol | Linux-Arctis-Manager device YAML, arctis-omni-linux README, HeadsetControl, ggoled, nova-chatmix-linux | Reference only (facts consolidated in §1.3) |
| HID transport (Rust) | `hidapi` crate | Use |
| macOS virtual devices | libASPL (MIT, C++), BlackHole (GPL-3.0), Apple "Creating an Audio Server Driver Plug-in" sample | Reference; write our own Rust AudioServerPlugIn (no mature Rust crate exists) |
| macOS per-app capture | Core Audio Process Taps via `objc2-core-audio`; AudioCap (BSD-2-Clause) as reference | Use bindings; reference sample |
| Windows per-app routing | `Windows.Media.Internal.AudioPolicyConfig` (as in EarTrumpet, SoundSwitch, Sonar) | Reimplement COM vtables in Rust with `windows` crate |
| Windows audio IO | `wasapi` crate (MIT) incl. process loopback | Use |
| Windows virtual endpoints | Microsoft SysVAD (MS-PL), VirtualDrivers/Virtual-Audio-Driver (MIT + MS-PL) | Reference / fork as a separate driver component (phase 2) |
| DSP | `biquad`, `rtrb`, `rubato`, `nnnoiseless`; DeepFilterNet libDF via `tract` | Use; compressor/limiter/gate/ChatMix law: build ourselves |
| UI | Tauri 2 + `tray-icon` (or egui) | Use Tauri 2 |
| Project license | — | **MIT OR Apache-2.0** (see §7) |

---

## 1. Arctis Nova Pro Wireless HID protocol

### 1.1 Candidates

| Project | Lang | License (SPDX) | Stars | Last push | Maturity | Verdict |
|---|---|---|---|---|---|---|
| [Sapd/HeadsetControl](https://github.com/Sapd/HeadsetControl) — `lib/devices/steelseries_arctis_nova_pro_wireless.hpp` | C++ | GPL-3.0 | 1946 | 2026-11-13 | Mature, widely packaged CLI; Nova Pro: sidetone, battery, "lights", auto-off, EQ + presets | **Reference only** (GPL-3.0). Good cross-check for EQ encoding and battery offsets. |
| [elegos/Linux-Arctis-Manager](https://github.com/elegos/Linux-Arctis-Manager) — `daemon/device-configs/base_arctis_nova_pro_wireless.yaml` | Rust daemon (v3) + Python | GPL-3.0 | 230 | 2026-11-12 | Active; v3 is a Rust rewrite with a declarative device DSL; the most complete public Nova Pro command map (settings blocks, events) | **Reference only** (GPL-3.0). Best single source of facts; its *data-driven device DSL* is a good design idea (ideas are not copyrightable). |
| [LDCanetti/arctis-omni-linux](https://github.com/LDCanetti/arctis-omni-linux) — README "Protocol" | Python | MIT | 1 | 2026-11-01 | Young, small; hardware-verified table incl. **ANC/transparency writes on `12e5`** | **Reference only** (MIT would allow reuse, but it is Python and we keep clean-room discipline). Notes its command set was derived from GG device definitions and then hardware-checked. |
| [JerwuQu/ggoled](https://github.com/JerwuQu/ggoled) | Rust (`hidapi` 2.6) | GPL-3.0 | 59 | 2026-09-03 | Working OLED drawing tool + tray app for Nova Pro / Nova Elite | **Reference only** (GPL-3.0). Only public source for OLED bitmap/return-to-UI commands and the two-collection layout. |
| [Dymstro/nova-chatmix-linux](https://github.com/Dymstro/nova-chatmix-linux) (moved to git.dymstro.nl) | Python | 0BSD | 51 | 2025-08-16 | Small, stable; ChatMix dial → PipeWire sinks | **Reference only** (0BSD is maximally permissive; still Python). Origin of the ChatMix enable/dial facts. |
| [Teknesyum/HeadsetBatteryTray](https://github.com/Teknesyum/HeadsetBatteryTray) (formerly ArctisPil) | C# | AGPL-3.0 | 1 | 2026-10-10 | Windows tray (battery, ANC, transparency, mic); only 12e0 | **Reference only** (AGPL-3.0 — never copy). Useful because it is **Windows**-tested (collections `0xFFC0` vs `0xFF00`). |
| [loteran/Arctis-Sound-Manager](https://github.com/loteran/Arctis-Sound-Manager) | Python | GPL-3.0 (with derived-from-LAM notice; GitHub shows NOASSERTION) | 200 | 2026-11-13 | Large Linux GG/Sonar alternative (PipeWire); many forks | **Reference only**. Feature-parity checklist and UX ideas; its `docs/HARDWARE-QUESTIONS.md` records open hardware questions. |
| Linux kernel `drivers/hid/hid-steelseries-arctis.c` | C | GPL-2.0-or-later | — | 2026 (split driver, Nova 5X/7 support merged Aug 2026) | Upstream | **Avoid for Nova Pro**: the device table does **not** include 12e0/12e5/225d (checked `torvalds/linux@master`). Worth knowing only because it may claim battery on other Arctis models. |
| [CalcProgrammer1/OpenRGB](https://github.com/CalcProgrammer1/OpenRGB) (mirror; main repo on GitLab) | C++ | GPL-2.0 | 4691 | 2026-11-14 | Mature | **Avoid** for this scope — Nova Pro Wireless has no RGB lighting role in our feature set. |
| Rust crates for this headset | — | — | — | — | No standalone crate found on crates.io; LAM v3 and ggoled are Rust but GPL | — |

### 1.2 Transport facts (shared by all sources)

| Fact | Sources | Confidence |
|---|---|---|
| VID `0x1038`. PIDs: `0x12E0` Nova Pro Wireless (PC/PlayStation base), `0x12E5` Nova Pro Wireless X (Xbox), `0x225D` Nova Pro Wireless X white | [LAM yaml](https://github.com/elegos/Linux-Arctis-Manager/blob/main/daemon/device-configs/nova_pro_wireless.yaml), [omni README](https://github.com/LDCanetti/arctis-omni-linux#protocol), [HeadsetControl](https://github.com/Sapd/HeadsetControl/blob/master/lib/devices/steelseries_arctis_nova_pro_wireless.hpp) (12e0/12e5 only), [ggoled](https://github.com/JerwuQu/ggoled/blob/master/ggoled_lib/src/lib.rs) (12e0/12e5) | High (3+ sources) |
| Bootloader PIDs `0x12E1`, `0x12E7`, `0x225F` (firmware-update mode) | LAM yaml | Single source — only needed to *ignore* the device in DFU mode |
| Control/vendor HID interface = **interface 4** (`bInterfaceNumber`) on the base station | nova-chatmix (`INTERFACE = 0x4`), LAM, HeadsetControl (`interface_id = 4`), HeadsetBatteryTray | High |
| Interface 4 has **two vendor top-level collections**: usage page **`0xFFC0`** (commands + OLED, output reports) and **`0xFF00`** (info/events, input reports). On Windows/macOS each collection is a separate HID device path; open both. | [HeadsetBatteryTray README](https://github.com/Teknesyum/HeadsetBatteryTray#how-it-works), ggoled (selects the collection whose descriptor byte 1 is `0xC0` vs `0x00`), omni README | High |
| Host → device: **output report ID `0x06`**, payload `[0x06, cmd, args…]` zero-padded to 64 bytes (HeadsetControl pads to 31, nova-chatmix to 63; device tolerates short reports) | all | High (pad to the descriptor's output-report length) |
| Replies to a command come back with report ID **`0x06`** and the same `cmd` byte; **unsolicited events** come with report ID **`0x07`**, interleaved — filter on both bytes | ggoled `parse_event`, omni README, nova-chatmix (`RX = 0x7`) | High |
| `0x06 0x09` = **save to flash** after setting changes | HeadsetControl, LAM, HeadsetBatteryTray | High |
| `0x06 0xFD` = factory reset — **never send** | HeadsetBatteryTray README | Single source; treat as dangerous |

### 1.3 Command / event table (Nova Pro Wireless, report ID `0x06` out, `0x06`/`0x07` in)

Byte offsets are 0-based over the full report **including** the report-ID byte (`[0]=0x06`, `[1]=cmd`).

| Feature (our scope) | Write | Read / event | Value encoding | Sources | Notes |
|---|---|---|---|---|---|
| Battery (headset + **spare battery in base station**) | — | `06 B7` → `[2]` headset 0–8, `[3]` spare/charger slot 0–8, `[4]` charging status (1 unknown/headset off, 2 charging, 4 plugged-not-charging, 8 discharging). Event `07 B7` same layout. | 0–8 = 12.5 % steps | LAM yaml, omni README, ggoled | ggoled labels `[3]` "charging" — **conflict**; LAM + omni (hardware-checked) say spare battery level. |
| Status block | — | `06 B0` → `[2]` BT power-on default, `[3]` BT call default, `[4]` BT connection mode, `[5]` BT status, `[6]` headset battery 0–8, `[7]` spare battery 0–8, `[8]` transparency level 1–10, `[9]` mic muted, `[10]` noise mode 0/1/2, `[11]` mic-mute LED brightness, `[12]` auto-off enum, `[13]` wireless mode, `[14]` radio state (`8` = connected), `[15]` headset power state (1 disconnected, 2 charging, 4 charged, 8 on battery) | — | LAM yaml (full), omni, HeadsetControl (`[6]`, `[15]`), HeadsetBatteryTray (`[6]`,`[8]`,`[10]`,`[15]`), ggoled | Five sources agree on `[6]`, `[8]`, `[10]`, `[15]`. |
| ANC / transparency mode | `06 BD <m>` m = 0 off, 1 transparency, 2 ANC | `B0[10]` | enum | omni (**hardware-verified on 12e5, "confirmed audibly"**), HeadsetBatteryTray | omni states GG itself only *reads* this on Nova Pro Wireless; writes found by testing. Physical button changes arrive via status re-read (event id not documented). |
| Transparency level | `06 B9 <1–10>` | `B0[8]` | 1–10 | omni (verified 12e5), HeadsetBatteryTray | — |
| Sidetone | `06 39 <0–3>` | `06 20`→`[18]`; event `07 39` | 0 off, 1 low, 2 med, 3 high | HeadsetControl, LAM, omni (verified) | — |
| Mic volume | `06 37 <1–10>` | `06 20`→`[17]`; event `07 37` | 1–10 (LAM read range 0–10) | LAM, HeadsetBatteryTray, omni | — |
| Headset volume | `06 25 <att>` | `06 20`→`[3]`; event `07 25 <att>` | attenuation 0 = loudest … 56 (`0x38`) = silent | nova-chatmix, HeadsetBatteryTray, ggoled, LAM | — |
| Hardware EQ preset | `06 2E <p>` | `06 20`→`[6]`; event `07 2E` | 0 flat, 1 bass boost, 2 focus, 3 smiley, **4 custom**; LAM lists 5–18 as game presets | HeadsetControl, nova-chatmix, LAM | Custom bands require preset 4. |
| Hardware 10-band EQ (custom) | `06 33 b1…b10` | `06 20`→`[7..16]`; event `07 31 <band> <value>` (per-band change from base station) | byte = `0x14 + 2·dB`, −10…+10 dB, 0.5 dB step | HeadsetControl (`EQUALIZER_BASELINE = 0x14`), LAM (`offset_db: 10`), nova-chatmix (`(v-20)/2`) | Band centre frequencies not in any source → **Unknown**; check GG spec files. This is the *device* EQ, separate from Sonar's software PEQ. |
| Mic gain "high gain" | `06 27 <0/1>` | `06 20`→`[4]` (1/2) | — | LAM | Single source, write uses a LAM-specific transform → verify. |
| **ChatMix dial** enable | `06 49 <0/1>` (1 = hub reports dial instead of mixing) | event `07 45 <game> <chat>`; also `06 20`→`[20]`,`[21]` | 0–100 each; centre = 100/100; turning fades the other side to 0 | nova-chatmix, LAM, omni (verified) | Press dial to toggle volume ↔ ChatMix mode (nova-chatmix README). Must be disabled on exit. |
| "Sonar present" icon | `06 8D <0/1>` | `06 80`→`[7]` | — | nova-chatmix, LAM, omni | Tells base station a PC mixer runs; LAM clears it on init/shutdown. |
| Line-out mode / stream mix | `06 43 <1 chatmix / 2 stream>`, `06 47 main 0 aux mic` | `06 20`→`[19]`, `[22..25]` | 0–100 | LAM | Out of v1 scope (streamer mode); note only. |
| Auto-off (inactivity) | `06 C1 <0–6>` | `B0[12]`; event `07 C1` | 0 off, 1=1 min, 2=5, 3=10, 4=15, 5=30, 6=60 min | HeadsetControl, LAM | Two sources agree. |
| **OLED brightness** | `06 85 <1–10>` | `06 80`→`[3]`; event `07 85` | 1–10 | LAM, ggoled | Two sources agree. |
| OLED dim timer, idle brightness, home-screen type | `06 83 <enum>`, —, `06 89 <0 detailed / 1 simple>` | `06 80`→`[2]`,`[4]`,`[5]`,`[6]` | — | LAM | Single source. |
| OLED custom bitmap / return to UI | `06 93 x y w h <bitmap>` as **feature report** (1024 B, screen 128×64); `06 95` = return to SteelSeries UI | — | 1-bpp | ggoled | Out of v1 scope; note only. |
| Mic-mute LED brightness | `06 BF <1–10>` | `B0[11]` | 1–10 | LAM; HeadsetControl calls this "lights" | Naming conflict: HeadsetControl's "lights" = mic LED, not OLED. |
| Wireless mode | `06 C3 <0 speed / 1 range>` | `B0[13]` | — | LAM | — |
| Bluetooth power-on default / call default | `06 B2 <0/1>`, `06 B3 <0 off,1 on,2 auto>` | `B0[2]`,`[3]` | — | LAM | — |
| Connection change | — | event `07 B5` (`[4]` radio state, `8` = connected) → re-read `B0` | — | LAM, ggoled | — |
| Firmware version | — | `06 10` | layout undocumented | ggoled | — |

**Action items for our own RE (`docs/re/engine.md`)**: confirm each row against the Nova Pro Wireless
`.edevice` specification shipped with GG (Observed evidence), resolve the `B7[3]`/`B0[7]` naming conflict,
find the EQ band frequencies, the ANC button event, and the report sizes from the HID descriptor.

---

## 2. macOS — virtual audio devices and per-app capture

| Project | Lang | License | Stars | Last push | Maturity | Verdict |
|---|---|---|---|---|---|---|
| [ExistentialAudio/BlackHole](https://github.com/ExistentialAudio/BlackHole) | C | **GPL-3.0** (+ trademark restrictions on name/logo) | 20055 | 2026-11-07 | Very mature loopback AudioServerPlugIn (2–256 ch) | **Reference only.** Read for HAL property behaviour, clock/timestamp handling; GPL forbids copying into MIT/Apache code. Users may *install* it alongside us (separate program). |
| [gavv/libASPL](https://github.com/gavv/libASPL) | C++17 | MIT | 89 | 2025-08-17 | Stable; clean object model for AudioServerPlugIn (devices, streams, controls) | **Reference only / optional dependency.** License compatible; could be linked via a thin C++ shim, but that adds a C++ toolchain to the driver. Prefer porting its *design* to Rust. |
| [roc-streaming/roc-vad](https://github.com/roc-streaming/roc-vad) | C++ (uses libASPL) | MPL-2.0 | 69 | 2025-04-13 | Production example of a libASPL driver + daemon control over gRPC | **Reference only** (architecture: driver ↔ user-space daemon IPC). |
| [kyleneideck/BackgroundMusic](https://github.com/kyleneideck/BackgroundMusic) | C++/ObjC | **GPL-2.0** | 19395 | 2026-06-10 | Mature; per-app volume via virtual default device + helper | **Reference only.** Pre-Process-Tap approach (become the default output, re-render). Useful for "device fallback" edge cases. |
| [bitgapp/eqMac](https://github.com/bitgapp/eqMac) | Swift (+ C++ driver) | Apache-2.0 | 6836 | 2026-09-19 | Mature system EQ; driver + app | **Reference only** (Swift). License-compatible; good for EQ UX and driver-install flow. |
| Apple sample "Creating an Audio Server Driver Plug-in" (NullAudio / SimpleAudio) | C | Apple sample-code license (permissive) | — | — | Canonical HAL plug-in reference | **Reference only.** Read alongside `AudioServerPlugIn.h`. |
| [0bmxa/Pancake](https://github.com/0bmxa/Pancake) | Swift | Apache-2.0 | 63 | 2019-08-27 | Abandoned | **Avoid.** |
| [penta2himajin/tympan-aspl](https://github.com/penta2himajin/tympan-aspl) (crate `tympan-aspl` 0.1.0) | Rust | MIT OR Apache-2.0 (crate) / repo shows Apache-2.0 | 0 | 2026-05-17 | Experimental; docs say live `extern "C"` entry points still pending; 59 downloads | **Avoid as dependency** (immature); skim for API ideas. |
| Other Rust HAL drivers: [JacobLinCool/mars](https://github.com/JacobLinCool/mars) (MIT, 0★), [gqf2008/vdev](https://github.com/gqf2008/vdev) (no license), [tsirysndr/snapcaster](https://github.com/tsirysndr/snapcaster) (MIT) | Rust | — | ≤2 | 2026 | Hobby-scale | **Reference only** — they prove a Rust `cdylib` packaged as `.driver` works inside `coreaudiod` (sandboxed: no network; Developer-ID signing required). `vdev` has no license → do not read code. |
| [insidegui/AudioCap](https://github.com/insidegui/AudioCap) | Swift | BSD-2-Clause | 513 | 2025-08-07 | Canonical **Process Tap** sample (tap → private aggregate device → IOProc), permission prompt `NSAudioCaptureUsageDescription` | **Reference only** (Swift); permissive, so short snippets are fine. |
| [madsmtm/objc2](https://github.com/madsmtm/objc2) — crates `objc2-core-audio`, `objc2-core-audio-types`, `objc2-audio-toolbox` 0.3.2 | Rust | Zlib OR Apache-2.0 OR MIT | 1088 | 2026-10-17 | Auto-generated, maintained; exposes `CATapDescription`, `AudioHardwareCreateProcessTap`, aggregate-device APIs (feature `AudioHardware`) | **Use as dependency.** |
| [RustAudio/coreaudio-rs](https://github.com/RustAudio/coreaudio-rs) 0.14.2 / [coreaudio-sys](https://github.com/RustAudio/coreaudio-sys) 0.2.18 | Rust | MIT OR Apache-2.0 / MIT | 298 / 86 | 2026-06 / 2026-09 | Mature (used by cpal) | **Use** (AUHAL output, device enumeration) where objc2 lacks ergonomic wrappers; `coreaudio-sys` needs bindgen at build. |

Notes
- Process Taps: the API appears in the macOS 14.2 SDK; AudioCap and most samples require **14.4+** (stable
  permission flow). Our `sonar.md` §6 says 14.4 — keep 14.4 as the supported floor (Inferred).
- No mature Rust AudioServerPlugIn crate exists → we write a small `openseries-hal-driver` crate
  (`cdylib`, C ABI vtable `AudioServerPlugInDriverInterface`) that is a dumb ring-buffer endpoint, with DSP in
  our daemon (as `sonar.md` "Implications" #1 proposes). Requires Developer-ID signing + notarisation and an
  admin install into `/Library/Audio/Plug-Ins/HAL`.

---

## 3. Windows — routing, capture, virtual endpoints

| Project | Lang | License | Stars | Last push | Maturity | Verdict |
|---|---|---|---|---|---|---|
| [File-New-Project/EarTrumpet](https://github.com/File-New-Project/EarTrumpet) | C# | MIT **with an "Excluded Entities" preamble** (GitHub: NOASSERTION) | 11544 | 2026-11-08 | Very mature; per-app output via `RoGetActivationFactory("Windows.Media.Internal.AudioPolicyConfig")`, two IIDs `ab3d4648-e242-459f-b02f-541c70306324` (21H2+) and `2a59116d-6c4f-45e0-a74f-707e3fef9258` (downlevel), `SetPersistedDefaultAudioEndpoint(pid, flow, role, HSTRING)` for eMultimedia + eConsole, device id as `\\?\SWD#MMDEVAPI#<id>#{e6327cad-…}`; also `IPolicyConfig` `F8679F50-…` | **Reference only.** Matches exactly what we Observed in Sonar (`sonar.md` §2.3). The non-standard license preamble is a reason not to copy code. |
| [Belphemur/SoundSwitch](https://github.com/Belphemur/SoundSwitch) | C# | GPL-3.0 | 3532 | 2026-11-14 | Mature; default-device switching + per-app rules (`IAudioPolicyConfig`), hotkeys | **Reference only** (GPL). Useful for hotkey UX and app-rule identity design docs (`docs/design/app-rules/`). |
| [audiorouterdev/audio-router](https://github.com/audiorouterdev/audio-router) | C++ | GPL-3.0 | 3725 | 2021-08-16 | Abandoned; per-app routing by **DLL injection** hooking `IMMDeviceEnumerator` | **Avoid** (injection triggers anti-cheat, unmaintained, superseded by `AudioPolicyConfig`). |
| NirSoft SoundVolumeCommandLine (svcl) / SoundVolumeView | — | Freeware, closed source | — | — | Mature CLI (`/SetAppDefault`) | **Avoid** as dependency (closed, no redistribution rights); handy as a manual test oracle only. |
| [HEnquist/wasapi-rs](https://github.com/HEnquist/wasapi-rs) — crate `wasapi` 0.27.0 | Rust | MIT | 95 | 2026-11-11 | Active; shared/exclusive, event-driven, `AudioClient::new_application_loopback_client(pid, include_tree)` (`AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK`), session control | **Use as dependency** (Windows audio IO + process loopback). |
| [microsoft/windows-rs](https://github.com/microsoft/windows-rs) — crate `windows` 0.62 | Rust | MIT OR Apache-2.0 | 12918 | 2026-11-13 | Official | **Use.** `IAudioPolicyConfig`/`IPolicyConfig` are undocumented → declare them with `#[interface]` in our adapter crate (cite `sonar.md` §2.3). |
| [RustAudio/cpal](https://github.com/RustAudio/cpal) 0.18.2 | Rust | Apache-2.0 | 4023 | 2026-11-11 | Mature cross-platform IO | **Use only for simple device IO / tests**; no process loopback, routing, taps or session control — not sufficient for the mixer core. |
| [microsoft/Windows-classic-samples](https://github.com/microsoft/Windows-classic-samples) — `ApplicationLoopback` | C++ | MIT | 5688 | 2026-09-03 | Official sample for process loopback | **Reference.** |
| [microsoft/Windows-driver-samples](https://github.com/microsoft/Windows-driver-samples) — `audio/sysvad` | C/C++ | MS-PL | 7921 | 2026-11-11 | Official virtual audio driver sample (PortCls/WaveRT) | **Reference / fork base for phase-2 driver.** MS-PL files must stay MS-PL (compatible with shipping beside MIT/Apache code; **incompatible with GPL**). |
| [VirtualDrivers/Virtual-Audio-Driver](https://github.com/VirtualDrivers/Virtual-Audio-Driver) | C++ | MIT (own code) + MS-PL (SysVAD-derived) | 431 | 2026-03-03 | Beta, needs test-signing, speaker + mic endpoints | **Reference only** — demonstrates a trimmed SysVAD; not production-signed. |
| [microsoft/windows-drivers-rs](https://github.com/microsoft/windows-drivers-rs) | Rust | Apache-2.0 | 1941 | 2026-11-12 | WDK bindings; no PortCls/audio miniport story | **Avoid for audio driver now**; revisit later. |
| VB-Audio VB-Cable / Voicemeeter | — | Proprietary donationware | — | — | — | **Avoid bundling** (no redistribution); can be detected and used if the user installed it (phase-1 option). |

Notes
- Driver signing (EV certificate + Microsoft attestation signing) is the main cost of a Windows virtual
  endpoint; that is why `sonar.md` proposes phase 1 without a driver.
- Some apps ignore persisted per-app defaults (Sonar's "routing-error-prone" list: Teams, Discord) — same
  limitation exists for EarTrumpet/SoundSwitch.

---

## 4. Rust DSP and real-time plumbing

| Crate | Repo | License | Last release / push | Maturity | Verdict |
|---|---|---|---|---|---|
| `biquad` 0.6.0 | [korken89/biquad-rs](https://github.com/korken89/biquad-rs) | MIT OR Apache-2.0 | 2026-03 | Small, `no_std`, RBJ cookbook: LowPass, HighPass, BandPass, Notch, AllPass, LowShelf, HighShelf, PeakingEQ | **Use** for the 10-band PEQ (covers Sonar's `FilterType` set except the BandPass-0 dB variant — trivial to add). Alternatively own 60-line implementation. |
| `fundsp` 0.23.0 | [SamiPerttu/fundsp](https://github.com/SamiPerttu/fundsp) | MIT OR Apache-2.0 | 2026-01 | Rich graph DSL, has limiter/filters | **Reference / prototyping only** — graph building allocates; we want a fixed, hand-written per-channel chain on the audio thread. |
| `dasp` 0.11.0 | [RustAudio/dasp](https://github.com/RustAudio/dasp) | MIT OR Apache-2.0 | 2020-05 (release) | Stable but stale | **Use selectively** (`dasp_sample` conversions) or skip. |
| `nnnoiseless` 0.5.2 | [jneem/nnnoiseless](https://github.com/jneem/nnnoiseless) | BSD-3-Clause | 2025-12 | Pure-Rust RNNoise port, 480-sample frames @48 kHz, cheap | **Use** as default mic denoise (low CPU, no model download). |
| DeepFilterNet / `libDF` (crate `deep_filter`) | [Rikorose/DeepFilterNet](https://github.com/Rikorose/DeepFilterNet) | MIT OR Apache-2.0 | crate 0.2.5 (2022); repo push 2026-11-06 | High-quality NN denoise; inference via `tract`; higher latency/CPU than RNNoise | **Use (optional "AI" mode)** — depend on the git `libDF` or vendor the ONNX models (check model license per release). |
| `ort` 2.0.0-rc.15 | [pykeio/ort](https://github.com/pykeio/ort) | MIT OR Apache-2.0 | 2026-10 | ONNX Runtime bindings (CoreML / DirectML EPs) | **Use only if** we need models `tract` can't run; ships a large native lib. |
| `tract-onnx` 0.23.10 | [sonos/tract](https://github.com/sonos/tract) | MIT OR Apache-2.0 | 2026-11 | Pure Rust inference, used by libDF | **Use** (default inference engine). |
| `rtrb` 0.4.1 | [mgeier/rtrb](https://github.com/mgeier/rtrb) | MIT OR Apache-2.0 | 2026-10 | Wait-free SPSC ring buffer designed for audio | **Use** (audio ↔ control and device ↔ DSP buffers). `ringbuf` 0.6 is an alternative. |
| `rubato` 5.1.0 | [HEnquist/rubato](https://github.com/HEnquist/rubato) | MIT OR Apache-2.0 | 2026-11 | Mature async/sync resamplers, RT-safe after setup | **Use** (drift correction between virtual and physical clocks). |
| `audio_thread_priority` 0.38 | [mozilla/audio_thread_priority](https://github.com/mozilla/audio_thread_priority) | MPL-2.0 | 2026-09 | Used by Firefox (MMCSS / Mach RT) | **Use** (MPL-2.0 file-level copyleft is fine as an unmodified dependency). |
| Compressor / limiter / noise gate / smart-volume crates | — | — | — | No well-maintained, RT-safe crate found | **Build ourselves** (feed-forward RMS/peak detector, gain computer with knee, attack/release smoothing; multiband via Linkwitz-Riley split built from `biquad`). |
| ChatMix gain law | — | — | — | Sonar curve Unknown (`sonar.md` §3.2) | **Build ourselves** (document equal-power law in an ADR). |
| Virtual surround (HRTF) | — | — | — | Out of v1 scope | Later: SOFA datasets (e.g. SADIE II, MIT KEMAR) + partitioned convolution. |

---

## 5. Rust HID and UI

### 5.1 HID

| Crate | Repo | License | Version / push | Verdict |
|---|---|---|---|---|
| `hidapi` 2.6.7 | [ruabmbua/hidapi-rs](https://github.com/ruabmbua/hidapi-rs) | MIT (bundled C hidapi is triple-licensed GPL-3/BSD/HIDAPI — we elect BSD) | 2026-08 | **Use.** Mature on Windows/macOS; supports output, input and feature reports (needed for OLED `0x93`), report descriptors, `usage_page` per collection. Proven on this exact device by ggoled and LAM v3. Blocking API → run on a dedicated I/O thread. |
| `async-hid` 0.5.3 | [sidit77/async-hid](https://github.com/sidit77/async-hid) | MIT | 2026-11 | **Watch / fallback.** Pure Rust (Win32/WinRT, IOHIDManager, hidraw); feature-report API is being redesigned per its roadmap. |
| `nusb` 0.2.9 | [kevinmehall/nusb](https://github.com/kevinmehall/nusb) | Apache-2.0 OR MIT | 2026-10 | **Avoid for HID** — raw USB; on Windows HID-class interfaces are owned by `hidclass` and not openable without WinUSB (Inferred from nusb's Windows backend design). Possibly useful later for firmware/DFU only (out of scope). |

### 5.2 UI (tray + mixer)

| Framework | License | Fit | Verdict |
|---|---|---|---|
| [Tauri 2](https://github.com/tauri-apps/tauri) 2.12.5 (+ `tray-icon` 0.26, `muda` 0.21) | Apache-2.0 OR MIT | Web UI (can reuse ideas from GG's Electron UI layouts), first-class tray, ~10 MB binaries, system WebView2/WKWebView, plugin ecosystem (global shortcuts, autostart, updater) | **Recommended.** Mixer sliders/EQ curve are easy in web tech; IPC to the daemon via Tauri commands. |
| [egui/eframe](https://github.com/emilk/egui) 0.37 | MIT OR Apache-2.0 | Immediate mode, pure Rust, great for dev/debug panels (meters, EQ plots); tray via `tray-icon`; less polished look, redraw cost | **Use for an internal debug/diagnostics tool**, not the main UI. |
| [iced](https://github.com/iced-rs/iced) 0.14 | MIT | Elm-style, pure Rust; no built-in tray; API churn | **Avoid** for now. |
| [Slint](https://github.com/slint-ui/slint) 1.18 | GPL-3.0-only OR Slint Royalty-free 2.0 OR commercial | Good native look, declarative | **Avoid** — tri-license adds attribution/terms obligations; not plain MIT/Apache. |

---

## 6. Sonar local-API community tools (compatibility only)

| Project | Lang | License | Stars | Last push | What it relies on |
|---|---|---|---|---|---|
| [wex/sonar-rev](https://github.com/wex/sonar-rev) | C# | MIT | 46 | 2024-11-06 | Original write-up: read `coreProps.json` → `ggEncryptedAddress` → `GET /subApps` (TLS, self-signed) → Sonar web server URL → REST |
| [Mark7888/steelseries-sonar-py](https://github.com/Mark7888/steelseries-sonar-py) | Python | MIT | 28 | 2026-05-12 | Same discovery; `/volumeSettings/classic`, streamer mode, mute, chat-mix |
| [DataNext27/SteelSeries-NET-API](https://github.com/DataNext27/SteelSeries-NET-API) | C# | MIT | 7 | 2026-08-30 | Same discovery; volumes, routing, configs |
| [Donkaos501/Steelseries-Sonar-API](https://github.com/Donkaos501/Steelseries-Sonar-API) | Java | none (all rights reserved) | 0 | 2025-01-19 | Do not read code |
| [Mathr81/steelseries-sonar-js](https://github.com/Mathr81/steelseries-sonar-js) | — | MIT | 0 | 2024-11-23 | Port of the Python lib |
| Stream Deck | — | — | — | — | The SteelSeries GG Stream Deck plugin is proprietary; no maintained open-source Sonar Stream Deck plugin found. |

Compatibility implication: these tools discover Sonar **through GG core**, not Sonar directly. To be a
drop-in for them we would need to (a) write a compatible `coreProps.json` and (b) answer `/subApps` with a
`sonar` entry pointing at our API, in addition to the Sonar REST subset listed in `sonar.md` §6. Recommend
making this an opt-in "Sonar compatibility mode" (and only when real GG is not installed, to avoid clashing).

---

## 7. License analysis and recommendation for OpenSeries

### 7.1 Copyleft items we must not copy into an MIT/Apache codebase

| Item | License | Consequence |
|---|---|---|
| HeadsetControl, Linux-Arctis-Manager (incl. its Rust v3 daemon), ggoled, SoundSwitch, BlackHole, Arctis-Sound-Manager, audio-router | GPL-3.0 | No code copy / no linking. Facts only. |
| BackgroundMusic | GPL-2.0 | Same; additionally GPL-2.0-only code would be incompatible even with a GPL-3.0 project. |
| HeadsetBatteryTray (ArctisPil) | AGPL-3.0 | Strongest copyleft; facts only. |
| Linux kernel Arctis driver, OpenRGB | GPL-2.0(-or-later) | Facts only (and not relevant for Nova Pro). |
| Slint | GPL-3.0 / proprietary options | Avoid. |
| EarTrumpet | MIT + entity exclusion preamble | Permissive but non-standard; reference only. |
| SysVAD | MS-PL | Allowed in a separately licensed driver sub-directory; incompatible with GPL projects. |
| `audio_thread_priority` | MPL-2.0 | Fine as an unmodified dependency (file-level copyleft). |

### 7.2 Options

| Option | Pros | Cons |
|---|---|---|
| **MIT OR Apache-2.0** (Rust ecosystem default) | Matches every dependency we plan to use; Apache-2.0 patent grant; can include MS-PL driver files and MPL deps; easy contributions; lets others embed the HID/protocol crates (e.g. a future `arctis-nova-pro` crate useful to the community) | Others may fork into closed products; we cannot copy GPL code (we don't intend to — clean-room policy forbids it anyway) |
| GPL-3.0-or-later | Could legally reuse HeadsetControl / LAM / ggoled / BlackHole code; guarantees derivatives stay open | Incompatible with MS-PL (SysVAD-derived driver) and GPL-2.0-only code; conflicts with our clean-room rule (copying is what we avoid); deters reuse of our protocol crates; App-store-style distribution friction |
| MPL-2.0 | File-level copyleft, compatible with Apache deps | Less common in Rust; little benefit over MIT/Apache here |

**Recommendation: dual-license `MIT OR Apache-2.0`** for all crates, with the (future) Windows driver
sub-tree under `MS-PL` if it is SysVAD-derived. Record this in an ADR (`docs/adr/0002-license.md`).
All third-party protocol knowledge is re-derived (facts + our own GG evidence), so no GPL obligation
arises.

---

## 8. Recommended stack

| Layer (crate in our workspace) | Dependencies |
|---|---|
| Domain: device model, mixer model, profiles, presets (`*-domain`) | `serde`, `serde_json`, `thiserror` (no OS deps) |
| Nova Pro Wireless protocol encoder/decoder (`device-arctis-nova-pro`) | none (pure functions over `&[u8]`; byte table from §1.3, tests per row) |
| HID adapter (`hid-adapter`) | `hidapi` (fallback `async-hid`); dedicated I/O thread, channel to services |
| DSP (`dsp`) | `biquad`, own compressor/limiter/gate/ChatMix, `nnnoiseless`, optional `deep_filter` (libDF) + `tract-onnx`, `rubato` |
| RT plumbing | `rtrb`, `audio_thread_priority`, atomics (`portable-atomic` if needed) |
| Windows audio adapter (`audio-windows`) | `wasapi`, `windows` (own `IAudioPolicyConfig` / `IPolicyConfig` definitions) |
| macOS audio adapter (`audio-macos`) | `objc2-core-audio`, `objc2-core-audio-types`, `objc2-audio-toolbox`, `coreaudio-rs` |
| macOS HAL driver (`hal-driver-macos`, `cdylib`) | `objc2-core-audio-types` / `core-foundation`; shared-memory ring to daemon |
| Windows virtual endpoints (phase 2, separate component) | SysVAD-derived WaveRT driver in C++ (MS-PL), or none in phase 1 |
| Services / daemon | `tokio`, `axum` (Sonar-compatible localhost API, opt-in), `rusqlite`, `tracing` |
| Hotkeys | `global-hotkey` (tauri-apps, MIT/Apache) or Tauri's global-shortcut plugin |
| UI | Tauri 2 + `tray-icon`/`muda`; egui for a dev diagnostics window |

## 9. What we can reuse vs build ourselves

| Reuse (as dependency) | Reuse as knowledge (re-derive, don't copy) | Build ourselves |
|---|---|---|
| `hidapi`, `wasapi`, `windows`, `objc2-core-audio`, `coreaudio-rs`, `biquad`, `rtrb`, `rubato`, `nnnoiseless`, `libDF`/`tract`, `audio_thread_priority`, Tauri 2, `tray-icon` | Nova Pro command table (LAM, omni, HeadsetControl, ggoled, nova-chatmix); `AudioPolicyConfig` IIDs + call pattern (EarTrumpet, Sonar RE); HAL plug-in property semantics (libASPL, BlackHole, Apple sample); Process Tap flow (AudioCap); SysVAD driver structure; Sonar API discovery (`coreProps.json`/`/subApps`) | Nova Pro protocol crate + event state machine; macOS AudioServerPlugIn in Rust; mixer engine (channels, per-channel output, device fallback, ChatMix law); compressor / smart volume / noise gate / mic compressor; preset import/export (Sonar JSON); per-app routing service (both OSes); Sonar-compat API layer; UI |

Open questions carried forward: EQ band centre frequencies for the hardware EQ; ANC button event id;
whether `0x06 0x09` (save) is needed after every write or only on exit; Windows phase-1 virtual endpoint
choice (none / user-installed cable / Sonar VAD interop).

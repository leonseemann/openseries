# Windows virtual audio driver and full AudioMix (M4b)

Decision context: [ADR 0004](../adr/0004-windows-audio-strategy.md) — process loopback cannot
capture an app while its playback is silenced (S3/S4, **Observed**), so per-channel DSP and the
virtual microphone need virtual endpoints. M4a (session mixer) stays as the fallback when the
driver is not installed.

Status (2026-11-14): **parked** — the driver cannot be loaded on the owner's PC (Secure Boot must stay on for anti-cheat; see ADR 0004 update). Development continues with VB-Cable as the channel endpoint; the user-mode pipeline in §2 is the same for both. First driver build done, not yet loaded. `drivers/windows/StealSeriesAudio`
builds with `scripts/windows/driver.ps1` into a test-signed package (driver, INF, signed catalog);
`scripts/windows/driver-install.ps1` installs/removes it once test-signing mode is on.

Implemented in this first step (reduction of SysVAD):

- Render: four sinks `WaveGame/TopologyGame` … `WaveAux/TopologyAux` = speaker-style topology
  with a per-endpoint Name GUID (`stealseriestoptable.h`) + SysVAD's HDMI wave filter (stereo,
  no offload pin, loopback pin), so every stream goes through the engine mix. Windows should show
  them as "Game (StealSeries Audio)" etc. (names from `MediaCategories` in the INF; **Inferred**
  until seen on the PC).
- Capture: SysVAD's mic-in pair only, named "Microphone (StealSeries Audio)", silent (tone
  generator off) until the feed loopback is added.
- Off by default: Bluetooth HFP/USB/A2DP sideband endpoints (the code needs their defines to
  compile, so they are disabled at runtime in `adapter.cpp`), data-file saving.
- Own INF (`StealSeriesAudio.inx`, hardware id `Root\StealSeriesAudio`, service
  `stealseriesaudio`) replaces the three sample INFs.

## 1. Endpoints (one driver package `StealSeriesAudio`)

| Endpoint (friendly name) | Flow | In the driver | Used by |
|---|---|---|---|
| StealSeries Game | render | sink: accepts and discards data, reports position | apps routed to Game |
| StealSeries Chat | render | sink | Discord, Teams … |
| StealSeries Media | render | sink; becomes the **Windows default output** while AudioMix runs, so unassigned apps land here (SON-06) | everything else |
| StealSeries Aux | render | sink | optional |
| StealSeries Mic Feed | render | writes into the driver-internal loopback buffer | our process only (hidden from users as far as Windows allows — Unknown) |
| StealSeries Microphone | capture | reads the loopback buffer | Discord, Zoom, OBS … |

Format: 48 kHz, stereo for render endpoints, mono for the microphone, 32-bit float and 16-bit PCM
offered (the shared-mode engine converts). The driver holds **no DSP and no IPC**, like the macOS
HAL driver ([virtual-mic-driver.md](virtual-mic-driver.md)): only sinks and one loopback pair.

Base: Microsoft **SysVAD** sample (`audio/sysvad`, MS-PL, WaveRT + PortCls, KMDF), reduced to the
endpoints above. Lives in `drivers/windows/` as a separate C++ component; MS-PL files keep their
licence headers (compatible with shipping beside MIT/Apache code, `prior-art.md` §3). Not a Rust
workspace member.

## 2. User-mode data path (`ss-audio-windows`, M4b backend)

```mermaid
flowchart LR
  APPS["apps"] -->|"per-app default endpoint<br/>(IAudioPolicyConfig)"| VG["StealSeries Game/Chat/Media/Aux<br/>(driver sinks)"]
  VG -->|"WASAPI loopback capture,<br/>one stream per endpoint"| DB["DriftBuffer per channel<br/>(ss-engine)"]
  DB --> ENG["MixEngine<br/>(same as macOS)"]
  ENG -->|"WASAPI shared render,<br/>event driven"| HS["Arctis Nova Pro headset"]
  MIC["headset mic<br/>(WASAPI capture)"] --> CHAIN["MicChain"] -->|"WASAPI render"| FEED["StealSeries Mic Feed"]
  FEED -->|"driver loopback"| VMIC["StealSeries Microphone"]
```

1. **Routing.** Assigned apps get their per-process default render endpoint set to the channel's
   endpoint with the undocumented `IAudioPolicyConfig::SetPersistedDefaultAudioEndpoint`
   (two IIDs by Windows build, **Observed** in Sonar, `sonar.md` §2.3; EarTrumpet uses the same).
   The device id is passed as `\\?\SWD#MMDEVAPI#<endpoint id>#{e6327cad-dcec-4949-ae8a-991e976a79d2}`.
   Unassigned apps follow the Windows default output, which AudioMix switches to "StealSeries
   Media" and restores on stop (like the default-input handling on macOS). Apps that ignore
   per-app defaults (Sonar keeps a "routing-error-prone" list) stay on Media.
2. **Capture.** A standard WASAPI **loopback** stream on each virtual render endpoint returns what
   the audio engine mixed for it (**Inferred** to work for a sink driver: loopback is taken in the
   engine before the data reaches the driver).
3. **Mixing.** `MixEngine` from `ss-engine` — identical DSP, ChatMix and fader law to macOS — runs
   in the render callback of the headset stream.
4. **Clocks.** Each endpoint is paced by its own driver clock: the virtual ones by a timer, the
   headset by its USB clock. Unlike the macOS aggregate device nothing aligns them, so every
   captured channel passes through a **`DriftBuffer`** (`ss-engine`, implemented and tested now):
   an SPSC ring buffer whose reader resamples by a few hundred ppm to hold its fill level at a
   target (PI control, cubic interpolation). Target fill ≈ 2 headset periods (≈ 20 ms) — the
   latency budget is Unknown until measured.
5. **Virtual mic.** Headset mic → `MicChain` (live changes as on macOS) → render to "StealSeries
   Mic Feed" → driver loopback → "StealSeries Microphone". The same `DriftBuffer` sits between the
   mic capture and the feed render.

## 3. Development and signing

| Step | Needs | Status |
|---|---|---|
| Build | Visual Studio Build Tools 2022 (**present**), Windows SDK 10.0.26100 (**present**), **WDK** matching the SDK (missing; contains `portcls.h`, `inf2cat`) | owner OK to install |
| Load on the owner's PC | `bcdedit /set testsigning on` (admin, reboot; usually requires Secure Boot off), test certificate, `pnputil /add-driver … /install` | owner OK |
| Distribution | EV code-signing certificate + Microsoft attestation signing (Partner Center) | later, before any release |

A driver bug can crash Windows (blue screen). Development therefore starts with the sinks only,
adds the loopback pair afterwards, and every install step is reversible (`pnputil /delete-driver`,
`bcdedit /set testsigning off`).

## 4. What is prepared now (no driver needed)

- `ss_engine::DriftBuffer` with unit tests (steady state, faster/slower producer, underrun,
  no allocation on the audio path).
- `ss-audio-windows::policy` — bindings for `IAudioPolicyConfig` (get/set per-app default
  endpoint) with the vtable layout read from Sonar's interop metadata (`sonar.md` §2.3). Verified
  read-only on the owner's PC: the 21H2+ interface activates and `Get…` works (`winspike
  policy`). `Set…` verified with a reversible round trip (`winspike policy-roundtrip`, 2026-11-14): set, read back, cleared with an empty id.

## 5. Open questions

- Can "StealSeries Mic Feed" be hidden from the Sound settings? (Unknown; SysVAD has no flag
  that removes an endpoint from the UI.)
- Latency of the whole loopback → engine → headset path (target < 30 ms; Unknown).
- Behaviour of apps in exclusive mode (bypass the virtual endpoints; documented limitation).

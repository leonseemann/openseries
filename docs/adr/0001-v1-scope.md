# ADR 0001 — v1 scope: Arctis Nova Pro Wireless + Sonar core

- **Status:** Accepted (2026-11-14); architectural limit to one device lifted by
  [ADR 0008](0008-multi-device-support.md) (proposed 2026-11-15) — the supported and verified device
  is still the Arctis Nova Pro Wireless
- **Deciders:** project owner

## Context

SteelSeries GG exposes ~245 user-visible features ([feature inventory](../re/feature-inventory.md)).
The only hardware available for testing is one **Arctis Nova Pro Wireless** headset. Device HID
layouts are inside encrypted spec files we will not decrypt ([engine.md §4.2](../re/engine.md)), so
every device needs its own clean-room protocol work, which can only be verified on owned hardware.
The main pain point of the original is that Sonar / AudioMix does not work on macOS.

## Decision

v1 implements only the rows marked **✅ v1** in the inventory's *Decision* column (49 features):

- **App shell:** UI window, background service, auto-start, tray, OS notifications, global hotkeys,
  crash recovery, about/licenses (CORE-01/03/04/14/15/16, NTU-01/02/06).
- **Device framework for one headset:** detection & hot-plug, configs CRUD, global settings, battery,
  power saving, factory reset, version info, local config export/import, base-station OLED
  (DEV-01…05/08/09/12/13/22/24).
- **Headset:** hardware EQ, auto-flatten, ANC/transparency, mic & sidetone, power options,
  ChatMix dial, default-device switching (HS-01/03/04/05/06/08/18).
- **Sonar core:** virtual channels + virtual mic, classic mixer, per-channel output, mic input,
  per-app routing, OS defaults, ChatMix, 10-band PEQ, boosts, smart volume, gain, own presets with
  file import/export, AI mic denoise (open model), noise gate, mic compressor, mic EQ, device
  manager & fallback, hotkeys, onboarding, on/off (SON-01…07/10…13/15/17/19/21/23/25/27/28/30/32/36).

Rows marked **⏳ later** are candidates for v2 (e.g. Streamer mode, Spatial Audio, AEC, overlays).
Rows marked **❌** are out of scope (other device families, account/cloud, telemetry, Moments,
aim tools, licensed DSP, firmware flashing).

## Consequences

- Architecture keeps a generic device abstraction so more devices can be added later, but only
  one driver (Nova Pro Wireless) is implemented.
- macOS per-app routing requires macOS 14.4+ (Audio Process Taps).
- Prior art for the headset protocol and macOS virtual devices is evaluated in
  [`docs/research/prior-art.md`](../research/prior-art.md).

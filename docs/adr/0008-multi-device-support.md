# ADR 0008 — A device interface for more headsets

- **Status:** proposed (2026-11-15) — the owner decides; implementation starts after acceptance
- **Updates:** [ADR 0001](0001-v1-scope.md) (scope "Arctis Nova Pro Wireless only")
- **Design:** [`docs/architecture/devices.md`](../architecture/devices.md)

## Context

OpenSeries was built for one device, the owner's Arctis Nova Pro Wireless (ADR 0001). The owner
now wants the app to be **extensible to other headsets**. A review of the code (2026-11-15) shows:

- **Already device-neutral:** the domain types in `ss-core` (battery, noise control, sidetone, EQ
  presets, hardware EQ, Game/Chat balance, the `HidTransport` trait), the HID transport `ss-hid`,
  and the whole audio side (AudioMix, DSP, mic chain in `ss-engine`, `ss-dsp`, `ss-audio-*`).
- **Hard-wired to the Nova Pro:**
  - the app's headset thread uses `NovaProWireless`, its `Command` / `Status` / `Settings` /
    `Display` types and its USB ids directly;
  - the Headset page draws fixed cards (transparency strength, OLED brightness, spare battery,
    10-band EQ);
  - the audio plan finds the output by the name "Arctis Nova Pro Wireless";
  - the base-station gate (`app.md` §8), the ChatMix hand-over and the macOS USB reset use
    Nova Pro specifics;
  - stored headset EQ presets assume 10 bands.

## Decision

1. **A device interface.** A new platform-free crate `ss-device` defines what the app needs from
   any headset:
   - a static **`DeviceInfo`** (id, name, USB/HID selector, audio endpoint names, support level);
   - a **`Capabilities`** description (which settings exist, their ranges, EQ band count, …);
   - device-neutral **`Setting`**, **`DeviceState`** and **`Change`** types;
   - the trait **`HeadsetDriver`**.

   Details are in `devices.md`.
2. **One crate per device family** implements the trait. `ss-nova-pro` is the first; its
   protocol code stays as it is, and a thin adapter maps it to the interface. A registry crate
   `ss-devices` lists all drivers, so the app depends on the registry instead of individual
   drivers (dependency direction `app → registry → drivers → ss-device → ss-core`).
3. **The app works only with the interface:**
   - the device thread opens the first connected supported device;
   - the Headset and Overview pages are built from `Capabilities` and show only what the device
     can do;
   - the audio output is chosen from the driver's endpoint names;
   - headset EQ presets are stored per device id.
4. **The gate becomes "a supported device is connected".** The owner requirement of `app.md` §8
   is kept: with no supported device connected the app stays completely passive, so other
   headsets are never touched.
5. **Safety for untested devices:**
   - each driver declares a support level, `Verified` or `Experimental`;
   - experimental drivers are off until the user enables "Experimental devices" in Settings, and
     the UI marks them;
   - protocol facts taken from public sources count as **Inferred** until checked on hardware
     (AGENTS.md rule 3), and each device gets a `docs/hardware/<device>.md` test log.
6. **No behaviour change for the Nova Pro.** The refactor is complete only when the existing
   Windows and macOS hardware rows still pass.

## Alternatives considered

- **Keep one hard-wired device.** It is simplest, but every new headset would mean copying the
  headset thread and the UI.
- **Data-driven device files (YAML/JSON, as Linux-Arctis-Manager does).** Attractive for simple
  devices, but the framing, checksums, multi-report replies and the ChatMix hand-over differ too
  much between vendors. It may come later as one *generic driver* behind the same trait, without
  changing the app.
- **One generic driver with a per-device opcode table.** It would only work within one vendor
  family and would hide real protocol differences. Rejected for now.

## Consequences

- The work is a refactor, roughly 1–2 sessions, with no new features for the owner's device.
  Afterwards a second headset is "one crate + one registry entry + one hardware test log".
- Only the Nova Pro can be tested by the owner; other devices need testers with the hardware. Until
  then they ship as Experimental (off by default) or not at all.
- The config format gains a device id for headset EQ presets (migration: existing curves belong to
  `arctis-nova-pro-wireless`).
- ADR 0001 stays valid for *which* device is supported and verified today; this ADR only removes
  the architectural limit.

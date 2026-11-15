# ADR 0008 — An extensible device platform (any device class, any transport)

- **Status:** accepted (2026-11-15, owner; revised the same day after owner feedback)
- **Updates:** [ADR 0001](0001-v1-scope.md) (scope "Arctis Nova Pro Wireless only")
- **Design:** [`docs/architecture/devices.md`](../architecture/devices.md)

## Context

OpenSeries was built for one device, the owner's Arctis Nova Pro Wireless, over USB (ADR 0001). The
owner wants the app to be **as abstract and easy to extend as possible**:

- other headsets, and later **keyboards, mice and other peripherals**;
- **Bluetooth** as well as USB.

A review of the code (2026-11-15) shows:

- **Already device-neutral:** the value types in `ss-core`, the HID transport `ss-hid`, and the
  whole audio side (AudioMix, DSP, mic chain).
- **Hard-wired to the Nova Pro:**
  - the app's headset thread, which uses the `NovaProWireless` types and USB ids directly;
  - the Headset page with its fixed cards;
  - the audio output, chosen by name;
  - the base-station gate, the ChatMix hand-over and the macOS USB reset;
  - 10-band EQ presets.
- **Hard-wired to USB:** discovery only knows USB HID selectors; nothing models Bluetooth.
- **Hard-wired to "one device":** one headset thread and one `HeadsetState`.

## Decision

The app becomes a small **device platform** with three independent extension axes. Adding to one
axis never requires changes to the others or to the app core.

1. **Features: what a device can do.** A device is described by a list of **features** from a
   shared catalogue.
   - Now: battery, wireless link, noise control, sidetone, mic volume / mute / LED, equalizer,
     EQ presets, Game/Chat balance dial, auto-off, display brightness, firmware info, USB re-plug.
   - Later: lighting zones, DPI stages, polling rate, button / key remapping, macros, onboard
     profiles.

   Each feature kind has typed parameters (ranges, band count, zones) and typed values.
   The UI has **one widget per feature kind** and builds every device page from the device's
   feature list. A new feature kind is one catalogue entry, one widget and its docs. A device
   class (headset, keyboard, mouse, …) is only a label and an icon; it never drives code paths.
2. **Transports: how we talk to a device.** A transport layer finds candidates and opens
   connections, with one implementation per transport:
   - **USB HID** (hidapi, today);
   - **Bluetooth HID** (Classic and BLE "HID over GATT"). The OS exposes these as HID devices, so
     hidapi reaches them on Windows and macOS (**Inferred**, verify with the first Bluetooth
     device);
   - **Bluetooth LE GATT** for vendor-specific services (`btleplug` or `bluest`, both free
     licences), added when the first device needs it;
   - room for further transports (serial, network) behind the same traits.

   Drivers state which transport and which match rules (VID/PID, usage page, GATT service UUID,
   name) they need.
3. **Drivers: one crate per device family.** Each driver implements the `Device` trait and
   registers a `DriverEntry` (static info, match rules, constructor). The `ss-nova-pro` protocol
   code stays as it is; a thin adapter maps it to the trait. A registry crate `ss-devices` lists
   all drivers. Dependencies run `app → registry → drivers → ss-device → ss-core`.

   Drivers are compiled in, not loaded at run time; see the alternatives below.

Around these axes:

- **Device manager** in the app: it discovers continuously, opens **every** connected supported
  device (one actor thread per device), tracks hot-plug, and routes commands by device key. The
  UI gets a "Devices" list in the sidebar with one page per connected device.
- **Audio stays separate.** AudioMix and the mic chain bind to the OS audio endpoints that a
  device declares (feature "audio endpoint"), not to a device type. Keyboards and mice simply have
  no such feature.
- **Gate.** The app stays completely passive while **no supported and enabled device** is
  connected (the owner requirement of `app.md` §8). Device-bound actions (Game/Chat balance,
  AudioMix routing to the headset) need *their* device to be present. Foreign devices are never
  opened or written to.
- **Settings per device instance.** The key is the driver id plus the serial or address, so two
  identical devices keep separate settings. Presets carry the device id and the feature
  parameters they fit (e.g. EQ band count).
- **Support levels.**
  - `Verified`: tested on hardware.
  - `Experimental`: based on public protocol facts, which stay **Inferred** until tested. Off
    until the user enables "Experimental devices", and marked in the UI.
  - Possibly irreversible commands (factory reset, firmware writes) are never sent by
    experimental drivers.
- **No behaviour change for the Nova Pro.** The refactor is complete only when the existing
  Windows and macOS hardware rows still pass.

## Alternatives considered

- **A headset-only interface** (the first version of this ADR). Too narrow for keyboards, mice and
  Bluetooth; it would have to be redone.
- **Run-time plugins (shared libraries or WebAssembly).**
  - Shared libraries: Rust has no stable ABI.
  - Code signing and notarization: every plugin would have to be signed and notarized on macOS.
  - Security: drivers write to hardware, so third-party code would run with full device access.
  - Compiled-in drivers keep all of that simple; WebAssembly drivers stay a later option behind
    the same trait.
- **Data-driven device files only (YAML, as Linux-Arctis-Manager does).** Framing, checksums,
  multi-report replies and hand-over logic differ too much between vendors. A generic driver for
  simple devices may still come later behind the same trait.
- **Device-class traits (`Headset`, `Keyboard`, `Mouse`).** Real devices mix classes (headsets
  with lighting, mice with batteries, keyboards with volume dials). Features compose; class traits
  do not.

## Consequences

- The work is a refactor in phases (see `devices.md` §8): about 2–3 sessions for the platform with
  the Nova Pro on it, then one session per new transport or feature kind when a real device needs
  it.
- After the refactor, a new device that uses existing features and transports means one crate,
  one registry entry and one hardware test log.
- Only the Nova Pro can be tested by the owner. Other devices need testers with the hardware;
  until then they ship as Experimental (off by default) or not at all.
- **Bluetooth:**
  - BLE GATT on macOS needs `NSBluetoothAlwaysUsageDescription` and a permission prompt;
  - on Windows, Bluetooth HID devices may expose other report layouts than over USB, so drivers
    match per transport.
- The config format changes: there are device instances, and presets carry a device id and
  feature parameters. Migration: existing headset EQ curves belong to `arctis-nova-pro-wireless`.
- ADR 0001 stays valid for *which* device is supported and verified today; this ADR removes the
  architectural limits.

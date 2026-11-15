# Devices: one interface, many headsets

Decision: [ADR 0008](../adr/0008-multi-device-support.md) (proposed). This document describes the
interface, how the app uses it, and the refactor plan. Status: **design, not implemented**.

## 1. Crates and dependency direction

```text
ss-app ──► ss-devices (registry) ──► ss-nova-pro ──► ss-device ──► ss-core
                                 └─► ss-<next device> ─┘
ss-app ──► ss-device (types only)          ss-hid (HidTransport impl) ──► ss-core
```

| Crate | Responsibility |
|---|---|
| `ss-core` | unchanged: value types (battery, noise control, sidetone, EQ, balance), `HidTransport` |
| `ss-device` (new) | `DeviceInfo`, `Capabilities`, `Setting`, `DeviceState`, `Change`, trait `HeadsetDriver`; platform-free, unit-tested |
| `ss-nova-pro` | protocol as today + `device.rs`: `NovaProDevice` implements `HeadsetDriver` |
| `ss-devices` (new) | registry: `SUPPORTED: &[&DeviceInfo]`, `open(info, transport) -> Box<dyn HeadsetDriver>` |
| `ss-app` | device thread, UI, gate, audio plan, all through `ss-device` types |

## 2. The interface (`ss-device`)

```rust
/// Static description of a device family (one per driver).
pub struct DeviceInfo {
    pub id: &'static str,                 // "arctis-nova-pro-wireless" (config key)
    pub name: &'static str,               // "Arctis Nova Pro Wireless"
    pub vendor_id: u16,
    pub product_ids: &'static [u16],
    pub interface: i32,                   // HID interface / usage pages as in ss-hid::DeviceSelector
    pub command_usage_page: u16,
    pub event_usage_page: u16,
    /// Substrings of the OS audio endpoint names (output and input) of this device.
    pub audio_endpoints: &'static [&'static str],
    pub support: Support,                 // Verified | Experimental
    pub has_base_station: bool,           // wording in UI/notifications ("base station" vs "dongle")
}

/// What the device can do; the UI shows exactly this.
pub struct Capabilities {
    pub noise_control: Option<NoiseControlCaps>, // modes + optional transparency range
    pub sidetone: Option<LevelRange>,
    pub mic_volume: Option<LevelRange>,
    pub hardware_eq: Option<EqCaps>,             // band count, centre freqs (if known), gain range/step, presets
    pub auto_off: Option<&'static [AutoOff]>,
    pub display_brightness: Option<LevelRange>,
    pub mic_led_brightness: Option<LevelRange>,
    pub batteries: BatteryCaps,                  // headset, spare slot, charge state
    pub balance: Option<BalanceCaps>,            // hardware Game/Chat dial, host-mixer hand-over
    pub firmware_info: bool,
    pub usb_reset: bool,                         // software re-plug supported (macOS)
}

/// A device-neutral setting the app can apply.
pub enum Setting {
    NoiseControl(NoiseControl), Transparency(u8), Sidetone(u8), MicVolume(u8),
    EqPreset(EqPreset), CustomEq(Vec<f32>), AutoOff(AutoOff),
    DisplayBrightness(u8), MicLedBrightness(u8), Balance(ChatMixBalance),
}

/// Everything the UI renders; `None` = unknown or not supported.
pub struct DeviceState {
    pub linked: Option<bool>,                    // headset ↔ base station / dongle
    pub batteries: Batteries,                    // headset, spare, charge state
    pub mic_muted: Option<bool>,
    pub values: SettingValues,                   // current value per Setting kind
    pub balance: Option<ChatMixBalance>,
    pub firmware: Vec<(String, String)>,         // label → version, e.g. ("Headset", "0.3.82")
}

/// Something that changed on the device (button, dial, link, battery …).
pub enum Change {
    State(DeviceState),          // full refresh
    Value(Setting),              // a single setting changed on the device
    Linked(bool),
    Batteries(Batteries),
    MicMuted(bool),
    BalanceDial(ChatMixBalance), // hardware dial moved (forwarded to AudioMix)
}

pub trait HeadsetDriver: Send {
    fn info(&self) -> &'static DeviceInfo;
    fn capabilities(&self) -> &Capabilities;
    /// Reads the full state (called after connecting and periodically).
    fn refresh(&mut self) -> Result<DeviceState, DeviceError>;
    /// Applies one setting; `Err(Unsupported)` if not in `capabilities()`.
    fn apply(&mut self, setting: &Setting) -> Result<(), DeviceError>;
    /// Waits up to `timeout` for the next device-initiated change.
    fn next_change(&mut self, timeout: Duration) -> Result<Option<Change>, DeviceError>;
    /// AudioMix took over / released the Game/Chat balance (Nova Pro: `06 49`, `06 45`, mixer LED).
    fn set_host_mixer(&mut self, active: bool) -> Result<(), DeviceError> { Ok(()) }
    /// Puts the device back into its normal state before the app quits.
    fn shutdown(&mut self) -> Result<(), DeviceError> { Ok(()) }
}
```

Rules:

- Value types stay in `ss-core`; ranges in `Capabilities` replace the hard-coded Nova Pro ranges
  in the UI. Device-specific labels (e.g. "Transparency strength 1–10") come from the
  capabilities, never from `if device == …` in the app.
- Drivers own their protocol quirks (multi-report replies, save-to-flash, echo reports). The app
  never sends raw opcodes.
- `Setting::CustomEq` is checked against `EqCaps` (band count, range) before it reaches a driver.

## 3. App changes

| Area | Today | With the interface |
|---|---|---|
| Device thread (`headset.rs`) | `NovaProWireless` + Nova Pro `Command`s | scans `ss_devices::SUPPORTED` (enabled ones), opens the first match, works with `Box<dyn HeadsetDriver>` |
| State (`HeadsetState`) | Nova Pro `Status`/`Settings`/`Display` | `DeviceState` + `&'static DeviceInfo` + `Capabilities` |
| Headset / Overview pages | fixed cards | cards from `Capabilities`; missing capability → card or row hidden |
| Gate (`app.md` §8) | "Nova Pro base station connected" | "a supported (and enabled) device connected"; texts use `DeviceInfo::name` |
| Audio plan | output named "Arctis Nova Pro Wireless" | output/input matching `DeviceInfo::audio_endpoints` of the connected device |
| Game/Chat balance hand-over | Nova Pro commands in the thread | `set_host_mixer(true/false)` |
| macOS USB reset | Nova Pro ids | ids from `DeviceInfo`, only if `capabilities.usb_reset` |
| Headset EQ presets (`ss-store`) | `headset_eq: name → [f32; 10]` | keyed by device id; existing entries migrate to `arctis-nova-pro-wireless` |
| `ssctl` | Nova Pro only | unchanged for now (protocol tool); may gain `--device` later |

Several supported devices at once: v1 of this design uses **one** active device (the first one
found, stable order from the registry). Choosing between several is a later UI addition.

## 4. Support levels

- **Verified:** tested on hardware by a maintainer, rows in `docs/hardware/<device>.md` pass.
  Enabled by default.
- **Experimental:** implemented from public protocol facts (Inferred) or tested only partly. Off
  until the user enables Settings → "Experimental devices"; the UI shows "Experimental" next to the
  device name. Never sends commands that are marked possibly irreversible (factory reset, firmware).

## 5. Adding a headset (after the refactor)

1. Collect protocol facts with sources in the private RE notes or in
   `docs/hardware/<device>.md` (Observed / Inferred / Unknown); clean-room rules apply.
2. New crate `ss-<device>`: codec + driver with unit tests against documented byte layouts, then
   `impl HeadsetDriver` with `DeviceInfo { support: Experimental, … }` and honest `Capabilities`.
3. Add it to `ss-devices`.
4. Hardware test log with a tester who owns the device; switch to `Verified` when it passes.

## 6. Refactor plan (each step keeps the app working; the owner re-tests at the end)

1. `ss-device` crate: types, trait, unit tests (no app change).
2. `NovaProDevice` adapter in `ss-nova-pro` + mapping tests (Nova Pro `Status`/`Settings`/`Event`
   → `DeviceState`/`Change`).
3. `ss-devices` registry; experimental flag in the config (`app.experimental_devices`, default off).
4. Device thread and `HeadsetState` on the trait; gate and notifications generic.
5. UI from `Capabilities` (Headset, Overview, tray texts).
6. Audio plan uses `audio_endpoints`; macOS USB reset via `DeviceInfo`.
7. `ss-store` migration of headset EQ presets to per-device keys.
8. Docs: ADR 0008 accepted, `app.md` §1/§6/§8 updated; owner re-runs the Windows and macOS rows
   that touch the headset (status, noise control, EQ, balance, gate).

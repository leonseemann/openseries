# Devices: an extensible platform for any device and transport

Decision: [ADR 0008](../adr/0008-multi-device-support.md) (accepted). Status: **Phase A
implemented 2026-11-15** (Nova Pro on the platform; verified on Windows by the owner, macOS re-test pending). Differences from the
plan below are listed in §10. Goal: adding a headset, keyboard, mouse or a new transport such as Bluetooth must
not touch the app core.

## 1. Three extension axes

| Axis | Question | Extension = | Touches |
|---|---|---|---|
| **Feature** | what can a device do? | a catalogue entry + one UI widget + docs | `ss-device`, `ss-app/ui/features/` |
| **Transport** | how do we reach it? | `Discovery` + `Connection` impl | a `ss-transport-*` crate + registration |
| **Driver** | which device speaks which protocol? | a crate implementing `Device` + a `DriverEntry` | `ss-<device>` + `ss-devices` |

The app core (device manager, state, gate, audio, config) only knows the traits and the feature
catalogue.

## 2. Crates

```text
ss-app ──► ss-devices (registry) ──► ss-nova-pro, ss-<next> … ──► ss-device ──► ss-core
   │                                                                  ▲
   └──► ss-transport-hid (USB + Bluetooth HID, hidapi) ───────────────┤
   └──► ss-transport-ble (BLE GATT, later) ───────────────────────────┘
```

| Crate | Responsibility |
|---|---|
| `ss-core` | value types (battery, noise control, EQ gain, balance …), unchanged |
| `ss-device` (new) | feature catalogue, `Device` trait, `DeviceInfo`, `DriverEntry`, match rules, transport traits (`Connection`, `Discovery`), `Candidate`; platform-free, unit-tested |
| `ss-transport-hid` (today `ss-hid`) | discovery + connection over hidapi for USB **and** Bluetooth HID (hidapi reports the bus type) |
| `ss-transport-ble` (later) | discovery + connection for vendor GATT services (`btleplug` or `bluest`) |
| `ss-devices` (new) | `DRIVERS: &[DriverEntry]`, `TRANSPORTS` list; the only crate that knows every driver |
| `ss-nova-pro` | protocol as today + `device.rs` adapter |
| `ss-app` | device manager, UI, gate, audio binding, config |

`ss-core::transport::HidTransport` becomes `ss-device::Connection` (same shape: write a report,
read with a timeout), so existing driver code keeps working.

## 3. Feature catalogue (`ss-device::feature`)

Each feature kind has a **descriptor** (static parameters the device reports or declares) and a
**value** type. A device lists `Feature { id: FeatureId, descriptor }`. `FeatureId` = kind +
index, so a keyboard can have several lighting zones and a mouse several DPI stages.

| Kind | Descriptor | Value | Now / later |
|---|---|---|---|
| `Battery` | slots (headset, spare, …) | level per slot + charge state | now |
| `WirelessLink` | — | linked: bool | now |
| `AudioEndpoint` | OS endpoint name patterns, in/out | — (used by AudioMix / mic chain) | now |
| `NoiseControl` | modes; optional transparency range | mode, transparency | now |
| `Sidetone`, `MicVolume`, `MicLedBrightness`, `DisplayBrightness` | `LevelRange` | level | now |
| `MicMute` | read-only or settable | muted | now |
| `Equalizer` | band count, centre freqs (if known), gain range/step | curve | now |
| `EqPreset` | preset list | preset | now |
| `BalanceDial` | hardware dial; supports host-mixer hand-over | Game/Chat balance | now |
| `AutoOff` | options | option | now |
| `Firmware` | — | list of (part, version), read-only | now |
| `UsbReplug` | — | action | now (macOS) |
| `Lighting` | zones, colour model, effects | colour/effect per zone | later (keyboards, mice) |
| `Dpi` | stages, range, step | stage values + active stage | later |
| `PollingRate` | rates | rate | later |
| `ButtonMap` / `KeyMap` | buttons/keys, assignable actions | mapping | later |
| `Macros`, `OnboardProfiles` | limits | data | later |

Rules:

- A device class (`Headset`, `Keyboard`, `Mouse`, `Other`) is only a label and an icon in
  `DeviceInfo`; code never branches on it.
- The UI has one widget per kind (`ss-app/src/ui/features/<kind>.rs`) and lays a device page out
  from the feature list (grouping order defined per kind). Unknown or unsupported features are
  simply not shown.
- Values are validated against the descriptor before they reach a driver.

## 4. The `Device` trait and driver registration

```rust
pub trait Device: Send {
    fn info(&self) -> &DeviceInfo;                 // static info + instance (serial/address, transport)
    fn features(&self) -> &[Feature];              // may be refined after connect (e.g. firmware-dependent)
    /// Reads all readable feature values.
    fn refresh(&mut self) -> Result<Vec<(FeatureId, Value)>, DeviceError>;
    /// Writes one value; `Err(Unsupported)` if the feature is read-only or absent.
    fn set(&mut self, id: FeatureId, value: Value) -> Result<(), DeviceError>;
    /// Next device-initiated change (button, dial, link, battery), waiting up to `timeout`.
    fn next_change(&mut self, timeout: Duration) -> Result<Option<Change>, DeviceError>;
    /// The app takes over / releases a role (e.g. `Role::HostMixer` for the balance dial).
    fn set_role(&mut self, role: Role, active: bool) -> Result<(), DeviceError> { Ok(()) }
    /// Back to the device's normal state before the app lets go.
    fn shutdown(&mut self) -> Result<(), DeviceError> { Ok(()) }
}

pub struct DriverEntry {
    pub info: &'static DeviceInfo,                 // id, name, class, support level
    pub matches: &'static [Match],
    pub open: fn(Box<dyn Connection>, &Candidate) -> Result<Box<dyn Device>, DeviceError>,
}

pub enum Match {
    Hid { bus: Bus, vendor_id: u16, product_ids: &'static [u16], interface: Option<i32>, usage_page: Option<u16> },
    BleService { service_uuid: Uuid, name_prefix: Option<&'static str> },
}
pub enum Bus { Usb, Bluetooth, Any }
```

`Change` = `Value(FeatureId, Value)` | `Refresh` (re-read everything) | `Gone` (device left).

## 5. Transports

```rust
pub trait Discovery: Send {
    fn kind(&self) -> TransportKind;               // UsbHid | BluetoothHid | Ble
    fn scan(&mut self) -> Vec<Candidate>;          // cheap, called every ~2 s (later: OS hot-plug events)
    fn open(&mut self, candidate: &Candidate) -> Result<Box<dyn Connection>, TransportError>;
}
pub trait Connection: Send {
    fn write(&mut self, frame: &[u8]) -> Result<(), TransportError>;
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError>;
    // BLE: frames go to / come from the matched characteristic(s); reports = notifications.
}
```

- **USB and Bluetooth HID** share `ss-transport-hid`. hidapi lists both with a bus type, so a
  driver can match only USB, only Bluetooth or both, with different report layouts if needed.
- **BLE GATT** (vendor services): `ss-transport-ble` on `btleplug` or `bluest`, chosen in a spike
  when the first device needs it. macOS needs `NSBluetoothAlwaysUsageDescription` in `Info.plist`.
- Exclusive access: some OSes open HID devices exclusively (hidapi on macOS opens the Nova Pro
  base station exclusively). The device manager opens only matched candidates, never foreign
  devices.

## 6. Device manager (app)

- A discovery thread asks every transport for candidates and matches them against the enabled
  `DriverEntry`s (Experimental only with Settings → "Experimental devices"). Each match gets one
  **device actor thread**, which owns the `Box<dyn Device>`, polls `next_change`, refreshes
  periodically and executes commands. A device that is unplugged sends `Gone` and the actor ends.
- `DeviceKey` = driver id + instance id (serial, Bluetooth address or USB path).
- `Snapshot.devices: Vec<DeviceSnapshot { key, info, features, values, error }>`.
- UI: the sidebar lists connected devices under "Devices", and each device gets a page built from
  its features. Overview shows the most important features of each device (battery, link, balance).
- Notifications become feature-based (battery low, link lost/back) and name the device.

### Gate and audio

- **Gate:** without a connected supported and enabled device the app is passive: no audio
  routing, no hotkeys, no device writes (`app.md` §8). Device-bound actions also need their own
  device: the balance hand-over needs a `BalanceDial` device, and AudioMix routing needs the
  `AudioEndpoint` of the selected output device.
- **Audio:** AudioMix and the mic chain pick their output/input from the `AudioEndpoint`
  features of connected devices (default: the first one; later selectable). Today's name lookup
  "Arctis Nova Pro Wireless" becomes the Nova Pro's endpoint descriptor.

## 7. Configuration and presets

- `devices: { "<driver id>/<instance id>": { values to restore, … } }`, so per-instance settings
  survive restarts and two identical devices stay separate.
- Presets: `kind` plus the **parameters they fit** (e.g. `Equalizer { bands: 10 }`); the UI offers
  only matching presets.
- Migration: today's `presets.headset_eq` (10 bands) becomes `Equalizer` presets with
  `bands: 10` and the device id `arctis-nova-pro-wireless`.
- `app.experimental_devices: bool` (default `false`).

## 8. Refactor plan (phases; the app keeps working after every step)

**Phase A — platform with the Nova Pro (no new devices, no behaviour change)**

1. `ss-device`: feature catalogue (the "now" kinds), `Device`, `DriverEntry`, `Match`, transport
   traits; unit tests.
2. `ss-hid` → `ss-transport-hid`: `Discovery` + `Connection`, bus type in candidates.
3. `ss-nova-pro::device`: adapter + mapping tests (status/settings/events → feature values).
4. `ss-devices` registry.
5. App: device manager + per-device actors replace the headset thread; snapshot with device list;
   gate and notifications generic.
6. UI: feature widgets, device pages, "Devices" sidebar section; Overview from features.
7. Audio from `AudioEndpoint`; macOS USB re-plug via the `UsbReplug` feature.
8. Config: device instances, preset parameters, migration; `experimental_devices` switch.
9. Docs (`app.md`, this file, ADR 0008 accepted) and the owner's re-test on Windows and macOS.

**Phase B — Bluetooth HID:** match `Bus::Bluetooth` in the HID transport. Verified with the first
Bluetooth device that a tester owns.

**Phase C — BLE GATT:** spike `btleplug` vs `bluest`, then `ss-transport-ble` and the macOS
Bluetooth permission. Only when a device needs it.

**Phase D — keyboard/mouse features:** `Lighting`, `Dpi`, `PollingRate`, `ButtonMap` … with their
widgets, added together with the first device that uses them.

## 9. Adding a device (after Phase A)

1. Collect protocol facts with sources (Observed / Inferred / Unknown) in the private RE notes or
   `docs/hardware/<device>.md`; clean-room rules apply (AGENTS.md).
2. New crate `ss-<device>`: codec + driver with unit tests against documented byte layouts;
   `impl Device`; `DriverEntry { info: { support: Experimental, … }, matches, open }`.
3. Add it to `ss-devices`. If it needs a feature kind or transport that does not exist yet, add it
   first (axis 1 or 2).
4. Hardware test log with a tester; switch to `Verified` when it passes.

## 10. Implementation notes (Phase A, 2026-11-15)

- The HID transport stayed in the crate `ss-hid` (`HidDiscovery`); renaming it to
  `ss-transport-hid` brought nothing.
- `Connection` is `ss-core`'s `HidTransport` re-exported; `Box<dyn HidTransport>` implements the
  trait, so drivers stay generic.
- Device settings live on the devices themselves; the config does **not** restore values per
  instance. The config has `devices.experimental` and per-device EQ presets
  (`presets.device_eq: device id → name → curve`; older `presets.headset_eq` curves move to
  `arctis-nova-pro-wireless` on load, the export kind stays `headset_eq`).
- The UI: `ui/device.rs` (presentation table + shape-based controls, "More" card for unknown
  kinds), the sidebar lists connected devices, Overview shows one card per device plus the quick
  controls of the first device that has them.
- Alerts are tracked per device instance and name the device (`alerts.rs`).
- The audio "headset off" fallback uses the connected device's `AudioEndpoint` names instead of
  the fixed "Arctis Nova Pro Wireless".
- Live check without sending commands: `cargo test -p ss-hid -- --ignored --nocapture scan` lists
  the HID candidates with bus type (2026-11-15: the Nova Pro appears as USB interface 4 with the
  `FFC0`/`FF00` collections; other vendors' keyboards and receivers are listed and never opened).

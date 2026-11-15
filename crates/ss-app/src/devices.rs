//! Device manager (ADR 0008, `docs/architecture/devices.md` §6): finds supported devices on every
//! transport, gives each one its own actor thread and keeps a device-neutral snapshot for the UI.
//!
//! Only candidates that an enabled driver recognises are ever opened, so foreign devices (another
//! person's headset, keyboards of other brands) are never touched (`app.md` §8).

use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ss_core::battery::{BatteryLevel, ChargeState};
use ss_core::headset::ChatMixBalance;
use ss_device::{
    Change, Descriptor, Device, DeviceInfo, Feature, FeatureId, FeatureKind, Instance, Role, Value,
};

use crate::notify::Notifier;

/// How often the transports are scanned for new devices.
const SCAN_INTERVAL: Duration = Duration::from_secs(2);
/// How long the manager waits for a message before checking actors and scanning.
const MANAGER_POLL: Duration = Duration::from_millis(250);
/// How long one device read blocks before messages are checked again.
const ACTOR_POLL: Duration = Duration::from_millis(100);
/// How often a device's full state is re-read.
const REFRESH_INTERVAL: Duration = Duration::from_secs(30);
/// Grace period after a software USB re-plug before the device is opened again.
const REPLUG_COOLDOWN: Duration = Duration::from_secs(4);

/// One connected device as the UI sees it.
#[derive(Debug, Clone)]
pub struct DeviceSnapshot {
    /// Static description of its family.
    pub info: &'static DeviceInfo,
    /// This connected instance.
    pub instance: Instance,
    /// What it can do.
    pub features: Vec<Feature>,
    /// Last known value per feature.
    pub values: BTreeMap<FeatureId, Value>,
    /// The first full read finished.
    pub ready: bool,
}

impl DeviceSnapshot {
    /// Unique key of the instance.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.instance.key
    }

    /// The first feature of `kind`.
    #[must_use]
    pub fn feature(&self, kind: FeatureKind) -> Option<&Feature> {
        self.features.iter().find(|f| f.id.kind == kind)
    }

    /// Whether the device has a feature of `kind`.
    #[must_use]
    pub fn has(&self, kind: FeatureKind) -> bool {
        self.feature(kind).is_some()
    }

    /// The value of the first feature of `kind`.
    #[must_use]
    pub fn value(&self, kind: FeatureKind) -> Option<&Value> {
        self.values.get(&FeatureId::of(kind))
    }

    /// A choice value.
    #[must_use]
    pub fn choice(&self, kind: FeatureKind) -> Option<&'static str> {
        match self.value(kind)? {
            Value::Choice(key) => Some(key),
            _ => None,
        }
    }

    /// A level value.
    #[must_use]
    pub fn level(&self, kind: FeatureKind) -> Option<u8> {
        match self.value(kind)? {
            Value::Level(level) => Some(*level),
            _ => None,
        }
    }

    /// A toggle value.
    #[must_use]
    pub fn flag(&self, kind: FeatureKind) -> Option<bool> {
        match self.value(kind)? {
            Value::Bool(on) => Some(*on),
            _ => None,
        }
    }

    /// Whether the wireless part is linked; `None` for wired devices or before the first read.
    #[must_use]
    pub fn linked(&self) -> Option<bool> {
        self.flag(FeatureKind::WirelessLink)
    }

    /// Battery levels per slot and charge state.
    #[must_use]
    pub fn batteries(&self) -> Option<(&[Option<BatteryLevel>], ChargeState)> {
        match self.value(FeatureKind::Battery)? {
            Value::Battery { levels, charge } => Some((levels, *charge)),
            _ => None,
        }
    }

    /// The main battery, only while the device is linked (a stale level would mislead).
    #[must_use]
    pub fn main_battery(&self) -> Option<BatteryLevel> {
        if self.linked() == Some(false) {
            return None;
        }
        self.batteries()?.0.first().copied().flatten()
    }

    /// The Game/Chat balance of the dial.
    #[must_use]
    pub fn balance(&self) -> Option<ChatMixBalance> {
        match self.value(FeatureKind::BalanceDial)? {
            Value::Balance(balance) => Some(*balance),
            _ => None,
        }
    }

    /// What the computer is plugged into, for messages ("base station", "dongle", "device").
    #[must_use]
    pub fn hub(&self) -> &'static str {
        self.info.hub_name.unwrap_or("device")
    }
}

/// All connected devices.
#[derive(Debug, Clone, Default)]
pub struct DevicesState {
    /// Connected devices, ordered by key.
    pub devices: Vec<DeviceSnapshot>,
    /// Last problem (opening or losing a device); cleared when a device connects.
    pub error: Option<String>,
}

impl DevicesState {
    /// At least one supported device is connected (the gate, `app.md` §8).
    #[must_use]
    pub fn any(&self) -> bool {
        !self.devices.is_empty()
    }

    /// The first connected device.
    #[must_use]
    pub fn primary(&self) -> Option<&DeviceSnapshot> {
        self.devices.first()
    }

    /// The first device with a feature of `kind`.
    #[must_use]
    pub fn with(&self, kind: FeatureKind) -> Option<&DeviceSnapshot> {
        self.devices.iter().find(|d| d.has(kind))
    }

    /// The device with `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&DeviceSnapshot> {
        self.devices.iter().find(|d| d.key() == key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut DeviceSnapshot> {
        self.devices.iter_mut().find(|d| d.key() == key)
    }
}

/// Messages to the device manager.
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceMsg {
    /// Write a value. `key: None` = the first device that has the feature.
    Set {
        /// Target device.
        key: Option<String>,
        /// Feature to write.
        id: FeatureId,
        /// New value.
        value: Value,
    },
    /// AudioMix on/off: the app takes over the Game/Chat dial ([`Role::HostMixer`]).
    HostMixer(bool),
    /// Allow experimental drivers (Settings).
    Experimental(bool),
    /// Software re-plug of a USB device (`key: None` = the first device that supports it).
    ReplugUsb {
        /// Target device.
        key: Option<String>,
    },
    /// Return all devices to their normal state and stop.
    Shutdown,
}

/// Messages to one device actor.
enum ActorMsg {
    Set(FeatureId, Value),
    HostMixer(bool),
    ReplugUsb,
    Shutdown,
}

struct Actor {
    tx: Sender<ActorMsg>,
    handle: JoinHandle<()>,
}

/// What the manager shares with every actor.
#[derive(Clone)]
struct Shared {
    state: Arc<Mutex<DevicesState>>,
    balance_out: Sender<ChatMixBalance>,
    notify: Notifier,
}

impl Shared {
    fn update(&self, change: impl FnOnce(&mut DevicesState)) {
        change(&mut lock(&self.state));
        (self.notify)();
    }
}

/// Starts the device manager.
///
/// # Errors
/// If the thread cannot be spawned.
pub fn spawn(
    rx: Receiver<DeviceMsg>,
    state: Arc<Mutex<DevicesState>>,
    balance_out: Sender<ChatMixBalance>,
    notify: Notifier,
    host_mixer: bool,
    experimental: bool,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("devices".to_owned())
        .spawn(move || {
            Manager {
                rx,
                shared: Shared {
                    state,
                    balance_out,
                    notify,
                },
                discoveries: ss_devices::discoveries(),
                actors: HashMap::new(),
                cooldown: HashMap::new(),
                host_mixer,
                experimental,
                last_scan: None,
            }
            .run();
        })
}

struct Manager {
    rx: Receiver<DeviceMsg>,
    shared: Shared,
    discoveries: Vec<Box<dyn ss_device::Discovery>>,
    actors: HashMap<String, Actor>,
    cooldown: HashMap<String, Instant>,
    host_mixer: bool,
    experimental: bool,
    last_scan: Option<Instant>,
}

impl Manager {
    fn run(mut self) {
        loop {
            match self.rx.recv_timeout(MANAGER_POLL) {
                Ok(DeviceMsg::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                    self.shutdown();
                    return;
                }
                Ok(message) => self.handle(message),
                Err(RecvTimeoutError::Timeout) => {}
            }
            self.reap();
            if self.last_scan.is_none_or(|t| t.elapsed() >= SCAN_INTERVAL) {
                self.last_scan = Some(Instant::now());
                self.scan();
            }
        }
    }

    fn handle(&mut self, message: DeviceMsg) {
        match message {
            DeviceMsg::Set { key, id, value } => {
                if let Some(key) = key.or_else(|| self.first_with(id.kind)) {
                    self.send(&key, ActorMsg::Set(id, value));
                }
            }
            DeviceMsg::HostMixer(active) => {
                self.host_mixer = active;
                for actor in self.actors.values() {
                    let _ = actor.tx.send(ActorMsg::HostMixer(active));
                }
            }
            DeviceMsg::Experimental(enabled) => {
                self.experimental = enabled;
                if !enabled {
                    // Close devices whose driver is no longer enabled.
                    let enabled_ids: Vec<&str> =
                        ss_devices::enabled(false).map(|d| d.info.id).collect();
                    for (key, actor) in &self.actors {
                        let driver = key.split('/').next().unwrap_or_default();
                        if !enabled_ids.contains(&driver) {
                            let _ = actor.tx.send(ActorMsg::Shutdown);
                        }
                    }
                }
            }
            DeviceMsg::ReplugUsb { key } => {
                if let Some(key) = key.or_else(|| self.first_with(FeatureKind::UsbReplug)) {
                    self.cooldown.insert(key.clone(), Instant::now());
                    self.send(&key, ActorMsg::ReplugUsb);
                }
            }
            DeviceMsg::Shutdown => {}
        }
    }

    fn first_with(&self, kind: FeatureKind) -> Option<String> {
        lock(&self.shared.state)
            .with(kind)
            .map(|d| d.key().to_owned())
    }

    fn send(&self, key: &str, message: ActorMsg) {
        if let Some(actor) = self.actors.get(key) {
            let _ = actor.tx.send(message);
        }
    }

    /// Forgets actors whose device is gone, so the next scan can open it again.
    fn reap(&mut self) {
        self.actors.retain(|_, actor| !actor.handle.is_finished());
        self.cooldown
            .retain(|_, since| since.elapsed() < REPLUG_COOLDOWN);
    }

    fn scan(&mut self) {
        for discovery in &mut self.discoveries {
            for candidate in discovery.scan() {
                let Some((driver, rule)) = ss_devices::driver_for(&candidate, self.experimental)
                else {
                    continue; // not ours: never opened
                };
                let key = Instance::of(driver.info.id, &candidate).key;
                if self.actors.contains_key(&key) || self.cooldown.contains_key(&key) {
                    continue;
                }
                let opened = discovery
                    .open(&candidate, rule)
                    .map_err(|e| e.to_string())
                    .and_then(|connection| {
                        (driver.open)(connection, &candidate).map_err(|e| e.to_string())
                    });
                match opened {
                    Ok(device) => {
                        let (tx, rx) = mpsc::channel();
                        let shared = self.shared.clone();
                        let host_mixer = self.host_mixer;
                        let spawned = std::thread::Builder::new()
                            .name(format!("device {}", driver.info.id))
                            .spawn(move || run_actor(device, &rx, &shared, host_mixer));
                        if let Ok(handle) = spawned {
                            self.actors.insert(key, Actor { tx, handle });
                        }
                    }
                    Err(error) => self.shared.update(|s| {
                        s.error = Some(format!("{}: {error}", driver.info.name));
                    }),
                }
            }
        }
    }

    fn shutdown(&mut self) {
        for actor in self.actors.values() {
            let _ = actor.tx.send(ActorMsg::Shutdown);
        }
        for (_, actor) in self.actors.drain() {
            let _ = actor.handle.join();
        }
    }
}

/// The life of one connected device: read, react, write, until it is gone or the app stops.
fn run_actor(
    mut device: Box<dyn Device>,
    rx: &Receiver<ActorMsg>,
    shared: &Shared,
    mut host_mixer_wanted: bool,
) {
    let key = device.instance().key.clone();
    let name = device.info().name;
    let snapshot = DeviceSnapshot {
        info: device.info(),
        instance: device.instance().clone(),
        features: device.features().to_vec(),
        values: BTreeMap::new(),
        ready: false,
    };
    shared.update(|s| {
        s.devices.push(snapshot);
        s.devices.sort_by(|a, b| a.key().cmp(b.key()));
        s.error = None;
    });
    let has_host_mixer = device
        .features()
        .iter()
        .any(|f| matches!(f.descriptor, Descriptor::Balance { host_mixer: true }));
    let mut host_mixer_active = false;
    let mut last_refresh = Instant::now();

    let outcome: Result<(), String> = (|| {
        refresh(&mut *device, &key, shared, true)?;
        loop {
            loop {
                match rx.try_recv() {
                    Ok(ActorMsg::Set(id, value)) => {
                        match device.set(id, &value) {
                            Ok(()) => {}
                            // Only a broken connection ends the actor; a refused value is
                            // reported and the device stays connected.
                            Err(error @ ss_device::DeviceError::Transport(_)) => {
                                return Err(error.to_string());
                            }
                            Err(error) => shared.update(|s| {
                                s.error = Some(format!("{name}: {error}"));
                            }),
                        }
                        // Devices report button presses but not host writes: read back.
                        refresh(&mut *device, &key, shared, false)?;
                        last_refresh = Instant::now();
                    }
                    Ok(ActorMsg::HostMixer(active)) => host_mixer_wanted = active,
                    Ok(ActorMsg::ReplugUsb) => {
                        let _ = device.shutdown();
                        let instance = device.instance().clone();
                        drop(device); // close our handles before the device disappears
                        return crate::platform::reset_usb(instance.vendor_id, instance.product_id)
                            .map_err(|e| format!("USB reconnect failed: {e}"));
                    }
                    Ok(ActorMsg::Shutdown) | Err(TryRecvError::Disconnected) => {
                        let _ = device.shutdown();
                        return Ok(());
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
            if has_host_mixer && host_mixer_wanted != host_mixer_active {
                device
                    .set_role(Role::HostMixer, host_mixer_wanted)
                    .map_err(|e| e.to_string())?;
                host_mixer_active = host_mixer_wanted;
            }
            match device.next_change(ACTOR_POLL).map_err(|e| e.to_string())? {
                Some(Change::Value(id, value)) => {
                    if let Value::Balance(balance) = value {
                        let _ = shared.balance_out.send(balance);
                    }
                    shared.update(|s| {
                        if let Some(d) = s.get_mut(&key) {
                            d.values.insert(id, value);
                        }
                    });
                }
                Some(Change::Refresh) => {
                    refresh(&mut *device, &key, shared, false)?;
                    last_refresh = Instant::now();
                }
                None => {}
            }
            if last_refresh.elapsed() >= REFRESH_INTERVAL {
                refresh(&mut *device, &key, shared, false)?;
                last_refresh = Instant::now();
            }
        }
    })();

    shared.update(|s| {
        s.devices.retain(|d| d.key() != key);
        if let Err(error) = outcome {
            s.error = Some(format!("{name}: {error}"));
        }
    });
}

/// Reads all values; `first` also forwards the dial position so AudioMix starts from it.
fn refresh(device: &mut dyn Device, key: &str, shared: &Shared, first: bool) -> Result<(), String> {
    let values = device
        .refresh()
        .map_err(|e| format!("connection lost: {e}"))?;
    if first {
        for (_, value) in &values {
            if let Value::Balance(balance) = value {
                let _ = shared.balance_out.send(*balance);
            }
        }
    }
    shared.update(|s| {
        if let Some(d) = s.get_mut(key) {
            d.values.extend(values);
            d.ready = true;
        }
    });
    Ok(())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss_device::{Bus, DeviceClass, LevelRange, Support};

    static INFO: DeviceInfo = DeviceInfo {
        id: "test",
        name: "Test headset",
        class: DeviceClass::Headset,
        support: Support::Verified,
        hub_name: None,
    };

    fn device(values: Vec<(FeatureKind, Value)>) -> DeviceSnapshot {
        DeviceSnapshot {
            info: &INFO,
            instance: Instance {
                key: "test/1".to_owned(),
                bus: Bus::Usb,
                vendor_id: 1,
                product_id: 2,
                release: None,
                serial: None,
            },
            features: vec![Feature {
                id: FeatureId::of(FeatureKind::Sidetone),
                descriptor: Descriptor::Level(LevelRange::new(0, 3)),
                writable: true,
            }],
            values: values
                .into_iter()
                .map(|(kind, value)| (FeatureId::of(kind), value))
                .collect(),
            ready: true,
        }
    }

    #[test]
    fn accessors_read_typed_values() {
        let d = device(vec![
            (FeatureKind::Sidetone, Value::Level(2)),
            (FeatureKind::NoiseControl, Value::Choice("anc")),
            (FeatureKind::WirelessLink, Value::Bool(true)),
        ]);
        assert_eq!(d.level(FeatureKind::Sidetone), Some(2));
        assert_eq!(d.choice(FeatureKind::NoiseControl), Some("anc"));
        assert_eq!(d.linked(), Some(true));
        assert_eq!(d.hub(), "device");
        assert!(d.has(FeatureKind::Sidetone));
        assert!(!d.has(FeatureKind::Equalizer));
    }

    #[test]
    fn battery_is_hidden_while_unlinked() {
        let level = BatteryLevel::from_percent(80).ok();
        let battery = Value::Battery {
            levels: vec![level],
            charge: ChargeState::Discharging,
        };
        let linked = device(vec![
            (FeatureKind::Battery, battery.clone()),
            (FeatureKind::WirelessLink, Value::Bool(true)),
        ]);
        assert_eq!(linked.main_battery(), level);
        let off = device(vec![
            (FeatureKind::Battery, battery),
            (FeatureKind::WirelessLink, Value::Bool(false)),
        ]);
        assert_eq!(off.main_battery(), None);
    }

    #[test]
    fn state_finds_devices_by_feature() {
        let state = DevicesState {
            devices: vec![device(Vec::new())],
            error: None,
        };
        assert!(state.any());
        assert!(state.with(FeatureKind::Sidetone).is_some());
        assert!(state.with(FeatureKind::Equalizer).is_none());
        assert!(state.get("test/1").is_some());
    }
}

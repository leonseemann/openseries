//! Keeps the running audio sessions in line with the configuration, the running apps and the
//! devices (`docs/architecture/app.md` §1). No threads and no CoreAudio here: the audio-control
//! thread drives it, tests drive it with a fake [`AudioBackend`].

use ss_core::headset::ChatMixBalance;
use ss_core::mixer::{ChannelId, Fader};
use ss_engine::Command as EngineCommand;
use ss_store::Config;
use ss_store::config::Channel;

use super::plan::{
    DeviceInfo, DeviceSet, MicPlan, MixDecision, MixPlan, ProcessInfo, mic_plan, mix_decision,
    resolve_device,
};

/// A running AudioMix session; dropping it stops the mix.
pub trait MixHandle {
    /// Sends a live parameter change to the engine. Returns `false` if the queue was full.
    fn send(&mut self, command: EngineCommand) -> bool;

    /// Periodic rescan while running (every tick). The Windows session mixer adopts new app
    /// sessions here; macOS taps need nothing.
    fn refresh(&mut self) {}
}

/// A running mic feed; dropping it stops the feed.
pub trait MicHandle {
    /// Applies a new chain while running. Returns `false` if this feed cannot (then it is
    /// restarted with the new chain).
    fn set_chain(&mut self, _chain: &ss_dsp::MicChainSettings) -> bool {
        false
    }
}

/// Everything the supervisor needs from the OS audio layer.
pub trait AudioBackend {
    /// Running audio processes.
    ///
    /// # Errors
    /// A message if the process list cannot be read.
    fn processes(&mut self) -> Result<Vec<ProcessInfo>, String>;
    /// Audio devices, without the app's own aggregate and virtual devices.
    fn devices(&mut self) -> DeviceSet;
    /// Starts AudioMix.
    ///
    /// # Errors
    /// A user-facing message (e.g. missing permission).
    fn start_mix(&mut self, plan: &MixPlan) -> Result<Box<dyn MixHandle>, String>;
    /// Starts the mic feed.
    ///
    /// # Errors
    /// A user-facing message (e.g. driver not installed).
    fn start_mic(&mut self, plan: &MicPlan) -> Result<Box<dyn MicHandle>, String>;
    /// UID of the virtual microphone, if this OS has one.
    fn virtual_mic_uid(&self) -> Option<&'static str> {
        None
    }
    /// Makes `uid` the system default input.
    ///
    /// # Errors
    /// A user-facing message.
    fn set_default_input(&mut self, _uid: &str) -> Result<(), String> {
        Err("not supported on this operating system".to_owned())
    }
}

/// State of one audio service as shown to the user.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ServiceStatus {
    /// Switched off.
    #[default]
    Off,
    /// Running.
    Running,
    /// Enabled but intentionally not started (reason shown).
    Blocked(String),
    /// Starting failed (message shown); retried after the next relevant change.
    Failed(String),
}

/// What the UI shows about the audio side.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AudioStatus {
    /// AudioMix.
    pub mix: ServiceStatus,
    /// Number of apps routed away from Media.
    pub routed_apps: usize,
    /// How often AudioMix was (re)started since the app launched (diagnostics: each start is a
    /// short audio gap, so this should only grow on real changes).
    pub mix_starts: u32,
    /// Mic feed into the virtual microphone.
    pub mic: ServiceStatus,
    /// Running audio processes (for the assignment list).
    pub processes: Vec<ProcessInfo>,
    /// Problem with making the virtual mic the default input (SON-06).
    pub default_input_error: Option<String>,
    /// Output devices.
    pub outputs: Vec<DeviceInfo>,
    /// Input devices.
    pub inputs: Vec<DeviceInfo>,
}

/// Whether the system default input was switched to the virtual mic.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DefaultInput {
    /// Left as the user set it.
    Untouched,
    /// Switched by us; `previous` (`None` = there was no default) is restored later.
    Replaced {
        /// The default input before the switch.
        previous: Option<String>,
    },
}

/// Shown for an enabled service while the base station is missing.
pub const PAUSED_REASON: &str = "no supported device is connected";

/// Status of a service while the base station is missing: off stays off, on is paused.
fn paused_status(enabled: bool) -> ServiceStatus {
    if enabled {
        ServiceStatus::Blocked(PAUSED_REASON.to_owned())
    } else {
        ServiceStatus::Off
    }
}

/// The reconciler.
pub struct AudioSupervisor<B: AudioBackend> {
    backend: B,
    config: Config,
    processes: Vec<ProcessInfo>,
    mix: Option<(MixPlan, Box<dyn MixHandle>)>,
    /// The output device the running mix was started on (to notice replugs and default changes).
    mix_device: Option<DeviceInfo>,
    /// Plan that failed to start; not retried until the plan changes.
    failed_mix: Option<MixPlan>,
    mic: Option<(MicPlan, Box<dyn MicHandle>)>,
    /// The input device the running mic feed was started on.
    mic_device: Option<DeviceInfo>,
    devices: DeviceSet,
    failed_mic: Option<MicPlan>,
    chatmix: Option<ChatMixBalance>,
    /// Headset link as last reported (`None` = unknown); drives the "headset off" fallback.
    headset_linked: Option<bool>,
    /// Name parts of the headset's own audio endpoints (from the device platform).
    headset_endpoints: Vec<String>,
    /// A supported device is connected. Without one nothing runs (`app.md` §8); `false`
    /// until the headset thread has looked (fail safe).
    device_present: bool,
    /// Whether we made the virtual mic the default input (SON-06).
    default_input: DefaultInput,
    status: AudioStatus,
}

impl<B: AudioBackend> AudioSupervisor<B> {
    /// Creates an idle supervisor.
    pub fn new(backend: B, config: Config) -> Self {
        Self {
            backend,
            config,
            processes: Vec::new(),
            mix: None,
            mix_device: None,
            failed_mix: None,
            mic: None,
            mic_device: None,
            devices: DeviceSet::default(),
            failed_mic: None,
            chatmix: None,
            headset_linked: None,
            headset_endpoints: Vec::new(),
            device_present: false,
            default_input: DefaultInput::Untouched,
            status: AudioStatus::default(),
        }
    }

    /// Current status for the UI.
    #[must_use]
    pub const fn status(&self) -> &AudioStatus {
        &self.status
    }

    /// The configuration changed (any field).
    pub fn on_config(&mut self, config: Config) {
        self.config = config;
        self.reconcile();
        self.push_live_settings();
    }

    /// Periodic re-scan of apps and devices.
    pub fn on_tick(&mut self) {
        match self.backend.processes() {
            Ok(processes) => self.processes = processes,
            Err(error) => self.status.mix = ServiceStatus::Failed(error),
        }
        self.devices = self.backend.devices();
        // Rebuild only when the device a session actually uses vanished, re-enumerated (new
        // object id after a replug or USB reset) or — for "system default" — was switched.
        // Other devices coming and going (iPhone mics, our own aggregates) must not interrupt
        // the audio.
        if let Some((plan, _)) = &self.mix {
            if self.devices.output(plan.output_uid.as_deref()) != self.mix_device.as_ref() {
                self.mix = None;
                self.failed_mix = None;
            }
        }
        if let Some((plan, _)) = &self.mic {
            if self.devices.input(plan.input_uid.as_deref()) != self.mic_device.as_ref() {
                self.mic = None;
                self.failed_mic = None;
            }
        }
        self.status.outputs.clone_from(&self.devices.outputs);
        self.status.inputs.clone_from(&self.devices.inputs);
        self.status.processes.clone_from(&self.processes);
        self.reconcile();
        if let Some((_, handle)) = self.mix.as_mut() {
            handle.refresh();
        }
    }

    /// The headset's ChatMix dial moved.
    pub fn on_chatmix(&mut self, balance: ChatMixBalance) {
        self.chatmix = Some(balance);
        if let Some((_, handle)) = self.mix.as_mut() {
            let _ = handle.send(EngineCommand::SetChatMix(balance));
        }
    }

    /// The audio device's endpoint names changed (device connected, ADR 0008).
    pub fn on_headset_endpoints(&mut self, names: Vec<String>) {
        if self.headset_endpoints != names {
            self.headset_endpoints = names;
            self.reconcile();
        }
    }

    /// The headset was turned on or off (base-station link).
    pub fn on_headset_link(&mut self, linked: bool) {
        if self.headset_linked != Some(linked) {
            self.headset_linked = Some(linked);
            self.reconcile();
        }
    }

    /// The last supported device disappeared, or one appeared. Without one AudioMix, the mic feed and
    /// the default-input switch stop (and restore what they changed), so OpenSeries never
    /// interferes with other headsets (`docs/architecture/app.md` §8).
    pub fn on_device_present(&mut self, present: bool) {
        if self.device_present != present {
            self.device_present = present;
            self.reconcile();
        }
    }

    /// The endpoint names that count as missing right now (the headset's, while it is off).
    fn missing_endpoints(&self) -> Vec<String> {
        if self.config.audiomix.fallback_when_headset_off && self.headset_linked == Some(false) {
            self.headset_endpoints.clone()
        } else {
            Vec::new()
        }
    }

    /// Stops everything (app quit).
    pub fn shutdown(&mut self) {
        self.restore_default_input();
        self.mix = None;
        self.mic = None;
        self.status.mix = ServiceStatus::Off;
        self.status.mic = ServiceStatus::Off;
    }

    fn reconcile(&mut self) {
        self.reconcile_mix();
        self.reconcile_mic();
        self.reconcile_default_input();
    }

    /// Switches the system default input to the virtual mic when the feed starts (option on) and
    /// back when it stops. Only acts on these transitions, so a default the user picks by hand in
    /// between is left alone.
    fn reconcile_default_input(&mut self) {
        let Some(virtual_uid) = self.backend.virtual_mic_uid() else {
            return;
        };
        let wanted = self.config.mic.set_as_default_input
            && matches!(self.status.mic, ServiceStatus::Running);
        if wanted && self.default_input == DefaultInput::Untouched {
            let previous = self.devices.default_input.clone();
            if previous.as_deref() == Some(virtual_uid) {
                return; // already the default; nothing to restore later
            }
            match self.backend.set_default_input(virtual_uid) {
                Ok(()) => {
                    self.default_input = DefaultInput::Replaced { previous };
                    self.devices.default_input = Some(virtual_uid.to_owned());
                    self.status.default_input_error = None;
                }
                Err(error) => self.status.default_input_error = Some(error),
            }
        } else if !wanted {
            self.restore_default_input();
        }
    }

    fn restore_default_input(&mut self) {
        let DefaultInput::Replaced { previous } =
            std::mem::replace(&mut self.default_input, DefaultInput::Untouched)
        else {
            return;
        };
        let still_ours = self.devices.default_input.as_deref() == self.backend.virtual_mic_uid();
        if let (true, Some(previous)) = (still_ours, previous) {
            if let Err(error) = self.backend.set_default_input(&previous) {
                self.status.default_input_error = Some(error);
            } else {
                self.devices.default_input = Some(previous);
            }
        }
    }

    fn reconcile_mix(&mut self) {
        if !self.device_present {
            self.mix = None;
            self.failed_mix = None;
            self.status.routed_apps = 0;
            self.status.mix = paused_status(self.config.audiomix.enabled);
            return;
        }
        let plan = match mix_decision(&self.config.audiomix, &self.processes) {
            MixDecision::Off => {
                self.mix = None;
                self.failed_mix = None;
                self.status.mix = ServiceStatus::Off;
                self.status.routed_apps = 0;
                return;
            }
            MixDecision::Blocked(tools) => {
                self.mix = None;
                self.status.mix = ServiceStatus::Blocked(format!(
                    "{} is running and also captures app audio",
                    tools.join(", ")
                ));
                return;
            }
            MixDecision::Run(mut plan) => {
                plan.output_uid = resolve_device(
                    plan.output_uid.as_deref(),
                    &self.config.audiomix.output_fallback,
                    &self.devices.outputs,
                    self.devices.default_output.as_deref(),
                    &self.missing_endpoints(),
                );
                plan
            }
        };
        let unchanged = self
            .mix
            .as_ref()
            .is_some_and(|(running, _)| *running == plan);
        if unchanged || self.failed_mix.as_ref() == Some(&plan) {
            return;
        }
        self.mix = None; // stop before starting the replacement (taps are exclusive per app)
        self.status.routed_apps = plan.routes.len();
        match self.backend.start_mix(&plan) {
            Ok(handle) => {
                self.mix_device = self.devices.output(plan.output_uid.as_deref()).cloned();
                self.status.mix_starts = self.status.mix_starts.saturating_add(1);
                self.mix = Some((plan, handle));
                self.failed_mix = None;
                self.status.mix = ServiceStatus::Running;
                self.push_live_settings();
            }
            Err(error) => {
                self.failed_mix = Some(plan);
                self.status.mix = ServiceStatus::Failed(error);
            }
        }
    }

    fn reconcile_mic(&mut self) {
        if !self.device_present {
            self.mic = None;
            self.failed_mic = None;
            self.status.mic = paused_status(self.config.mic.enabled);
            return;
        }
        let plan = match mic_plan(&self.config.mic) {
            Ok(None) => {
                self.mic = None;
                self.failed_mic = None;
                self.status.mic = ServiceStatus::Off;
                return;
            }
            Ok(Some(mut plan)) => {
                plan.input_uid = resolve_device(
                    plan.input_uid.as_deref(),
                    &self.config.mic.input_fallback,
                    &self.devices.inputs,
                    self.devices.default_input.as_deref(),
                    &self.missing_endpoints(),
                );
                plan
            }
            Err(error) => {
                self.mic = None;
                self.status.mic = ServiceStatus::Failed(error);
                return;
            }
        };
        // Only the chain changed: apply it live instead of restarting the feed.
        if let Some((running, handle)) = self.mic.as_mut() {
            if running.input_uid == plan.input_uid
                && running.chain != plan.chain
                && handle.set_chain(&plan.chain)
            {
                running.chain = plan.chain;
                return;
            }
        }
        let unchanged = self
            .mic
            .as_ref()
            .is_some_and(|(running, _)| *running == plan);
        if unchanged || self.failed_mic.as_ref() == Some(&plan) {
            return;
        }
        self.mic = None;
        match self.backend.start_mic(&plan) {
            Ok(handle) => {
                self.mic_device = self.devices.input(plan.input_uid.as_deref()).cloned();
                self.mic = Some((plan, handle));
                self.failed_mic = None;
                self.status.mic = ServiceStatus::Running;
            }
            Err(error) => {
                self.failed_mic = Some(plan);
                self.status.mic = ServiceStatus::Failed(error);
            }
        }
    }

    /// Sends faders, strips and ChatMix to the running engine (cheap; no rebuild).
    fn push_live_settings(&mut self) {
        let Some((_, handle)) = self.mix.as_mut() else {
            return;
        };
        let audiomix = &self.config.audiomix;
        for id in ChannelId::ALL {
            let channel = audiomix.channels.get(Channel::from(id));
            let _ = handle.send(EngineCommand::SetFader(id, Fader::from(channel.fader)));
            if let Ok(strip) = channel.strip() {
                let _ = handle.send(EngineCommand::SetStrip(id, strip));
            }
        }
        let _ = handle.send(EngineCommand::SetMaster(Fader::from(audiomix.master)));
        if let Some(balance) = self.chatmix {
            let _ = handle.send(EngineCommand::SetChatMix(balance));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use ss_store::config::Channel;

    /// Records what the supervisor asked for.
    #[derive(Default)]
    struct Log {
        mix_attempts: usize,
        mix_starts: Vec<MixPlan>,
        mic_starts: Vec<MicPlan>,
        commands: usize,
        live_mixes: usize,
        refreshes: usize,
        default_inputs: Vec<String>,
        live_chain_changes: usize,
        /// Simulates a feed that cannot change its chain live.
        no_live_chain: bool,
    }

    struct FakeMix(Rc<RefCell<Log>>);

    impl MixHandle for FakeMix {
        fn send(&mut self, _command: EngineCommand) -> bool {
            self.0.borrow_mut().commands += 1;
            true
        }

        fn refresh(&mut self) {
            self.0.borrow_mut().refreshes += 1;
        }
    }

    impl Drop for FakeMix {
        fn drop(&mut self) {
            self.0.borrow_mut().live_mixes -= 1;
        }
    }

    struct FakeMic(Rc<RefCell<Log>>);
    impl MicHandle for FakeMic {
        fn set_chain(&mut self, _chain: &ss_dsp::MicChainSettings) -> bool {
            let mut log = self.0.borrow_mut();
            if log.no_live_chain {
                return false;
            }
            log.live_chain_changes += 1;
            true
        }
    }

    struct FakeBackend {
        log: Rc<RefCell<Log>>,
        processes: Rc<RefCell<Vec<ProcessInfo>>>,
        devices: Rc<RefCell<DeviceSet>>,
        fail_mix: bool,
    }

    impl AudioBackend for FakeBackend {
        fn processes(&mut self) -> Result<Vec<ProcessInfo>, String> {
            Ok(self.processes.borrow().clone())
        }

        fn devices(&mut self) -> DeviceSet {
            self.devices.borrow().clone()
        }

        fn start_mix(&mut self, plan: &MixPlan) -> Result<Box<dyn MixHandle>, String> {
            self.log.borrow_mut().mix_attempts += 1;
            if self.fail_mix {
                return Err("no permission".to_owned());
            }
            let mut log = self.log.borrow_mut();
            log.mix_starts.push(plan.clone());
            log.live_mixes += 1;
            Ok(Box::new(FakeMix(Rc::clone(&self.log))))
        }

        fn start_mic(&mut self, plan: &MicPlan) -> Result<Box<dyn MicHandle>, String> {
            self.log.borrow_mut().mic_starts.push(plan.clone());
            Ok(Box::new(FakeMic(Rc::clone(&self.log))))
        }

        fn virtual_mic_uid(&self) -> Option<&'static str> {
            Some("vmic")
        }

        fn set_default_input(&mut self, uid: &str) -> Result<(), String> {
            self.log.borrow_mut().default_inputs.push(uid.to_owned());
            self.devices.borrow_mut().default_input = Some(uid.to_owned());
            Ok(())
        }
    }

    struct Harness {
        supervisor: AudioSupervisor<FakeBackend>,
        log: Rc<RefCell<Log>>,
        processes: Rc<RefCell<Vec<ProcessInfo>>>,
        devices: Rc<RefCell<DeviceSet>>,
    }

    fn harness(fail_mix: bool) -> Harness {
        let log = Rc::new(RefCell::new(Log::default()));
        let processes = Rc::new(RefCell::new(Vec::new()));
        let devices = Rc::new(RefCell::new(DeviceSet::default()));
        let backend = FakeBackend {
            log: Rc::clone(&log),
            processes: Rc::clone(&processes),
            devices: Rc::clone(&devices),
            fail_mix,
        };
        let mut supervisor = AudioSupervisor::new(backend, Config::default());
        // Most tests assume the base station is connected; the gate has its own tests.
        supervisor.on_device_present(true);
        Harness {
            supervisor,
            log,
            processes,
            devices,
        }
    }

    #[test]
    fn nothing_starts_before_the_device_present_is_seen() {
        let mut h = harness(false);
        h.supervisor.on_device_present(false);
        let mut config = enabled_with(&[]);
        config.mic.enabled = true;
        h.supervisor.on_tick();
        h.supervisor.on_config(config);
        h.supervisor.on_tick();
        assert_eq!(h.log.borrow().mix_starts.len(), 0);
        assert_eq!(h.log.borrow().mic_starts.len(), 0);
        let paused = ServiceStatus::Blocked(PAUSED_REASON.to_owned());
        assert_eq!(h.supervisor.status().mix, paused);
        assert_eq!(h.supervisor.status().mic, paused);
    }

    #[test]
    fn unplugging_the_device_present_stops_everything_and_restores_the_default_input() {
        let mut h = harness(false);
        h.devices.borrow_mut().default_input = Some("hs-mic".to_owned());
        h.supervisor.on_tick();
        let mut config = enabled_with(&[]);
        config.mic.enabled = true;
        config.mic.set_as_default_input = true;
        h.supervisor.on_config(config);
        assert_eq!(h.log.borrow().live_mixes, 1);

        h.supervisor.on_device_present(false);
        assert_eq!(h.log.borrow().live_mixes, 0, "mix handle dropped");
        assert_eq!(h.log.borrow().default_inputs, vec!["vmic", "hs-mic"]);
        assert_eq!(h.supervisor.status().routed_apps, 0);

        h.supervisor.on_device_present(true);
        assert_eq!(h.log.borrow().live_mixes, 1, "resumes on its own");
        assert_eq!(h.log.borrow().mix_starts.len(), 2);
    }

    #[test]
    fn disabled_services_stay_off_while_paused() {
        let mut h = harness(false);
        h.supervisor.on_device_present(false);
        h.supervisor.on_config(Config::default());
        assert_eq!(h.supervisor.status().mix, ServiceStatus::Off);
        assert_eq!(h.supervisor.status().mic, ServiceStatus::Off);
    }

    fn process(pid: i32, bundle: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            bundle_id: Some(bundle.to_owned()),
            playing: false,
        }
    }

    fn enabled_with(assignments: &[(&str, Channel)]) -> Config {
        let mut config = Config::default();
        config.audiomix.enabled = true;
        for (bundle, channel) in assignments {
            config
                .audiomix
                .assignments
                .insert((*bundle).to_owned(), *channel);
        }
        config
    }

    #[test]
    fn nothing_starts_while_the_master_switch_is_off() {
        let mut h = harness(false);
        h.supervisor.on_tick();
        h.supervisor.on_config(Config::default());
        assert_eq!(h.log.borrow().mix_starts.len(), 0);
        assert_eq!(h.supervisor.status().mix, ServiceStatus::Off);
    }

    #[test]
    fn headset_off_moves_the_mix_to_the_backup_and_back() {
        let mut h = harness(false);
        *h.devices.borrow_mut() = DeviceSet {
            outputs: vec![
                DeviceInfo {
                    uid: "hs".to_owned(),
                    name: "Arctis Nova Pro Wireless".to_owned(),
                    id: 1,
                },
                DeviceInfo {
                    uid: "spk".to_owned(),
                    name: "Speakers".to_owned(),
                    id: 2,
                },
            ],
            ..DeviceSet::default()
        };
        let mut config = enabled_with(&[]);
        config.audiomix.output_device_uid = Some("hs".to_owned());
        config.audiomix.output_fallback = vec!["spk".to_owned()];
        config.audiomix.fallback_when_headset_off = true;
        h.supervisor.on_tick();
        h.supervisor.on_config(config);
        h.supervisor
            .on_headset_endpoints(vec!["Arctis Nova Pro Wireless".to_owned()]);
        h.supervisor.on_headset_link(true);
        h.supervisor.on_headset_link(false);
        h.supervisor.on_headset_link(true);
        let outputs: Vec<_> = h
            .log
            .borrow()
            .mix_starts
            .iter()
            .map(|p| p.output_uid.clone())
            .collect();
        assert_eq!(
            outputs,
            vec![
                Some("hs".to_owned()),
                Some("spk".to_owned()),
                Some("hs".to_owned())
            ]
        );
    }

    #[test]
    fn virtual_mic_becomes_default_input_while_running_and_is_restored() {
        let mut h = harness(false);
        h.devices.borrow_mut().default_input = Some("hs-mic".to_owned());
        h.supervisor.on_tick();
        let mut config = Config::default();
        config.mic.enabled = true;
        config.mic.set_as_default_input = true;
        h.supervisor.on_config(config.clone());
        h.supervisor.on_tick();
        h.supervisor.on_tick(); // no repeated switching
        config.mic.enabled = false;
        h.supervisor.on_config(config);
        assert_eq!(h.log.borrow().default_inputs, vec!["vmic", "hs-mic"]);
    }

    #[test]
    fn a_default_chosen_by_hand_meanwhile_is_kept() {
        let mut h = harness(false);
        h.devices.borrow_mut().default_input = Some("hs-mic".to_owned());
        h.supervisor.on_tick();
        let mut config = Config::default();
        config.mic.enabled = true;
        config.mic.set_as_default_input = true;
        h.supervisor.on_config(config.clone());
        // The user picks another input in System Settings.
        h.devices.borrow_mut().default_input = Some("usb-mic".to_owned());
        h.supervisor.on_tick();
        config.mic.enabled = false;
        h.supervisor.on_config(config);
        assert_eq!(h.log.borrow().default_inputs, vec!["vmic"]);
    }

    #[test]
    fn mic_chain_changes_apply_live_and_device_changes_restart() {
        let mut h = harness(false);
        h.supervisor.on_tick();
        let mut config = Config::default();
        config.mic.enabled = true;
        h.supervisor.on_config(config.clone());
        config.mic.stabilizer = true;
        h.supervisor.on_config(config.clone());
        config.mic.gain_db = 3.0;
        h.supervisor.on_config(config.clone());
        assert_eq!(h.log.borrow().mic_starts.len(), 1);
        assert_eq!(h.log.borrow().live_chain_changes, 2);
        config.mic.input_device_uid = Some("usb-mic".to_owned());
        h.supervisor.on_config(config);
        assert_eq!(h.log.borrow().mic_starts.len(), 2);
    }

    #[test]
    fn ticks_refresh_the_running_mix_only() {
        let mut h = harness(false);
        h.supervisor.on_tick();
        assert_eq!(h.log.borrow().refreshes, 0);
        h.supervisor.on_config(enabled_with(&[]));
        h.supervisor.on_tick();
        h.supervisor.on_tick();
        assert_eq!(h.log.borrow().refreshes, 2);
    }

    #[test]
    fn enabling_starts_once_and_ticks_do_not_rebuild() {
        let mut h = harness(false);
        h.supervisor.on_config(enabled_with(&[]));
        h.supervisor.on_tick();
        h.supervisor.on_tick();
        assert_eq!(h.log.borrow().mix_starts.len(), 1);
        assert_eq!(h.supervisor.status().mix, ServiceStatus::Running);
    }

    #[test]
    fn an_assigned_app_starting_later_triggers_one_rebuild() {
        let mut h = harness(false);
        h.supervisor
            .on_config(enabled_with(&[("com.hnc.Discord", Channel::Chat)]));
        h.processes
            .borrow_mut()
            .push(process(42, "com.hnc.Discord"));
        h.supervisor.on_tick();
        h.supervisor.on_tick();
        let log = h.log.borrow();
        assert_eq!(log.mix_starts.len(), 2);
        assert_eq!(log.mix_starts[1].routes.get(&42), Some(&ChannelId::Chat));
        assert_eq!(log.live_mixes, 1, "the old session was dropped first");
    }

    #[test]
    fn switching_off_stops_the_mix() {
        let mut h = harness(false);
        h.supervisor.on_config(enabled_with(&[]));
        h.supervisor.on_config(Config::default());
        assert_eq!(h.log.borrow().live_mixes, 0);
        assert_eq!(h.supervisor.status().mix, ServiceStatus::Off);
    }

    #[test]
    fn a_running_tap_tool_blocks_until_it_quits() {
        let mut h = harness(false);
        h.processes
            .borrow_mut()
            .push(process(7, "com.finetuneapp.FineTune"));
        h.supervisor.on_tick();
        h.supervisor.on_config(enabled_with(&[]));
        assert!(matches!(
            h.supervisor.status().mix,
            ServiceStatus::Blocked(_)
        ));
        assert_eq!(h.log.borrow().mix_starts.len(), 0);

        h.processes.borrow_mut().clear();
        h.supervisor.on_tick();
        assert_eq!(h.supervisor.status().mix, ServiceStatus::Running);
    }

    #[test]
    fn live_changes_do_not_rebuild() {
        let mut h = harness(false);
        let mut config = enabled_with(&[]);
        h.supervisor.on_config(config.clone());
        let commands_before = h.log.borrow().commands;
        config.audiomix.channels.game.fader.volume = 0.5;
        config.audiomix.channels.media.boosts.bass_db = 6.0;
        h.supervisor.on_config(config);
        let log = h.log.borrow();
        assert_eq!(log.mix_starts.len(), 1);
        assert!(log.commands > commands_before);
    }

    #[test]
    fn a_failed_start_is_not_retried_until_something_changes() {
        let mut h = harness(true);
        h.supervisor.on_config(enabled_with(&[]));
        h.supervisor.on_tick();
        h.supervisor.on_tick();
        assert!(matches!(
            h.supervisor.status().mix,
            ServiceStatus::Failed(_)
        ));
        assert_eq!(
            h.log.borrow().mix_attempts,
            1,
            "no retry loop on an unchanged plan"
        );

        // A relevant change (an assigned app appears) is worth another attempt.
        h.supervisor
            .on_config(enabled_with(&[("com.hnc.Discord", Channel::Chat)]));
        h.processes.borrow_mut().push(process(9, "com.hnc.Discord"));
        h.supervisor.on_tick();
        assert_eq!(h.log.borrow().mix_attempts, 2);
    }

    #[test]
    fn mic_chain_changes_restart_a_feed_without_live_control() {
        let mut h = harness(false);
        h.log.borrow_mut().no_live_chain = true;
        let mut config = Config::default();
        config.mic.enabled = true;
        h.supervisor.on_config(config.clone());
        config.mic.denoise = Some(0.9);
        h.supervisor.on_config(config.clone());
        h.supervisor.on_config(config);
        assert_eq!(h.log.borrow().mic_starts.len(), 2);
        assert_eq!(h.supervisor.status().mic, ServiceStatus::Running);
    }

    fn device(uid: &str, id: u32) -> DeviceInfo {
        DeviceInfo {
            uid: uid.to_owned(),
            name: uid.to_owned(),
            id,
        }
    }

    /// Mix running on the default output "arctis" (id 119).
    fn running_on_default_output() -> Harness {
        let h = harness(false);
        {
            let mut devices = h.devices.borrow_mut();
            devices.outputs = vec![device("arctis", 119), device("speakers", 85)];
            devices.default_output = Some("arctis".to_owned());
        }
        let mut h = h;
        h.supervisor.on_tick();
        h.supervisor.on_config(enabled_with(&[]));
        h.supervisor.on_tick();
        assert_eq!(h.log.borrow().mix_starts.len(), 1);
        h
    }

    #[test]
    fn a_re_enumerated_output_device_triggers_a_rebuild() {
        let mut h = running_on_default_output();
        // USB reset: same UID, new object id.
        h.devices.borrow_mut().outputs[0] = device("arctis", 143);
        h.supervisor.on_tick();
        h.supervisor.on_tick();
        let log = h.log.borrow();
        assert_eq!(log.mix_starts.len(), 2, "exactly one rebuild");
        assert_eq!(log.live_mixes, 1);
    }

    #[test]
    fn unrelated_devices_coming_and_going_do_not_interrupt_the_mix() {
        // Regression: our own aggregate device and iPhone mics appearing in the list caused a
        // rebuild every tick (audible stutter).
        let mut h = running_on_default_output();
        h.devices.borrow_mut().outputs.push(device("iphone", 200));
        h.devices
            .borrow_mut()
            .inputs
            .push(device("iphone-mic", 201));
        h.supervisor.on_tick();
        h.devices.borrow_mut().outputs.pop();
        h.supervisor.on_tick();
        assert_eq!(h.log.borrow().mix_starts.len(), 1);
    }

    #[test]
    fn switching_the_system_default_moves_a_default_mix() {
        let mut h = running_on_default_output();
        h.devices.borrow_mut().default_output = Some("speakers".to_owned());
        h.supervisor.on_tick();
        assert_eq!(h.log.borrow().mix_starts.len(), 2);
    }

    #[test]
    fn chatmix_is_reapplied_after_a_rebuild() {
        let mut h = harness(false);
        h.supervisor.on_chatmix(ss_dsp::balance_from_slider(0.0));
        h.supervisor.on_config(enabled_with(&[]));
        assert!(h.log.borrow().commands > 0);
    }
}

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
pub trait MicHandle {}

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
    /// Output devices.
    pub outputs: Vec<DeviceInfo>,
    /// Input devices.
    pub inputs: Vec<DeviceInfo>,
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

    /// Stops everything (app quit).
    pub fn shutdown(&mut self) {
        self.mix = None;
        self.mic = None;
        self.status.mix = ServiceStatus::Off;
        self.status.mic = ServiceStatus::Off;
    }

    fn reconcile(&mut self) {
        self.reconcile_mix();
        self.reconcile_mic();
    }

    fn reconcile_mix(&mut self) {
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
            MixDecision::Run(plan) => plan,
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
        let plan = match mic_plan(&self.config.mic) {
            Ok(None) => {
                self.mic = None;
                self.failed_mic = None;
                self.status.mic = ServiceStatus::Off;
                return;
            }
            Ok(Some(plan)) => plan,
            Err(error) => {
                self.mic = None;
                self.status.mic = ServiceStatus::Failed(error);
                return;
            }
        };
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

    struct FakeMic;
    impl MicHandle for FakeMic {}

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
            Ok(Box::new(FakeMic))
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
        Harness {
            supervisor: AudioSupervisor::new(backend, Config::default()),
            log,
            processes,
            devices,
        }
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
    fn mic_chain_changes_restart_the_mic_feed() {
        let mut h = harness(false);
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

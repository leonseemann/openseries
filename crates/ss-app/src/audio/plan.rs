//! What the audio side should run for a given configuration and set of running apps
//! (`docs/architecture/app.md` §1). Pure data, compared by value to decide on rebuilds.

use std::collections::BTreeMap;

use ss_audio_macos::plan::{Pid, conflicting_tools};
use ss_core::mixer::ChannelId;
use ss_dsp::MicChainSettings;
use ss_store::config::{AudioMixConfig, MicConfig};

/// A process known to the audio system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    /// Unix process id.
    pub pid: Pid,
    /// Bundle id, if any.
    pub bundle_id: Option<String>,
    /// Currently producing audio.
    pub playing: bool,
}

/// An audio device offered in the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Persistent UID.
    pub uid: String,
    /// Display name.
    pub name: String,
    /// Current CoreAudio object id; changes when the device re-enumerates (replug, USB reset).
    pub id: u32,
}

/// The audio devices relevant to the app (own aggregate and virtual devices filtered out).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceSet {
    /// Output devices.
    pub outputs: Vec<DeviceInfo>,
    /// Input devices.
    pub inputs: Vec<DeviceInfo>,
    /// UID of the system default output.
    pub default_output: Option<String>,
    /// UID of the system default input.
    pub default_input: Option<String>,
}

impl DeviceSet {
    /// The output a session with `uid` (`None` = system default) uses right now.
    #[must_use]
    pub fn output(&self, uid: Option<&str>) -> Option<&DeviceInfo> {
        let uid = uid.or(self.default_output.as_deref())?;
        self.outputs.iter().find(|d| d.uid == uid)
    }

    /// The input a session with `uid` (`None` = system default) uses right now.
    #[must_use]
    pub fn input(&self, uid: Option<&str>) -> Option<&DeviceInfo> {
        let uid = uid.or(self.default_input.as_deref())?;
        self.inputs.iter().find(|d| d.uid == uid)
    }
}

/// The AudioMix session that should run. Any difference requires rebuilding the taps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixPlan {
    /// Output device UID, `None` = system default.
    pub output_uid: Option<String>,
    /// Running processes routed away from Media.
    pub routes: BTreeMap<Pid, ChannelId>,
}

/// Why AudioMix does not run although it is enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixDecision {
    /// Master switch off.
    Off,
    /// Enabled, but another tap-based tool is running.
    Blocked(Vec<&'static str>),
    /// Run this plan.
    Run(MixPlan),
}

/// Decides the AudioMix session for `config` and the running `processes`.
#[must_use]
pub fn mix_decision(config: &AudioMixConfig, processes: &[ProcessInfo]) -> MixDecision {
    if !config.enabled {
        return MixDecision::Off;
    }
    let conflicts = conflicting_tools(processes.iter().filter_map(|p| p.bundle_id.as_deref()));
    if !conflicts.is_empty() {
        return MixDecision::Blocked(conflicts);
    }
    let routes = processes
        .iter()
        .filter_map(|process| {
            let bundle_id = process.bundle_id.as_deref()?;
            let channel = assigned_channel(config, bundle_id)?;
            (channel != ChannelId::Media).then_some((process.pid, channel))
        })
        .collect();
    MixDecision::Run(MixPlan {
        output_uid: config.output_device_uid.clone(),
        routes,
    })
}

/// Channel of the longest assignment that is a prefix of `bundle_id` (helper processes such as
/// `com.google.Chrome.helper` follow their app).
fn assigned_channel(config: &AudioMixConfig, bundle_id: &str) -> Option<ChannelId> {
    config
        .assignments
        .iter()
        .filter(|(prefix, _)| bundle_id.starts_with(prefix.as_str()))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, &channel)| channel.into())
}

/// Part of the name of the Nova Pro's own audio devices (**Observed** on macOS and Windows:
/// "Arctis Nova Pro Wireless", e.g. "Kopfhörer (2- Arctis Nova Pro Wireless)").
const HEADSET_DEVICE_NAME: &str = "Arctis Nova Pro Wireless";

/// Picks the device a session should use (SON-27/28): the selected one (`None` = system
/// default), else the first connected backup in priority order. With `headset_off`, the
/// headset's own devices count as missing. If nothing usable is connected, the selection is kept
/// unchanged (the session then fails or waits as before). Without backups and with the headset
/// on, the result is exactly `selected`, so "system default" keeps following the default.
#[must_use]
pub fn resolve_device(
    selected: Option<&str>,
    fallback: &[String],
    devices: &[DeviceInfo],
    default: Option<&str>,
    headset_off: bool,
) -> Option<String> {
    if fallback.is_empty() && !headset_off {
        return selected.map(str::to_owned);
    }
    let usable = |uid: &str| {
        devices
            .iter()
            .any(|d| d.uid == uid && !(headset_off && d.name.contains(HEADSET_DEVICE_NAME)))
    };
    let primary = selected.or(default);
    if primary.is_some_and(usable) {
        return selected.map(str::to_owned);
    }
    fallback
        .iter()
        .find(|uid| usable(uid))
        .cloned()
        .or_else(|| selected.map(str::to_owned))
}

/// The mic feed that should run. The chain is part of the plan because it cannot be changed
/// while running yet (live control comes with the command queue).
#[derive(Debug, Clone, PartialEq)]
pub struct MicPlan {
    /// Source mic UID, `None` = system default input.
    pub input_uid: Option<String>,
    /// Processing chain.
    pub chain: MicChainSettings,
}

/// The mic feed for `config`, or `None` if it is off.
///
/// # Errors
/// A message if the stored mic EQ is invalid.
pub fn mic_plan(config: &MicConfig) -> Result<Option<MicPlan>, String> {
    if !config.enabled {
        return Ok(None);
    }
    let chain = config.chain().map_err(|e| e.to_string())?;
    Ok(Some(MicPlan {
        input_uid: config.input_device_uid.clone(),
        chain,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss_store::config::Channel;

    fn process(pid: Pid, bundle: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            bundle_id: Some(bundle.to_owned()),
            playing: false,
        }
    }

    fn enabled(assignments: &[(&str, Channel)]) -> AudioMixConfig {
        AudioMixConfig {
            enabled: true,
            assignments: assignments
                .iter()
                .map(|(b, c)| ((*b).to_owned(), *c))
                .collect(),
            ..AudioMixConfig::default()
        }
    }

    #[test]
    fn master_switch_off_means_nothing_runs() {
        let config = AudioMixConfig::default();
        assert_eq!(mix_decision(&config, &[]), MixDecision::Off);
    }

    #[test]
    fn helper_processes_follow_their_app_and_media_is_implicit() {
        let config = enabled(&[
            ("com.google.Chrome", Channel::Game),
            ("com.spotify.client", Channel::Media),
        ]);
        let processes = [
            process(10, "com.google.Chrome.helper"),
            process(11, "com.google.Chrome.helper"),
            process(20, "com.spotify.client"),
            process(30, "us.zoom.xos"),
        ];
        let MixDecision::Run(plan) = mix_decision(&config, &processes) else {
            panic!("expected a plan");
        };
        assert_eq!(
            plan.routes,
            BTreeMap::from([(10, ChannelId::Game), (11, ChannelId::Game)])
        );
    }

    #[test]
    fn longest_prefix_wins() {
        let config = enabled(&[
            ("com.example", Channel::Game),
            ("com.example.voice", Channel::Chat),
        ]);
        let MixDecision::Run(plan) =
            mix_decision(&config, &[process(1, "com.example.voice.helper")])
        else {
            panic!("expected a plan");
        };
        assert_eq!(plan.routes.get(&1), Some(&ChannelId::Chat));
    }

    #[test]
    fn tap_tools_block_the_mix() {
        let config = enabled(&[]);
        let decision = mix_decision(&config, &[process(5, "com.finetuneapp.FineTune")]);
        assert_eq!(decision, MixDecision::Blocked(vec!["FineTune"]));
    }

    fn device(uid: &str, name: &str) -> DeviceInfo {
        DeviceInfo {
            uid: uid.to_owned(),
            name: name.to_owned(),
            id: 0,
        }
    }

    #[test]
    fn selected_device_wins_while_connected() {
        let devices = [
            device("hs", "Arctis Nova Pro Wireless"),
            device("spk", "Speakers"),
        ];
        let backups = ["spk".to_owned()];
        assert_eq!(
            resolve_device(Some("hs"), &backups, &devices, None, false).as_deref(),
            Some("hs")
        );
        // System default stays "None" so it keeps following the default.
        assert_eq!(
            resolve_device(None, &backups, &devices, Some("hs"), false),
            None
        );
        assert_eq!(resolve_device(None, &[], &devices, Some("hs"), false), None);
    }

    #[test]
    fn missing_device_falls_back_in_priority_order() {
        let devices = [device("spk", "Speakers"), device("usb", "USB DAC")];
        let backups = ["gone".to_owned(), "usb".to_owned(), "spk".to_owned()];
        assert_eq!(
            resolve_device(Some("hs"), &backups, &devices, None, false).as_deref(),
            Some("usb")
        );
        // Nothing usable: keep the selection.
        assert_eq!(
            resolve_device(Some("hs"), &["gone".to_owned()], &devices, None, false).as_deref(),
            Some("hs")
        );
    }

    #[test]
    fn headset_off_skips_the_headset_devices() {
        let devices = [
            device("hs", "Kopfhörer (2- Arctis Nova Pro Wireless)"),
            device("spk", "Speakers"),
        ];
        let backups = ["spk".to_owned()];
        assert_eq!(
            resolve_device(Some("hs"), &backups, &devices, None, true).as_deref(),
            Some("spk")
        );
        assert_eq!(
            resolve_device(None, &backups, &devices, Some("hs"), true).as_deref(),
            Some("spk")
        );
    }

    #[test]
    fn mic_plan_follows_the_switch() {
        assert_eq!(mic_plan(&MicConfig::default()), Ok(None));
        let config = MicConfig {
            enabled: true,
            denoise: Some(0.9),
            ..MicConfig::default()
        };
        let plan = mic_plan(&config).unwrap().unwrap();
        assert_eq!(plan.chain.denoise, Some(0.9));
    }
}

//! [`AudioBackend`] on top of `ss-audio-macos` (process taps, aggregate devices, mic feed).

use ss_audio_macos::plan::{Pid, plan_taps};
use ss_audio_macos::{
    AudioMixSession, InputDevice, MicFeedSession, OutputDevice, VIRTUAL_MIC_UID,
    default_input_device, default_output_device, list_audio_processes, list_input_devices,
    list_output_devices,
};
use ss_dsp::MicChain;
use ss_engine::{Command as EngineCommand, EngineController};

use super::plan::{DeviceInfo, MicPlan, MixPlan, ProcessInfo};
use super::supervisor::{AudioBackend, MicHandle, MixHandle};

/// Sample rate of the mic chain and the virtual microphone (`virtual-mic-driver.md` §3).
const MIC_SAMPLE_RATE: f32 = 48_000.0;

/// The macOS backend (stateless; sessions live in the handles).
pub struct MacAudioBackend;

struct MixSession {
    // Field order = drop order: stop the engine control before the session.
    controller: EngineController,
    _session: AudioMixSession,
}

impl MixHandle for MixSession {
    fn send(&mut self, command: EngineCommand) -> bool {
        self.controller.send(command).is_ok()
    }
}

struct MicSession(#[allow(dead_code)] MicFeedSession); // kept alive; drop stops the feed

impl MicHandle for MicSession {}

impl AudioBackend for MacAudioBackend {
    fn processes(&mut self) -> Result<Vec<ProcessInfo>, String> {
        let processes = list_audio_processes().map_err(|e| e.to_string())?;
        Ok(processes
            .into_iter()
            .map(|p| ProcessInfo {
                pid: p.pid,
                bundle_id: p.bundle_id,
                playing: p.is_playing,
            })
            .collect())
    }

    fn devices(&mut self) -> (Vec<DeviceInfo>, Vec<DeviceInfo>) {
        let outputs = list_output_devices()
            .unwrap_or_default()
            .into_iter()
            .map(|d| DeviceInfo {
                uid: d.uid,
                name: d.name,
            })
            .collect();
        let inputs = list_input_devices()
            .unwrap_or_default()
            .into_iter()
            .filter(|d| d.uid != VIRTUAL_MIC_UID)
            .map(|d| DeviceInfo {
                uid: d.uid,
                name: d.name,
            })
            .collect();
        (outputs, inputs)
    }

    fn start_mix(&mut self, plan: &MixPlan) -> Result<Box<dyn MixHandle>, String> {
        let output = output_device(plan.output_uid.as_deref())?;
        let specs = plan_taps(&plan.routes, own_pid());
        let (session, controller) = AudioMixSession::start(&output, &specs).map_err(|e| {
            format!(
                "{e} — allow StealSeries under Privacy & Security → Screen & System Audio Recording"
            )
        })?;
        Ok(Box::new(MixSession {
            controller,
            _session: session,
        }))
    }

    fn start_mic(&mut self, plan: &MicPlan) -> Result<Box<dyn MicHandle>, String> {
        let input = input_device(plan.input_uid.as_deref())?;
        let mut chain = MicChain::new(MIC_SAMPLE_RATE).map_err(|e| e.to_string())?;
        chain.set(&plan.chain);
        let session = MicFeedSession::start(&input, chain).map_err(|e| e.to_string())?;
        Ok(Box::new(MicSession(session)))
    }
}

fn output_device(uid: Option<&str>) -> Result<OutputDevice, String> {
    match uid {
        None => default_output_device().map_err(|e| e.to_string()),
        Some(uid) => list_output_devices()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|d| d.uid == uid)
            .ok_or_else(|| "the selected output device is not connected".to_owned()),
    }
}

fn input_device(uid: Option<&str>) -> Result<InputDevice, String> {
    let device = match uid {
        None => default_input_device().map_err(|e| e.to_string())?,
        Some(uid) => list_input_devices()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|d| d.uid == uid)
            .ok_or_else(|| "the selected microphone is not connected".to_owned())?,
    };
    if device.uid == VIRTUAL_MIC_UID {
        return Err(
            "the system input is StealSeries Microphone itself; choose the headset as source"
                .to_owned(),
        );
    }
    Ok(device)
}

fn own_pid() -> Pid {
    Pid::try_from(std::process::id()).unwrap_or(Pid::MAX)
}

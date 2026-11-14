//! Command implementations.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use ss_audio_macos::plan::{Pid, plan_taps};
use ss_audio_macos::{
    AudioMixSession, AudioProcess, MAX_PROBED_BUFFERS, OutputDevice, ProbeSnapshot,
    default_output_device, list_audio_processes, list_output_devices,
};
use ss_core::mixer::ChannelId;
use ss_dsp::{BoostSettings, ChannelStripSettings, SmartVolume};
use ss_engine::Command as EngineCommand;

use crate::cli::{Action, Cli, RunArgs};

/// How often the main thread checks for Ctrl-C.
const IDLE_POLL: Duration = Duration::from_millis(200);
/// Main-loop polls between two probe reports (= one second).
const POLLS_PER_PROBE_REPORT: u32 = 5;
/// Peaks below this are printed as silence.
const SILENCE_DBFS: f32 = -120.0;
/// Seconds of exact digital silence on every tap before warning about the permission.
const SILENT_TAP_WARNING_SECONDS: u32 = 3;

/// Runs the selected action.
///
/// # Errors
/// Any CoreAudio, HID or argument error.
pub fn execute(cli: Cli) -> Result<()> {
    match cli.action {
        Action::Devices => {
            for d in list_output_devices()? {
                println!(
                    "{:>4}  {:<40} {} ch  {}",
                    d.id, d.name, d.output_channels, d.uid
                );
            }
            Ok(())
        }
        Action::Apps => {
            for p in list_audio_processes()? {
                let state = if p.is_playing { "playing" } else { "" };
                println!(
                    "{:>7}  {:<45} {state}",
                    p.pid,
                    p.bundle_id.as_deref().unwrap_or("?")
                );
            }
            Ok(())
        }
        Action::Run(args) => run(&args),
    }
}

fn run(args: &RunArgs) -> Result<()> {
    let output = select_output(args.output.as_deref())?;
    let processes = list_audio_processes()?;
    let assignments = assignments(args, &processes)?;
    let specs = plan_taps(&assignments, own_pid());

    let (session, mut controller) = AudioMixSession::start(&output, &specs)
        .context("starting AudioMix (grant audio capture permission if macOS asked)")?;
    println!(
        "Mixing {} tap(s) onto \"{}\". Ctrl-C to stop.",
        session.tap_count(),
        output.name
    );

    configure_strips(args, &mut controller)?;
    let dial = start_dial(args.dial, controller)?;

    if args.probe {
        print_stream_layout(&session)?;
    }
    let mut polls = 0_u32;
    let mut silence = SilentTapWatch::default();
    wait_for_ctrl_c(|| {
        polls += 1;
        if polls % POLLS_PER_PROBE_REPORT == 0 {
            let snapshot = session.probe();
            silence.observe(&session, &snapshot);
            if args.probe {
                print_probe(&snapshot);
            }
        }
    })?;
    if let Some(dial) = dial {
        dial.stop()?;
    }
    drop(session); // removes taps → apps play normally again
    println!("Stopped.");
    Ok(())
}

#[cfg(feature = "nova-pro-dial")]
fn start_dial(
    enabled: bool,
    controller: ss_engine::EngineController,
) -> Result<Option<crate::dial::DialForwarder>> {
    if enabled {
        Ok(Some(crate::dial::DialForwarder::start(controller)?))
    } else {
        Ok(None)
    }
}

/// Without the dial feature there is nothing to stop; mirrors the forwarder's API.
#[cfg(not(feature = "nova-pro-dial"))]
struct NoDial;

#[cfg(not(feature = "nova-pro-dial"))]
impl NoDial {
    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    fn stop(self) -> Result<()> {
        Ok(())
    }
}

#[cfg(not(feature = "nova-pro-dial"))]
fn start_dial(enabled: bool, _controller: ss_engine::EngineController) -> Result<Option<NoDial>> {
    if enabled {
        bail!("ssmix was built without the `nova-pro-dial` feature");
    }
    Ok(None)
}

fn select_output(filter: Option<&str>) -> Result<OutputDevice> {
    let Some(filter) = filter else {
        return Ok(default_output_device()?);
    };
    let needle = filter.to_lowercase();
    list_output_devices()?
        .into_iter()
        .find(|d| d.name.to_lowercase().contains(&needle))
        .with_context(|| format!("no output device matching \"{filter}\""))
}

fn assignments(args: &RunArgs, processes: &[AudioProcess]) -> Result<BTreeMap<Pid, ChannelId>> {
    let mut map = BTreeMap::new();
    for (apps, channel) in [
        (&args.game, ChannelId::Game),
        (&args.chat, ChannelId::Chat),
        (&args.aux, ChannelId::Aux),
    ] {
        for app in apps {
            let pids = resolve(app, processes);
            if pids.is_empty() {
                bail!("app \"{app}\" is not running or has no audio (see `ssmix apps`)");
            }
            for pid in pids {
                map.insert(pid, channel);
            }
        }
    }
    Ok(map)
}

/// Matches a PID or a bundle id (all helper processes of an app share its prefix).
fn resolve(app: &str, processes: &[AudioProcess]) -> Vec<Pid> {
    if let Ok(pid) = app.parse::<Pid>() {
        return vec![pid];
    }
    processes
        .iter()
        .filter(|p| p.bundle_id.as_deref().is_some_and(|b| b.starts_with(app)))
        .map(|p| p.pid)
        .collect()
}

fn configure_strips(args: &RunArgs, controller: &mut ss_engine::EngineController) -> Result<()> {
    let game = ChannelStripSettings {
        boosts: BoostSettings {
            bass_db: args.game_bass,
            ..BoostSettings::default()
        },
        ..ChannelStripSettings::default()
    };
    controller.send(EngineCommand::SetStrip(ChannelId::Game, game))?;
    if args.media_smart_volume {
        let media = ChannelStripSettings {
            smart_volume: Some(SmartVolume::Balanced),
            ..ChannelStripSettings::default()
        };
        controller.send(EngineCommand::SetStrip(ChannelId::Media, media))?;
    }
    Ok(())
}

fn own_pid() -> Pid {
    Pid::try_from(std::process::id()).unwrap_or(Pid::MAX)
}

fn print_stream_layout(session: &AudioMixSession) -> Result<()> {
    let layout = session.stream_layout()?;
    println!(
        "probe: device input buffers (skipped) = {}, taps = {:?}",
        layout.device_inputs,
        session.tap_channels()
    );
    println!(
        "probe: aggregate input  channels per buffer = {:?}",
        layout.inputs
    );
    println!(
        "probe: aggregate output channels per buffer = {:?}",
        layout.outputs
    );
    Ok(())
}

/// Warns once when all taps deliver exact zeros: macOS does that (and still mutes the tapped
/// apps) when the terminal lacks "System Audio Recording" permission
/// (`docs/hardware/macos-audiomix.md`, Prerequisites).
#[derive(Debug, Default)]
struct SilentTapWatch {
    silent_seconds: u32,
    warned: bool,
}

impl SilentTapWatch {
    fn observe(&mut self, session: &AudioMixSession, snapshot: &ProbeSnapshot) {
        let Ok(layout) = session.stream_layout() else {
            return;
        };
        let taps = layout.device_inputs..snapshot.input_buffers.min(MAX_PROBED_BUFFERS);
        if taps.is_empty() || snapshot.peaks[taps].iter().any(|p| p.to_bits() != 0) {
            self.silent_seconds = 0;
            return;
        }
        self.silent_seconds += 1;
        if !self.warned && self.silent_seconds >= SILENT_TAP_WARNING_SECONDS {
            self.warned = true;
            eprintln!(
                "warning: all taps deliver digital silence. If apps are playing, allow this \
                 terminal under System Settings → Privacy & Security → Screen & System Audio \
                 Recording → System Audio Recording Only, then restart the terminal."
            );
        }
    }
}

fn print_probe(snapshot: &ProbeSnapshot) {
    let shown = snapshot.input_buffers.min(MAX_PROBED_BUFFERS);
    let peaks: Vec<String> = (0..shown)
        .map(|i| {
            format!(
                "#{i}({}ch) {:>6.1}",
                snapshot.channels[i],
                to_dbfs(snapshot.peaks[i])
            )
        })
        .collect();
    println!(
        "probe: cb={} frames={} out={}x{}ch | in dBFS: {}",
        snapshot.callbacks,
        snapshot.frames,
        snapshot.output_buffers,
        snapshot.output_channels,
        peaks.join("  ")
    );
}

fn to_dbfs(peak: f32) -> f32 {
    if peak > 0.0 {
        (20.0 * peak.log10()).max(SILENCE_DBFS)
    } else {
        SILENCE_DBFS
    }
}

/// Blocks until Ctrl-C, calling `tick` every [`IDLE_POLL`].
fn wait_for_ctrl_c(mut tick: impl FnMut()) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&running);
    ctrlc::set_handler(move || flag.store(false, Ordering::SeqCst))?;
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(IDLE_POLL);
        tick();
    }
    Ok(())
}

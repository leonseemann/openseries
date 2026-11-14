//! Command implementations.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use ss_audio_macos::plan::{Pid, plan_taps};
use ss_audio_macos::{
    AudioMixSession, AudioProcess, InputDevice, MAX_PROBED_BUFFERS, MicFeedSession, OutputDevice,
    ProbeSnapshot, default_input_device, default_output_device, list_audio_processes,
    list_input_devices, list_output_devices,
};
use ss_core::eq::ParametricEqSettings;
use ss_core::mixer::ChannelId;
use ss_dsp::{
    BoostSettings, ChannelStripSettings, GateMode, MicChain, MicChainSettings, SmartVolume,
    voice_stabilizer,
};
use ss_engine::Command as EngineCommand;

use crate::cli::{Action, Cli, MicArgs, RunArgs};

/// How often the main thread checks for Ctrl-C.
const IDLE_POLL: Duration = Duration::from_millis(200);
/// Main-loop polls between two probe reports (= one second).
const POLLS_PER_PROBE_REPORT: u32 = 5;
/// Peaks below this are printed as silence.
const SILENCE_DBFS: f32 = -120.0;
/// Seconds of exact digital silence before warning about a missing permission.
const SILENCE_WARNING_SECONDS: u32 = 3;

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
        Action::Inputs => {
            for d in list_input_devices()? {
                println!(
                    "{:>4}  {:<40} {} ch  {}",
                    d.id, d.name, d.input_channels, d.uid
                );
            }
            Ok(())
        }
        Action::Mic(args) => mic(&args),
    }
}

fn mic(args: &MicArgs) -> Result<()> {
    let input = select_input(args.input.as_deref())?;
    let settings = mic_chain_settings(args)?;
    let mut chain = MicChain::new(MIC_SAMPLE_RATE)?;
    chain.set(&settings);
    let latency_ms = ms(chain.latency_frames());
    let session = MicFeedSession::start(&input, chain)
        .context("starting the mic feed (grant microphone permission if macOS asked)")?;
    println!(
        "Feeding \"{}\" into \"StealSeries Microphone\": {}. Chain latency {latency_ms:.1} ms. Ctrl-C to stop.",
        input.name,
        describe(&settings)
    );
    if args.probe {
        let layout = session.stream_layout()?;
        println!(
            "probe: aggregate input  channels per buffer = {:?}",
            layout.inputs
        );
        println!(
            "probe: aggregate output channels per buffer = {:?}, feed = #{}",
            layout.outputs,
            session.feed_buffer_index()
        );
    }
    let mut polls = 0_u32;
    let mut silence = SilenceWatch::new(SILENT_MIC_HINT);
    wait_for_ctrl_c(|| {
        polls += 1;
        if polls % POLLS_PER_PROBE_REPORT == 0 {
            let snapshot = session.probe();
            // The last probed slot is the feed; the ones before it are the mic buffers.
            let mics = snapshot
                .input_buffers
                .min(MAX_PROBED_BUFFERS)
                .saturating_sub(1);
            silence.observe(snapshot.peaks.get(..mics).unwrap_or_default());
            if args.probe {
                print_mic_probe(&snapshot);
            }
        }
    })?;
    drop(session);
    println!("Stopped.");
    Ok(())
}

/// Sample rate of the virtual microphone and the mic chain (`virtual-mic-driver.md` §3).
const MIC_SAMPLE_RATE: f32 = 48_000.0;

fn ms(frames: usize) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let frames = frames as f32;
    frames * 1000.0 / MIC_SAMPLE_RATE
}

fn mic_chain_settings(args: &MicArgs) -> Result<MicChainSettings> {
    let gate = match (args.gate, args.auto_gate) {
        (Some(threshold_db), _) => Some(GateMode::Manual { threshold_db }),
        (None, Some(offset_db)) => Some(GateMode::Automatic { offset_db }),
        (None, None) => None,
    };
    let mut eq = ParametricEqSettings::from_band_specs(&args.bands)?;
    eq.enabled = !args.bands.is_empty();
    Ok(MicChainSettings {
        denoise: args.denoise.map(|mix| mix.clamp(0.0, 1.0)),
        gate,
        stabilizer: args.stabilizer.then(voice_stabilizer),
        eq,
        gain_db: args.gain_db,
    })
}

fn describe(settings: &MicChainSettings) -> String {
    let mut parts = Vec::new();
    if let Some(mix) = settings.denoise {
        parts.push(format!("denoise {mix:.2}"));
    }
    match settings.gate {
        Some(GateMode::Manual { threshold_db }) => {
            parts.push(format!("gate {threshold_db:.0} dBFS"));
        }
        Some(GateMode::Automatic { offset_db }) => {
            parts.push(format!("auto gate +{offset_db:.0} dB"));
        }
        None => {}
    }
    if settings.stabilizer.is_some() {
        parts.push("stabilizer".to_owned());
    }
    if settings.eq.enabled {
        parts.push("mic EQ".to_owned());
    }
    parts.push(format!("gain {:+.1} dB", settings.gain_db));
    parts.join(", ")
}

fn print_mic_probe(snapshot: &ProbeSnapshot) {
    let shown = snapshot.input_buffers.min(MAX_PROBED_BUFFERS);
    let Some(feed) = shown.checked_sub(1) else {
        return;
    };
    let mics: Vec<String> = (0..feed)
        .map(|i| format!("mic#{i} {:>6.1}", to_dbfs(snapshot.peaks[i])))
        .collect();
    println!(
        "probe: cb={} frames={} | dBFS: {}  feed {:>6.1}",
        snapshot.callbacks,
        snapshot.frames,
        mics.join("  "),
        to_dbfs(snapshot.peaks[feed])
    );
}

fn select_input(filter: Option<&str>) -> Result<InputDevice> {
    let Some(filter) = filter else {
        return Ok(default_input_device()?);
    };
    let needle = filter.to_lowercase();
    list_input_devices()?
        .into_iter()
        .find(|d| d.name.to_lowercase().contains(&needle))
        .with_context(|| format!("no input device matching \"{filter}\""))
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
    let mut silence = SilenceWatch::new(SILENT_TAPS_HINT);
    let skipped = session.stream_layout()?.device_inputs;
    wait_for_ctrl_c(|| {
        polls += 1;
        if polls % POLLS_PER_PROBE_REPORT == 0 {
            let snapshot = session.probe();
            let shown = snapshot.input_buffers.min(MAX_PROBED_BUFFERS);
            silence.observe(snapshot.peaks.get(skipped..shown).unwrap_or_default());
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

/// Hint printed when all taps deliver exact zeros: macOS does that (and still mutes the tapped
/// apps) when the terminal lacks "System Audio Recording" permission
/// (`docs/hardware/macos-audiomix.md`, Prerequisites).
const SILENT_TAPS_HINT: &str = "all taps deliver digital silence. If apps are playing, allow \
    this terminal under System Settings → Privacy & Security → Screen & System Audio Recording \
    → System Audio Recording Only, then restart the terminal.";
/// Hint printed when the source mic delivers exact zeros: macOS does that when the terminal's
/// microphone access is denied (`docs/hardware/macos-virtual-mic.md`).
const SILENT_MIC_HINT: &str = "the microphone delivers digital silence. Allow this terminal \
    under System Settings → Privacy & Security → Microphone, then restart the terminal.";

/// Warns once when the watched buffers stay at exact digital zero, which real microphones and
/// playing apps never produce, but a denied permission does.
#[derive(Debug)]
struct SilenceWatch {
    hint: &'static str,
    silent_seconds: u32,
    warned: bool,
}

impl SilenceWatch {
    const fn new(hint: &'static str) -> Self {
        Self {
            hint,
            silent_seconds: 0,
            warned: false,
        }
    }

    /// Call once per second with the peaks of the watched buffers.
    fn observe(&mut self, peaks: &[f32]) {
        if peaks.is_empty() || peaks.iter().any(|p| p.to_bits() != 0) {
            self.silent_seconds = 0;
            return;
        }
        self.silent_seconds += 1;
        if !self.warned && self.silent_seconds >= SILENCE_WARNING_SECONDS {
            self.warned = true;
            eprintln!("warning: {}", self.hint);
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

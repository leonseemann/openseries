//! Windows implementation of the spike (see `main.rs`).

// Spike / FFI code: `unsafe` is needed for raw COM and Toolhelp calls. Every block has a SAFETY
// note. Not production code.
#![allow(unsafe_code)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use wasapi::{AudioClient, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::{
    AudioSessionStateActive, IAudioSessionControl2, IAudioSessionManager2, ISimpleAudioVolume,
};
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::core::Interface;

type Res<T> = Result<T, Box<dyn Error>>;

/// Capture format requested from process loopback (it has no mix format of its own).
const SAMPLE_RATE: usize = 48_000;
const CHANNELS: usize = 2;
const BYTES_PER_SAMPLE: usize = 4;
/// Wait for a capture event at most this long before reporting a stall.
const EVENT_TIMEOUT_MS: u32 = 500;

pub fn run(args: &[String]) -> Res<()> {
    wasapi::initialize_mta().ok()?;
    match args.first().map(String::as_str) {
        Some("endpoints") => endpoints(),
        Some("sessions") => sessions(),
        Some("policy") => policy(),
        Some("policy-roundtrip") => {
            let exe = args
                .get(1)
                .ok_or("usage: winspike policy-roundtrip <exe name>")?;
            policy_roundtrip(exe)
        }
        Some("capture") => capture(&CaptureArgs::parse(&args[1..])?),
        _ => {
            eprintln!(
                "usage: winspike endpoints | sessions | capture (--pid N | --name exe) \
                 [--silence none|mute|volume|half] [--seconds 30] [--exclude] [--wav file]"
            );
            Ok(())
        }
    }
}

// ---------------------------------------------------------------- endpoints

fn endpoints() -> Res<()> {
    let enumerator = DeviceEnumerator::new()?;
    for direction in [Direction::Render, Direction::Capture] {
        let default_id = enumerator
            .get_default_device(direction)
            .and_then(|d| d.get_id())
            .unwrap_or_default();
        println!("{direction:?} endpoints:");
        let collection = enumerator.get_device_collection(direction)?;
        for index in 0..collection.get_nbr_devices()? {
            let device = collection.get_device_at_index(index)?;
            let id = device.get_id()?;
            let format = device.get_device_format()?;
            println!(
                "  {} {:<45} {} ch {:>6} Hz  {}",
                if id == default_id { "*" } else { " " },
                device.get_friendlyname()?,
                format.get_nchannels(),
                format.get_samplespersec(),
                id
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- sessions

/// One audio session on one render endpoint.
struct Session {
    endpoint: String,
    pid: u32,
    active: bool,
    volume: ISimpleAudioVolume,
    meter: IAudioMeterInformation,
}

fn all_sessions() -> Res<Vec<Session>> {
    let enumerator = DeviceEnumerator::new()?;
    let collection = enumerator.get_device_collection(Direction::Render)?;
    let mut sessions = Vec::new();
    for index in 0..collection.get_nbr_devices()? {
        let device = collection.get_device_at_index(index)?;
        let endpoint = device.get_friendlyname()?;
        // SAFETY: COM calls on valid interfaces; COM is initialised (MTA) in `run`.
        unsafe {
            let manager: IAudioSessionManager2 =
                device.as_immdevice().Activate(CLSCTX_ALL, None)?;
            let list = manager.GetSessionEnumerator()?;
            for i in 0..list.GetCount()? {
                let control = list.GetSession(i)?;
                let control2: IAudioSessionControl2 = control.cast()?;
                sessions.push(Session {
                    endpoint: endpoint.clone(),
                    pid: control2.GetProcessId()?,
                    active: control.GetState()? == AudioSessionStateActive,
                    volume: control.cast()?,
                    meter: control.cast()?,
                });
            }
        }
    }
    Ok(sessions)
}

fn sessions() -> Res<()> {
    let names = process_names()?;
    println!(
        "{:>7}  {:<28} {:<7} {:>6} {:<5} {:>8}  endpoint",
        "pid", "exe", "state", "vol", "mute", "peak"
    );
    for s in all_sessions()? {
        // SAFETY: COM getters on valid interfaces.
        let (volume, mute, peak) = unsafe {
            (
                s.volume.GetMasterVolume()?,
                s.volume.GetMute()?.as_bool(),
                s.meter.GetPeakValue()?,
            )
        };
        println!(
            "{:>7}  {:<28} {:<7} {:>5.0}% {:<5} {:>8}  {}",
            s.pid,
            names.get(&s.pid).map_or("?", String::as_str),
            if s.active { "active" } else { "idle" },
            volume * 100.0,
            mute,
            dbfs(peak),
            s.endpoint
        );
    }
    Ok(())
}

// ---------------------------------------------------------------- per-app routing (read-only)

/// Reads (never writes) the per-app default render endpoint of every audio session's process.
fn policy() -> Res<()> {
    let policy = ss_audio_windows::policy::AudioPolicy::open()?;
    println!("AudioPolicyConfig: {}", policy.variant());
    let names = process_names()?;
    let mut seen = HashSet::new();
    for session in all_sessions()? {
        if session.pid == 0 || !seen.insert(session.pid) {
            continue;
        }
        let endpoint = policy.render_endpoint(session.pid)?;
        println!(
            "{:>7}  {:<28} {}",
            session.pid,
            names.get(&session.pid).map_or("?", String::as_str),
            endpoint
                .as_deref()
                .unwrap_or("(follows the system default)")
        );
    }
    Ok(())
}

/// Verifies `Set…` (slot 25) reversibly: routes the app to the render endpoint it already uses
/// (the system default), reads it back, then restores the previous value and reads that back.
fn policy_roundtrip(exe: &str) -> Res<()> {
    let policy = ss_audio_windows::policy::AudioPolicy::open()?;
    let pid = root_pid(exe)?;
    let default = DeviceEnumerator::new()?
        .get_default_device(Direction::Render)?
        .get_id()?;
    let before = policy.render_endpoint(pid)?;
    println!("pid {pid} ({exe}), variant {}", policy.variant());
    println!("before:   {before:?}");
    println!("set to:   {default} (the current system default, so nothing audible changes)");
    let result = policy
        .set_render_endpoint(pid, Some(&default))
        .and_then(|()| policy.render_endpoint(pid));
    // Always restore, even if the read-back failed.
    let restored = policy.set_render_endpoint(pid, before.as_deref());
    let after = policy.render_endpoint(pid)?;
    let read_back = result?;
    restored?;
    println!("read back: {read_back:?}");
    println!("restored: {after:?}");
    let ok = read_back.as_deref() == Some(default.as_str()) && after == before;
    println!(
        "{}",
        if ok {
            "ROUND TRIP OK"
        } else {
            "ROUND TRIP MISMATCH"
        }
    );
    Ok(())
}

// ---------------------------------------------------------------- processes

/// pid → (parent pid, exe name) from a Toolhelp snapshot.
fn process_table() -> Res<HashMap<u32, (u32, String)>> {
    let mut table = HashMap::new();
    // SAFETY: Toolhelp snapshot API; `entry.dwSize` is set as required and the handle is closed.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        let mut entry = PROCESSENTRY32W {
            dwSize: u32::try_from(size_of::<PROCESSENTRY32W>())?,
            ..Default::default()
        };
        let mut more = Process32FirstW(snapshot, &raw mut entry).is_ok();
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            table.insert(entry.th32ProcessID, (entry.th32ParentProcessID, name));
            more = Process32NextW(snapshot, &raw mut entry).is_ok();
        }
        CloseHandle(snapshot)?;
    }
    Ok(table)
}

fn process_names() -> Res<HashMap<u32, String>> {
    Ok(process_table()?
        .into_iter()
        .map(|(pid, (_, name))| (pid, name))
        .collect())
}

/// The root process of `exe` (one whose parent has a different name), e.g. the Chrome browser
/// process rather than its renderer/audio children.
fn root_pid(exe: &str) -> Res<u32> {
    let table = process_table()?;
    table
        .iter()
        .filter(|(_, (_, name))| name.eq_ignore_ascii_case(exe))
        .find(|(_, (parent, _))| {
            table
                .get(parent)
                .is_none_or(|(_, parent_name)| !parent_name.eq_ignore_ascii_case(exe))
        })
        .map(|(pid, _)| *pid)
        .ok_or_else(|| format!("no process named {exe}").into())
}

/// `root` and all its descendants.
fn process_tree(root: u32) -> Res<HashSet<u32>> {
    let table = process_table()?;
    let mut tree = HashSet::from([root]);
    loop {
        let before = tree.len();
        for (pid, (parent, _)) in &table {
            if tree.contains(parent) {
                tree.insert(*pid);
            }
        }
        if tree.len() == before {
            return Ok(tree);
        }
    }
}

// ---------------------------------------------------------------- capture

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Silence {
    None,
    Mute,
    Volume,
    /// Session volume 0.5: checks whether session volume is a linear factor (≈ −6 dB).
    Half,
}

struct CaptureArgs {
    pid: u32,
    silence: Silence,
    seconds: u64,
    exclude: bool,
    wav: Option<String>,
}

impl CaptureArgs {
    fn parse(args: &[String]) -> Res<Self> {
        let mut pid = None;
        let mut parsed = Self {
            pid: 0,
            silence: Silence::Mute,
            seconds: 30,
            exclude: false,
            wav: None,
        };
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            let mut value = || it.next().ok_or_else(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--pid" => pid = Some(value()?.parse()?),
                "--name" => pid = Some(root_pid(value()?)?),
                "--seconds" => parsed.seconds = value()?.parse()?,
                "--exclude" => parsed.exclude = true,
                "--wav" => parsed.wav = Some(value()?.clone()),
                "--silence" => {
                    parsed.silence = match value()?.as_str() {
                        "none" => Silence::None,
                        "mute" => Silence::Mute,
                        "volume" => Silence::Volume,
                        "half" => Silence::Half,
                        other => return Err(format!("unknown --silence {other}").into()),
                    }
                }
                other => return Err(format!("unknown argument {other}").into()),
            }
        }
        parsed.pid = pid.ok_or("--pid or --name is required")?;
        Ok(parsed)
    }
}

/// Saved session state so it can be restored exactly.
struct Saved {
    volume: ISimpleAudioVolume,
    level: f32,
    mute: bool,
}

/// Sessions of the target process tree.
fn target_sessions(tree: &HashSet<u32>) -> Res<Vec<Session>> {
    Ok(all_sessions()?
        .into_iter()
        .filter(|s| tree.contains(&s.pid))
        .collect())
}

fn silence(sessions: &[Session], how: Silence) -> Res<Vec<Saved>> {
    let mut saved = Vec::new();
    for s in sessions {
        // SAFETY: COM calls on valid interfaces; null event context is allowed.
        unsafe {
            saved.push(Saved {
                volume: s.volume.clone(),
                level: s.volume.GetMasterVolume()?,
                mute: s.volume.GetMute()?.as_bool(),
            });
            match how {
                Silence::None => {}
                Silence::Mute => s.volume.SetMute(true, std::ptr::null())?,
                Silence::Volume => s.volume.SetMasterVolume(0.0, std::ptr::null())?,
                Silence::Half => s.volume.SetMasterVolume(0.5, std::ptr::null())?,
            }
        }
    }
    Ok(saved)
}

fn restore(saved: &[Saved]) {
    for s in saved {
        // SAFETY: as in `silence`. Errors are ignored: the session may be gone.
        unsafe {
            let _ = s.volume.SetMasterVolume(s.level, std::ptr::null());
            let _ = s.volume.SetMute(s.mute, std::ptr::null());
        }
    }
}

/// Per-second statistics.
#[derive(Default)]
struct Window {
    peak: f32,
    frames: usize,
    packets: usize,
    discontinuities: usize,
    silent_flags: usize,
    stalls: usize,
    session_peak: f32,
}

#[allow(clippy::too_many_lines)] // linear measurement script
fn capture(args: &CaptureArgs) -> Res<()> {
    let names = process_names()?;
    let tree = process_tree(args.pid)?;
    let exe = names.get(&args.pid).map_or("?", String::as_str);
    println!(
        "target pid {} ({exe}), tree {} processes, mode {}, silence {:?}",
        args.pid,
        tree.len(),
        if args.exclude {
            "EXCLUDE tree"
        } else {
            "INCLUDE tree"
        },
        args.silence
    );
    let sessions = target_sessions(&tree)?;
    for s in &sessions {
        println!(
            "  session pid {} ({}) on {}{}",
            s.pid,
            names.get(&s.pid).map_or("?", String::as_str),
            s.endpoint,
            if s.active { ", active" } else { "" }
        );
    }
    if sessions.is_empty() && args.silence != Silence::None {
        println!("  warning: no session yet; start playback first so it can be silenced");
    }

    let format = WaveFormat::new(32, 32, SampleType::Float, SAMPLE_RATE, CHANNELS, None);
    let mut client = AudioClient::new_application_loopback_client(args.pid, !args.exclude)?;
    client.initialize_client(
        &format,
        Direction::Capture,
        StreamMode::EventsShared {
            autoconvert: true,
            buffer_duration_hns: 0,
        },
        None,
    )?;
    let event = client.set_get_eventhandle()?;
    let capture = client.get_audiocaptureclient()?;
    println!(
        "buffer {} frames ({:.1} ms)",
        client.get_buffer_size().unwrap_or(0),
        f64::from(client.get_buffer_size().unwrap_or(0)) * 1000.0
            / f64::from(u32::try_from(SAMPLE_RATE).unwrap_or(1))
    );

    let running = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&running);
    ctrlc::set_handler(move || flag.store(false, Ordering::SeqCst))?;

    let mut writer = match &args.wav {
        Some(path) => Some(hound::WavWriter::create(
            path,
            hound::WavSpec {
                channels: u16::try_from(CHANNELS)?,
                sample_rate: u32::try_from(SAMPLE_RATE)?,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )?),
        None => None,
    };

    let phase_len = Duration::from_secs(args.seconds / 3).max(Duration::from_secs(1));
    let start = Instant::now();
    let mut phase = 0;
    let mut saved: Vec<Saved> = Vec::new();
    let mut queue: VecDeque<u8> = VecDeque::new();
    let mut window = Window::default();
    let mut window_start = Instant::now();
    let mut second = 0;
    client.start_stream()?;
    println!("phase 1/3: normal playback");
    println!(
        "{:>4}  {:<9} {:>9} {:>9} {:>7} {:>5} {:>5} {:>6} {:>6}",
        "t", "phase", "loopback", "session", "frames", "pkts", "disc", "silent", "stall"
    );

    let result: Res<()> = (|| {
        while running.load(Ordering::SeqCst) && start.elapsed() < phase_len * 3 {
            let wanted = usize::try_from(start.elapsed().as_nanos() / phase_len.as_nanos())?;
            if wanted != phase && wanted < 3 {
                phase = wanted;
                if phase == 1 {
                    // Re-query: sessions may have appeared after start.
                    saved = silence(&target_sessions(&tree)?, args.silence)?;
                    println!(
                        "phase 2/3: session(s) silenced ({:?}, {} sessions) — should be INAUDIBLE now",
                        args.silence,
                        saved.len()
                    );
                } else {
                    restore(&saved);
                    saved.clear();
                    println!("phase 3/3: restored — audible again");
                }
            }

            if event.wait_for_event(EVENT_TIMEOUT_MS).is_err() {
                window.stalls += 1;
            }
            while let Some(frames) = capture.get_next_packet_size()? {
                if frames == 0 {
                    break;
                }
                let info = capture.read_from_device_to_deque(&mut queue)?;
                window.packets += 1;
                window.discontinuities += usize::from(info.flags.data_discontinuity);
                window.silent_flags += usize::from(info.flags.silent);
            }
            let samples = queue.len() / BYTES_PER_SAMPLE;
            let bytes: Vec<u8> = queue.drain(..samples * BYTES_PER_SAMPLE).collect();
            for chunk in bytes.chunks_exact(BYTES_PER_SAMPLE) {
                let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                window.peak = window.peak.max(sample.abs());
                if let Some(w) = writer.as_mut() {
                    w.write_sample(sample)?;
                }
            }
            window.frames += samples / CHANNELS;
            for s in &sessions {
                // SAFETY: COM getter on a valid interface.
                if let Ok(peak) = unsafe { s.meter.GetPeakValue() } {
                    window.session_peak = window.session_peak.max(peak);
                }
            }

            if window_start.elapsed() >= Duration::from_secs(1) {
                second += 1;
                println!(
                    "{second:>4}  {:<9} {:>9} {:>9} {:>7} {:>5} {:>5} {:>6} {:>6}",
                    ["normal", "SILENCED", "restored"][phase],
                    dbfs(window.peak),
                    dbfs(window.session_peak),
                    window.frames,
                    window.packets,
                    window.discontinuities,
                    window.silent_flags,
                    window.stalls
                );
                window = Window::default();
                window_start = Instant::now();
            }
        }
        Ok(())
    })();

    restore(&saved);
    let _ = client.stop_stream();
    if let Some(w) = writer {
        w.finalize()?;
    }
    println!("done; session state restored");
    result
}

fn dbfs(peak: f32) -> String {
    if peak <= 0.0 {
        "-inf".to_owned()
    } else {
        format!("{:.1}", 20.0 * peak.log10())
    }
}

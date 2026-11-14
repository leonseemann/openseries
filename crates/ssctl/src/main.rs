//! `ssctl` — command-line control for the Arctis Nova Pro Wireless.

mod cli;
mod output;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use ss_core::headset::{
    ChatMixBalance, DisplayBrightness, EqGain, EqPreset, HARDWARE_EQ_BANDS, HardwareEq,
    MicLedBrightness, MicVolume, Percent, TransparencyLevel,
};
use ss_hid::{DeviceSelector, HidapiTransport};
use ss_nova_pro::{Command, NovaProWireless, ids};

use crate::cli::{Action, Cli, RawBlock, Setting};

/// How long `watch` blocks per read before checking for Ctrl-C.
const WATCH_POLL: Duration = Duration::from_millis(200);

const NOVA_PRO: DeviceSelector<'static> = DeviceSelector {
    vendor_id: ids::VENDOR_ID,
    product_ids: &ids::PRODUCT_IDS,
    interface: ids::CONTROL_INTERFACE,
    command_usage_page: ids::COMMAND_USAGE_PAGE,
    event_usage_page: ids::EVENT_USAGE_PAGE,
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.action {
        Action::List => list(),
        Action::Status => {
            let mut device = open()?;
            output::print_status(&device.status()?);
            output::print_settings(&device.settings()?);
            output::print_display(device.display()?);
            Ok(())
        }
        Action::Battery => {
            let report = open()?.battery()?;
            output::print_battery(report);
            Ok(())
        }
        Action::Set { setting, save } => set(setting, save),
        Action::Raw { block } => {
            let reply = open()?.query_raw(query_for(block))?;
            output::print_hex(&reply);
            Ok(())
        }
        Action::Watch { chatmix_dial } => watch(chatmix_dial),
    }
}

fn list() -> Result<()> {
    let api = ss_hid::api()?;
    let found = ss_hid::find_collections(&api, &NOVA_PRO);
    if found.is_empty() {
        println!("No Arctis Nova Pro Wireless base station found.");
    }
    for c in found {
        println!(
            "PID {:#06x}  usage page {:#06x}  {}  {}",
            c.product_id,
            c.usage_page,
            c.product.as_deref().unwrap_or("?"),
            c.path
        );
    }
    Ok(())
}

fn open() -> Result<NovaProWireless<HidapiTransport>> {
    let api = ss_hid::api()?;
    let transport = HidapiTransport::open(&api, &NOVA_PRO)
        .context("could not open the Arctis Nova Pro Wireless base station")?;
    Ok(NovaProWireless::new(transport))
}

/// Maps a raw block to its (read-only) query command.
fn query_for(block: RawBlock) -> Command {
    match block {
        RawBlock::Status => Command::QueryStatus,
        RawBlock::Settings => Command::QuerySettings,
        RawBlock::Display => Command::QueryDisplay,
        RawBlock::Battery => Command::QueryBattery,
    }
}

fn set(setting: Setting, save: bool) -> Result<()> {
    let commands = commands_for(setting)?;
    let mut device = open()?;
    for command in commands {
        device.send(command)?;
    }
    if save {
        device.send(Command::Save)?;
    }
    println!("ok");
    Ok(())
}

/// Translates a CLI setting into the device commands that apply it.
fn commands_for(setting: Setting) -> Result<Vec<Command>> {
    Ok(match setting {
        Setting::NoiseControl { mode } => vec![Command::SetNoiseControl(mode.into())],
        Setting::Transparency { level } => {
            vec![Command::SetTransparency(TransparencyLevel::new(level)?)]
        }
        Setting::Sidetone { level } => vec![Command::SetSidetone(level.into())],
        Setting::MicVolume { level } => vec![Command::SetMicVolume(MicVolume::new(level)?)],
        Setting::EqPreset { preset } => vec![Command::SetEqPreset(preset.into())],
        Setting::Eq { gains } => vec![
            Command::SetCustomEq(hardware_eq(&gains)?),
            Command::SetEqPreset(EqPreset::Custom),
        ],
        Setting::AutoOff { timeout } => vec![Command::SetAutoOff(timeout.into())],
        Setting::Chatmix { game, chat } => vec![Command::SetChatMix(ChatMixBalance {
            game: Percent::new(game)?,
            chat: Percent::new(chat)?,
        })],
        Setting::OledBrightness { level } => {
            vec![Command::SetDisplayBrightness(DisplayBrightness::new(
                level,
            )?)]
        }
        Setting::MicLed { level } => {
            vec![Command::SetMicLedBrightness(MicLedBrightness::new(level)?)]
        }
    })
}

fn hardware_eq(gains: &[f32]) -> Result<HardwareEq> {
    anyhow::ensure!(
        gains.len() == HARDWARE_EQ_BANDS,
        "expected {HARDWARE_EQ_BANDS} gains, got {}",
        gains.len()
    );
    let mut eq = HardwareEq::FLAT;
    for (band, &db) in eq.bands.iter_mut().zip(gains) {
        *band = EqGain::from_db(db)?;
    }
    Ok(eq)
}

fn watch(chatmix_dial: bool) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&running);
    ctrlc::set_handler(move || flag.store(false, Ordering::SeqCst))?;

    let mut device = open()?;
    if chatmix_dial {
        device.send(Command::SetChatMixDial(true))?;
    }
    println!("Watching events, press Ctrl-C to stop…");
    let result = (|| -> Result<()> {
        while running.load(Ordering::SeqCst) {
            if let Some(event) = device.next_event(WATCH_POLL)? {
                output::print_event(&event);
            }
        }
        Ok(())
    })();
    if chatmix_dial {
        // Always hand mixing back to the base station, even after an error.
        device.send(Command::SetChatMixDial(false))?;
    }
    result
}

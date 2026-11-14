//! Forwards Nova Pro ChatMix dial events to the engine on a background thread.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{Context, Result};
use ss_engine::{Command as EngineCommand, EngineController};
use ss_hid::{DeviceSelector, HidapiTransport};
use ss_nova_pro::{Command, Event, NovaProWireless, ids};

/// How long one read waits before checking the stop flag.
const POLL: Duration = Duration::from_millis(100);

const NOVA_PRO: DeviceSelector<'static> = DeviceSelector {
    vendor_id: ids::VENDOR_ID,
    product_ids: &ids::PRODUCT_IDS,
    interface: ids::CONTROL_INTERFACE,
    command_usage_page: ids::COMMAND_USAGE_PAGE,
    event_usage_page: ids::EVENT_USAGE_PAGE,
};

/// Running dial forwarder; restores the base station's own mixing when stopped.
pub struct DialForwarder {
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<()>>>,
}

impl DialForwarder {
    /// Opens the headset, hands ChatMix over to the host and starts forwarding.
    ///
    /// # Errors
    /// Fails if the base station cannot be opened or configured.
    pub fn start(mut controller: EngineController) -> Result<Self> {
        let api = ss_hid::api()?;
        let transport = HidapiTransport::open(&api, &NOVA_PRO)
            .context("Arctis Nova Pro Wireless base station not found")?;
        let mut device = NovaProWireless::new(transport);
        device.send(Command::SetChatMixDial(true))?;
        device.send(Command::SetMixerPresent(true))?;
        if let Some(balance) = device.settings()?.chatmix {
            let _ = controller.send(EngineCommand::SetChatMix(balance));
        }

        let running = Arc::new(AtomicBool::new(true));
        let flag = Arc::clone(&running);
        let thread = std::thread::spawn(move || -> Result<()> {
            let result = forward(&mut device, &mut controller, &flag);
            // Always give mixing back to the base station.
            device.send(Command::SetChatMixDial(false))?;
            device.send(Command::SetMixerPresent(false))?;
            result
        });
        Ok(Self {
            running,
            thread: Some(thread),
        })
    }

    /// Stops forwarding and restores the base station.
    ///
    /// # Errors
    /// Returns the forwarder thread's error, if any.
    pub fn stop(mut self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        match self.thread.take().map(JoinHandle::join) {
            Some(Ok(result)) => result,
            Some(Err(_)) => anyhow::bail!("dial thread panicked"),
            None => Ok(()),
        }
    }
}

fn forward(
    device: &mut NovaProWireless<HidapiTransport>,
    controller: &mut EngineController,
    running: &AtomicBool,
) -> Result<()> {
    while running.load(Ordering::SeqCst) {
        if let Some(Event::ChatMix(balance)) = device.next_event(POLL)? {
            println!(
                "ChatMix game {:>3} %  chat {:>3} %",
                balance.game.get(),
                balance.chat.get()
            );
            // A full queue only drops this update; the next dial event corrects it.
            let _ = controller.send(EngineCommand::SetChatMix(balance));
        }
    }
    Ok(())
}

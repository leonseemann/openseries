//! The audio-control thread. CoreAudio sessions hold non-`Send` blocks, so this one thread
//! creates, updates and drops them; everyone else talks to it through [`AudioMsg`].

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use ss_core::headset::ChatMixBalance;
use ss_store::Config;

use super::supervisor::{AudioBackend, AudioStatus, AudioSupervisor};
use crate::notify::Notifier;

/// How often running apps and devices are re-scanned.
const RESCAN_INTERVAL: Duration = Duration::from_secs(2);

/// Messages to the audio thread.
#[derive(Debug, Clone)]
pub enum AudioMsg {
    /// The configuration changed.
    Config(Box<Config>),
    /// The ChatMix dial moved.
    ChatMix(ChatMixBalance),
    /// The headset was turned on (`true`) or off.
    HeadsetLinked(bool),
    /// A supported device is connected (`true`) or none is (`app.md` §8).
    DevicePresent(bool),
    /// Stop everything and end the thread.
    Shutdown,
}

/// Starts the audio thread; `make_backend` runs on that thread.
pub fn spawn<B, F>(
    rx: Receiver<AudioMsg>,
    config: Config,
    status: Arc<Mutex<AudioStatus>>,
    notify: Notifier,
    make_backend: F,
) -> std::io::Result<JoinHandle<()>>
where
    B: AudioBackend,
    F: FnOnce() -> B + Send + 'static,
{
    std::thread::Builder::new()
        .name("audio-control".to_owned())
        .spawn(move || {
            let mut supervisor = AudioSupervisor::new(make_backend(), Config::default());
            supervisor.on_tick();
            supervisor.on_config(config);
            publish(&supervisor, &status, &notify);
            loop {
                match rx.recv_timeout(RESCAN_INTERVAL) {
                    Ok(AudioMsg::Config(config)) => supervisor.on_config(*config),
                    Ok(AudioMsg::ChatMix(balance)) => supervisor.on_chatmix(balance),
                    Ok(AudioMsg::HeadsetLinked(linked)) => supervisor.on_headset_link(linked),
                    Ok(AudioMsg::DevicePresent(present)) => supervisor.on_device_present(present),
                    Ok(AudioMsg::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                        supervisor.shutdown();
                        publish(&supervisor, &status, &notify);
                        return;
                    }
                    Err(RecvTimeoutError::Timeout) => supervisor.on_tick(),
                }
                publish(&supervisor, &status, &notify);
            }
        })
}

fn publish<B: AudioBackend>(
    supervisor: &AudioSupervisor<B>,
    status: &Mutex<AudioStatus>,
    notify: &Notifier,
) {
    let new = supervisor.status();
    if let Ok(mut current) = status.lock() {
        if *current == *new {
            return;
        }
        current.clone_from(new);
    }
    notify();
}

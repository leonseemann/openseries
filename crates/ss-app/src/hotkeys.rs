//! Global hotkeys (SON-30 subset): ChatMix steps and virtual-mic mute.
//!
//! Registered once at start from the config; the handler acts on [`AppCore`] directly so it
//! works while the window is hidden.

use std::collections::HashMap;
use std::sync::Arc;

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use ss_store::config::HotkeysConfig;

use crate::core::AppCore;

/// One ChatMix step of a hotkey press (the slider runs from −1 = game to +1 = chat).
const CHATMIX_STEP: f32 = 0.1;

#[derive(Debug, Clone, Copy)]
enum Action {
    ChatMix(f32),
    ToggleMic,
}

/// Keeps the hotkeys registered while alive.
pub struct Hotkeys {
    _manager: GlobalHotKeyManager,
}

impl Hotkeys {
    /// Registers the configured hotkeys. Returns the registration plus a message per hotkey that
    /// could not be parsed or registered.
    ///
    /// # Errors
    /// If the OS hotkey service is unavailable.
    pub fn register(
        config: &HotkeysConfig,
        core: Arc<AppCore>,
    ) -> Result<(Self, Vec<String>), global_hotkey::Error> {
        let manager = GlobalHotKeyManager::new()?;
        let mut actions = HashMap::new();
        let mut problems = Vec::new();
        for (binding, action) in [
            (&config.chatmix_game, Action::ChatMix(-CHATMIX_STEP)),
            (&config.chatmix_chat, Action::ChatMix(CHATMIX_STEP)),
            (&config.mic_mute, Action::ToggleMic),
        ] {
            let Some(text) = binding else {
                continue;
            };
            match text.parse::<HotKey>() {
                Ok(hotkey) => match manager.register(hotkey) {
                    Ok(()) => {
                        actions.insert(hotkey.id(), action);
                    }
                    Err(error) => problems.push(format!("hotkey {text}: {error}")),
                },
                Err(error) => problems.push(format!("hotkey {text}: {error}")),
            }
        }
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state != HotKeyState::Pressed {
                return;
            }
            match actions.get(&event.id) {
                Some(Action::ChatMix(step)) => core.nudge_chatmix(*step),
                Some(Action::ToggleMic) => {
                    core.update_config(|c| c.mic.enabled = !c.mic.enabled);
                }
                None => {}
            }
        }));
        Ok((Self { _manager: manager }, problems))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_parse() {
        let config = HotkeysConfig::default();
        for binding in [config.chatmix_game, config.chatmix_chat, config.mic_mute]
            .into_iter()
            .flatten()
        {
            assert!(binding.parse::<HotKey>().is_ok(), "{binding}");
        }
    }
}

//! Global hotkeys (SON-30 subset): ChatMix steps, virtual-mic mute and USB reconnect.
//!
//! Registered from the config and re-registered whenever the hotkey editor on the Settings page
//! changes it (`docs/architecture/app.md` §2). The handler acts on [`AppCore`] directly so it
//! works while the window is hidden.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use eframe::egui;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use ss_store::config::HotkeysConfig;

use crate::core::AppCore;
use crate::i18n::tr;

/// One ChatMix step of a hotkey press (the slider runs from −1 = game to +1 = chat).
const CHATMIX_STEP: f32 = 0.1;

/// What a hotkey does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    /// One ChatMix step towards Game.
    ChatMixGame,
    /// One ChatMix step towards Chat.
    ChatMixChat,
    /// Toggle the virtual microphone.
    MicMute,
    /// Reconnect the base station's USB connection.
    ReconnectUsb,
}

impl HotkeyAction {
    /// Every action, in display order.
    pub const ALL: [Self; 4] = [
        Self::ChatMixGame,
        Self::ChatMixChat,
        Self::MicMute,
        Self::ReconnectUsb,
    ];

    /// Label in the editor.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::ChatMixGame => tr!("hotkey-chatmix-game"),
            Self::ChatMixChat => tr!("hotkey-chatmix-chat"),
            Self::MicMute => tr!("hotkey-mic-mute"),
            Self::ReconnectUsb => tr!("hotkey-reconnect-usb"),
        }
    }

    /// Whether the action can work on this OS.
    #[must_use]
    pub const fn available(self) -> bool {
        match self {
            Self::ReconnectUsb => crate::platform::USB_RESET_SUPPORTED,
            Self::MicMute => crate::platform::VIRTUAL_MIC_SUPPORTED,
            Self::ChatMixGame | Self::ChatMixChat => true,
        }
    }

    /// The configured binding.
    #[must_use]
    pub fn binding(self, config: &HotkeysConfig) -> Option<&str> {
        match self {
            Self::ChatMixGame => config.chatmix_game.as_deref(),
            Self::ChatMixChat => config.chatmix_chat.as_deref(),
            Self::MicMute => config.mic_mute.as_deref(),
            Self::ReconnectUsb => config.reconnect_usb.as_deref(),
        }
    }

    /// The configured binding, for editing.
    pub fn binding_mut(self, config: &mut HotkeysConfig) -> &mut Option<String> {
        match self {
            Self::ChatMixGame => &mut config.chatmix_game,
            Self::ChatMixChat => &mut config.chatmix_chat,
            Self::MicMute => &mut config.mic_mute,
            Self::ReconnectUsb => &mut config.reconnect_usb,
        }
    }

    fn run(self, core: &AppCore) {
        match self {
            Self::ChatMixGame => core.nudge_chatmix(-CHATMIX_STEP),
            Self::ChatMixChat => core.nudge_chatmix(CHATMIX_STEP),
            Self::MicMute => core.update_config(|c| c.mic.enabled = !c.mic.enabled),
            Self::ReconnectUsb => core.reconnect_usb(None),
        }
    }
}

type ActionMap = Arc<Mutex<HashMap<u32, HotkeyAction>>>;

/// Keeps the hotkeys registered while alive.
pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    registered: Vec<HotKey>,
    actions: ActionMap,
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
        let actions: ActionMap = Arc::default();
        let handler_actions = Arc::clone(&actions);
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state != HotKeyState::Pressed {
                return;
            }
            let action = handler_actions
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&event.id)
                .copied();
            // Without a base station OpenSeries is paused and ignores its hotkeys (app.md §8).
            if let Some(action) = action.filter(|_| core.active()) {
                action.run(&core);
            }
        }));
        let mut hotkeys = Self {
            manager: GlobalHotKeyManager::new()?,
            registered: Vec::new(),
            actions,
        };
        let problems = hotkeys.apply(config);
        Ok((hotkeys, problems))
    }

    /// Replaces all registrations with `config`; returns a message per failed binding.
    pub fn apply(&mut self, config: &HotkeysConfig) -> Vec<String> {
        self.suspend();
        let mut problems = Vec::new();
        let mut actions = HashMap::new();
        for action in HotkeyAction::ALL {
            let Some(text) = action.binding(config).filter(|_| action.available()) else {
                continue;
            };
            let registered = text
                .parse::<HotKey>()
                .map_err(|_| tr!("hotkey-invalid", binding = text))
                .and_then(|hotkey| {
                    self.manager
                        .register(hotkey)
                        .map(|()| hotkey)
                        .map_err(|error| registration_problem(text, &error))
                });
            match registered {
                Ok(hotkey) => {
                    actions.insert(hotkey.id(), action);
                    self.registered.push(hotkey);
                }
                Err(problem) => problems.push(format!("{}: {problem}", action.label())),
            }
        }
        *self.actions.lock().unwrap_or_else(PoisonError::into_inner) = actions;
        problems
    }

    /// Unregisters everything (while the editor records a new combination, so the OS does not
    /// swallow it).
    pub fn suspend(&mut self) {
        for hotkey in self.registered.drain(..) {
            let _ = self.manager.unregister(hotkey);
        }
        self.actions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

/// A plain-language reason why a hotkey could not be registered.
fn registration_problem(text: &str, error: &global_hotkey::Error) -> String {
    match error {
        global_hotkey::Error::AlreadyRegistered(_) => tr!("hotkey-taken-elsewhere", binding = text),
        other => tr!(
            "hotkey-register-failed",
            binding = text,
            error = other.to_string()
        ),
    }
}

/// Turns a key press recorded in the editor into a binding string such as `"Ctrl+Alt+M"`.
///
/// # Errors
/// A user-facing message if the combination has no Ctrl/Alt/Cmd modifier (it would steal normal
/// typing) or the key cannot be used as a global hotkey.
pub fn binding_from_key(key: egui::Key, modifiers: egui::Modifiers) -> Result<String, String> {
    if !(modifiers.ctrl || modifiers.alt || modifiers.mac_cmd) {
        return Err(tr!("hotkey-needs-modifier"));
    }
    let mut parts = Vec::new();
    if modifiers.ctrl {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    if modifiers.mac_cmd {
        parts.push("Cmd");
    }
    // egui names that `global-hotkey` spells differently.
    let name = match key {
        egui::Key::Equals => "=",
        egui::Key::OpenBracket => "[",
        egui::Key::CloseBracket => "]",
        egui::Key::Backtick => "`",
        other => other.name(),
    };
    parts.push(name);
    let binding = parts.join("+");
    binding
        .parse::<HotKey>()
        .map(|_| binding.clone())
        .map_err(|_| tr!("hotkey-key-unusable", key = name))
}

/// Whether two binding strings mean the same key combination.
#[must_use]
pub fn same_binding(a: &str, b: &str) -> bool {
    match (a.parse::<HotKey>(), b.parse::<HotKey>()) {
        (Ok(a), Ok(b)) => a.id() == b.id(),
        _ => a.eq_ignore_ascii_case(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CTRL_ALT: egui::Modifiers = egui::Modifiers {
        alt: true,
        ctrl: true,
        shift: false,
        mac_cmd: false,
        command: true,
    };

    #[test]
    fn default_bindings_parse() {
        let config = HotkeysConfig::default();
        for action in HotkeyAction::ALL {
            let binding = action.binding(&config).unwrap();
            assert!(binding.parse::<HotKey>().is_ok(), "{binding}");
        }
    }

    #[test]
    fn recorded_keys_become_parseable_bindings() {
        assert_eq!(
            binding_from_key(egui::Key::M, CTRL_ALT).as_deref(),
            Ok("Ctrl+Alt+M")
        );
        assert_eq!(
            binding_from_key(egui::Key::ArrowLeft, CTRL_ALT).as_deref(),
            Ok("Ctrl+Alt+Left")
        );
        assert_eq!(
            binding_from_key(egui::Key::Equals, CTRL_ALT).as_deref(),
            Ok("Ctrl+Alt+=")
        );
        assert!(binding_from_key(egui::Key::F5, CTRL_ALT).is_ok());
    }

    #[test]
    fn plain_keys_are_rejected() {
        assert!(binding_from_key(egui::Key::M, egui::Modifiers::SHIFT).is_err());
        assert!(binding_from_key(egui::Key::M, egui::Modifiers::NONE).is_err());
    }

    #[test]
    fn bindings_compare_by_combination() {
        assert!(same_binding("Ctrl+Alt+M", "alt+ctrl+m"));
        assert!(!same_binding("Ctrl+Alt+M", "Ctrl+Alt+N"));
    }
}

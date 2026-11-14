//! Channel gains of the session mixer (`docs/architecture/audio-engine.md` §5).
//!
//! Mirrors the gain part of `ss_engine::MixEngine` (faders, master, ChatMix law) without any DSP,
//! so the same [`Command`]s drive both backends.

use ss_core::mixer::{ChannelId, Fader};
use ss_dsp::{ChatMixGains, chatmix_gains};
use ss_engine::Command;

/// Current fader, master and ChatMix state.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelGains {
    faders: [Fader; ChannelId::COUNT],
    master: Fader,
    chatmix: ChatMixGains,
}

impl Default for ChannelGains {
    fn default() -> Self {
        Self {
            faders: [Fader::UNITY; ChannelId::COUNT],
            master: Fader::UNITY,
            chatmix: ChatMixGains {
                game: 1.0,
                chat: 1.0,
            },
        }
    }
}

impl ChannelGains {
    /// Applies a command. DSP strips are ignored (they need the M4b driver).
    /// Returns `true` if a gain changed.
    pub fn apply(&mut self, command: &Command) -> bool {
        let before = self.clone();
        match command {
            Command::SetStrip(..) => {}
            Command::SetFader(channel, fader) => self.faders[channel.index()] = *fader,
            Command::SetMaster(fader) => self.master = *fader,
            Command::SetChatMix(balance) => self.chatmix = chatmix_gains(*balance),
        }
        *self != before
    }

    /// Linear gain applied to every app on `channel`, within `0.0..=1.0`.
    #[must_use]
    pub fn gain(&self, channel: ChannelId) -> f32 {
        let chatmix = match channel {
            ChannelId::Game => self.chatmix.game,
            ChannelId::Chat => self.chatmix.chat,
            ChannelId::Media | ChannelId::Aux => 1.0,
        };
        (self.faders[channel.index()].effective_gain() * self.master.effective_gain() * chatmix)
            .clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss_core::headset::{ChatMixBalance, Percent};

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn unity_by_default() {
        let gains = ChannelGains::default();
        for channel in ChannelId::ALL {
            assert!(approx(gains.gain(channel), 1.0));
        }
    }

    #[test]
    fn fader_master_and_mute_multiply() {
        let mut gains = ChannelGains::default();
        assert!(gains.apply(&Command::SetFader(
            ChannelId::Media,
            Fader {
                gain: 0.5,
                muted: false
            }
        )));
        gains.apply(&Command::SetMaster(Fader {
            gain: 0.5,
            muted: false,
        }));
        assert!(approx(gains.gain(ChannelId::Media), 0.25));
        assert!(approx(gains.gain(ChannelId::Aux), 0.5));
        gains.apply(&Command::SetMaster(Fader {
            gain: 0.5,
            muted: true,
        }));
        assert!(approx(gains.gain(ChannelId::Aux), 0.0));
    }

    #[test]
    fn chatmix_only_touches_game_and_chat() {
        let mut gains = ChannelGains::default();
        let balance = ChatMixBalance {
            game: Percent::saturating(100),
            chat: Percent::saturating(0),
        };
        assert!(gains.apply(&Command::SetChatMix(balance)));
        assert!(approx(gains.gain(ChannelId::Game), 1.0));
        assert!(approx(gains.gain(ChannelId::Chat), 0.0));
        assert!(approx(gains.gain(ChannelId::Media), 1.0));
    }

    #[test]
    fn unchanged_commands_report_no_change() {
        let mut gains = ChannelGains::default();
        assert!(!gains.apply(&Command::SetMaster(Fader::UNITY)));
    }
}

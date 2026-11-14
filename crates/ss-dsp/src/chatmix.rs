//! ChatMix law (SON-07, HS-08): dial/slider position → Game and Chat bus gains.
//!
//! Sonar's `AttenuationCurveCoefficient = 2.0` (Observed, `docs/re/sonar.md` §3) suggests a
//! power law; we use `gain = (level / 100)^2`. The exact original curve is Unknown.

use ss_core::headset::{ChatMixBalance, Percent};

/// Exponent of the attenuation curve.
const CURVE_EXPONENT: i32 = 2;

/// Linear gains for the two buses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChatMixGains {
    /// Gain of the Game bus.
    pub game: f32,
    /// Gain of the Chat bus.
    pub chat: f32,
}

fn curve(level: Percent) -> f32 {
    (f32::from(level.get()) / 100.0).powi(CURVE_EXPONENT)
}

/// Gains for a ChatMix balance (as reported by the Nova Pro dial).
#[must_use]
pub fn chatmix_gains(balance: ChatMixBalance) -> ChatMixGains {
    ChatMixGains {
        game: curve(balance.game),
        chat: curve(balance.chat),
    }
}

/// Converts a software slider (−1 = only game … 0 = centre … +1 = only chat) into a
/// balance with the same shape the hardware dial reports.
#[must_use]
pub fn balance_from_slider(position: f32) -> ChatMixBalance {
    let p = if position.is_finite() {
        position.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let to_percent = |fraction: f32| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let value = (fraction * 100.0).round() as u8; // fraction is within 0..=1
        Percent::saturating(value)
    };
    ChatMixBalance {
        game: to_percent(1.0 - p.max(0.0)),
        chat: to_percent(1.0 + p.min(0.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balance(game: u8, chat: u8) -> ChatMixBalance {
        ChatMixBalance {
            game: Percent::new(game).unwrap(),
            chat: Percent::new(chat).unwrap(),
        }
    }

    #[test]
    fn centre_is_unity_and_extremes_mute() {
        assert_eq!(
            chatmix_gains(balance(100, 100)),
            ChatMixGains {
                game: 1.0,
                chat: 1.0
            }
        );
        assert_eq!(chatmix_gains(balance(100, 0)).chat, 0.0);
        assert!((chatmix_gains(balance(50, 100)).game - 0.25).abs() < 1e-6);
    }

    #[test]
    fn slider_matches_dial_shape() {
        assert_eq!(balance_from_slider(0.0), balance(100, 100));
        assert_eq!(balance_from_slider(1.0), balance(0, 100));
        assert_eq!(balance_from_slider(-0.5), balance(100, 50));
        assert_eq!(balance_from_slider(f32::NAN), balance(100, 100));
    }
}

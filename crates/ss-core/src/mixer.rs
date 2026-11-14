//! Mixer model of AudioMix (SON-01/02): playback channels and their faders.

/// Playback channels of the mixer (Sonar's Game/Chat/Media/Aux render roles, `docs/re/sonar.md` §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChannelId {
    /// Game audio (ChatMix "game" side).
    Game,
    /// Voice chat (ChatMix "chat" side).
    Chat,
    /// Music, browser, everything unassigned.
    Media,
    /// Auxiliary channel.
    Aux,
}

impl ChannelId {
    /// Number of playback channels.
    pub const COUNT: usize = 4;
    /// All channels in a stable order.
    pub const ALL: [Self; Self::COUNT] = [Self::Game, Self::Chat, Self::Media, Self::Aux];

    /// Stable index (`0..COUNT`), usable for fixed-size arrays.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Game => 0,
            Self::Chat => 1,
            Self::Media => 2,
            Self::Aux => 3,
        }
    }
}

/// Volume and mute state of a channel or of the master bus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fader {
    /// Linear gain, clamped to `0.0..=1.0` when applied.
    pub gain: f32,
    /// Muted faders output silence regardless of `gain`.
    pub muted: bool,
}

impl Fader {
    /// Full volume, not muted.
    pub const UNITY: Self = Self {
        gain: 1.0,
        muted: false,
    };

    /// The linear gain this fader applies.
    #[must_use]
    pub fn effective_gain(self) -> f32 {
        if self.muted || !self.gain.is_finite() {
            0.0
        } else {
            self.gain.clamp(0.0, 1.0)
        }
    }
}

impl Default for Fader {
    fn default() -> Self {
        Self::UNITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_are_unique_and_dense() {
        for (i, ch) in ChannelId::ALL.iter().enumerate() {
            assert_eq!(ch.index(), i);
        }
    }

    fn gain_of(gain: f32, muted: bool) -> f32 {
        Fader { gain, muted }.effective_gain()
    }

    #[test]
    fn fader_clamps_and_mutes() {
        assert!((gain_of(2.0, false) - 1.0).abs() < f32::EPSILON);
        assert!(gain_of(0.5, true).abs() < f32::EPSILON);
        assert!(gain_of(f32::NAN, false).abs() < f32::EPSILON);
    }
}

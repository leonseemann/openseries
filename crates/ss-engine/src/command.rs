//! Control messages from the daemon to the audio callback.

use ss_core::headset::ChatMixBalance;
use ss_core::mixer::{ChannelId, Fader};
use ss_dsp::ChannelStripSettings;

/// A change applied at the start of the next audio callback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Replace the DSP settings of one channel.
    SetStrip(ChannelId, ChannelStripSettings),
    /// Set volume/mute of one channel.
    SetFader(ChannelId, Fader),
    /// Set volume/mute of the summed output.
    SetMaster(Fader),
    /// Set the Game/Chat balance (from the headset dial or a UI slider).
    SetChatMix(ChatMixBalance),
}

//! Host → device commands.

use ss_core::headset::{
    AutoOff, ChatMixBalance, DisplayBrightness, EqPreset, HardwareEq, MicLedBrightness, MicVolume,
    NoiseControl, SidetoneLevel, TransparencyLevel,
};

use crate::codec;
use crate::protocol::{ARGS_OFFSET, OPCODE_OFFSET, REPORT_ID_COMMAND, REPORT_LEN, op};

/// A command the driver can send to the base station.
///
/// Only safe, documented operations are representable; factory reset and firmware update
/// commands are deliberately absent (reading the firmware version is allowed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Request the general status block (reply: [`crate::Event::Status`]).
    QueryStatus,
    /// Request the audio settings block (reply: [`crate::Event::Settings`]).
    QuerySettings,
    /// Request the display settings block (reply: [`crate::Event::Display`]).
    QueryDisplay,
    /// Request battery levels (reply: [`crate::Event::Battery`]).
    QueryBattery,
    /// Request the firmware version block (read-only; reply layout see `nova-pro-wireless.md`).
    QueryFirmware,
    /// Set the noise-control mode.
    SetNoiseControl(NoiseControl),
    /// Set the transparency strength.
    SetTransparency(TransparencyLevel),
    /// Set sidetone.
    SetSidetone(SidetoneLevel),
    /// Set hardware microphone gain.
    SetMicVolume(MicVolume),
    /// Select an EQ preset.
    SetEqPreset(EqPreset),
    /// Write the custom EQ curve (takes effect with [`EqPreset::Custom`]).
    SetCustomEq(HardwareEq),
    /// `true`: the dial reports ChatMix events and a host mixer applies them.
    /// Must be reset to `false` before the host mixer exits.
    SetChatMixDial(bool),
    /// Show/hide the "PC mixer present" indicator on the base station.
    SetMixerPresent(bool),
    /// Set the ChatMix position shown/held by the base station: `06 45 <game> <chat>`.
    ///
    /// **Observed** (`docs/hardware/nova-pro-wireless.md`): the settings block reads the new values
    /// back (found by analogy: every verified setting writes with the opcode of its event).
    /// Values are 0–100 by type ([`ss_core::headset::Percent`]).
    SetChatMix(ChatMixBalance),
    /// Set the inactivity auto-off timer.
    SetAutoOff(AutoOff),
    /// Set base-station OLED brightness.
    SetDisplayBrightness(DisplayBrightness),
    /// Set mic-mute LED brightness.
    SetMicLedBrightness(MicLedBrightness),
    /// Persist current settings to flash.
    Save,
}

impl Command {
    /// Encodes the command into a zero-padded output report.
    #[must_use]
    pub fn encode(&self) -> [u8; REPORT_LEN] {
        let mut report = [0u8; REPORT_LEN];
        report[0] = REPORT_ID_COMMAND;
        let (opcode, args) = self.opcode_and_args();
        report[OPCODE_OFFSET] = opcode;
        let args = args.as_slice();
        report[ARGS_OFFSET..ARGS_OFFSET + args.len()].copy_from_slice(args);
        report
    }

    /// The opcode this command uses (also the opcode of its reply, if any).
    #[must_use]
    pub fn opcode(&self) -> u8 {
        self.opcode_and_args().0
    }

    fn opcode_and_args(&self) -> (u8, Args) {
        match *self {
            Self::QueryStatus => (op::QUERY_STATUS, Args::none()),
            Self::QueryBattery => (op::BATTERY, Args::none()),
            Self::QuerySettings => (op::QUERY_SETTINGS, Args::none()),
            Self::QueryDisplay => (op::QUERY_DISPLAY, Args::none()),
            Self::QueryFirmware => (op::QUERY_FIRMWARE, Args::none()),
            Self::SetNoiseControl(m) => (
                op::NOISE_CONTROL,
                Args::one(codec::noise_control_to_byte(m)),
            ),
            Self::SetTransparency(l) => (op::TRANSPARENCY, Args::one(l.get())),
            Self::SetSidetone(l) => (op::SIDETONE, Args::one(codec::sidetone_to_byte(l))),
            Self::SetMicVolume(v) => (op::MIC_VOLUME, Args::one(v.get())),
            Self::SetEqPreset(p) => (op::EQ_PRESET, Args::one(codec::eq_preset_to_byte(p))),
            Self::SetCustomEq(eq) => (op::EQ_BANDS, Args::eq(&eq)),
            Self::SetChatMixDial(on) => (op::CHATMIX_DIAL_ENABLE, Args::one(u8::from(on))),
            Self::SetMixerPresent(on) => (op::MIXER_PRESENT, Args::one(u8::from(on))),
            Self::SetChatMix(balance) => (
                op::CHATMIX,
                Args::two(balance.game.get(), balance.chat.get()),
            ),
            Self::SetAutoOff(t) => (op::AUTO_OFF, Args::one(codec::auto_off_to_byte(t))),
            Self::SetDisplayBrightness(b) => (op::OLED_BRIGHTNESS, Args::one(b.get())),
            Self::SetMicLedBrightness(b) => (op::MIC_LED_BRIGHTNESS, Args::one(b.get())),
            Self::Save => (op::SAVE, Args::none()),
        }
    }
}

/// Small fixed-capacity argument buffer (avoids allocation).
struct Args {
    bytes: [u8; ss_core::headset::HARDWARE_EQ_BANDS],
    len: usize,
}

impl Args {
    const fn none() -> Self {
        Self {
            bytes: [0; ss_core::headset::HARDWARE_EQ_BANDS],
            len: 0,
        }
    }

    const fn one(value: u8) -> Self {
        let mut args = Self::none();
        args.bytes[0] = value;
        args.len = 1;
        args
    }

    const fn two(first: u8, second: u8) -> Self {
        let mut args = Self::none();
        args.bytes[0] = first;
        args.bytes[1] = second;
        args.len = 2;
        args
    }

    fn eq(eq: &HardwareEq) -> Self {
        let mut args = Self::none();
        for (slot, gain) in args.bytes.iter_mut().zip(eq.bands) {
            *slot = codec::eq_gain_to_byte(gain);
        }
        args.len = eq.bands.len();
        args
    }

    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss_core::headset::EqGain;
    use ss_core::headset::Percent;

    fn head(cmd: Command, n: usize) -> Vec<u8> {
        cmd.encode()[..n].to_vec()
    }

    #[test]
    fn encodes_documented_bytes() {
        // Expected bytes from docs/research/prior-art.md §1.3.
        assert_eq!(head(Command::QueryStatus, 3), [0x06, 0xB0, 0x00]);
        assert_eq!(head(Command::QueryBattery, 2), [0x06, 0xB7]);
        assert_eq!(
            head(Command::SetNoiseControl(NoiseControl::NoiseCancelling), 3),
            [0x06, 0xBD, 2]
        );
        assert_eq!(
            head(
                Command::SetTransparency(TransparencyLevel::new(7).unwrap()),
                3
            ),
            [0x06, 0xB9, 7]
        );
        assert_eq!(
            head(Command::SetSidetone(SidetoneLevel::High), 3),
            [0x06, 0x39, 3]
        );
        assert_eq!(
            head(Command::SetMicVolume(MicVolume::new(10).unwrap()), 3),
            [0x06, 0x37, 10]
        );
        assert_eq!(
            head(Command::SetEqPreset(EqPreset::Custom), 3),
            [0x06, 0x2E, 4]
        );
        assert_eq!(head(Command::SetChatMixDial(true), 3), [0x06, 0x49, 1]);
        assert_eq!(head(Command::SetMixerPresent(false), 3), [0x06, 0x8D, 0]);
        let balance = ChatMixBalance {
            game: Percent::new(100).unwrap(),
            chat: Percent::new(50).unwrap(),
        };
        assert_eq!(head(Command::SetChatMix(balance), 4), [0x06, 0x45, 100, 50]);
        assert_eq!(
            head(Command::SetAutoOff(AutoOff::After15Min), 3),
            [0x06, 0xC1, 4]
        );
        assert_eq!(
            head(
                Command::SetDisplayBrightness(DisplayBrightness::new(5).unwrap()),
                3
            ),
            [0x06, 0x85, 5]
        );
        assert_eq!(
            head(
                Command::SetMicLedBrightness(MicLedBrightness::new(1).unwrap()),
                3
            ),
            [0x06, 0xBF, 1]
        );
        assert_eq!(head(Command::Save, 2), [0x06, 0x09]);
    }

    #[test]
    fn encodes_custom_eq_bands() {
        let mut eq = HardwareEq::FLAT;
        eq.bands[0] = EqGain::from_db(-10.0).unwrap();
        eq.bands[9] = EqGain::from_db(3.5).unwrap();
        let report = Command::SetCustomEq(eq).encode();
        assert_eq!(report[..2], [0x06, 0x33]);
        assert_eq!(report[2], 0);
        assert_eq!(report[3..11], [0x14; 8]);
        assert_eq!(report[11], 0x14 + 7);
        assert!(report[12..].iter().all(|&b| b == 0), "rest is zero padding");
    }

    #[test]
    fn reports_are_full_length() {
        assert_eq!(Command::Save.encode().len(), REPORT_LEN);
    }
}

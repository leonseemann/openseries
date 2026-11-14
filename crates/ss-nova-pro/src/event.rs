//! Device → host reports: replies to queries and unsolicited events.

use ss_core::battery::{BatteryLevel, BatteryReport, ChargeState};
use ss_core::headset::{
    AutoOff, ChatMixBalance, DisplayBrightness, EqGain, EqPreset, HARDWARE_EQ_BANDS, HardwareEq,
    MicLedBrightness, MicVolume, NoiseControl, Percent, SidetoneLevel, TransparencyLevel,
};
use thiserror::Error;

use crate::codec;
use crate::protocol::{
    ARGS_OFFSET, CONNECTION_RADIO_OFFSET, OPCODE_OFFSET, RADIO_CONNECTED, REPORT_ID_COMMAND,
    REPORT_ID_EVENT, battery_offset, display_offset, op, settings_offset, status_offset,
};

/// Snapshot of the general status block (`QueryStatus` reply).
///
/// Fields are `None` when the device reported a value outside the documented range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    /// `true` if the headset is linked to the base station.
    pub connected: bool,
    /// Headset battery level.
    pub headset_battery: Option<BatteryLevel>,
    /// Spare battery in the base-station slot.
    pub spare_battery: Option<BatteryLevel>,
    /// Charging state of the headset.
    pub charge_state: ChargeState,
    /// Current noise-control mode.
    pub noise_control: Option<NoiseControl>,
    /// Current transparency strength.
    pub transparency: Option<TransparencyLevel>,
    /// Microphone mute switch state.
    pub mic_muted: bool,
    /// Auto-off timer.
    pub auto_off: Option<AutoOff>,
    /// Mic-mute LED brightness.
    pub mic_led: Option<MicLedBrightness>,
}

/// Snapshot of the audio settings block (`QuerySettings` reply).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// Active EQ preset (`None` for game presets this driver does not model).
    pub eq_preset: Option<EqPreset>,
    /// Stored custom EQ curve (`None` if any band is out of range).
    pub custom_eq: Option<HardwareEq>,
    /// Hardware microphone volume.
    pub mic_volume: Option<MicVolume>,
    /// Sidetone level.
    pub sidetone: Option<SidetoneLevel>,
    /// Last ChatMix dial position.
    pub chatmix: Option<ChatMixBalance>,
}

/// Snapshot of the display settings block (`QueryDisplay` reply).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
    /// OLED brightness.
    pub brightness: Option<DisplayBrightness>,
    /// Whether the "PC mixer present" indicator is shown.
    pub mixer_present: bool,
}

/// A decoded report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// Reply to [`crate::Command::QueryStatus`].
    Status(Status),
    /// Reply to [`crate::Command::QuerySettings`].
    Settings(Settings),
    /// Reply to [`crate::Command::QueryDisplay`].
    Display(Display),
    /// Battery levels (reply to [`crate::Command::QueryBattery`] or periodic event).
    Battery(BatteryReport),
    /// Noise-control mode changed with the headset button (Observed).
    NoiseControl(NoiseControl),
    /// Microphone mute switch toggled (Observed).
    MicMute(bool),
    /// Headset volume dial turned.
    Volume(Percent),
    /// Sidetone changed.
    Sidetone(SidetoneLevel),
    /// Microphone volume changed.
    MicVolume(MicVolume),
    /// EQ preset changed.
    EqPreset(EqPreset),
    /// A single EQ band was changed on the base station.
    EqBand {
        /// Zero-based band index.
        index: u8,
        /// New gain.
        gain: EqGain,
    },
    /// ChatMix dial moved (only while the dial is enabled).
    ChatMix(ChatMixBalance),
    /// Auto-off timer changed.
    AutoOff(AutoOff),
    /// OLED brightness changed.
    DisplayBrightness(DisplayBrightness),
    /// Reply to [`crate::Command::QueryFirmware`] (`None` if the layout is not the Observed one).
    Firmware(Option<crate::FirmwareVersions>),
    /// Headset connected/disconnected; re-query status for details.
    Connection {
        /// `true` if the headset is now linked.
        connected: bool,
    },
    /// Acknowledgement/echo of a command without a modelled payload.
    Reply {
        /// Opcode of the acknowledged command.
        opcode: u8,
    },
    /// A report this driver does not (yet) model. Useful for protocol exploration.
    Unrecognized {
        /// Report ID byte.
        report_id: u8,
        /// Opcode byte.
        opcode: u8,
        /// First argument bytes (zero-padded), for protocol exploration.
        args: [u8; UNRECOGNIZED_ARGS],
    },
}

/// Number of argument bytes kept in [`Event::Unrecognized`].
pub const UNRECOGNIZED_ARGS: usize = 4;

/// Errors while decoding a report.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecodeError {
    /// The report is shorter than its layout requires.
    #[error("report too short: need {needed} bytes, got {actual}")]
    TooShort {
        /// Bytes required.
        needed: usize,
        /// Bytes received.
        actual: usize,
    },
}

impl Event {
    /// Decodes one input report (including its report-ID byte).
    ///
    /// # Errors
    /// Returns [`DecodeError::TooShort`] if the report is truncated.
    pub fn decode(report: &[u8]) -> Result<Self, DecodeError> {
        let reader = Reader(report);
        let report_id = reader.byte(0)?;
        let opcode = reader.byte(OPCODE_OFFSET)?;
        let mut args = [0u8; UNRECOGNIZED_ARGS];
        for (i, slot) in args.iter_mut().enumerate() {
            *slot = report.get(ARGS_OFFSET + i).copied().unwrap_or(0);
        }
        let unrecognized = Self::Unrecognized {
            report_id,
            opcode,
            args,
        };

        let event = match (report_id, opcode) {
            (REPORT_ID_COMMAND, op::QUERY_STATUS) => Some(Self::Status(decode_status(&reader)?)),
            (REPORT_ID_COMMAND, op::QUERY_SETTINGS) => {
                Some(Self::Settings(decode_settings(&reader)?))
            }
            (REPORT_ID_COMMAND, op::QUERY_DISPLAY) => Some(Self::Display(decode_display(&reader)?)),
            (REPORT_ID_COMMAND, op::QUERY_FIRMWARE) => Some(Self::Firmware(
                crate::FirmwareVersions::parse(report.get(ARGS_OFFSET..).unwrap_or_default()),
            )),
            (REPORT_ID_COMMAND | REPORT_ID_EVENT, op::BATTERY) => decode_battery(&reader)?,
            (REPORT_ID_EVENT, op::NOISE_CONTROL) => {
                codec::noise_control_from_byte(reader.arg(0)?).map(Self::NoiseControl)
            }
            (REPORT_ID_EVENT, op::MIC_MUTE) => Some(Self::MicMute(reader.arg(0)? != 0)),
            (REPORT_ID_EVENT, op::VOLUME) => {
                codec::volume_from_attenuation(reader.arg(0)?).map(Self::Volume)
            }
            (REPORT_ID_EVENT, op::SIDETONE) => {
                codec::sidetone_from_byte(reader.arg(0)?).map(Self::Sidetone)
            }
            (REPORT_ID_EVENT, op::MIC_VOLUME) => {
                MicVolume::new(reader.arg(0)?).ok().map(Self::MicVolume)
            }
            (REPORT_ID_EVENT, op::EQ_PRESET) => {
                codec::eq_preset_from_byte(reader.arg(0)?).map(Self::EqPreset)
            }
            (REPORT_ID_EVENT, op::EQ_BAND) => {
                let index = reader.arg(0)?;
                codec::eq_gain_from_byte(reader.arg(1)?).map(|gain| Self::EqBand { index, gain })
            }
            (REPORT_ID_EVENT, op::CHATMIX) => decode_chatmix(&reader)?,
            (REPORT_ID_EVENT, op::AUTO_OFF) => {
                codec::auto_off_from_byte(reader.arg(0)?).map(Self::AutoOff)
            }
            (REPORT_ID_EVENT, op::OLED_BRIGHTNESS) => DisplayBrightness::new(reader.arg(0)?)
                .ok()
                .map(Self::DisplayBrightness),
            (REPORT_ID_EVENT, op::CONNECTION) => Some(Self::Connection {
                connected: reader.byte(CONNECTION_RADIO_OFFSET)? == RADIO_CONNECTED,
            }),
            (REPORT_ID_COMMAND, _) => Some(Self::Reply { opcode }),
            _ => None,
        };
        Ok(event.unwrap_or(unrecognized))
    }
}

fn decode_status(r: &Reader<'_>) -> Result<Status, DecodeError> {
    let (_, charge_state) = codec::power_state(r.byte(status_offset::POWER_STATE)?);
    Ok(Status {
        connected: r.byte(status_offset::RADIO_STATE)? == RADIO_CONNECTED,
        headset_battery: codec::battery_level(r.byte(status_offset::HEADSET_BATTERY)?),
        spare_battery: codec::battery_level(r.byte(status_offset::SPARE_BATTERY)?),
        charge_state,
        noise_control: codec::noise_control_from_byte(r.byte(status_offset::NOISE_CONTROL)?),
        transparency: TransparencyLevel::new(r.byte(status_offset::TRANSPARENCY)?).ok(),
        mic_muted: r.byte(status_offset::MIC_MUTED)? != 0,
        auto_off: codec::auto_off_from_byte(r.byte(status_offset::AUTO_OFF)?),
        mic_led: MicLedBrightness::new(r.byte(status_offset::MIC_LED_BRIGHTNESS)?).ok(),
    })
}

fn decode_settings(r: &Reader<'_>) -> Result<Settings, DecodeError> {
    let mut custom_eq = Some(HardwareEq::FLAT);
    for i in 0..HARDWARE_EQ_BANDS {
        let gain = codec::eq_gain_from_byte(r.byte(settings_offset::EQ_BANDS + i)?);
        custom_eq = custom_eq.zip(gain).map(|(mut eq, gain)| {
            eq.bands[i] = gain;
            eq
        });
    }
    let game = Percent::new(r.byte(settings_offset::CHATMIX_GAME)?).ok();
    let chat = Percent::new(r.byte(settings_offset::CHATMIX_CHAT)?).ok();
    Ok(Settings {
        eq_preset: codec::eq_preset_from_byte(r.byte(settings_offset::EQ_PRESET)?),
        custom_eq,
        mic_volume: MicVolume::new(r.byte(settings_offset::MIC_VOLUME)?).ok(),
        sidetone: codec::sidetone_from_byte(r.byte(settings_offset::SIDETONE)?),
        chatmix: game
            .zip(chat)
            .map(|(game, chat)| ChatMixBalance { game, chat }),
    })
}

fn decode_display(r: &Reader<'_>) -> Result<Display, DecodeError> {
    Ok(Display {
        brightness: DisplayBrightness::new(r.byte(display_offset::BRIGHTNESS)?).ok(),
        mixer_present: r.byte(display_offset::MIXER_PRESENT)? != 0,
    })
}

fn decode_battery(r: &Reader<'_>) -> Result<Option<Event>, DecodeError> {
    let headset = codec::battery_level(r.byte(battery_offset::HEADSET)?);
    let spare = codec::battery_level(r.byte(battery_offset::SPARE)?);
    let state = codec::battery_charge_state(r.byte(battery_offset::STATE)?);
    Ok(headset.map(|headset| {
        Event::Battery(BatteryReport {
            headset,
            spare,
            state,
        })
    }))
}

fn decode_chatmix(r: &Reader<'_>) -> Result<Option<Event>, DecodeError> {
    let game = Percent::new(r.arg(0)?).ok();
    let chat = Percent::new(r.arg(1)?).ok();
    Ok(game
        .zip(chat)
        .map(|(game, chat)| Event::ChatMix(ChatMixBalance { game, chat })))
}

/// Bounds-checked byte access.
struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn byte(&self, offset: usize) -> Result<u8, DecodeError> {
        self.0.get(offset).copied().ok_or(DecodeError::TooShort {
            needed: offset + 1,
            actual: self.0.len(),
        })
    }

    fn arg(&self, index: usize) -> Result<u8, DecodeError> {
        self.byte(ARGS_OFFSET + index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(bytes: &[u8]) -> Vec<u8> {
        let mut r = bytes.to_vec();
        r.resize(crate::REPORT_LEN, 0);
        r
    }

    #[test]
    fn decodes_status_block() {
        let mut r = report(&[0x06, 0xB0]);
        r[6] = 8; // headset battery full
        r[7] = 4; // spare 50 %
        r[8] = 5; // transparency
        r[9] = 1; // mic muted
        r[10] = 2; // ANC
        r[12] = 3; // auto-off 10 min
        r[14] = 8; // connected
        r[15] = 8; // on battery
        let Event::Status(s) = Event::decode(&r).unwrap() else {
            panic!("not status")
        };
        assert!(s.connected);
        assert_eq!(s.headset_battery.unwrap().percent(), 100);
        assert_eq!(s.spare_battery.unwrap().percent(), 50);
        assert_eq!(s.transparency.unwrap().get(), 5);
        assert!(s.mic_muted);
        assert_eq!(s.noise_control, Some(NoiseControl::NoiseCancelling));
        assert_eq!(s.auto_off, Some(AutoOff::After10Min));
        assert_eq!(s.charge_state, ChargeState::Discharging);
    }

    /// Replies captured from a real base station (PID 0x12E0) on 2026-11-14,
    /// see docs/hardware/nova-pro-wireless.md.
    const CAPTURED_SETTINGS: [u8; 26] = [
        0x06, 0x20, 0x01, 0x1c, 0x01, 0x00, 0x00, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14,
        0x14, 0x14, 0x0a, 0x00, 0x01, 0x64, 0x64, 0x64, 0x00, 0x64, 0x64,
    ];
    const CAPTURED_DISPLAY: [u8; 8] = [0x06, 0x80, 0x03, 0x0a, 0x01, 0x00, 0x00, 0x01];
    const CAPTURED_STATUS: [u8; 16] = [
        0x06, 0xb0, 0x00, 0x00, 0x01, 0x00, 0x04, 0x08, 0x08, 0x00, 0x00, 0x0a, 0x05, 0x00, 0x08,
        0x08,
    ];

    #[test]
    fn decodes_captured_settings() {
        let Event::Settings(s) = Event::decode(&report(&CAPTURED_SETTINGS)).unwrap() else {
            panic!("not settings")
        };
        assert_eq!(s.eq_preset, Some(EqPreset::Flat));
        assert!(s.custom_eq.unwrap().is_flat());
        assert_eq!(s.mic_volume.unwrap().get(), 10);
        assert_eq!(s.sidetone, Some(SidetoneLevel::Off));
        let mix = s.chatmix.unwrap();
        assert_eq!((mix.game.get(), mix.chat.get()), (100, 100));
    }

    #[test]
    fn decodes_captured_display_and_status() {
        let Event::Display(d) = Event::decode(&report(&CAPTURED_DISPLAY)).unwrap() else {
            panic!("not display")
        };
        assert_eq!(d.brightness.unwrap().get(), 10);
        assert!(d.mixer_present);

        let Event::Status(s) = Event::decode(&report(&CAPTURED_STATUS)).unwrap() else {
            panic!("not status")
        };
        assert!(s.connected);
        assert_eq!(s.headset_battery.unwrap().percent(), 50);
        assert_eq!(s.spare_battery.unwrap().percent(), 100);
        assert_eq!(s.transparency.unwrap().get(), 8);
        assert_eq!(s.noise_control, Some(NoiseControl::Off));
        assert_eq!(s.mic_led.unwrap().get(), 10);
        assert_eq!(s.auto_off, Some(AutoOff::After30Min));
    }

    #[test]
    fn decodes_battery_event() {
        let ev = Event::decode(&report(&[0x07, 0xB7, 6, 8, 2])).unwrap();
        assert_eq!(
            ev,
            Event::Battery(BatteryReport {
                headset: BatteryLevel::from_percent(75).unwrap(),
                spare: Some(BatteryLevel::from_percent(100).unwrap()),
                state: ChargeState::Charging,
            })
        );
    }

    #[test]
    fn decodes_chatmix_dial() {
        let ev = Event::decode(&report(&[0x07, 0x45, 100, 40])).unwrap();
        let Event::ChatMix(b) = ev else {
            panic!("not chatmix")
        };
        assert_eq!((b.game.get(), b.chat.get()), (100, 40));
    }

    #[test]
    fn decodes_simple_events() {
        // Observed on hardware 2026-11-14: ANC button, mute switch, volume dial.
        assert_eq!(
            Event::decode(&report(&[0x07, 0xBD, 2])).unwrap(),
            Event::NoiseControl(NoiseControl::NoiseCancelling)
        );
        assert_eq!(
            Event::decode(&report(&[0x07, 0xBB, 1])).unwrap(),
            Event::MicMute(true)
        );
        assert_eq!(
            Event::decode(&report(&[0x07, 0xBB, 0])).unwrap(),
            Event::MicMute(false)
        );
        assert_eq!(
            Event::decode(&report(&[0x07, 0x25, 0x1c])).unwrap(),
            Event::Volume(Percent::new(50).unwrap())
        );
        assert_eq!(
            Event::decode(&report(&[0x07, 0x39, 2])).unwrap(),
            Event::Sidetone(SidetoneLevel::Medium)
        );
        assert_eq!(
            Event::decode(&report(&[0x07, 0x2E, 1])).unwrap(),
            Event::EqPreset(EqPreset::BassBoost)
        );
        assert_eq!(
            Event::decode(&report(&[0x07, 0x31, 3, 0x16])).unwrap(),
            Event::EqBand {
                index: 3,
                gain: EqGain::from_db(1.0).unwrap()
            }
        );
        assert_eq!(
            Event::decode(&report(&[0x07, 0xB5, 0, 0, 8])).unwrap(),
            Event::Connection { connected: true }
        );
        assert_eq!(
            Event::decode(&report(&[0x06, 0x39])).unwrap(),
            Event::Reply { opcode: 0x39 }
        );
    }

    #[test]
    fn out_of_range_values_are_unrecognized() {
        assert_eq!(
            Event::decode(&report(&[0x07, 0x39, 9])).unwrap(),
            Event::Unrecognized {
                report_id: 0x07,
                opcode: 0x39,
                args: [9, 0, 0, 0]
            }
        );
        assert_eq!(
            Event::decode(&report(&[0x07, 0x2E, 7])).unwrap(),
            Event::Unrecognized {
                report_id: 0x07,
                opcode: 0x2E,
                args: [7, 0, 0, 0]
            }
        );
    }

    #[test]
    fn truncated_reports_fail() {
        assert!(Event::decode(&[]).is_err());
        assert!(Event::decode(&[0x06, 0xB0, 0, 0]).is_err());
    }
}

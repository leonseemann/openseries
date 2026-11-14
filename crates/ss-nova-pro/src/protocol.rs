//! Report framing and opcodes (prior-art.md §1.2–§1.3).
//!
//! Byte offsets are 0-based over the full report including the report-ID byte:
//! `[0]` = report ID, `[1]` = opcode, `[2..]` = arguments.

/// Length of every report sent to the base station (zero-padded).
pub const REPORT_LEN: usize = 64;

/// Report ID of host → device commands and of device replies to them.
pub const REPORT_ID_COMMAND: u8 = 0x06;
/// Report ID of unsolicited device events.
pub const REPORT_ID_EVENT: u8 = 0x07;

/// Offset of the opcode byte.
pub const OPCODE_OFFSET: usize = 1;
/// Offset of the first argument byte.
pub const ARGS_OFFSET: usize = 2;

/// Opcodes. Names describe the effect; values are the documented wire bytes.
pub mod op {
    /// Persist current settings to flash.
    pub const SAVE: u8 = 0x09;
    /// Equalizer preset.
    pub const EQ_PRESET: u8 = 0x2E;
    /// Single EQ band changed on the base station (event).
    pub const EQ_BAND: u8 = 0x31;
    /// Write all ten custom EQ bands.
    pub const EQ_BANDS: u8 = 0x33;
    /// Microphone volume.
    pub const MIC_VOLUME: u8 = 0x37;
    /// Sidetone level.
    pub const SIDETONE: u8 = 0x39;
    /// ChatMix dial position (event).
    pub const CHATMIX: u8 = 0x45;
    /// Let the dial report ChatMix instead of mixing internally.
    pub const CHATMIX_DIAL_ENABLE: u8 = 0x49;
    /// OLED brightness.
    pub const OLED_BRIGHTNESS: u8 = 0x85;
    /// Show the "PC mixer present" icon on the base station.
    pub const MIXER_PRESENT: u8 = 0x8D;
    /// Read the general status block.
    pub const QUERY_STATUS: u8 = 0xB0;
    /// Connection state changed (event).
    pub const CONNECTION: u8 = 0xB5;
    /// Battery levels.
    pub const BATTERY: u8 = 0xB7;
    /// Transparency level.
    pub const TRANSPARENCY: u8 = 0xB9;
    /// Noise-control mode (off / transparency / ANC).
    pub const NOISE_CONTROL: u8 = 0xBD;
    /// Mic-mute LED brightness.
    pub const MIC_LED_BRIGHTNESS: u8 = 0xBF;
    /// Inactivity auto-off timer.
    pub const AUTO_OFF: u8 = 0xC1;
}

/// Field offsets inside the `QUERY_STATUS` (0xB0) reply.
pub mod status_offset {
    /// Headset battery, 0–8.
    pub const HEADSET_BATTERY: usize = 6;
    /// Spare battery in the base station, 0–8.
    pub const SPARE_BATTERY: usize = 7;
    /// Transparency level, 1–10.
    pub const TRANSPARENCY: usize = 8;
    /// Microphone muted flag.
    pub const MIC_MUTED: usize = 9;
    /// Noise-control mode.
    pub const NOISE_CONTROL: usize = 10;
    /// Auto-off enum.
    pub const AUTO_OFF: usize = 12;
    /// Radio state ([`super::RADIO_CONNECTED`] when the headset is linked).
    pub const RADIO_STATE: usize = 14;
    /// Headset power state.
    pub const POWER_STATE: usize = 15;
}

/// Field offsets inside a `BATTERY` (0xB7) reply/event.
pub mod battery_offset {
    /// Headset battery, 0–8.
    pub const HEADSET: usize = 2;
    /// Spare battery, 0–8 (conflicting source labels it "charging", see prior-art §1.3).
    pub const SPARE: usize = 3;
    /// Charge state code.
    pub const STATE: usize = 4;
}

/// Offset of the radio-state byte in a `CONNECTION` (0xB5) event.
pub const CONNECTION_RADIO_OFFSET: usize = 4;

/// Radio-state value meaning "headset connected".
pub const RADIO_CONNECTED: u8 = 8;

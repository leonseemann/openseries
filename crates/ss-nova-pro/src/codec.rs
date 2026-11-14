//! Conversions between domain values (`ss-core`) and wire bytes (prior-art.md §1.3).

use ss_core::battery::{BatteryLevel, ChargeState};
use ss_core::headset::{AutoOff, EqGain, EqPreset, NoiseControl, SidetoneLevel};

/// Battery is reported in eighths (0–8, 12.5 % per step).
const BATTERY_STEPS: u8 = 8;

/// Converts a 0–8 battery step value into a percentage (rounded half up).
pub fn battery_level(steps: u8) -> Option<BatteryLevel> {
    if steps > BATTERY_STEPS {
        return None;
    }
    let percent =
        (u16::from(steps) * 100 + u16::from(BATTERY_STEPS / 2)) / u16::from(BATTERY_STEPS);
    // `percent` is at most 100, so the conversion and validation cannot fail.
    u8::try_from(percent)
        .ok()
        .and_then(|p| BatteryLevel::from_percent(p).ok())
}

/// Decodes the charge-state code of a battery report.
pub fn battery_charge_state(code: u8) -> ChargeState {
    match code {
        2 => ChargeState::Charging,
        4 => ChargeState::PluggedNotCharging,
        8 => ChargeState::Discharging,
        _ => ChargeState::Unknown,
    }
}

/// Headset power state from the status block: `(connected, charge state)`.
pub fn power_state(code: u8) -> (bool, ChargeState) {
    match code {
        2 => (true, ChargeState::Charging),
        4 => (true, ChargeState::PluggedNotCharging),
        8 => (true, ChargeState::Discharging),
        _ => (false, ChargeState::Unknown),
    }
}

pub fn noise_control_to_byte(mode: NoiseControl) -> u8 {
    match mode {
        NoiseControl::Off => 0,
        NoiseControl::Transparency => 1,
        NoiseControl::NoiseCancelling => 2,
    }
}

pub fn noise_control_from_byte(byte: u8) -> Option<NoiseControl> {
    match byte {
        0 => Some(NoiseControl::Off),
        1 => Some(NoiseControl::Transparency),
        2 => Some(NoiseControl::NoiseCancelling),
        _ => None,
    }
}

pub fn sidetone_to_byte(level: SidetoneLevel) -> u8 {
    match level {
        SidetoneLevel::Off => 0,
        SidetoneLevel::Low => 1,
        SidetoneLevel::Medium => 2,
        SidetoneLevel::High => 3,
    }
}

pub fn sidetone_from_byte(byte: u8) -> Option<SidetoneLevel> {
    match byte {
        0 => Some(SidetoneLevel::Off),
        1 => Some(SidetoneLevel::Low),
        2 => Some(SidetoneLevel::Medium),
        3 => Some(SidetoneLevel::High),
        _ => None,
    }
}

pub fn auto_off_to_byte(timeout: AutoOff) -> u8 {
    match timeout {
        AutoOff::Never => 0,
        AutoOff::After1Min => 1,
        AutoOff::After5Min => 2,
        AutoOff::After10Min => 3,
        AutoOff::After15Min => 4,
        AutoOff::After30Min => 5,
        AutoOff::After60Min => 6,
    }
}

pub fn auto_off_from_byte(byte: u8) -> Option<AutoOff> {
    match byte {
        0 => Some(AutoOff::Never),
        1 => Some(AutoOff::After1Min),
        2 => Some(AutoOff::After5Min),
        3 => Some(AutoOff::After10Min),
        4 => Some(AutoOff::After15Min),
        5 => Some(AutoOff::After30Min),
        6 => Some(AutoOff::After60Min),
        _ => None,
    }
}

pub fn eq_preset_to_byte(preset: EqPreset) -> u8 {
    match preset {
        EqPreset::Flat => 0,
        EqPreset::BassBoost => 1,
        EqPreset::Focus => 2,
        EqPreset::Smiley => 3,
        EqPreset::Custom => 4,
    }
}

/// Decodes a preset; game presets (5–18 per one source) are not modelled and yield `None`.
pub fn eq_preset_from_byte(byte: u8) -> Option<EqPreset> {
    match byte {
        0 => Some(EqPreset::Flat),
        1 => Some(EqPreset::BassBoost),
        2 => Some(EqPreset::Focus),
        3 => Some(EqPreset::Smiley),
        4 => Some(EqPreset::Custom),
        _ => None,
    }
}

/// Wire value of 0 dB; each step is 0.5 dB (`byte = 0x14 + 2·dB`).
const EQ_ZERO_DB_BYTE: u8 = 0x14;
/// Wire steps per decibel.
const EQ_STEPS_PER_DB: f32 = 2.0;

/// Encodes a gain, quantised to the device's 0.5 dB resolution.
pub fn eq_gain_to_byte(gain: EqGain) -> u8 {
    let steps = (gain.db() * EQ_STEPS_PER_DB).round();
    // EqGain guarantees −10..=10 dB, so `steps` is in −20..=20 and the sum in 0..=40.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = (f32::from(EQ_ZERO_DB_BYTE) + steps) as u8;
    byte
}

/// Decodes a wire byte into a gain; `None` if outside the documented range.
pub fn eq_gain_from_byte(byte: u8) -> Option<EqGain> {
    let db = (f32::from(byte) - f32::from(EQ_ZERO_DB_BYTE)) / EQ_STEPS_PER_DB;
    EqGain::from_db(db).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_steps_map_to_percent() {
        let expected = [0, 13, 25, 38, 50, 63, 75, 88, 100];
        for (steps, pct) in (0u8..=8).zip(expected) {
            assert_eq!(
                battery_level(steps).unwrap().percent(),
                pct,
                "steps {steps}"
            );
        }
        assert!(battery_level(9).is_none());
    }

    #[test]
    fn eq_gain_round_trips_and_quantises() {
        for byte in 0u8..=40 {
            let gain = eq_gain_from_byte(byte).unwrap();
            assert_eq!(eq_gain_to_byte(gain), byte);
        }
        assert_eq!(eq_gain_to_byte(EqGain::ZERO), 0x14);
        assert_eq!(eq_gain_to_byte(EqGain::from_db(-10.0).unwrap()), 0);
        assert_eq!(eq_gain_to_byte(EqGain::from_db(10.0).unwrap()), 40);
        assert_eq!(eq_gain_to_byte(EqGain::from_db(1.3).unwrap()), 0x14 + 3);
        assert!(eq_gain_from_byte(41).is_none());
    }

    #[test]
    fn enums_round_trip() {
        for b in 0..=2 {
            assert_eq!(
                noise_control_to_byte(noise_control_from_byte(b).unwrap()),
                b
            );
        }
        for b in 0..=3 {
            assert_eq!(sidetone_to_byte(sidetone_from_byte(b).unwrap()), b);
        }
        for b in 0..=6 {
            assert_eq!(auto_off_to_byte(auto_off_from_byte(b).unwrap()), b);
        }
        for b in 0..=4 {
            assert_eq!(eq_preset_to_byte(eq_preset_from_byte(b).unwrap()), b);
        }
        assert!(noise_control_from_byte(3).is_none());
        assert!(eq_preset_from_byte(5).is_none());
    }
}

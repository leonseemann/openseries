//! Headset page: status and the settings the Nova Pro supports (`ss-nova-pro` commands).

use eframe::egui;
use ss_core::battery::ChargeState;
use ss_core::headset::{
    AutoOff, DisplayBrightness, EqPreset, MicLedBrightness, MicVolume, NoiseControl, SidetoneLevel,
    TransparencyLevel,
};
use ss_nova_pro::{Command, Settings, Status};

use crate::core::{AppCore, Snapshot};
use crate::headset::HeadsetState;

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let headset = &snapshot.headset;
    ui.heading("Arctis Nova Pro Wireless");
    if let Some(error) = &headset.error {
        ui.colored_label(ui.visuals().warn_fg_color, error);
    }
    ui.horizontal(|ui| {
        if crate::platform::USB_RESET_SUPPORTED
            && ui
                .button("Reconnect USB")
                .on_hover_text(
                    "Resets the base station's USB connection, like unplugging and replugging the \
                 cable (keeps power). Audio drops for about a second.",
                )
                .clicked()
        {
            core.reset_headset_usb();
        }
        ui.label(
            egui::RichText::new("Battery is reported in 9 steps (12.5 %) by the base station.")
                .weak(),
        );
    });
    let Some(status) = headset.status else {
        ui.label(if headset.base_station {
            "Reading the base station…"
        } else {
            "Base station not found. Connect it via USB (USB-1 port on the base station)."
        });
        return;
    };
    if !status.connected {
        ui.label("Base station connected, headset is off or out of range.");
    }

    status_grid(ui, status);
    noise_control(ui, core, status);
    microphone_and_sound(ui, core, headset.settings);
    power_and_display(ui, core, status, headset);
}

fn status_grid(ui: &mut egui::Ui, status: Status) {
    egui::Grid::new("headset-status")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Battery");
            ui.label(match status.headset_battery {
                Some(level) => format!(
                    "≈{} % ({})",
                    level.percent(),
                    charge_text(status.charge_state)
                ),
                None => "unknown".to_owned(),
            });
            ui.end_row();
            ui.label("Spare battery");
            ui.label(status.spare_battery.map_or_else(
                || "none".to_owned(),
                |level| format!("≈{} %", level.percent()),
            ));
            ui.end_row();
            ui.label("Microphone");
            ui.label(if status.mic_muted { "muted" } else { "live" });
            ui.end_row();
        });
}

fn noise_control(ui: &mut egui::Ui, core: &AppCore, status: Status) {
    ui.add_space(8.0);
    ui.heading("Noise control");
    let current = status.noise_control;
    ui.horizontal(|ui| {
        for (mode, label) in [
            (NoiseControl::NoiseCancelling, "ANC"),
            (NoiseControl::Transparency, "Transparency"),
            (NoiseControl::Off, "Off"),
        ] {
            if ui.selectable_label(current == Some(mode), label).clicked() {
                core.headset(Command::SetNoiseControl(mode));
            }
        }
    });
    if current == Some(NoiseControl::Transparency) {
        if let Some(level) = status.transparency {
            if let Some(value) = level_slider(
                ui,
                level.get(),
                TransparencyLevel::MIN..=TransparencyLevel::MAX,
                "Transparency",
            ) {
                core.headset(Command::SetTransparency(TransparencyLevel::saturating(
                    value,
                )));
            }
        }
    }
}

fn microphone_and_sound(ui: &mut egui::Ui, core: &AppCore, settings: Option<Settings>) {
    ui.add_space(8.0);
    ui.heading("Microphone and sound");
    ui.horizontal(|ui| {
        ui.label("Sidetone");
        let current = settings.and_then(|s| s.sidetone);
        for (level, label) in [
            (SidetoneLevel::Off, "Off"),
            (SidetoneLevel::Low, "Low"),
            (SidetoneLevel::Medium, "Medium"),
            (SidetoneLevel::High, "High"),
        ] {
            if ui.selectable_label(current == Some(level), label).clicked() {
                core.headset(Command::SetSidetone(level));
            }
        }
    });
    if let Some(volume) = settings.and_then(|s| s.mic_volume) {
        if let Some(value) = level_slider(
            ui,
            volume.get(),
            MicVolume::MIN..=MicVolume::MAX,
            "Mic volume",
        ) {
            core.headset(Command::SetMicVolume(MicVolume::saturating(value)));
        }
    }
    ui.horizontal(|ui| {
        ui.label("Headset EQ");
        let current = settings.and_then(|s| s.eq_preset);
        for (preset, label) in [
            (EqPreset::Flat, "Flat"),
            (EqPreset::BassBoost, "Bass boost"),
            (EqPreset::Focus, "Focus"),
            (EqPreset::Smiley, "Smiley"),
            (EqPreset::Custom, "Custom"),
        ] {
            if ui
                .selectable_label(current == Some(preset), label)
                .clicked()
            {
                core.headset(Command::SetEqPreset(preset));
            }
        }
    });
}

fn power_and_display(ui: &mut egui::Ui, core: &AppCore, status: Status, headset: &HeadsetState) {
    ui.add_space(8.0);
    ui.heading("Power and display");
    ui.horizontal(|ui| {
        ui.label("Auto-off");
        let current = status.auto_off;
        egui::ComboBox::from_id_salt("auto-off")
            .selected_text(current.map_or("unknown", auto_off_label))
            .show_ui(ui, |ui| {
                for option in [
                    AutoOff::Never,
                    AutoOff::After1Min,
                    AutoOff::After5Min,
                    AutoOff::After10Min,
                    AutoOff::After15Min,
                    AutoOff::After30Min,
                    AutoOff::After60Min,
                ] {
                    if ui
                        .selectable_label(current == Some(option), auto_off_label(option))
                        .clicked()
                    {
                        core.headset(Command::SetAutoOff(option));
                    }
                }
            });
    });
    if let Some(brightness) = headset.display.and_then(|d| d.brightness) {
        if let Some(value) = level_slider(
            ui,
            brightness.get(),
            DisplayBrightness::MIN..=DisplayBrightness::MAX,
            "Base station display",
        ) {
            core.headset(Command::SetDisplayBrightness(
                DisplayBrightness::saturating(value),
            ));
        }
    }
    if let Some(led) = status.mic_led {
        if let Some(value) = level_slider(
            ui,
            led.get(),
            MicLedBrightness::MIN..=MicLedBrightness::MAX,
            "Mic LED",
        ) {
            core.headset(Command::SetMicLedBrightness(MicLedBrightness::saturating(
                value,
            )));
        }
    }
}

/// An integer slider that reports only finished changes (release or click), so dragging does
/// not flood the headset with commands.
fn level_slider(
    ui: &mut egui::Ui,
    value: u8,
    range: std::ops::RangeInclusive<u8>,
    label: &str,
) -> Option<u8> {
    // While dragging, the device still reports the old value; keep the dragged one locally.
    let id = ui.id().with(("level", label));
    let mut edited = ui.data(|d| d.get_temp::<u8>(id)).unwrap_or(value);
    let response = ui.add(egui::Slider::new(&mut edited, range).text(label));
    if response.dragged() {
        ui.data_mut(|d| d.insert_temp(id, edited));
        return None;
    }
    ui.data_mut(|d| d.remove::<u8>(id));
    let finished = response.drag_stopped() || response.changed();
    (finished && edited != value).then_some(edited)
}

const fn charge_text(state: ChargeState) -> &'static str {
    match state {
        ChargeState::Discharging => "on battery",
        ChargeState::Charging => "charging",
        ChargeState::PluggedNotCharging => "plugged in",
        ChargeState::Unknown => "unknown",
    }
}

const fn auto_off_label(option: AutoOff) -> &'static str {
    match option {
        AutoOff::Never => "Never",
        AutoOff::After1Min => "1 min",
        AutoOff::After5Min => "5 min",
        AutoOff::After10Min => "10 min",
        AutoOff::After15Min => "15 min",
        AutoOff::After30Min => "30 min",
        AutoOff::After60Min => "60 min",
    }
}

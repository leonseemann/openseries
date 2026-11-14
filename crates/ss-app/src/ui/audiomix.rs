//! AudioMix page: master switch, output, channels and per-app routing.

use std::collections::BTreeMap;

use eframe::egui;
use ss_core::eq::{BandGain, ParametricEqSettings, PeqBand};
use ss_store::config::{BandConfig, Channel, EqConfig, Loudness};

use ss_store::presets::{PresetData, apply_playback};

use super::{presets, widgets};
use crate::core::{AppCore, Snapshot};

const CHANNELS: [(Channel, &str); 4] = [
    (Channel::Game, "Game"),
    (Channel::Chat, "Chat"),
    (Channel::Media, "Media"),
    (Channel::Aux, "Aux"),
];
/// Largest boost the UI offers (dB), matching Sonar's range.
const MAX_BOOST_DB: f32 = 12.0;
/// Largest per-channel volume boost (dB); our choice, Sonar's range is Unknown.
const MAX_VOLUME_BOOST_DB: f32 = 12.0;
/// Whether the channel DSP controls do anything on this OS.
const DSP: bool = crate::platform::CHANNEL_DSP_SUPPORTED;

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, show_idle: &mut bool) {
    let config = &snapshot.config.audiomix;

    ui.heading("AudioMix");
    let mut enabled = config.enabled;
    if ui
        .checkbox(&mut enabled, "Route app audio through StealSeries")
        .changed()
    {
        core.update_config(|c| c.audiomix.enabled = enabled);
    }
    ui.label(
        egui::RichText::new(
            "Off: StealSeries does not touch system audio at all — use this when another \
             audio tool such as FineTune should handle your apps.",
        )
        .weak(),
    );
    widgets::status_detail(ui, &snapshot.audio.mix);
    if !config.enabled {
        return;
    }
    ui.label(
        egui::RichText::new(format!(
            "Started {} time(s) since launch ({} app(s) routed)",
            snapshot.audio.mix_starts, snapshot.audio.routed_apps
        ))
        .weak(),
    );

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Output");
        if let Some(uid) = widgets::device_combo(
            ui,
            "audiomix-output",
            config.output_device_uid.as_deref(),
            &snapshot.audio.outputs,
        ) {
            core.update_config(|c| c.audiomix.output_device_uid = uid.into_uid());
        }
    });
    backup_devices(ui, core, snapshot);
    chatmix(ui, core, snapshot);

    ui.add_space(8.0);
    ui.separator();
    channels(ui, core, snapshot);
    ui.separator();
    apps(ui, core, snapshot, show_idle);
}

/// Backup outputs and the headset-off trigger (SON-27/28).
fn backup_devices(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.audiomix;
    egui::CollapsingHeader::new("Backup outputs")
        .id_salt("backup-outputs")
        .default_open(!config.output_fallback.is_empty())
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Used in this order when the selected output is missing. Also applies to                      the microphone's backups.",
                )
                .weak(),
            );
            if let Some(list) = widgets::fallback_list(
                ui,
                "output",
                &config.output_fallback,
                &snapshot.audio.outputs,
            ) {
                core.update_config(|c| c.audiomix.output_fallback = list);
            }
            let mut trigger = config.fallback_when_headset_off;
            if ui
                .checkbox(&mut trigger, "Switch to the backups while the headset is turned off")
                .changed()
            {
                core.update_config(|c| c.audiomix.fallback_when_headset_off = trigger);
            }
        });
}

/// Software ChatMix (SON-07): Game ↔ Chat balance, following the headset dial.
fn chatmix(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    ui.horizontal(|ui| {
        ui.label("ChatMix");
        ui.label("Game");
        let mut position = snapshot.chatmix_position;
        let response = ui.add(
            egui::Slider::new(&mut position, -1.0..=1.0)
                .show_value(false)
                .clamping(egui::SliderClamping::Always),
        );
        ui.label("Chat");
        if response.changed() {
            core.set_chatmix(position);
        }
        // The mix follows every drag step; the base station only gets the final position.
        if response.drag_stopped() || (response.changed() && !response.dragged()) {
            core.sync_chatmix_to_headset();
        }
        if ui.small_button("Centre").clicked() {
            core.set_chatmix(0.0);
            core.sync_chatmix_to_headset();
        }
        let balance = ss_dsp::balance_from_slider(snapshot.chatmix_position);
        ui.label(
            egui::RichText::new(format!(
                "Game {} % · Chat {} %",
                balance.game.get(),
                balance.chat.get()
            ))
            .weak(),
        );
    });
}

fn channels(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.audiomix;
    ui.heading("Channels");
    if !DSP {
        ui.label(
            egui::RichText::new(
                "On Windows, channels set the volume of their apps (volume, mute, ChatMix). \
                 EQ, boosts and Smart Volume per channel are available on macOS only: on \
                 Windows they would need a signed virtual audio driver.",
            )
            .weak(),
        );
    }
    ui.horizontal(|ui| {
        ui.label("Master");
        let master = config.master;
        if let Some(volume) = widgets::slider(ui, master.volume * 100.0, 0.0..=100.0, "", " %") {
            core.update_config(|c| c.audiomix.master.volume = volume / 100.0);
        }
        let mut muted = master.muted;
        if ui.checkbox(&mut muted, "Mute").changed() {
            core.update_config(|c| c.audiomix.master.muted = muted);
        }
    });

    for (channel, name) in CHANNELS {
        let settings = config.channels.get(channel);
        egui::CollapsingHeader::new(egui::RichText::new(name).strong())
            .id_salt(("channel", name))
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let fader = settings.fader;
                    if let Some(volume) =
                        widgets::slider(ui, fader.volume * 100.0, 0.0..=100.0, "Volume", " %")
                    {
                        core.update_config(|c| {
                            c.audiomix.channels.get_mut(channel).fader.volume = volume / 100.0;
                        });
                    }
                    let mut muted = fader.muted;
                    if ui.checkbox(&mut muted, "Mute").changed() {
                        core.update_config(|c| {
                            c.audiomix.channels.get_mut(channel).fader.muted = muted;
                        });
                    }
                    if DSP {
                        smart_volume(ui, core, channel, settings.smart_volume);
                    }
                });
                if !DSP {
                    return;
                }
                channel_preset(ui, core, snapshot, channel, name, settings);
                ui.horizontal(|ui| {
                    let boosts = settings.boosts;
                    if let Some(db) =
                        widgets::slider(ui, boosts.bass_db, 0.0..=MAX_BOOST_DB, "Bass", " dB")
                    {
                        core.update_config(|c| {
                            c.audiomix.channels.get_mut(channel).boosts.bass_db = db;
                        });
                    }
                    if let Some(db) =
                        widgets::slider(ui, boosts.treble_db, 0.0..=MAX_BOOST_DB, "Treble", " dB")
                    {
                        core.update_config(|c| {
                            c.audiomix.channels.get_mut(channel).boosts.treble_db = db;
                        });
                    }
                    if let Some(db) =
                        widgets::slider(ui, boosts.clarity_db, 0.0..=MAX_BOOST_DB, "Clarity", " dB")
                    {
                        core.update_config(|c| {
                            c.audiomix.channels.get_mut(channel).boosts.clarity_db = db;
                        });
                    }
                });
                // SON-13 volume boost: extra gain after the DSP strip; the engine's soft clipper
                // on the summed output prevents hard clipping.
                if let Some(db) = widgets::slider(
                    ui,
                    settings.gain_db,
                    0.0..=MAX_VOLUME_BOOST_DB,
                    "Volume boost",
                    " dB",
                ) {
                    core.update_config(|c| c.audiomix.channels.get_mut(channel).gain_db = db);
                }
                if let Some(eq) = eq_editor(ui, ("eq", name), &settings.eq) {
                    core.update_config(|c| c.audiomix.channels.get_mut(channel).eq = eq);
                }
            });
    }
}

/// Preset bar of one channel (playback presets keep the channel's fader).
fn channel_preset(
    ui: &mut egui::Ui,
    core: &AppCore,
    snapshot: &Snapshot,
    channel: Channel,
    name: &str,
    settings: &ss_store::config::ChannelConfig,
) {
    let preset = presets::bar(
        ui,
        core,
        snapshot,
        presets::Kind::Playback,
        ("channel", name),
        || Some(PresetData::playback_of(settings)),
    );
    if let Some(PresetData::Playback(preset)) = preset {
        core.update_config(|c| apply_playback(c.audiomix.channels.get_mut(channel), &preset));
    }
}

fn smart_volume(ui: &mut egui::Ui, core: &AppCore, channel: Channel, current: Option<Loudness>) {
    let label = |value: Option<Loudness>| match value {
        None => "Smart Volume off",
        Some(Loudness::Soft) => "Smart Volume soft",
        Some(Loudness::Balanced) => "Smart Volume balanced",
        Some(Loudness::Extreme) => "Smart Volume extreme",
    };
    let mut selected = current;
    egui::ComboBox::from_id_salt(("smart-volume", label(None), channel_index(channel)))
        .selected_text(label(current))
        .show_ui(ui, |ui| {
            for option in [
                None,
                Some(Loudness::Soft),
                Some(Loudness::Balanced),
                Some(Loudness::Extreme),
            ] {
                ui.selectable_value(&mut selected, option, label(option));
            }
        });
    if selected != current {
        core.update_config(|c| c.audiomix.channels.get_mut(channel).smart_volume = selected);
    }
}

const fn channel_index(channel: Channel) -> usize {
    match channel {
        Channel::Game => 0,
        Channel::Chat => 1,
        Channel::Media => 2,
        Channel::Aux => 3,
    }
}

/// 10-band gain editor on the default band frequencies. Returns the edited EQ.
pub fn eq_editor(ui: &mut egui::Ui, id: impl std::hash::Hash, eq: &EqConfig) -> Option<EqConfig> {
    let mut edited: Option<EqConfig> = None;
    egui::CollapsingHeader::new("Equalizer")
        .id_salt(id)
        .show(ui, |ui| {
            let mut enabled = eq.enabled;
            if ui.checkbox(&mut enabled, "Enabled").changed() {
                edited = Some(EqConfig {
                    enabled,
                    bands: editable_bands(eq),
                });
            }
            ui.horizontal(|ui| {
                let bands = editable_bands(eq);
                for (index, band) in bands.iter().enumerate() {
                    ui.vertical(|ui| {
                        let mut gain = band.gain_db;
                        let response = ui.add(
                            egui::Slider::new(&mut gain, BandGain::MIN..=BandGain::MAX)
                                .vertical()
                                .show_value(false),
                        );
                        ui.label(format!("{:+.0}", band.gain_db));
                        ui.label(frequency_label(band.frequency));
                        if response.changed() {
                            let mut new_bands = bands.clone();
                            new_bands[index].gain_db = gain;
                            edited = Some(EqConfig {
                                enabled: true,
                                bands: new_bands,
                            });
                        }
                    });
                }
            });
            if ui.button("Flat").clicked() {
                edited = Some(EqConfig {
                    enabled: eq.enabled,
                    bands: Vec::new(),
                });
            }
        });
    edited
}

/// The stored bands, or the flat defaults, as ten editable bands.
fn editable_bands(eq: &EqConfig) -> Vec<BandConfig> {
    let domain = eq
        .to_domain()
        .unwrap_or_else(|_| ParametricEqSettings::flat());
    domain.bands.iter().map(band_config).collect()
}

fn band_config(band: &PeqBand) -> BandConfig {
    BandConfig {
        enabled: band.enabled,
        filter: band.filter.sonar_id(),
        frequency: band.frequency.get(),
        gain_db: band.gain.get(),
        q: band.q.get(),
    }
}

fn frequency_label(hz: f32) -> String {
    if hz >= 1000.0 {
        format!("{:.0}k", hz / 1000.0)
    } else {
        format!("{hz:.0}")
    }
}

fn apps(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, show_idle: &mut bool) {
    let assignments = &snapshot.config.audiomix.assignments;
    ui.horizontal(|ui| {
        ui.heading("Apps");
        ui.checkbox(show_idle, "Show apps that are not playing");
    });
    ui.label(egui::RichText::new("Unassigned apps play on Media.").weak());

    // App key → playing (helper processes are folded into their app).
    let mut running: BTreeMap<String, bool> = BTreeMap::new();
    for process in &snapshot.audio.processes {
        if let Some(bundle_id) = &process.bundle_id {
            *running.entry(app_key(bundle_id)).or_default() |= process.playing;
        }
    }
    let mut rows: Vec<(String, bool, bool)> = running
        .iter()
        .filter(|(key, playing)| **playing || *show_idle || assignments.contains_key(*key))
        .map(|(key, playing)| (key.clone(), *playing, true))
        .collect();
    for key in assignments.keys() {
        if !running.contains_key(key) {
            rows.push((key.clone(), false, false));
        }
    }
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    egui::Grid::new("apps").striped(true).show(ui, |ui| {
        for (key, playing, is_running) in rows {
            let text = if playing {
                format!("▶ {key}")
            } else {
                key.clone()
            };
            if is_running {
                ui.label(text);
            } else {
                ui.label(egui::RichText::new(format!("{text} (not running)")).weak());
            }
            let current = assignments.get(&key).copied().unwrap_or(Channel::Media);
            let mut selected = current;
            egui::ComboBox::from_id_salt(("app", key.as_str()))
                .selected_text(channel_name(current))
                .show_ui(ui, |ui| {
                    for (channel, name) in CHANNELS {
                        ui.selectable_value(&mut selected, channel, name);
                    }
                });
            if selected != current {
                core.update_config(|c| {
                    if selected == Channel::Media {
                        c.audiomix.assignments.remove(&key);
                    } else {
                        c.audiomix.assignments.insert(key.clone(), selected);
                    }
                });
            }
            ui.end_row();
        }
    });
}

fn channel_name(channel: Channel) -> &'static str {
    CHANNELS
        .iter()
        .find(|(c, _)| *c == channel)
        .map_or("Media", |(_, name)| name)
}

/// Folds helper processes into their app: `com.google.Chrome.helper` → `com.google.Chrome`,
/// `com.hnc.Discord.helper.Renderer` → `com.hnc.Discord`.
fn app_key(bundle_id: &str) -> String {
    let lower = bundle_id.to_ascii_lowercase();
    lower.find(".helper").map_or_else(
        || bundle_id.to_owned(),
        |index| bundle_id[..index].to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_fold_into_their_app() {
        assert_eq!(app_key("com.google.Chrome.helper"), "com.google.Chrome");
        assert_eq!(
            app_key("com.hnc.Discord.helper.Renderer"),
            "com.hnc.Discord"
        );
        assert_eq!(app_key("com.spotify.client"), "com.spotify.client");
    }

    #[test]
    fn empty_eq_edits_start_from_flat_bands() {
        let bands = editable_bands(&EqConfig::default());
        assert_eq!(bands.len(), 10);
        assert!(bands.iter().all(|b| b.gain_db.abs() < f32::EPSILON));
    }
}

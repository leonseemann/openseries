//! AudioMix page: switch, output, ChatMix, channels and per-app routing
//! (`docs/architecture/ui-design.md` §4).

use std::collections::BTreeMap;

use eframe::egui::{self, RichText};
use ss_core::eq::{BandGain, ParametricEqSettings, PeqBand};
use ss_store::config::{BandConfig, Channel, ChannelConfig, EqConfig, Loudness};
use ss_store::presets::{PresetData, apply_playback};

use super::components::{self, Tone};
use super::{presets, theme, widgets};
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
/// Last name components of bundle ids that say nothing about the app.
const GENERIC_NAME_PARTS: [&str; 7] = [
    "client",
    "app",
    "desktop",
    "mac",
    "osx",
    "xos",
    "application",
];

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, show_idle: &mut bool) {
    let config = &snapshot.config.audiomix;
    components::page_header(
        ui,
        "AudioMix",
        "Sort your apps into Game, Chat, Media and Aux and control each group separately.",
    );
    switch_card(ui, core, snapshot);
    if !config.enabled {
        return;
    }
    components::card(ui, Some("Game/Chat balance"), |ui| {
        chatmix_control(ui, core, snapshot);
    });
    apps_card(ui, core, snapshot, show_idle);
    channels(ui, core, snapshot);
    output_card(ui, core, snapshot);
}

fn switch_card(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.audiomix;
    components::card(ui, None, |ui| {
        let mut enabled = config.enabled;
        let description = if DSP {
            "Routes app audio through OpenSeries. When off, OpenSeries does not touch system \
             audio at all — use that when another audio tool should handle your apps."
        } else {
            "Sets each app's volume by its channel and balances Game and Chat. When off, \
             OpenSeries does not touch app volumes."
        };
        components::row(ui, "AudioMix", Some(description), |ui| {
            if components::switch(ui, &mut enabled) {
                core.update_config(|c| c.audiomix.enabled = enabled);
            }
            let (tone, word) = components::service_tone(&snapshot.audio.mix);
            components::status(ui, tone, word);
        });
        components::service_problem(ui, &snapshot.audio.mix);
    });
}

/// Software ChatMix (SON-07): Game ↔ Chat balance, following the headset dial. Shared with the
/// overview.
pub(super) fn chatmix_control(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let p = theme::palette(ui);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Game").color(p.text));
        let mut position = snapshot.chatmix_position;
        ui.spacing_mut().slider_width = (ui.available_width() - 60.0).max(120.0);
        let response = ui
            .add(
                egui::Slider::new(&mut position, -1.0..=1.0)
                    .show_value(false)
                    .clamping(egui::SliderClamping::Always),
            )
            .on_hover_text("Also follows the dial on your device and the balance hotkeys.");
        ui.label(RichText::new("Chat").color(p.text));
        if response.changed() {
            core.set_chatmix(position);
        }
        // The mix follows every drag step; the base station only gets the final position.
        if response.drag_stopped() || (response.changed() && !response.dragged()) {
            core.sync_chatmix_to_headset();
        }
    });
    ui.horizontal(|ui| {
        let balance = ss_dsp::balance_from_slider(snapshot.chatmix_position);
        ui.label(
            RichText::new(format!(
                "Game {} %  ·  Chat {} %",
                balance.game.get(),
                balance.chat.get()
            ))
            .small()
            .color(p.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Centre").clicked() {
                core.set_chatmix(0.0);
                core.sync_chatmix_to_headset();
            }
        });
    });
}

fn output_card(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.audiomix;
    if !DSP {
        // On Windows apps keep playing on their own device; the output choice does not apply.
        return;
    }
    components::card(ui, Some("Output"), |ui| {
        components::row(ui, "Play through", Some("Usually your headset."), |ui| {
            if let Some(uid) = widgets::device_combo(
                ui,
                "audiomix-output",
                config.output_device_uid.as_deref(),
                &snapshot.audio.outputs,
            ) {
                core.update_config(|c| c.audiomix.output_device_uid = uid.into_uid());
            }
        });
        egui::CollapsingHeader::new("Backup outputs")
            .id_salt("backup-outputs")
            .default_open(!config.output_fallback.is_empty())
            .show(ui, |ui| {
                let p = theme::palette(ui);
                ui.label(
                    RichText::new(
                        "Used in this order when the output above is missing (also for the \
                         microphone's backups).",
                    )
                    .small()
                    .color(p.text_secondary),
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
                components::row(ui, "Use backups while the headset is off", None, |ui| {
                    if components::switch(ui, &mut trigger) {
                        core.update_config(|c| c.audiomix.fallback_when_headset_off = trigger);
                    }
                });
            });
    });
}

fn channels(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.audiomix;
    components::card(ui, Some("Channels"), |ui| {
        let p = theme::palette(ui);
        if !DSP {
            ui.label(
                RichText::new(
                    "On Windows, channels set the volume of their apps. Equalizer, boosts and \
                     Smart Volume per channel are available on macOS.",
                )
                .small()
                .color(p.text_secondary),
            );
            ui.add_space(4.0);
        }
        let master = config.master;
        fader_row(
            ui,
            "All channels",
            master.volume,
            master.muted,
            |volume, muted| {
                core.update_config(|c| {
                    c.audiomix.master.volume = volume;
                    c.audiomix.master.muted = muted;
                });
            },
        );
        for (channel, name) in CHANNELS {
            components::divider(ui);
            let settings = config.channels.get(channel);
            fader_row(
                ui,
                name,
                settings.fader.volume,
                settings.fader.muted,
                |volume, muted| {
                    core.update_config(|c| {
                        let fader = &mut c.audiomix.channels.get_mut(channel).fader;
                        fader.volume = volume;
                        fader.muted = muted;
                    });
                },
            );
            if DSP {
                egui::CollapsingHeader::new(RichText::new(format!("{name} sound")).small())
                    .id_salt(("channel-sound", name))
                    .show(ui, |ui| {
                        channel_sound(ui, core, snapshot, channel, name, settings);
                    });
            }
        }
    });
}

/// Volume slider plus a mute switch.
fn fader_row(
    ui: &mut egui::Ui,
    label: &str,
    volume: f32,
    muted: bool,
    mut apply: impl FnMut(f32, bool),
) {
    components::row(ui, label, None, |ui| {
        let mut on = !muted;
        if components::switch(ui, &mut on) {
            apply(volume, !on);
        }
        ui.spacing_mut().slider_width = 200.0;
        if let Some(new) = components::slider(ui, volume * 100.0, 0.0..=100.0, " %") {
            apply(new / 100.0, muted);
        }
    });
}

/// The macOS-only DSP of one channel: Smart Volume, boosts, EQ, presets.
fn channel_sound(
    ui: &mut egui::Ui,
    core: &AppCore,
    snapshot: &Snapshot,
    channel: Channel,
    name: &str,
    settings: &ChannelConfig,
) {
    components::row(
        ui,
        "Smart Volume",
        Some("Evens out loud and quiet passages."),
        |ui| {
            if let Some(choice) = components::segmented(
                ui,
                Some(settings.smart_volume),
                &[
                    (None, "Off"),
                    (Some(Loudness::Soft), "Soft"),
                    (Some(Loudness::Balanced), "Balanced"),
                    (Some(Loudness::Extreme), "Strong"),
                ],
            ) {
                core.update_config(|c| c.audiomix.channels.get_mut(channel).smart_volume = choice);
            }
        },
    );
    let boosts = settings.boosts;
    for (label, value, set) in [
        (
            "Bass",
            boosts.bass_db,
            (|c: &mut ChannelConfig, v| c.boosts.bass_db = v) as fn(&mut ChannelConfig, f32),
        ),
        ("Treble", boosts.treble_db, |c, v| c.boosts.treble_db = v),
        ("Voice clarity", boosts.clarity_db, |c, v| {
            c.boosts.clarity_db = v;
        }),
        // SON-13 volume boost: extra gain after the DSP strip; the engine's soft clipper
        // on the summed output prevents hard clipping.
        ("Volume boost", settings.gain_db, |c, v| c.gain_db = v),
    ] {
        let max = if label == "Volume boost" {
            MAX_VOLUME_BOOST_DB
        } else {
            MAX_BOOST_DB
        };
        components::row(ui, label, None, |ui| {
            if let Some(db) = components::slider(ui, value, 0.0..=max, " dB") {
                core.update_config(|c| set(c.audiomix.channels.get_mut(channel), db));
            }
        });
    }
    if let Some(eq) = eq_editor(ui, ("eq", name), &settings.eq) {
        core.update_config(|c| c.audiomix.channels.get_mut(channel).eq = eq);
    }
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

/// 10-band gain editor on the default band frequencies. Returns the edited EQ.
pub fn eq_editor(ui: &mut egui::Ui, id: impl std::hash::Hash, eq: &EqConfig) -> Option<EqConfig> {
    let mut edited: Option<EqConfig> = None;
    egui::CollapsingHeader::new("Equalizer")
        .id_salt(id)
        .show(ui, |ui| {
            let p = theme::palette(ui);
            let mut enabled = eq.enabled;
            components::row(ui, "Equalizer", None, |ui| {
                if ui.button("Flat").clicked() {
                    edited = Some(EqConfig {
                        enabled: eq.enabled,
                        bands: Vec::new(),
                    });
                }
                if components::switch(ui, &mut enabled) {
                    edited = Some(EqConfig {
                        enabled,
                        bands: editable_bands(eq),
                    });
                }
            });
            ui.horizontal(|ui| {
                let bands = editable_bands(eq);
                let band_width = (ui.available_width() / 10.0).floor();
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.spacing_mut().slider_width = 130.0;
                for (index, band) in bands.iter().enumerate() {
                    ui.allocate_ui_with_layout(
                        egui::vec2(band_width, 190.0),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            let mut gain = band.gain_db;
                            let response = ui.add(
                                egui::Slider::new(&mut gain, BandGain::MIN..=BandGain::MAX)
                                    .vertical()
                                    .show_value(false),
                            );
                            ui.label(
                                RichText::new(format!("{:+.0}", band.gain_db))
                                    .small()
                                    .color(p.text),
                            );
                            ui.label(
                                RichText::new(frequency_label(band.frequency))
                                    .small()
                                    .color(p.text_secondary),
                            );
                            if response.changed() {
                                let mut new_bands = bands.clone();
                                new_bands[index].gain_db = gain;
                                edited = Some(EqConfig {
                                    enabled: true,
                                    bands: new_bands,
                                });
                            }
                        },
                    );
                }
            });
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

fn apps_card(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, show_idle: &mut bool) {
    let assignments = &snapshot.config.audiomix.assignments;
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

    components::card(ui, Some("Apps"), |ui| {
        let p = theme::palette(ui);
        components::row(
            ui,
            "Show apps that are not playing",
            Some("Apps you have not assigned play on Media."),
            |ui| {
                components::switch(ui, show_idle);
            },
        );
        if rows.is_empty() {
            components::divider(ui);
            ui.label(
                RichText::new(
                    "No app is playing sound right now. Start some audio and it appears here.",
                )
                .color(p.text_secondary),
            );
        }
        for (key, playing, is_running) in rows {
            components::divider(ui);
            let current = assignments.get(&key).copied().unwrap_or(Channel::Media);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.4).max(160.0));
                    ui.horizontal(|ui| {
                        let (tone, word) = if playing {
                            (Tone::Good, "Playing")
                        } else if is_running {
                            (Tone::Neutral, "Idle")
                        } else {
                            (Tone::Neutral, "Not running")
                        };
                        ui.label(RichText::new(display_name(&key)).color(p.text));
                        components::status(ui, tone, word);
                    });
                    ui.label(RichText::new(&key).small().color(p.text_secondary));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(selected) = components::segmented(
                        ui,
                        Some(current),
                        &[
                            (Channel::Aux, "Aux"),
                            (Channel::Media, "Media"),
                            (Channel::Chat, "Chat"),
                            (Channel::Game, "Game"),
                        ],
                    ) {
                        core.update_config(|c| {
                            if selected == Channel::Media {
                                c.audiomix.assignments.remove(&key);
                            } else {
                                c.audiomix.assignments.insert(key.clone(), selected);
                            }
                        });
                    }
                });
            });
        }
    });
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

/// A readable name for an app key: `chrome.exe` → "Chrome", `com.google.Chrome` → "Chrome",
/// `com.spotify.client` → "Spotify".
fn display_name(key: &str) -> String {
    let stem = key
        .strip_suffix(".exe")
        .or_else(|| key.strip_suffix(".EXE"))
        .unwrap_or(key);
    let name = if stem.contains('.') {
        let parts: Vec<&str> = stem.split('.').collect();
        parts
            .iter()
            .rev()
            .find(|part| {
                !part.is_empty()
                    && !GENERIC_NAME_PARTS.contains(&part.to_ascii_lowercase().as_str())
            })
            .copied()
            .unwrap_or(stem)
    } else {
        stem
    };
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
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
    fn app_names_are_readable() {
        assert_eq!(display_name("chrome.exe"), "Chrome");
        assert_eq!(display_name("com.google.Chrome"), "Chrome");
        assert_eq!(display_name("com.spotify.client"), "Spotify");
        assert_eq!(display_name("us.zoom.xos"), "Zoom");
        assert_eq!(display_name("discord.exe"), "Discord");
    }

    #[test]
    fn empty_eq_edits_start_from_flat_bands() {
        let bands = editable_bands(&EqConfig::default());
        assert_eq!(bands.len(), 10);
        assert!(bands.iter().all(|b| b.gain_db.abs() < f32::EPSILON));
    }
}

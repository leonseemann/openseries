//! AudioMix page: switch, output, ChatMix, channels and per-app routing
//! (`docs/architecture/ui-design.md` §4).

use std::collections::BTreeMap;

use eframe::egui::{self, RichText};
use ss_core::eq::{BandGain, ParametricEqSettings, PeqBand};
use ss_store::config::{AudioMixConfig, BandConfig, Channel, ChannelConfig, EqConfig, Loudness};
use ss_store::presets::{PresetData, apply_playback};

use super::components::{self, Tone};
use super::{presets, theme, widgets};
use crate::audio::catalogue::{Source, classify};
use crate::audio::plan::ProcessInfo;
use crate::core::{AppCore, Snapshot};
use crate::i18n::tr;

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
    components::page_header(ui, "AudioMix", &tr!("mix-subtitle"));
    switch_card(ui, core, snapshot);
    if !config.enabled {
        return;
    }
    components::card(ui, Some(&tr!("mix-balance-title")), |ui| {
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
            tr!("mix-switch-hint-dsp")
        } else {
            tr!("mix-switch-hint-volumes")
        };
        components::row(ui, "AudioMix", Some(&description), |ui| {
            if components::switch(ui, &mut enabled) {
                core.update_config(|c| c.audiomix.enabled = enabled);
            }
            let (tone, word) = components::service_tone(&snapshot.audio.mix);
            components::status(ui, tone, &word);
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
            .on_hover_text(tr!("mix-balance-hint"));
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
            RichText::new(tr!(
                "mix-balance-split",
                game = balance.game.get(),
                chat = balance.chat.get()
            ))
            .small()
            .color(p.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button(tr!("mix-balance-centre")).clicked() {
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
    components::card(ui, Some(&tr!("mix-output-title")), |ui| {
        let hint = tr!("mix-play-through-hint");
        components::row(ui, &tr!("mix-play-through"), Some(&hint), |ui| {
            if let Some(uid) = widgets::device_combo(
                ui,
                "audiomix-output",
                config.output_device_uid.as_deref(),
                &snapshot.audio.outputs,
            ) {
                core.update_config(|c| c.audiomix.output_device_uid = uid.into_uid());
            }
        });
        egui::CollapsingHeader::new(tr!("mix-backup-outputs"))
            .id_salt("backup-outputs")
            .default_open(!config.output_fallback.is_empty())
            .show(ui, |ui| {
                let p = theme::palette(ui);
                ui.label(
                    RichText::new(tr!("mix-backup-outputs-hint"))
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
                components::row(ui, &tr!("mix-backup-when-headset-off"), None, |ui| {
                    if components::switch(ui, &mut trigger) {
                        core.update_config(|c| c.audiomix.fallback_when_headset_off = trigger);
                    }
                });
            });
    });
}

fn channels(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.audiomix;
    components::card(ui, Some(&tr!("mix-channels-title")), |ui| {
        let p = theme::palette(ui);
        if !DSP {
            ui.label(
                RichText::new(tr!("mix-channels-windows-hint"))
                    .small()
                    .color(p.text_secondary),
            );
            ui.add_space(4.0);
        }
        let master = config.master;
        fader_row(
            ui,
            &tr!("mix-all-channels"),
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
                egui::CollapsingHeader::new(
                    RichText::new(tr!("mix-channel-sound", channel = name)).small(),
                )
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
    let (off, soft, balanced, strong) = (
        tr!("mix-smart-volume-off"),
        tr!("mix-smart-volume-soft"),
        tr!("mix-smart-volume-balanced"),
        tr!("mix-smart-volume-strong"),
    );
    components::row(
        ui,
        "Smart Volume",
        Some(&tr!("mix-smart-volume-hint")),
        |ui| {
            if let Some(choice) = components::segmented(
                ui,
                Some(settings.smart_volume),
                &[
                    (None, off.as_str()),
                    (Some(Loudness::Soft), soft.as_str()),
                    (Some(Loudness::Balanced), balanced.as_str()),
                    (Some(Loudness::Extreme), strong.as_str()),
                ],
            ) {
                core.update_config(|c| c.audiomix.channels.get_mut(channel).smart_volume = choice);
            }
        },
    );
    let boosts = settings.boosts;
    for (label, value, max, set) in [
        (
            tr!("mix-boost-bass"),
            boosts.bass_db,
            MAX_BOOST_DB,
            (|c: &mut ChannelConfig, v| c.boosts.bass_db = v) as fn(&mut ChannelConfig, f32),
        ),
        (
            tr!("mix-boost-treble"),
            boosts.treble_db,
            MAX_BOOST_DB,
            |c, v| {
                c.boosts.treble_db = v;
            },
        ),
        (
            tr!("mix-boost-clarity"),
            boosts.clarity_db,
            MAX_BOOST_DB,
            |c, v| {
                c.boosts.clarity_db = v;
            },
        ),
        // SON-13 volume boost: extra gain after the DSP strip; the engine's soft clipper
        // on the summed output prevents hard clipping.
        (
            tr!("mix-boost-volume"),
            settings.gain_db,
            MAX_VOLUME_BOOST_DB,
            |c, v| {
                c.gain_db = v;
            },
        ),
    ] {
        components::row(ui, &label, None, |ui| {
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
    egui::CollapsingHeader::new(tr!("mix-eq"))
        .id_salt(id)
        .show(ui, |ui| {
            let p = theme::palette(ui);
            let mut enabled = eq.enabled;
            components::row(ui, &tr!("mix-eq"), None, |ui| {
                if ui.button(tr!("mix-eq-flat")).clicked() {
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

    components::card(ui, Some(&tr!("mix-apps-title")), |ui| {
        let p = theme::palette(ui);
        components::row(
            ui,
            &tr!("mix-show-idle"),
            Some(&tr!("mix-show-idle-hint")),
            |ui| {
                components::switch(ui, show_idle);
            },
        );
        if rows.is_empty() {
            components::divider(ui);
            ui.label(RichText::new(tr!("mix-no-apps")).color(p.text_secondary));
        }
        for (key, playing, is_running) in rows {
            components::divider(ui);
            let process = representative(snapshot, &key);
            let (current, source) = effective(&snapshot.config.audiomix, &process);
            let automatic = automatic(&snapshot.config.audiomix, &process);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.4).max(160.0));
                    ui.horizontal(|ui| {
                        let (tone, word) = if playing {
                            (Tone::Good, tr!("mix-app-playing"))
                        } else if is_running {
                            (Tone::Neutral, tr!("mix-app-idle"))
                        } else {
                            (Tone::Neutral, tr!("mix-app-not-running"))
                        };
                        ui.label(RichText::new(display_name(&key)).color(p.text));
                        components::status(ui, tone, &word);
                    });
                    let origin = source.map_or_else(|| tr!("mix-app-unassigned"), Source::label);
                    ui.label(
                        RichText::new(format!("{key} · {origin}"))
                            .small()
                            .color(p.text_secondary),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    channel_controls(ui, core, &key, current, automatic, source);
                });
            });
        }
    });
}

/// Channel choice of one app plus "Reset" for the user's own choices.
fn channel_controls(
    ui: &mut egui::Ui,
    core: &AppCore,
    key: &str,
    current: Channel,
    automatic: Channel,
    source: Option<Source>,
) {
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
        // Picking the automatic channel removes the own assignment; anything else (also Media)
        // is stored as the user's choice.
        core.update_config(|c| {
            if selected == automatic {
                c.audiomix.assignments.remove(key);
            } else {
                c.audiomix.assignments.insert(key.to_owned(), selected);
            }
        });
    }
    // Back to the default (catalogue, game detection or Media); only for the user's choices.
    if source == Some(Source::Own)
        && ui
            .small_button(tr!("mix-reset"))
            .on_hover_text(tr!("mix-reset-hint", channel = channel_name(automatic)))
            .clicked()
    {
        core.update_config(|c| {
            c.audiomix.assignments.remove(key);
        });
    }
}

/// A process standing for the app `key` (its path and category drive game detection).
fn representative(snapshot: &Snapshot, key: &str) -> ProcessInfo {
    snapshot
        .audio
        .processes
        .iter()
        .find(|p| p.bundle_id.as_deref().is_some_and(|id| app_key(id) == key))
        .cloned()
        .map_or_else(
            || ProcessInfo {
                pid: 0,
                bundle_id: Some(key.to_owned()),
                playing: false,
                path: None,
                category: None,
            },
            |mut p| {
                p.bundle_id = Some(key.to_owned());
                p
            },
        )
}

/// The channel the app plays on and where that comes from (`None` = not assigned).
fn effective(config: &AudioMixConfig, process: &ProcessInfo) -> (Channel, Option<Source>) {
    classify(config, process).map_or((Channel::Media, None), |(channel, source)| {
        (Channel::from(channel), Some(source))
    })
}

/// User-facing name of a channel.
const fn channel_name(channel: Channel) -> &'static str {
    match channel {
        Channel::Game => "Game",
        Channel::Chat => "Chat",
        Channel::Media => "Media",
        Channel::Aux => "Aux",
    }
}

/// The channel without the user's own assignment (catalogue or detection, else Media).
fn automatic(config: &AudioMixConfig, process: &ProcessInfo) -> Channel {
    let without_own = AudioMixConfig {
        assignments: std::collections::BTreeMap::new(),
        ..config.clone()
    };
    effective(&without_own, process).0
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
    if let Some(name) = crate::audio::catalogue::display_name(key) {
        return name.to_owned();
    }
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
        // Catalogue names first …
        assert_eq!(display_name("chrome.exe"), "Google Chrome");
        assert_eq!(display_name("com.spotify.client"), "Spotify");
        assert_eq!(display_name("us.zoom.xos"), "Zoom");
        assert_eq!(display_name("discord.exe"), "Discord");
        // … otherwise derived from the id.
        assert_eq!(display_name("com.acme.Mixer"), "Mixer");
        assert_eq!(display_name("tool.exe"), "Tool");
    }

    #[test]
    fn empty_eq_edits_start_from_flat_bands() {
        let bands = editable_bands(&EqConfig::default());
        assert_eq!(bands.len(), 10);
        assert!(bands.iter().all(|b| b.gain_db.abs() < f32::EPSILON));
    }
}

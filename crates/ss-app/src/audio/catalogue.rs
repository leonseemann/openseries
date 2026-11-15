//! Automatic AudioMix channel assignment (`docs/architecture/app.md` §3.2): own assignment →
//! built-in catalogue (`data/app-catalogue.toml`) → game detection → none (plays on Media).
//!
//! Pure functions on [`ProcessInfo`], so the same rules run and are tested on macOS and Windows.

use std::sync::LazyLock;

use serde::Deserialize;
use ss_core::mixer::ChannelId;
use ss_store::config::{AudioMixConfig, Channel};

use super::plan::ProcessInfo;

/// The catalogue file, compiled into the app.
const CATALOGUE_TOML: &str = include_str!("../../data/app-catalogue.toml");

/// Folder parts of game libraries (lower case, `/` separators): an executable below one of them
/// is a game.
const GAME_FOLDERS: &[&str] = &[
    "/steamapps/common/",
    "/epic games/",
    "/gog galaxy/games/",
    "/gog games/",
    "/ea games/",
    "/ubisoft game launcher/games/",
    "/xboxgames/",
    "/riot games/",
];
/// Launchers inside game library folders; they are not games.
const LAUNCHER_FOLDERS: &[&str] = &["/epic games/launcher/", "/riot games/riot client/"];
/// `LSApplicationCategoryType` of games on macOS (also `public.app-category.*-games`).
const GAMES_CATEGORY: &str = "public.app-category.games";
/// Prefix of the macOS app categories.
const CATEGORY_PREFIX: &str = "public.app-category.";
/// Suffix of the game sub-categories (`public.app-category.action-games` …).
const GAMES_SUFFIX: &str = "-games";

/// Where a channel assignment comes from (shown in the AudioMix list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Set by the user.
    Own,
    /// The built-in catalogue.
    Catalogue,
    /// The executable lies in a game library folder.
    GameLibrary,
    /// The app declares a games category (macOS).
    GamesCategory,
}

impl Source {
    /// Short label for the UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Own => "your choice",
            Self::Catalogue => "built-in list",
            Self::GameLibrary => "game library",
            Self::GamesCategory => "games category",
        }
    }
}

/// One program of the catalogue.
#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    /// Display name.
    pub name: String,
    /// Suggested channel.
    pub channel: Channel,
    /// macOS bundle-id prefixes.
    #[serde(default)]
    pub macos: Vec<String>,
    /// Windows executable names (case-insensitive).
    #[serde(default)]
    pub windows: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CatalogueFile {
    #[serde(default)]
    app: Vec<Entry>,
}

/// The parsed built-in catalogue (empty if the file were broken; a test guarantees it is not).
static CATALOGUE: LazyLock<Vec<Entry>> =
    LazyLock::new(|| parse(CATALOGUE_TOML).unwrap_or_default());

fn parse(text: &str) -> Result<Vec<Entry>, toml::de::Error> {
    toml::from_str::<CatalogueFile>(text).map(|file| file.app)
}

/// The channel `process` plays on and why; `None` = not assigned (plays on Media, not routed).
#[must_use]
pub fn classify(config: &AudioMixConfig, process: &ProcessInfo) -> Option<(ChannelId, Source)> {
    let id = process.bundle_id.as_deref()?;
    if let Some(channel) = own_assignment(config, id) {
        return Some((channel, Source::Own));
    }
    if let Some(entry) = catalogue_entry(&CATALOGUE, id) {
        return Some((entry.channel.into(), Source::Catalogue));
    }
    if process.path.as_deref().is_some_and(in_game_library) {
        return Some((ChannelId::Game, Source::GameLibrary));
    }
    if process.category.as_deref().is_some_and(is_games_category) {
        return Some((ChannelId::Game, Source::GamesCategory));
    }
    None
}

/// Channel of the longest own assignment that is a prefix of `id` (helper processes such as
/// `com.google.Chrome.helper` follow their app; Windows names match whole).
fn own_assignment(config: &AudioMixConfig, id: &str) -> Option<ChannelId> {
    config
        .assignments
        .iter()
        .filter(|(prefix, _)| starts_with_ignore_case(id, prefix))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, &channel)| channel.into())
}

/// The catalogue entry for `id`: a Windows executable name (whole, case-insensitive) or the
/// longest matching macOS bundle-id prefix.
fn catalogue_entry<'a>(catalogue: &'a [Entry], id: &str) -> Option<&'a Entry> {
    if let Some(entry) = catalogue
        .iter()
        .find(|e| e.windows.iter().any(|exe| exe.eq_ignore_ascii_case(id)))
    {
        return Some(entry);
    }
    catalogue
        .iter()
        .flat_map(|e| e.macos.iter().map(move |prefix| (prefix, e)))
        .filter(|(prefix, _)| id.starts_with(prefix.as_str()))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, e)| e)
}

/// The catalogue's display name for `id`, if listed.
#[must_use]
pub fn display_name(id: &str) -> Option<&'static str> {
    catalogue_entry(&CATALOGUE, id).map(|e| e.name.as_str())
}

fn in_game_library(path: &str) -> bool {
    let path = path.replace('\\', "/").to_lowercase();
    GAME_FOLDERS.iter().any(|folder| path.contains(folder))
        && !LAUNCHER_FOLDERS.iter().any(|folder| path.contains(folder))
}

fn is_games_category(category: &str) -> bool {
    category == GAMES_CATEGORY
        || (category.starts_with(CATEGORY_PREFIX) && category.ends_with(GAMES_SUFFIX))
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len()
        && text
            .get(..prefix.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn process(id: &str, path: Option<&str>, category: Option<&str>) -> ProcessInfo {
        ProcessInfo {
            pid: 1,
            bundle_id: Some(id.to_owned()),
            playing: true,
            path: path.map(str::to_owned),
            category: category.map(str::to_owned),
        }
    }

    #[test]
    fn built_in_catalogue_is_valid() {
        let entries = parse(CATALOGUE_TOML).expect("app-catalogue.toml must parse");
        assert!(entries.len() > 20);
        let mut seen = HashSet::new();
        for entry in &entries {
            assert!(
                !entry.macos.is_empty() || !entry.windows.is_empty(),
                "{} has no ids",
                entry.name
            );
            for id in entry.macos.iter().chain(&entry.windows) {
                assert!(
                    seen.insert(id.to_lowercase()),
                    "{id} is listed twice ({})",
                    entry.name
                );
            }
            for exe in &entry.windows {
                assert!(
                    exe.to_lowercase().ends_with(".exe"),
                    "{exe} is not an executable name"
                );
            }
        }
    }

    #[test]
    fn catalogue_matches_on_both_systems() {
        let config = AudioMixConfig::default();
        assert_eq!(
            classify(&config, &process("Discord.exe", None, None)),
            Some((ChannelId::Chat, Source::Catalogue))
        );
        assert_eq!(
            classify(&config, &process("com.hnc.Discord.helper", None, None)),
            Some((ChannelId::Chat, Source::Catalogue))
        );
        assert_eq!(display_name("spotify.exe"), Some("Spotify"));
    }

    #[test]
    fn own_assignment_beats_the_catalogue() {
        let mut config = AudioMixConfig::default();
        config
            .assignments
            .insert("discord.exe".to_owned(), Channel::Aux);
        assert_eq!(
            classify(&config, &process("Discord.exe", None, None)),
            Some((ChannelId::Aux, Source::Own))
        );
    }

    #[test]
    fn games_are_detected_by_folder_and_category() {
        let config = AudioMixConfig::default();
        let steam = r"D:\SteamLibrary\steamapps\common\Counter-Strike Global Offensive\game\bin\win64\cs2.exe";
        assert_eq!(
            classify(&config, &process("cs2.exe", Some(steam), None)),
            Some((ChannelId::Game, Source::GameLibrary))
        );
        let launcher = r"C:\Program Files (x86)\Epic Games\Launcher\Portal\Binaries\Win64\EpicGamesLauncher.exe";
        assert_eq!(
            classify(
                &config,
                &process("epicgameslauncher.exe", Some(launcher), None)
            ),
            None
        );
        assert_eq!(
            classify(
                &config,
                &process(
                    "com.example.game",
                    Some("/Applications/Game.app/Contents/MacOS/Game"),
                    Some("public.app-category.action-games")
                )
            ),
            Some((ChannelId::Game, Source::GamesCategory))
        );
        assert_eq!(
            classify(
                &config,
                &process(
                    "com.example.notes",
                    None,
                    Some("public.app-category.productivity")
                )
            ),
            None
        );
    }

    #[test]
    fn unknown_apps_stay_unassigned() {
        let config = AudioMixConfig::default();
        assert_eq!(
            classify(
                &config,
                &process("some-tool.exe", Some(r"C:\Tools\some-tool.exe"), None)
            ),
            None
        );
    }
}

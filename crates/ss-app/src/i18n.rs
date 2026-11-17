//! UI translations (`docs/architecture/i18n.md`).
//!
//! Every language is a Fluent file `i18n/<language>/openseries.ftl`: the files in the crate are
//! embedded at build time, files in `<config dir>/i18n/` are read at start-up and override or add
//! languages. English is the fallback for every message a translation lacks.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, OnceLock};

use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed::{DesktopLanguageRequester, I18nAssets, LanguageLoader};
use rust_embed::RustEmbed;
use unic_langid::LanguageIdentifier;

/// Looks up a message: `tr!("settings-title")`, `tr!("update-available", version = v)`.
/// Message ids and argument names are checked against the English file at compile time.
macro_rules! tr {
    ($($args:tt)*) => {
        i18n_embed_fl::fl!($crate::i18n::LOADER, $($args)*)
    };
}
pub(crate) use tr;

/// Fluent domain: the file name of every language, `openseries.ftl` (also in `i18n.toml`).
const DOMAIN: &str = "openseries";
/// Language of the source strings and fallback for missing messages.
const FALLBACK: &str = "en";
/// Folder under the config directory with user-provided translations.
const USER_DIR: &str = "i18n";
/// The message every translation defines with its own name, e.g. "Deutsch".
const LANGUAGE_NAME: &str = "language-name";

/// The translations built into the app.
#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Builtin;

/// The user's translation folder, set once by [`init`].
static USER_TRANSLATIONS: OnceLock<PathBuf> = OnceLock::new();
/// The language preference last applied, to skip redundant reloads; `None` before the first.
static APPLIED: Mutex<Option<Preference>> = Mutex::new(None);

/// A language setting: `tag` from Settings, or `None` for the system language.
#[derive(Debug, PartialEq, Eq)]
struct Preference {
    tag: Option<String>,
}

/// The loader used by [`tr!`]; English until [`apply`] selects a language.
pub static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader = FluentLanguageLoader::new(DOMAIN, fallback());
    if let Err(error) = loader.load_fallback_language(&Assets) {
        eprintln!("warning: English texts unavailable: {error}");
    }
    no_isolation_marks(&loader);
    loader
});

/// A language offered in Settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    /// BCP 47 tag as stored in the config, e.g. `"de"`.
    pub tag: String,
    /// The language's name in itself, e.g. "Deutsch".
    pub name: String,
}

/// Registers the user translation folder (`<config dir>/i18n`). Call before the first lookup.
pub fn init(config_dir: &Path) {
    let _ = USER_TRANSLATIONS.set(config_dir.join(USER_DIR));
}

/// Switches the UI language: `Some(tag)` from Settings, `None` for the system language. Returns
/// whether the language changed (callers refresh texts they cache, such as the tray menu).
pub fn apply(preference: Option<&str>) -> bool {
    let mut applied = APPLIED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let wanted = Preference {
        tag: preference.map(str::to_owned),
    };
    if applied.as_ref() == Some(&wanted) {
        return false;
    }
    let requested: Vec<LanguageIdentifier> = match preference.and_then(|tag| tag.parse().ok()) {
        Some(language) => vec![language],
        None => DesktopLanguageRequester::requested_languages(),
    };
    if let Err(error) = i18n_embed::select(&*LOADER, &Assets, &requested) {
        eprintln!("warning: could not load translations for {requested:?}: {error}");
    }
    no_isolation_marks(&LOADER);
    *applied = Some(wanted);
    true
}

/// The language currently shown (its tag).
#[must_use]
pub fn current() -> String {
    LOADER.current_language().to_string()
}

/// The languages to offer, sorted by tag, each named in itself.
#[must_use]
pub fn available() -> Vec<Language> {
    let mut tags = LOADER
        .available_languages(&Assets)
        .unwrap_or_else(|_| vec![fallback()]);
    tags.sort_by_key(ToString::to_string);
    tags.dedup();
    let names = FluentLanguageLoader::new(DOMAIN, fallback());
    if names.load_languages(&Assets, &tags).is_err() {
        // A broken file must not hide the others; their tags still work as names.
        return tags
            .iter()
            .map(|tag| language(tag, tag.to_string()))
            .collect();
    }
    tags.iter()
        .map(|tag| language(tag, names.select_languages(&[tag]).get(LANGUAGE_NAME)))
        .collect()
}

/// A message looked up by a run-time id, e.g. a device option (`device-option-anc`); `None` if no
/// translation defines it, so callers fall back to the device driver's English text.
#[must_use]
pub fn lookup(id: &str) -> Option<String> {
    LOADER.has(id).then(|| LOADER.get(id))
}

/// Kebab case of an identifier or label, for building message ids: `NoiseControl` → `noise-control`, `bass_boost` →
/// `bass-boost`, `base station` → `base-station`.
pub fn id_part(text: &str) -> String {
    let mut id = String::with_capacity(text.len() + 4);
    let mut after_lower = false;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && after_lower {
                id.push('-');
            }
            id.push(c.to_ascii_lowercase());
            after_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else {
            if !id.is_empty() && !id.ends_with('-') {
                id.push('-');
            }
            after_lower = false;
        }
    }
    id.trim_end_matches('-').to_owned()
}

/// A device hub's name ("base station") in the UI language: the message `device-hub-<name>`, or
/// the driver's English `name`.
#[must_use]
pub fn hub_name(name: &str) -> String {
    lookup(&format!("device-hub-{}", id_part(name))).unwrap_or_else(|| name.to_owned())
}

fn language(tag: &LanguageIdentifier, name: String) -> Language {
    Language {
        tag: tag.to_string(),
        name,
    }
}

/// Turns off the Unicode isolation marks Fluent puts around arguments (egui renders them as
/// boxes). The setting belongs to the loaded bundles, so it is repeated after every load.
fn no_isolation_marks(loader: &FluentLanguageLoader) {
    loader.set_use_isolating(false);
}

fn fallback() -> LanguageIdentifier {
    // FALLBACK is a valid tag (unit tested), so parsing only fails if the constant is edited.
    FALLBACK.parse().unwrap_or_default()
}

/// Embedded translations plus the user's folder; a user file comes first, so its messages win.
struct Assets;

impl I18nAssets for Assets {
    fn get_files(&self, file_path: &str) -> Vec<Cow<'_, [u8]>> {
        let user = USER_TRANSLATIONS
            .get()
            .and_then(|dir| std::fs::read(dir.join(file_path)).ok())
            .map(Cow::Owned);
        let builtin = Builtin::get(file_path).map(|file| file.data);
        user.into_iter().chain(builtin).collect()
    }

    fn filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        let builtin = Builtin::iter().map(Cow::into_owned);
        Box::new(builtin.chain(user_files()))
    }
}

/// `<language>/openseries.ftl` for every folder in the user's translation folder whose name is a
/// valid language tag and that holds the file.
fn user_files() -> Vec<String> {
    let Some(dir) = USER_TRANSLATIONS.get() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let file_name = format!("{DOMAIN}.ftl");
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join(&file_name).is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|tag| tag.parse::<LanguageIdentifier>().is_ok())
        .map(|tag| format!("{tag}/{file_name}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use fluent_syntax::ast::Entry;

    use super::*;

    /// Message ids (with their attributes) of one embedded language.
    fn message_ids(language: &str) -> BTreeSet<String> {
        let file = Builtin::get(&format!("{language}/{DOMAIN}.ftl"))
            .unwrap_or_else(|| panic!("{language} has no {DOMAIN}.ftl"));
        let source = String::from_utf8(file.data.into_owned()).expect("UTF-8");
        let resource = fluent_syntax::parser::parse(source.as_str())
            .unwrap_or_else(|(_, errors)| panic!("{language}: syntax errors {errors:?}"));
        resource
            .body
            .iter()
            .filter_map(|entry| match entry {
                Entry::Message(message) => Some(message.id.name.to_owned()),
                _ => None,
            })
            .collect()
    }

    fn builtin_languages() -> BTreeSet<String> {
        Builtin::iter()
            .filter_map(|path| path.split('/').next().map(str::to_owned))
            .collect()
    }

    #[test]
    fn fallback_is_a_valid_tag() {
        assert!(FALLBACK.parse::<LanguageIdentifier>().is_ok());
    }

    #[test]
    fn every_language_parses_and_names_itself() {
        for language in builtin_languages() {
            assert!(
                language.parse::<LanguageIdentifier>().is_ok(),
                "folder {language}"
            );
            assert!(
                message_ids(&language).contains(LANGUAGE_NAME),
                "{language}: no {LANGUAGE_NAME}"
            );
        }
    }

    #[test]
    fn built_in_translations_are_complete() {
        // User translations may be partial (English fills the gaps); the built-in ones may not.
        let english = message_ids(FALLBACK);
        for language in builtin_languages() {
            let ids = message_ids(&language);
            let missing: Vec<_> = english.difference(&ids).collect();
            let unknown: Vec<_> = ids.difference(&english).collect();
            assert!(missing.is_empty(), "{language} lacks {missing:?}");
            assert!(
                unknown.is_empty(),
                "{language} has ids English lacks: {unknown:?}"
            );
        }
    }

    #[test]
    fn arguments_have_no_isolation_marks() {
        let text = tr!("tray-device-off", device = "Headset");
        assert_eq!(text, "Headset · off");
    }

    #[test]
    fn german_is_offered_and_selectable() {
        assert!(
            available()
                .iter()
                .any(|l| l.tag == "de" && l.name == "Deutsch")
        );
    }
}

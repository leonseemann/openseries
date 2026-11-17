# ADR 0010 — UI translations with Fluent

- **Status:** accepted (2026-11-17, owner)

## Context

The app's UI was English only. The owner wants English and German first, the system language
picked automatically, and new languages addable **without changing code**.

Options checked (all MIT or Apache-2.0):

| Option | Format | Plurals / grammar | Missing-key check | Add a language |
|---|---|---|---|---|
| **A. Fluent via `i18n-embed` + `i18n-embed-fl`** | `.ftl` (Mozilla) | yes (CLDR plural rules, selectors) | compile time (`fl!`) | drop a folder |
| B. `rust-i18n` | YAML/TOML/JSON | no plural rules | run time | drop a file, rebuild |
| C. gettext (`.po`) | `.po` | yes | none | needs tooling (`msgfmt`) |
| D. own key/value table | — | no | — | code change |

## Decision

**Option A.** Fluent is the format of Firefox and many Rust desktop apps (COSMIC). It lets
translators handle plurals, gender and word order without code changes, and `fl!` rejects unknown
message ids and argument names at compile time.

- One file per language: `crates/ss-app/i18n/<BCP 47 tag>/openseries.ftl`, embedded with
  `rust-embed`. A new folder is found automatically and shows up in Settings.
- Files in `<config dir>/i18n/<tag>/openseries.ftl` are loaded at start-up and **take priority**:
  users add or fix a language without a rebuild.
- English is the source language and the fallback for every missing message. Built-in
  translations must be complete (unit test); user translations may be partial.
- Language: `app.language` in the config, `None` = system language (`sys-locale`, negotiated
  against the available languages).
- Device drivers keep their English labels; the UI looks up a translation by a stable id and
  falls back to the driver's text, so domain crates stay free of UI concerns.

Design details: [i18n.md](../architecture/i18n.md).

## Consequences

- Every user-visible string in `ss-app` goes through `tr!`. Log output, errors from lower crates
  and developer tools (`ssctl`, `ssmix`, `ssdsp`) stay English.
- Translations for scripts egui's bundled fonts lack (CJK, Arabic, …) need a font fallback before
  they render; Latin, Greek and Cyrillic work.

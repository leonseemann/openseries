# UI design (`ss-app` window)

Redesign of the egui window (2026-11-14). Goal: a calm, modern settings app that a first-time
user understands without reading docs — in the spirit of macOS System Settings, Windows 11
Settings and well-made audio apps, with our own look (no SteelSeries assets or branding).

## 1. Principles

1. **One thing per screen.** Sidebar navigation with five destinations; each page is a column of
   cards, each card one topic.
2. **Most-used first.** "Overview" shows the headset state and the controls people touch daily
   (battery, noise control, ChatMix, AudioMix on/off, EQ preset). Details live on their pages.
3. **Settings rows.** Label (and a short description) on the left, the control on the right —
   the same pattern everywhere, so the eye learns where to look.
4. **The right control for the job.** Switches for on/off, segmented buttons for 2–4 exclusive
   choices, drop-downs for longer lists, sliders with a visible value for ranges.
5. **Plain language.** "Noise cancelling" instead of "ANC mode 2"; every non-obvious row has a
   one-line description; technical details in tooltips.
6. **Visible system status.** Connection, battery and service state are always visible in the
   sidebar; problems appear as an inline banner at the top of the affected card with what to do.
7. **Progressive disclosure.** Advanced options (backup devices, presets, hotkeys, licences) sit in
   collapsible sections or lower on the page.
8. **Consistency.** One spacing scale, one corner radius, one accent colour, one type scale.
9. **Accessible.** Text contrast ≥ 4.5 : 1 against its background in both themes, controls at
   least 28 px high, everything reachable by keyboard (egui focus), no meaning by colour alone
   (status dots always come with a word).
10. **Platform-honest.** Features that do not exist on the current OS are hidden or explained in
    one sentence, never shown as broken controls.

## 2. Layout

```text
┌──────────────┬───────────────────────────────────────────────┐
│ ◉ OpenSeries│  Page title                                   │
│              │  One-line description                         │
│ ⌂ Overview   │  ┌─ Card ───────────────────────────────────┐ │
│ 🎧 Headset   │  │ Section title                            │ │
│ 🎚 AudioMix  │  │ Label            description     [control]│ │
│ 🎤 Microphone│  └──────────────────────────────────────────┘ │
│ ⚙ Settings   │  ┌─ Card ───────────────────────────────────┐ │
│              │  │ …                                        │ │
│ ● Headset 88%│  └──────────────────────────────────────────┘ │
│ ● AudioMix on│                                               │
└──────────────┴───────────────────────────────────────────────┘
```

- Sidebar 196 px, content max width 720 px (centred when the window is wider), page padding 20 px.
  Compact by default (owner feedback 2026-11-14): window 920 × 660.
- Cards: radius 10 px, inner padding 14 px, 10 px gap between cards, 1 px border; controls 26 px
  high, switches 36 × 20 px.
- Spacing scale: 4 · 8 · 12 · 16 · 24 · 32 px.

## 3. Theme

Follows the OS light/dark setting (egui `ThemePreference::System`). Accent: indigo `#5B7CFA`
(dark) / `#3D5AF1` (light) — not SteelSeries orange.

| Token | Dark | Light |
|---|---|---|
| window background | `#121317` | `#F3F4F7` |
| sidebar | `#17181D` | `#EBEDF1` |
| card | `#1D1F25` | `#FFFFFF` |
| card border | `#2A2D35` | `#DFE2E8` |
| text | `#E8EAEF` | `#1B1D22` |
| secondary text | `#A0A6B2` | `#5D6370` |
| success / warning / error | `#3FB97F` / `#E3A43B` / `#E5606B` | `#1E8E5A` / `#B26F00` / `#C93D48` |

Type scale (system UI font: Segoe UI on Windows, the egui default elsewhere): page title 20,
card title 14.5 (strong), body 13.5, secondary 12. Sidebar icons come from the emoji font, whose
metrics differ from the text font; they are centred on their glyph shape at the optical middle of
the label (measured on "H"), not on the line height. The app icon (window, `.exe`, installer, macOS bundle) is a white
headphone on an accent-coloured rounded square, drawn in code (`src/icon.rs`; generated files in
`bundle/icon/`, kept in sync by a test); the tray uses the plain glyph.

## 4. Pages

| Page | Cards |
|---|---|
| Overview | Headset (status, battery bar, spare battery, charging) · Quick controls (noise control segments, AudioMix switch, mic mute state) · ChatMix · Headset EQ preset |
| Headset | Noise control (mode, transparency strength) · Microphone (volume, sidetone, mute LED) · Equalizer (preset, custom curve, presets) · Power & display (auto-off, OLED brightness) · Device (firmware, USB, reconnect) |
| AudioMix | AudioMix switch + status · Output (device, backups) · ChatMix · Channels (one card per channel) · Apps (assignment list) |
| Microphone | Virtual microphone switch + status · Source (device, backups, default input) · Processing (noise suppression, gate, stabilizer, EQ, gain) · Presets |
| Settings | General (start at login, notifications) · Hotkeys · Crackling · Updates (automatic check, status, install) · About (version, licences, getting started, reset, quit) |

## 5. Implementation

`ui/theme.rs` (visuals, fonts, sizes), `ui/components.rs` (card, settings row, switch, segmented
control, status dot, banner, battery bar), `ui/mod.rs` (sidebar + page routing). Pages use only
these components. Development hooks for screenshots: `OPENSERIES_START_PAGE` (overview,
headset, audiomix, mic, settings) and `OPENSERIES_THEME` (dark, light).

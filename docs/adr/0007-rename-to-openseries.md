# ADR 0007 — Rename StealSeries to OpenSeries

- **Status:** accepted (2026-11-15, owner)

## Context

The licence and legal review before the first public release (2026-11-15) found one serious
risk: the name "StealSeries" sounds exactly like the trademark "SteelSeries" (steal / steel are
homophones), is used for software around the same products, and the pun on "steal" could be read
as disparaging a well-known mark. No release had been published yet, so renaming was still cheap.

Candidates were checked for existing audio / headset software of the same name (web search, not a
trademark register search). The owner chose **OpenSeries**, following the pattern of OpenRGB,
OpenRazer and OpenLogi (independent open-source tools for a vendor's hardware).

## Decision

- Product name **OpenSeries** everywhere: app, window, tray, installer, macOS bundle
  `OpenSeries.app` (`dev.openseries.app`), executable `openseries(.exe)`, virtual microphone
  "OpenSeries Microphone" (`OpenSeriesMic.driver`), config folder `OpenSeries`, Windows toast ID
  `OpenSeries.App`, installer AppId (new GUID), release packages `OpenSeries-X.Y.Z-…`, GitHub
  repository `leonseemann/openseries`, update signature context `openseries-update-v1`.
- Our own feature labels no longer use "ChatMix" (a SteelSeries term): the slider and hotkeys are
  called **Game/Chat balance**. "ChatMix" remains only where it names SteelSeries' own dial or
  format (docs, protocol).
- The trademark notice in the README and the "not affiliated" line in the app stay.

## Migration from StealSeries builds

**Removed again on 2026-11-16:** StealSeries never had a public release, so the only installs
were the owner's test builds, which were migrated by hand. The migration code below is gone from
the app; the table records what it did.

| Item | What happens |
|---|---|
| Settings and presets | `ss-store` moves the `StealSeries` config folder to `OpenSeries` on first start (only if the new one does not exist yet). |
| Start at login | the old Run value / LaunchAgent is removed; if autostart was on it is re-created for the new executable at start-up. |
| Windows toast registration | the old `StealSeries.App` key is removed. |
| macOS virtual microphone | "Install driver…" removes `StealSeriesMic.driver` together with installing `OpenSeriesMic.driver`. |
| Installed app | not touched: the owner's test installs of StealSeries are removed by hand once (Windows: Apps → StealSeries → Uninstall; macOS: delete `StealSeries.app`). |

## Consequences

- No user besides the owner ever had a StealSeries build, so the clean break (new AppId, new
  bundle ID) costs nothing; macOS asks for microphone / audio-capture permission once more.
- GitHub redirects the old repository URLs; the app only uses the new one.
- The local working folder may keep its old name; nothing depends on it.

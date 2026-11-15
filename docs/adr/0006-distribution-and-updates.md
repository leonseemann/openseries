# ADR 0006 — Installers and in-app updates

- **Status:** accepted (2026-11-15, owner)
- **Context owner request:** "CI installer mit automatischen Updates in der Anwendung"

## Context

Until now OpenSeries is built from source (`cargo build`, `scripts/macos/app.sh install`). The
owner switches between a Windows PC and a Mac and wants installers built by CI plus updates from
inside the app. Hard constraints:

- The app stays **completely free**: no paid code-signing certificates (Windows Authenticode,
  Apple Developer ID / notarization), no paid services ([app-stays-free], ADR 0004 updates).
- The repository was private. A private repo's releases cannot be downloaded by the app without
  embedding a token, which anyone could extract.

## Decision

1. **The repository becomes public**; GitHub Releases of `leonseemann/openseries` are the update
   source (owner decision 2026-11-15). Public repos also get free CI minutes on every OS.
2. **CI builds the packages** on a version tag `vX.Y.Z` (`.github/workflows/release.yml`):
   - Windows: `OpenSeries-X.Y.Z-windows-x64-setup.exe`, an [Inno Setup] installer (free).
     Per-user install into `%LOCALAPPDATA%\Programs\OpenSeries` — no admin rights, so updates
     never show a UAC prompt. Start-menu entry and uninstaller included.
   - macOS: `OpenSeries-X.Y.Z-macos-arm64.zip` with `OpenSeries.app` (Apple Silicon; the
     owner's Mac), Developer ID signed and notarized (see *Update 2026-11-15* below).
   - The tag must equal the workspace version; the release is created with generated notes.
3. **Updates are signed with our own Ed25519 key.** CI signs every package; the public key is
   compiled into the app (`ss-update`). The signed message binds the file name (which contains the
   version) and its SHA-256, so a package can neither be altered nor passed off as another
   version. The private key lives only in the repository secret `UPDATE_SIGNING_KEY` and in the
   owner's password manager. This replaces the trust that paid code signing would give for the
   update path; it costs nothing.
4. **Update flow: notify, install on click** (owner decision). The app asks the GitHub API for the
   latest release shortly after start and every 6 hours (switch "Check for updates
   automatically", default on), and on "Check now". A newer version shows in Settings → Updates
   and in the tray menu, plus one notification per version. "Install update" downloads the
   package, verifies the signature, then:
   - Windows: starts the installer silently and quits; the installer replaces the files and
     starts the new version.
   - macOS: a detached helper waits for the app to quit, swaps the `.app` bundle in place and
     opens it. Only offered when the app runs from an `.app` bundle.
   The app never installs without the click (no restarts in the middle of a game or call).
5. **Platform-free logic** (version comparison, release parsing, asset choice, signature check)
   lives in the new crate `ss-update`; network and installation live in `ss-app`
   (`src/update/`, `platform::install_update`).

## Consequences

- Windows first installs are unsigned in the OS sense: SmartScreen shows "Windows protected your
  PC" (→ *More info* → *Run anyway*). Updates downloaded by the app carry no Mark-of-the-Web, so
  they install without this prompt.
- The update check contacts `api.github.com` and `github.com` (no personal data; User-Agent
  `OpenSeries/<version>`). It can be switched off.
- Losing the signing key means existing installs cannot verify new updates; the owner then
  installs the next version manually once (with a new key compiled in). Keep the key backed up.
- The macOS virtual-mic driver is not updated by an app update; "Install driver…" on the
  Microphone page copies the bundled version again when needed.
- Releases are published by pushing a tag; `docs/architecture/release.md` describes the steps.

## Update 2026-11-15 — Apple Developer ID

The owner has a paid Apple Developer account and wants to use it. macOS release builds are
therefore signed with the team's Developer ID Application certificate (hardened runtime) and
notarized in CI (App Store Connect API key). Users still pay nothing; the account is the
maintainer's. Effects: no Gatekeeper warning on the first open, and a stable signing identity, so
macOS keeps the microphone / audio-capture permissions across updates. Our own Ed25519 update
signature stays: it also covers Windows and does not depend on Apple. Details and secrets:
`docs/architecture/release.md` §3.

[Inno Setup]: https://jrsoftware.org/isinfo.php
[app-stays-free]: 0004-windows-audio-strategy.md

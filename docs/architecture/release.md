# Releases, installers and in-app updates

Decision: [ADR 0006](../adr/0006-distribution-and-updates.md). Workflow:
[`.github/workflows/release.yml`](../../.github/workflows/release.yml).

## 1. Publishing a release

1. Bump `version` in `[workspace.package]` of the root `Cargo.toml` (semver: `0.2.0`, `0.2.1` …),
   run the checks from AGENTS.md, commit, push.
2. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`.
3. CI builds both packages, signs them, checks the signatures against the key built into the app
   and publishes the release `v0.2.0` with generated notes. Installed apps see it within 6 hours
   (or at once via Settings → Updates → Check now).

A manual run (Actions → Release → Run workflow) builds the same packages as workflow artifacts
without publishing anything — useful to try the installer before tagging.

| Package | Built by | Contents |
|---|---|---|
| `OpenSeries-X.Y.Z-windows-x64-setup.exe` (+ `.sig`) | `windows` job: `cargo build --release`, Inno Setup ([`installer/windows/openseries.iss`](../../installer/windows/openseries.iss)) | per-user install to `%LOCALAPPDATA%\Programs\OpenSeries`, Start-menu entry, optional desktop shortcut, uninstaller (removes the "start at login" value; keeps the settings in `%APPDATA%`) |
| `OpenSeries-X.Y.Z-macos-arm64.zip` (+ `.sig`) | `macos` job: [`scripts/macos/app.sh build`](../../scripts/macos/app.sh), `ditto` | `OpenSeries.app` incl. the bundled virtual-mic driver; Apple Silicon only |

## 2. Update signing key (Ed25519)

- **Public key:** `ss-update/src/signature.rs` (`PUBLIC_KEY`), compiled into every app.
- **Private key:** repository secret `UPDATE_SIGNING_KEY` (base64, one line). Generated
  2026-11-15 with `ss-release-sign keygen`; the owner keeps a copy in the password manager.
  `scripts/release-sign.sh` signs and then verifies every package, so a wrong secret fails the
  build instead of shipping updates that installed apps would reject.
- **New key** (only if the old one is lost): `cargo run -p ss-update --features sign --bin
  ss-release-sign -- keygen <file outside the repo>`, put the printed public key into
  `PUBLIC_KEY`, the file content into the secret. Existing installs must then install the next
  version manually once.

## 3. macOS: Developer ID signing and notarization

The owner has an Apple Developer account (2026-11-15), so release builds are signed with the
team's **Developer ID Application** certificate, use the hardened runtime
([`entitlements.plist`](../../crates/ss-app/bundle/entitlements.plist): microphone only) and are
**notarized** and stapled. Gatekeeper then opens the app without warnings, and the stable identity
keeps the microphone / audio-capture permissions across updates. The bundled virtual-mic driver is
signed the same way (`hal-driver.sh`).

Repository secrets (a tag build fails if one is missing; a manual run falls back to ad-hoc):

| Secret | Content |
|---|---|
| `MACOS_CERTIFICATE` | base64 of the `.p12` export of the "Developer ID Application" certificate **with its private key** |
| `MACOS_CERTIFICATE_PASSWORD` | the `.p12` export password |
| `APPLE_API_KEY` | base64 of the App Store Connect API key file `AuthKey_XXXX.p8` (role Developer) |
| `APPLE_API_KEY_ID` | its key ID |
| `APPLE_API_ISSUER_ID` | the issuer ID shown above the key list |

CI imports the certificate into a temporary keychain, sets `SIGN_IDENTITY` for
`scripts/macos/app.sh build`, submits the app with `xcrun notarytool submit --wait`, staples the
ticket and checks it with `spctl`. Locally the same works with
`SIGN_IDENTITY="Developer ID Application: … (TEAMID)" scripts/macos/app.sh build`.

The Developer ID certificate is valid for 5 years; renew it before it expires and replace the
secret. Already notarized releases keep working after expiry (secure timestamp).

## 4. First install

- **Windows:** download the `setup.exe` from the release page and run it. SmartScreen shows
  "Windows protected your PC" (no paid Authenticode certificate) → *More info* → *Run anyway*.
  No admin rights needed.
- **macOS:** download the zip, open it, move `OpenSeries.app` to *Applications*, open it
  normally (notarized: macOS only asks "downloaded from the internet – open?"). Allow microphone
  and audio capture; install the virtual-mic driver from the Getting-started window.

Later updates come through the app (Settings → Updates or the tray / menu bar).

## 5. In-app update flow

`ss-app/src/update.rs` (thread `updater`):

| Step | What happens |
|---|---|
| Check | 20 s after start, then every 6 h while "Check for updates automatically" is on, or on "Check now" / tray. `GET https://api.github.com/repos/leonseemann/openseries/releases/latest` (unauthenticated: 60 requests/h per IP, far above our 4/day). |
| Offer | Newer semver than the running app + a package and `.sig` for this platform → Settings → Updates, a line at the top of every page, tray item "Install update X.Y.Z…", one notification per version (if notifications are on). |
| Download | Signature first, then the package (≤ 256 MB) into memory with progress; verified with `ss-update::verify` (name + SHA-256 + Ed25519) before anything touches the disk. |
| Install | Windows: the installer runs with `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-`, the app quits (restoring app volumes, saving the config); the installer replaces the files and starts the new version. macOS: `ditto` unpacks the bundle, a detached `/bin/sh` helper waits for the app to exit, moves the old bundle aside, moves the new one in (rolls back if that fails) and opens it. |

The app refreshes the "start at login" entry at every start, so it follows the executable when an
update (or the first installer run from a development build) moves it.

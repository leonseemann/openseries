#!/bin/sh
# Builds and installs StealSeries.app (docs/architecture/app.md §5).
#
#   scripts/macos/app.sh build     # → target/StealSeries.app (ad-hoc signed)
#   scripts/macos/app.sh install   # build + copy to ~/Applications (no sudo) and open it
#
# Signing: ad-hoc by default (development; the signature changes with every build, so macOS may
# ask for permissions again). Set SIGN_IDENTITY to a "Developer ID Application: …" identity in the
# keychain for release builds: hardened runtime + secure timestamp, ready for notarization
# (docs/architecture/release.md §3). hal-driver.sh uses the same variable.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
CRATE=ss-app
BUNDLE="$ROOT/target/StealSeries.app"
TARGET_DIR="$HOME/Applications"

# Builds the .icns from the generated 1024 px master (crates/ss-app/src/icon.rs).
app_icon() {
    master="$ROOT/crates/$CRATE/bundle/icon/AppIcon.png"
    iconset="$ROOT/target/AppIcon.iconset"
    rm -rf "$iconset"
    mkdir -p "$iconset"
    for size in 16 32 128 256 512; do
        sips -z "$size" "$size" "$master" --out "$iconset/icon_${size}x${size}.png" >/dev/null
        double=$((size * 2))
        sips -z "$double" "$double" "$master" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
    done
    iconutil -c icns "$iconset" -o "$1"
}

build() {
    cargo build --release -p "$CRATE" --manifest-path "$ROOT/Cargo.toml"
    version=$(cargo pkgid -p "$CRATE" --manifest-path "$ROOT/Cargo.toml" | sed 's/.*[#@]//')
    rm -rf "$BUNDLE"
    mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
    sed "s/@VERSION@/$version/g" "$ROOT/crates/$CRATE/bundle/Info.plist" > "$BUNDLE/Contents/Info.plist"
    cp "$ROOT/target/release/stealseries" "$BUNDLE/Contents/MacOS/stealseries"
    app_icon "$BUNDLE/Contents/Resources/AppIcon.icns"
    # The virtual-mic driver travels inside the app so "Install driver…" can copy it
    # (app.md §2, onboarding). Signed on its own by hal-driver.sh, then sealed with the app.
    "$ROOT/scripts/macos/hal-driver.sh" build
    cp -R "$ROOT/target/StealSeriesMic.driver" "$BUNDLE/Contents/Resources/"
    if [ -n "${SIGN_IDENTITY:-}" ]; then
        codesign --force --options runtime --timestamp \
            --entitlements "$ROOT/crates/$CRATE/bundle/entitlements.plist" \
            --sign "$SIGN_IDENTITY" "$BUNDLE"
    else
        codesign --force --sign - "$BUNDLE"
    fi
    echo "built $BUNDLE"
}

case "${1:-}" in
    build)
        build
        ;;
    install)
        build
        if pkill -x stealseries 2>/dev/null; then
            # Wait until the old instance is gone; opening too early fails with error -600.
            while pgrep -x stealseries >/dev/null; do sleep 0.2; done
            sleep 1
        fi
        mkdir -p "$TARGET_DIR"
        rm -rf "$TARGET_DIR/StealSeries.app"
        cp -R "$BUNDLE" "$TARGET_DIR/"
        echo "installed $TARGET_DIR/StealSeries.app"
        open "$TARGET_DIR/StealSeries.app"
        ;;
    *)
        echo "usage: $0 build|install" >&2
        exit 2
        ;;
esac

#!/bin/sh
# Builds and installs StealSeries.app (docs/architecture/app.md §5).
#
#   scripts/macos/app.sh build     # → target/StealSeries.app (ad-hoc signed)
#   scripts/macos/app.sh install   # build + copy to ~/Applications (no sudo) and open it
#
# Ad-hoc signatures change with every build, so macOS may ask for permissions again after an
# update. Set SIGN_IDENTITY to a code-signing certificate in the keychain (CI uses a free
# self-signed one, ADR 0006) to keep one stable identity across updates.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
CRATE=ss-app
BUNDLE="$ROOT/target/StealSeries.app"
TARGET_DIR="$HOME/Applications"

build() {
    cargo build --release -p "$CRATE" --manifest-path "$ROOT/Cargo.toml"
    version=$(cargo pkgid -p "$CRATE" --manifest-path "$ROOT/Cargo.toml" | sed 's/.*[#@]//')
    rm -rf "$BUNDLE"
    mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
    sed "s/@VERSION@/$version/g" "$ROOT/crates/$CRATE/bundle/Info.plist" > "$BUNDLE/Contents/Info.plist"
    cp "$ROOT/target/release/stealseries" "$BUNDLE/Contents/MacOS/stealseries"
    # The virtual-mic driver travels inside the app so "Install driver…" can copy it
    # (app.md §2, onboarding). Signed on its own by hal-driver.sh, then sealed with the app.
    "$ROOT/scripts/macos/hal-driver.sh" build
    cp -R "$ROOT/target/StealSeriesMic.driver" "$BUNDLE/Contents/Resources/"
    codesign --force --sign "${SIGN_IDENTITY:--}" "$BUNDLE"
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

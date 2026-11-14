#!/bin/sh
# Builds and installs StealSeries.app (docs/architecture/app.md §5).
#
#   scripts/macos/app.sh build     # → target/StealSeries.app (ad-hoc signed)
#   scripts/macos/app.sh install   # build + copy to ~/Applications (no sudo) and open it
#
# Ad-hoc signatures change with every build, so macOS may ask for permissions again after an
# update (a stable signing identity comes with distribution signing).
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
    codesign --force --sign - "$BUNDLE"
    echo "built $BUNDLE"
}

case "${1:-}" in
    build)
        build
        ;;
    install)
        build
        pkill -x stealseries 2>/dev/null || true
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

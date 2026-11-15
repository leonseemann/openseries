#!/bin/sh
# Builds, installs or removes the virtual-mic HAL plug-in StealSeriesMic.driver.
# See docs/architecture/virtual-mic-driver.md §6 and docs/hardware/macos-virtual-mic.md.
#
#   scripts/macos/hal-driver.sh build      # → target/StealSeriesMic.driver (ad-hoc signed)
#   scripts/macos/hal-driver.sh install    # build + copy to /Library/Audio/Plug-Ins/HAL (sudo)
#   scripts/macos/hal-driver.sh uninstall  # remove it again (sudo)
#
# install/uninstall restart coreaudiod: all audio stops for about a second.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
NAME=StealSeriesMic
BUNDLE="$ROOT/target/$NAME.driver"
HAL_DIR=/Library/Audio/Plug-Ins/HAL
CRATE=ss-hal-driver-macos

build() {
    cargo build --release -p "$CRATE" --manifest-path "$ROOT/Cargo.toml"
    version=$(cargo pkgid -p "$CRATE" --manifest-path "$ROOT/Cargo.toml" | sed 's/.*[#@]//')
    rm -rf "$BUNDLE"
    mkdir -p "$BUNDLE/Contents/MacOS"
    sed "s/@VERSION@/$version/g" "$ROOT/crates/$CRATE/bundle/Info.plist" > "$BUNDLE/Contents/Info.plist"
    cp "$ROOT/target/release/libss_hal_driver_macos.dylib" "$BUNDLE/Contents/MacOS/$NAME"
    # Release builds: Developer ID with hardened runtime (SIGN_IDENTITY, see app.sh).
    if [ -n "${SIGN_IDENTITY:-}" ]; then
        codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$BUNDLE"
    else
        codesign --force --sign - "$BUNDLE"
    fi
    echo "built $BUNDLE"
}

restart_coreaudiod() {
    sudo killall coreaudiod
    echo "coreaudiod restarted"
}

case "${1:-}" in
    build)
        build
        ;;
    install)
        build
        sudo rm -rf "$HAL_DIR/$NAME.driver"
        sudo cp -R "$BUNDLE" "$HAL_DIR/"
        restart_coreaudiod
        ;;
    uninstall)
        sudo rm -rf "$HAL_DIR/$NAME.driver"
        restart_coreaudiod
        ;;
    *)
        echo "usage: $0 build|install|uninstall" >&2
        exit 2
        ;;
esac

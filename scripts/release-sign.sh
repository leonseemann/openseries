#!/bin/sh
# Signs release packages for in-app updates and checks them against the key built into the app
# (ADR 0006 §3, docs/architecture/release.md).
#
#   UPDATE_SIGNING_KEY=… scripts/release-sign.sh <package>…
#
# Without the key a manual (workflow_dispatch) build stays unsigned; a tag build fails, because
# installed apps would refuse the update.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)

if [ -z "${UPDATE_SIGNING_KEY:-}" ]; then
    if [ "${GITHUB_REF_TYPE:-}" = tag ]; then
        echo "::error::the UPDATE_SIGNING_KEY secret is missing; releases must be signed" >&2
        exit 1
    fi
    echo "UPDATE_SIGNING_KEY not set: packages stay unsigned (test build)" >&2
    exit 0
fi

run() {
    cargo run --quiet --release --locked --manifest-path "$ROOT/Cargo.toml" \
        -p ss-update --features sign --bin ss-release-sign -- "$@"
}

run sign "$@"
# Fails if the secret does not belong to the public key compiled into the app.
run verify "$@"

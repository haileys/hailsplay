#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/protocol"

TMP="/tmp/gen-types.$$"
mkdir -p "$TMP"

USE_COLOR=
[ -t 1 ] && USE_COLOR=1

cargo-build() {
    [ -n "$USE_COLOR" ] && {
        export CARGO_TERM_COLOR=always
        export CARGO_TERM_PROGRESS_WHEN=never
    }

    set -x
    cargo build --lib --target wasm32-unknown-unknown
}

wasm-bindgen() {
    set -x
    exec wasm-bindgen ../target/wasm32-unknown-unknown/debug/hailsplay_protocol.wasm \
        --out-dir "$TMP" \
        --typescript \
        --target bundler \
        --debug
}

try() {
    local func="$1"
    local log="$TMP/$1.log"
    if ! ( "$func" ) &> "$log"; then
        echo >&2 "!!! failed to run $func, output below:"
        cat  >&2 "$log"
        echo >&2
        echo >&2 "Leaving files in $TMP"
        exit 1
    fi
}

echo "+++ building"
try cargo-build

echo "+++ generating bindings"
try wasm-bindgen

cp "$TMP/hailsplay_protocol.d.ts" protocol.d.ts
rm -rf "$TMP"

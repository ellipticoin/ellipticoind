#!/bin/bash
set -e

TARGET_PATH="../target/wasm32-unknown-unknown/release"
DIST_DIR="../ellipticoind/dist"

MODULE_NAME=$(cat Cargo.toml | grep name | sed -n 's/name *= *"\(.*\)"/\1/p')
BASE_NAME="$TARGET_PATH/$MODULE_NAME"
cargo build --target wasm32-unknown-unknown --release
if ! command -v  wasm-gc > /dev/null; then
  cargo install wasm-gc
fi

if ! command -v  wasm-snip > /dev/null; then
  cargo install wasm-snip
fi

cp "$BASE_NAME.wasm" "$BASE_NAME.min.wasm"
wasm-snip --snip-rust-fmt-code --snip-rust-panicking-code "$BASE_NAME.min.wasm" -o "$BASE_NAME.min.wasm"
wasm-gc "$BASE_NAME.min.wasm"
wasm-opt -Oz --strip-debug --strip-producers "$BASE_NAME.min.wasm" -o "$BASE_NAME.min.wasm"

mkdir -p $DIST_DIR

cp "$BASE_NAME.min.wasm" $DIST_DIR

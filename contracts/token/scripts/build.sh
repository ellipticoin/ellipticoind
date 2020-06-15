#!/bin/bash
set -e

TARGET_PATH="target/wasm32-unknown-unknown/release"
DIST_DIR="dist"

MODULE_NAME=$(cat Cargo.toml | grep name | sed -n 's/name *= *"\(.*\)"/\1/p')
FILE_NAME="$TARGET_PATH/$MODULE_NAME.wasm"
cargo build --target wasm32-unknown-unknown --release
if ! command -v  wasm-gc > /dev/null; then
  cargo install wasm-gc
fi

if ! command -v  wasm-snip > /dev/null; then
  cargo install wasm-snip
fi

#wasm-snip --snip-rust-fmt-code --snip-rust-panicking-code $FILE_NAME -o $FILE_NAME
#wasm-gc $FILE_NAME
#wasm-opt -Oz --strip-debug --strip-producers $FILE_NAME -o $FILE_NAME

mkdir -p $DIST_DIR

cp $FILE_NAME $DIST_DIR

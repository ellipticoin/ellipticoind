set -e

TARGET_PATH="../target/wasm32-unknown-unknown/release/"
DIST_DIR="`pwd`/dist"

scriptsDir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd $scriptsDir/..
moduleName=$(cat Cargo.toml | grep name | sed -n 's/name *= *"\(.*\)"/\1/p')
wasmFilename="$moduleName.wasm"
cargo +nightly build --target wasm32-unknown-unknown --release
if ! command -v  wasm-gc > /dev/null; then
  cargo install wasm-gc
fi
cd $TARGET_PATH
wasm-snip --snip-rust-fmt-code $wasmFilename -o $wasmFilename
wasm-snip --snip-rust-panicking-code $wasmFilename -o $wasmFilename
wasm-snip $wasmFilename -o $wasmFilename -p "BTreeMap"
wasm-gc $wasmFilename

mkdir -p $DIST_DIR
cp -R $wasmFilename "$DIST_DIR/$wasmFilename"

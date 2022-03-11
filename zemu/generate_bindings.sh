#!/usr/bin/env bash

SCRIPT_DIR=$(dirname "$0")

echo "THIS WILL REGENERATE THE HEADERS TO BE USED IN THE APP LATER FOR UI"
echo "PLEASE CHECK THE GENERATED FILES"

export RUSTUP_TOOLCHAIN=nightly

rm "$SCRIPT_DIR"/include/zemu_ui_x.h || true
cbindgen --config "$SCRIPT_DIR"/cbindgen_x.toml \
    --crate zemu-sys \
    --output "$SCRIPT_DIR"/include/zemu_ui_x.h

echo "Cleaning up old Nano S bindings and regenerating them"

rm "$SCRIPT_DIR"/include/zemu_ui_s.h || true
cbindgen --config "$SCRIPT_DIR"/cbindgen_s.toml \
    --crate zemu-sys \
    --output "$SCRIPT_DIR"/include/zemu_ui_s.h

echo "Cleaning up old Nano S+ bindings and regenerating them"

rm "$SCRIPT_DIR"/include/zemu_ui_sp.h || true
cbindgen --config "$SCRIPT_DIR"/cbindgen_sp.toml \
    --crate zemu-sys \
    --output "$SCRIPT_DIR"/include/zemu_ui_sp.h

echo "Done!"

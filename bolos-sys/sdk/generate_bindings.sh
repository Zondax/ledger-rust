#!/usr/bin/env bash

SCRIPT_DIR=$(dirname "$0")
pushd "$SCRIPT_DIR" || return

: "${TMP_HEADERS_PATH:=/tmp/bolos/arm-none-eabi}"
: "${DOCKER_IMAGE:=zondax/builder-bolos:latest}"
: "${GCC_BOLOS_PATH:=gcc-arm-none-eabi-10-2020-q4-major}"

# match versions from the official builder https://github.com/LedgerHQ/ledger-app-builder/blob/master/lite/Dockerfile
# Nano S (not unified for now)
: "${BOLOS_SDK_S_PATH:=nanos-secure-sdk}"
: "${BOLOS_SDK_S_GIT:=https://github.com/LedgerHQ/nanos-secure-sdk}"
: "${BOLOS_SDK_S_GIT_HASH:=d37bdf1caf98ab2df4b8729c9cb8648ab21cd4b7}"

# Unified SDK
: "${BOLOS_SDK_PATH:=ledger-secure-sdk}"
: "${BOLOS_SDK_GIT:=https://github.com/LedgerHQ/ledger-secure-sdk}"

# Nano X
: "${BOLOS_SDK_X_PATH:=nanox-secure-sdk}"
: "${BOLOS_SDK_X_GIT_TAG:=v1.2.1}"
# Nano S+
: "${BOLOS_SDK_SP_PATH:=nanosplus-secure-sdk}"
: "${BOLOS_SDK_SP_GIT_TAG:=v1.2.1}"

TMP_HEADERS=$(dirname "$TMP_HEADERS_PATH")

echo "Checkout S SDK & update in $BOLOS_SDK_S_PATH from $BOLOS_SDK_S_GIT $BOLOS_SDK_S_GIT_HASH"
git submodule add --force "$BOLOS_SDK_S_GIT" "$BOLOS_SDK_S_PATH" || true
git submodule update --init "$BOLOS_SDK_S_PATH"
pushd "$BOLOS_SDK_S_PATH" || exit
git fetch origin
git checkout "$BOLOS_SDK_S_GIT_HASH"
popd || exit

echo "Checkout Unified SDK & update in $BOLOS_SDK_PATH from $BOLOS_SDK_GIT"
git submodule add --force "$BOLOS_SDK_GIT" "$BOLOS_SDK_PATH" || true
git submodule update --init --remote "$BOLOS_SDK_PATH"

echo "Checkoux X SDK in $BOLOS_SDK_X_PATH from $BOLOS_SDK_GIT $BOLOS_SDK_X_GIT_TAG"
git -C "$BOLOS_SDK_PATH" worktree remove -f ../"$BOLOS_SDK_X_PATH"
git -C "$BOLOS_SDK_PATH" worktree add -f ../"$BOLOS_SDK_X_PATH" "$BOLOS_SDK_X_GIT_TAG"
echo nanox > "$BOLOS_SDK_X_PATH"/.target

echo "Checkoux S+ SDK in $BOLOS_SDK_SP_PATH from $BOLOS_SDK_GIT $BOLOS_SDK_SP_GIT_TAG"
git -C "$BOLOS_SDK_PATH" worktree remove -f ../"$BOLOS_SDK_SP_PATH"
git -C "$BOLOS_SDK_PATH" worktree add -f ../"$BOLOS_SDK_SP_PATH" "$BOLOS_SDK_SP_GIT_TAG"
echo nanos2 > "$BOLOS_SDK_SP_PATH"/.target

echo "Making sure $TMP_HEADERS_PATH exists"
mkdir -p "$TMP_HEADERS_PATH" || true

echo "Copying necessary header files..."
docker run --rm \
    -d --log-driver=none \
    -v "$TMP_HEADERS":/shared \
    "$DOCKER_IMAGE" \
    "cp -r /opt/bolos/$GCC_BOLOS_PATH/arm-none-eabi/include /shared/arm-none-eabi/"

echo "Cleaning up old Nano X bindings and regenerating them"
rm ../src/bindings/bindingsX.rs || true
bindgen --use-core \
        --with-derive-default \
        --ctypes-prefix cty \
        -o ../src/bindings/bindingsX.rs \
        ../bindgen/wrapperX.h -- \
        -I"$BOLOS_SDK_X_PATH"/include \
        -I"$BOLOS_SDK_X_PATH"/target/nanox/include \
        -I"$BOLOS_SDK_X_PATH"/lib_ux/include \
        -I"$BOLOS_SDK_X_PATH"/lib_cxng/include \
        -I"$BOLOS_SDK_X_PATH"/lib_bagl/include \
        -I"$TMP_HEADERS_PATH"/include \
        -I../bindgen/include \
        -target thumbv6-none-eabi \
        -mcpu=cortex-m0 -mthumb

echo "Cleaning up old Nano S bindings and regenerating them"
rm ../src/bindings/bindingsS.rs || true
bindgen --use-core \
        --with-derive-default \
        --ctypes-prefix cty \
        -o ../src/bindings/bindingsS.rs \
        ../bindgen/wrapperS.h -- \
        -I"$BOLOS_SDK_S_PATH"/include \
        -I"$BOLOS_SDK_S_PATH"/target/nanos/include \
        -I"$BOLOS_SDK_S_PATH"/lib_ux/include \
        -I"$BOLOS_SDK_S_PATH"/lib_cxng/include \
        -I"$BOLOS_SDK_S_PATH"/lib_bagl/include \
        -I"$TMP_HEADERS_PATH"/include \
        -I../bindgen/include \
        -target thumbv6-none-eabi \
        -mcpu=cortex-m0 -mthumb

echo "Cleaning up old Nano S+ bindings and regenerating them"
rm ../src/bindings/bindingsSP.rs || true
bindgen --use-core \
        --with-derive-default \
        --ctypes-prefix cty \
        -o ../src/bindings/bindingsSP.rs \
        ../bindgen/wrapperSP.h -- \
        -I"$BOLOS_SDK_SP_PATH"/include \
        -I"$BOLOS_SDK_SP_PATH"/target/nanos2/include \
        -I"$BOLOS_SDK_SP_PATH"/lib_ux/include \
        -I"$BOLOS_SDK_SP_PATH"/lib_cxng/include \
        -I"$BOLOS_SDK_SP_PATH"/lib_bagl/include \
        -I"$TMP_HEADERS_PATH"/include \
        -I../bindgen/include \
        -target thumbv8m.main-none-eabi \
        -mcpu=cortex-m35p -mthumb

popd || true
echo "Done!"

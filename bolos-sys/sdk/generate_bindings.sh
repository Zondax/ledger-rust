#!/usr/bin/env bash

SCRIPT_DIR=$(dirname "$0")

: "${TMP_HEADERS_PATH:=/tmp/bolos/arm-none-eabi}"
: "${DOCKER_IMAGE:=zondax/builder-bolos:latest}"
: "${GCC_BOLOS_PATH:=gcc-arm-none-eabi-10-2020-q4-major}"

: "${BOLOS_SDK_S_PATH:=$SCRIPT_DIR/nanos-secure-sdk}"
: "${BOLOS_SDK_S_GIT:=https://github.com/LedgerHQ/nanos-secure-sdk}"
: "${BOLOS_SDK_X_PATH:=$SCRIPT_DIR/nanox-secure-sdk}"
: "${BOLOS_SDK_X_GIT:=https://github.com/LedgerHQ/nanox-secure-sdk}"

TMP_HEADERS=$(dirname $TMP_HEADERS_PATH)

echo "Checkout X SDK & update in $BOLOS_SDK_X_PATH from $BOLOS_SDK_S_GIT"
git submodule add "$BOLOS_SDK_X_GIT" "$BOLOS_SDK_X_PATH" || true
git submodule update --init "$BOLOS_SDK_X_PATH"

echo "Checkout S SDK & update in $BOLOS_SDK_S_PATH from $BOLOS_SDK_S_GIT"
git submodule add "$BOLOS_SDK_S_GIT" "$BOLOS_SDK_S_PATH" || true
git submodule update --init "$BOLOS_SDK_S_PATH"

echo "Making sure $TMP_HEADERS_PATH exists"
mkdir -p $TMP_HEADERS_PATH || true

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
        -I"$BOLOS_SDK_X_PATH"/lib_ux/include \
        -I"$BOLOS_SDK_X_PATH"/lib_cxng/include \
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
        -I"$BOLOS_SDK_S_PATH"/lib_ux/include \
        -I"$BOLOS_SDK_S_PATH"/lib_cxng/include \
        -I"$TMP_HEADERS_PATH"/include \
        -I../bindgen/include \
        -target thumbv6-none-eabi \
        -mcpu=cortex-m0 -mthumb

echo "Done!"

#!/usr/bin/env bash

SCRIPT_DIR=$(dirname "$0")
pushd "$SCRIPT_DIR" || return

: "${TMP_HEADERS_PATH:=/tmp/bolos/arm-none-eabi}"
: "${DOCKER_IMAGE:=zondax/builder-bolos:latest}"
: "${GCC_BOLOS_PATH:=gcc-arm-none-eabi-10-2020-q4-major}"

: "${BOLOS_SDK_S_PATH:=nanos-secure-sdk}"
: "${BOLOS_SDK_X_PATH:=nanox-secure-sdk}"
: "${BOLOS_SDK_SP_PATH:=nanosplus-secure-sdk}"

(./fetch_sdk.sh)

TMP_HEADERS=$(dirname "$TMP_HEADERS_PATH")

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

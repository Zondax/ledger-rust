#!/usr/bin/env bash

# match versions from the official builder https://github.com/LedgerHQ/ledger-app-builder/blob/master/lite/Dockerfile
# Nano S (not unified for now)
: "${BOLOS_SDK_S_PATH:=nanos-secure-sdk}"
: "${BOLOS_SDK_S_GIT:=https://github.com/LedgerHQ/nanos-secure-sdk}"
: "${BOLOS_SDK_S_GIT_HASH:=ba973f5f4be506e93fe51e1114b9a3ac93adab2a}"

# Unified SDK
: "${BOLOS_SDK_PATH:=ledger-secure-sdk}"
: "${BOLOS_SDK_GIT:=https://github.com/LedgerHQ/ledger-secure-sdk}"

# Nano X
: "${BOLOS_SDK_X_PATH:=nanox-secure-sdk}"
: "${BOLOS_SDK_X_GIT_TAG:=v1.5.0}"
NANOX=(nanox "$BOLOS_SDK_X_PATH" "$BOLOS_SDK_X_GIT_TAG")
# Nano S+
: "${BOLOS_SDK_SP_PATH:=nanosplus-secure-sdk}"
: "${BOLOS_SDK_SP_GIT_TAG:=v1.5.0}"
NANOS2=(nanos2 "$BOLOS_SDK_SP_PATH" "$BOLOS_SDK_SP_GIT_TAG")
# Stax
: "${BOLOS_SDK_FS_PATH:=stax-secure-sdk}"
: "${BOLOS_SDK_FS_GIT_TAG:=v10.1.1}"
STAX=(stax "$BOLOS_SDK_FS_PATH" "$BOLOS_SDK_FS_GIT_TAG")

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
setup_unified_sdk() {
    DEVICE=("${@}")

    TARGET="${DEVICE[0]}"
    OUT="${DEVICE[1]}"
    GIT_TAG="${DEVICE[2]}"

    echo "Create $TARGET worktree in $OUT from $BOLOS_SDK_GIT $GIT_TAG"
    git -C "$BOLOS_SDK_PATH" worktree remove -f ../"$OUT"
    git -C "$BOLOS_SDK_PATH" worktree add -f ../"$OUT" "$GIT_TAG"
    echo "$TARGET" >"$OUT"/.target
}

setup_unified_sdk "${NANOX[@]}"
setup_unified_sdk "${NANOS2[@]}"
setup_unified_sdk "${STAX[@]}"

#!/usr/bin/env bash

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
: "${BOLOS_SDK_X_GIT_TAG:=v1.3.0}"
# Nano S+
: "${BOLOS_SDK_SP_PATH:=nanosplus-secure-sdk}"
: "${BOLOS_SDK_SP_GIT_TAG:=v1.3.0}"

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

echo "Create X SDK worktree in $BOLOS_SDK_X_PATH from $BOLOS_SDK_GIT $BOLOS_SDK_X_GIT_TAG"
git -C "$BOLOS_SDK_PATH" worktree remove -f ../"$BOLOS_SDK_X_PATH"
git -C "$BOLOS_SDK_PATH" worktree add -f ../"$BOLOS_SDK_X_PATH" "$BOLOS_SDK_X_GIT_TAG"
echo nanox > "$BOLOS_SDK_X_PATH"/.target

echo "Create S+ SDK worktree in $BOLOS_SDK_SP_PATH from $BOLOS_SDK_GIT $BOLOS_SDK_SP_GIT_TAG"
git -C "$BOLOS_SDK_PATH" worktree remove -f ../"$BOLOS_SDK_SP_PATH"
git -C "$BOLOS_SDK_PATH" worktree add -f ../"$BOLOS_SDK_SP_PATH" "$BOLOS_SDK_SP_GIT_TAG"
echo nanos2 > "$BOLOS_SDK_SP_PATH"/.target

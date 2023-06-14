alias default := _help
# Show this menu
@_help:
    just --list --unsorted

alias c := cargo
# Invoke cargo
cargo *args:
    cargo {{args}}

# Create symlinks in the target dir for the Ledger SDKs
[no-cd]
link-ledger-sdk base-dir=justfile_directory() target-dir=invocation_directory() force="false": fetch-bindings
    #!/usr/bin/env sh
    f={{ if force == "true" { "-f" } else { "" } }}
    ln -s $f {{base-dir}}/bolos-sys/sdk/ledger-secure-sdk {{target-dir}}/
    ln -s $f {{base-dir}}/bolos-sys/sdk/nanosplus-secure-sdk {{target-dir}}/
    ln -s $f {{base-dir}}/bolos-sys/sdk/nanos-secure-sdk {{target-dir}}/
    ln -s $f {{base-dir}}/bolos-sys/sdk/nanox-secure-sdk {{target-dir}}/
    ln -s $f {{base-dir}}/bolos-sys/sdk/stax-secure-sdk {{target-dir}}/

alias fetch-sdk := fetch-bindings
# Fetch Ledger SDK's and do the requires setup
@fetch-bindings:
    cd bolos-sys/sdk; ./fetch_sdk.sh

alias bindings := generate-bindings
# Generate SDK bindings
@generate-bindings:
    ./bolos-sys/sdk/generate_bindings.sh

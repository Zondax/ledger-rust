alias default := _help
# Show this menu
@_help:
    just --list --unsorted

alias c := cargo
# Invoke cargo
cargo *args:
    cargo {{args}}

alias fetch-sdk := fetch-bindings
@fetch-bindings:
    cd bolos-sys/sdk; ./fetch_sdk.sh

alias bindings := generate-bindings
@generate-bindings:
    ./bolos-sys/sdk/generate_bindings.sh

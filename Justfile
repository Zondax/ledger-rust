alias default := _help
# Show this menu
@_help:
    just --list --unsorted

alias c := cargo
# Invoke cargo
cargo *args:
    cargo {{args}}

alias bindings := generate-bindings
@generate-bindings:
    ./bolos-sys/sdk/generate_bindings.sh

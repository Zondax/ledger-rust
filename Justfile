alias default := _help
# Show this menu
@_help:
    just --list --unsorted

alias c := cargo
# Invoke cargo
cargo *args:
    cargo {{args}}

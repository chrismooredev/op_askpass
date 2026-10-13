#!/bin/bash

set -e

if ! command -v cargo &> /dev/null; then
    echo "Cargo is not installed. Please install the Rust toolchain to proceed."
    exit 1
fi

cargo install op_askpass --git https://github.com/chrismooredev/op_askpass

# Set SSH_ASKPASS_REQUIRE to "force" to ensure it is used even in non-graphical environments
# Set SSH_ASKPASS to op_askpass
# Set this in .bashrc, conditionally on it not already being set, to avoid overwriting existing configurations
# If .bashrc values are different, log and exit - do not overwrite

if [ -f "$HOME/.bashrc" ]; then
    if grep -q "export SSH_ASKPASS=op_askpass" "$HOME/.bashrc"; then
        echo "SSH_ASKPASS is already set to op_askpass in .bashrc, skipping"
    elif grep -q "export SSH_ASKPASS=" "$HOME/.bashrc"; then
        echo "SSH_ASKPASS is set to a different value in .bashrc, please resolve manually" >&2
        exit 1
    else
        echo "export SSH_ASKPASS=op_askpass" >> "$HOME/.bashrc"
        echo "export SSH_ASKPASS_REQUIRE=force" >> "$HOME/.bashrc"
        echo "SSH_ASKPASS configured in .bashrc"
    fi
fi

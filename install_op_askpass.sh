#!/bin/bash

set -e

if ! command -v cargo &> /dev/null; then
    echo "Cargo is not installed. Please install the Rust toolchain to proceed."
    exit 1
fi

if grep -qs '^name = "op_askpass"' Cargo.toml; then
    echo "Installing op_askpass from local checkout"
    cargo install --path .
else
    echo "Installing op_askpass from GitHub"
    cargo install op_askpass --git https://github.com/chrismooredev/op_askpass
fi

# Set SSH_ASKPASS_REQUIRE to "force" to ensure it is used even in non-graphical environments
# Set SSH_ASKPASS to op_askpass
# Set this in each shell rc file, conditionally on it not already being set, to avoid overwriting existing configurations
# If an rc file has a different value, log it and fail at the end - do not overwrite

conflict=0

configure_rc() {
    local rc="$1"
    local name
    name="$(basename "$rc")"
    if grep -qs "export SSH_ASKPASS=op_askpass" "$rc"; then
        echo "SSH_ASKPASS is already set to op_askpass in $name, skipping"
    elif grep -qs "export SSH_ASKPASS=" "$rc"; then
        echo "SSH_ASKPASS is set to a different value in $name, please resolve manually" >&2
        conflict=1
    else
        echo "export SSH_ASKPASS=op_askpass" >> "$rc"
        echo "export SSH_ASKPASS_REQUIRE=force" >> "$rc"
        echo "SSH_ASKPASS configured in $name"
    fi
}

if [ -f "$HOME/.bashrc" ]; then
    configure_rc "$HOME/.bashrc"
fi

# zsh is the default shell on macOS, where ~/.zshrc may not exist yet
if [ -f "$HOME/.zshrc" ] || [ "$(basename "${SHELL:-}")" = "zsh" ]; then
    configure_rc "$HOME/.zshrc"
fi

cat <<'EOF'

Note: your current shell may still need configuring. op_askpass requires:
    export SSH_ASKPASS=op_askpass
    export SSH_ASKPASS_REQUIRE=force
- bash: run `source ~/.bashrc` or open a new terminal
- zsh: run `source ~/.zshrc` or open a new terminal
- other shells: add the lines above to your shell's startup file
EOF

exit "$conflict"

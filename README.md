
# op_askpass

An SSH `askpass` implementation that wraps around the [1Password CLI](https://developer.1password.com/docs/cli).

# Usage

It is driven by OpenSSH, and the 1Password [secret reference](https://developer.1password.com/docs/cli/secret-references) to read is driven by a custom `OnepassUri` directive in the configuration:

```ssh_config
IgnoreUnknown OnepassUri

Host myserver
    User myuser
    OnepassUri "op://Vault/My Server/password"
```

# Installation

On macOS or Linux, install with:

```bash
curl -fsSL https://raw.githubusercontent.com/chrismooredev/op_askpass/main/install_op_askpass.sh | bash -
```

On Windows PowerShell, install with:

```powershell
irm https://raw.githubusercontent.com/chrismooredev/op_askpass/main/install_op_askpass.ps1 | iex
```

When run from a checkout of this repository, the scripts install from the local source instead of GitHub.

## Shell configuration

`op_askpass` is only used by SSH when these environment variables are set:

```bash
export SSH_ASKPASS=op_askpass
export SSH_ASKPASS_REQUIRE=force
```

- **bash** / **zsh**: the install script appends these to `~/.bashrc` and/or `~/.zshrc`. Already-open shells won't pick them up; run `source ~/.bashrc` / `source ~/.zshrc` or open a new terminal.
- **Other shells**: add the lines above to your shell's startup file yourself, then restart the shell.
- **fish**: `set -Ux SSH_ASKPASS op_askpass; set -Ux SSH_ASKPASS_REQUIRE force`
- **PowerShell**: the install script sets both as user environment variables and in the current session. Other already-open terminals need to be restarted.

# Just use keys......

If keys work for you, great! Check out the [1Password-authored SSH Agent](https://developer.1password.com/docs/ssh/agent/)

If you operate a wide environment whose authentication is centrally managed via a RADIUS server or similar, keys may not be practical due to configuration management or security concerns. Other niche or externally-managed SSH servers may also make key management difficult. This is where this project comes in use: when key management isn't practical but legacy username/password tooling still works fine.

# Common Issues
- Other SSH configuration parsers may not be tolerant of IgnoreUnknown/OnepassUri. This tooling is designed for OpenSSH
    - You may consider using `Match exec` to optionally `Include` an OpenSSH config file depending on the parent process/parser. You can use the `op_askpass --is-ssh` command in your exec string for this.
    - Parsers that unconditionally include for syntax/etc checks may still not play nicely here.

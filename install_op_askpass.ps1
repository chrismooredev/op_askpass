
cargo install op_askpass --git https://github.com/chrismooredev/op_askpass

$exe = (Get-Command op_askpass).Source

# Sets local env if sourced
$env:SSH_ASKPASS = $exe
$env:SSH_ASKPASS_REQUIRE = "force"

# Sets profile defaults
[Environment]::SetEnvironmentVariable("SSH_ASKPASS", $exe, "User")
[Environment]::SetEnvironmentVariable("SSH_ASKPASS_REQUIRE", "force", "User")


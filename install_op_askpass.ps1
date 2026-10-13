
if ((Test-Path "Cargo.toml") -and (Select-String -Path "Cargo.toml" -Pattern '^name = "op_askpass"' -Quiet)) {
    Write-Host "Installing op_askpass from local checkout"
    cargo install --path .
} else {
    Write-Host "Installing op_askpass from GitHub"
    cargo install op_askpass --git https://github.com/chrismooredev/op_askpass
}
if ($LASTEXITCODE -ne 0) { throw "cargo install failed" }

$exe = (Get-Command op_askpass).Source

# Sets env for the current process/session
$env:SSH_ASKPASS = $exe
$env:SSH_ASKPASS_REQUIRE = "force"

# Sets profile defaults
[Environment]::SetEnvironmentVariable("SSH_ASKPASS", $exe, "User")
[Environment]::SetEnvironmentVariable("SSH_ASKPASS_REQUIRE", "force", "User")

Write-Host @"

Note: other already-open terminals may still need configuring. Restart them, or run:
    `$env:SSH_ASKPASS = "$exe"
    `$env:SSH_ASKPASS_REQUIRE = "force"
"@


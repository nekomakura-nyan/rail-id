[CmdletBinding()]
param(
    [switch]$Tpm,
    [switch]$NoTpm,
    [switch]$SkipTests,
    [switch]$SkipClippy,
    [switch]$SkipDoc
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# `-NoTpm` is retained as a compatibility no-op: the default build has no TPM.
$featureArgs = @()
if ($Tpm) {
    $featureArgs = @("--features", "tpm")
}

function Invoke-Checked {
    param([string]$Label, [scriptblock]$Action)
    Write-Host "== $Label =="
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
}

Invoke-Checked "cargo fmt --check" { & cargo fmt --check }
if (-not $SkipClippy) {
    Invoke-Checked "cargo clippy" { & cargo clippy --all-targets @featureArgs -- -D warnings }
}
if (-not $SkipTests) {
    Invoke-Checked "cargo test" { & cargo test @featureArgs }
    if ($Tpm) {
        Invoke-Checked "cargo test (railid-tpm)" {
            & cargo test --manifest-path railid-tpm/Cargo.toml --features windows
        }
    }
}
Invoke-Checked "cargo build --release" { & cargo build --release @featureArgs }
if (-not $SkipDoc) {
    $previous = $env:RUSTDOCFLAGS
    $env:RUSTDOCFLAGS = "-D warnings"
    try {
        Invoke-Checked "cargo doc" { & cargo doc --no-deps @featureArgs }
    } finally {
        $env:RUSTDOCFLAGS = $previous
    }
}

Write-Host "railid checks completed."

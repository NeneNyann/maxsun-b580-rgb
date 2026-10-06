$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $Root

$Required = @(
    'driver\IntelXeGraphicI2cDeviceDriver.sys',
    'driver\IntelXeGraphicI2cDeviceDriver.inf',
    'driver\IntelXeGraphicI2cDeviceDriver.cat'
)

if ($Required | Where-Object { -not (Test-Path -LiteralPath $_) }) {
    & (Join-Path $Root 'get-driver.ps1')
}

# Hide user and checkout paths in compiler-generated output.
$PreviousEncodedFlags = $env:CARGO_ENCODED_RUSTFLAGS
$RustFlags = @()
if ($null -ne $PreviousEncodedFlags) {
    $RustFlags = @($PreviousEncodedFlags -split [char]0x1f)
} elseif (-not [string]::IsNullOrWhiteSpace($env:RUSTFLAGS)) {
    $RustFlags = @($env:RUSTFLAGS.Trim() -split '\s+')
}
$UserProfilePath = [Environment]::GetFolderPath('UserProfile')
if (-not [string]::IsNullOrWhiteSpace($UserProfilePath)) {
    $RustFlags += "--remap-path-prefix=$UserProfilePath=."
}
$RustFlags += "--remap-path-prefix=$Root=."

try {
    # Encoded flags preserve paths containing spaces.
    $env:CARGO_ENCODED_RUSTFLAGS = $RustFlags -join [char]0x1f
    cargo build --release
    $BuildExitCode = $LASTEXITCODE
} finally {
    $env:CARGO_ENCODED_RUSTFLAGS = $PreviousEncodedFlags
}
if ($BuildExitCode -ne 0) { exit $BuildExitCode }

Write-Host "Built: $Root\target\release\maxsun-b580-rgb.exe"

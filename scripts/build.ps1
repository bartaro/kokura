param([switch]$Offline)
$ErrorActionPreference = 'Stop'
# Resolve paths from this script so the command works from any working directory.
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$previousRustFlags = $env:RUSTFLAGS
Push-Location $repositoryRoot
try {
    # Append static MSVC runtime linkage for the locked Windows x64 CLI release build.
    $env:RUSTFLAGS = (($previousRustFlags + ' -C target-feature=+crt-static').Trim())
    $cargoArgs = @('build','--release','--locked','--target','x86_64-pc-windows-msvc','-p','kokura-cli','--bin','kokura-cli')
    if ($Offline) { $cargoArgs += '--offline' }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    # Read Cargo metadata offline to honor its configured target directory after the build.
    $metadataText = & cargo metadata --no-deps --format-version 1 --locked --offline
    if ($LASTEXITCODE -ne 0) { throw 'Unable to locate the Cargo target directory' }
    $metadata = $metadataText | ConvertFrom-Json
    $binary = Join-Path $metadata.target_directory 'x86_64-pc-windows-msvc/release/kokura-cli.exe'
    # Publish the successfully built executable at the documented repository-root location.
    Copy-Item -LiteralPath $binary -Destination (Join-Path $repositoryRoot 'kokura-cli.exe') -Force
    Write-Host 'Ready: kokura-cli.exe (Windows x64 CLI)'
} finally {
    # Restore the caller's flags and working directory even after a failed build or copy.
    $env:RUSTFLAGS = $previousRustFlags
    Pop-Location
}

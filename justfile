set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

[private]
default:
    @just --list

# Build the release binary and copy BeamMP-Panel.exe into dist.
[script("powershell.exe", "-NoLogo", "-NoProfile", "-File")]
build:
    $ErrorActionPreference = "Stop"
    cargo build --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $built = "target\release\BeamMP-Panel.exe"
    if (-not (Test-Path -LiteralPath $built)) { Write-Error "Missing $built"; exit 1 }

    if (Test-Path -LiteralPath dist) { Remove-Item -LiteralPath dist -Recurse -Force }
    New-Item -ItemType Directory -Force -Path dist | Out-Null
    Copy-Item -Force -LiteralPath $built -Destination "dist\BeamMP-Panel.exe"

    Write-Output "exe: dist\BeamMP-Panel.exe"

# Remove build artifacts and the dist folder.
clean:
    cargo clean
    if (Test-Path -LiteralPath dist) { Remove-Item -LiteralPath dist -Recurse -Force }

# Run the release build.
run:
    cargo run --release

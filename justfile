set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

[private]
default:
    @just --list

# Build the Windows x64 portable app into dist/portable.
build:
    powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/tasks.ps1 build

# Build the app and an Inno Setup 7 installer into dist/installer.
installer:
    powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/tasks.ps1 installer

# Check formatting and lints, then build the test fixture and run all tests.
check:
    powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/tasks.ps1 check

# Remove Cargo artifacts and generated distribution files.
clean:
    powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/tasks.ps1 clean

# Run the app on the current platform.
run:
    cargo run --locked --release

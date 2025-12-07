# Build Instructions

## Prerequisites

### Required
- Rust 1.70 or later (install from https://rustup.rs/)

### For MSI Installer (Windows only)
- WiX Toolset v3.11 or later (download from https://wixtoolset.org/)
  - Or install via winget: `winget install WiX.Toolset`
- cargo-wix: `cargo install cargo-wix`

## Building the Application

### Standard Build

**Debug build:**
```bash
cargo build
```
Output: `target/debug/beammp-panel.exe`

**Release build (optimized):**
```bash
cargo build --release
```
Output: `target/release/beammp-panel.exe`

### Building MSI Installer

1. **Ensure WiX Toolset is installed and in PATH:**
```bash
# Test WiX installation
candle.exe -?
```

2. **Install cargo-wix (one-time setup):**
```bash
cargo install cargo-wix
```

3. **Build the MSI installer:**
```bash
# This will build the release binary and create the MSI
cargo wix
```

Output: `target/wix/beammp-panel-0.1.0-x86_64.msi`

### MSI Installer Features

The installer includes:
- Main application executable
- Start Menu shortcut
- Desktop shortcut
- Automatic PATH updates (optional)
- Clean uninstall support

### Build Options

**Clean build:**
```bash
cargo clean
cargo build --release
```

**Check without building:**
```bash
cargo check
```

**Run tests:**
```bash
cargo test
```

## Customizing the Installer

Edit `wix/main.wxs` to customize:
- Product name and version
- Installation directory
- Shortcuts
- Registry keys
- Additional files

After making changes, rebuild with `cargo wix`.

## Troubleshooting

**Error: "candle.exe not found"**
- Ensure WiX Toolset is installed
- Add WiX bin directory to PATH: `C:\Program Files (x86)\WiX Toolset v3.11\bin`

**Error: "cargo-wix not found"**
- Install with: `cargo install cargo-wix`
- Ensure `~/.cargo/bin` is in PATH

**MSI build fails:**
- Ensure release binary exists: `cargo build --release`
- Check that `target/release/beammp-panel.exe` is present
- Verify WiX configuration in `wix/main.wxs`

## Distribution

To distribute your application:
1. Build the MSI installer: `cargo wix`
2. Test the installer on a clean Windows machine
3. Distribute the MSI file from `target/wix/`

Users can install by double-clicking the MSI file or via command line:
```bash
msiexec /i beammp-panel-0.1.0-x86_64.msi
```


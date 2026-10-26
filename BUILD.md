# Build Instructions

## Prerequisites

### Required
- Rust 1.92 or later (install from https://rustup.rs/)

### For the installer (Windows only)
- WiX Toolset 3.11 or a newer 3.x release, which provides `candle.exe` and `light.exe` (https://wixtoolset.org/docs/wix3/)
  - WiX 4 and later are a different toolchain and are not used here
  - The script looks on `PATH`, in the `WIX` environment variable, and under `Program Files\WiX Toolset v3.14` and `v3.11` (including `Program Files (x86)`)

## Building the Application

### Standard Build

**Debug build:**
```bash
cargo build
```
Output: `target/debug/BeamMP-Panel.exe`

**Release build (optimized):**
```bash
cargo build --release
```
Output: `target/release/BeamMP-Panel.exe`

### Release build and installer

Running `just` with no arguments lists the recipes (`build`, `clean`, `run`).

`just build` builds the release binary, then writes a portable exe and an MSI into `dist` at the repo root. The version comes from `Cargo.toml`. The platform name comes from the rustc host triple, so an x64 Windows build looks like this:

```bash
just build
```

Outputs in `dist/`:
- `BeamMP-Panel-0.3.0-windows-x86_64-portable.exe` — portable exe
- `BeamMP-Panel-0.3.0-windows-x86_64.msi` — installer

`just run` runs the release build (`cargo run --release`).

`just clean` removes Cargo build output (`cargo clean`) and deletes `dist`.

The WiX source is `packaging/BeamMP-Panel.wxs`. `just build` passes the Cargo version in as `Version`. To compile the installer by hand after `cargo build --release`, run `candle.exe` and `light.exe` on that file (see the `build` recipe in the justfile). The 32-bit edition of WiX Toolset 3 installs under `C:\Program Files (x86)\WiX Toolset v3.14` instead.

### Installer features

The installer includes:
- Main application executable
- Start Menu shortcut
- Optional desktop shortcut
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

Besides unit tests, this runs end-to-end tests that start, stop, crash and
flood a stand-in server built from `examples/fake_server.rs`. To try the UI
without a real server, build it with `cargo build --example fake_server`,
copy `target/debug/examples/fake_server.exe` into a folder as
`BeamMP-Server.exe` next to a `ServerConfig.toml`, and add that folder in the
panel.

## Customizing the Installer

Edit `packaging/BeamMP-Panel.wxs` to customize:
- Product name and publisher
- Installation directory
- Shortcuts
- Additional files

Keep `UpgradeCode` stable across releases. The product version is supplied by `just build` from `Cargo.toml`. After making changes, run `just build` again.

## Troubleshooting

**Error: "WiX Toolset 3 (candle.exe) not found"**
- Install WiX Toolset 3.11 or a newer 3.x release
- `candle.exe` is found on `PATH`, via the `WIX` environment variable, or in the default install folders

**Installer build fails because the exe is missing**
- Build the release binary first: `cargo build --release`
- Check that `target/release/BeamMP-Panel.exe` is present

## Distribution

To distribute the application:
1. Run `just build`
2. Test the installer on a clean Windows machine
3. Distribute `dist/BeamMP-Panel-<version>-windows-<arch>.msi`, or `dist/BeamMP-Panel-<version>-windows-<arch>-portable.exe` for a portable copy

Users install by running that MSI.


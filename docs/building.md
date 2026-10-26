# Building and packaging

## Prerequisites

- Rust 1.92 or newer, with the `rustfmt` and `clippy` components.
- For Windows packages: the `x86_64-pc-windows-msvc` Rust target, Visual Studio C++ Build Tools, and a Windows SDK.
- For the installer: [Inno Setup 7](https://jrsoftware.org/isdl.php), either compiler edition. Inno Setup 6 is not supported by the packaging task.
- Optional: [`just`](https://github.com/casey/just) for the command shortcuts below.

```powershell
rustup component add rustfmt clippy
rustup target add x86_64-pc-windows-msvc
```

## Windows tasks

Run from the repository root:

| Command | Result |
| --- | --- |
| `just build` | Release executable and license in `dist/portable/` |
| `just installer` | Builds the portable app, then `dist/installer/BeamMP-Panel-<version>-windows-x64-setup.exe` |
| `just check` | Formatting check, Clippy with warnings treated as errors, fixture build, and tests |
| `just run` | Runs the release app on the current platform |
| `just clean` | Removes Cargo build output and `dist/` |

`just` delegates to one PowerShell task runner. Without `just`, run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/tasks.ps1 installer
```

Replace `installer` with `build`, `check`, or `clean` as needed. The execution policy setting applies only to that PowerShell process. The script resolves paths relative to itself, so an absolute script path also works from another directory.

The compiler is discovered in Inno Setup 7's standard installation locations, then on `PATH`. For a custom installation, set `ISCC` to the full path of `ISCC.exe`, or pass it explicitly:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/tasks.ps1 installer -IsccPath 'D:\Tools\Inno Setup 7\ISCC.exe'
```

Builds use the committed `Cargo.lock` and explicitly select Windows x64 MSVC. The repository's Cargo configuration links the MSVC runtime statically so users do not need a separate Visual C++ runtime installation. The script uses Cargo metadata to locate build output, including a custom `CARGO_TARGET_DIR`. Building the portable app preserves existing installers and other files in `dist/`.

## Installer behavior

The installer source is `installer/BeamMP-Panel.iss`. Its application version comes from the executable's Windows resources, which `winresource` derives from `Cargo.toml`. Change the Cargo package version for a release; keep the installer `AppId` unchanged so future versions upgrade the same installation.

The default location is `%LOCALAPPDATA%\Programs\BeamMP Panel`. The installer adds a Start menu shortcut and an optional desktop shortcut. It supports Windows 10 or newer on x64-compatible systems, including Windows on Arm systems with x64 emulation. It packages only the panel and its license; obtain BeamMP Server separately.

Quit the panel from its tray menu before upgrading or uninstalling, especially while it manages running servers. Closing the main window may only hide it to the tray. Uninstall removes installed application files and shortcuts while preserving the panel's user settings and external server folders.

Generated packages are unsigned. A public release can add code signing through Inno Setup's signing configuration.

## Native builds and tests

The application can also be built directly on Windows, macOS, or Linux:

```sh
cargo build --locked --release
```

This writes the native executable into Cargo's release directory. On Linux, install your distribution's C/C++ build tools, GTK 3 development packages (for the file dialog), and graphics/window-system development libraries first. Inno Setup packaging is Windows-only.

For checks without the Windows task runner:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --example fake_server
cargo test --locked --all-targets
```

Build the fixture with the same target, profile, and target directory as the tests. `cargo test --all-targets` builds an example test harness, which is different from the standalone executable that the process tests spawn.

To try the UI without a real BeamMP server, copy the built `fake_server` executable from Cargo's `debug/examples/` directory into a scratch server folder as `BeamMP-Server.exe` (`BeamMP-Server` on Unix), alongside a `ServerConfig.toml`. This fixture implements `list`, `say`, `exit`, `crash`, `hang`, and `spam` for development only.

## Repository layout

| Path | Purpose |
| --- | --- |
| `src/` | Rust application code; UI components live in `src/ui/` |
| `icons/` | Application icons and original artwork |
| `examples/` | Fake server used by process tests and manual development |
| `scripts/` | Build, packaging, verification, and cleanup tasks |
| `installer/` | Inno Setup source, with no generated executables |
| `docs/` | Contributor and packaging documentation |
| `.cargo/config.toml` | Windows static runtime linking configuration |
| `target/` | Ignored Cargo build output |
| `dist/portable/` | Ignored portable distribution files |
| `dist/installer/` | Ignored installer output |

Use `cargo fmt --all` before submitting Rust changes. `.editorconfig` and `.gitattributes` keep text encoding, whitespace, and line endings consistent across editors and operating systems.

# BeamMP Panel

A desktop application for managing BeamMP dedicated servers, built with Rust and egui.

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/Aspenini/BeamMP-Panel?style=for-the-badge)](https://github.com/Aspenini/BeamMP-Panel/releases)
[![GitHub all releases downloads](https://img.shields.io/github/downloads/Aspenini/BeamMP-Panel/total?style=for-the-badge)](https://github.com/Aspenini/BeamMP-Panel/releases)

## Features

- **Server Management** - Register and manage multiple BeamMP server installations
- **Configuration Editor** - Edit ServerConfig.toml through an intuitive GUI interface
- **Mod Management** - Add, enable, disable, and delete mods with a visual interface
- **Server Control** - Execute server commands, manage players, and broadcast messages
- **Integrated Terminal** - Per-server console with real-time output when servers are running
- **Persistent Storage** - Server configurations saved automatically

## Requirements

- Rust 1.70 or later
- Existing BeamMP server installation(s)

## Installation

Build the application from source:

```bash
cargo build --release
```

The compiled binary will be located at `target/release/beammp-panel` (or `beammp-panel.exe` on Windows).

### Building MSI Installer (Windows)

To create a Windows MSI installer:

1. Install WiX Toolset: Download from https://wixtoolset.org/ or use `winget install WiX.Toolset`
2. Install cargo-wix:
```bash
cargo install cargo-wix
```

3. Build the installer:
```bash
cargo wix
```

The MSI installer will be created at `target/wix/beammp-panel-0.1.0-x86_64.msi`.

## Usage

### Server Management

**Adding a Server**
1. Click "Add Server" in the left panel
2. Select the folder containing your BeamMP server executable and ServerConfig.toml
3. The server will appear in the list

**Removing a Server**
1. Select the server from the list
2. Click "Remove Server"
3. Confirm the removal (files on disk are not deleted)

### Configuration

1. Select a server from the list
2. Navigate to the Config tab
3. Modify settings as needed
4. Click "Apply" to save changes or "Revert" to discard

Supported settings include port, authentication, player limits, maps, and more.

### Mod Management

1. Select a server and navigate to the Mods tab
2. Available actions:
   - **Add Mod** - Import mod files into the Resources folder
   - **Enable/Disable** - Toggle mods by moving them between Resources and Resources_disabled
   - **Delete** - Permanently remove mod files

### Server Control

**Starting a Server**
1. Select a server from the list
2. Click "Start Server" in the top-right corner
3. The server console appears at the bottom showing real-time output

**Server Commands**
Navigate to the Control tab while a server is running to access:
- Player list management
- Player kick functionality
- Server-wide message broadcasting
- Quick access to common server commands (status, version, reload mods, etc.)

All command outputs are displayed in the integrated console.

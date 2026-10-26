# BeamMP Panel

A desktop application for managing BeamMP dedicated servers, built with Rust and egui.

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/Aspenini/BeamMP-Panel?style=for-the-badge)](https://github.com/Aspenini/BeamMP-Panel/releases)
[![GitHub all releases downloads](https://img.shields.io/github/downloads/Aspenini/BeamMP-Panel/total?style=for-the-badge)](https://github.com/Aspenini/BeamMP-Panel/releases)

## Features

- **Multiple servers at once** - Register any number of BeamMP server folders and run several simultaneously, each with its own console
- **Runs in the system tray** - Closing the window keeps your servers running in the tray (configurable; Windows and macOS)
- **Configuration editor** - Edit ServerConfig.toml without losing its comments, formatting or settings the panel doesn't know about
- **Mod management** - Add (button or drag & drop), enable, disable, inspect and delete client mods; manage server plugins
- **Live console** - Colored server output, a command line with history, copy-to-clipboard
- **Server control** - Player list, kick, broadcast, quick commands
- **Reliability** - Graceful stop with force-kill fallback, optional auto-start and crash auto-restart, and servers never outlive the panel as hidden orphan processes (Windows)

## Requirements

- Windows 10/11 (primary), macOS or Linux
- Rust 1.92 or later to build
- Existing BeamMP server installation(s)

## Installation

Build the application from source:

```bash
cargo build --release
```

The compiled binary will be located at `target/release/BeamMP-Panel` (or `BeamMP-Panel.exe` on Windows). See [BUILD.md](BUILD.md) for the MSI installer.

## Usage

### Servers

- Click **➕** in the left panel and pick the folder containing `BeamMP-Server.exe` and `ServerConfig.toml`.
- Right-click a server for Start/Stop, Open folder and Remove (removing never deletes files).
- **Start**, **Stop**, **Restart** and **Force stop** are in the top-right corner. Stop asks the server to `exit` and force-kills it after the stop timeout.
- The dot next to each server shows its state: green = running, yellow = stopping/restarting, grey = stopped.

### Console

The console appears below the tabs once a server has been started, and keeps its log after the server exits. Type any server command in the box at the bottom (Enter to send, Up/Down for history).

### Configuration

Edit settings in the **Config** tab and press **Save** (Ctrl+S). Only the fields you changed are written, so comments in ServerConfig.toml and edits made by the server itself are kept. Restart the server to apply changes.

The **Panel** section at the bottom holds per-server panel options:

- **Start this server when the panel opens**
- **Restart automatically if it crashes** (pauses after 3 crashes within 5 minutes)

### Mods

- **Client** mods are `.zip` files in `Resources/Client`. Add them with **Add mods...** or by dropping files onto the window. Each mod is tagged as a Level and/or Vehicle; filter and search the list, and click ℹ for details.
- **Server** mods are Lua plugin folders in `Resources/Server`.
- Unchecking a mod disables it by moving it to `Resources_disabled`; checking it moves it back.

### Settings

Open **⚙ Settings** at the bottom of the server list:

- **When the window is closed** - minimize to the system tray (default) or quit
- **Start minimized to the tray** - also available as the `--minimized` command line flag, handy for launching at login
- **Ask before quitting while servers are running**
- Theme, console history length and stop timeout

Quitting the panel (tray menu → Quit, or closing the window when tray mode is off) stops all running servers first. Launching the panel while it is already running brings the existing window to the front.

Panel data lives in `%APPDATA%\BeamMP-Panel` on Windows (`servers.json`, `settings.json`). Set the `BEAMMP_PANEL_CONFIG_DIR` environment variable to keep it somewhere else, e.g. next to the exe for a portable install.

### Performance

The panel only redraws when something changes (new console output on screen, a click, a timer), and does essentially nothing while hidden in the tray. It renders with OpenGL for low memory use and automatically falls back to wgpu (DirectX/Vulkan) on machines without OpenGL, such as Remote Desktop sessions on GPU-less servers.

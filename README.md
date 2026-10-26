# BeamMP Panel

A desktop app for running and configuring [BeamMP](https://beammp.com) dedicated servers. Built with Rust and egui.

[![Release](https://img.shields.io/github/v/release/Aspenini/BeamMP-Panel)](https://github.com/Aspenini/BeamMP-Panel/releases)
[![Downloads](https://img.shields.io/github/downloads/Aspenini/BeamMP-Panel/total)](https://github.com/Aspenini/BeamMP-Panel/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Point it at a BeamMP server folder you already have. Server setup itself is covered in the [BeamMP server guide](https://docs.beammp.com/server/server-installation/).

## Install

**Windows 10 or later (x64).** Download the `BeamMP-Panel-<version>-windows-x64-setup.exe` installer from [releases](https://github.com/Aspenini/BeamMP-Panel/releases), or build it with the steps below. It installs for your Windows account without administrator access, adds a Start menu shortcut, and offers an optional desktop shortcut. Uninstall it through Windows Settings; your server folders and panel settings are preserved.

For a portable copy, use `BeamMP-Panel.exe` instead. It runs without installation. See [Data](#data) to keep its settings alongside the executable.

The tray icon works on Windows and macOS. On Linux, closing the window quits the panel.

**macOS and Linux.** Build from source. Release packages are published for Windows.

## Add a server

1. Click **+ Add** and choose the folder that contains `ServerConfig.toml` and `BeamMP-Server` (`BeamMP-Server.exe` on Windows).
2. On the **Config** tab, set the auth key from the [BeamMP Keymaster](https://keymaster.beammp.com) and save.
3. Click **Start server**.

The dot next to a server is green while it runs, yellow while it stops or restarts, and grey while it is stopped. Several servers can run at once. Each keeps its own console.

Right-click a server for **Start**, **Stop**, **Open folder**, or **Remove from panel**. Remove forgets the folder in the panel. The files stay on disk. A running server has to be stopped before it can be removed.

**Stop** sends `exit`, then kills the process if it is still up when the stop timeout ends. **Force stop** appears during that wait and kills it immediately. **Restart** stops, then starts again. **Cancel restart** aborts a countdown after a crash.

On Windows, server processes are bound to the panel, so they exit with it.

## Config

The **Config** tab edits `ServerConfig.toml`: name, description, tags, map, privacy, auth key, guests, player and car limits, port, bind address, resource folder, logging, and update reminders. The auth key stays hidden until you click **Show**. The map list includes the stock BeamNG levels. A mod map is a path such as `/levels/<folder>/info.json`. The server name may use BeamMP `^` color codes. The sidebar shows the name with those codes removed.

**Save changes** (Ctrl+S, or Cmd+S on macOS) writes the fields you edited and leaves the rest of the file alone, including comments and keys the panel does not show. Edits the server wrote to disk after the file was loaded are kept. **Revert** drops unsaved edits. **Reload from disk** reads the file again. Restart the server to apply a saved config.

Two options are stored by the panel, in `servers.json`:

- **Start this server when the panel opens**
- **Restart automatically if it crashes.** It waits 5 seconds, then starts again. After 3 crashes within 5 minutes it stops trying.

## Mods

Switch between **Client mods** and **Server plugins**.

Client mods are `.zip` files in `<resource folder>/Client`, usually `Resources/Client`. **Add mods** copies zips in. You can also drop zip files on the window while this tab is open. Levels and vehicles are tagged, so the list can be filtered and searched. Open a mod for its contents, or delete it.

Server plugins are folders in `<resource folder>/Server`.

Clearing the checkbox moves a mod into `<resource folder>_disabled` (usually `Resources_disabled`). Checking it moves the mod back. **Reload plugins** on the Control tab reloads Lua plugins on a running server.

## Control and console

The **Control** tab works while the server is running.

- A player list from the server `list` command, with **Kick** on each row
- Kick by name, with an optional reason
- A broadcast box that sends `say`
- **Status**, **Version**, **Reload plugins**, **Help**, and **Lua console** (`lua`; type `exit()` to leave it)

The console sits under the tabs after a server has been started, and stays after it exits. Lines are colored by level. Type a command and press Enter. Up and Down recall the last 100 commands. **Copy** copies the log, **Clear** empties the view, **Open folder** opens the server directory (`Server.log` is there), and **Auto-scroll** follows new lines.

## Settings

**Settings** is at the bottom of the server list. Changes are saved as you make them.

| Setting | Default |
| --- | --- |
| When the window is closed | Keep running in the system tray |
| Start minimized to the tray | Off. The `--minimized` flag does this for one launch. |
| Ask before quitting while servers run | On |
| Theme | System, Light, or Dark |
| Console history | 5,000 lines (500 to 100,000) |
| Stop timeout | 10 seconds (2 to 120) |

On Windows and macOS, closing the window hides it. Left-click the tray icon, or choose **Show BeamMP Panel**, to open it again. **Quit** stops every running server, then exits. A second launch shows the window that is already running.

## Data

The server list (`servers.json`) and settings (`settings.json`) are stored here:

| System | Folder |
| --- | --- |
| Windows | `%APPDATA%\BeamMP-Panel\config` |
| macOS | `~/Library/Application Support/BeamMP-Panel` |
| Linux | `~/.config/beammp-panel` |

Window size and position are remembered separately: `%APPDATA%\beammp-panel\data` on Windows, `~/Library/Application Support/beammp-panel` on macOS, and `~/.local/share/beammp-panel` on Linux.

Set `BEAMMP_PANEL_CONFIG_DIR` to keep the server list, settings, and window state (`window.ron`) in one folder. Point it at a directory next to the exe when you want that copy to carry its own servers.

## Build

Rust 1.92 or newer.

```bash
cargo build --release
```

The binary is `target/release/BeamMP-Panel` (`BeamMP-Panel.exe` on Windows).

On Windows, `just build` creates `dist/portable/BeamMP-Panel.exe`, and `just installer` builds the app and an installer using **Inno Setup 7**. `just check` runs formatting, lints, and tests. See [Building and packaging](docs/building.md) for prerequisites, commands without `just`, output locations, and the repository layout.

The window redraws when something changes, and the panel stays idle while it is hidden in the tray. It renders with OpenGL, and uses DirectX or Vulkan when OpenGL is unavailable, such as a Remote Desktop session on a machine without a GPU.

## License

[MIT](LICENSE)

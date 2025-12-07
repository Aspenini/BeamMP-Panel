# BeamMP Server Manager

A desktop application for managing BeamMP dedicated servers, built with Rust and egui.

## Features

- **Server Management**: Register and manage multiple BeamMP server installations
- **Config Editor**: Edit `ServerConfig.toml` through an intuitive GUI
- **Mod Management**: 
  - Add new mods via file picker
  - Enable/disable mods by moving between `Resources` and `Resources_disabled`
  - Delete mods with confirmation
- **Persistent Storage**: Server list saved in your user config directory

## Requirements

- Rust 1.70+ (2021 edition or later)
- A BeamMP server installation (the app manages existing servers, it doesn't create them)

## Building

```bash
cargo build --release
```

The executable will be in `target/release/beammp-manager` (or `beammp-manager.exe` on Windows).

## Running

```bash
cargo run --release
```

Or run the compiled executable directly from `target/release/`.

## Usage

### Adding a Server

1. Click **"Add Server"** in the left panel
2. Select the folder containing your BeamMP server (must have `ServerConfig.toml`)
3. The server will appear in the list

### Editing Configuration

1. Select a server from the list
2. Go to the **Config** tab
3. Edit any settings you want
4. Click **Apply** to save changes to `ServerConfig.toml`
5. Click **Revert** to discard unsaved changes

### Managing Mods

1. Select a server from the list
2. Go to the **Mods** tab
3. Use the buttons to:
   - **Add Mod**: Copy files into the `Resources` folder
   - **Disable**: Move mod from `Resources` to `Resources_disabled`
   - **Enable**: Move mod back from `Resources_disabled` to `Resources`
   - **Delete**: Permanently remove the mod file

## Data Storage

The app stores your server list in:
- **Windows**: `C:\Users\<USERNAME>\AppData\Roaming\BeamMP-Manager\servers.json`
- **Linux**: `~/.config/BeamMP-Manager/servers.json`
- **macOS**: `~/Library/Application Support/BeamMP-Manager/servers.json`

## License

MIT or Apache-2.0 (choose your preferred license)


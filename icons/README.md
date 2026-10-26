# Application icons

- `BeamMP-Panel.jxl`: original artwork.
- `BeamMP-Panel.ico`: Windows executable, window, tray, and installer icon. It contains 16, 24, 32, 48, 64, 128, and 256 pixel frames.
- `BeamMP-Panel.icns`: macOS icon asset.

Keep ICO frames at 256 pixels or smaller, with directory dimensions matching the embedded images. A 1024 pixel PNG wrapped in an ICO header is rejected by Inno Setup's resource updater.

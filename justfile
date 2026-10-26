set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

[private]
default:
    @just --list

# Build the release binary and write the portable exe and MSI into dist.
[script("powershell.exe", "-NoLogo", "-NoProfile", "-File")]
build:
    $ErrorActionPreference = "Stop"
    cargo build --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $meta = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $pkg = $meta.packages | Where-Object { $_.name -eq "BeamMP-Panel" } | Select-Object -First 1
    if (-not $pkg) { Write-Error "Could not read the BeamMP-Panel package from Cargo.toml"; exit 1 }
    $name = $pkg.name
    $version = $pkg.version

    $hostLine = rustc -vV | Select-String -Pattern "^host:"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $parts = (($hostLine.Line -split "\s+")[1] -split "-")
    $platform = "{0}-{1}" -f $parts[2], $parts[0]
    $wixArch = switch ($parts[0]) {
        "x86_64" { "x64" }
        "aarch64" { "arm64" }
        "i686" { "x86" }
        default { Write-Error "Unsupported host architecture: $($parts[0])"; exit 1 }
    }

    $portable = "$name-$version-$platform-portable.exe"
    $msi = "$name-$version-$platform.msi"
    $built = "target\release\$name.exe"
    if (-not (Test-Path -LiteralPath $built)) { Write-Error "Missing $built"; exit 1 }

    if (Test-Path -LiteralPath dist) { Remove-Item -LiteralPath dist -Recurse -Force }
    if (Test-Path -LiteralPath target\dist) { Remove-Item -LiteralPath target\dist -Recurse -Force }
    New-Item -ItemType Directory -Force -Path dist, target\wix | Out-Null
    Copy-Item -Force -LiteralPath $built -Destination (Join-Path dist $portable)

    $candidates = @()
    $onPath = Get-Command candle.exe -ErrorAction SilentlyContinue
    if ($onPath) { $candidates += $onPath.Source }
    if ($env:WIX) {
        $candidates += Join-Path $env:WIX "candle.exe"
        $candidates += Join-Path $env:WIX "bin\candle.exe"
    }
    foreach ($root in @($env:ProgramFiles, ${env:ProgramFiles(x86)}) ) {
        if (-not $root) { continue }
        $candidates += Join-Path $root "WiX Toolset v3.14\bin\candle.exe"
        $candidates += Join-Path $root "WiX Toolset v3.11\bin\candle.exe"
    }
    $candle = $candidates | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
    if (-not $candle) { Write-Error "WiX Toolset 3 (candle.exe) not found. Install WiX Toolset 3.11 or newer 3.x."; exit 1 }
    $light = Join-Path (Split-Path -Parent $candle) "light.exe"

    & $candle -nologo -arch $wixArch -ext WixUIExtension "-dVersion=$version" "-dCargoTargetBinDir=target\release" -out target\wix\main.wixobj packaging\BeamMP-Panel.wxs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & $light -nologo -spdb -ext WixUIExtension -sice:ICE38 -sice:ICE43 -sice:ICE57 -out (Join-Path dist $msi) target\wix\main.wixobj
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Output "portable: dist\$portable"
    Write-Output "installer: dist\$msi"

# Remove build artifacts and the dist folder.
clean:
    cargo clean
    if (Test-Path -LiteralPath dist) { Remove-Item -LiteralPath dist -Recurse -Force }

# Run the release build.
run:
    cargo run --release

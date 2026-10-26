#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet('build', 'installer', 'check', 'clean')]
    [string] $Task = 'build',

    # An explicit compiler path takes precedence over discovery.
    [string] $IsccPath = $env:ISCC
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$distributionDir = Join-Path $repoRoot 'dist'
$windowsTarget = 'x86_64-pc-windows-msvc'

function Invoke-Cargo {
    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "cargo $($args -join ' ') failed (exit $LASTEXITCODE)."
    }
}

function Find-InnoCompiler {
    if ($IsccPath) {
        $candidates = @($IsccPath)
    } else {
        $candidates = @()
        foreach ($root in @($env:ProgramFiles, ${env:ProgramFiles(x86)}, $env:LOCALAPPDATA)) {
            if ($root) {
                $candidates += Join-Path $root 'Inno Setup 7\ISCC.exe'
                $candidates += Join-Path $root 'Programs\Inno Setup 7\ISCC.exe'
            }
        }
        $onPath = Get-Command ISCC.exe -CommandType Application -ErrorAction SilentlyContinue
        if ($onPath) { $candidates += $onPath.Source }
    }

    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            $compiler = (Resolve-Path -LiteralPath $candidate).Path
            # ISCC's PE file version may be 0.0.0.0; ask the compiler engine.
            $compilerVersion = & $compiler --version
            if ($LASTEXITCODE -eq 0 -and $compilerVersion -match '^7\.\d+\.\d+') {
                return $compiler
            }
        }
    }
    throw 'Inno Setup 7 was not found. Install it or pass -IsccPath C:\path\to\ISCC.exe (or set ISCC).'
}

function Build-Portable {
    # An explicit target prevents CARGO_BUILD_TARGET from packaging another architecture.
    Invoke-Cargo build --locked --release --bin $package.name --target $windowsTarget
    $binary = Join-Path $metadata.target_directory "$windowsTarget\release\$($package.name).exe"
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "Build succeeded but the executable is missing: $binary"
    }
    $portableDir = Join-Path $distributionDir 'portable'
    New-Item -ItemType Directory -Force -Path $portableDir | Out-Null
    Copy-Item -LiteralPath $binary -Destination $portableDir -Force
    Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination $portableDir -Force
    Write-Host "Portable app: $portableDir"
}

Push-Location -LiteralPath $repoRoot
try {
    if ($Task -in @('build', 'installer')) {
        if ($env:OS -ne 'Windows_NT') { throw 'Windows packaging requires Windows and the MSVC build tools.' }
        # Cargo resolves target_directory, including CARGO_TARGET_DIR and Cargo config overrides.
        $metadata = (Invoke-Cargo metadata --locked --no-deps --format-version 1) | ConvertFrom-Json
        $package = $metadata.packages | Where-Object { $_.manifest_path -eq (Join-Path $repoRoot 'Cargo.toml') }
        if (-not $package) { throw 'Could not find the root package in Cargo metadata.' }
    }

    switch ($Task) {
        'build' { Build-Portable }
        'installer' {
            # Fail before building if the compiler is missing or from another major version.
            $compiler = Find-InnoCompiler
            Build-Portable
            & $compiler (Join-Path $repoRoot 'installer\BeamMP-Panel.iss')
            if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed (exit $LASTEXITCODE)." }
            Write-Host "Installer: $(Join-Path $distributionDir 'installer')"
        }
        'check' {
            Invoke-Cargo fmt --all '--' --check
            Invoke-Cargo clippy --locked --all-targets '--' -D warnings
            # Cargo's test harness for examples is not the executable these tests spawn.
            Invoke-Cargo build --locked --example fake_server
            Invoke-Cargo test --locked --all-targets
        }
        'clean' {
            Invoke-Cargo clean
            if (Test-Path -LiteralPath $distributionDir) {
                $resolved = (Resolve-Path -LiteralPath $distributionDir).Path
                $expected = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'dist'))
                $item = Get-Item -LiteralPath $distributionDir -Force
                if ($resolved -ne $expected -or ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
                    throw "Refusing to remove an unexpected distribution directory: $resolved"
                }
                Remove-Item -LiteralPath $resolved -Recurse -Force
            }
        }
    }
} finally {
    Pop-Location
}

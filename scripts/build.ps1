# VietFlow Release Build Script
# Builds the static ImGui/DX11 UI library, tests the Rust workspace, then links
# one or more VietFlow executables (x64 / x86 / ARM64) with embedded font and icon.
#
# Default CRT is static (/MT + Rust +crt-static), so the exe does not need the
# VC++ Redistributable. Use -DynamicCrt for the smaller /MD build.
#
# Version/changelog are single-sourced from VERSION and CHANGELOG.md.
param(
    [ValidateSet('x64', 'x86', 'arm64', 'All')]
    [string]$Arch = 'x64',
    [string]$DistDir = "",
    [switch]$Velopack,
    [switch]$Msi,
    [string]$VelopackVersion = "",
    [switch]$StaticCrt,
    [switch]$DynamicCrt,
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

if ([string]::IsNullOrWhiteSpace($DistDir)) {
    $DistDir = Join-Path $root "dist"
} elseif (-not [System.IO.Path]::IsPathRooted($DistDir)) {
    $DistDir = Join-Path $root $DistDir
}

New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
function Write-Utf8NoBom([string]$Path, [string]$Content) {
    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $encoding)
}

function Get-ChangelogSection([string]$Text, [string]$Version) {
    if ([string]::IsNullOrWhiteSpace($Text)) { return "" }
    $escaped = [regex]::Escape($Version)
    $pattern = "(?ms)^##\s+\[?v?$escaped\]?[^\r\n]*\r?\n(.*?)(?=^##\s+|\z)"
    $match = [regex]::Match($Text, $pattern)
    if ($match.Success) {
        return $match.Groups[1].Value.Trim()
    }
    return ""
}

# ---------------------------------------------------------------------------
# Version (VERSION is the only place to bump)
# ---------------------------------------------------------------------------
$versionPath = Join-Path $root "VERSION"
if (-not (Test-Path $versionPath)) {
    throw "Missing VERSION file at $versionPath"
}
$version = (Get-Content $versionPath -Raw -Encoding UTF8).Trim()
if ([string]::IsNullOrWhiteSpace($version)) {
    throw "VERSION file is empty"
}
if ($VelopackVersion -and $VelopackVersion.Trim() -ne $version) {
    throw "VelopackVersion '$VelopackVersion' does not match VERSION '$version'."
}

# Keep Cargo.toml's [workspace.package] version in sync. VERSION stays the only
# file a maintainer edits; build.rs still fails loudly if the two ever diverge.
$cargoToml = Join-Path $root "Cargo.toml"
if (Test-Path $cargoToml) {
    $cargoText = Get-Content $cargoToml -Raw -Encoding UTF8
    $versionPattern = '(?ms)(\[workspace\.package\][^\[]*?version\s*=\s*")[^"]+(")'
    $syncedCargo = [regex]::Replace($cargoText, $versionPattern, "`${1}$version`${2}")
    if ($syncedCargo -ne $cargoText) {
        Write-Utf8NoBom $cargoToml $syncedCargo
        Write-Host "Synced Cargo.toml to version $version" -ForegroundColor Yellow
    }
}

$archs = if ($Arch -eq 'All') { @('x64', 'x86', 'arm64') } else { @($Arch) }
$releaseBase = "https://github.com/KienPC1234/VietFlow/releases"
$releaseTagUrl = "$releaseBase/tag/v$version"
$releaseDownloadBase = "$releaseBase/download/v$version"

Write-Host "=================================================" -ForegroundColor Cyan
Write-Host "  VietFlow - Release Build v$version ($Arch)  " -ForegroundColor Cyan
Write-Host "=================================================" -ForegroundColor Cyan

# ---------------------------------------------------------------------------
# CRT selection
# ---------------------------------------------------------------------------
$previousRustFlags = $env:RUSTFLAGS
$useStatic = -not $DynamicCrt
if ($DynamicCrt -and $StaticCrt) {
    throw "Use either -DynamicCrt or -StaticCrt (static is the default)."
}
if ($useStatic) {
    $env:RUSTFLAGS = if ($previousRustFlags) {
        "$previousRustFlags -C target-feature=+crt-static"
    } else {
        "-C target-feature=+crt-static"
    }
    Write-Host "  CRT: static (/MT, no VC++ redist)" -ForegroundColor Yellow
} else {
    if ($previousRustFlags -match '\+crt-static') {
        $env:RUSTFLAGS = ($previousRustFlags -replace '\s*-C\s+target-feature=\+crt-static', '').Trim()
    }
    Write-Host "  CRT: dynamic (/MD, needs VC++ 2015-2022 redist)" -ForegroundColor DarkGray
}
$staticCrtValue = if ($useStatic) { "ON" } else { "OFF" }

# ---------------------------------------------------------------------------
# 1. Build the C++ ImGui/DX11 static library for each selected architecture
# ---------------------------------------------------------------------------
$imguiSourceDir = Join-Path $root "ui_imgui"
foreach ($a in $archs) {
    $buildDir = switch ($a) {
        'x86' { Join-Path $imguiSourceDir "build-x86" }
        'arm64' { Join-Path $imguiSourceDir "build-arm64" }
        default { Join-Path $imguiSourceDir "build" }
    }

    Write-Host "`n[1/5] Building C++ ImGui library ($a)..." -ForegroundColor Yellow
    $cmakeArgs = @(
        "-B", $buildDir,
        "-S", $imguiSourceDir,
        "-DCMAKE_BUILD_TYPE=Release",
        "-DVIETFLOW_STATIC_CRT=$staticCrtValue"
    )
    if ($a -eq 'x86') {
        $cmakeArgs += @("-A", "Win32")
    } elseif ($a -eq 'arm64') {
        $cmakeArgs += @("-A", "ARM64")
    }
    & cmake @cmakeArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "CMake configure failed for $a!"
        exit $LASTEXITCODE
    }
    & cmake --build $buildDir --config Release
    if ($LASTEXITCODE -ne 0) {
        Write-Error "C++ ImGui build failed for $a!"
        exit $LASTEXITCODE
    }
}

# ---------------------------------------------------------------------------
# 2. Tests (host x64 when x64 is selected, otherwise the x86 target)
# ---------------------------------------------------------------------------
Write-Host "`n[2/5] Running workspace unit tests..." -ForegroundColor Yellow
if ($SkipTests) {
    Write-Host "  Tests skipped (-SkipTests)." -ForegroundColor DarkGray
} elseif ($archs -contains 'x64') {
    & cargo test --manifest-path (Join-Path $root "Cargo.toml") --workspace
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Unit tests failed!"
        exit $LASTEXITCODE
    }
} elseif ($archs -contains 'x86') {
    & cargo test --manifest-path (Join-Path $root "Cargo.toml") --workspace --target i686-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Unit tests failed!"
        exit $LASTEXITCODE
    }
} else {
    Write-Host "  ARM64-only build: tests run in the x64 CI job instead." -ForegroundColor DarkGray
}

# ---------------------------------------------------------------------------
# 3. Build release executables
# ---------------------------------------------------------------------------
Write-Host "`n[3/5] Building release executable(s)..." -ForegroundColor Yellow
$built = @{}
$builtDist = @{}
$cargoManifest = Join-Path $root "Cargo.toml"

foreach ($a in $archs) {
    if ($a -eq 'x86') {
        Write-Host "  Compiling x86 (i686-pc-windows-msvc)..." -ForegroundColor DarkGray
        & cargo build --manifest-path $cargoManifest --release --target i686-pc-windows-msvc -p vietflow-windows
        if ($LASTEXITCODE -ne 0) {
            Write-Error "x86 release build failed!"
            exit $LASTEXITCODE
        }
        $src = Join-Path $root "target\i686-pc-windows-msvc\release\VietFlow.exe"
        $distName = "VietFlow-x86.exe"
    } elseif ($a -eq 'arm64') {
        Write-Host "  Compiling ARM64 (aarch64-pc-windows-msvc)..." -ForegroundColor DarkGray
        & cargo build --manifest-path $cargoManifest --release --target aarch64-pc-windows-msvc -p vietflow-windows
        if ($LASTEXITCODE -ne 0) {
            Write-Error "ARM64 release build failed!"
            exit $LASTEXITCODE
        }
        $src = Join-Path $root "target\aarch64-pc-windows-msvc\release\VietFlow.exe"
        $distName = "VietFlow-arm64.exe"
    } else {
        Write-Host "  Compiling x64 (host target)..." -ForegroundColor DarkGray
        & cargo build --manifest-path $cargoManifest --release -p vietflow-windows
        if ($LASTEXITCODE -ne 0) {
            Write-Error "x64 release build failed!"
            exit $LASTEXITCODE
        }
        & cargo build --manifest-path $cargoManifest --release -p vietflow-tsf
        if ($LASTEXITCODE -ne 0) {
            Write-Error "x64 TSF DLL release build failed!"
            exit $LASTEXITCODE
        }
        $src = Join-Path $root "target\release\VietFlow.exe"
        $distName = "VietFlow.exe"
    }
    if (-not (Test-Path $src)) {
        Write-Error "Expected executable not found: $src"
        exit 1
    }
    $built[$a] = $src
    $builtDist[$a] = Join-Path $DistDir $distName
}

Stop-Process -Name "VietFlow" -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 400

foreach ($a in $archs) {
    Copy-Item $built[$a] $builtDist[$a] -Force
    if ($a -eq 'x64') {
        Copy-Item $built[$a] (Join-Path $DistDir "VietFlow-x64.exe") -Force
        $tsfDll = Join-Path $root "target\release\vietflow_tip.dll"
        if (Test-Path $tsfDll) {
            $destTip = Join-Path $DistDir "vietflow_tip.dll"
            try {
                Copy-Item $tsfDll $destTip -Force -ErrorAction Stop
            } catch {
                # In-process COM DLL may be loaded in Windows CTF. Move locked file with unique name then copy.
                $tempOld = Join-Path $DistDir "vietflow_tip.dll.$([System.Guid]::NewGuid().ToString('N')).old"
                Move-Item -LiteralPath $destTip -Destination $tempOld -Force -ErrorAction SilentlyContinue
                Copy-Item $tsfDll $destTip -Force
            }
            # Best-effort purge of stale old DLLs
            Get-ChildItem -Path $DistDir -Filter "vietflow_tip.dll.*.old" -File -ErrorAction SilentlyContinue | ForEach-Object {
                Remove-Item $_.FullName -Force -ErrorAction SilentlyContinue
            }
            Write-Host "      TSF: $($tsfDll) -> $($destTip)" -ForegroundColor Green
        }
    }
}

$oldDll = Join-Path $DistDir "vietflow_ui.dll"
if (Test-Path $oldDll) {
    Remove-Item $oldDll -Force
}

# ---------------------------------------------------------------------------
# 4. Updater metadata (common for all architectures)
# ---------------------------------------------------------------------------
Write-Host "`n[4/5] Writing updater metadata (version.txt, latest.json, CHANGELOG.md)..." -ForegroundColor Yellow

$versionFile = Join-Path $DistDir "version.txt"
Write-Utf8NoBom $versionFile "$version`n"

$changelogSource = Join-Path $root "CHANGELOG.md"
$changelogFile = Join-Path $DistDir "CHANGELOG.md"
$changelogText = ""
if (Test-Path $changelogSource) {
    $changelogText = Get-Content $changelogSource -Raw -Encoding UTF8
    Copy-Item $changelogSource $changelogFile -Force
} else {
    Write-Warning "CHANGELOG.md not found at $changelogSource"
}

$vpk = if ($Velopack) { Get-Command "vpk" -ErrorAction SilentlyContinue } else { $null }
$willPackage = $Velopack -and $vpk
$downloads = [ordered]@{}
if ($willPackage) {
    if ($archs -contains 'x64') {
        $downloads['x64'] = "$releaseDownloadBase/VietFlow-win-x64-Setup.exe"
        $downloads['msi_x64'] = "$releaseDownloadBase/VietFlow-win-x64.msi"
    }
    if ($archs -contains 'x86') {
        $downloads['x86'] = "$releaseDownloadBase/VietFlow-win-x86-Setup.exe"
        $downloads['msi_x86'] = "$releaseDownloadBase/VietFlow-win-x86.msi"
    }
    if ($archs -contains 'arm64') {
        $downloads['arm64'] = "$releaseDownloadBase/VietFlow-win-arm64-Setup.exe"
        $downloads['msi_arm64'] = "$releaseDownloadBase/VietFlow-win-arm64.msi"
    }
}

$notes = Get-ChangelogSection $changelogText $version
$manifest = [ordered]@{
    version   = $version
    notes     = $notes
    url       = $releaseTagUrl
    released  = (Get-Date).ToString("yyyy-MM-dd")
    downloads = $downloads
}
$manifestFile = Join-Path $DistDir "latest.json"
Write-Utf8NoBom $manifestFile (($manifest | ConvertTo-Json -Depth 4) + "`n")

foreach ($a in $archs) {
    $sizeMb = [math]::Round(((Get-Item $builtDist[$a]).Length / 1MB), 3)
    Write-Host "      ${a}: $($builtDist[$a]) ($sizeMb MB)" -ForegroundColor Green
}
Write-Host "      Version:  $versionFile" -ForegroundColor Green
Write-Host "      Manifest: $manifestFile" -ForegroundColor Green

# ---------------------------------------------------------------------------
# 5. Optional Velopack packaging, one channel per architecture
# ---------------------------------------------------------------------------
if ($willPackage) {
    $releasesDir = Join-Path $root "releases"
    New-Item -ItemType Directory -Force -Path $releasesDir | Out-Null

    foreach ($a in $archs) {
        $channel = switch ($a) {
            'x86' { "win-x86" }
            'arm64' { "win-arm64" }
            default { "win-x64" }
        }
        $runtime = $channel
        $packDir = Join-Path $root "target\release\pack_staging-$a"
        Remove-Item -Recurse -Force $packDir -ErrorAction SilentlyContinue
        New-Item -ItemType Directory -Force -Path $packDir | Out-Null
        Copy-Item -Path $built[$a] -Destination (Join-Path $packDir "VietFlow.exe") -Force
        $tsfDll = Join-Path $DistDir "vietflow_tip.dll"
        if (Test-Path $tsfDll) {
            Copy-Item -Path $tsfDll -Destination $packDir -Force
        }
        Copy-Item -Path $versionFile, $manifestFile -Destination $packDir -Force
        if (Test-Path $changelogFile) {
            Copy-Item -Path $changelogFile -Destination $packDir -Force
        }

        Write-Host "`n[5/5] Packaging $a with Velopack v$version (channel $channel)..." -ForegroundColor Magenta
        $packArgs = @(
            "pack",
            "--packId", "VietFlow",
            "--packVersion", $version,
            "--channel", $channel,
            "--runtime", $runtime,
            "--packDir", $packDir,
            "--packTitle", "VietFlow",
            "--packAuthors", "VietFlow Team",
            "--mainExe", "VietFlow.exe",
            "--icon", (Join-Path $root "assets\VietFlow.ico"),
            "--outputDir", $releasesDir
        )
        if ($Msi) {
            $packArgs += "--msi"
        }
        & $vpk.Source @packArgs
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Velopack packaging failed for $a!"
            exit $LASTEXITCODE
        }
    }

    Copy-Item -Path $versionFile, $manifestFile -Destination $releasesDir -Force
    if (Test-Path $changelogFile) {
        Copy-Item -Path $changelogFile -Destination $releasesDir -Force
    }

    Write-Host "Velopack releases ready in: $releasesDir" -ForegroundColor Green
    Write-Host "Upload these assets to the GitHub release:" -ForegroundColor Green
    Write-Host "  VietFlow-x64.exe, VietFlow-x86.exe, VietFlow-arm64.exe" -ForegroundColor Green
    Write-Host "  VietFlow-win-x64-Setup.exe, VietFlow-win-x86-Setup.exe, VietFlow-win-arm64-Setup.exe" -ForegroundColor Green
    Write-Host "  latest.json, version.txt, CHANGELOG.md" -ForegroundColor Green
} elseif ($Velopack) {
    Write-Warning "'vpk' tool was not found in PATH. Skipping Velopack packaging."
} else {
    Write-Host "`n[5/5] Velopack packaging skipped (use -Velopack to build installers)." -ForegroundColor DarkGray
}

if ($env:RUSTFLAGS -ne $previousRustFlags) {
    if ($null -eq $previousRustFlags) {
        Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
    } else {
        $env:RUSTFLAGS = $previousRustFlags
    }
}

Write-Host "`n[OK] VietFlow build finished successfully!" -ForegroundColor Green

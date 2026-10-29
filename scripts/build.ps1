# VietFlow Release Build & Package Script
# Produces a single portable VietFlow.exe with the Qt UI runtime embedded.
param(
    [string]$DistDir = "",
    [switch]$Separate,
    [switch]$Velopack,
    [switch]$Msi,
    [string]$VelopackVersion = ""
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($DistDir)) {
    $DistDir = Join-Path $root "dist"
} elseif (-not [System.IO.Path]::IsPathRooted($DistDir)) {
    $DistDir = Join-Path $root $DistDir
}
$dist = $DistDir
$payload = Join-Path $root "ui-runtime.zip"

New-Item -ItemType Directory -Force -Path $dist | Out-Null

function Remove-Path([string]$Path) {
    if (Test-Path $Path) {
        Remove-Item $Path -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# A running daemon locks its own exe; keep packaging the rest and warn instead
# of failing the whole build.
function Copy-Artifact([string]$Source) {
    $target = Join-Path $dist (Split-Path $Source -Leaf)
    if ((Test-Path $target) -and ($Source -notlike "*VietFlow.exe")) {
        Remove-Path $target
    }
    try {
        Copy-Item $Source $target -Force
    } catch {
        $name = [System.IO.Path]::GetFileNameWithoutExtension($Source)
        $ext = [System.IO.Path]::GetExtension($Source)
        $fallback = Join-Path $dist "$name.new$ext"
        Copy-Item $Source $fallback -Force
        Write-Warning "Could not overwrite '$target' (app is running). Saved '$fallback' instead."
    }
}

if ($Separate) {
    Remove-Path $payload
}

Write-Host "Building Rust core..." -ForegroundColor Cyan
cargo build --manifest-path (Join-Path $root "Cargo.toml") --release -p vietflow-core

$qt = $env:QT_ROOT
if (-not $qt -or -not (Test-Path $qt)) {
    $qt = "C:\Qt\6.8.3\msvc2022_64"
}

$uiExe = $null
$windeployqt = Join-Path $qt "bin\windeployqt.exe"
if (Test-Path $windeployqt) {
    Write-Host "Building VietFlow Qt UI..." -ForegroundColor Cyan
    $uiSource = Join-Path $root "ui"
    $uiBuild = Join-Path $uiSource "build"

    cmake -S $uiSource -B $uiBuild -DCMAKE_PREFIX_PATH="$qt"
    cmake --build $uiBuild --config Release
    $uiExe = Join-Path $uiBuild "Release\vietflow-ui.exe"

    if ($Separate) {
        & $windeployqt --qmldir (Join-Path $uiSource "qml") --dir $dist --release `
            --no-translations --no-opengl-sw $uiExe
        Copy-Artifact $uiExe
        Copy-Artifact (Join-Path $root "target\release\vietflow_core.dll")
    } else {
        $stage = Join-Path $uiSource "runtime"
        Remove-Path $stage
        New-Item -ItemType Directory -Force -Path $stage | Out-Null

        & $windeployqt --qmldir (Join-Path $uiSource "qml") --dir $stage --release `
            --no-translations --no-opengl-sw `
            --skip-plugin-types "qmltooling,generic,networkinformation,tls,styles,iconengines" `
            --exclude-plugins "qgif,qjpeg,qtiff,qwebp,qico,qminimal,qoffscreen,qdirect2d" `
            --no-quickcontrols2material --no-quickcontrols2materialstyleimpl `
            --no-quickcontrols2universal --no-quickcontrols2universalstyleimpl `
            --no-quickcontrols2fusion --no-quickcontrols2fusionstyleimpl `
            --no-quickcontrols2imagine --no-quickcontrols2imaginestyleimpl `
            --no-quickcontrols2windowsstyleimpl --no-quickcontrols2fluentwinui3styleimpl `
            $uiExe

        Copy-Item $uiExe $stage -Force
        Copy-Item (Join-Path $root "target\release\vietflow_core.dll") $stage -Force

        # Optional libraries not used by this UI; removing them keeps the payload small.
        # Keep Qt6Network (imported by Qt6Quick) and Qt6QuickShapes (imported by the
        # Basic style); Qt6QuickEffects is not used.
        Remove-Path (Join-Path $stage "Qt6QuickEffects.dll")
        foreach ($style in @("FluentWinUI3", "Fusion", "Imagine", "Material", "Universal", "Windows")) {
            Remove-Path (Join-Path $stage "qml\QtQuick\Controls\$style")
        }
        Remove-Path (Join-Path $stage "styles")
        Remove-Path (Join-Path $stage "iconengines")
        Get-ChildItem (Join-Path $stage "imageformats") -File -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -ne "qsvg.dll" } |
            Remove-Item -Force -ErrorAction SilentlyContinue
        Get-ChildItem (Join-Path $stage "platforms") -File -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -ne "qwindows.dll" } |
            Remove-Item -Force -ErrorAction SilentlyContinue

        # MSVC runtime DLLs: required when the target machine lacks the VC++ redist.
        $crt = Get-ChildItem "C:\Program Files\Microsoft Visual Studio" -Recurse -Filter "msvcp140.dll" -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\x64\\Microsoft\.VC14\d\.CRT\\' } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($crt) {
            Copy-Item (Join-Path $crt.DirectoryName "*.dll") $stage -Force -ErrorAction SilentlyContinue
        }

        Remove-Path (Join-Path $stage "vc_redist.x64.exe")
        Remove-Path (Join-Path $stage "qt.conf")

        Remove-Path $payload
        Write-Host "Packing Qt runtime..." -ForegroundColor Cyan
        Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $payload -CompressionLevel Optimal
        Write-Host ("Payload: {0:N1} MB" -f ((Get-Item $payload).Length / 1MB)) -ForegroundColor DarkGray
    }
} else {
    Write-Warning "Qt 6 not found at '$qt'. A UI-less host will be built (set QT_ROOT to override)."
    Remove-Path $payload
}

Write-Host "Building VietFlow host..." -ForegroundColor Cyan
# Force a rebuild so the optional payload state (present/absent) is respected.
cargo clean --manifest-path (Join-Path $root "Cargo.toml") -p vietflow-windows --release 2>$null | Out-Null
cargo build --manifest-path (Join-Path $root "Cargo.toml") --release -p vietflow-windows

if (-not $Separate) {
    # Remove files from the previous separate-runtime layout.
    foreach ($name in @("vietflow-ui.exe", "vietflow_core.dll", "opengl32sw.dll", "D3Dcompiler_47.dll", "vc_redist.x64.exe", "qt.conf")) {
        Remove-Path (Join-Path $dist $name)
    }
    foreach ($dir in @("qml", "platforms", "imageformats", "iconengines", "styles", "translations", "tls", "networkinformation", "generic", "qmltooling")) {
        Remove-Path (Join-Path $dist $dir)
    }
    Get-ChildItem $dist -Filter "Qt6*.dll" -File -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
}

Remove-Path (Join-Path $dist "VietFlow.new.exe")
Copy-Artifact (Join-Path $root "target\release\VietFlow.exe")

Write-Host "`nBuild succeeded! Artifacts available in:" -ForegroundColor Green
Get-ChildItem $dist | Select-Object Name, @{N = "MB"; E = { [math]::Round($_.Length / 1MB, 1) } } | Format-Table -AutoSize

# The single-file exe already contains the payload; keep the tree clean.
if (-not $Separate) {
    Remove-Path (Join-Path $root "ui\runtime")
    Remove-Path $payload
}

if ($Velopack) {
    Write-Host "`nPackaging with Velopack..." -ForegroundColor Cyan
    $vpk = Get-Command "vpk" -ErrorAction SilentlyContinue
    if (-not $vpk) {
        Write-Error "Velopack CLI 'vpk' not found. Please install via 'dotnet tool install -g vpk'."
        exit 1
    }

    if ([string]::IsNullOrWhiteSpace($VelopackVersion)) {
        $cargoContent = Get-Content (Join-Path $root "platforms\windows\Cargo.toml") -Raw
        if ($cargoContent -match 'version\s*=\s*"([^"]+)"') {
            $VelopackVersion = $matches[1]
        } else {
            $VelopackVersion = "0.1.0"
        }
    }

    $releasesDir = Join-Path $root "releases"
    New-Item -ItemType Directory -Force -Path $releasesDir | Out-Null

    $iconPath = Join-Path $root "assets\VietFlow.ico"
    $vpkArgs = @(
        "pack",
        "--packId", "VietFlow",
        "--packVersion", $VelopackVersion,
        "--packDir", $dist,
        "--outputDir", $releasesDir,
        "--mainExe", "VietFlow.exe",
        "--packTitle", "VietFlow",
        "--packAuthors", "VietFlow Team"
    )
    if (Test-Path $iconPath) {
        $vpkArgs += @("--icon", $iconPath)
    }
    if ($Msi) {
        $vpkArgs += @("--msi")
    }

    & vpk @vpkArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Velopack pack failed with code $LASTEXITCODE."
        exit $LASTEXITCODE
    }

    Write-Host "`nVelopack packaging completed! Release packages in:" -ForegroundColor Green
    Get-ChildItem $releasesDir | Select-Object Name, @{N = "MB"; E = { [math]::Round($_.Length / 1MB, 1) } } | Format-Table -AutoSize
}

# VietFlow Native WiX v5 MSI Builder
param(
    [ValidateSet('x64', 'x86', 'arm64', 'All')]
    [string]$Arch = 'x64',
    [string]$Version = "",
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$wixFile = Join-Path $scriptDir "VietFlow.wxs"
$vietflowRoot = Split-Path -Parent (Split-Path -Parent $scriptDir)

# Ensure output directory exists
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $vietflowRoot "releases"
} elseif (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path $vietflowRoot $OutputDir
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# Resolve version
if ([string]::IsNullOrWhiteSpace($Version)) {
    $versionFile = Join-Path $vietflowRoot "VERSION"
    if (Test-Path $versionFile) {
        $Version = (Get-Content $versionFile -Raw).Trim()
    } else {
        $cargoToml = Join-Path $vietflowRoot "Cargo.toml"
        $content = Get-Content $cargoToml -Raw
        if ($content -match '(?m)^version\s*=\s*"([^"]+)"') {
            $Version = $matches[1]
        } else {
            $Version = "0.1.0"
        }
    }
}

# Verify wix tool
$wixCmd = Get-Command "wix" -ErrorAction SilentlyContinue
if (-not $wixCmd) {
    Write-Host "Installing WiX v5 CLI..." -ForegroundColor Yellow
    dotnet tool install --global wix --version 5.0.2
    $wixCmd = Get-Command "wix" -ErrorAction SilentlyContinue
}

# Ensure WiX extensions
$extList = & $wixCmd.Source extension list -g 2>&1 | Out-String
if ($extList -notmatch "WixToolset.UI.wixext\s+5\.0\.2") {
    Write-Host "Adding WixToolset.UI.wixext 5.0.2..." -ForegroundColor Cyan
    & $wixCmd.Source extension add -g WixToolset.UI.wixext/5.0.2
}
if ($extList -notmatch "WixToolset.Util.wixext\s+5\.0\.2") {
    Write-Host "Adding WixToolset.Util.wixext 5.0.2..." -ForegroundColor Cyan
    & $wixCmd.Source extension add -g WixToolset.Util.wixext/5.0.2
}

$archList = if ($Arch -eq 'All') { @('x64', 'x86', 'arm64') } else { @($Arch) }

foreach ($a in $archList) {
    $channel = switch ($a) {
        'x86' { "win-x86" }
        'arm64' { "win-arm64" }
        default { "win-x64" }
    }

    $binCandidates = @(
        (Join-Path $vietflowRoot "dist\VietFlow-$a.exe"),
        (Join-Path $vietflowRoot "dist\VietFlow.exe"),
        (Join-Path $vietflowRoot "target\release\VietFlow.exe")
    )
    $binPath = $binCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1

    if (-not $binPath) {
        Write-Error "Could not find binary for architecture $a in dist or target\release."
        exit 1
    }

    $tsfCandidates = @(
        (Join-Path $vietflowRoot "platforms\windows_tsf\build-$a\Release\vietflow_tsf.dll"),
        (Join-Path $vietflowRoot "dist\vietflow_tsf_$a.dll"),
        (Join-Path $vietflowRoot "dist\vietflow_tsf.dll")
    )
    $tsfDll = $tsfCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $tsfDll) {
        $tsfDll = Join-Path $vietflowRoot "dist\vietflow_tsf.dll"
    }

    $outputMsi = Join-Path $OutputDir "VietFlow-$channel.msi"
    Write-Host "`nBuilding native WiX MSI: $outputMsi ($a, v$Version)..." -ForegroundColor Cyan

    $buildArgs = @(
        "build",
        $wixFile,
        "-arch", $a,
        "-ext", "WixToolset.UI.wixext/5.0.2",
        "-ext", "WixToolset.Util.wixext/5.0.2",
        "-d", "Version=$Version",
        "-d", "SourceDir=$vietflowRoot",
        "-d", "BinaryPath=$binPath",
        "-d", "TsfDllPath=$tsfDll",
        "-o", $outputMsi
    )

    & $wixCmd.Source @buildArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "WiX build failed for architecture $a."
        exit $LASTEXITCODE
    }

    $injectScript = Join-Path $vietflowRoot "scripts\inject_msi_bitmaps.ps1"
    if (Test-Path $injectScript) {
        Write-Host "Optimizing MSI database with ActionText & dialog bitmaps..." -ForegroundColor Cyan
        & pwsh -NoProfile -NonInteractive -File $injectScript -MsiPath $outputMsi
    }

    $sizeMb = [math]::Round(((Get-Item $outputMsi).Length / 1MB), 2)
    Write-Host "Successfully generated: $outputMsi ($sizeMb MB)" -ForegroundColor Green
}


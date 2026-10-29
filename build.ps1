# VietFlow Release Build & Package Script
$ErrorActionPreference = "Stop"

Write-Host "Building VietFlow Release..." -ForegroundColor Cyan
$manifest = Join-Path $PSScriptRoot "Cargo.toml"
cargo build --manifest-path $manifest --release

$distDir = Join-Path $PSScriptRoot "dist"
if (!(Test-Path $distDir)) {
    New-Item -ItemType Directory -Path $distDir | Out-Null
}

$releaseExe = Join-Path $PSScriptRoot "target\release\VietFlow.exe"
$releaseDll = Join-Path $PSScriptRoot "target\release\vietflow_core.dll"

Copy-Item $releaseExe -Destination (Join-Path $distDir "VietFlow.exe") -Force
Copy-Item $releaseDll -Destination (Join-Path $distDir "vietflow_core.dll") -Force

Write-Host "`nBuild succeeded! Artifacts available in:" -ForegroundColor Green
Write-Host "  $(Join-Path $distDir 'VietFlow.exe')" -ForegroundColor Yellow
Write-Host "  $(Join-Path $distDir 'vietflow_core.dll')" -ForegroundColor Yellow

# VietFlow - Release & Packaging Script (Setup.exe + MSI)
param (
    [string]$Version = "1.0.4"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host "==> [1/3] Publishing VietFlow.Windows (net10.0-windows, win-x64)..." -ForegroundColor Cyan
$PublishDir = Join-Path $ScriptDir "..\dist\publish"
if (Test-Path $PublishDir) { Remove-Item -Recurse -Force $PublishDir }

dotnet publish (Join-Path $ScriptDir "src\VietFlow.Windows\VietFlow.Windows.csproj") `
    -c Release `
    -r win-x64 `
    --self-contained `
    -o $PublishDir

Write-Host "==> [2/3] Checking Velopack CLI (vpk)..." -ForegroundColor Cyan
if (-not (Get-Command vpk -ErrorAction SilentlyContinue)) {
    Write-Host "Installing vpk tool..." -ForegroundColor Yellow
    dotnet tool install -g vpk
}

Write-Host "==> [3/3] Packaging Release with Velopack (Setup.exe + MSI + Portable)..." -ForegroundColor Cyan
$ReleasesDir = Join-Path $ScriptDir "..\dist\releases"
$IconPath = Join-Path $ScriptDir "src\VietFlow.Windows\Assets\VietFlow.ico"

vpk pack `
    -u VietFlow `
    -v $Version `
    -p $PublishDir `
    -e VietFlow.exe `
    -r win-x64 `
    --msi `
    --packTitle "VietFlow" `
    --packAuthors "KienPC1234" `
    --icon $IconPath `
    -o $ReleasesDir

Write-Host "`nRelease build completed successfully! Output directory: $ReleasesDir" -ForegroundColor Green
Get-ChildItem -Path $ReleasesDir | Format-Table Name, Length, LastWriteTime


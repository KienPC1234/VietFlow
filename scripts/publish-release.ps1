# VietFlow Release Publisher
# Synchronizes release metadata with GitHub Releases and uploads all built artifacts.
# Supports GitHub Token from GITHUB_TOKEN environment variable, GitHub CLI (gh), or Git credential helper.

param(
    [string]$Tag = "",
    [switch]$Draft = $false,
    [switch]$Prerelease = $false
)

$ErrorActionPreference = "Stop"
$env:GIT_TERMINAL_PROMPT = '0'
$env:GCM_INTERACTIVE = 'never'

$repoRoot = Split-Path -Parent $PSScriptRoot
$distDir = Join-Path $repoRoot "dist"
$releasesDir = Join-Path $repoRoot "releases"

# 1. Resolve GitHub Token
$token = $env:GITHUB_TOKEN
if ([string]::IsNullOrWhiteSpace($token)) {
    $ghCmd = Get-Command "gh" -ErrorAction SilentlyContinue
    if ($ghCmd) {
        $token = & $ghCmd.Source auth token 2>$null
    }
}
if ([string]::IsNullOrWhiteSpace($token)) {
    try {
        $cred = @('protocol=https', 'host=github.com', '') | git credential fill 2>$null
        $tokenLine = $cred | Select-String '^password=' | Select-Object -First 1
        if ($tokenLine) {
            $token = $tokenLine.Line.Substring(9).Trim()
        }
    } catch {}
}
if ([string]::IsNullOrWhiteSpace($token)) {
    Write-Error "No GitHub credential available. Set GITHUB_TOKEN, authenticate with 'gh auth login', or configure git credentials."
    exit 1
}

$headers = @{
    Authorization = "Bearer $token"
    'User-Agent'  = 'VietFlow-Publisher/1.0'
    Accept        = 'application/vnd.github+json'
}

# 2. Read Release Metadata
$manifestPath = Join-Path $distDir "latest.json"
if (-not (Test-Path $manifestPath)) {
    Write-Error "Missing manifest at $manifestPath. Please run scripts/build.ps1 first."
    exit 1
}

$manifest = Get-Content $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$version = $manifest.version
if ([string]::IsNullOrWhiteSpace($Tag)) {
    $Tag = "v$version"
}

$releaseNotes = if ($manifest.notes) { $manifest.notes } else { "VietFlow Release $version" }
$releaseTitle = "VietFlow $version"

Write-Host "=================================================" -ForegroundColor Cyan
Write-Host "  VietFlow Release Publisher: $Tag               " -ForegroundColor Cyan
Write-Host "=================================================" -ForegroundColor Cyan

# 3. Create or Update GitHub Release
$repoApi = "https://api.github.com/repos/KienPC1234/VietFlow/releases"
$release = $null

try {
    $release = Invoke-RestMethod -Method Get -Uri "$repoApi/tags/$Tag" -Headers $headers
    Write-Host "Found existing release $Tag (id: $($release.id)). Synchronizing metadata..." -ForegroundColor Yellow

    $updateBody = [ordered]@{
        name       = $releaseTitle
        body       = $releaseNotes
        draft      = [bool]$Draft
        prerelease = [bool]$Prerelease
    } | ConvertTo-Json -Depth 4

    $release = Invoke-RestMethod -Method Patch `
        -Uri "$repoApi/$($release.id)" `
        -Headers $headers `
        -Body ([System.Text.Encoding]::UTF8.GetBytes($updateBody)) `
        -ContentType 'application/json; charset=utf-8'
} catch {
    Write-Host "Creating new release $Tag on GitHub..." -ForegroundColor Cyan

    $createBody = [ordered]@{
        tag_name         = $Tag
        target_commitish = 'main'
        name             = $releaseTitle
        body             = $releaseNotes
        draft            = [bool]$Draft
        prerelease       = [bool]$Prerelease
    } | ConvertTo-Json -Depth 4

    $release = Invoke-RestMethod -Method Post `
        -Uri $repoApi `
        -Headers $headers `
        -Body ([System.Text.Encoding]::UTF8.GetBytes($createBody)) `
        -ContentType 'application/json; charset=utf-8'
}

Write-Host "Release URL: $($release.html_url)" -ForegroundColor Green

# 4. Discover Release Assets
$candidates = @(
    (Join-Path $distDir "VietFlow-x64.exe"),
    (Join-Path $distDir "VietFlow-x86.exe"),
    (Join-Path $distDir "VietFlow-arm64.exe"),
    (Join-Path $distDir "VietFlow.exe"),
    (Join-Path $distDir "vietflow_tip.dll"),
    (Join-Path $distDir "latest.json"),
    (Join-Path $distDir "version.txt"),
    (Join-Path $distDir "CHANGELOG.md"),
    (Join-Path $releasesDir "VietFlow-win-x64.msi"),
    (Join-Path $releasesDir "VietFlow-win-x86.msi"),
    (Join-Path $releasesDir "VietFlow-win-arm64.msi"),
    (Join-Path $releasesDir "VietFlow-win-x64-Portable.zip"),
    (Join-Path $releasesDir "VietFlow-win-x86-Portable.zip"),
    (Join-Path $releasesDir "VietFlow-win-arm64-Portable.zip")
)

$assetsToUpload = @()
foreach ($c in $candidates) {
    if (Test-Path $c) {
        $assetsToUpload += (Resolve-Path $c).Path
    }
}

if ($assetsToUpload.Count -eq 0) {
    Write-Error "No release assets found to upload in dist/ or releases/."
    exit 1
}

# 5. Fetch Existing Assets for Deduplication
$currentRelease = Invoke-RestMethod -Method Get -Uri "$repoApi/$($release.id)" -Headers $headers
$existingAssets = @($currentRelease.assets)

Write-Host "`nUploading $($assetsToUpload.Count) assets..." -ForegroundColor Cyan
Write-Host "-------------------------------------------------" -ForegroundColor Gray

foreach ($assetPath in $assetsToUpload) {
    $fileName = Split-Path $assetPath -Leaf
    $fileInfo = Get-Item $assetPath
    $sizeMb = [math]::Round(($fileInfo.Length / 1MB), 2)

    # Delete existing asset with identical name to avoid conflicts
    $existing = $existingAssets | Where-Object { $_.name -eq $fileName } | Select-Object -First 1
    if ($existing) {
        Write-Host "  Replacing outdated asset: $fileName..." -ForegroundColor Yellow
        Invoke-RestMethod -Method Delete -Uri $existing.url -Headers $headers | Out-Null
    }

    $uploadUri = "https://uploads.github.com/repos/KienPC1234/VietFlow/releases/$($release.id)/assets?name=$([uri]::EscapeDataString($fileName))"
    $uploaded = Invoke-RestMethod -Method Post `
        -Uri $uploadUri `
        -Headers $headers `
        -InFile $assetPath `
        -ContentType 'application/octet-stream'

    Write-Host "  Uploaded: $fileName ($sizeMb MB)" -ForegroundColor Green
}

Write-Host "`n[OK] Release $Tag published successfully!" -ForegroundColor Green

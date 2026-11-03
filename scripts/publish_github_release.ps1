param(
    [string]$Tag = "",
    [string]$ReleasesDir = ""
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($ReleasesDir)) {
    $ReleasesDir = Join-Path $root "releases"
}

if ([string]::IsNullOrWhiteSpace($Tag)) {
    $versionFile = Join-Path $root "VERSION"
    if (Test-Path $versionFile) {
        $Tag = "v" + (Get-Content $versionFile -Raw).Trim()
    } else {
        $Tag = "v0.1.4"
    }
}

# 1. Get token from Git Credential Manager
$inputStr = @"
protocol=https
host=github.com

"@
$cred = $inputStr | git credential fill
$token = ($cred | Where-Object { $_ -match '^password=' }) -replace '^password=', ''
if ([string]::IsNullOrWhiteSpace($token)) {
    Write-Error "Could not retrieve GitHub token from Git Credential Manager."
    exit 1
}

$headers = @{
    "Authorization" = "token $token"
    "User-Agent"    = "VietFlow-Release-Tool"
    "Accept"        = "application/vnd.github+json"
}

# 2. Extract release notes from CHANGELOG.md
$version = $Tag.TrimStart('v', 'V')
$changelogPath = Join-Path $root "CHANGELOG.md"
$changelogText = Get-Content $changelogPath -Raw -Encoding UTF8
$escaped = [regex]::Escape($version)
$pattern = "(?ms)^##\s+\[?v?$escaped\]?[^\r\n]*\r?\n(.*?)(?=^##\s+|\z)"
$match = [regex]::Match($changelogText, $pattern)
$notes = if ($match.Success) { $match.Groups[1].Value.Trim() } else { "VietFlow release $Tag" }

# 3. Check or create Release
$repo = "KienPC1234/VietFlow"
$releaseApiUrl = "https://api.github.com/repos/$repo/releases"
Write-Host "Checking release $Tag on $repo..." -ForegroundColor Cyan

$release = $null
try {
    $release = Invoke-RestMethod -Uri "$releaseApiUrl/tags/$Tag" -Headers $headers -Method Get
    Write-Host "Found release: ID $($release.id)" -ForegroundColor Yellow
} catch {
    Write-Host "Release $Tag does not exist. Creating..." -ForegroundColor Cyan
    $createBody = @{
        tag_name         = $Tag
        target_commitish = "main"
        name             = "VietFlow $version"
        body             = $notes
        draft            = $false
        prerelease       = $false
    } | ConvertTo-Json -Depth 4

    $release = Invoke-RestMethod -Uri $releaseApiUrl -Headers $headers -Method Post -Body ([System.Text.Encoding]::UTF8.GetBytes($createBody)) -ContentType "application/json; charset=utf-8"
    Write-Host "Created release: ID $($release.id)" -ForegroundColor Green
}

$releaseId = $release.id

# 4. List existing assets
$relDetail = Invoke-RestMethod -Uri "$releaseApiUrl/$releaseId" -Headers $headers -Method Get
$existingAssets = @{}
if ($relDetail.assets) {
    foreach ($asset in $relDetail.assets) {
        $existingAssets[$asset.name] = $asset.id
    }
}

# 5. Assets to upload
$assetsToUpload = @(
    "VietFlow-win-x64.msi",
    "VietFlow-win-x86.msi",
    "VietFlow-win-x64-Portable.zip",
    "VietFlow-win-x86-Portable.zip",
    "VietFlow-x64.exe",
    "VietFlow-x86.exe",
    "VietFlow.exe",
    "vietflow_tsf.dll",
    "latest.json",
    "version.txt",
    "CHANGELOG.md"
)

foreach ($assetName in $assetsToUpload) {
    $filePath = Join-Path $ReleasesDir $assetName
    if (-not (Test-Path $filePath)) {
        Write-Warning "Asset not found: $filePath (skipped)"
        continue
    }

    # Delete existing asset if present
    if ($existingAssets.ContainsKey($assetName)) {
        $oldId = $existingAssets[$assetName]
        Write-Host "Deleting existing asset $assetName (ID $oldId)..." -ForegroundColor DarkGray
        try {
            Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/assets/$oldId" -Headers $headers -Method Delete | Out-Null
        } catch {
            Write-Warning "Could not delete existing asset: $_"
        }
    }

    $contentType = switch -Regex ($assetName) {
        '\.msi$'  { "application/octet-stream" }
        '\.exe$'  { "application/octet-stream" }
        '\.dll$'  { "application/octet-stream" }
        '\.zip$'  { "application/zip" }
        '\.json$' { "application/json" }
        '\.txt$'  { "text/plain" }
        '\.md$'   { "text/markdown" }
        default   { "application/octet-stream" }
    }

    $uploadUrl = "https://uploads.github.com/repos/$repo/releases/$releaseId/assets?name=$assetName"
    $sizeKb = [math]::Round(((Get-Item $filePath).Length / 1KB), 1)
    Write-Host "Uploading $assetName ($sizeKb KB)..." -ForegroundColor Cyan

    $resolvedPath = (Resolve-Path $filePath).Path
    $curlArgs = @(
        "-s",
        "-X", "POST",
        "-H", "Authorization: token $token",
        "-H", "Content-Type: $contentType",
        "--data-binary", "@$resolvedPath",
        $uploadUrl
    )
    $resp = & curl.exe @curlArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to upload $assetName (curl exit code $LASTEXITCODE)"
        exit 1
    }
    Write-Host "Uploaded $assetName successfully." -ForegroundColor Green
}

Write-Host "`nAll release assets successfully published for $Tag!" -ForegroundColor Green
Write-Host "Release URL: https://github.com/$repo/releases/tag/$Tag" -ForegroundColor Green

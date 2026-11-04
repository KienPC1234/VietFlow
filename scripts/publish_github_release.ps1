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
        throw "VERSION is required; cannot guess release tag"
    }
}

# 0. Validate all release assets BEFORE accessing credentials or remote APIs.
$version = (Get-Content (Join-Path $root 'VERSION') -Raw).Trim()
if ($Tag -ne "v$version" -or $Tag -notmatch '^v[0-9]+[.][0-9]+[.][0-9]+$') {
    throw "Tag must match VERSION exactly: v$version"
}
$manifestPath = Join-Path $ReleasesDir 'release-artifacts.json'
if (-not (Test-Path $manifestPath)) { throw "Missing full release provenance manifest" }
$manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
$commit = (& git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $manifest.version -ne $version -or
    $manifest.git_commit -ne $commit -or
    (@($manifest.architectures) -join ',') -ne 'x64,x86,arm64') {
    throw "Release provenance version/commit/architecture mismatch"
}
$assetsToUpload = @(
    'VietFlow-win-x64.msi','VietFlow-win-x86.msi','VietFlow-win-arm64.msi',
    'VietFlow-win-x64-Portable.zip','VietFlow-win-x86-Portable.zip',
    'VietFlow-win-arm64-Portable.zip',
    'VietFlow-x64.exe','VietFlow-x86.exe','VietFlow-arm64.exe','VietFlow.exe',
    'vietflow_tsf.dll','vietflow_tsf_x86.dll','vietflow_tsf_arm64.dll',
    'latest.json','version.txt','CHANGELOG.md','release-artifacts.json','SHA256SUMS.txt'
)
foreach ($name in $assetsToUpload) {
    $file = Join-Path $ReleasesDir $name
    if (-not (Test-Path -LiteralPath $file -PathType Leaf) -or
        (Get-Item -LiteralPath $file).Length -eq 0) {
        throw "Missing required release asset: $name"
    }
}
foreach ($entry in $manifest.assets) {
    $file = Join-Path $ReleasesDir $entry.file
    if (-not (Test-Path -LiteralPath $file) -or
        (Get-Item -LiteralPath $file).Length -ne $entry.bytes -or
        (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256) {
        throw "Release artifact digest does not match provenance: $($entry.file)"
    }
}
Write-Host "Release preflight passed: $version, commit $commit, $($assetsToUpload.Count) files."
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
        draft            = $true
        prerelease       = $false
    } | ConvertTo-Json -Depth 4

    $release = Invoke-RestMethod -Uri $releaseApiUrl -Headers $headers -Method Post -Body ([System.Text.Encoding]::UTF8.GetBytes($createBody)) -ContentType "application/json; charset=utf-8"
    Write-Host "Created release: ID $($release.id)" -ForegroundColor Green
}

# Never destructively replace assets on an already-public release.
# A new tag or explicit maintainer action is required for published versions.
if (-not $release.draft) {
    throw "Existing release is already public; refusing to replace its assets"
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

# 5. Upload only previously verified assets.
foreach ($assetName in $assetsToUpload) {
    $filePath = Join-Path $ReleasesDir $assetName
    if (-not (Test-Path $filePath)) {
        throw "Required asset missing: $filePath"
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

    # Avoid exposing bearer token through another process command line.
    $resolvedPath = (Resolve-Path $filePath).Path
    $uploaded = Invoke-RestMethod -Method Post -Uri $uploadUrl -Headers $headers -InFile $resolvedPath -ContentType $contentType
    if (-not $uploaded -or $uploaded.name -ne $assetName) {
        throw "GitHub did not acknowledge asset: $assetName"
    }
    Write-Host "Uploaded $assetName successfully." -ForegroundColor Green
}

# 6. Remote verification: every required artifact must now be uploaded in
# full before an initial draft release becomes public.
$remote = Invoke-RestMethod -Uri "$releaseApiUrl/$releaseId" -Headers $headers -Method Get
foreach ($name in $assetsToUpload) {
    $matches = @($remote.assets | Where-Object { $_.name -eq $name })
    if ($matches.Count -ne 1) {
        throw "Missing/duplicated uploaded artifact on GitHub: $name"
    }
    $expectedBytes = (Get-Item (Join-Path $ReleasesDir $name)).Length
    if ($matches[0].size -ne $expectedBytes) {
        throw "GitHub artifact length differs for $name"
    }
    $entry = @($manifest.assets | Where-Object { $_.file -eq $name })
    if ($entry.Count -eq 1 -and $matches[0].digest) {
        if ($matches[0].digest -ne ('sha256:' + $entry[0].sha256)) {
            throw "GitHub asset SHA-256 mismatch: $name"
        }
    }
}
$publishBody = @{ draft = $false } | ConvertTo-Json
Invoke-RestMethod -Method Patch -Uri "$releaseApiUrl/$releaseId" -Headers $headers -Body $publishBody -ContentType "application/json" | Out-Null
Write-Host "All GitHub asset sizes and available digests verified before publication."

Write-Host "`nAll release assets successfully published for $Tag!" -ForegroundColor Green
Write-Host "Release URL: https://github.com/$repo/releases/tag/$Tag" -ForegroundColor Green

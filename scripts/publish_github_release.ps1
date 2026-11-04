param(
    [string]$Tag = "",
    [string]$ReleasesDir = "",
    [switch]$Publish
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

# Pure release-payload checks; no file mutation or GitHub API calls.
function Get-PeArch([IO.Stream]$stream, [string]$label) {
    if ($stream.Length -lt 64) { throw "PE too short: $label" }
    $r = [IO.BinaryReader]::new($stream, [Text.Encoding]::ASCII, $true)
    try {
        $stream.Position = 0
        if ($r.ReadUInt16() -ne 0x5A4D) { throw "Invalid DOS header: $label" }
        $stream.Position = 0x3C
        $pe = $r.ReadInt32()
        if ($pe -lt 64 -or $pe -gt ($stream.Length - 6)) {
            throw "Invalid PE offset: $label"
        }
        $stream.Position = $pe
        if ($r.ReadUInt32() -ne 0x4550) { throw "Invalid PE signature: $label" }
        switch ($r.ReadUInt16()) {
            0x014c { return 'x86' }
            0x8664 { return 'x64' }
            0xaa64 { return 'arm64' }
            default { throw "Unknown PE architecture: $label" }
        }
    } finally { $r.Dispose() }
}
function Assert-PeArchFile([string]$file, [string]$arch) {
    $f = [IO.File]::OpenRead($file)
    try {
        if ((Get-PeArch $f $file) -ne $arch) {
            throw "Architecture mismatch: $file must be $arch"
        }
    } finally { $f.Dispose() }
}
function Assert-MsiContainer([string]$file) {
    $f = [IO.File]::OpenRead($file)
    try {
        $bytes = [byte[]]::new(8)
        if ($f.Read($bytes, 0, 8) -ne 8 -or
            [BitConverter]::ToString($bytes) -ne 'D0-CF-11-E0-A1-B1-1A-E1') {
            throw "Invalid MSI container: $file"
        }
    } finally { $f.Dispose() }
}
function Assert-PortableZip([string]$file, [string]$arch, [string]$exeDigest, [string]$dllDigest) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($file)
    try {
        foreach ($item in @(
            @{ name='VietFlow.exe'; digest=$exeDigest },
            @{ name='vietflow_tsf.dll'; digest=$dllDigest }
        )) {
            $entry = $zip.GetEntry($item.name)
            if ($null -eq $entry) { throw "Portable ZIP missing $($item.name): $file" }
            $inputStream = $entry.Open()
            try {
                $memory = [IO.MemoryStream]::new()
                try {
                    $inputStream.CopyTo($memory)
                    $memory.Position = 0
                    if ((Get-PeArch $memory "$file!$($item.name)") -ne $arch) {
                        throw "Wrong PE architecture: $file!$($item.name)"
                    }
                    $memory.Position = 0
                    $hasher = [Security.Cryptography.SHA256]::Create()
                    try {
                        $digest = [BitConverter]::ToString($hasher.ComputeHash($memory)).Replace('-', '').ToLowerInvariant()
                    } finally { $hasher.Dispose() }
                    if ($digest -ne $item.digest) {
                        throw "Portable ZIP binary differs from verified file: $file!$($item.name)"
                    }
                } finally { $memory.Dispose() }
            } finally { $inputStream.Dispose() }
        }
    } finally { $zip.Dispose() }
}

# 0. Read-only, offline release preflight. Do not fetch credentials unless
# -Publish was explicitly requested and every provenance check passed.
$version = (Get-Content (Join-Path $root 'VERSION') -Raw).Trim()
if ($Tag -ne "v$version" -or $Tag -notmatch '^v[0-9]+[.][0-9]+[.][0-9]+$') {
    throw "Tag must exactly match VERSION: v$version"
}
$manifestPath = Join-Path $ReleasesDir 'release-artifacts.json'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw 'Missing full release provenance manifest'
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$commit = (& git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $manifest.version -ne $version -or
    $manifest.git_commit -ne $commit -or
    (@($manifest.architectures) -join ',') -ne 'x64,x86,arm64') {
    throw 'Release version, source commit or architecture provenance mismatch'
}

# Required release payloads: one exact, SHA-256-locked entry each.
$expectedPayloads = @{}
foreach ($arch in @('x64','x86','arm64')) {
    $dll = switch ($arch) {
        'x86' { 'vietflow_tsf_x86.dll' }
        'arm64' { 'vietflow_tsf_arm64.dll' }
        default { 'vietflow_tsf.dll' }
    }
    $expectedPayloads["VietFlow-$arch.exe"] = $arch
    $expectedPayloads[$dll] = $arch
    $expectedPayloads["VietFlow-win-$arch.msi"] = $arch
    $expectedPayloads["VietFlow-win-$arch-Portable.zip"] = $arch
}
$metadataNames = @(
    'VietFlow.exe', 'latest.json', 'version.txt', 'CHANGELOG.md',
    'release-artifacts.json', 'SHA256SUMS.txt'
)
$assetsToUpload = @($expectedPayloads.Keys | Sort-Object) + $metadataNames
$manifestEntries = @($manifest.assets)
if ($manifestEntries.Count -ne $expectedPayloads.Count) {
    throw 'Release manifest has missing, extra or duplicate payload records'
}
$actualDigests = @{}
foreach ($entry in $manifestEntries) {
    $name = [string]$entry.file
    if (-not $expectedPayloads.ContainsKey($name) -or $actualDigests.ContainsKey($name)) {
        throw "Unexpected or duplicated payload in manifest: $name"
    }
    if ([string]$entry.arch -ne $expectedPayloads[$name] -or
        [long]$entry.bytes -le 0 -or
        [string]$entry.sha256 -notmatch '^[0-9a-fA-F]{64}$') {
        throw "Invalid payload architecture, size or SHA-256: $name"
    }
    $path = Join-Path $ReleasesDir $name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing payload: $name"
    }
    $info = Get-Item -LiteralPath $path
    $digest = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($info.Length -ne [long]$entry.bytes -or
        $digest -ne ([string]$entry.sha256).ToLowerInvariant()) {
        throw "Payload bytes or SHA-256 differ from source provenance: $name"
    }
    $actualDigests[$name] = $digest
}
foreach ($name in $expectedPayloads.Keys) {
    if (-not $actualDigests.ContainsKey($name)) {
        throw "Unverified release payload: $name"
    }
}
# Reject mixed-architecture artifacts even when an incorrect binary
# was accidentally rehashed into its manifest.
foreach ($arch in @('x64','x86','arm64')) {
    $dllName = switch ($arch) {
        'x86' { 'vietflow_tsf_x86.dll' }
        'arm64' { 'vietflow_tsf_arm64.dll' }
        default { 'vietflow_tsf.dll' }
    }
    $exeName = "VietFlow-$arch.exe"
    Assert-PeArchFile (Join-Path $ReleasesDir $exeName) $arch
    Assert-PeArchFile (Join-Path $ReleasesDir $dllName) $arch
    Assert-MsiContainer (Join-Path $ReleasesDir "VietFlow-win-$arch.msi")
    Assert-PortableZip (Join-Path $ReleasesDir "VietFlow-win-$arch-Portable.zip") $arch $actualDigests[$exeName] $actualDigests[$dllName]
}

foreach ($name in $metadataNames) {
    $path = Join-Path $ReleasesDir $name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or
        (Get-Item -LiteralPath $path).Length -le 0) {
        throw "Missing or empty release metadata: $name"
    }
}
# The unversioned executable is a byte-for-byte x64 alias, not a new build.
$x64Alias = (Get-FileHash -LiteralPath (Join-Path $ReleasesDir 'VietFlow.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
if ($x64Alias -ne $actualDigests['VietFlow-x64.exe']) {
    throw 'VietFlow.exe alias differs from the verified x64 executable'
}
if ((Get-Content (Join-Path $ReleasesDir 'version.txt') -Raw).Trim() -ne $version) {
    throw 'version.txt differs from VERSION'
}
$latest = Get-Content (Join-Path $ReleasesDir 'latest.json') -Raw | ConvertFrom-Json
if ($latest.version -ne $version) {
    throw 'Updater manifest version differs from VERSION'
}
$releaseBase = 'https://github.com/KienPC1234/VietFlow/releases'
if ($latest.url -ne "$releaseBase/tag/$Tag") {
    throw 'Updater release link does not point to the verified tag'
}
$expectedDownloads = @{}
foreach ($arch in @('x64','x86','arm64')) {
    $expectedDownloads[$arch] = "$releaseBase/download/$Tag/VietFlow-$arch.exe"
    $expectedDownloads["msi_$arch"] = "$releaseBase/download/$Tag/VietFlow-win-$arch.msi"
}
$downloadNames = @($latest.downloads.PSObject.Properties.Name)
if ($downloadNames.Count -ne $expectedDownloads.Count) {
    throw 'Updater download list has missing or extra architecture entries'
}
foreach ($name in $expectedDownloads.Keys) {
    if ($latest.downloads.$name -ne $expectedDownloads[$name]) {
        throw "Updater download URL mismatch: $name"
    }
}
$expectedChangelog = (Get-FileHash (Join-Path $root 'CHANGELOG.md') -Algorithm SHA256).Hash
$actualChangelog = (Get-FileHash (Join-Path $ReleasesDir 'CHANGELOG.md') -Algorithm SHA256).Hash
if ($expectedChangelog -ne $actualChangelog) {
    throw 'Release changelog differs from the source changelog'
}
# SHA256SUMS must cover exactly every manifest payload, with no duplicates.
$sums = @{}
foreach ($line in @(Get-Content -LiteralPath (Join-Path $ReleasesDir 'SHA256SUMS.txt'))) {
    if ($line -notmatch '^([0-9a-fA-F]{64})  ([^\r\n]+)$') {
        throw 'Malformed SHA256SUMS entry'
    }
    $digest = $Matches[1].ToLowerInvariant()
    $name = $Matches[2]
    if (-not $expectedPayloads.ContainsKey($name) -or $sums.ContainsKey($name)) {
        throw "Extra or duplicate SHA256SUMS entry: $name"
    }
    $sums[$name] = $digest
}
if ($sums.Count -ne $expectedPayloads.Count) {
    throw 'SHA256SUMS must cover every release payload exactly once'
}
foreach ($name in $expectedPayloads.Keys) {
    if ($sums[$name] -ne $actualDigests[$name]) {
        throw "SHA256SUMS does not match verified payload: $name"
    }
}
# A previously created local version tag must never point at a different
# source commit; tags are immutable release identities.
$localTag = & git -C $root rev-parse -q --verify "refs/tags/$Tag^{commit}" 2>$null
if ($LASTEXITCODE -eq 0 -and $localTag -and $localTag.Trim() -ne $commit) {
    throw 'Existing release tag does not refer to the verified source commit'
}
Write-Host "Offline release preflight PASS: $version, $commit, $($assetsToUpload.Count) assets."
if (-not $Publish) {
    Write-Host 'Read-only preflight complete. No remote release action performed.'
    return
}
# Never publish from a dirty source tree or when origin/main has advanced.
# These checks only run after explicit approval through -Publish.
& git -C $root diff-index --quiet HEAD --
if ($LASTEXITCODE -ne 0) {
    throw 'Release requires a clean committed source tree'
}
$remoteLine = & git -C $root ls-remote origin refs/heads/main
if ($LASTEXITCODE -ne 0 -or -not $remoteLine) {
    throw 'Cannot verify origin/main source commit'
}
$remoteCommit = ([string]$remoteLine).Split([char]9)[0]
if ($remoteCommit -ne $commit) {
    throw 'origin/main differs from the verified release source commit'
}
# The publish path is unreachable without explicit -Publish.

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

# A Git tag that already exists is immutable for this release. Resolve an
# annotated tag through to its commit before any remote write.
function Get-RemoteTagCommit([string]$tagName) {
    try {
        $ref = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/git/ref/tags/$tagName" -Headers $headers -Method Get
    } catch {
        $code = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { 0 }
        if ($code -eq 404) { return $null }
        throw
    }
    $obj = $ref.object
    for ($i = 0; $i -lt 3 -and $obj.type -eq 'tag'; $i++) {
        $tagObject = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/git/tags/$($obj.sha)" -Headers $headers -Method Get
        $obj = $tagObject.object
    }
    if ($obj.type -ne 'commit' -or [string]::IsNullOrWhiteSpace($obj.sha)) {
        throw "Cannot resolve remote tag to a commit: $tagName"
    }
    return [string]$obj.sha
}
$tagCommit = Get-RemoteTagCommit $Tag
if ($tagCommit -and $tagCommit -ne $commit) {
    throw 'Existing remote release tag differs from the verified source commit'
}
$release = $null
$notFound = $false
try {
    $release = Invoke-RestMethod -Uri "$releaseApiUrl/tags/$Tag" -Headers $headers -Method Get
    Write-Host "Found release: ID $($release.id)" -ForegroundColor Yellow
} catch {
    $code = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { 0 }
    if ($code -eq 404) { $notFound = $true } else { throw }
}
if ($notFound) {
    Write-Host "Release $Tag does not exist. Creating a draft..." -ForegroundColor Cyan
    $createBody = @{
        tag_name         = $Tag
        target_commitish = $commit
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
if ($release.tag_name -ne $Tag) {
    throw 'Release tag identity differs from requested tag'
}
if (-not $tagCommit -and $release.target_commitish -ne $commit) {
    throw 'Draft release target_commitish differs from verified source commit'
}
$releaseId = $release.id

# 4. List existing assets
$relDetail = Invoke-RestMethod -Uri "$releaseApiUrl/$releaseId" -Headers $headers -Method Get
$existingAssets = @{}
if ($relDetail.assets) {
    foreach ($asset in $relDetail.assets) {
        if ($existingAssets.ContainsKey($asset.name)) {
            throw "Duplicate remote draft asset: $($asset.name)"
        }
        $existingAssets[$asset.name] = $asset
    }
}

# 5. Upload only previously verified assets.
foreach ($assetName in $assetsToUpload) {
    $filePath = Join-Path $ReleasesDir $assetName
    if (-not (Test-Path $filePath)) {
        throw "Required asset missing: $filePath"
    }

    # Never replace even a draft asset destructively. A successful retry
    # can skip only an exact size+SHA match. Mismatches require operator review.
    if ($existingAssets.ContainsKey($assetName)) {
        $existing = $existingAssets[$assetName]
        $expectedDigest = 'sha256:' + (Get-FileHash -LiteralPath $filePath -Algorithm SHA256).Hash.ToLowerInvariant()
        $expectedLength = (Get-Item -LiteralPath $filePath).Length
        if ($existing.size -ne $expectedLength -or
            [string]$existing.digest -ne $expectedDigest) {
            throw "Existing draft asset differs or lacks a trusted digest: $assetName"
        }
        Write-Host "Verified existing draft asset: $assetName"
        continue
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
    $expectedDigest = 'sha256:' + (Get-FileHash -LiteralPath (Join-Path $ReleasesDir $name) -Algorithm SHA256).Hash.ToLowerInvariant()
    if ([string]$matches[0].digest -ne $expectedDigest) {
        throw "Remote asset digest missing or mismatched: $name"
    }
}
if (@($remote.assets).Count -ne $assetsToUpload.Count) {
    throw 'Remote draft contains unexpected extra assets; refusing publication'
}
# Check that an existing tag has not changed during upload.
$finalTagCommit = Get-RemoteTagCommit $Tag
if ($finalTagCommit -and $finalTagCommit -ne $commit) {
    throw 'Remote tag moved while uploading draft assets'
}
$publishBody = @{ draft = $false } | ConvertTo-Json
Invoke-RestMethod -Method Patch -Uri "$releaseApiUrl/$releaseId" -Headers $headers -Body $publishBody -ContentType "application/json" | Out-Null
Write-Host "All GitHub asset sizes and available digests verified before publication."

Write-Host "`nAll release assets successfully published for $Tag!" -ForegroundColor Green
Write-Host "Release URL: https://github.com/$repo/releases/tag/$Tag" -ForegroundColor Green

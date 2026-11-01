$ErrorActionPreference = "Stop"
$env:GIT_TERMINAL_PROMPT = '0'
$env:GCM_INTERACTIVE = 'never'

$cred = @('protocol=https', 'host=github.com', '') | git credential fill 2>$null
$token = ($cred | Select-String '^password=').Line.Substring(9)
if (-not $token) { throw "No GitHub credential available" }

$headers = @{
    Authorization = "token $token"
    'User-Agent'  = 'VietFlow'
    Accept        = 'application/vnd.github+json'
}

$manifest = Get-Content (Join-Path $PSScriptRoot '..\dist\latest.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$tag = "v$($manifest.version)"

$releaseBody = [ordered]@{
    tag_name         = $tag
    target_commitish = 'main'
    name             = "VietFlow $($manifest.version)"
    body             = $manifest.notes
    draft            = $false
    prerelease       = $false
} | ConvertTo-Json -Depth 4

$release = $null
try {
    $release = Invoke-RestMethod -Method Get `
        -Uri "https://api.github.com/repos/KienPC1234/VietFlow/releases/tags/$tag" `
        -Headers $headers
    Write-Host "Found existing release $($release.tag_name) (id $($release.id))"
} catch {
    $release = Invoke-RestMethod -Method Post `
        -Uri "https://api.github.com/repos/KienPC1234/VietFlow/releases" `
        -Headers $headers `
        -Body ([System.Text.Encoding]::UTF8.GetBytes($releaseBody)) `
        -ContentType 'application/json; charset=utf-8'
    Write-Host "Created release $($release.tag_name) (id $($release.id))"
}
Write-Host $release.html_url

$requiredAssets = @(
    'dist\VietFlow-x64.exe',
    'dist\VietFlow-x86.exe',
    'dist\latest.json',
    'dist\version.txt',
    'dist\CHANGELOG.md'
)
$optionalAssets = @(
    'dist\VietFlow.exe',
    'dist\vietflow_tip.dll',
    'dist\VietFlow-arm64.exe',
    'releases\VietFlow-win-x64-Setup.exe',
    'releases\VietFlow-win-x86-Setup.exe',
    'releases\VietFlow-win-arm64-Setup.exe',
    'releases\VietFlow-win-x64.msi',
    'releases\VietFlow-win-x86.msi',
    'releases\VietFlow-win-arm64.msi',
    'releases\VietFlow-win-x64-Portable.zip',
    'releases\VietFlow-win-x86-Portable.zip',
    'releases\VietFlow-win-arm64-Portable.zip'
)

$assets = @($requiredAssets)
foreach ($asset in $optionalAssets) {
    $path = Join-Path (Split-Path -Parent $PSScriptRoot) $asset
    if (Test-Path $path) { $assets += $asset }
}

# Refresh current assets list
$currentRelease = Invoke-RestMethod -Method Get -Uri "https://api.github.com/repos/KienPC1234/VietFlow/releases/$($release.id)" -Headers $headers
$existingAssets = $currentRelease.assets

foreach ($asset in $assets) {
    $path = Join-Path (Split-Path -Parent $PSScriptRoot) $asset
    if (-not (Test-Path $path)) { throw "Missing asset: $path" }
    $name = Split-Path $path -Leaf

    $existing = $existingAssets | Where-Object { $_.name -eq $name }
    if ($existing) {
        Write-Host "Deleting outdated asset $name (id $($existing.id))..."
        Invoke-RestMethod -Method Delete -Uri $existing.url -Headers $headers
    }

    $uri = "https://uploads.github.com/repos/KienPC1234/VietFlow/releases/$($release.id)/assets?name=$([uri]::EscapeDataString($name))"
    $uploaded = Invoke-RestMethod -Method Post -Uri $uri -Headers $headers -InFile $path -ContentType 'application/octet-stream'
    Write-Host ("Uploaded {0} ({1:N2} MB)" -f $uploaded.name, ($uploaded.size / 1MB))
}

Write-Host "DONE"

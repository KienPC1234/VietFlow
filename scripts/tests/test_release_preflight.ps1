# Entirely offline regression tests for the release publisher's read-only
# preflight. Fixtures are synthetic and must never be published.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$publisher = Join-Path $root 'scripts/publish-release.ps1'
$artifactVerifier = Join-Path $root 'scripts/verify_release_artifacts.ps1'
$version = (Get-Content (Join-Path $root 'VERSION') -Raw).Trim()
$commit = (& git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot determine source commit' }
$dir = Join-Path ([IO.Path]::GetTempPath()) ('vietflow-preflight-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $dir -Force | Out-Null

function Make-Pe([string]$arch) {
    $machines = @{ x64=0x8664; x86=0x014c; arm64=0xaa64 }
    $bytes = [byte[]]::new(128)
    $bytes[0] = 0x4d; $bytes[1] = 0x5a
    [BitConverter]::GetBytes([int]64).CopyTo($bytes, 0x3c)
    $bytes[64] = 0x50; $bytes[65] = 0x45
    [BitConverter]::GetBytes([uint16]$machines[$arch]).CopyTo($bytes, 68)
    return ,$bytes
}
function File-Sha([string]$file) {
    (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
}
function Set-Text([string]$file, [string]$text) {
    [IO.File]::WriteAllText($file, $text, [Text.UTF8Encoding]::new($false))
}
function Make-Zip([string]$file, [byte[]]$exe, [byte[]]$dll) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::Open($file, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($part in @(
            @{name='VietFlow.exe';bytes=$exe},
            @{name='vietflow_tsf.dll';bytes=$dll}
        )) {
            $entry = $zip.CreateEntry($part.name)
            $stream = $entry.Open()
            try {
                $stream.Write($part.bytes, 0, $part.bytes.Length)
            } finally { $stream.Dispose() }
        }
    } finally { $zip.Dispose() }
}
# Create a real, minimal Windows Installer database for the synthetic test.
# This is not an installable product and is NEVER submitted to msiexec.
# The publisher must reject fake OLE headers and identity/version mismatches.
function Make-Msi([string]$path, [string]$arch, [string]$msiVersion,
                  [string]$name = 'VietFlow',
                  [string]$upgrade = '{B9A5320F-D409-4BFB-A63A-9E3DA4D3A1F8}',
                  [string]$scope = '1',
                  [string]$productCode = '',
                  [string]$templateOverride = '') {
    if ([string]::IsNullOrWhiteSpace($productCode)) {
        $productCode = '{' + [guid]::NewGuid().ToString().ToUpperInvariant() + '}'
    }
    $template = if ($templateOverride) { $templateOverride } else {
        switch ($arch) {
            'x86' { 'Intel;1033' }
            'arm64' { 'Arm64;1033' }
            default { 'x64;1033' }
        }
    }
    $inst = New-Object -ComObject WindowsInstaller.Installer
    $bt = [char]96
    $db = $inst.OpenDatabase($path, 3)
    try {
        $create = 'CREATE TABLE ' + $bt + 'Property' + $bt + ' (' +
                  $bt + 'Property' + $bt + ' CHAR(72) NOT NULL, ' +
                  $bt + 'Value' + $bt + ' CHAR(255) NOT NULL PRIMARY KEY ' +
                  $bt + 'Property' + $bt + ')'
        $view = $db.OpenView($create)
        $view.Execute()
        $view.Close()
        $view = $null
        $properties = [ordered]@{
            ProductVersion = $msiVersion
            ProductName = $name
            ProductCode = $productCode
            UpgradeCode = $upgrade
            ProductLanguage = '1033'
            ALLUSERS = $scope
        }
        $query = 'INSERT INTO ' + $bt + 'Property' + $bt + ' (' +
                 $bt + 'Property' + $bt + ', ' + $bt + 'Value' + $bt +
                 ') VALUES (?, ?)'
        foreach ($item in $properties.GetEnumerator()) {
            $view = $db.OpenView($query)
            $record = $inst.CreateRecord(2)
            $record.StringData(1) = [string]$item.Key
            $record.StringData(2) = [string]$item.Value
            $view.Execute($record)
            $view.Close()
            $record = $null
            $view = $null
        }
        $summary = $db.SummaryInformation(20)
        $summary.Property(7) = $template
        $summary.Persist()
        $summary = $null
        $db.Commit()
    } finally {
        $record = $null
        $view = $null
        $summary = $null
        $db = $null
        $inst = $null
        [gc]::Collect()
        [gc]::WaitForPendingFinalizers()
    }
}

function Verify-Offline([string]$fixture, [string]$caseName, [bool]$expectPass) {
    $passed = $false
    try {
        & $publisher -Tag "v$version" -ReleasesDir $fixture | Out-Null
        $passed = $true
    } catch {
        if ($expectPass) { throw "Unexpected rejection ($caseName): $_" }
    }
    if ($passed -ne $expectPass) {
        throw "Incorrect result for $caseName (expected=$expectPass, got=$passed)"
    }
    Write-Host "PASS offline: $caseName"
}
function New-Case([string]$name) {
    $target = Join-Path $dir ('case-' + $name)
    Copy-Item -LiteralPath (Join-Path $dir 'valid') -Destination $target -Recurse
    return $target
}
function Update-ManifestHash([string]$fixture, [string]$name) {
    $p = Join-Path $fixture 'release-artifacts.json'
    $manifest = Get-Content -LiteralPath $p -Raw | ConvertFrom-Json
    $entry = @($manifest.assets | Where-Object { $_.file -eq $name })
    if ($entry.Count -ne 1) { throw 'Test fixture missing hash entry' }
    $entry[0].bytes = (Get-Item (Join-Path $fixture $name)).Length
    $entry[0].sha256 = File-Sha (Join-Path $fixture $name)
    Set-Text $p ($manifest | ConvertTo-Json -Depth 8)
    $sums = Join-Path $fixture 'SHA256SUMS.txt'
    $lines = @(Get-Content $sums)
    $lines = @($lines | ForEach-Object {
        if ($_ -match ('^[0-9a-fA-F]{64}  ' + [regex]::Escape($name) + '$')) {
            $entry[0].sha256 + '  ' + $name
        } else { $_ }
    })
    Set-Text $sums (($lines -join [Environment]::NewLine) + [Environment]::NewLine)
}

try {
    $valid = Join-Path $dir 'valid'
    New-Item -ItemType Directory -Path $valid | Out-Null
    $manifestAssets = @()
    $sums = @()
    foreach ($arch in @('x64','x86','arm64')) {
        $dll = switch ($arch) {
            'x86' { 'vietflow_tsf_x86.dll' }
            'arm64' { 'vietflow_tsf_arm64.dll' }
            default { 'vietflow_tsf.dll' }
        }
        $exe = 'VietFlow-' + $arch + '.exe'
        $msi = 'VietFlow-win-' + $arch + '.msi'
        $zipName = 'VietFlow-win-' + $arch + '-Portable.zip'
        $pe = Make-Pe $arch
        [IO.File]::WriteAllBytes((Join-Path $valid $exe), $pe)
        [IO.File]::WriteAllBytes((Join-Path $valid $dll), $pe)
        Make-Msi (Join-Path $valid $msi) $arch $version
        Make-Zip (Join-Path $valid $zipName) $pe $pe
        foreach ($name in @($exe,$dll,$msi,$zipName)) {
            $file = Join-Path $valid $name
            $sha = File-Sha $file
            $manifestAssets += [ordered]@{
                file=$name;arch=$arch;kind='test-fixture'
                bytes=(Get-Item $file).Length;sha256=$sha
            }
            $sums += ($sha + '  ' + $name)
        }
    }
    Copy-Item (Join-Path $valid 'VietFlow-x64.exe') (Join-Path $valid 'VietFlow.exe')
    Copy-Item (Join-Path $root 'CHANGELOG.md') (Join-Path $valid 'CHANGELOG.md')
    Set-Text (Join-Path $valid 'version.txt') ($version + [Environment]::NewLine)
    $base = 'https://github.com/KienPC1234/VietFlow/releases'
    $downloads = @{}
    foreach ($arch in @('x64','x86','arm64')) {
        $downloads[$arch] = "$base/download/v$version/VietFlow-$arch.exe"
        $downloads["msi_$arch"] = "$base/download/v$version/VietFlow-win-$arch.msi"
    }
    Set-Text (Join-Path $valid 'latest.json') (
        @{version=$version;url="$base/tag/v$version";notes='Test only';downloads=$downloads} | ConvertTo-Json -Depth 6)
    Set-Text (Join-Path $valid 'SHA256SUMS.txt') (($sums -join [Environment]::NewLine) + [Environment]::NewLine)
    Set-Text (Join-Path $valid 'release-artifacts.json') (
        [ordered]@{schema=1;version=$version;git_commit=$commit
                   architectures=@('x64','x86','arm64');assets=$manifestAssets} | ConvertTo-Json -Depth 8)

    Verify-Offline $valid 'complete 3-architecture fixture' $true

    # Exercise the earlier manifest-generating verification path, not only
    # the final publisher. Both must reject a forged MSI database.
    $case = New-Case 'artifact-verifier-good'
    & $artifactVerifier -Arch All -DistDir $case -ReleaseDir $case -RequireRelease | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Artifact verifier failed a valid MSI fixture' }
    Write-Host 'PASS offline: release manifest verifier accepts native MSI tables'

    $case = New-Case 'artifact-verifier-fake-header'
    $file = Join-Path $case 'VietFlow-win-x64.msi'
    [IO.File]::WriteAllBytes($file, [byte[]](0xD0,0xCF,0x11,0xE0,0xA1,0xB1,0x1A,0xE1,0x00))
    $rejected = $false
    try { & $artifactVerifier -Arch All -DistDir $case -ReleaseDir $case -RequireRelease | Out-Null }
    catch { $rejected = $true }
    if (-not $rejected) { throw 'Manifest generation accepted a fake MSI container' }
    Write-Host 'PASS offline: manifest generator rejects fake MSI header'


    $case = New-Case 'missing'
    Remove-Item (Join-Path $case 'VietFlow-win-arm64.msi')
    Verify-Offline $case 'missing MSI' $false

    $case = New-Case 'tampered'
    [IO.File]::AppendAllText((Join-Path $case 'VietFlow-x86.exe'), 'tampered')
    Verify-Offline $case 'modified binary digest' $false

    $case = New-Case 'duplicate'
    $manifest = Get-Content (Join-Path $case 'release-artifacts.json') -Raw | ConvertFrom-Json
    $manifest.assets = @($manifest.assets) + @($manifest.assets[0])
    Set-Text (Join-Path $case 'release-artifacts.json') ($manifest | ConvertTo-Json -Depth 8)
    Verify-Offline $case 'duplicated manifest payload' $false

    $case = New-Case 'wrong-arch'
    Copy-Item (Join-Path $case 'VietFlow-x86.exe') (Join-Path $case 'VietFlow-x64.exe') -Force
    Update-ManifestHash $case 'VietFlow-x64.exe'
    Copy-Item (Join-Path $case 'VietFlow-x64.exe') (Join-Path $case 'VietFlow.exe') -Force
    Verify-Offline $case 'incorrect PE machine even with consistent digests' $false

    $case = New-Case 'wrong-zip'
    Remove-Item (Join-Path $case 'VietFlow-win-x64-Portable.zip')
    Make-Zip (Join-Path $case 'VietFlow-win-x64-Portable.zip') (Make-Pe 'x86') (Make-Pe 'x64')
    Update-ManifestHash $case 'VietFlow-win-x64-Portable.zip'
    Verify-Offline $case 'wrong ZIP contents even with consistent digests' $false


    $case = New-Case 'fake-ole-msi'
    $file = Join-Path $case 'VietFlow-win-x64.msi'
    [IO.File]::WriteAllBytes($file, [byte[]](0xD0,0xCF,0x11,0xE0,0xA1,0xB1,0x1A,0xE1,0x00))
    Update-ManifestHash $case 'VietFlow-win-x64.msi'
    Verify-Offline $case 'OLE header alone cannot impersonate an MSI' $false

    $case = New-Case 'msi-wrong-version'
    $file = Join-Path $case 'VietFlow-win-x64.msi'
    Remove-Item -LiteralPath $file -Force
    Make-Msi $file 'x64' '9.9.9'
    Update-ManifestHash $case 'VietFlow-win-x64.msi'
    Verify-Offline $case 'MSI ProductVersion disagrees with tag despite matching digests' $false

    $case = New-Case 'msi-wrong-arch'
    $file = Join-Path $case 'VietFlow-win-arm64.msi'
    Remove-Item -LiteralPath $file -Force
    Make-Msi $file 'arm64' $version -templateOverride 'Intel;1033'
    Update-ManifestHash $case 'VietFlow-win-arm64.msi'
    Verify-Offline $case 'MSI database template wrong architecture' $false


    $case = New-Case 'msi-wrong-product'
    $file = Join-Path $case 'VietFlow-win-x86.msi'
    Remove-Item -LiteralPath $file -Force
    Make-Msi $file 'x86' $version -name 'Impostor'
    Update-ManifestHash $case 'VietFlow-win-x86.msi'
    Verify-Offline $case 'MSI ProductName is not VietFlow' $false

    $case = New-Case 'msi-wrong-upgrade'
    $file = Join-Path $case 'VietFlow-win-x86.msi'
    Remove-Item -LiteralPath $file -Force
    Make-Msi $file 'x86' $version -upgrade '{11111111-1111-1111-1111-111111111111}'
    Update-ManifestHash $case 'VietFlow-win-x86.msi'
    Verify-Offline $case 'MSI unrelated upgrade family' $false

    $case = New-Case 'msi-wrong-scope'
    $file = Join-Path $case 'VietFlow-win-x64.msi'
    Remove-Item -LiteralPath $file -Force
    Make-Msi $file 'x64' $version -scope '2'
    Update-ManifestHash $case 'VietFlow-win-x64.msi'
    Verify-Offline $case 'MSI not per-machine' $false

    $case = New-Case 'msi-wrong-product-code'
    $file = Join-Path $case 'VietFlow-win-x64.msi'
    Remove-Item -LiteralPath $file -Force
    Make-Msi $file 'x64' $version -productCode '{00000000-0000-0000-0000-000000000000}'
    Update-ManifestHash $case 'VietFlow-win-x64.msi'
    Verify-Offline $case 'MSI has empty ProductCode GUID' $false

    $case = New-Case 'wrong-sums'
    Set-Text (Join-Path $case 'SHA256SUMS.txt') ('0' * 64 + '  VietFlow-x64.exe')
    Verify-Offline $case 'incomplete SHA256SUMS' $false

    $case = New-Case 'wrong-version'
    Set-Text (Join-Path $case 'version.txt') '999.999.999'
    Verify-Offline $case 'wrong metadata version' $false

    $case = New-Case 'wrong-alias'
    [IO.File]::AppendAllText((Join-Path $case 'VietFlow.exe'), 'extra')
    Verify-Offline $case 'incorrect x64 alias' $false

    $case = New-Case 'updater-redirect'
    $json = Get-Content (Join-Path $case 'latest.json') -Raw | ConvertFrom-Json
    $json.downloads.x64 = 'https://untrusted.example/unsafe.exe'
    Set-Text (Join-Path $case 'latest.json') ($json | ConvertTo-Json -Depth 8)
    Verify-Offline $case 'unexpected updater download origin' $false

    $blocked = $false
    try { & $publisher -Draft -Publish -ReleasesDir $valid | Out-Null }
    catch { $blocked = $true }
    if (-not $blocked) { throw 'Conflicting publish and draft flags were not blocked' }
    Write-Host 'PASS offline: conflicting public release flags'

    Write-Host 'PASS: all release publisher offline regressions'
} finally {
    Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue
}

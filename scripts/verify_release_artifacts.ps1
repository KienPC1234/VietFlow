# VietFlow reproducible PE/ZIP/MSI architecture and release provenance gate.
# Read-only validation; never installs binaries or touches running VietFlow.
[CmdletBinding()]
param(
    [ValidateSet('x64','x86','arm64','All')][string]$Arch = 'All',
    [string]$DistDir = 'dist',
    [string]$ReleaseDir = 'releases',
    [switch]$RequireRelease
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
# Prevent accidental data erasure through an elevated MSI uninstall custom action.
$wix = Get-Content (Join-Path $root 'installer/wix/VietFlow.wxs') -Raw
if ($wix -match '--uninstall-cleanup' -or $wix -match 'FullCleanupOnUninstall') {
    throw 'WiX uninstaller may erase user configuration; release blocked'
}

if (-not [IO.Path]::IsPathRooted($DistDir)) { $DistDir = Join-Path $root $DistDir }
if (-not [IO.Path]::IsPathRooted($ReleaseDir)) { $ReleaseDir = Join-Path $root $ReleaseDir }

function Assert-True([bool]$ok, [string]$message) {
    if (-not $ok) { throw $message }
}
function Get-PeMachine([string]$path) {
    Assert-True (Test-Path -LiteralPath $path -PathType Leaf) "Missing PE: $path"
    $f = [IO.File]::OpenRead($path)
    try {
        Assert-True ($f.Length -ge 64) "PE file too short: $path"
        $r = [IO.BinaryReader]::new($f)
        Assert-True ($r.ReadUInt16() -eq 0x5A4D) "Invalid MZ: $path"
        $f.Position = 0x3C
        $offset = $r.ReadInt32()
        Assert-True ($offset -ge 64 -and $offset -le $f.Length-6) "Invalid PE offset: $path"
        $f.Position = $offset
        Assert-True ($r.ReadUInt32() -eq 0x00004550) "Invalid PE header: $path"
        $machine = $r.ReadUInt16()
        switch ($machine) {
            0x014C {'x86'}
            0x8664 {'x64'}
            0xAA64 {'arm64'}
            default { throw "Unsupported PE machine $machine in $path" }
        }
    } finally { $f.Dispose() }
}
function Assert-PE([string]$path, [string]$arch) {
    $machine = Get-PeMachine $path
    Assert-True ($machine -eq $arch) "Architecture mismatch: expected $arch, got $machine in $path"
}
function Assert-Msi([string]$path) {
    Assert-True (Test-Path -LiteralPath $path -PathType Leaf) "Missing MSI: $path"
    $f=[IO.File]::OpenRead($path)
    try {
        $b = [byte[]]::new(8)
        Assert-True ($f.Read($b,0,8) -eq 8) "MSI too short"
        Assert-True (([BitConverter]::ToString($b)) -eq 'D0-CF-11-E0-A1-B1-1A-E1') "Invalid MSI magic: $path"
    } finally { $f.Dispose() }
}
function Assert-Zip([string]$path, [string]$arch) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    Assert-True (Test-Path -LiteralPath $path -PathType Leaf) "Missing ZIP: $path"
    $zip=[IO.Compression.ZipFile]::OpenRead($path)
    try {
        foreach ($name in @('VietFlow.exe','vietflow_tsf.dll')) {
            $entry = $zip.GetEntry($name)
            Assert-True ($null -ne $entry) "ZIP missing $name : $path"
            $tmp = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString('N')+'.bin')
            try {
                $inputStream = $entry.Open()
                try {
                    $outputStream=[IO.File]::Create($tmp)
                    try { $inputStream.CopyTo($outputStream) } finally { $outputStream.Dispose() }
                } finally { $inputStream.Dispose() }
                Assert-PE $tmp $arch
            } finally { Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }
        }
    } finally { $zip.Dispose() }
}
$archs = if ($Arch -eq 'All') { @('x64','x86','arm64') } else { @($Arch) }
$assets = [System.Collections.Generic.List[object]]::new()
function Record-Asset([string]$path, [string]$arch, [string]$kind) {
    $info=Get-Item -LiteralPath $path
    $hash=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    $script:assets.Add([ordered]@{
        file=$info.Name; arch=$arch; kind=$kind; bytes=$info.Length; sha256=$hash
    })
}
foreach ($a in $archs) {
    $exe = Join-Path $DistDir "VietFlow-$a.exe"
    $dllName = switch ($a) {
        'x86' {'vietflow_tsf_x86.dll'}
        'arm64' {'vietflow_tsf_arm64.dll'}
        default {'vietflow_tsf.dll'}
    }
    $dll = Join-Path $DistDir $dllName
    Assert-PE $exe $a
    Assert-PE $dll $a
    Record-Asset $exe $a 'portable-exe'
    Record-Asset $dll $a 'tsf-dll'
    if ($RequireRelease) {
        $msi = Join-Path $ReleaseDir "VietFlow-win-$a.msi"
        $zip = Join-Path $ReleaseDir "VietFlow-win-$a-Portable.zip"
        Assert-Msi $msi
        Assert-Zip $zip $a
        Record-Asset $msi $a 'msi'
        Record-Asset $zip $a 'portable-zip'
        $publishedDll = Join-Path $ReleaseDir $dllName
        Assert-PE $publishedDll $a
        Assert-True ((Get-FileHash $publishedDll -Algorithm SHA256).Hash -eq
                     (Get-FileHash $dll -Algorithm SHA256).Hash) "Release DLL differs from validated build"
    }
}
$version = (Get-Content (Join-Path $root 'VERSION') -Raw).Trim()
$commit = (& git -C $root rev-parse HEAD)
Assert-True ($LASTEXITCODE -eq 0) 'Cannot resolve git source commit'
$manifest = [ordered]@{
    schema = 1
    version = $version
    git_commit = $commit.Trim()
    architectures = $archs
    assets = @($assets.ToArray())
}
$outDir = if ($RequireRelease) { $ReleaseDir } else { $DistDir }
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$name = if ($RequireRelease) {'release-artifacts.json'} else {"pe-manifest-$Arch.json"}
$out = Join-Path $outDir $name
[IO.File]::WriteAllText($out, (($manifest | ConvertTo-Json -Depth 7) + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
if ($RequireRelease) {
    $sumPath = Join-Path $ReleaseDir 'SHA256SUMS.txt'
    $text = ($assets | ForEach-Object { "$($_.sha256)  $($_.file)" }) -join [Environment]::NewLine
    [IO.File]::WriteAllText($sumPath, ($text + [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
}
Write-Host "Verified $($assets.Count) assets for $($archs -join ','): $out"

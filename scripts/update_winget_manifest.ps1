[CmdletBinding()]
param(
    [string]$Version = '',
    [string]$ReleasesDir = '',
    [string]$PackageIdentifier = 'KienPC1234.VietFlow',
    [switch]$Submit,
    [switch]$Update
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = (Get-Content -LiteralPath (Join-Path $root 'VERSION') -Raw).Trim()
}
$Version = $Version.TrimStart('v', 'V')

if ([string]::IsNullOrWhiteSpace($ReleasesDir)) {
    $ReleasesDir = Join-Path $root 'releases'
}

$tag = "v$Version"
$releaseBase = 'https://github.com/KienPC1234/VietFlow/releases'
$manifestDir = Join-Path $root "installer\winget\$PackageIdentifier\$Version"
New-Item -ItemType Directory -Force -Path $manifestDir | Out-Null

function Get-MsiMetadata([string]$MsiPath) {
    if (-not (Test-Path -LiteralPath $MsiPath -PathType Leaf)) {
        throw "Missing MSI file: $MsiPath"
    }
    $resolved = (Resolve-Path -LiteralPath $MsiPath).Path
    $wi = New-Object -ComObject WindowsInstaller.Installer
    $db = $wi.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $wi, @($resolved, 0))
    $props = @{}
    foreach ($p in @('ProductCode', 'UpgradeCode', 'ProductName', 'ProductVersion', 'Manufacturer')) {
        $view = $db.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $db, @("SELECT Value FROM Property WHERE Property='$p'"))
        $view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null)
        $rec = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)
        if ($rec) {
            $props[$p] = [string]$rec.GetType().InvokeMember('StringData', 'GetProperty', $null, $rec, @(1))
        }
        $view.GetType().InvokeMember('Close', 'InvokeMethod', $null, $view, $null)
    }
    $props['Sha256'] = (Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash.ToUpperInvariant()
    return $props
}

$arches = @('x64', 'x86', 'arm64')
$msiData = @{}
foreach ($arch in $arches) {
    $msiPath = Join-Path $ReleasesDir "VietFlow-win-$arch.msi"
    $msiData[$arch] = Get-MsiMetadata $msiPath
}

$upgradeCode = $msiData['x64'].UpgradeCode
$displayName = $msiData['x64'].ProductName
$publisher = $msiData['x64'].Manufacturer
$releaseDate = (Get-Date).ToString('yyyy-MM-dd')
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)

$versionYaml = @"
# Created using update_winget_manifest.ps1
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.version.1.10.0.schema.json

PackageIdentifier: $PackageIdentifier
PackageVersion: $Version
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.10.0
"@

$installerYaml = @"
# Created using update_winget_manifest.ps1
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.installer.1.10.0.schema.json

PackageIdentifier: $PackageIdentifier
PackageVersion: $Version
InstallerLocale: en-US
InstallerType: msi
Scope: machine
InstallModes:
- interactive
- silent
- silentWithProgress
UpgradeBehavior: install
ReleaseDate: $releaseDate
AppsAndFeaturesEntries:
- DisplayName: $displayName
  Publisher: $publisher
  DisplayVersion: $Version
  UpgradeCode: '$upgradeCode'
Installers:
- Architecture: x64
  InstallerUrl: $releaseBase/download/$tag/VietFlow-win-x64.msi
  InstallerSha256: $($msiData['x64'].Sha256)
  ProductCode: '$($msiData['x64'].ProductCode)'
- Architecture: x86
  InstallerUrl: $releaseBase/download/$tag/VietFlow-win-x86.msi
  InstallerSha256: $($msiData['x86'].Sha256)
  ProductCode: '$($msiData['x86'].ProductCode)'
- Architecture: arm64
  InstallerUrl: $releaseBase/download/$tag/VietFlow-win-arm64.msi
  InstallerSha256: $($msiData['arm64'].Sha256)
  ProductCode: '$($msiData['arm64'].ProductCode)'
ManifestType: installer
ManifestVersion: 1.10.0
"@

$localeYaml = @"
# Created using update_winget_manifest.ps1
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.defaultLocale.1.10.0.schema.json

PackageIdentifier: $PackageIdentifier
PackageVersion: $Version
PackageLocale: en-US
Publisher: $publisher
PublisherUrl: https://github.com/KienPC1234/VietFlow
PublisherSupportUrl: https://github.com/KienPC1234/VietFlow/issues
PackageName: $displayName
PackageUrl: https://github.com/KienPC1234/VietFlow
License: GPL-3.0
LicenseUrl: https://github.com/KienPC1234/VietFlow/blob/main/LICENSE
ShortDescription: A fast Vietnamese input method for Windows.
Description: VietFlow is a fast, modern Vietnamese input method engine (IME) for Windows supporting Telex, VNI, VIQR, native Text Services Framework (TSF), smart English auto-restore, and per-application profiles.
Moniker: vietflow
Tags:
- vietnamese
- ime
- input-method
- unikey
- telex
- vni
- tsf
ReleaseNotesUrl: $releaseBase/tag/$tag
ManifestType: defaultLocale
ManifestVersion: 1.10.0
"@

[IO.File]::WriteAllText((Join-Path $manifestDir "$PackageIdentifier.yaml"), ($versionYaml.TrimEnd() + "`n"), $utf8NoBom)
[IO.File]::WriteAllText((Join-Path $manifestDir "$PackageIdentifier.installer.yaml"), ($installerYaml.TrimEnd() + "`n"), $utf8NoBom)
[IO.File]::WriteAllText((Join-Path $manifestDir "$PackageIdentifier.locale.en-US.yaml"), ($localeYaml.TrimEnd() + "`n"), $utf8NoBom)

Write-Host "Generated WinGet manifests in: $manifestDir" -ForegroundColor Green

& winget validate --manifest $manifestDir
if ($LASTEXITCODE -ne 0) {
    throw "winget validate failed for $manifestDir"
}

if ($Submit) {
    $prTitle = if ($Update) {
        "New version: $PackageIdentifier version $Version"
    } else {
        "New package: $PackageIdentifier version $Version"
    }
    $token = (& gh auth token 2>$null)
    if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($token)) {
        & wingetcreate submit --no-open --prtitle $prTitle --token $token.Trim() $manifestDir
    } else {
        & wingetcreate submit --no-open --prtitle $prTitle $manifestDir
    }
    if ($LASTEXITCODE -ne 0) {
        throw "wingetcreate submit failed"
    }
}

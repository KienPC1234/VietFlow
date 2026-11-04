# Legacy release entry point. All publishing uses the canonical verified path.
# Default is offline preflight. No remote mutation without explicit -Publish.
[CmdletBinding()]
param(
    [string]$Tag = '',
    [switch]$Draft,
    [switch]$Prerelease,
    [switch]$Publish,
    [string]$ReleasesDir = ''
)
$ErrorActionPreference = 'Stop'
if ($Prerelease) {
    throw 'Prerelease publishing is not implemented by the verified publisher'
}
if ($Draft -and $Publish) {
    throw 'Conflicting -Draft and -Publish: run the preflight without -Publish'
}
$forward = @{}
if (-not [string]::IsNullOrWhiteSpace($Tag)) { $forward.Tag = $Tag }
if (-not [string]::IsNullOrWhiteSpace($ReleasesDir)) { $forward.ReleasesDir = $ReleasesDir }
if ($Publish) { $forward.Publish = $true }
& (Join-Path $PSScriptRoot 'publish_github_release.ps1') @forward
if (-not $?) { throw 'Canonical release gate failed' }

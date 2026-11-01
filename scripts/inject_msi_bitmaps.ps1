param(
    [string]$MsiPath
)

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$dialogBmp = Join-Path $root "assets\wix_dialog.bmp"
$bannerBmp = Join-Path $root "assets\wix_banner.bmp"

if (-not (Test-Path $MsiPath)) {
    Write-Warning "File not found: $MsiPath"
    exit 1
}

$wi = New-Object -ComObject WindowsInstaller.Installer
$db = $wi.OpenDatabase($MsiPath, 1)

# 1. Update WixUI_Bmp_Dialog
$view = $db.OpenView("SELECT Name, Data FROM Binary WHERE Name = 'WixUI_Bmp_Dialog'")
$view.Execute()
$rec = $view.Fetch()
if ($rec) {
    $rec.SetStream(2, $dialogBmp)
    $view.Modify(2, $rec)
    Write-Host "Updated WixUI_Bmp_Dialog in $MsiPath"
}
$view.Close()

# 2. Update WixUI_Bmp_Banner
$view = $db.OpenView("SELECT Name, Data FROM Binary WHERE Name = 'WixUI_Bmp_Banner'")
$view.Execute()
$rec = $view.Fetch()
if ($rec) {
    $rec.SetStream(2, $bannerBmp)
    $view.Modify(2, $rec)
    Write-Host "Updated WixUI_Bmp_Banner in $MsiPath"
}
$view.Close()

$db.Commit()
[System.Runtime.InteropServices.Marshal]::ReleaseComObject($db) | Out-Null
[System.Runtime.InteropServices.Marshal]::ReleaseComObject($wi) | Out-Null


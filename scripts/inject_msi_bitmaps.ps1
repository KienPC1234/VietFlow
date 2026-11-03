param(
    [string]$MsiPath
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $MsiPath)) {
    Write-Warning "File not found: $MsiPath"
    exit 1
}

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$dialogBmp = Join-Path $root "assets\wix_dialog.bmp"
$bannerBmp = Join-Path $root "assets\wix_banner.bmp"

$wi = $null
$db = $null

try {
    $wi = New-Object -ComObject WindowsInstaller.Installer
    $db = $wi.OpenDatabase($MsiPath, 1) # 1 = msiOpenDatabaseModeTransact

    # 1. ActionText table: prevents blank "Status:" text during standard install actions
    $sqlCreate = "CREATE TABLE ``ActionText`` (``Action`` CHAR(72) NOT NULL, ``Description`` CHAR(64) LOCALIZABLE, ``Template`` CHAR(128) LOCALIZABLE PRIMARY KEY ``Action``)"
    try {
        $view = $db.OpenView($sqlCreate)
        $view.Execute()
        $view.Close()
        [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
    } catch {}

    $actions = @(
        @("InstallValidate", "Validating install", ""),
        @("InstallInitialize", "Initializing installation...", ""),
        @("ProcessComponents", "Updating component registration...", ""),
        @("InstallFiles", "Copying new files...", "File: [1], Directory: [9], Size: [6]"),
        @("CreateShortcuts", "Creating shortcuts...", "Shortcut: [1]"),
        @("WriteRegistryValues", "Writing system registry values...", "Key: [1], Name: [2], Value: [3]"),
        @("RemoveFiles", "Removing files...", "File: [1], Directory: [9]"),
        @("InstallFinalize", "Finalizing installation...", "")
    )
    foreach ($act in $actions) {
        try {
            $sql = "INSERT INTO ``ActionText`` (``Action``, ``Description``, ``Template``) VALUES ('$($act[0])', '$($act[1])', '$($act[2])')"
            $view = $db.OpenView($sql)
            $view.Execute()
            $view.Close()
            [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
        } catch {}
    }

    # 2. Inject VietFlow branded bitmaps (Dialog background & Top banner) if present
    if (Test-Path $dialogBmp) {
        try {
            $view = $db.OpenView("SELECT Name, Data FROM Binary WHERE Name = 'WixUI_Bmp_Dialog'")
            $view.Execute()
            $rec = $view.Fetch()
            if ($rec) {
                $rec.SetStream(2, $dialogBmp)
                $view.Modify(2, $rec)
                [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($rec) | Out-Null
            }
            $view.Close()
            [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
        } catch {}
    }
    if (Test-Path $bannerBmp) {
        try {
            $view = $db.OpenView("SELECT Name, Data FROM Binary WHERE Name = 'WixUI_Bmp_Banner'")
            $view.Execute()
            $rec = $view.Fetch()
            if ($rec) {
                $rec.SetStream(2, $bannerBmp)
                $view.Modify(2, $rec)
                [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($rec) | Out-Null
            }
            $view.Close()
            [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
        } catch {}
    }

    $db.Commit()
    Write-Host "Successfully optimized ActionText for MSI package: $MsiPath" -ForegroundColor Green
} catch {
    Write-Warning "Failed to optimize MSI package $($MsiPath): $_"
} finally {
    if ($db) { [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($db) | Out-Null }
    if ($wi) { [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($wi) | Out-Null }
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}

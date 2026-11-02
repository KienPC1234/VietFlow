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

    # 1. ActionText table
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

    # 2. Fix Shortcut table Description field
    try {
        $sqlShortcut = "UPDATE ``Shortcut`` SET ``Description`` = 'VietFlow - Modern Vietnamese Input Method'"
        $view = $db.OpenView($sqlShortcut)
        $view.Execute()
        $view.Close()
        [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
    } catch {}

    # 3. Standard Property metadata
    $props = @{
        "MsiStartMenuShortcutDescription" = "VietFlow - Modern Vietnamese Input Method"
        "MsiDesktopShortcutDescription"   = "VietFlow - Modern Vietnamese Input Method"
        "ARPCOMMENTS"                     = "VietFlow - Modern Vietnamese Input Method"
        "ARPCONTACT"                      = "VietFlow Team"
        "ARPHELPLINK"                     = "https://github.com/KienPC1234/VietFlow"
        "ARPURLINFOABOUT"                 = "https://github.com/KienPC1234/VietFlow"
        "ARPURLUPDATEINFO"                = "https://github.com/KienPC1234/VietFlow/releases"
    }
    foreach ($k in $props.Keys) {
        $val = $props[$k]
        try {
            $view = $db.OpenView("INSERT INTO ``Property`` (``Property``, ``Value``) VALUES ('$k', '$val')")
            $view.Execute()
            $view.Close()
            [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
        } catch {
            try {
                $view = $db.OpenView("UPDATE ``Property`` SET ``Value`` = '$val' WHERE ``Property`` = '$k'")
                $view.Execute()
                $view.Close()
                [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
            } catch {}
        }
    }

    # 4. Fix RemoveFile table: Purge desktop & start menu shortcuts and all residual app files on uninstall
    $removeFiles = @(
        @("CleanUpDesktopShortcut", "ApplicationDesktopShortcut", "VietFlow.lnk", "DesktopFolder", 2),
        @("CleanUpStartMenuShortcut", "ApplicationStartMenuRootShortcut", "VietFlow.lnk", "ProgramMenuFolder", 2),
        @("CleanUpAllAppFolderFiles", "ApplicationDesktopShortcut", "*.*", "INSTALLFOLDER", 2)
    )
    foreach ($rf in $removeFiles) {
        try {
            $view = $db.OpenView("INSERT INTO ``RemoveFile`` (``FileKey``, ``Component_``, ``FileName``, ``DirProperty``, ``InstallMode``) VALUES ('$($rf[0])', '$($rf[1])', '$($rf[2])', '$($rf[3])', $($rf[4]))")
            $view.Execute()
            $view.Close()
            [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
        } catch {
            try {
                $view = $db.OpenView("UPDATE ``RemoveFile`` SET ``Component_`` = '$($rf[1])', ``FileName`` = '$($rf[2])', ``DirProperty`` = '$($rf[3])', ``InstallMode`` = $($rf[4]) WHERE ``FileKey`` = '$($rf[0])'")
                $view.Execute()
                $view.Close()
                [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
            } catch {}
        }
    }

    # 5. Elevate UninstallHookDeferred so it has full rights to remove HKLM uninstaller entries
    try {
        $view = $db.OpenView("UPDATE ``CustomAction`` SET ``Type`` = 3137 WHERE ``Action`` = 'UninstallHookDeferred'")
        $view.Execute()
        $view.Close()
        [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) | Out-Null
    } catch {}

    # 6. Inject VietFlow branded bitmaps (Dialog background & Top banner)
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
    Write-Host "Successfully optimized MSI package and branding: $MsiPath" -ForegroundColor Green
} catch {
    Write-Warning "Failed to optimize MSI package $($MsiPath): $_"
} finally {
    if ($db) { [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($db) | Out-Null }
    if ($wi) { [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($wi) | Out-Null }
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}

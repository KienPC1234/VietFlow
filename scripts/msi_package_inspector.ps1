# Read-only Windows Installer structured MSI verification. No installation.
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

function Assert-MsiPackage([string]$file, [string]$arch, [string]$version) {
    Assert-MsiContainer $file
    $expectedTemplate = switch ($arch) {
        'x86' { 'Intel;1033' }
        'arm64' { 'Arm64;1033' }
        default { 'x64;1033' }
    }
    $expectedUpgrade = '{B9A5320F-D409-4BFB-A63A-9E3DA4D3A1F8}'
    try {
        $installer = New-Object -ComObject WindowsInstaller.Installer
        $db = $installer.OpenDatabase($file, 0) # strictly read-only
        $summary = $db.SummaryInformation(0)
        $template = [string]$summary.Property(7)
        if ($template -ine $expectedTemplate) {
            throw "MSI template architecture/language mismatch ($template vs $expectedTemplate)"
        }
        $properties = @{}
        $bt = [char]96
        foreach ($name in @('ProductVersion','ProductName','ProductCode',
                             'UpgradeCode','ProductLanguage','ALLUSERS')) {
            $query = "SELECT " + $bt + "Value" + $bt + " FROM " +
                     $bt + "Property" + $bt + " WHERE " + $bt +
                     "Property" + $bt + "='" + $name + "'"
            $view = $db.OpenView($query)
            try {
                $view.Execute()
                $record = $view.Fetch()
                if ($null -eq $record) { throw "MSI missing Property.$name" }
                $properties[$name] = [string]$record.StringData(1)
            } finally {
                $record = $null
                $view.Close()
                $view = $null
            }
        }
        if ($properties['ProductVersion'] -cne $version -or
            $properties['ProductName'] -cne 'VietFlow' -or
            $properties['UpgradeCode'] -ine $expectedUpgrade -or
            $properties['ProductLanguage'] -ne '1033' -or
            $properties['ALLUSERS'] -ne '1') {
            throw 'MSI version, product name, upgrade identity, language or installation scope differs from verified release'
        }
        $productGuid = [guid]::Empty
        if (-not [guid]::TryParse($properties['ProductCode'], [ref]$productGuid) -or
            $productGuid -eq [guid]::Empty) {
            throw 'MSI ProductCode is not a valid unique product GUID'
        }
    } catch {
        throw "Invalid or inconsistent Windows Installer database for $arch ($file): $_"
    } finally {
        $record = $null
        $view = $null
        $summary = $null
        $db = $null
        $installer = $null
    }
}

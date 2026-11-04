# Shared, read-only selection of release inputs for the MSI and portable ZIP.
# Never fall back to an earlier compiler output or a different architecture.
function Get-VietFlowReleaseInputs {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][ValidateSet('x64','x86','arm64')][string]$Arch,
        [Parameter(Mandatory)][string]$DistDir
    )
    if (-not (Test-Path -LiteralPath $DistDir -PathType Container)) {
        throw "Validated dist folder is missing: $DistDir"
    }
    $source = (Resolve-Path -LiteralPath $DistDir).Path
    $dllName = switch ($Arch) {
        'x86' { 'vietflow_tsf_x86.dll' }
        'arm64' { 'vietflow_tsf_arm64.dll' }
        default { 'vietflow_tsf.dll' }
    }
    $paths = [ordered]@{
        ExePath = Join-Path $source "VietFlow-$Arch.exe"
        DllPath = Join-Path $source $dllName
    }
    $expectedMachine = switch ($Arch) {
        'x86' { 0x014c }
        'arm64' { 0xaa64 }
        default { 0x8664 }
    }
    foreach ($entry in $paths.GetEnumerator()) {
        $file = $entry.Value
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            throw "Canonical release input missing: $file"
        }
        $f = [IO.File]::OpenRead($file)
        try {
            $r = [IO.BinaryReader]::new($f, [Text.Encoding]::ASCII, $true)
            try {
                if ($f.Length -lt 70 -or $r.ReadUInt16() -ne 0x5a4d) {
                    throw "Invalid PE DOS header: $file"
                }
                $f.Position = 0x3c
                $pe = $r.ReadInt32()
                if ($pe -lt 64 -or $pe -gt ($f.Length - 6)) {
                    throw "Invalid PE offset: $file"
                }
                $f.Position = $pe
                if ($r.ReadUInt32() -ne 0x00004550) {
                    throw "Invalid PE signature: $file"
                }
                $machine = [int]$r.ReadUInt16()
                if ($machine -ne $expectedMachine) {
                    throw "Wrong PE architecture for $Arch input: $file"
                }
            } finally { $r.Dispose() }
        } finally { $f.Dispose() }
    }
    return [pscustomobject]@{
        Arch = $Arch
        ExePath = $paths.ExePath
        DllPath = $paths.DllPath
        ExeSha256 = (Get-FileHash -LiteralPath $paths.ExePath -Algorithm SHA256).Hash.ToLowerInvariant()
        DllSha256 = (Get-FileHash -LiteralPath $paths.DllPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

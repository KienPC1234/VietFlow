# Isolated release-input source selection tests. No builds, MSI installs,
# registry changes, Windows hooks, network or user data access.
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
. (Join-Path $repo 'scripts/release_asset_sources.ps1')
$msiBuilder = Join-Path $repo 'installer/wix/build_msi.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ('vf-source-test-' + [guid]::NewGuid().ToString('N'))
$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Path $dist -Force | Out-Null

function Make-Pe([string]$arch) {
    $machines = @{x64=0x8664;x86=0x014c;arm64=0xaa64}
    $data = [byte[]]::new(128)
    $data[0] = 0x4d; $data[1] = 0x5a
    [BitConverter]::GetBytes([int]64).CopyTo($data, 0x3c)
    $data[64] = 0x50; $data[65] = 0x45
    [BitConverter]::GetBytes([uint16]$machines[$arch]).CopyTo($data, 68)
    return ,$data
}
function Check-Reject([scriptblock]$action,[string]$name) {
    $rejected = $false
    try { & $action | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw "Expected safe rejection: $name" }
    Write-Host "PASS: $name"
}
function Write-Pe([string]$path,[string]$arch) {
    [IO.File]::WriteAllBytes($path, (Make-Pe $arch))
}
try {
    foreach($arch in @('x64','x86','arm64')) {
        $dll = switch($arch) {
            'x86' {'vietflow_tsf_x86.dll'}
            'arm64' {'vietflow_tsf_arm64.dll'}
            default {'vietflow_tsf.dll'}
        }
        Write-Pe (Join-Path $dist "VietFlow-$arch.exe") $arch
        Write-Pe (Join-Path $dist $dll) $arch
    }
    # A tempting but unverified alias must never be selected over canonical
    # architecture-qualified executables.
    Write-Pe (Join-Path $dist 'VietFlow.exe') 'x86'
    foreach($arch in @('x64','x86','arm64')) {
        $found = Get-VietFlowReleaseInputs -Arch $arch -DistDir $dist
        if ($found.Arch -ne $arch -or
            $found.ExePath -ne (Join-Path $dist "VietFlow-$arch.exe") -or
            $found.ExeSha256.Length -ne 64 -or $found.DllSha256.Length -ne 64) {
            throw "Invalid canonical input returned for $arch"
        }
    }
    Write-Host 'PASS: all 3 canonical EXE/DLL pairs and no x64 alias fallback'
    # Prove the actual MSI entry point calls this exact helper and returns
    # before installing WiX or writing an MSI.
    $output = Join-Path $root 'no-build-output'
    & $msiBuilder -Arch All -DistDir $dist -Version '0.1.4' -OutputDir $output -ValidateSourcesOnly | Out-Null
    if (Test-Path $output) { throw 'Validation-only builder created a release output' }
    Write-Host 'PASS: MSI builder validates all architectures without packaging'

    $exeX86 = Join-Path $dist 'VietFlow-x86.exe'
    Write-Pe $exeX86 'x64'
    Check-Reject { Get-VietFlowReleaseInputs -Arch x86 -DistDir $dist } 'wrong PE machine is rejected'
    Check-Reject { & $msiBuilder -Arch x86 -DistDir $dist -ValidateSourcesOnly } 'MSI builder rejects wrong architecture'
    Write-Pe $exeX86 'x86'

    $dll = Join-Path $dist 'vietflow_tsf.dll'
    Remove-Item -LiteralPath $dll
    # Fake stale build artifacts must not become fallback release inputs.
    $oldBuild = Join-Path $root 'platforms/windows_tsf/build/Release'
    New-Item -ItemType Directory -Path $oldBuild -Force | Out-Null
    Write-Pe (Join-Path $oldBuild 'vietflow_tsf.dll') 'x64'
    Check-Reject { Get-VietFlowReleaseInputs -Arch x64 -DistDir $dist } 'missing canonical DLL never falls back to stale build'
    Check-Reject { & $msiBuilder -Arch x64 -DistDir $dist -ValidateSourcesOnly } 'MSI builder blocks missing canonical DLL'
    Write-Pe $dll 'x64'

    $exe = Join-Path $dist 'VietFlow-arm64.exe'
    [IO.File]::WriteAllBytes($exe, [byte[]](0x4d,0x5a))
    Check-Reject { Get-VietFlowReleaseInputs -Arch arm64 -DistDir $dist } 'truncated PE rejected'
    Write-Pe $exe 'arm64'
    $contents = [IO.File]::ReadAllBytes($exe)
    [BitConverter]::GetBytes([int]-1).CopyTo($contents,0x3c)
    [IO.File]::WriteAllBytes($exe,$contents)
    Check-Reject { Get-VietFlowReleaseInputs -Arch arm64 -DistDir $dist } 'out-of-range PE offset rejected'
    Write-Pe $exe 'arm64'

    $absent = Join-Path $root 'empty-dist'
    Check-Reject { Get-VietFlowReleaseInputs -Arch x64 -DistDir $absent } 'missing dist rejected'
    Write-Host 'PASS: canonical MSI/ZIP release source test suite'
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}

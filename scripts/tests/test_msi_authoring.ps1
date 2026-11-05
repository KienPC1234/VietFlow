# Offline authoring/policy regression only: never run msiexec, regsvr32,
# custom actions, Windows hooks or modify installed TSF/shortcuts.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$wxs = Join-Path $root 'installer/wix/VietFlow.wxs'
$exe = Join-Path $root 'platforms/windows/src/main.rs'
$tsf = Join-Path $root 'platforms/windows/src/tsf.rs'

function Assert-True([bool]$value, [string]$message) {
    if (-not $value) { throw "MSI policy failure: $message" }
}
function Verify-MsiPolicy([xml]$doc) {
    $ns = [xml.XmlNamespaceManager]::new($doc.NameTable)
    $ns.AddNamespace('w', 'http://wixtoolset.org/schemas/v4/wxs')
    $ns.AddNamespace('util', 'http://wixtoolset.org/schemas/v4/wxs/util')
    $pkg = $doc.SelectSingleNode('/w:Wix/w:Package', $ns)
    Assert-True ($null -ne $pkg -and $pkg.GetAttribute('Scope') -eq 'perMachine') 'per-machine package'
    $major = $doc.SelectSingleNode('//w:MajorUpgrade',$ns)
    Assert-True ($null -ne $major -and $major.GetAttribute('AllowSameVersionUpgrades') -ne 'yes') 'same-version major replacement must be disabled'
    $close = $doc.SelectNodes('//util:CloseApplication',$ns)
    Assert-True ($close.Count -eq 1) 'exactly one close rule'
    $c = $close[0]
    Assert-True ($c.GetAttribute('Target') -eq 'VietFlow.exe') 'only VietFlow is closed'
    Assert-True ($c.GetAttribute('CloseMessage') -eq 'yes') 'graceful close message'
    Assert-True (-not $c.HasAttribute('TerminateProcess')) 'installer may not force kill active text composition'
    Assert-True ($c.GetAttribute('PromptToContinue') -eq 'yes') 'user is able to abort/retry when app running'
    Assert-True ($c.GetAttribute('RebootPrompt') -eq 'yes') 'locked application may be handled via reboot'
    Assert-True ($c.GetAttribute('Timeout') -ge 10) 'closing timeout avoids a rushed shutdown'
    Assert-True (-not [string]::IsNullOrWhiteSpace($c.GetAttribute('Description'))) 'user-facing close prompt'

    $actions = $doc.SelectNodes('//w:CustomAction',$ns)
    $unreg = @($actions | Where-Object { $_.GetAttribute('Id') -eq 'UnregisterTSF' })
    Assert-True ($unreg.Count -eq 1) 'one TSF custom action'
    Assert-True ($unreg[0].GetAttribute('Execute') -eq 'deferred') 'unregister requires deferred action'
    Assert-True ($unreg[0].GetAttribute('Impersonate') -eq 'no') 'unregister uses machine identity'
    Assert-True ($unreg[0].GetAttribute('ExeCommand').Contains('--msi-final-unregister-tsf')) 'safe installer-only TSF command'
    Assert-True (-not $unreg[0].GetAttribute('ExeCommand').Contains('--unregister-tsf"')) 'never execute interactive unregister CLI'
    foreach ($action in $actions) {
        Assert-True (-not $action.GetAttribute('ExeCommand').Contains('--clean-shortcuts')) 'no SYSTEM-account shortcut deletion'
        Assert-True (-not $action.GetAttribute('ExeCommand').Contains('--uninstall-cleanup')) 'no recursive shared data deletion'
    }
    $sequences = @($doc.SelectNodes('//w:InstallExecuteSequence/w:Custom',$ns))
    Assert-True ($sequences.Count -eq 1) 'no extra destructive execute actions'
    $seq = $sequences[0]
    Assert-True ($seq.GetAttribute('Action') -eq 'UnregisterTSF') 'only TSF cleanup in execute sequence'
    Assert-True ($seq.GetAttribute('Before') -eq 'RemoveFiles') 'unregister before deleting TSF DLL'
    Assert-True ($seq.GetAttribute('Condition') -eq 'Installed AND REMOVE="ALL" AND NOT UPGRADINGPRODUCTCODE') 'only final uninstall, never upgrade/repair'
    Assert-True ($doc.SelectNodes('//w:RemoveFolder',$ns).Count -eq 0) 'no user data directory removals'
    Assert-True ($doc.SelectNodes('//w:RemoveFile',$ns).Count -eq 0) 'MSI manages owned shortcuts; forbid extra name-based deletions of replaced user links'
    $appData = $doc.SelectSingleNode('//w:Directory[@Id="CommonAppDataVietFlow"]',$ns)
    Assert-True ($null -ne $appData) 'persistent ProgramData directory is preserved'
    $shortcutComponents = @($doc.SelectNodes('//w:Component/w:Shortcut',$ns))
    Assert-True ($shortcutComponents.Count -eq 2) 'MSI owns exactly two shortcuts'
}

[xml]$document = Get-Content -LiteralPath $wxs -Raw -Encoding UTF8
Verify-MsiPolicy $document
Write-Host 'PASS MSI: safe final uninstall, non-force close, no shortcut or config data deletion'

$main = Get-Content $exe -Raw -Encoding UTF8
$tsfSource = Get-Content $tsf -Raw -Encoding UTF8
$start = $main.IndexOf('if args.iter().any(|a| a == "--msi-final-unregister-tsf")')
$end = $main.IndexOf('if args.iter().any(|a| a == "--unregister-tsf")', $start)
Assert-True ($start -gt 0 -and $end -gt $start) 'installer CLI branch exists before interactive branch'
$branch = $main.Substring($start,$end-$start)
Assert-True ($branch.Contains('unregister_tsf_from_msi_final_uninstall()')) 'installer branch calls the non-user path'
Assert-True (-not $branch.Contains('AppConfig::load')) 'installer does not load or rewrite user config'
Assert-True (-not $branch.Contains('clean_all_shortcuts_and_cache')) 'installer never deletes user shortcuts'
$start = $tsfSource.IndexOf('pub fn unregister_tsf_from_msi_final_uninstall()')
$end = $tsfSource.IndexOf('pub fn unregister_tsf()',$start)
Assert-True ($start -gt 0 -and $end -gt $start) 'non-user TSF cleanup function exists'
$impl = $tsfSource.Substring($start,$end-$start)
Assert-True ($impl.Contains('regsvr32.exe') -and $impl.Contains('/u /s')) 'installer performs native COM unregistration'
foreach($forbidden in @('delete_hkcu_tree(', 'call_install_layout_or_tip(', 'AppConfig', 'config.save(', 'remove_dir_all(')) {
    Assert-True (-not $impl.Contains($forbidden)) "installer TSF handler must not call $forbidden"
}
Write-Host 'PASS MSI: SYSTEM-route code avoids per-user layout and configuration writes'

# Fault-inject bad authoring in memory to prove our guard would fail CI.
$bad = [xml]$document.OuterXml
$ns = [xml.XmlNamespaceManager]::new($bad.NameTable)
$ns.AddNamespace('u','http://wixtoolset.org/schemas/v4/wxs/util')
$bad.SelectSingleNode('//u:CloseApplication',$ns).SetAttribute('TerminateProcess','1')
$blocked = $false
try { Verify-MsiPolicy $bad } catch { $blocked = $true }
Assert-True $blocked 'regression test must catch future forced termination'
Write-Host 'PASS MSI: regression detects reintroduced forced termination'

$bad = [xml]$document.OuterXml
$ns = [xml.XmlNamespaceManager]::new($bad.NameTable)
$ns.AddNamespace('w','http://wixtoolset.org/schemas/v4/wxs')
$bad.SelectSingleNode('//w:InstallExecuteSequence/w:Custom',$ns).SetAttribute(
    'Condition','Installed AND REMOVE="ALL"')
$blocked = $false
try { Verify-MsiPolicy $bad } catch { $blocked = $true }
Assert-True $blocked 'regression test must catch unconditional upgrade cleanup'
Write-Host 'PASS MSI: regression detects missing upgrade protection'

$bad = [xml]$document.OuterXml
$ns = [xml.XmlNamespaceManager]::new($bad.NameTable)
$ns.AddNamespace('w','http://wixtoolset.org/schemas/v4/wxs')
$bad.SelectSingleNode('//w:MajorUpgrade',$ns).SetAttribute('AllowSameVersionUpgrades','yes')
$blocked = $false
try { Verify-MsiPolicy $bad } catch { $blocked = $true }
Assert-True $blocked 'regression must detect unsafe same-version major upgrades'
Write-Host 'PASS MSI: regression detects unsafe same-version major upgrades'

# An MSI-managed shortcut has its own component lifecycle. A separate
# RemoveFile-by-name can also delete a replacement .lnk the user created.
# Mutation-testing this prohibition prevents wildcard and exact-name cleanup.
$bad = [xml]$document.OuterXml
$ns = [xml.XmlNamespaceManager]::new($bad.NameTable)
$ns.AddNamespace('w','http://wixtoolset.org/schemas/v4/wxs')
$component = $bad.SelectSingleNode('//w:Component[@Id="DesktopShortcutComponent"]',$ns)
Assert-True ($null -ne $component) 'test fixture has desktop shortcut component'
$extra = $bad.CreateElement('RemoveFile', 'http://wixtoolset.org/schemas/v4/wxs')
$extra.SetAttribute('Id', 'DangerousNameBasedDelete')
$extra.SetAttribute('Name', 'VietFlow.lnk')
$extra.SetAttribute('On', 'uninstall')
[void]$component.AppendChild($extra)
$blocked = $false
try { Verify-MsiPolicy $bad } catch { $blocked = $true }
Assert-True $blocked 'policy must reject name-based deletion of user-replaced shortcuts'
Write-Host 'PASS MSI: regression detects extra name-based shortcut deletion'
Write-Host 'PASS: isolated MSI authoring safety suite'

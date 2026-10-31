# VietFlow Runtime Memory Audit

Measured on Windows 11 x64 with a temporary `%APPDATA%` and a default config.
The settings UI was opened explicitly; the daemon ran in the background.

## Results

| Process | Working set | Private bytes | Handles | Threads |
| --- | ---: | ---: | ---: | ---: |
| x64 daemon (idle) | 16.4 MB | 2.9 MB | 157 | 4 |
| x64 settings UI | 40.7 MB | 14.2 MB after 10 s | 286 | 6 |
| x86 daemon (idle) | 17.6 MB | 2.8 MB | 181 | 4 |
| x86 settings UI | 36.4 MB | 9.5 MB | 306 | 6 |

The daemon is the always-on part. Its private memory is under 3 MB. The UI is
a separate process and only runs while the settings or converter window is
open. Working set includes shared DLL/code pages, so private bytes are the
better number for "how much RAM does it add".

## Optimizations applied

- Tray icons are no longer rebuilt when icon-related config fields did not
  change. This avoids two glyph renders on every save.
- Tray glyphs are rasterized from the embedded JetBrains Mono TTF through GDI
  `GetGlyphOutline`, with no GDI+ startup or GDI+ image objects.
- The settings UI releases its D3D11 device, swap chain and render target while
  minimized, then recreates them on restore.

Measured UI minimization on x64:

| UI state | Working set | Private bytes |
| --- | ---: | ---: |
| Normal | 40.9 MB | 17.7 MB |
| Minimized | 30.8 MB | 11.2 MB |
| Restored | 41.5 MB | 14.6 MB |

The minimized UI saves about 10 MB working set / 6.5 MB private bytes. The
window restores and renders normally.

## Fix applied

Tray icons are rebuilt on every config save. The old `TrayIcon::set_icons`
replaced the handles without destroying them, leaking two GDI objects per
reload. The fallback `LoadIconW` handle was also shared and must not be passed
to `DestroyIcon`.

- `TrayIcon::set_icons` now destroys the previous private handles.
- The system fallback icon is copied with `CopyIcon` so ownership is clear.
- V/E glyphs now come from a 1.4 KB embedded JetBrains Mono subset containing
  only those two outlines, rasterized at 8x through GDI `GetGlyphOutline`;
  GDI+ is no longer initialized or linked.

Verification: 20 config saves through the live reload timer, and 200 direct
V/E icon renders. GDI objects stayed at 4 before and after the render loop.

| Build | GDI objects before | GDI objects after |
| --- | ---: | ---: |
| Before fix | 12 | 132 |
| After fix | 12 | 12 |

## Remaining optimization candidates

1. **Font atlas oversampling.** `OversampleH = 2` improves high-DPI text but
   enlarges the atlas. Dropping to 1 at 150-200% is a small, visual-tradeoff
   optimization.
2. **Destroy the ImGui context while minimized.** The D3D11 device is released
   now, but the ImGui context and font atlas stay resident. Rebuilding the
   context on every restore would be slower and risk losing UI state, so it is
   not recommended for the current workflow.
3. **No daemon-side action needed.** The engine hot path is allocation-free for
   normal keys; foreground process lookup and config parsing only run on focus
   or file changes.

## How to reproduce

```powershell
# Start daemon with a temporary config, then read the process counters.
$env:APPDATA = "$env:TEMP\vf-mem"
New-Item -ItemType Directory -Force "$env:APPDATA\VietFlow" | Out-Null
'{"enabled":true}' | Set-Content "$env:APPDATA\VietFlow\vietflow_config.json"
$d = Start-Process .\dist\VietFlow.exe -PassThru
Start-Sleep 4
Get-Process -Id $d.Id | Select-Object WorkingSet64,PrivateMemorySize64,Handles
```

Use `GetGuiResources(OpenProcess(...), 0)` to count GDI objects; the standard
`Handles` counter does not include them.

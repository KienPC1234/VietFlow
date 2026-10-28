# VietFlow (.NET 10)

Ultra-high performance, pure C# Vietnamese input engine and unified Windows desktop application built on **.NET 10**.

Re-engineered from Pham Kim Long's canonical UniKey input automaton with modern low-level .NET 10 primitives (`[InlineArray]`, `FrozenDictionary`, `SearchValues`, atomic `SendInput` batching).

Licensed under the **GNU General Public License v3.0 (GPLv3)**.

---

## Highlights & Killer Features

| Feature | Description |
| :--- | :--- |
| **Zero-Allocation Engine** | Steady-state typing performs zero heap allocations (span-based lookups, packed vowel/coda tables), so the keyboard hook never adds GC pressure. |
| **UniKey-Equivalent Linguistics** | Ports UniKey's `VSeqList`, `VCPairList`, `isValidCV`/`isValidVC`, `qu`/`gi` consonant handling and stop-coda tone rules, so `quơ`, `thuở`, `giường`, `tuần`, `xoắn` behave exactly like the original engine. |
| **Atomic `SendInput` Batching** | Packages backspaces and replacement Unicode characters into a single system call array, eliminating the duplicate character bug (`dđược`, `tthành`) on Chrome/Edge address bars and VS Code/Rider autocomplete. |
| **Shift-Lag / CapsLock Slip Correction** | Automatically demotes the second capitalized character to lowercase when the user types quickly (e.g., `VIệt` -> `Việt`, `TRường` -> `Trường`). |
| **Consonant Shortcuts** | Fast consonant expansions for rapid typing: initial `f` -> `ph`, `j` -> `gi`, `w` -> `qu`. |
| **Dynamic Multi-Line Macros** | Supports snippets with dynamic runtime variables: `@date` (current date), `@time` (current time), and `@clipboard` (current clipboard content). |
| **Smart App-Switching / Game Mode** | Automatically detects active foreground processes (e.g., `valorant.exe`, `cs2.exe`, `dota2.exe`, `code.exe`) and bypasses the keyboard hook or locks English mode for zero-latency gaming. |
| **Clipboard Toolkit** | Global hotkey `Ctrl+Shift+F9` to convert character sets or strip Vietnamese diacritics directly within the clipboard. |
| **Silent Admin Logon** | Windows Task Scheduler integration (`RunLevel.Highest`) to automatically launch at Windows startup with administrator privileges without triggering UAC prompts. |
| **Single Config File** | Unified configuration stored in `%APPDATA%\VietFlow\vietflow_config.json`. |

---

## Supported Methods & Character Sets

VietFlow eliminates obsolete encodings (BK HCM, Vietware, VISCII, VPS, NCR) and focuses on the four industry standards:

### Input Methods
- **Telex**: Standard Vietnamese Telex (`aa`, `ee`, `oo`, `dd`, `aw`, `ow`, `uw`, tone marks `s`, `f`, `r`, `x`, `j`, bracket shortcuts `[ ] { }`).
- **Simple Telex**: Gaming/coding-friendly Telex where lone `w` remains `w` (only modifying preceding vowels).
- **VNI**: Standard numeric diacritics (`1`–`9`, `0`).
- **VIQR**: `^ + ( ' \` ? ~ .`
- **Microsoft Vietnamese**: Native MS Vietnamese keyboard layout (`1`=ă, `2`=â, `3`=ê, `4`=ô, `0`=đ, `5`–`9` tones).

### Character Sets
- **Unicode (NFC)**: Precomposed UTF-16 (standard default).
- **Unicode Composite (NFD)**: Decomposed Unicode.
- **TCVN3 (ABC)**: Legacy single-byte encoding (`.VnTime`).
- **VNI-Windows**: Legacy double-byte encoding (`VNI-Times`).

---

## Global Hotkeys

| Hotkey | Action |
| :--- | :--- |
| `Ctrl + Shift` or `Alt + Z` | Toggle Vietnamese / English mode |
| `Ctrl + Shift + F5` | Open VietFlow Settings Window |
| `Ctrl + Shift + F9` | Quick Clipboard Toolkit (convert / strip accents) |

---

## Solution Architecture

```
lib-csharp/
├── src/
│   ├── VietFlow/                 # VietFlow.Core (.NET 10 cross-platform library, Native AOT ready)
│   │   ├── Engine/               # Automaton, word buffer, tone rules, macro table
│   │   ├── Charsets/             # NFC, NFD, TCVN3, VNI-Windows
│   │   ├── Configuration/        # VietFlowConfig (System.Text.Json source generation)
│   │   └── Tools/                # ClipboardConverter (charset conversion & strip accents)
│   └── VietFlow.Windows/         # VietFlow.exe (WPF Windows 11 Fluent settings, Win32 hook, tray)
│       └── Assets/               # App icons, PNG badges, VietFlow.ico
│           └── Icons/            # Standalone SVG vector icons
├── tests/
│   └── VietFlow.Tests/           # xUnit test suite (292 tests passing, 100% green)
├── samples/
│   └── VietFlow.Demo/            # Benchmarks (10M+ keystrokes/sec)
└── scripts/
    └── convert_svg.js            # Node.js script using @resvg/resvg-js and png-to-ico
```

---

## Building & Running

### Prerequisites
- [.NET 10 SDK](https://dotnet.microsoft.com/download)

### Run Unit Tests
```bash
dotnet test lib-csharp/tests/VietFlow.Tests/VietFlow.Tests.csproj
```

### Build Desktop Application (`VietFlow.exe`)
```bash
dotnet build lib-csharp/src/VietFlow.Windows/VietFlow.Windows.csproj
```
The output executable is created at:
`lib-csharp/src/VietFlow.Windows/bin/Debug/net10.0-windows/VietFlow.exe`

### Publish Self-Contained Single File
```bash
dotnet publish lib-csharp/src/VietFlow.Windows/VietFlow.Windows.csproj -c Release -r win-x64 --self-contained -p:PublishSingleFile=true
```

---

## License

GNU General Public License v3.0 (GPLv3). See [LICENSE](LICENSE) for details.

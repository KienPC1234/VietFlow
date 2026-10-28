# VietFlow `0.1.0-alpha`

Vietnamese input engine + Windows tray app, written in pure C# (.NET 10).

## Build

```bash
dotnet build src/VietFlow.Windows/VietFlow.Windows.csproj -c Release
dotnet test tests/VietFlow.Tests/VietFlow.Tests.csproj
```

Output: `src/VietFlow.Windows/bin/Release/net10.0-windows/VietFlow.exe`

## Layout

- `src/VietFlow` — cross-platform engine (`net10.0`, no Windows APIs).
- `src/VietFlow.Windows` — WPF shell, Win32 keyboard hook, tray menu.
- `tests/VietFlow.Tests` — UniKey parity and regression tests.

## Porting

Only the input/output adapters need replacing:

- Engine entry points: `VietFlowEngine.ProcessKey(char, Span<char>, out int)` and `ProcessUtf8`.
- Data tables: `Tables/CharsetTables*`, `Engine/ToneRules`.
- `VietFlow.Core` contains no P/Invoke, registry access or `System.Drawing`.

## License

GPLv3. Based on UniKey by Pham Kim Long.

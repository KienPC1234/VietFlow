# VietFlow

High-performance Vietnamese input engine for Windows, written in C# on .NET 10.
Source: [https://github.com/KienPC1234/VietFlow](https://github.com/KienPC1234/VietFlow)

## Build & test

```bash
dotnet build src/VietFlow.Windows/VietFlow.Windows.csproj -c Release
dotnet test tests/VietFlow.Tests/VietFlow.Tests.csproj
```

## Publish

Publish a single-file, trimmed, self-contained Windows build (about 23 MB), then package it with Velopack:

```bash
dotnet publish src/VietFlow.Windows/VietFlow.Windows.csproj -c Release -r win-x64 \
  --self-contained -p:PublishSingleFile=true -o dist/publish

vpk pack -u VietFlow -v 0.1.0 -p dist/publish -e VietFlow.exe -r win-x64 --msi \
  --packTitle "VietFlow" --packAuthors "KienPC1234" \
  --icon src/VietFlow.Windows/Assets/VietFlow.ico -o dist/releases
```

## Project layout

* `src/VietFlow` — cross-platform core engine (`net10.0`, no Win32 dependency).
* `src/VietFlow.Windows` — Avalonia UI, `WH_KEYBOARD_LL` hook, tray integration.
* `tests/VietFlow.Tests` — engine and UniKey parity tests.

## License

GNU General Public License v3.0 (GPLv3).
Core algorithms derived from UniKey by Phạm Kim Long.

# TODO

Tracked work. Do not put TODO comments in source files.

## Features

- [ ] Per-app special cases: app -> Vietnamese on/off, input method, charset, mode.
- [ ] Show-settings-on-startup option and admin-startup option (UAC-free task).
- [ ] Auto-exclude foreground apps / auto game mode.
- [ ] Update checker (GitHub releases) and "Check for updates" UI.
- [ ] "Standard key send" and "Disable Vietnamese on non-US keyboard layout".
- [ ] Correct tone-bug modes (Go 1 / Go 2).
- [ ] Custom charset: import/export mapping (JSON), preview sample text.
- [ ] Macro import/export (CSV/JSON) and per-app macros.
- [ ] Localization for the newer labels (Advanced, theme, tray dialog, About note).

## Engine

- [ ] Make keyword list + custom charset injectable per session (multi-session hosts).
- [ ] Add a porting sample: console/Avalonia adapter using `ProcessKey`/`ProcessUtf8`.
- [ ] Multi-target `net9.0` for hosts that cannot move to .NET 10 yet.
- [ ] In-repo benchmark harness (currently ad-hoc, measured with the Demo).
- [ ] Fuzz tests for `ProcessKey` (random key streams, backspace, undo, charsets).

## UI / polish

- [ ] Dark scrollbar + dark WinForms tray menu renderer.
- [ ] Resize/min-size behavior for the settings window at 125%/150% DPI.
- [ ] Screenshot/QA mode cleanup (`--shot` is developer-only).
- [ ] Replace default `DataGrid` chrome with the compact theme style.

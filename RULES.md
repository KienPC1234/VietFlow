# Engineering rules

Short, enforceable rules for VietFlow. Keep the engine fast, portable and boring.

## Architecture

- `src/VietFlow` (`VietFlow.Core`) is **platform-neutral**: no P/Invoke, registry, `System.Drawing`, `System.Windows` or Win32 types.
- Platform code lives in `src/VietFlow.Windows` (Avalonia shell, `WH_KEYBOARD_LL` hook, `SendInput`, tray).
- **Do not add TSF / IME / TIP components.** VietFlow deliberately uses a low-level keyboard hook + `SendInput`; TSF adds complexity and breaks in games/legacy apps.

## Commits

- **Batch changes.** Commit only when a coherent feature, fix or refactor is complete and green.
- Never commit per-file, per-typo or WIP. One logical change = one commit.
- Conventional messages: `feat:`, `fix:`, `refactor:`, `perf:`, `docs:`, `test:`, `chore:`.
- Before a commit: Release build with **0 warnings**, all tests green, no generated files (`bin/`, `obj/`, `*.user`).

## Code

- No `TODO`, `FIXME`, `XXX` or commented-out code in source. Track work in `TODO.md`.
- Nullable enabled. Avoid `!` unless the invariant is obvious and local.
- No new dependencies in the engine. Windows-only packages stay in `VietFlow.Windows`.
- Public engine APIs stay span-based and allocation-free.
- Prefer `ReadOnlySpan<char>` / `stackalloc` / packed integer keys over strings and LINQ in hot paths.
- Engine state must stay per-instance; only documented snapshots may be global (`EnglishPhonetics` keywords, `CustomCharset` mappings), updated on config apply.

## Engine hot path

`ProcessKey(char, Span<char>, out int)` must not allocate or call Win32.
Keep incremental updates (`ValidateIncremental`, `_vowelStartIndex`, `_fastVowelKey`) instead of rescanning the buffer.
Any tone/charset fix must be covered by a test in `tests/VietFlow.Tests` (UniKey parity when possible).

## UI

- Colors only through theme resources (`{DynamicResource Brush...}`); no hardcoded hex in windows.
- Every card description needs `TextTrimming` or `TextWrapping`; no overlapping text.
- Verify both **Light** and **Dark** themes after UI changes.
- New visible strings go through `Localization` when practical.
- Keep `README.md` English and minimal.

## Versioning

- Current line: `0.1.0-alpha` (`<InformationalVersion>`).
- Bump `<Version>` / `<InformationalVersion>` only for user-visible milestones.

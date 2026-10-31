# VietFlow Runtime Validation Checklist

This is the manual test pass required before the next public release. Unit tests
cover the engine and config parsing; they do not prove that Windows apps accept
the injected keys.

## Automated pre-checks (2026-10-31)

- [x] `cargo test -p vietflow-core`: 40 tests pass.
- [x] `cargo test -p vietflow-windows`: 28 tests pass, 1 network test ignored.
- [ ] `cargo test --workspace`: blocked by the TSF crate's pre-existing
      `windows-core` dependency mismatch (TSF workstream, not changed here).
- [x] x64 Release build passes locally.
- [x] GitHub Actions x64, x86 and ARM64 jobs pass.
- [x] ARM64 artifact downloaded; PE header reports `Arm64` / PE32+.
- [x] `dumpbin /dependents` shows no `VCRUNTIME140` or `MSVCP140`.
- [x] `VietFlow.exe --settings` and `VietFlow.exe --convert` start and stay alive.
- [x] Daemon starts with a temporary config containing `lock_hotkeys_in_excluded_apps=false` and an app profile.
- [x] Tray glyphs use a 1,196-byte Montserrat Bold subset containing only `V`
      and `E`; `GetTextFaceW` resolves the embedded family as `Montserrat`.
- [x] Glyph masks are crisp at 16 px; no bitmap scaling or GDI+ path is used.
- [x] GDI/USER objects stay stable over 20 live icon-colour reloads with the
      border-disabled path (GDI 12 -> 12, USER 15 -> 15).
- [x] Startup registry verified: enabling writes the quoted executable path;
      disabling removes the value. The test restored the original registry state.
- [x] Minimizing the settings UI releases D3D11 resources and restores normally
      (40.9 MB / 17.7 MB -> 30.8 MB / 11.2 MB minimized).
- [ ] Interactive typing and game/app matrix below.

## Environment

- [ ] Windows 10 22H2 or Windows 11 current.
- [ ] x64 build from `.\scripts\build.ps1 -Arch x64`.
- [ ] x86 build from `.\scripts\build.ps1 -Arch x86 -SkipTests` (test on 64-bit
      Windows with the x86 binary).
- [ ] ARM64 build from the `windows-11-vs2026-arm` CI artifact.
- [ ] Static CRT: `dumpbin /dependents dist\VietFlow.exe` must not list
      `VCRUNTIME140.dll` or `MSVCP140.dll`.
- [ ] Close any other Vietnamese IME before testing.

## 1. Basic typing

- [ ] Telex: `Tieengs Vieetj` -> `Tiếng Việt`.
- [ ] VNI: `Tie6ng1 Vie6t5` -> `Tiếng Việt`.
- [ ] VIQR and Simple Telex presets.
- [ ] Tone undo: `ass` -> `as`; bracket undo: `[[` -> `[`.
- [ ] English restore: `code` stays `code`; `mẹ` does not become `me`.
- [ ] Custom English keyword popup: add, remove and restore a default.
- [ ] Custom auto-capitalize characters: set `。` and verify the next word is
      capitalized after it.

## 2. Hotkey lock and exclusion

- [ ] Add `notepad.exe` to exclusions.
- [ ] With the checkbox on, focus Notepad: V/E, settings and converter hotkeys
      do nothing.
- [ ] Tray icon still toggles the language.
- [ ] First tray click in Notepad enables typing; second click returns to the
      excluded state.
- [ ] Click the tray from a focused game (Windows moves focus to the shell),
      return to the game: the temporary Vietnamese mode is still active.
- [ ] Switch from that game to a browser: the temporary override resets.
- [ ] Uncheck the hotkey-lock checkbox; V/E, settings and converter work again.
- [ ] Press V/E in the excluded app with the lock off: first press enables typing
      through the override, second returns to excluded.
- [ ] Restart VietFlow and confirm the checkbox state persisted.

## 3. Per-app profiles

- [ ] Add a profile for `Code.exe`: Auto or Safe mode, macros on.
- [ ] Add a profile for a game: scan-code or safe mode, macros off.
- [ ] Add a profile for `chrome.exe`: Auto or Safe mode.
- [ ] Load an old config with `injection_mode: 3`; it becomes Auto.
- [ ] Switch apps and confirm the active profile changes without restarting.
- [ ] Delete a profile and confirm the global settings apply again.
- [ ] Type in the VietFlow settings window itself; it must not be excluded.

## 4. Injection modes

For each mode, test Notepad, Chrome/Edge, VS Code, Telegram Web and one
DirectInput game.

- [ ] Auto: normal typing; if `SendInput` is rejected, paced Unicode fallback
      runs; the clipboard is never changed automatically.
- [ ] Unicode: normal typing.
- [ ] Scan-code: ASCII keys work; Vietnamese characters are not dropped.
- [ ] Safe: slower output, no missing first character.
- [ ] Remote desktop / VM: test Auto, Unicode and Safe modes.
- [ ] Confirm the injection dropdown has no Clipboard entry.
- [ ] No duplicated first character in Telegram Web.
- [x] No missing characters in VS Code Markdown (user test passed).
- [ ] Open an ANSI-only window such as the VBA IDE: Vietnamese typing is disabled
      by default; unchecking the new option restores the old behavior.

## 5. Macros

- [ ] `vn` + Space -> `Việt Nam` in a normal app.
- [ ] Fresh config: macro is OFF by default; `vn` + Space stays `vn `.
- [ ] Enable macro explicitly; `vn` + Space -> `Việt Nam`.
- [ ] Macro while English mode is active with "allow macro when disabled" on:
      only the macro expands; Vietnamese typing stays off.
- [ ] Macro in an excluded app with the profile macro flag on.
- [ ] Macro variables: `@date`, `@time`, `@datetime`, `@clipboard`, `@tab`,
      `@newline`.
- [ ] Large macro replacement (> 64 characters).
- [ ] Import TSV: `key<TAB>value`.
- [ ] Import `key=value`, `key||value`, `key,value` and `key: value`.
- [ ] Import Unicode replacement text.
- [ ] Export and re-import; no duplicate or lost entries.

## 6. Converter

- [ ] Unicode -> TCVN3: copy from a Unicode source; paste into a legacy TCVN3 app.
- [ ] TCVN3 -> Unicode: copy raw CF_TEXT; convert; paste into a Unicode app.
- [ ] VNI -> Unicode and Unicode -> VNI.
- [ ] Unicode Composite roundtrip.
- [ ] Strip accents: `Đường hoa Nguyễn Huệ` -> `Duong hoa Nguyen Hue`.
- [ ] Uppercase/lowercase conversion preserves Vietnamese diacritics.
- [ ] Empty input and clipboard unavailable show an error, not a false success.
- [ ] Clipboard with only CF_UNICODETEXT and source TCVN3/VNI shows the clear
      "no CF_TEXT" error.

## 7. Admin relaunch

- [ ] Tray -> "Chạy lại với quyền quản trị"; accept UAC. A single daemon starts,
      the old one exits, and typing still works.
- [ ] Cancel UAC. The old daemon keeps running and an error is shown.
- [ ] Launch a second normal instance: it opens the settings instead of starting a
      second hook.
- [ ] Elevated daemon can type into an elevated Notepad; verify the hook is active.

## 8. UI and packaging

- [ ] 100%, 150% and 200% DPI; no clipped tabs or buttons.
- [ ] Minimum window 760×560; popups stay inside the window.
- [ ] Long macro and exclusion lists scroll inside the page.
- [x] Tray V/E glyphs render from the two-glyph Montserrat Bold subset; no
      missing-glyph fallback.
- [ ] Tray icons for V/E and custom colors.
- [ ] Sound preview and V/E notification.
- [x] Notification sound uses the embedded 2.1 KB / 0.16 s IMA ADPCM soft chime
      first, then `%SystemRoot%\Media`/alias/beep fallbacks. One serialized
      worker per process; only one UI/daemon playback per V/E toggle. Listening
      check still pending on the target machine.
- [x] "Kiểm tra bản cập nhật" opens the update dialog above the settings window,
      not behind it. Verified on 2026-10-30: window class `#32770`, `topmost=True`,
      `foreground=True` after posting `IDM_CHECK_UPDATE` from the settings UI.
      The tray menu uses the same updater path.
- [ ] `dist\latest.json` has the correct version, notes and downloads.
- [ ] Velopack installers start the app and preserve `%APPDATA%\VietFlow`.
- [ ] Uninstall leaves the user config intact.

## Known risks to watch

1. **TCVN3 uppercase** may lose upper/lower distinction because the legacy table
   uses one byte for both. Test uppercase words explicitly.
2. **Auto mode** falls back to paced Unicode only when `SendInput` reports
   failure and never changes the clipboard. Apps that silently swallow Unicode
   remain unsupported until the TSF spike proves an alternative.
3. **Admin relaunch** uses a retrying single-instance mutex. Test UAC accept and
   cancel paths on a slow machine.
4. **Concurrent work**: QR/donation files may be present in the working tree. Do
   not include them in the validation build unless that feature is also being
   reviewed.

## Sign-off

- Tester:
- Build / commit:
- Date:
- Result: PASS / FAIL
- Issues found:

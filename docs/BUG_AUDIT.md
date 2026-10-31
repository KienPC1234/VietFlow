# Bug audit: core engine and ImGui UI

Date: 2026-10-31

Scope: `core/`, `ui_imgui/src/`, and the Windows/TSF integration touched by the
feature audit.

## Fixed in this pass

### Core

1. **NFD stripping left combining marks**
   - `strip_accents("Tie\u{0302}\u{0301}ng")` returned the same combining
     sequence instead of `Tieng`.
   - `core/src/converter.rs` now removes Vietnamese combining marks
     (`U+0300`, `U+0301`, `U+0302`, `U+0303`, `U+0306`, `U+0309`, `U+031B`,
     `U+0323`, canonical variants).
   - Added `test_strip_accents_handles_nfd`.

2. **Unicode Composite (NFD) conversion was only partial**
   - Input only recognized a tone mark directly after a precomposed base.
     Full NFD such as `e + U+0302 + U+0301` fell through as raw characters, so
     converting NFD to TCVN3/VNI did not work.
   - `compose_composite_index()` now consumes modifier and tone marks in either
     order.
   - `encode_composite_index()` now emits true NFD (plain base + modifier +
     tone) instead of precomposed base + tone.
   - `EncodedChar` widened from two to four characters for cases such as
     `ơ` + tone.
   - Added `test_convert_full_nfd_composite` (both directions).

3. **C ABI option setters changed state without resetting the word**
   - `vf_engine_set_spell_check`, `vf_engine_set_modern_style` and
     `vf_engine_set_auto_restore` now call `engine.reset()`, matching the
     method/charset setters.

4. **`@clipboard` was advertised but not implemented**
   - The core now emits a private clipboard placeholder; the Windows host
     resolves it from a clipboard cache on the daemon message loop, and the TSF
     TIP resolves it in-process. The low-level keyboard hook never opens the
     clipboard.

5. **Macro keys were case-insensitive at runtime but not in the UI map**
   - New and imported keys are lowercased before insertion, and existing config
     entries are normalized on load, so the editor cannot show duplicate `VN`
     and `vn` entries that resolve to the same trigger.

### UI

1. **All per-app profile combos shared one ImGui ID**
   - In `PageExclusions`, table rows did not push an ID scope, so every row's
     `##PMethod`, `##PCharset`, `##PInject` and `##PMacro` combo had the same ID.
   - Each row now pushes `PushID(name)` before its widgets and pops after the
     delete button.

2. **Tray color pickers used stale static state**
   - `PageAdvanced` kept `static ImVec4` color values initialized from the first
     configuration. `ResetToDefaults` or a reload could show/save old colors.
   - The colors are now per-frame locals rebuilt from `config_`.

3. **One-byte buffer overrun in path/title UTF-8 conversion**
   - `WideCharToMultiByte(..., -1, ...)` returns a length that includes the
     terminating NUL, but `PickMacroFile` and `GetRunningApps` allocated
     `len - 1` bytes and then asked Windows to write `len` bytes.
   - Both now allocate `len` bytes, convert, then resize to `len - 1`.

4. **`std::tolower` was called on negative `char` values**
   - Macro filtering and running-app/title filtering invoked `::tolower` on
     UTF-8 bytes, which is undefined behavior for non-ASCII Vietnamese text.
   - Added `LowerAsciiInPlace()` using `unsigned char`; all filtered strings use
     it now.

5. **Malformed config could index ImGui combo arrays out of bounds**
   - `Config::Load` now clamps `method`, `charset`, `typing_timeout_ms` and
     per-profile overrides.
   - Profile combo indices are also clamped defensively in `PageExclusions`.

6. **D3D render-target creation was unchecked**
   - `CreateRenderTarget()` now checks `GetBuffer` and
     `CreateRenderTargetView`; device creation and resize abort cleanly on
     failure instead of rendering to a null render target.

## Open issues / decisions

1. **`vf_engine_process_key` may split UTF-8 when `max_out_len` is too small**
   - The header documents "up to max_out_len bytes", but a caller can receive an
     invalid truncated UTF-8 sequence. Callers should query/allocate, or the ABI
     should copy only complete characters and report the required size.

## Verification

- `cargo test -p vietflow-core`: 42 passed.
- `cargo test -p vietflow-windows`: 31 passed, 1 network test ignored.
- `cargo test -p vietflow-tsf`: 2 passed.
- `cmake --build ui_imgui/build --config Release` (x64): passed.
- `cmake --build ui_imgui/build-x86 --config Release` (x86): passed.

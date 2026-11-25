/*
 * VietFlow Core - Stable C ABI (Rust <-> C/C++/Swift)
 * Licensed under GNU General Public License v3.0 (GPLv3)
 *
 * This header is the only contract between the Rust engine and platform/UI
 * code. Keep it small: never pass Qt, std::string or language-specific types
 * across this boundary.
 */

#ifndef VIETFLOW_CORE_H
#define VIETFLOW_CORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Input methods (matches vietflow_core::InputMethod). */
enum VfInputMethod {
    VF_METHOD_TELEX = 0,
    VF_METHOD_VNI = 1,
    VF_METHOD_VIQR = 2,
    VF_METHOD_SIMPLE_TELEX = 3,
    VF_METHOD_TELEX_SIMPLE2 = 4,
    VF_METHOD_TELEX_VNI = 5
};

/* Charsets (matches vietflow_core::CharsetId). */
enum VfCharset {
    VF_CHARSET_UNICODE = 0,
    VF_CHARSET_UNICODE_COMPOSITE = 1,
    VF_CHARSET_TCVN3 = 2,
    VF_CHARSET_VNI_WINDOWS = 3,
    VF_CHARSET_VIQR = 4
};

/* Unified IME actions (matches vietflow_core::ImeAction) for Fcitx5 / TSF / IMK. */
enum VfImeAction {
    VF_IME_PASSTHROUGH = 0,
    VF_IME_UPDATE_PREEDIT = 1,
    VF_IME_COMMIT_CONSUME = 2,
    VF_IME_COMMIT_FORWARD = 3
};

typedef struct VfEngine VfEngine;

/*
 * Owned UTF-8 text: no truncation or NUL terminator. Return values are valid
 * until released exactly once via vf_owned_utf8_free. Empty = {NULL, 0}.
 * A failed call leaves result empty. Never reuse a populated result without
 * freeing it. The returned pointer/length must be passed unchanged to free.
 */
typedef struct VfOwnedUtf8 {
    uint8_t *data;
    size_t len;
} VfOwnedUtf8;
void vf_owned_utf8_free(VfOwnedUtf8 value);

/* These methods mutate the engine at most once. Unlike the legacy borrowed
 * APIs, no buffer-capacity guess or retry of a mutating key is necessary. */
bool vf_engine_process_key_owned(VfEngine *engine, uint32_t ch,
                                 VfOwnedUtf8 *result, size_t *out_backspaces);
bool vf_engine_get_preedit_owned(VfEngine *engine, VfOwnedUtf8 *result);
bool vf_engine_finish_on_break_owned(VfEngine *engine, uint32_t break_char,
                                     VfOwnedUtf8 *result);
bool vf_engine_process_backspace_owned(VfEngine *engine, VfOwnedUtf8 *result);
uint8_t vf_engine_process_ime_key_owned(VfEngine *engine, uint32_t ch,
                                        uint64_t now_ms, VfOwnedUtf8 *result);

VfEngine *vf_engine_create(void);
void vf_engine_destroy(VfEngine *engine);
void vf_engine_reset(VfEngine *engine);
bool vf_engine_is_empty(const VfEngine *engine);
int8_t vf_engine_take_auto_switch_request(VfEngine *engine);

void vf_engine_set_method(VfEngine *engine, uint8_t method);
void vf_engine_set_charset(VfEngine *engine, uint8_t charset);
void vf_engine_set_spell_check(VfEngine *engine, bool enabled);
void vf_engine_set_modern_style(VfEngine *engine, bool enabled);
void vf_engine_set_auto_restore(VfEngine *engine, bool enabled);
void vf_engine_set_free_marking(VfEngine *engine, bool enabled);
void vf_engine_set_enable_macro(VfEngine *engine, bool enabled);
void vf_engine_set_auto_switch_by_typing(VfEngine *engine, bool enabled);
void vf_engine_set_fix_shift_lag(VfEngine *engine, bool enabled);
void vf_engine_set_typing_timeout_ms(VfEngine *engine, uint64_t timeout_ms);
void vf_engine_set_can_rewrite_previous_word(VfEngine *engine, bool enabled);
void vf_engine_clear_macros(VfEngine *engine);
void vf_engine_add_macro(VfEngine *engine, const uint8_t *trigger_utf8,
                         size_t trigger_len, const uint8_t *replacement_utf8,
                         size_t replacement_len);
void vf_engine_set_foreign_initials(VfEngine *engine, const uint8_t *chars_utf8,
                                    size_t len);
void vf_engine_set_custom_english_keywords(VfEngine *engine,
                                           const uint8_t *newline_sep_utf8,
                                           size_t len);
void vf_engine_set_removed_english_keywords(VfEngine *engine,
                                            const uint8_t *newline_sep_utf8,
                                            size_t len);

/*
 * Legacy borrowed UTF-8 ABI: out_len is the FULL required byte count on
 * every valid call. Null output or zero capacity performs a size query, but
 * mutating operations MUST NOT be repeated to retrieve data. If the supplied
 * buffer is too small, returns false and writes NO output bytes. Otherwise
 * writes the full text, without a NUL terminator. Prefer owned methods.
 *
 * In-line TSF composition preedit helpers.
 */
bool vf_engine_get_preedit(VfEngine *engine, uint8_t *out_utf8, size_t max_out_len, size_t *out_len);
bool vf_engine_finish_on_break(VfEngine *engine, uint32_t break_char, uint8_t *out_utf8, size_t max_out_len, size_t *out_len);
bool vf_engine_process_backspace(VfEngine *engine, uint8_t *out_utf8, size_t max_out_len, size_t *out_len);

/*
 * Process one key. `out_utf8` receives up to `max_out_len` UTF-8 bytes.
 * Returns false on invalid arguments or insufficient capacity; on every
 * valid call out_len reports FULL required bytes and out_backspaces the number
 * of backspaces. No text bytes are written if capacity is insufficient.
 */
bool vf_engine_process_key(VfEngine *engine, uint32_t ch,
                           uint8_t *out_utf8, size_t max_out_len,
                           size_t *out_len, size_t *out_backspaces);

/*
 * Default English keyword list, newline-separated UTF-8. Same size-query
 * contract as vf_english_keywords: call with out = NULL to get the size, then
 * call again with a buffer.
 */
size_t vf_english_keywords(uint8_t *out, size_t out_cap);

#ifdef __cplusplus
}
#endif

#endif /* VIETFLOW_CORE_H */

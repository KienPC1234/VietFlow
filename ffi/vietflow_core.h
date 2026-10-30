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
    VF_METHOD_SIMPLE_TELEX = 1,
    VF_METHOD_VNI = 2,
    VF_METHOD_VIQR = 3,
    VF_METHOD_USER_CUSTOM = 6
};

/* Charsets (matches vietflow_core::CharsetId). */
enum VfCharset {
    VF_CHARSET_UNICODE = 0,
    VF_CHARSET_UNICODE_COMPOSITE = 1,
    VF_CHARSET_TCVN3 = 2,
    VF_CHARSET_VNI_WINDOWS = 3,
    VF_CHARSET_VIQR = 4
};

typedef struct VfEngine VfEngine;

VfEngine *vf_engine_create(void);
void vf_engine_destroy(VfEngine *engine);
void vf_engine_reset(VfEngine *engine);

void vf_engine_set_method(VfEngine *engine, uint8_t method);
void vf_engine_set_charset(VfEngine *engine, uint8_t charset);
void vf_engine_set_spell_check(VfEngine *engine, bool enabled);
void vf_engine_set_modern_style(VfEngine *engine, bool enabled);
void vf_engine_set_auto_restore(VfEngine *engine, bool enabled);

/*
 * Process one key. `out_utf8` receives up to `max_out_len` UTF-8 bytes.
 * Returns false on invalid arguments; otherwise `out_len` receives the number
 * of bytes written and `out_backspaces` the number of backspaces to send first.
 */
bool vf_engine_process_key(VfEngine *engine, uint32_t ch,
                           uint8_t *out_utf8, size_t max_out_len,
                           size_t *out_len, size_t *out_backspaces);

/*
 * Charset conversion helpers.
 *
 * Both functions return the number of bytes required for the result. Pass
 * `out = NULL` to query the size, then call again with a buffer of at least
 * that size. At most `out_cap` bytes are written.
 */
size_t vf_convert_charset(const uint8_t *input, size_t input_len,
                          uint8_t source, uint8_t target,
                          uint8_t *out, size_t out_cap);

size_t vf_strip_accents(const uint8_t *input, size_t input_len,
                        uint8_t *out, size_t out_cap);

/*
 * Raw-byte charset conversion. Use this for TCVN3/VNI Windows clipboard or file
 * content, where the input/output is not UTF-8. Same size-query contract.
 */
size_t vf_convert_charset_bytes(const uint8_t *input, size_t input_len,
                                uint8_t source, uint8_t target,
                                uint8_t *out, size_t out_cap);

/*
 * Default English keyword list, newline-separated UTF-8. Same size-query
 * contract as vf_convert_charset: call with out = NULL to get the size, then
 * call again with a buffer.
 */
size_t vf_english_keywords(uint8_t *out, size_t out_cap);

#ifdef __cplusplus
}
#endif

#endif /* VIETFLOW_CORE_H */

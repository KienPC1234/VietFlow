// Isolated C++/Rust UTF-8 ABI smoke. No TSF registration, hooks or UI.
#include "vietflow_core.h"
#include <windows.h>
#include <cstdint>
#include <iostream>
#include <string>

static int Fail(const char* why) {
    std::cerr << "F04 owned UTF-8 smoke FAIL: " << why << '\n';
    return 1;
}

static std::string Take(VfOwnedUtf8& owned) {
    std::string result;
    if (owned.data && owned.len) {
        result.assign(reinterpret_cast<const char*>(owned.data), owned.len);
    }
    vf_owned_utf8_free(owned);
    owned = {};
    return result;
}

static bool ValidUtf8(const std::string& text) {
    if (text.empty()) return true;
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                               static_cast<int>(text.size()), nullptr, 0) > 0;
}

int main() {
    VfEngine* engine = vf_engine_create();
    if (!engine) return Fail("engine allocation");
    vf_engine_set_enable_macro(engine, true);
    int status = 0;
    for (const char ch : std::string("chxhcnvn")) {
        VfOwnedUtf8 owned = {};
        size_t backspaces = 0;
        if (!vf_engine_process_key_owned(engine, static_cast<uint32_t>(ch),
                                         &owned, &backspaces)) {
            status = Fail("owned key processing"); break;
        }
        const std::string keyText = Take(owned);
        if (!ValidUtf8(keyText)) { status = Fail("key invalid UTF-8"); break; }
    }
    if (!status) {
        VfOwnedUtf8 result = {};
        if (!vf_engine_finish_on_break_owned(engine, static_cast<uint32_t>(' '), &result))
            status = Fail("owned finalization");
        else {
            const std::string text = Take(result);
            if (text.size() < 20 || !ValidUtf8(text))
                status = Fail("Vietnamese macro expansion missing/invalid");
        }
    }
    if (!status) {
        VfOwnedUtf8 before = {};
        if (!vf_engine_get_preedit_owned(engine, &before)) status = Fail("owned preedit");
        else if (!ValidUtf8(Take(before))) status = Fail("preedit invalid UTF-8");
    }
    if (!status) {
        VfOwnedUtf8 after = {};
        if (!vf_engine_process_backspace_owned(engine, &after)) status = Fail("owned backspace");
        else if (!ValidUtf8(Take(after))) status = Fail("backspace invalid UTF-8");
    }
    if (!status) {
        const size_t needed = vf_english_keywords(nullptr, 0);
        uint8_t tiny[4] = {0xa5, 0xa5, 0xa5, 0xa5};
        if (needed <= sizeof(tiny) || vf_english_keywords(tiny, sizeof(tiny)) != needed ||
            tiny[0] != 0xa5 || tiny[3] != 0xa5)
            status = Fail("keyword buffer partial write");
    }
    vf_engine_destroy(engine);
    if (!status) std::cout << "F04 owned UTF-8 C++/Rust ABI smoke PASS\n";
    return status;
}

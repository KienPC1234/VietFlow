// VietFlow TSF: interpret characters in the focused thread's keyboard layout.
// Unlike hard-coded US VK tables, this preserves AZERTY/QWERTZ and Shift digits.
#include "keyboard_layout.h"
#include <initializer_list>

wchar_t VfTsfKeyToAsciiWithLayout(UINT vk, UINT scan, const BYTE* state, HKL layout) {
    if (vk >= 256 || !state || !layout) return 0;
    // Shortcut and AltGr (Ctrl+Alt) keys belong to the host application.
    for (UINT modifier : {VK_CONTROL, VK_LCONTROL, VK_RCONTROL,
                          VK_MENU, VK_LMENU, VK_RMENU, VK_LWIN, VK_RWIN}) {
        if (state[modifier] & 0x80) return 0;
    }
    if (scan == 0) scan = MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC, layout);
    if (scan == 0) return 0;

    wchar_t buffer[8] = {};
    // Bit 2: never modify the kernel dead-key buffer (Win10 1607+).
    constexpr UINT NO_KEYBOARD_STATE_MUTATION = 0x04;
    const int count = ToUnicodeEx(vk, scan, state, buffer, 8,
                                  NO_KEYBOARD_STATE_MUTATION, layout);
    // A dead key, ligature, surrogate pair or non-ASCII code point should be
    // handled by Windows, not simplified or partially swallowed by the TIP.
    if (count != 1 || buffer[0] < L' ' || buffer[0] > L'~') return 0;
    return buffer[0];
}

wchar_t VfTsfKeyToAscii(UINT vk, LPARAM keyData) {
    if (vk >= 256) return 0;
    BYTE state[256] = {};
    if (!GetKeyboardState(state)) return 0;
    const UINT scan = (static_cast<UINT_PTR>(keyData) >> 16) & 0xFF;
    return VfTsfKeyToAsciiWithLayout(vk, scan, state, GetKeyboardLayout(0));
}

bool VfTsfIsCompositionChar(wchar_t ch, uint32_t method) {
    if ((ch >= L'a' && ch <= L'z') || (ch >= L'A' && ch <= L'Z'))
        return true;
    if (ch >= L'0' && ch <= L'9')
        return method == 1 || method == 5 || method == 6; // VNI
    if (ch == L'[' || ch == L']')
        return method == 0 || method == 6; // Telex
    if (method == 2 || method == 6) { // VIQR marks by CHAR, not physical VK
        switch (ch) {
            case L'\'': case L'`': case L'?': case L'.': case L'+':
            case L'^': case L'(': case L'~': case L'=': case L'/':
                return true;
        }
    }
    return false;
}

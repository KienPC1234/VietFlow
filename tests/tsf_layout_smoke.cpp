// Isolated TSF keyboard-layout regression: no COM/TIP registration, no
// keyboard hooks, no user layout activation or unloading.
#include "keyboard_layout.h"
#include <windows.h>
#include <iostream>
#include <vector>

static bool Check(bool condition, const char* msg) {
    if (!condition) std::cerr << "TSF layout FAIL: " << msg << "\n";
    return condition;
}

int main() {
    bool ok = true;
    // Composition decisions follow the decoded CHAR, never a hard-coded US VK.
    ok &= Check(VfTsfIsCompositionChar(L'a', 0), "Telex lowercase letter");
    ok &= Check(VfTsfIsCompositionChar(L'Z', 0), "Telex uppercase letter");
    ok &= Check(VfTsfIsCompositionChar(L'[', 0), "Telex bracket");
    ok &= Check(!VfTsfIsCompositionChar(L'&', 1), "VNI punctuation not a digit");
    ok &= Check(VfTsfIsCompositionChar(L'7', 1), "VNI translated digit");
    ok &= Check(!VfTsfIsCompositionChar(L'7', 0), "Telex digit pass-through");
    ok &= Check(VfTsfIsCompositionChar(L'?', 2), "VIQR question mark");
    ok &= Check(VfTsfIsCompositionChar(L'^', 2), "VIQR circumflex");
    ok &= Check(!VfTsfIsCompositionChar(L'[', 2), "VIQR bracket pass-through");
    ok &= Check(!VfTsfIsCompositionChar(0, 6), "dead-key rejected");
    ok &= Check(!VfTsfIsCompositionChar(L'\u00e9', 6), "non-ASCII native");
    ok &= Check(!VfTsfIsCompositionChar(L' ', 6), "space separate");

    std::vector<HKL> layouts;
    const int count = GetKeyboardLayoutList(0, nullptr);
    if (count > 0 && count <= 64) {
        layouts.resize(count);
        const int actual = GetKeyboardLayoutList(count, layouts.data());
        if (actual > 0 && actual <= count) layouts.resize(actual);
        else layouts.clear();
    }
    if (layouts.empty()) layouts.push_back(GetKeyboardLayout(0));

    // Explicitly test every already-loaded layout WITHOUT activating it.
    // Deterministic synthetic Shift/AltGr state avoids changing real HKCU
    // keyboard preferences or affecting the desktop input language.
    BYTE plain[256] = {};
    BYTE shifted[256] = {};
    shifted[VK_SHIFT] = 0x80;
    shifted[VK_LSHIFT] = 0x80;
    BYTE altgr[256] = {};
    altgr[VK_MENU] = 0x80;
    altgr[VK_RMENU] = 0x80;
    altgr[VK_CONTROL] = 0x80;
    for (HKL layout : layouts) {
        if (!layout) continue;
        const wchar_t lower = VfTsfKeyToAsciiWithLayout('A', 0, plain, layout);
        const wchar_t upper = VfTsfKeyToAsciiWithLayout('A', 0, shifted, layout);
        ok &= Check(lower == VfTsfKeyToAsciiWithLayout('A', 0, plain, layout),
                    "repeated non-mutating lookup");
        ok &= Check(upper == VfTsfKeyToAsciiWithLayout('A', 0, shifted, layout),
                    "repeated Shift lookup");
        if (lower && upper) {
            ok &= Check(lower >= L' ' && lower <= L'~', "ASCII-only output");
            ok &= Check(upper >= L' ' && upper <= L'~', "ASCII-only Shift");
        }
        ok &= Check(VfTsfKeyToAsciiWithLayout('A', 0, altgr, layout) == 0,
                    "AltGr stays native");
        ok &= Check(VfTsfKeyToAsciiWithLayout(VK_SHIFT, 0, plain, layout) == 0,
                    "Shift not printable");
        ok &= Check(VfTsfKeyToAsciiWithLayout(0x10000u, 0, plain, layout) == 0,
                    "out-of-range VK");
    }
    if (ok) std::cout << "TSF layout smoke PASS: " << layouts.size()
                      << " loaded layout(s), non-mutating Shift/AltGr translation\n";
    return ok ? 0 : 1;
}

// VietFlow TSF keyboard-layout-aware, non-mutating ASCII translation.
// Windows 10 1607+/11. Never steal dead-key, AltGr or multi-unit text.
#pragma once
#include <windows.h>
#include <cstdint>

// Return 0 if Windows should handle the key natively.
wchar_t VfTsfKeyToAscii(UINT vk, LPARAM keyData);
// Exposed for isolated native layout tests. Does not activate/change the HKL.
wchar_t VfTsfKeyToAsciiWithLayout(UINT vk, UINT scan, const BYTE* keyState, HKL layout);
bool VfTsfIsCompositionChar(wchar_t ch, uint32_t method);

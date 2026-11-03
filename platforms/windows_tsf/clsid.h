// VietFlow Windows TSF - GUIDs & Constants
// Licensed under GNU General Public License v3.0 (GPLv3)

#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <initguid.h>

// Fresh unique GUIDs for VietFlow TSF
// {02B8B9EC-2D3E-42B6-97C0-26043A29BFCB}
DEFINE_GUID(CLSID_VietFlowTip,
    0x02b8b9ec, 0x2d3e, 0x42b6, 0x97, 0xc0, 0x26, 0x04, 0x3a, 0x29, 0xbf, 0xcb);

// {FBA81B7C-5EAD-454F-9F81-413780E9A0BA}
DEFINE_GUID(GUID_VietFlowProfile,
    0xfba81b7c, 0x5ead, 0x454f, 0x9f, 0x81, 0x41, 0x37, 0x80, 0xe9, 0xa0, 0xba);

// {5868879F-7B13-490F-8445-B4A73DC4A681}
DEFINE_GUID(GUID_VietFlowDisplayAttr,
    0x5868879f, 0x7b13, 0x490f, 0x84, 0x45, 0xb4, 0xa7, 0x3d, 0xc4, 0xa6, 0x81);

// Legacy GUIDs kept for clean unregistration and registry removal
// {7C2B5E10-9A31-4B8F-B623-192E48270101}
DEFINE_GUID(LEGACY_CLSID_VietFlowTip,
    0x7c2b5e10, 0x9a31, 0x4b8f, 0xb6, 0x23, 0x19, 0x2e, 0x48, 0x27, 0x01, 0x01);

// {7C2B5E11-9A31-4B8F-B623-192E48270102}
DEFINE_GUID(LEGACY_GUID_VietFlowProfile,
    0x7c2b5e11, 0x9a31, 0x4b8f, 0xb6, 0x23, 0x19, 0x2e, 0x48, 0x27, 0x01, 0x02);

constexpr const wchar_t* c_szClsidStr = L"{02B8B9EC-2D3E-42B6-97C0-26043A29BFCB}";
constexpr const wchar_t* c_szProfileStr = L"{FBA81B7C-5EAD-454F-9F81-413780E9A0BA}";

constexpr const wchar_t* c_szLegacyClsidStr = L"{7C2B5E10-9A31-4B8F-B623-192E48270101}";
constexpr const wchar_t* c_szLegacyProfileStr = L"{7C2B5E11-9A31-4B8F-B623-192E48270102}";

constexpr const wchar_t* c_szTextServiceName = L"VietFlow TSF Keyboard";
constexpr const wchar_t* c_szTextServiceProfileName = L"VietFlow TSF";

constexpr LANGID c_langidEnglishUS = 0x0409;
constexpr LANGID c_langidVietnamese = 0x042A;


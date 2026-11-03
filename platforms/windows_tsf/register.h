// VietFlow Windows TSF - COM & TSF Profile Registration
// Licensed under GNU General Public License v3.0 (GPLv3)

#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

HRESULT RegisterServer(HMODULE hModule);
HRESULT UnregisterServer();


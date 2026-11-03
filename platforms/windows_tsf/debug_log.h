// VietFlow Windows TSF - Diagnostic Logging Subsystem
// Licensed under GNU General Public License v3.0 (GPLv3)

#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

// Diagnostic logging disabled for clean, high-performance production build.
#define VF_LOG_ENABLED 0

#if VF_LOG_ENABLED
void VF_LogMessage(const char* function, const wchar_t* format, ...);
#define VF_LOG(fmt, ...) VF_LogMessage(__FUNCTION__, fmt, ##__VA_ARGS__)
#else
#define VF_LOG(fmt, ...) ((void)0)
#endif


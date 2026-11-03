// VietFlow Windows TSF - Diagnostic Logging Subsystem
// Licensed under GNU General Public License v3.0 (GPLv3)

#include "debug_log.h"

#if VF_LOG_ENABLED
#include <cstdio>
#include <cstdarg>
#include <string>

static void GetCurrentProcessBaseName(wchar_t* outName, DWORD maxLen) {
    wchar_t path[MAX_PATH] = {};
    if (GetModuleFileNameW(nullptr, path, MAX_PATH) > 0) {
        const wchar_t* lastSlash = wcsrchr(path, L'\\');
        const wchar_t* name = lastSlash ? (lastSlash + 1) : path;
        wcsncpy_s(outName, maxLen, name, _TRUNCATE);
    } else {
        wcsncpy_s(outName, maxLen, L"unknown.exe", _TRUNCATE);
    }
}

void VF_LogMessage(const char* function, const wchar_t* format, ...) {
    va_list args;
    va_start(args, format);

    wchar_t msgBuf[1024] = {};
    _vsnwprintf_s(msgBuf, _countof(msgBuf), _TRUNCATE, format, args);
    va_end(args);

    SYSTEMTIME st;
    GetLocalTime(&st);

    DWORD pid = GetCurrentProcessId();
    DWORD tid = GetCurrentThreadId();

    wchar_t procName[64] = {};
    GetCurrentProcessBaseName(procName, _countof(procName));

    wchar_t fullLine[2048] = {};
    _snwprintf_s(
        fullLine,
        _countof(fullLine),
        _TRUNCATE,
        L"[%02d:%02d:%02d.%03d] [PID:%5u (%s) TID:%5u] [%S] %s\r\n",
        st.wHour, st.wMinute, st.wSecond, st.wMilliseconds,
        pid, procName, tid,
        function ? function : "unknown",
        msgBuf
    );

    // 1. OutputDebugStringW for DebugView / Visual Studio
    OutputDebugStringW(fullLine);

    // 2. Append to %TEMP%\vietflow_tsf.log
    wchar_t tempPath[MAX_PATH] = {};
    if (GetTempPathW(MAX_PATH, tempPath) > 0) {
        std::wstring logFilePath = std::wstring(tempPath) + L"vietflow_tsf.log";
        HANDLE hFile = CreateFileW(
            logFilePath.c_str(),
            FILE_APPEND_DATA,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            nullptr,
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            nullptr
        );
        if (hFile != INVALID_HANDLE_VALUE) {
            int utf8Len = WideCharToMultiByte(CP_UTF8, 0, fullLine, -1, nullptr, 0, nullptr, nullptr);
            if (utf8Len > 1) {
                std::string utf8Buf(utf8Len - 1, '\0');
                WideCharToMultiByte(CP_UTF8, 0, fullLine, -1, &utf8Buf[0], utf8Len - 1, nullptr, nullptr);
                DWORD written = 0;
                WriteFile(hFile, utf8Buf.data(), static_cast<DWORD>(utf8Buf.length()), &written, nullptr);
            }
            CloseHandle(hFile);
        }
    }
}
#endif


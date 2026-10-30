#include "app_window.h"
#include "theme.h"
#include <windows.h>
#include <string>

#ifdef __cplusplus
extern "C" {
#endif

void SetEmbeddedFontData(const void* data, size_t size) {
    vietflow::SetEmbeddedFontData(data, size);
}

int ShowSettingsGui(const char* config_path, void (*on_saved)()) {
    std::string path = config_path ? config_path : "";
    return vietflow::RunAppWindow(path, false, on_saved);
}

int ShowConverterGui(const char* config_path) {
    std::string path = config_path ? config_path : "";
    return vietflow::RunAppWindow(path, true, nullptr);
}

#ifdef __cplusplus
}
#endif


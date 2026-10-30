#pragma once

#include <windows.h>
#include <string>

namespace vietflow {

int RunAppWindow(const std::string& config_path, bool convert_mode, void (*on_saved)() = nullptr);

} // namespace vietflow

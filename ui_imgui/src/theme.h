#pragma once

#include <imgui.h>

// Nerd Font UTF-8 Icon codepoints (embedded in JetBrains Mono Nerd Font)
#define NF_COG        "\xEF\x80\x93" // 
#define NF_KEYBOARD   "\xEF\x84\x9C" // 
#define NF_BOLT       "\xEF\x83\xA7" // 
#define NF_GAMEPAD    "\xEF\x84\x9B" // 
#define NF_CONVERT    "\xEF\x80\xA1" // 
#define NF_PAINT      "\xEF\x87\xBC" // 
#define NF_INFO       "\xEF\x81\x9A" // 
#define NF_PLUS       "\xEF\x81\xA7" // 
#define NF_TRASH      "\xEF\x87\xB8" // 
#define NF_SAVE       "\xEF\x83\x87" // 
#define NF_COPY       "\xEF\x83\x85" // 
#define NF_FOLDER     "\xEF\x81\xBB" // 
#define NF_CHECK      "\xEF\x80\x8C" // 
#define NF_EXCHANGE   "\xEF\x83\xAC" // 

namespace vietflow {

void SetEmbeddedFontData(const void* data, size_t size);
void SetTrayFontData(const void* data, size_t size);
ImFont* TrayFont();
void ApplyTheme(float dpi_scale = 1.0f);
void SetupFonts(ImGuiIO& io, float dpi_scale = 1.0f);

// Current UI scale (1.0 at 96 DPI). Every hard-coded pixel size must go through
// Px() so the layout keeps its proportions on high-DPI/multi-monitor setups.
float UiScale();
inline float Px(float value) { return value * UiScale(); }

// Helper UI widgets for ImGui
void SectionHeader(const char* title);
void HotkeyBadge(const char* text);
bool NavItem(const char* label, bool selected);

} // namespace vietflow

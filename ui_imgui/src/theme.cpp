#include "theme.h"
#include <windows.h>
#include <cstdlib>
#include <cstring>

namespace vietflow {

static const void* s_embedded_font_data = nullptr;
static size_t s_embedded_font_size = 0;
static const void* s_tray_font_data = nullptr;
static size_t s_tray_font_size = 0;
static ImFont* s_tray_font = nullptr;
static float s_ui_scale = 1.0f;

void SetEmbeddedFontData(const void* data, size_t size) {
    s_embedded_font_data = data;
    s_embedded_font_size = size;
}

void SetTrayFontData(const void* data, size_t size) {
    s_tray_font_data = data;
    s_tray_font_size = size;
}

ImFont* TrayFont() {
    return s_tray_font;
}

static void AddTrayFont(ImGuiIO& io, float dpi_scale) {
    s_tray_font = nullptr;
    if (!s_tray_font_data || s_tray_font_size == 0) return;

    void* font_copy = malloc(s_tray_font_size);
    if (!font_copy) return;
    memcpy(font_copy, s_tray_font_data, s_tray_font_size);

    ImFontConfig cfg;
    cfg.FontDataOwnedByAtlas = true;
    cfg.OversampleH = 1;
    cfg.OversampleV = 1;
    cfg.PixelSnapH = true;
    static const ImWchar tray_ranges[] = { 0x45, 0x45, 0x56, 0x56, 0 };
    s_tray_font = io.Fonts->AddFontFromMemoryTTF(
        font_copy, (int)s_tray_font_size, 42.0f * dpi_scale, &cfg, tray_ranges);
}

float UiScale() {
    return s_ui_scale > 0.0f ? s_ui_scale : 1.0f;
}

void SetupFonts(ImGuiIO& io, float dpi_scale) {
    if (dpi_scale <= 0.0f) dpi_scale = 1.0f;
    float font_size = 15.0f * dpi_scale;

    ImFontConfig font_cfg;
    font_cfg.OversampleH = 2;
    font_cfg.OversampleV = 1;
    font_cfg.PixelSnapH = true;

    // Only the glyphs the UI actually renders. Loading the whole Nerd Font PUA
    // range (0xE000-0xF8FF) inflates the atlas by thousands of unused icons and
    // costs tens of MB of VRAM at high DPI.
    static const ImWchar icon_ranges[] = {
        0x2022, 0x2022, // bullet used by list rows
        0xF00C, 0xF00C, // NF_CHECK
        0xF013, 0xF013, // NF_COG
        0xF01A, 0xF01A, // NF_INFO
        0xF021, 0xF021, // NF_CONVERT
        0xF067, 0xF067, // NF_PLUS
        0xF07B, 0xF07B, // NF_FOLDER
        0xF0C5, 0xF0C5, // NF_COPY
        0xF0C7, 0xF0C7, // NF_SAVE
        0xF0E7, 0xF0E7, // NF_BOLT
        0xF0EC, 0xF0EC, // NF_EXCHANGE
        0xF11B, 0xF11B, // NF_GAMEPAD
        0xF11C, 0xF11C, // NF_KEYBOARD
        0xF1F8, 0xF1F8, // NF_TRASH
        0xF1FC, 0xF1FC, // NF_PAINT
        0
    };

    ImFontGlyphRangesBuilder builder;
    builder.AddRanges(io.Fonts->GetGlyphRangesDefault());
    builder.AddRanges(io.Fonts->GetGlyphRangesVietnamese());
    builder.AddRanges(icon_ranges);

    static ImVector<ImWchar> full_ranges;
    builder.BuildRanges(&full_ranges);

    // 1. Direct in-memory font embedded in Rust binary (zero disk I/O, zero extraction)
    if (s_embedded_font_data && s_embedded_font_size > 0) {
        void* font_copy = malloc(s_embedded_font_size);
        if (font_copy) {
            memcpy(font_copy, s_embedded_font_data, s_embedded_font_size);
            font_cfg.FontDataOwnedByAtlas = true;
            io.Fonts->AddFontFromMemoryTTF(font_copy, (int)s_embedded_font_size, font_size, &font_cfg, full_ranges.Data);
            AddTrayFont(io, dpi_scale);
            return;
        }
    }

    // 2. Fallback: Windows system font
    const char* fallback_font = "C:\\Windows\\Fonts\\segoeui.ttf";
    if (GetFileAttributesA(fallback_font) != INVALID_FILE_ATTRIBUTES) {
        io.Fonts->AddFontFromFileTTF(fallback_font, 16.0f * dpi_scale, &font_cfg, full_ranges.Data);
    } else {
        io.Fonts->AddFontDefault(&font_cfg);
    }
    AddTrayFont(io, dpi_scale);
}

void ApplyTheme(float dpi_scale) {
    if (dpi_scale <= 0.0f) dpi_scale = 1.0f;
    s_ui_scale = dpi_scale;
    ImGuiStyle& style = ImGui::GetStyle();

    // Reset to classic Dear ImGui Dark style
    ImGui::StyleColorsDark(&style);

    style.WindowPadding     = ImVec2(12.0f, 12.0f);
    style.FramePadding      = ImVec2(8.0f, 4.0f);
    style.CellPadding       = ImVec2(6.0f, 4.0f);
    style.ItemSpacing       = ImVec2(8.0f, 8.0f);
    style.ItemInnerSpacing  = ImVec2(6.0f, 4.0f);
    style.IndentSpacing     = 18.0f;
    style.ScrollbarSize     = 12.0f;
    style.GrabMinSize       = 10.0f;

    style.WindowBorderSize  = 1.0f;
    style.ChildBorderSize   = 1.0f;
    style.PopupBorderSize   = 1.0f;
    style.FrameBorderSize   = 1.0f;

    // Classic subtle rounding (3-4px like standard ImGui)
    style.WindowRounding    = 4.0f;
    style.ChildRounding     = 3.0f;
    style.FrameRounding     = 3.0f;
    style.PopupRounding     = 3.0f;
    style.ScrollbarRounding = 4.0f;
    style.GrabRounding      = 3.0f;
    style.TabRounding       = 4.0f;
    style.TabBorderSize     = 0.0f;

    // Scale sizes by DPI
    style.ScaleAllSizes(dpi_scale);

    // SeparatorText clean style
    style.SeparatorTextBorderSize = 1.0f;
    style.SeparatorTextAlign = ImVec2(0.0f, 0.5f);
    style.SeparatorTextPadding = ImVec2(6.0f, 4.0f);

    ImVec4* colors = style.Colors;
    colors[ImGuiCol_WindowBg]           = ImVec4(0.09f, 0.10f, 0.13f, 1.00f);
    colors[ImGuiCol_ChildBg]            = ImVec4(0.09f, 0.10f, 0.13f, 0.00f);
    colors[ImGuiCol_PopupBg]            = ImVec4(0.12f, 0.13f, 0.17f, 0.98f);
    colors[ImGuiCol_Border]             = ImVec4(0.24f, 0.27f, 0.35f, 0.80f);
    colors[ImGuiCol_FrameBg]            = ImVec4(0.15f, 0.17f, 0.23f, 1.00f);
    colors[ImGuiCol_FrameBgHovered]     = ImVec4(0.20f, 0.24f, 0.32f, 1.00f);
    colors[ImGuiCol_FrameBgActive]      = ImVec4(0.25f, 0.30f, 0.40f, 1.00f);

    colors[ImGuiCol_Button]             = ImVec4(0.20f, 0.38f, 0.60f, 0.70f);
    colors[ImGuiCol_ButtonHovered]      = ImVec4(0.26f, 0.48f, 0.75f, 1.00f);
    colors[ImGuiCol_ButtonActive]       = ImVec4(0.16f, 0.32f, 0.52f, 1.00f);

    colors[ImGuiCol_Header]             = ImVec4(0.20f, 0.38f, 0.60f, 0.55f);
    colors[ImGuiCol_HeaderHovered]      = ImVec4(0.26f, 0.48f, 0.75f, 0.80f);
    colors[ImGuiCol_HeaderActive]       = ImVec4(0.26f, 0.48f, 0.75f, 1.00f);

    colors[ImGuiCol_CheckMark]          = ImVec4(0.38f, 0.72f, 1.00f, 1.00f);
    colors[ImGuiCol_SliderGrab]         = ImVec4(0.30f, 0.55f, 0.85f, 1.00f);
    colors[ImGuiCol_SliderGrabActive]   = ImVec4(0.38f, 0.72f, 1.00f, 1.00f);

    colors[ImGuiCol_Separator]          = ImVec4(0.22f, 0.25f, 0.32f, 1.00f);
    colors[ImGuiCol_SeparatorHovered]   = ImVec4(0.30f, 0.45f, 0.65f, 1.00f);
    colors[ImGuiCol_SeparatorActive]    = ImVec4(0.38f, 0.72f, 1.00f, 1.00f);

    colors[ImGuiCol_Tab]                = ImVec4(0.14f, 0.22f, 0.35f, 0.70f);
    colors[ImGuiCol_TabHovered]         = ImVec4(0.22f, 0.42f, 0.68f, 0.90f);
    colors[ImGuiCol_TabSelected]        = ImVec4(0.18f, 0.38f, 0.65f, 1.00f);
    colors[ImGuiCol_TabDimmed]          = ImVec4(0.12f, 0.16f, 0.24f, 0.70f);
    colors[ImGuiCol_TabDimmedSelected]  = ImVec4(0.16f, 0.28f, 0.46f, 1.00f);
    colors[ImGuiCol_TabSelectedOverline]       = ImVec4(0.0f, 0.0f, 0.0f, 0.0f);
    colors[ImGuiCol_TabDimmedSelectedOverline] = ImVec4(0.0f, 0.0f, 0.0f, 0.0f);
}

void SectionHeader(const char* title) {
    ImGui::Spacing();
    ImGui::SeparatorText(title);
}

void HotkeyBadge(const char* text) {
    ImVec2 p = ImGui::GetCursorScreenPos();
    ImVec2 size = ImGui::CalcTextSize(text);
    size.x += Px(16.0f);
    size.y += Px(8.0f);

    ImDrawList* draw_list = ImGui::GetWindowDrawList();
    ImU32 bg_col = IM_COL32(30, 50, 75, 255);
    ImU32 border_col = IM_COL32(60, 110, 170, 255);

    draw_list->AddRectFilled(p, ImVec2(p.x + size.x, p.y + size.y), bg_col, Px(3.0f));
    draw_list->AddRect(p, ImVec2(p.x + size.x, p.y + size.y), border_col, Px(3.0f));

    ImVec2 text_pos(p.x + Px(8.0f), p.y + Px(4.0f));
    draw_list->AddText(text_pos, IM_COL32(230, 235, 245, 255), text);

    ImGui::Dummy(size);
}

bool NavItem(const char* label, bool selected) {
    return ImGui::Selectable(label, selected, 0, ImVec2(0.0f, Px(28.0f)));
}

} // namespace vietflow

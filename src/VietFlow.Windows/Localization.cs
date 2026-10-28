// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

namespace VietFlow.Windows;

public static class Localization
{
    public static bool IsEnglish(string lang) => string.Equals(lang, "en", StringComparison.OrdinalIgnoreCase);

    public static string Get(string key, string lang = "vi")
    {
        bool en = IsEnglish(lang);
        return key switch
        {
            // App & Nav
            "AppTitle" => en ? "VietFlow - High Performance Vietnamese Input Engine" : "VietFlow - Bộ gõ tiếng Việt hiệu năng cao",
            "NavOverview" => en ? "Overview" : "Tổng quan",
            "NavTyping" => en ? "Typing & Spelling" : "Bộ gõ & Chính tả",
            "NavMacro" => en ? "Macros & Snippets" : "Gõ tắt & Biến động",
            "NavGame" => en ? "Game & Apps" : "Game & Ứng dụng",

            // Status Card
            "StatusVn" => en ? "VIETNAMESE MODE" : "Đang gõ TIẾNG VIỆT",
            "StatusEn" => en ? "ENGLISH MODE [E]" : "Đang ở chế độ TIẾNG ANH [E]",
            "StatusReady" => en ? "Ready" : "Đang bật (Sẵn sàng)",
            "StatusDisabled" => en ? "Vietnamese Disabled" : "Đã tắt tiếng Việt",

            // Overview Tab
            "OverviewTitle" => en ? "Overview & Settings" : "Tổng quan & Thiết lập",
            "OverviewDesc" => en ? "Character encoding, input methods, and system integration." : "Bảng mã, kiểu gõ và các tùy chọn tích hợp hệ thống.",
            "SectionInput" => en ? "INPUT CONFIGURATION" : "THIẾT LẬP BỘ GÕ",
            "Charset" => en ? "Character Set" : "Bảng mã ký tự",
            "CharsetDesc" => en ? "Standard Unicode NFC precomposed default" : "Mặc định Unicode NFC chuẩn quốc tế",
            "Method" => en ? "Input Method" : "Kiểu gõ",
            "MethodDesc" => en ? "Supports Telex, Simple Telex, and VNI" : "Hỗ trợ Telex, Simple Telex, và VNI",
            "SwitchKey" => en ? "Switch Language Key" : "Phím chuyển đổi",
            "SwitchKeyDesc" => en ? "Hotkey to toggle between Vietnamese and English" : "Phím tắt chuyển đổi giữa tiếng Việt và tiếng Anh",
            "UiLanguage" => en ? "Interface Language" : "Ngôn ngữ giao diện",
            "UiLanguageDesc" => en ? "Select display language for VietFlow" : "Chọn ngôn ngữ hiển thị cho phần mềm",

            // System integration
            "SectionSystem" => en ? "SYSTEM INTEGRATION" : "TÍCH HỢP HỆ THỐNG",
            "StartWithWin" => en ? "Start with Windows" : "Khởi động cùng Windows",
            "StartWithWinDesc" => en ? "Automatically launch at Windows logon with silent privileges" : "Tự động khởi động khi bật máy với quyền tối ưu",

            // Typing Tab
            "TypingTitle" => en ? "Typing Rules & Orthography" : "Quy tắc gõ & Chính tả",
            "TypingDesc" => en ? "Tone placement conventions, spell check, and shorthand expansions." : "Tùy biến thuật toán ghép chữ, quy tắc bỏ dấu và gõ tắt.",
            "FreeMarking" => en ? "Free Tone Placement" : "Cho phép gõ tự do",
            "FreeMarkingDesc" => en ? "Place tone marks anywhere in the word" : "Bỏ dấu ở bất kỳ vị trí nào trong từ mà không cần gõ ngay sau nguyên âm",
            "SpellCheck" => en ? "Vietnamese Spell Check" : "Bật kiểm tra chính tả tiếng Việt",
            "SpellCheckDesc" => en ? "Strict syllable structure; bypass tones for non-Vietnamese patterns" : "Nhận diện cấu trúc âm tiết chuẩn; ngắt bỏ dấu khi gõ các từ không có trong tiếng Việt",
            "AutoRestore" => en ? "Auto Restore English Words" : "Tự động khôi phục từ tiếng Anh (custom, cool, post)",
            "AutoRestoreDesc" => en ? "Restores original raw keys when typed word is an English word" : "Tự động trả lại các ký tự gốc khi từ gõ vào là từ tiếng Anh hoàn chỉnh",
            "ModernStyle" => en ? "Modern Tone Style (oà, uỳ)" : "Đặt dấu kiểu mới (oà, oè, uỳ thay vì òa, òe, ùy)",
            "ModernStyleDesc" => en ? "Place tone on secondary vowel (modern educational convention)" : "Theo chuẩn ngữ pháp tiếng Việt mới",
            "ShiftLag" => en ? "Shift Key Lag Auto-Correction" : "Tự động sửa lỗi dính phím Shift (VIệt -> Việt, TRường -> Trường)",
            "ShiftLagDesc" => en ? "Automatically demotes second uppercase letter when typing fast" : "Tự động hạ chữ in hoa thứ hai thành chữ thường khi gõ lướt",
            "ConsonantShortcuts" => en ? "Consonant Shortcuts (f->ph, j->gi, w->qu)" : "Bật phím tắt phụ âm đầu (f->ph, j->gi, w->qu)",
            "ConsonantShortcutsDesc" => en ? "Single key shorthand expansions for fast typing" : "Gõ nhanh phụ âm đầu bằng 1 phím",
            "UseClipboard" => en ? "Always Use Clipboard for Unicode" : "Luôn sử dụng Clipboard cho Unicode",
            "UseClipboardDesc" => en ? "Send characters via clipboard instead of virtual keys (for legacy games/apps)" : "Gửi ký tự qua bộ nhớ tạm thay vì phím ảo (dành cho game hoặc app cũ)",

            // Macro Tab
            "MacroTitle" => en ? "Macros & Dynamic Snippets" : "Gõ tắt & Biến động",
            "MacroDesc" => en ? "Manage abbreviations with dynamic date/time and clipboard insertion." : "Quản lý các cụm từ viết tắt và hỗ trợ biến động chèn ngày giờ, nội dung copy.",
            "EnableMacro" => en ? "Enable Macro Feature" : "Bật tính năng gõ tắt (Macro)",
            "EnableMacroDesc" => en ? "Expands abbreviations upon pressing Space or punctuation" : "Tự động bung từ thay thế khi nhấn phím Cách (Space) hoặc dấu câu",
            "MacroWhenDisabled" => en ? "Allow Macros in English Mode [E]" : "Cho phép gõ tắt cả khi đang tắt tiếng Việt [E]",
            "MacroWhenDisabledDesc" => en ? "Expands macros even when the tray icon is 'E'" : "Vẫn mở rộng các từ viết tắt ngay cả khi biểu tượng khay đang là chữ E",
            "BtnAddSample" => en ? "Add Samples" : "Thêm mẫu",
            "BtnImport" => en ? "Import .txt" : "Nhập .txt",
            "BtnExport" => en ? "Export .txt" : "Xuất .txt",
            "BtnClearAll" => en ? "Clear All" : "Xóa hết",
            "ColMacroKey" => en ? "Abbreviation" : "Từ viết tắt",
            "ColMacroVal" => en ? "Replacement" : "Cụm thay thế",
            "BtnSaveMacro" => en ? "Save Macro" : "Lưu từ",
            "BtnDeleteMacro" => en ? "Delete" : "Xóa",

            // Game Tab
            "GameTitle" => en ? "Game Mode & Excluded Apps" : "Game & Ứng dụng thông minh",
            "GameDesc" => en ? "Automatically lock English mode or bypass hook for games." : "Tự động chuyển chế độ tiếng Anh hoặc bỏ qua hook để không kẹt phím khi chơi game.",
            "ExcludedListTitle" => en ? "EXCLUDED APPLICATIONS (GAME MODE)" : "DANH SÁCH ỨNG DỤNG LOẠI TRỪ (GAME MODE)",
            "BtnSelectProcess" => en ? "Select Process..." : "Chọn tiến trình...",
            "BtnBrowseExe" => en ? "Browse .exe..." : "Duyệt file .exe...",
            "BtnAddManual" => en ? "Add Manual" : "Thêm thủ công",

            // Bottom Buttons
            "BtnReset" => en ? "Restore Defaults" : "Khôi phục mặc định",
            "BtnClose" => en ? "Close" : "Đóng",
            "BtnSave" => en ? "Save Settings" : "Lưu cấu hình",

            _ => key
        };
    }
}


// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Text;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Platform.Storage;
using Avalonia.Threading;
using VietFlow.Configuration;
using VietFlow.Engine;
using VietFlow.Tools;
using EngineInputMethod = VietFlow.Engine.InputMethod;

namespace VietFlow.Windows;

public partial class SettingsWindow : Window
{
    private readonly VietFlowConfig _config;
    private readonly VietFlowSession _session;
    private readonly Action _onStateUpdated;
    private readonly UpdateService _updateService;

    public sealed class MacroItem : INotifyPropertyChanged
    {
        private string _key = string.Empty;
        private string _value = string.Empty;

        public string Key
        {
            get => _key;
            set
            {
                if (_key == value)
                {
                    return;
                }

                _key = value;
                PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Key)));
            }
        }

        public string Value
        {
            get => _value;
            set
            {
                if (_value == value)
                {
                    return;
                }

                _value = value;
                PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Value)));
            }
        }

        public event PropertyChangedEventHandler? PropertyChanged;
    }

    private readonly ObservableCollection<MacroItem> _macroItems = new();
    private readonly ObservableCollection<ProcessInfo> _excludedAppItems = new();

    private UiOption<CharsetId>[] _charsetOptions = Array.Empty<UiOption<CharsetId>>();
    private UiOption<EngineInputMethod>[] _methodOptions = Array.Empty<UiOption<EngineInputMethod>>();
    private UiOption<string>[] _hotkeyOptions = Array.Empty<UiOption<string>>();
    private UiOption<string>[] _languageOptions = Array.Empty<UiOption<string>>();
    private UiOption<string>[] _themeOptions = Array.Empty<UiOption<string>>();
    private UiOption<int>[] _timeoutOptions = Array.Empty<UiOption<int>>();

    private bool _loading = true;
    private string _versionText = "0.1.0";

    public SettingsWindow(VietFlowConfig config, VietFlowSession session, Action onStateUpdated, UpdateService? updateService = null)
    {
        InitializeComponent();
        _config = config;
        _session = session;
        _onStateUpdated = onStateUpdated;
        _updateService = updateService ?? new UpdateService();

        PopulateControls();
        LoadConfigData();
        UpdateLiveStatusCard();
        UpdateMacroCount();
        UpdateExcludedAppsCount();
        UpdateTrayPreview();

        var version = typeof(SettingsWindow).Assembly.GetName().Version;
        var informational = typeof(SettingsWindow).Assembly
            .GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion;
        _versionText = (informational ?? version?.ToString(3) ?? "0.1.0").Split('+')[0];
        TxtAboutVersion.Text = _config.UiLanguage == "en"
            ? $"Version {_versionText}"
            : $"Phiên bản {_versionText}";

        _loading = false;
        Closed += (_, _) => NativeMethods.TrimProcessMemory();
    }

    private void PopulateControls()
    {
        PopulateCharsetOptions(_config.UiLanguage);
        PopulateTimeoutOptions();

        _methodOptions = new[]
        {
            new UiOption<EngineInputMethod>(EngineInputMethod.Telex, "Telex"),
            new UiOption<EngineInputMethod>(EngineInputMethod.SimpleTelex, "Simple Telex"),
            new UiOption<EngineInputMethod>(EngineInputMethod.Vni, "VNI"),
            new UiOption<EngineInputMethod>(EngineInputMethod.Viqr, "VIQR")
        };
        CboMethod.ItemsSource = _methodOptions;

        PopulateHotkeyOptions(_config.UiLanguage);

        _languageOptions = new[]
        {
            new UiOption<string>("vi", "Tiếng Việt"),
            new UiOption<string>("en", "English")
        };
        CboUiLanguage.ItemsSource = _languageOptions;

        PopulateThemeOptions(_config.UiLanguage);

        DgMacros.ItemsSource = _macroItems;
        LstExcludedApps.ItemsSource = _excludedAppItems;
    }

    private void PopulateCharsetOptions(string lang)
    {
        bool en = Localization.IsEnglish(lang);
        var selected = CboCharset.SelectedItem as UiOption<CharsetId>;
        _charsetOptions = new[]
        {
            new UiOption<CharsetId>(CharsetId.Unicode, en ? "Unicode (NFC)" : "Unicode dựng sẵn (NFC)"),
            new UiOption<CharsetId>(CharsetId.UnicodeComposite, en ? "Unicode Composite (NFD)" : "Unicode tổ hợp (NFD)"),
            new UiOption<CharsetId>(CharsetId.Tcvn3, "TCVN3 (ABC)"),
            new UiOption<CharsetId>(CharsetId.VniWindows, "VNI Windows"),
            new UiOption<CharsetId>(CharsetId.Custom, en ? "Custom..." : "Tùy chỉnh...")
        };
        CboCharset.ItemsSource = _charsetOptions;

        if (selected != null)
        {
            CboCharset.SelectedItem = FindCharset(selected.Id);
        }
    }

    private void PopulateHotkeyOptions(string lang)
    {
        bool en = Localization.IsEnglish(lang);
        var selected = CboToggleHotkey.SelectedItem as UiOption<string>;
        _hotkeyOptions = new[]
        {
            new UiOption<string>("CtrlShift", "Ctrl + Shift"),
            new UiOption<string>("AltZ", "Alt + Z"),
            new UiOption<string>("CtrlSpace", "Ctrl + Space"),
            new UiOption<string>("ShiftSpace", "Shift + Space"),
            new UiOption<string>("AltShift", "Alt + Shift"),
            new UiOption<string>("WinSpace", "Win + Space"),
            new UiOption<string>("CapsLock", "Caps Lock"),
            new UiOption<string>("RightCtrl", en ? "Right Control" : "Ctrl phải"),
            new UiOption<string>("Custom", en ? "Custom key..." : "Phím tự gán (Custom)...")
        };
        CboToggleHotkey.ItemsSource = _hotkeyOptions;
        if (selected != null)
        {
            CboToggleHotkey.SelectedItem = _hotkeyOptions.FirstOrDefault(option => option.Id == selected.Id);
        }
    }

    private void PopulateTimeoutOptions()
    {
        _timeoutOptions = new[]
        {
            new UiOption<int>(500, "0.5 s"),
            new UiOption<int>(1000, "1 s"),
            new UiOption<int>(1500, "1.5 s"),
            new UiOption<int>(2000, "2 s"),
            new UiOption<int>(2500, "2.5 s"),
            new UiOption<int>(3000, "3 s"),
            new UiOption<int>(5000, "5 s")
        };
        CboTypingTimeout.ItemsSource = _timeoutOptions;
    }

    private void PopulateThemeOptions(string lang)
    {
        bool en = Localization.IsEnglish(lang);
        var selected = CboTheme.SelectedItem as UiOption<string>;
        _themeOptions = new[]
        {
            new UiOption<string>(ThemeManager.SystemMode, en ? "System" : "Theo hệ thống"),
            new UiOption<string>(ThemeManager.LightMode, en ? "Light" : "Sáng (Light)"),
            new UiOption<string>(ThemeManager.DarkMode, en ? "Dark" : "Tối (Dark)")
        };
        CboTheme.ItemsSource = _themeOptions;
        if (selected != null)
        {
            CboTheme.SelectedItem = _themeOptions.FirstOrDefault(option => option.Id == selected.Id);
        }
    }

    private UiOption<CharsetId> FindCharset(CharsetId id) =>
        _charsetOptions.First(option => option.Id == id);

    private UiOption<EngineInputMethod> FindMethod(EngineInputMethod id) =>
        _methodOptions.First(option => option.Id == id);

    private UiOption<string> FindOption(UiOption<string>[] options, string id) =>
        options.First(option => option.Id == id);

    private void LoadConfigData()
    {
        CboCharset.SelectedItem = FindCharset(_config.Charset);
        CboMethod.SelectedItem = FindMethod(_config.Method);

        string hotkey = string.IsNullOrEmpty(_config.ToggleHotkey) ? "CtrlShift" : _config.ToggleHotkey;
        CboToggleHotkey.SelectedItem = _hotkeyOptions.FirstOrDefault(option => option.Id == hotkey) ?? _hotkeyOptions[0];
        CboUiLanguage.SelectedItem = _languageOptions.FirstOrDefault(option => option.Id == _config.UiLanguage) ?? _languageOptions[0];
        CboTheme.SelectedItem = _themeOptions.FirstOrDefault(option => option.Id == ThemeManager.Normalize(_config.ThemeMode)) ?? _themeOptions[0];
        CboTypingTimeout.SelectedItem = _timeoutOptions.FirstOrDefault(option => option.Id == _config.TypingTimeoutMs) ?? _timeoutOptions[5];

        UpdateCustomHotkeyUi();
        ApplyLocalization(_config.UiLanguage);

        SwStartWithWindows.IsChecked = StartupHelper.IsEnabled();
        SwNotificationSound.IsChecked = _config.NotificationSound;

        SwFreeMarking.IsChecked = _config.FreeMarking;
        SwSpellCheck.IsChecked = _config.SpellCheck;
        SwAutoRestore.IsChecked = _config.AutoNonVnRestore;
        SwModernStyle.IsChecked = _config.ModernStyle;

        SwFixShiftLag.IsChecked = _config.FixShiftLag;
        SwConsonantShortcuts.IsChecked = _config.ConsonantShortcuts;
        SwUseClipboardForUnicode.IsChecked = _config.UseClipboardForUnicode;
        SwAutoCapitalize.IsChecked = _config.AutoCapitalize;
        SwAutoCheckUpdates.IsChecked = _config.AutoCheckUpdates;

        SwEnableMacro.IsChecked = _config.EnableMacro;
        SwMacroWhenDisabled.IsChecked = _config.AllowMacroWhenDisabled;

        var keywords = _config.EnglishKeywords.Count > 0 ? _config.EnglishKeywords : EnglishPhonetics.DefaultKeywords;
        TxtEnglishKeywords.Text = string.Join(Environment.NewLine, keywords);
        TxtForeignInitials.Text = string.IsNullOrWhiteSpace(_config.ForeignInitials) ? "fjwz" : _config.ForeignInitials;
        _config.MacroVariables ??= new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        TxtMacroDateFormat.Text = string.IsNullOrWhiteSpace(_config.MacroDateFormat) ? "dd/MM/yyyy" : _config.MacroDateFormat;
        TxtMacroTimeFormat.Text = string.IsNullOrWhiteSpace(_config.MacroTimeFormat) ? "HH:mm" : _config.MacroTimeFormat;
        TxtMacroDateTimeFormat.Text = string.IsNullOrWhiteSpace(_config.MacroDateTimeFormat) ? "dd/MM/yyyy HH:mm" : _config.MacroDateTimeFormat;
        TxtMacroVariables.Text = string.Join(Environment.NewLine, _config.MacroVariables.Select(pair => $"{pair.Key}={pair.Value}"));
        TxtConfigPath.Text = VietFlowConfig.GetConfigFilePath();
        UpdateKeywordsCount();

        _macroItems.Clear();
        foreach (var (key, value) in _config.Macros)
        {
            _macroItems.Add(new MacroItem { Key = key, Value = value });
        }

        _excludedAppItems.Clear();
        foreach (string app in _config.ExcludedApps)
        {
            _excludedAppItems.Add(ProcessInfo.FromExe(app));
        }

        UpdateLiveStatusCard();
        UpdateMacroCount();
        UpdateMacroListHeight();
        UpdateExcludedAppsCount();
        UpdateTrayPreview();
    }

    private void ApplyLocalization(string lang)
    {
        bool en = Localization.IsEnglish(lang);

        TxtAppTagline.Text = en ? "Vietnamese input engine" : "Bộ gõ tiếng Việt";
        LblOverviewTitle.Text = en ? "Overview & Settings" : "Tổng quan & Thiết lập";
        LblTypingTitle.Text = en ? "Typing & Spelling" : "Bộ gõ & Chính tả";
        LblMacroTitle.Text = en ? "Macros & Snippets" : "Gõ tắt & Biến động";
        LblGameTitle.Text = en ? "Game & Apps" : "Game & Ứng dụng";
        LblAdvancedTitle.Text = en ? "Advanced settings" : "Cấu hình nâng cao";
        LblAboutTitle.Text = en ? "About VietFlow" : "Giới thiệu VietFlow";
        LblKeywordSection.Text = en ? "WORD LISTS" : "DANH SÁCH TỪ";
        LblAdvancedSystem.Text = en ? "SYSTEM" : "HỆ THỐNG";
        LblAboutLinks.Text = en ? "LINKS" : "LIÊN KẾT";
        LblUpdateSection.Text = en ? "APPLICATION UPDATES" : "CẬP NHẬT ỨNG DỤNG";
        LblAutoUpdate.Text = en ? "Check for updates automatically" : "Tự động kiểm tra cập nhật";
        TxtAboutVersion.Text = en ? $"Version {_versionText}" : $"Phiên bản {_versionText}";

        TabOverview.Header = Localization.Get("NavOverview", lang);
        TabTyping.Header = Localization.Get("NavTyping", lang);
        TabMacro.Header = Localization.Get("NavMacro", lang);
        TabGame.Header = Localization.Get("NavGame", lang);
        TabAbout.Header = Localization.Get("NavAbout", lang);
        TabAdvanced.Header = Localization.Get("NavAdvanced", lang);

        LblOverviewDesc.Text = Localization.Get("OverviewDesc", lang);
        LblSectionInput.Text = Localization.Get("SectionInput", lang);
        LblCharsetTitle.Text = Localization.Get("Charset", lang);
        LblCharsetDesc.Text = Localization.Get("CharsetDesc", lang);
        LblMethodTitle.Text = Localization.Get("Method", lang);
        LblMethodDesc.Text = Localization.Get("MethodDesc", lang);
        LblSwitchKeyTitle.Text = Localization.Get("SwitchKey", lang);
        LblSwitchKeyDesc.Text = Localization.Get("SwitchKeyDesc", lang);
        LblUiLangTitle.Text = Localization.Get("UiLanguage", lang);
        LblUiLangDesc.Text = Localization.Get("UiLanguageDesc", lang);
        LblSectionSystem.Text = Localization.Get("SectionSystem", lang);
        LblStartWithWinTitle.Text = Localization.Get("StartWithWin", lang);
        LblStartWithWinDesc.Text = Localization.Get("StartWithWinDesc", lang);
        LblThemeTitle.Text = en ? "Theme" : "Chế độ màu";
        LblThemeDesc.Text = en ? "System, Light or Dark" : "Theo hệ thống, Sáng hoặc Tối";
        LblSoundTitle.Text = en ? "Notification sound" : "Âm thanh thông báo";
        LblSoundDesc.Text = en ? "Play a sound when switching V / E" : "Phát âm thanh khi chuyển đổi V / E";
        LblTrayTitle.Text = en ? "Customize tray icon" : "Tùy chỉnh Tray icon";
        LblTrayDesc.Text = en ? "Badge and V / E letter colors in the notification area" : "Màu badge và màu chữ V / E trên khay hệ thống";
        BtnCustomizeTray.Content = en ? "Customize..." : "Tùy chỉnh...";
        BtnEditCharset.Content = Localization.Get("BtnEditCharset", lang);

        LblTypingDesc.Text = Localization.Get("TypingDesc", lang);
        LblKillerFeatures.Text = Localization.Get("KillerFeatures", lang);
        LblShiftLagTitle.Text = Localization.Get("ShiftLag", lang);
        LblShiftLagDesc.Text = Localization.Get("ShiftLagDesc", lang);
        LblConsonantTitle.Text = Localization.Get("ConsonantShortcuts", lang);
        LblConsonantDesc.Text = Localization.Get("ConsonantShortcutsDesc", lang);
        LblModernStyleTitle.Text = Localization.Get("ModernStyle", lang);
        LblModernStyleDesc.Text = Localization.Get("ModernStyleDesc", lang);
        LblSectionSpelling.Text = Localization.Get("SectionSpelling", lang);
        LblFreeMarkingTitle.Text = Localization.Get("FreeMarking", lang);
        LblFreeMarkingDesc.Text = Localization.Get("FreeMarkingDesc", lang);
        LblSpellCheckTitle.Text = Localization.Get("SpellCheck", lang);
        LblSpellCheckDesc.Text = Localization.Get("SpellCheckDesc", lang);
        LblAutoRestoreTitle.Text = Localization.Get("AutoRestore", lang);
        LblAutoRestoreDesc.Text = Localization.Get("AutoRestoreDesc", lang);
        LblUseClipboardTitle.Text = Localization.Get("UseClipboard", lang);
        LblUseClipboardDesc.Text = Localization.Get("UseClipboardDesc", lang);
        LblAutoCapTitle.Text = en ? "Auto-capitalize after Enter . ! ?" : "Tự động viết hoa sau Enter . ! ?";
        LblAutoCapDesc.Text = en ? "Capitalize the first letter of the next sentence" : "Viết hoa chữ cái đầu tiên của câu tiếp theo";
        LblTypingTimeoutTitle.Text = en ? "Key composition timeout" : "Thời gian chờ giữa các phím";
        LblTypingTimeoutDesc.Text = en
            ? "The current word is finalized after this idle time. Increase it if you type slowly."
            : "Từ đang gõ sẽ được chốt sau khoảng thời gian này. Tăng lên nếu bạn gõ chậm.";

        LblMacroDesc.Text = Localization.Get("MacroDesc", lang);
        LblEnableMacroTitle.Text = Localization.Get("EnableMacro", lang);
        LblEnableMacroDesc.Text = Localization.Get("EnableMacroDesc", lang);
        LblMacroWhenDisabledTitle.Text = Localization.Get("MacroWhenDisabled", lang);
        LblMacroWhenDisabledDesc.Text = Localization.Get("MacroWhenDisabledDesc", lang);
        LblMacroInfoTitle.Text = Localization.Get("MacroInfoTitle", lang);
        LblMacroInfoBody.Text = Localization.Get("MacroInfoBody", lang);
        LblMacroListHeader.Text = Localization.Get("MacroList", lang);
        TxtMacroSearch.PlaceholderText = Localization.Get("MacroSearchHint", lang);
        LblMacroKeyLabel.Text = Localization.Get("MacroKeyLabel", lang);
        LblMacroValueLabel.Text = Localization.Get("MacroValueLabel", lang);
        LblMacroKeyColumn.Text = Localization.Get("ColMacroKey", lang);
        LblMacroValueColumn.Text = Localization.Get("ColMacroVal", lang);
        BtnAddSampleMacros.Content = Localization.Get("BtnAddSample", lang);
        BtnImportMacros.Content = Localization.Get("BtnImport", lang);
        BtnExportMacros.Content = Localization.Get("BtnExport", lang);
        BtnClearMacros.Content = Localization.Get("BtnClearAll", lang);
        BtnAddMacro.Content = Localization.Get("BtnSaveMacro", lang);
        BtnDeleteMacro.Content = Localization.Get("BtnDeleteMacro", lang);

        LblGameDesc.Text = Localization.Get("GameDesc", lang);
        LblExcludedAppsHeader.Text = Localization.Get("ExcludedListTitle", lang);
        BtnPickRunningProcess.Content = Localization.Get("BtnSelectProcess", lang);
        BtnBrowseExe.Content = Localization.Get("BtnBrowseExe", lang);
        BtnClearExcludedApps.Content = Localization.Get("BtnClearAll", lang);
        BtnAddApp.Content = Localization.Get("BtnAddManual", lang);
        BtnDeleteApp.Content = Localization.Get("BtnDeleteMacro", lang);

        LblAdvancedDesc.Text = en ? "Tune rarely used word lists and engine parameters." : "Tùy chỉnh các danh sách từ và tham số ít dùng của engine.";
        LblKeywordsTitle.Text = en ? "English / foreign words" : "Từ tiếng Anh / từ ngoại lai";
        LblKeywordsDesc.Text = en ? "One word per line. These words keep their original keys and are not accented." : "Mỗi dòng một từ. Các từ này sẽ không bị bỏ dấu tiếng Việt.";
        LblForeignTitle.Text = en ? "Foreign initial letters" : "Phụ âm đầu nước ngoài";
        LblForeignDesc.Text = en ? "Words starting with these letters are not accented (default: fjwz)." : "Các chữ cái đầu này sẽ không nhận dấu tiếng Việt (mặc định: fjwz).";
        LblMacroVarsSection.Text = en ? "MACRO VARIABLES" : "BIẾN ĐỘNG MACRO";
        LblMacroFormatsTitle.Text = en ? "Date / time formats" : "Định dạng ngày / giờ";
        LblMacroFormatsDesc.Text = en ? "Uses .NET format syntax, e.g. dd/MM/yyyy or HH:mm." : "Dùng cú pháp .NET, ví dụ dd/MM/yyyy hoặc HH:mm.";
        LblMacroCustomTitle.Text = en ? "Custom variables" : "Biến tùy chỉnh";
        LblMacroCustomDesc.Text = en ? "One variable per line: name=value; use @name in a macro." : "Mỗi dòng một biến dạng tên=giá trị; dùng @tên trong macro.";
        LblMacroBuiltins.Text = en
            ? "Built-in: @date @time @datetime @day @month @year @weekday @timestamp @random @uuid @user @computer @clipboard @newline @tab"
            : "Có sẵn: @date @time @datetime @day @month @year @weekday @timestamp @random @uuid @user @computer @clipboard @newline @tab";
        BtnResetKeywords.Content = en ? "Restore defaults" : "Khôi phục mặc định";
        BtnSaveKeywords.Content = en ? "Save list" : "Lưu danh sách";
        BtnOpenConfigFolder.Content = en ? "Open folder" : "Mở thư mục";

        LblAboutDesc.Text = en ? "High-performance Vietnamese input engine." : "Bộ gõ tiếng Việt hiệu năng cao.";
        TxtAboutLicense.Text = en
            ? "GNU GPL v3.0 license · Based on UniKey by Phạm Kim Long"
            : "Giấy phép GNU GPL v3.0 · Dựa trên UniKey của Phạm Kim Long";
        TxtMacroKey.PlaceholderText = en ? "Abbreviation" : "Từ gõ tắt";
        TxtMacroValue.PlaceholderText = en ? "Replacement" : "Cụm thay thế";
        TxtAppProcess.PlaceholderText = en ? "Enter .exe (e.g. cs2.exe)" : "Nhập file .exe (ví dụ: cs2.exe)";
        LblCustomHotkeyPrompt.Text = en ? "Custom shortcut:" : "Phím tắt tự gán:";
        LblCustomHotkeyHint.Text = en
            ? "Click the box on the right and press the key combination"
            : "Bấm vào ô bên phải và nhấn tổ hợp phím muốn gán";
        if (_config.CustomToggleVk == 0)
        {
            TxtCustomHotkey.Text = en ? "Click to assign a key" : "Bấm để gán phím";
        }
        TxtUpdateStatus.Text = en ? "Check for application updates" : "Kiểm tra bản cập nhật mới nhất";
        TxtUpdateDetail.Text = en ? "Checks directly from GitHub" : "Kiểm tra trực tiếp từ GitHub";
        BtnCheckUpdate.Content = en ? "Check for updates" : "Kiểm tra cập nhật";
        TxtAboutNote.Text = en
            ? "Thanks for using VietFlow. Press Ctrl + Shift to switch between Vietnamese [V] and English [E]."
            : "Cảm ơn bạn đã sử dụng VietFlow. Nhấn Ctrl + Shift để chuyển nhanh giữa tiếng Việt [V] và English [E].";

        BtnReset.Content = Localization.Get("BtnReset", lang);
        BtnClose.Content = Localization.Get("BtnClose", lang);
        BtnSave.Content = Localization.Get("BtnSave", lang);

        PopulateCharsetOptions(lang);
        PopulateHotkeyOptions(lang);
        PopulateThemeOptions(lang);
        UpdateMacroCount();
        UpdateExcludedAppsCount();
        UpdateKeywordsCount();
    }

    private void UpdateLiveStatusCard()
    {
        if (BdLiveBadge == null || TxtLiveLetter == null || TxtLiveTitle == null || TxtLiveSubtitle == null || SwLiveMode == null)
        {
            return;
        }

        bool en = Localization.IsEnglish(_config.UiLanguage);

        if (_config.IsEnabled)
        {
            ApplyTrayPreview(BdLiveBadge, TxtLiveLetter, _config.TrayVnTop, _config.TrayVnBottom, "#EF4444", "#B91C1C");
            TxtLiveLetter.Text = "V";
            TxtLiveTitle.Text = en ? "Vietnamese" : "Tiếng Việt";
            TxtLiveSubtitle.Text = en ? "Active (Ready)" : "Đang bật (Sẵn sàng)";
            TxtLiveSubtitle.Foreground = new SolidColorBrush(Color.Parse("#16A34A"));
            SwLiveMode.IsChecked = true;
        }
        else
        {
            ApplyTrayPreview(BdLiveBadge, TxtLiveLetter, _config.TrayEnTop, _config.TrayEnBottom, "#2563EB", "#1D4ED8");
            TxtLiveLetter.Text = "E";
            TxtLiveTitle.Text = en ? "English" : "Tiếng Anh";
            TxtLiveSubtitle.Text = en ? "Vietnamese disabled" : "Đã tắt tiếng Việt";
            TxtLiveSubtitle.Foreground = new SolidColorBrush(Color.Parse("#64748B"));
            SwLiveMode.IsChecked = false;
        }

        string hotkey = TrayIconManager.GetHotkeyDisplay(_config.ToggleHotkey, _config.UiLanguage);
        TxtHotkeyBadge.Text = (en ? "Toggle: " : "Chuyển: ") + hotkey;
    }

    private void SwLiveMode_Click(object? sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        _config.IsEnabled = SwLiveMode.IsChecked == true;
        _session.IsEnabled = _config.IsEnabled;
        _config.Save();
        _onStateUpdated?.Invoke();
        UpdateLiveStatusCard();
    }

    private void UpdateCustomHotkeyUi()
    {
        bool custom = CboToggleHotkey.SelectedItem is UiOption<string> { Id: "Custom" };
        RowCustomHotkey.IsVisible = custom;
        if (custom && _config.CustomToggleVk != 0)
        {
            TxtCustomHotkey.Text = FormatHotkey(_config.CustomToggleModifiers, _config.CustomToggleVk);
        }
    }

    private void CboToggleHotkey_SelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (!_loading)
        {
            UpdateCustomHotkeyUi();
        }
    }

    private void CboCharset_SelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (BtnEditCharset != null)
        {
            BtnEditCharset.IsEnabled = CboCharset.SelectedItem is UiOption<CharsetId> { Id: CharsetId.Custom };
        }
    }

    private void CboUiLanguage_SelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (_loading || CboUiLanguage.SelectedItem is not UiOption<string> language)
        {
            return;
        }

        _config.UiLanguage = language.Id;
        ApplyLocalization(language.Id);
        UpdateLiveStatusCard();
    }

    private void CboTheme_SelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (_loading || CboTheme.SelectedItem is not UiOption<string> theme)
        {
            return;
        }

        ThemeManager.Apply(theme.Id);
    }

    private void TxtCustomHotkey_KeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key is Key.LeftCtrl or Key.RightCtrl or Key.LeftShift or Key.RightShift
            or Key.LeftAlt or Key.RightAlt or Key.LWin or Key.RWin)
        {
            e.Handled = true;
            return;
        }

        int vk = KeyToVirtualKey(e.Key);
        if (vk == 0)
        {
            e.Handled = true;
            return;
        }

        int modifiers = 0;
        if (e.KeyModifiers.HasFlag(KeyModifiers.Control)) modifiers |= 1;
        if (e.KeyModifiers.HasFlag(KeyModifiers.Shift)) modifiers |= 2;
        if (e.KeyModifiers.HasFlag(KeyModifiers.Alt)) modifiers |= 4;
        if (e.KeyModifiers.HasFlag(KeyModifiers.Meta)) modifiers |= 8;

        _config.CustomToggleVk = vk;
        _config.CustomToggleModifiers = modifiers;
        TxtCustomHotkey.Text = FormatHotkey(modifiers, vk);
        CboToggleHotkey.SelectedItem = FindOption(_hotkeyOptions, "Custom");
        e.Handled = true;
    }

    private static int KeyToVirtualKey(Key key)
    {
        if (key is >= Key.A and <= Key.Z)
        {
            return 0x41 + ((int)key - (int)Key.A);
        }

        if (key is >= Key.D0 and <= Key.D9)
        {
            return 0x30 + ((int)key - (int)Key.D0);
        }

        if (key is >= Key.F1 and <= Key.F12)
        {
            return 0x70 + ((int)key - (int)Key.F1);
        }

        return key switch
        {
            Key.Space => 0x20,
            Key.Tab => 0x09,
            Key.Enter => 0x0D,
            Key.Escape => 0x1B,
            Key.Oem3 => 0xC0,
            Key.OemMinus => 0xBD,
            Key.OemPlus => 0xBB,
            Key.OemOpenBrackets => 0xDB,
            Key.OemCloseBrackets => 0xDD,
            Key.OemPipe => 0xDC,
            Key.OemSemicolon => 0xBA,
            Key.OemQuotes => 0xDE,
            Key.OemComma => 0xBC,
            Key.OemPeriod => 0xBE,
            Key.OemQuestion => 0xBF,
            _ => 0
        };
    }

    private static string FormatHotkey(int modifiers, int vk)
    {
        var parts = new List<string>();
        if ((modifiers & 1) != 0) parts.Add("Ctrl");
        if ((modifiers & 2) != 0) parts.Add("Shift");
        if ((modifiers & 4) != 0) parts.Add("Alt");
        if ((modifiers & 8) != 0) parts.Add("Win");

        string key = vk switch
        {
            >= 0x41 and <= 0x5A => ((char)vk).ToString(),
            >= 0x30 and <= 0x39 => ((char)vk).ToString(),
            0x20 => "Space",
            0x09 => "Tab",
            0x0D => "Enter",
            0x1B => "Esc",
            _ => $"VK{vk:X2}"
        };

        parts.Add(key);
        return string.Join(" + ", parts);
    }

    private async void BtnEditCharset_Click(object? sender, RoutedEventArgs e)
    {
        var dialog = new CustomCharsetDialog(_config.CustomCharsetMap);
        bool? result = await dialog.ShowDialog<bool?>(this);
        if (result != true)
        {
            return;
        }

        _config.CustomCharsetMap = dialog.Mappings;
        _config.Charset = CharsetId.Custom;
        Charsets.CustomCharset.Instance.SetMappings(_config.CustomCharsetMap);
        CboCharset.SelectedItem = FindCharset(CharsetId.Custom);
        _session.Charset = Charsets.CharsetFactory.GetCharset(CharsetId.Custom);
    }

    private async void BtnCustomizeTray_Click(object? sender, RoutedEventArgs e)
    {
        var dialog = new TrayIconDialog(
            _config.TrayVnTop,
            _config.TrayVnBottom,
            _config.TrayEnTop,
            _config.TrayEnBottom);

        bool? result = await dialog.ShowDialog<bool?>(this);
        if (result != true)
        {
            return;
        }

        _config.TrayVnTop = dialog.VnTop;
        _config.TrayVnBottom = dialog.VnBottom;
        _config.TrayEnTop = dialog.EnTop;
        _config.TrayEnBottom = dialog.EnBottom;
        _config.Save();
        UpdateTrayPreview();
        UpdateLiveStatusCard();
        _onStateUpdated?.Invoke();
    }

    public void RefreshTrayPreview()
    {
        UpdateTrayPreview();
        UpdateLiveStatusCard();
    }

    private void UpdateTrayPreview()
    {
        if (TrayPreviewV == null || TrayPreviewE == null)
        {
            return;
        }

        ApplyTrayPreview(TrayPreviewV, TxtTrayPreviewV, _config.TrayVnTop, _config.TrayVnBottom, "#EF4444", "#B91C1C");
        ApplyTrayPreview(TrayPreviewE, TxtTrayPreviewE, _config.TrayEnTop, _config.TrayEnBottom, "#2563EB", "#1D4ED8");
    }

    private static void ApplyTrayPreview(Border badge, TextBlock letter, string top, string bottom, string fallbackTop, string fallbackBottom)
    {
        try
        {
            Color topColor = Color.Parse(string.IsNullOrWhiteSpace(top) ? fallbackTop : top);
            Color bottomColor = Color.Parse(string.IsNullOrWhiteSpace(bottom) ? fallbackBottom : bottom);

            badge.Background = new LinearGradientBrush
            {
                StartPoint = new RelativePoint(0, 0, RelativeUnit.Relative),
                EndPoint = new RelativePoint(0, 1, RelativeUnit.Relative),
                GradientStops =
                {
                    new GradientStop(topColor, 0),
                    new GradientStop(bottomColor, 1)
                }
            };

            double luminance = (0.299 * (topColor.R + bottomColor.R) +
                                0.587 * (topColor.G + bottomColor.G) +
                                0.114 * (topColor.B + bottomColor.B)) / 2.0;
            letter.Foreground = luminance > 165
                ? new SolidColorBrush(Color.FromRgb(17, 24, 39))
                : Brushes.White;
        }
        catch
        {
            badge.Background = new SolidColorBrush(Colors.Gray);
            letter.Foreground = Brushes.White;
        }
    }

    private async void BtnSave_Click(object? sender, RoutedEventArgs e)
    {
        if (CboCharset.SelectedItem is UiOption<CharsetId> charset)
        {
            _config.Charset = charset.Id;
            _session.Charset = Charsets.CharsetFactory.GetCharset(charset.Id);
        }

        if (CboMethod.SelectedItem is UiOption<EngineInputMethod> method)
        {
            _config.Method = method.Id;
            _session.Method = method.Id;
        }

        if (CboToggleHotkey.SelectedItem is UiOption<string> hotkey)
        {
            _config.ToggleHotkey = hotkey.Id;
        }

        if (CboUiLanguage.SelectedItem is UiOption<string> language)
        {
            _config.UiLanguage = language.Id;
        }

        bool startWithWindows = SwStartWithWindows.IsChecked == true;
        bool startupSuccess = StartupHelper.SetEnabled(startWithWindows);
        if (!startupSuccess)
        {
            bool actual = StartupHelper.IsEnabled();
            _config.StartWithWindows = actual;
            SwStartWithWindows.IsChecked = actual;
            await MessageWindow.ShowInfoAsync(
                this,
                "VietFlow",
                Localization.IsEnglish(_config.UiLanguage)
                    ? "Could not update the Windows startup setting. Try running VietFlow normally (without administrator rights) and enable it again."
                    : "Không thể cập nhật khởi động cùng Windows. Hãy chạy VietFlow bình thường (không cần quyền Admin) rồi bật lại.");
        }
        else
        {
            _config.StartWithWindows = startWithWindows;
        }

        _config.FreeMarking = SwFreeMarking.IsChecked == true;
        _config.SpellCheck = SwSpellCheck.IsChecked == true;
        _config.AutoNonVnRestore = SwAutoRestore.IsChecked == true;
        _config.ModernStyle = SwModernStyle.IsChecked == true;

        _config.FixShiftLag = SwFixShiftLag.IsChecked == true;
        _config.ConsonantShortcuts = SwConsonantShortcuts.IsChecked == true;
        _config.UseClipboardForUnicode = SwUseClipboardForUnicode.IsChecked == true;
        _config.AutoCapitalize = SwAutoCapitalize.IsChecked == true;
        if (CboTypingTimeout.SelectedItem is UiOption<int> timeout)
        {
            _config.TypingTimeoutMs = timeout.Id;
        }
        _config.AutoCheckUpdates = SwAutoCheckUpdates.IsChecked == true;
        _config.NotificationSound = SwNotificationSound.IsChecked == true;

        if (CboTheme.SelectedItem is UiOption<string> theme)
        {
            _config.ThemeMode = theme.Id;
            ThemeManager.Apply(theme.Id);
        }

        _config.EnableMacro = SwEnableMacro.IsChecked == true;
        _config.AllowMacroWhenDisabled = SwMacroWhenDisabled.IsChecked == true;

        _config.EnglishKeywords = ParseKeywords(TxtEnglishKeywords.Text ?? string.Empty);
        _config.ForeignInitials = string.IsNullOrWhiteSpace(TxtForeignInitials.Text) ? "fjwz" : TxtForeignInitials.Text.Trim();
        _config.MacroDateFormat = string.IsNullOrWhiteSpace(TxtMacroDateFormat.Text) ? "dd/MM/yyyy" : TxtMacroDateFormat.Text.Trim();
        _config.MacroTimeFormat = string.IsNullOrWhiteSpace(TxtMacroTimeFormat.Text) ? "HH:mm" : TxtMacroTimeFormat.Text.Trim();
        _config.MacroDateTimeFormat = string.IsNullOrWhiteSpace(TxtMacroDateTimeFormat.Text) ? "dd/MM/yyyy HH:mm" : TxtMacroDateTimeFormat.Text.Trim();
        _config.MacroVariables = ParseMacroVariables(TxtMacroVariables.Text ?? string.Empty);

        _config.Macros.Clear();
        foreach (var item in _macroItems)
        {
            if (!string.IsNullOrWhiteSpace(item.Key))
            {
                _config.Macros[item.Key.Trim()] = item.Value ?? string.Empty;
            }
        }

        _config.ExcludedApps.Clear();
        foreach (var app in _excludedAppItems)
        {
            if (!string.IsNullOrWhiteSpace(app.Exe))
            {
                _config.ExcludedApps.Add(app.Exe.Trim());
            }
        }

        _config.Save();
        _config.ApplyToSession(_session);
        _onStateUpdated?.Invoke();

        UpdateLiveStatusCard();
        Close();
    }

    private void BtnReset_Click(object? sender, RoutedEventArgs e)
    {
        var def = new VietFlowConfig();
        _config.Method = def.Method;
        _config.Charset = def.Charset;
        _config.ToggleHotkey = def.ToggleHotkey;
        _config.UiLanguage = def.UiLanguage;
        _config.CustomToggleVk = def.CustomToggleVk;
        _config.CustomToggleModifiers = def.CustomToggleModifiers;
        _config.FreeMarking = def.FreeMarking;
        _config.SpellCheck = def.SpellCheck;
        _config.AutoNonVnRestore = def.AutoNonVnRestore;
        _config.ModernStyle = def.ModernStyle;
        _config.FixShiftLag = def.FixShiftLag;
        _config.ConsonantShortcuts = def.ConsonantShortcuts;
        _config.EnableMacro = def.EnableMacro;
        _config.AllowMacroWhenDisabled = def.AllowMacroWhenDisabled;
        _config.ThemeMode = def.ThemeMode;
        _config.NotificationSound = def.NotificationSound;
        _config.AutoCapitalize = def.AutoCapitalize;
        _config.TypingTimeoutMs = def.TypingTimeoutMs;
        _config.TrayVnTop = def.TrayVnTop;
        _config.TrayVnBottom = def.TrayVnBottom;
        _config.TrayEnTop = def.TrayEnTop;
        _config.TrayEnBottom = def.TrayEnBottom;
        _config.EnglishKeywords.Clear();
        _config.ForeignInitials = def.ForeignInitials;
        _config.MacroDateFormat = def.MacroDateFormat;
        _config.MacroTimeFormat = def.MacroTimeFormat;
        _config.MacroDateTimeFormat = def.MacroDateTimeFormat;
        _config.MacroVariables = def.MacroVariables;
        _config.CustomCharsetMap = def.CustomCharsetMap;
        _config.ExcludedApps = def.ExcludedApps;
        _config.Macros = def.Macros;

        LoadConfigData();
    }

    private void RefreshMacroView()
    {
        if (DgMacros == null)
        {
            return;
        }

        string query = TxtMacroSearch.Text?.Trim() ?? string.Empty;
        var filtered = string.IsNullOrEmpty(query)
            ? _macroItems.ToList()
            : _macroItems.Where(item =>
                    item.Key.Contains(query, StringComparison.OrdinalIgnoreCase) ||
                    item.Value.Contains(query, StringComparison.OrdinalIgnoreCase))
                .ToList();

        var selected = DgMacros.SelectedItem as MacroItem;
        DgMacros.ItemsSource = filtered;
        if (selected != null && filtered.Contains(selected))
        {
            DgMacros.SelectedItem = selected;
        }

        UpdateMacroCount();
        UpdateMacroListHeight();
    }

    private void UpdateMacroListHeight()
    {
        if (DgMacros == null)
        {
            return;
        }

        int count = DgMacros.ItemsSource?.Cast<object>().Count() ?? _macroItems.Count;
        DgMacros.Height = Math.Clamp((count * 46) + 10, 106, 310);
    }

    private void TxtMacroSearch_TextChanged(object? sender, TextChangedEventArgs e) => RefreshMacroView();

    private void UpdateMacroCount()
    {
        if (TxtMacroCount == null)
        {
            return;
        }

        int visible = DgMacros?.ItemsSource?.Cast<object>().Count() ?? _macroItems.Count;
        TxtMacroCount.Text = Localization.IsEnglish(_config.UiLanguage)
            ? $"Showing: {visible} macros (Total: {_macroItems.Count})"
            : $"Hiển thị: {visible} từ gõ tắt (Tổng: {_macroItems.Count})";
    }

    private void UpdateExcludedAppsCount()
    {
        if (TxtExcludedAppsCount != null)
        {
            TxtExcludedAppsCount.Text = Localization.IsEnglish(_config.UiLanguage)
                ? $"Excluded applications: {_excludedAppItems.Count}"
                : $"Tổng số ứng dụng loại trừ: {_excludedAppItems.Count}";
        }
    }

    private void DgMacros_SelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (DgMacros.SelectedItem is MacroItem item)
        {
            TxtMacroKey.Text = item.Key;
            TxtMacroValue.Text = item.Value;
        }
    }

    private void AddOrUpdateMacro(string key, string value)
    {
        if (string.IsNullOrEmpty(key))
        {
            return;
        }

        var existing = _macroItems.FirstOrDefault(item => string.Equals(item.Key, key, StringComparison.OrdinalIgnoreCase));
        if (existing != null)
        {
            existing.Value = value;
        }
        else
        {
            _macroItems.Add(new MacroItem { Key = key, Value = value });
        }
    }

    private void BtnAddMacro_Click(object? sender, RoutedEventArgs e)
    {
        string key = TxtMacroKey.Text?.Trim() ?? string.Empty;
        string value = TxtMacroValue.Text?.Trim() ?? string.Empty;
        if (string.IsNullOrEmpty(key))
        {
            return;
        }

        AddOrUpdateMacro(key, value);
        RefreshMacroView();

        TxtMacroKey.Text = string.Empty;
        TxtMacroValue.Text = string.Empty;
    }

    private void BtnDeleteMacro_Click(object? sender, RoutedEventArgs e)
    {
        if (DgMacros.SelectedItem is MacroItem item)
        {
            _macroItems.Remove(item);
            RefreshMacroView();
        }
    }

    private void BtnAddSampleMacros_Click(object? sender, RoutedEventArgs e)
    {
        var samples = new (string Key, string Value)[]
        {
            ("vn", "Việt Nam"),
            ("hn", "Hà Nội"),
            ("hcm", "TP. Hồ Chí Minh"),
            ("dc", "được"),
            ("ko", "không"),
            ("ng", "người"),
            ("tg", "thời gian"),
            ("clp", "@clipboard"),
            ("ngay", "@date"),
            ("gio", "@time")
        };

        foreach (var (key, value) in samples)
        {
            AddOrUpdateMacro(key, value);
        }

        RefreshMacroView();
    }

    private async void BtnExportMacros_Click(object? sender, RoutedEventArgs e)
    {
        var file = await StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Xuất bảng gõ tắt",
            SuggestedFileName = "VietFlow_Macros.txt",
            FileTypeChoices = new[]
            {
                new FilePickerFileType("File văn bản (*.txt)") { Patterns = new[] { "*.txt" } }
            }
        });

        if (file == null)
        {
            return;
        }

        var builder = new StringBuilder();
        builder.AppendLine("# Bảng gõ tắt VietFlow");
        foreach (var item in _macroItems)
        {
            builder.AppendLine($"{item.Key}:{item.Value}");
        }

        await using var stream = await file.OpenWriteAsync();
        await using var writer = new StreamWriter(stream, Encoding.UTF8);
        await writer.WriteAsync(builder.ToString());
    }

    private async void BtnImportMacros_Click(object? sender, RoutedEventArgs e)
    {
        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Nhập bảng gõ tắt",
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType("File văn bản (*.txt)") { Patterns = new[] { "*.txt" } }
            }
        });

        if (files.Count == 0)
        {
            return;
        }

        await using var stream = await files[0].OpenReadAsync();
        using var reader = new StreamReader(stream, Encoding.UTF8);
        while (await reader.ReadLineAsync() is { } line)
        {
            string trimmed = line.Trim();
            if (string.IsNullOrWhiteSpace(trimmed) || trimmed.StartsWith('#'))
            {
                continue;
            }

            int colonIndex = trimmed.IndexOf(':');
            if (colonIndex > 0)
            {
                AddOrUpdateMacro(trimmed[..colonIndex].Trim(), trimmed[(colonIndex + 1)..].Trim());
                continue;
            }

            int tabIndex = trimmed.IndexOf('\t');
            if (tabIndex > 0)
            {
                AddOrUpdateMacro(trimmed[..tabIndex].Trim(), trimmed[(tabIndex + 1)..].Trim());
            }
        }

        RefreshMacroView();
    }

    private async void BtnClearMacros_Click(object? sender, RoutedEventArgs e)
    {
        bool confirmed = await MessageWindow.ConfirmAsync(
            this,
            "Xác nhận",
            "Bạn có chắc chắn muốn xóa toàn bộ danh sách từ gõ tắt không?");

        if (!confirmed)
        {
            return;
        }

        _macroItems.Clear();
        RefreshMacroView();
    }

    private void BtnAddApp_Click(object? sender, RoutedEventArgs e)
    {
        string app = TxtAppProcess.Text?.Trim() ?? string.Empty;
        if (string.IsNullOrEmpty(app))
        {
            return;
        }

        if (!app.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
        {
            app += ".exe";
        }

        var info = ProcessInfo.FromExe(app);
        if (!_excludedAppItems.Any(item => string.Equals(item.Exe, info.Exe, StringComparison.OrdinalIgnoreCase)))
        {
            _excludedAppItems.Add(info);
            UpdateExcludedAppsCount();
        }

        TxtAppProcess.Text = string.Empty;
    }

    private async void BtnPickRunningProcess_Click(object? sender, RoutedEventArgs e)
    {
        var picker = new ProcessPickerWindow(_config.UiLanguage);
        ProcessInfo? selected = await picker.ShowDialog<ProcessInfo?>(this);
        if (selected == null)
        {
            return;
        }

        if (!_excludedAppItems.Any(item => string.Equals(item.Exe, selected.Exe, StringComparison.OrdinalIgnoreCase)))
        {
            _excludedAppItems.Add(selected);
            UpdateExcludedAppsCount();
        }
    }

    private async void BtnBrowseExe_Click(object? sender, RoutedEventArgs e)
    {
        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Chọn file ứng dụng hoặc game để loại trừ",
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType("File thực thi (*.exe)") { Patterns = new[] { "*.exe" } }
            }
        });

        if (files.Count == 0)
        {
            return;
        }

        string exeName = files[0].Name;
        string path = files[0].Path.LocalPath;
        var info = ProcessInfo.FromPath(exeName, path);
        if (!_excludedAppItems.Any(item => string.Equals(item.Exe, info.Exe, StringComparison.OrdinalIgnoreCase)))
        {
            _excludedAppItems.Add(info);
            UpdateExcludedAppsCount();
        }
    }

    private void BtnClearExcludedApps_Click(object? sender, RoutedEventArgs e)
    {
        _excludedAppItems.Clear();
        UpdateExcludedAppsCount();
    }

    private void BtnDeleteApp_Click(object? sender, RoutedEventArgs e)
    {
        if (LstExcludedApps.SelectedItem is ProcessInfo app)
        {
            _excludedAppItems.Remove(app);
            UpdateExcludedAppsCount();
        }
    }

    public void RefreshLiveState() => UpdateLiveStatusCard();

    public void ShowTab(string tab)
    {
        Tabs.SelectedItem = tab.ToLowerInvariant() switch
        {
            "typing" => TabTyping,
            "macro" => TabMacro,
            "game" => TabGame,
            "about" => TabAbout,
            "advanced" => TabAdvanced,
            _ => TabOverview
        };
    }

    public void CaptureTo(string path)
    {
        UpdateLayout();
        int width = Math.Max(1, (int)Math.Ceiling(Bounds.Width));
        int height = Math.Max(1, (int)Math.Ceiling(Bounds.Height));
        using var bitmap = new RenderTargetBitmap(new PixelSize(width, height), new Vector(96, 96));
        bitmap.Render(this);
        using var stream = File.Create(path);
        bitmap.Save(stream, PngBitmapEncoderOptions.Default);
    }

    private async void BtnCheckUpdate_Click(object? sender, RoutedEventArgs e)
    {
        if (_updateService.Status == UpdateStatus.UpdateAvailable)
        {
            BtnCheckUpdate.IsEnabled = false;
            PrgUpdate.IsVisible = true;
            PrgUpdate.IsIndeterminate = false;
            PrgUpdate.Value = 0;

            bool success = await _updateService.DownloadAndInstallUpdateAsync(progress =>
            {
                Dispatcher.UIThread.Post(() => PrgUpdate.Value = progress);
            });

            BtnCheckUpdate.IsEnabled = true;
            if (success)
            {
                BtnCheckUpdate.Content = "Khởi động lại ngay";
                TxtUpdateStatus.Text = "Bản cập nhật đã tải xong!";
                TxtUpdateDetail.Text = "Nhấn 'Khởi động lại ngay' để áp dụng phiên bản mới.";
            }
            return;
        }

        if (_updateService.Status == UpdateStatus.ReadyToRestart)
        {
            _updateService.RestartAppToApplyUpdate();
            return;
        }

        BtnCheckUpdate.IsEnabled = false;
        PrgUpdate.IsVisible = true;
        PrgUpdate.IsIndeterminate = true;
        TxtUpdateStatus.Text = "Đang kiểm tra bản phát hành trên GitHub...";
        TxtUpdateDetail.Text = "Vui lòng chờ giây lát...";

        var update = await _updateService.CheckForUpdatesAsync(silent: false);

        PrgUpdate.IsIndeterminate = false;
        BtnCheckUpdate.IsEnabled = true;

        if (update != null)
        {
            PrgUpdate.IsVisible = false;
            TxtUpdateStatus.Text = $"Đã có phiên bản mới: v{update.TargetFullRelease.Version}";
            TxtUpdateDetail.Text = "Có bản cập nhật mới sẵn sàng tải về máy của bạn.";
            BtnCheckUpdate.Content = "Tải & Cài đặt";
        }
        else
        {
            PrgUpdate.IsVisible = false;
            if (_updateService.Status == UpdateStatus.UpToDate)
            {
                TxtUpdateStatus.Text = $"VietFlow đã là bản mới nhất ({_updateService.CurrentVersionText})";
                TxtUpdateDetail.Text = "Hệ thống của bạn đang sử dụng bản phát hành mới nhất.";
                BtnCheckUpdate.Content = "Kiểm tra lại";
            }
            else
            {
                TxtUpdateStatus.Text = "Không thể kiểm tra cập nhật";
                TxtUpdateDetail.Text = _updateService.StatusMessage;
                BtnCheckUpdate.Content = "Thử lại";
            }
        }
    }

    private static Dictionary<string, string> ParseMacroVariables(string text)
    {
        var result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (string rawLine in text.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries))
        {
            string line = rawLine.Trim();
            if (line.Length == 0 || line.StartsWith('#'))
            {
                continue;
            }

            int separator = line.IndexOf('=');
            if (separator <= 0)
            {
                separator = line.IndexOf(':');
            }

            if (separator <= 0)
            {
                continue;
            }

            string key = line[..separator].Trim().TrimStart('@');
            string value = line[(separator + 1)..].Trim();
            if (key.Length > 0)
            {
                result[key] = value;
            }
        }

        return result;
    }

    private static List<string> ParseKeywords(string text)
    {
        return text
            .Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries)
            .Select(line => line.Trim())
            .Where(line => line.Length > 0)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToList();
    }

    private void UpdateKeywordsCount()
    {
        if (TxtKeywordsCount != null && TxtEnglishKeywords != null)
        {
            int count = ParseKeywords(TxtEnglishKeywords.Text ?? string.Empty).Count;
            TxtKeywordsCount.Text = Localization.IsEnglish(_config.UiLanguage)
                ? $"{count} words"
                : $"{count} từ";
        }
    }

    private void TxtEnglishKeywords_TextChanged(object? sender, TextChangedEventArgs e) => UpdateKeywordsCount();

    private void BtnSaveKeywords_Click(object? sender, RoutedEventArgs e)
    {
        var words = ParseKeywords(TxtEnglishKeywords.Text ?? string.Empty);
        _config.EnglishKeywords = words;
        EnglishPhonetics.SetKeywords(words.Count > 0 ? words : EnglishPhonetics.DefaultKeywords);
        _config.Save();
        UpdateKeywordsCount();
    }

    private void BtnResetKeywords_Click(object? sender, RoutedEventArgs e)
    {
        _config.EnglishKeywords.Clear();
        TxtEnglishKeywords.Text = string.Join(Environment.NewLine, EnglishPhonetics.DefaultKeywords);
        EnglishPhonetics.SetKeywords(EnglishPhonetics.DefaultKeywords);
        _config.Save();
        UpdateKeywordsCount();
    }

    private void BtnOpenConfigFolder_Click(object? sender, RoutedEventArgs e)
    {
        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = "explorer.exe",
                Arguments = $"\"{VietFlowConfig.GetConfigDirectory()}\"",
                UseShellExecute = true
            });
        }
        catch
        {
            // Ignore shell failures.
        }
    }

    private static void OpenUrl(string url)
    {
        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = url,
                UseShellExecute = true
            });
        }
        catch
        {
            // Ignore shell failures.
        }
    }

    private void GitHubLink_Click(object? sender, RoutedEventArgs e) =>
        OpenUrl("https://github.com/KienPC1234/VietFlow");

    private void UniKeyLink_Click(object? sender, RoutedEventArgs e) =>
        OpenUrl("https://www.unikey.org");

    private void EvKeyLink_Click(object? sender, RoutedEventArgs e) =>
        OpenUrl("https://evkeyvn.com");

    private void BtnClose_Click(object? sender, RoutedEventArgs e) => Close();
}

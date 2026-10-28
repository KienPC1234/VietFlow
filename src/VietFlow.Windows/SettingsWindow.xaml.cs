// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Collections.ObjectModel;
using System.ComponentModel;
using System.IO;
using System.Reflection;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Input;
using System.Windows.Media;
using VietFlow.Configuration;
using VietFlow.Engine;
using EngineInputMethod = VietFlow.Engine.InputMethod;
using MediaColor = System.Windows.Media.Color;

namespace VietFlow.Windows;

public partial class SettingsWindow : Window
{
    private readonly VietFlowConfig _config;
    private readonly VietFlowSession _session;
    private readonly Action _onStateUpdated;

    public sealed class MacroItem
    {
        public required string Key { get; set; }
        public required string Value { get; set; }
    }

    private readonly ObservableCollection<MacroItem> _macroItems = new();
    private readonly ObservableCollection<string> _excludedAppItems = new();
    private ICollectionView _macroCollectionView = null!;

    public SettingsWindow(VietFlowConfig config, VietFlowSession session, Action onStateUpdated)
    {
        InitializeComponent();
        _config = config;
        _session = session;
        _onStateUpdated = onStateUpdated;

        PopulateControls();
        LoadConfigData();
        UpdateLiveStatusCard();
        UpdateMacroCount();
        UpdateExcludedAppsCount();

        var version = typeof(SettingsWindow).Assembly.GetName().Version;
        var informational = typeof(SettingsWindow).Assembly
            .GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion;
        TxtAboutVersion.Text = $"Phiên bản {informational ?? version?.ToString(3) ?? "0.1.0"}";
        TxtAboutRuntime.Text = $".NET {Environment.Version} · WPF · Win32 low-level keyboard hook";
    }

    private void PopulateControls()
    {
        CboCharset.ItemsSource = new[]
        {
            new { Id = CharsetId.Unicode, Name = "1. Unicode dựng sẵn (NFC)" },
            new { Id = CharsetId.UnicodeComposite, Name = "2. Unicode tổ hợp (NFD)" },
            new { Id = CharsetId.Tcvn3, Name = "3. TCVN3 (ABC)" },
            new { Id = CharsetId.VniWindows, Name = "4. VNI Windows" },
            new { Id = CharsetId.Custom, Name = "5. Tùy chỉnh (Custom)" }
        };
        CboCharset.DisplayMemberPath = "Name";
        CboCharset.SelectedValuePath = "Id";

        CboMethod.ItemsSource = new[]
        {
            new { Id = EngineInputMethod.Telex, Name = "Telex" },
            new { Id = EngineInputMethod.SimpleTelex, Name = "Simple Telex" },
            new { Id = EngineInputMethod.Vni, Name = "VNI" },
            new { Id = EngineInputMethod.MicrosoftVi, Name = "Microsoft Vietnamese" },
            new { Id = EngineInputMethod.Viqr, Name = "VIQR" }
        };
        CboMethod.DisplayMemberPath = "Name";
        CboMethod.SelectedValuePath = "Id";

        CboToggleHotkey.ItemsSource = new[]
        {
            new { Id = "CtrlShift", Name = "Ctrl + Shift" },
            new { Id = "AltZ", Name = "Alt + Z" },
            new { Id = "CtrlSpace", Name = "Ctrl + Space" },
            new { Id = "ShiftSpace", Name = "Shift + Space" },
            new { Id = "AltShift", Name = "Alt + Shift" },
            new { Id = "WinSpace", Name = "Win + Space" },
            new { Id = "CapsLock", Name = "Caps Lock" },
            new { Id = "RightCtrl", Name = "Right Control" },
            new { Id = "Custom", Name = "Phím tự gán (Custom)..." }
        };
        CboToggleHotkey.DisplayMemberPath = "Name";
        CboToggleHotkey.SelectedValuePath = "Id";

        CboUiLanguage.ItemsSource = new[]
        {
            new { Id = "vi", Name = "Tiếng Việt" },
            new { Id = "en", Name = "English" }
        };
        CboUiLanguage.DisplayMemberPath = "Name";
        CboUiLanguage.SelectedValuePath = "Id";

        CboTheme.ItemsSource = new[]
        {
            new { Id = ThemeManager.SystemMode, Name = "Theo hệ thống" },
            new { Id = ThemeManager.LightMode, Name = "Sáng (Light)" },
            new { Id = ThemeManager.DarkMode, Name = "Tối (Dark)" }
        };
        CboTheme.DisplayMemberPath = "Name";
        CboTheme.SelectedValuePath = "Id";

        DgMacros.ItemsSource = _macroItems;
        _macroCollectionView = CollectionViewSource.GetDefaultView(_macroItems);
        _macroCollectionView.Filter = FilterMacro;

        LstExcludedApps.ItemsSource = _excludedAppItems;
    }

    private bool FilterMacro(object obj)
    {
        if (obj is not MacroItem item) return false;
        string query = TxtMacroSearch.Text.Trim();
        if (string.IsNullOrEmpty(query)) return true;

        return item.Key.Contains(query, StringComparison.OrdinalIgnoreCase) ||
               (item.Value != null && item.Value.Contains(query, StringComparison.OrdinalIgnoreCase));
    }

    private void TxtMacroSearch_TextChanged(object sender, TextChangedEventArgs e)
    {
        _macroCollectionView?.Refresh();
        UpdateMacroCount();
    }

    private void UpdateMacroCount()
    {
        if (_macroCollectionView != null && TxtMacroCount != null)
        {
            int visible = _macroCollectionView.Cast<object>().Count();
            TxtMacroCount.Text = $"Hiển thị: {visible} từ gõ tắt (Tổng: {_macroItems.Count})";
        }
    }

    private void UpdateExcludedAppsCount()
    {
        if (TxtExcludedAppsCount != null)
        {
            TxtExcludedAppsCount.Text = $"Tổng số ứng dụng loại trừ: {_excludedAppItems.Count}";
        }
    }

    private void LoadConfigData()
    {
        CboCharset.SelectedValue = _config.Charset;
        CboMethod.SelectedValue = _config.Method;

        CboToggleHotkey.SelectedValue = string.IsNullOrEmpty(_config.ToggleHotkey) ? "CtrlShift" : _config.ToggleHotkey;
        CboUiLanguage.SelectedValue = string.IsNullOrEmpty(_config.UiLanguage) ? "vi" : _config.UiLanguage;
        CboTheme.SelectedValue = ThemeManager.Normalize(_config.ThemeMode);
        UpdateCustomHotkeyUi();
        ApplyLocalization(_config.UiLanguage);

        SwStartWithWindows.IsChecked = TaskSchedulerHelper.IsScheduledToRunAtLogon();
        SwNotificationSound.IsChecked = _config.NotificationSound;

        SwFreeMarking.IsChecked = _config.FreeMarking;
        SwSpellCheck.IsChecked = _config.SpellCheck;
        SwAutoRestore.IsChecked = _config.AutoNonVnRestore;
        SwModernStyle.IsChecked = _config.ModernStyle;

        SwFixShiftLag.IsChecked = _config.FixShiftLag;
        SwConsonantShortcuts.IsChecked = _config.ConsonantShortcuts;
        SwUseClipboardForUnicode.IsChecked = _config.UseClipboardForUnicode;
        SwAutoCapitalize.IsChecked = _config.AutoCapitalize;

        SwEnableMacro.IsChecked = _config.EnableMacro;
        SwMacroWhenDisabled.IsChecked = _config.AllowMacroWhenDisabled;

        var keywords = _config.EnglishKeywords.Count > 0 ? _config.EnglishKeywords : EnglishPhonetics.DefaultKeywords;
        TxtEnglishKeywords.Text = string.Join(Environment.NewLine, keywords);
        TxtForeignInitials.Text = string.IsNullOrWhiteSpace(_config.ForeignInitials) ? "fjwz" : _config.ForeignInitials;
        TxtConfigPath.Text = VietFlowConfig.GetConfigFilePath();
        UpdateKeywordsCount();

        _macroItems.Clear();
        foreach (var (k, v) in _config.Macros)
        {
            _macroItems.Add(new MacroItem { Key = k, Value = v });
        }

        _excludedAppItems.Clear();
        foreach (var app in _config.ExcludedApps)
        {
            _excludedAppItems.Add(app);
        }

        UpdateLiveStatusCard();
        UpdateMacroCount();
        UpdateExcludedAppsCount();
    }

    private void CboToggleHotkey_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        UpdateCustomHotkeyUi();
    }

    private void CboCharset_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (BtnEditCharset != null)
        {
            BtnEditCharset.IsEnabled = CboCharset.SelectedValue is CharsetId.Custom;
        }
    }

    private void BtnEditCharset_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new CustomCharsetDialog(_config.CustomCharsetMap)
        {
            Owner = this
        };

        if (dialog.ShowDialog() != true)
        {
            return;
        }

        _config.CustomCharsetMap = dialog.Mappings;
        _config.Charset = CharsetId.Custom;
        Charsets.CustomCharset.Instance.SetMappings(_config.CustomCharsetMap);
        CboCharset.SelectedValue = CharsetId.Custom;
        _session.Charset = Charsets.CharsetFactory.GetCharset(CharsetId.Custom);
    }

    private void UpdateCustomHotkeyUi()
    {
        string? val = CboToggleHotkey?.SelectedValue as string;
        if (RowCustomHotkey != null)
        {
            RowCustomHotkey.Visibility = val == "Custom" ? Visibility.Visible : Visibility.Collapsed;
        }

        if (TxtCustomHotkey != null)
        {
            if (_config.CustomToggleVk > 0)
            {
                var parts = new List<string>();
                if ((_config.CustomToggleModifiers & 2) != 0) parts.Add("Ctrl");
                if ((_config.CustomToggleModifiers & 4) != 0) parts.Add("Alt");
                if ((_config.CustomToggleModifiers & 1) != 0) parts.Add("Shift");
                Key key = KeyInterop.KeyFromVirtualKey(_config.CustomToggleVk);
                parts.Add(key.ToString());
                TxtCustomHotkey.Text = string.Join(" + ", parts);
            }
            else
            {
                TxtCustomHotkey.Text = _config.UiLanguage == "en" ? "Click to assign hotkey" : "Bấm để gán phím";
            }
        }

        UpdateHotkeyBadge();
    }

    private void TxtCustomHotkey_PreviewKeyDown(object sender, System.Windows.Input.KeyEventArgs e)
    {
        e.Handled = true;
        var key = e.Key == Key.System ? e.SystemKey : e.Key;
        if (key is Key.LeftCtrl or Key.RightCtrl or Key.LeftAlt or Key.RightAlt or Key.LeftShift or Key.RightShift or Key.LWin or Key.RWin)
        {
            return;
        }

        int modifiers = 0;
        var parts = new List<string>();
        if ((Keyboard.Modifiers & System.Windows.Input.ModifierKeys.Control) != 0)
        {
            modifiers |= 2;
            parts.Add("Ctrl");
        }
        if ((Keyboard.Modifiers & System.Windows.Input.ModifierKeys.Alt) != 0)
        {
            modifiers |= 4;
            parts.Add("Alt");
        }
        if ((Keyboard.Modifiers & System.Windows.Input.ModifierKeys.Shift) != 0)
        {
            modifiers |= 1;
            parts.Add("Shift");
        }

        int vk = KeyInterop.VirtualKeyFromKey(key);
        parts.Add(key.ToString());

        _config.CustomToggleVk = vk;
        _config.CustomToggleModifiers = modifiers;
        TxtCustomHotkey.Text = string.Join(" + ", parts);
        UpdateHotkeyBadge();
    }

    private void UpdateHotkeyBadge()
    {
        if (TxtHotkeyBadge == null) return;
        string prefix = _config.UiLanguage == "en" ? "Toggle: " : "Chuyển: ";
        string hotkey = _config.ToggleHotkey switch
        {
            "AltZ" => "Alt + Z",
            "CtrlSpace" => "Ctrl + Space",
            "ShiftSpace" => "Shift + Space",
            "AltShift" => "Alt + Shift",
            "WinSpace" => "Win + Space",
            "CapsLock" => "Caps Lock",
            "RightCtrl" => "Right Ctrl",
            "Custom" => TxtCustomHotkey?.Text ?? "Custom",
            _ => "Ctrl + Shift"
        };
        TxtHotkeyBadge.Text = prefix + hotkey;
    }

    private void CboUiLanguage_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (CboUiLanguage.SelectedValue is string lang)
        {
            _config.UiLanguage = lang;
            ApplyLocalization(lang);
            UpdateHotkeyBadge();
            UpdateLiveStatusCard();
        }
    }

    private void ApplyLocalization(string lang)
    {
        NavOverview.Content = Localization.Get("NavOverview", lang);
        NavTyping.Content = Localization.Get("NavTyping", lang);
        NavMacro.Content = Localization.Get("NavMacro", lang);
        NavGame.Content = Localization.Get("NavGame", lang);

        if (LblOverviewTitle != null) LblOverviewTitle.Text = Localization.Get("OverviewTitle", lang);
        if (LblOverviewDesc != null) LblOverviewDesc.Text = Localization.Get("OverviewDesc", lang);
        if (LblSectionInput != null) LblSectionInput.Text = Localization.Get("SectionInput", lang);
        if (LblCharsetTitle != null) LblCharsetTitle.Text = Localization.Get("Charset", lang);
        if (LblCharsetDesc != null) LblCharsetDesc.Text = Localization.Get("CharsetDesc", lang);
        if (LblMethodTitle != null) LblMethodTitle.Text = Localization.Get("Method", lang);
        if (LblMethodDesc != null) LblMethodDesc.Text = Localization.Get("MethodDesc", lang);
        if (LblSwitchKeyTitle != null) LblSwitchKeyTitle.Text = Localization.Get("SwitchKey", lang);
        if (LblSwitchKeyDesc != null) LblSwitchKeyDesc.Text = Localization.Get("SwitchKeyDesc", lang);
        if (LblUiLangTitle != null) LblUiLangTitle.Text = Localization.Get("UiLanguage", lang);
        if (LblUiLangDesc != null) LblUiLangDesc.Text = Localization.Get("UiLanguageDesc", lang);
        if (LblSectionSystem != null) LblSectionSystem.Text = Localization.Get("SectionSystem", lang);
        if (LblStartWithWinTitle != null) LblStartWithWinTitle.Text = Localization.Get("StartWithWin", lang);
        if (LblStartWithWinDesc != null) LblStartWithWinDesc.Text = Localization.Get("StartWithWinDesc", lang);

        if (LblTypingTitle != null) LblTypingTitle.Text = Localization.Get("TypingTitle", lang);
        if (LblTypingDesc != null) LblTypingDesc.Text = Localization.Get("TypingDesc", lang);
        if (LblShiftLagTitle != null) LblShiftLagTitle.Text = Localization.Get("ShiftLag", lang);
        if (LblShiftLagDesc != null) LblShiftLagDesc.Text = Localization.Get("ShiftLagDesc", lang);
        if (LblConsonantTitle != null) LblConsonantTitle.Text = Localization.Get("ConsonantShortcuts", lang);
        if (LblConsonantDesc != null) LblConsonantDesc.Text = Localization.Get("ConsonantShortcutsDesc", lang);
        if (LblModernStyleTitle != null) LblModernStyleTitle.Text = Localization.Get("ModernStyle", lang);
        if (LblModernStyleDesc != null) LblModernStyleDesc.Text = Localization.Get("ModernStyleDesc", lang);
        if (LblFreeMarkingTitle != null) LblFreeMarkingTitle.Text = Localization.Get("FreeMarking", lang);
        if (LblFreeMarkingDesc != null) LblFreeMarkingDesc.Text = Localization.Get("FreeMarkingDesc", lang);
        if (LblSpellCheckTitle != null) LblSpellCheckTitle.Text = Localization.Get("SpellCheck", lang);
        if (LblSpellCheckDesc != null) LblSpellCheckDesc.Text = Localization.Get("SpellCheckDesc", lang);
        if (LblAutoRestoreTitle != null) LblAutoRestoreTitle.Text = Localization.Get("AutoRestore", lang);
        if (LblAutoRestoreDesc != null) LblAutoRestoreDesc.Text = Localization.Get("AutoRestoreDesc", lang);
        if (LblUseClipboardTitle != null) LblUseClipboardTitle.Text = Localization.Get("UseClipboard", lang);
        if (LblUseClipboardDesc != null) LblUseClipboardDesc.Text = Localization.Get("UseClipboardDesc", lang);

        if (LblMacroTitle != null) LblMacroTitle.Text = Localization.Get("MacroTitle", lang);
        if (LblMacroDesc != null) LblMacroDesc.Text = Localization.Get("MacroDesc", lang);
        if (LblEnableMacroTitle != null) LblEnableMacroTitle.Text = Localization.Get("EnableMacro", lang);
        if (LblEnableMacroDesc != null) LblEnableMacroDesc.Text = Localization.Get("EnableMacroDesc", lang);
        if (LblMacroWhenDisabledTitle != null) LblMacroWhenDisabledTitle.Text = Localization.Get("MacroWhenDisabled", lang);
        if (LblMacroWhenDisabledDesc != null) LblMacroWhenDisabledDesc.Text = Localization.Get("MacroWhenDisabledDesc", lang);

        if (BtnAddSampleMacros != null) BtnAddSampleMacros.Content = Localization.Get("BtnAddSample", lang);
        if (BtnImportMacros != null) BtnImportMacros.Content = Localization.Get("BtnImport", lang);
        if (BtnExportMacros != null) BtnExportMacros.Content = Localization.Get("BtnExport", lang);
        if (BtnClearMacros != null) BtnClearMacros.Content = Localization.Get("BtnClearAll", lang);
        if (BtnAddMacro != null) BtnAddMacro.Content = Localization.Get("BtnSaveMacro", lang);
        if (BtnDeleteMacro != null) BtnDeleteMacro.Content = Localization.Get("BtnDeleteMacro", lang);

        if (LblGameTitle != null) LblGameTitle.Text = Localization.Get("GameTitle", lang);
        if (LblGameDesc != null) LblGameDesc.Text = Localization.Get("GameDesc", lang);
        if (LblExcludedAppsHeader != null) LblExcludedAppsHeader.Text = Localization.Get("ExcludedListTitle", lang);
        if (BtnPickRunningProcess != null) BtnPickRunningProcess.Content = Localization.Get("BtnSelectProcess", lang);
        if (BtnBrowseExe != null) BtnBrowseExe.Content = Localization.Get("BtnBrowseExe", lang);
        if (BtnClearExcludedApps != null) BtnClearExcludedApps.Content = Localization.Get("BtnClearAll", lang);
        if (BtnAddApp != null) BtnAddApp.Content = Localization.Get("BtnAddManual", lang);
        if (BtnDeleteApp != null) BtnDeleteApp.Content = Localization.Get("BtnDeleteMacro", lang);

        if (BtnReset != null) BtnReset.Content = Localization.Get("BtnReset", lang);
        if (BtnClose != null) BtnClose.Content = Localization.Get("BtnClose", lang);
        if (BtnSave != null) BtnSave.Content = Localization.Get("BtnSave", lang);
    }

    private void UpdateLiveStatusCard()
    {
        if (BdLiveBadge == null || TxtLiveLetter == null || TxtLiveTitle == null || TxtLiveSubtitle == null || SwLiveMode == null) return;

        bool en = Localization.IsEnglish(_config.UiLanguage);

        if (_config.IsEnabled)
        {
            BdLiveBadge.Background = new SolidColorBrush(MediaColor.FromRgb(220, 38, 38)); // Red-600
            TxtLiveLetter.Text = "V";
            TxtLiveTitle.Text = en ? "Vietnamese Mode" : "Tiếng Việt";
            TxtLiveSubtitle.Text = en ? "Active (Ready)" : "Đang bật (Sẵn sàng)";
            TxtLiveSubtitle.Foreground = new SolidColorBrush(MediaColor.FromRgb(22, 163, 74)); // Green-600
            SwLiveMode.IsChecked = true;
        }
        else
        {
            BdLiveBadge.Background = new SolidColorBrush(MediaColor.FromRgb(37, 99, 235)); // Blue-600
            TxtLiveLetter.Text = "E";
            TxtLiveTitle.Text = en ? "English Mode" : "English Mode";
            TxtLiveSubtitle.Text = en ? "Vietnamese Disabled" : "Đã tắt tiếng Việt";
            TxtLiveSubtitle.Foreground = new SolidColorBrush(MediaColor.FromRgb(100, 116, 139)); // Slate-500
            SwLiveMode.IsChecked = false;
        }

        UpdateHotkeyBadge();
    }

    private void SwLiveMode_Click(object sender, RoutedEventArgs e)
    {
        _config.IsEnabled = SwLiveMode.IsChecked == true;
        _session.IsEnabled = _config.IsEnabled;
        _config.Save();
        _onStateUpdated?.Invoke();
        UpdateLiveStatusCard();
    }

    private void NavButton_Click(object sender, RoutedEventArgs e)
    {
        if (ViewOverview == null || ViewTyping == null || ViewMacro == null || ViewGame == null || ViewAbout == null || ViewAdvanced == null) return;

        ViewOverview.Visibility = NavOverview.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        ViewTyping.Visibility = NavTyping.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        ViewMacro.Visibility = NavMacro.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        ViewGame.Visibility = NavGame.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        ViewAbout.Visibility = NavAbout.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
        ViewAdvanced.Visibility = NavAdvanced.IsChecked == true ? Visibility.Visible : Visibility.Collapsed;
    }

    /// <summary>
    /// Opens a specific tab. Used by the "--tab=..." startup switch and automation.
    /// </summary>
    public void ShowTab(string tab)
    {
        switch (tab.ToLowerInvariant())
        {
            case "typing":
                NavTyping.IsChecked = true;
                break;
            case "macro":
                NavMacro.IsChecked = true;
                break;
            case "game":
                NavGame.IsChecked = true;
                break;
            case "about":
                NavAbout.IsChecked = true;
                break;
            case "advanced":
                NavAdvanced.IsChecked = true;
                break;
            default:
                NavOverview.IsChecked = true;
                break;
        }

        NavButton_Click(this, new RoutedEventArgs());
    }

    private void TitleBar_MouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.LeftButton == MouseButtonState.Pressed)
        {
            DragMove();
        }
    }

    private void BtnMinimize_Click(object sender, RoutedEventArgs e)
    {
        WindowState = WindowState.Minimized;
    }

    private void BtnClose_Click(object sender, RoutedEventArgs e)
    {
        Close();
    }

    private void BtnSave_Click(object sender, RoutedEventArgs e)
    {
        if (CboCharset.SelectedValue is CharsetId charset)
        {
            _config.Charset = charset;
            _session.Charset = Charsets.CharsetFactory.GetCharset(charset);
        }

        if (CboMethod.SelectedValue is EngineInputMethod method)
        {
            _config.Method = method;
            _session.Method = method;
        }

        if (CboToggleHotkey.SelectedValue is string hotkey)
        {
            _config.ToggleHotkey = hotkey;
        }

        if (CboUiLanguage.SelectedValue is string lang)
        {
            _config.UiLanguage = lang;
        }

        bool startWithWindows = SwStartWithWindows.IsChecked == true;
        _config.StartWithWindows = startWithWindows;
        TaskSchedulerHelper.SetRunAtLogon(startWithWindows);

        _config.FreeMarking = SwFreeMarking.IsChecked == true;
        _config.SpellCheck = SwSpellCheck.IsChecked == true;
        _config.AutoNonVnRestore = SwAutoRestore.IsChecked == true;
        _config.ModernStyle = SwModernStyle.IsChecked == true;

        _config.FixShiftLag = SwFixShiftLag.IsChecked == true;
        _config.ConsonantShortcuts = SwConsonantShortcuts.IsChecked == true;
        _config.UseClipboardForUnicode = SwUseClipboardForUnicode.IsChecked == true;
        _config.AutoCapitalize = SwAutoCapitalize.IsChecked == true;
        _config.NotificationSound = SwNotificationSound.IsChecked == true;

        if (CboTheme.SelectedValue is string themeMode)
        {
            _config.ThemeMode = themeMode;
            ThemeManager.Apply(themeMode);
        }

        _config.EnableMacro = SwEnableMacro.IsChecked == true;
        _config.AllowMacroWhenDisabled = SwMacroWhenDisabled.IsChecked == true;

        _config.EnglishKeywords = ParseKeywords(TxtEnglishKeywords.Text);
        _config.ForeignInitials = string.IsNullOrWhiteSpace(TxtForeignInitials.Text) ? "fjwz" : TxtForeignInitials.Text.Trim();

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
            if (!string.IsNullOrWhiteSpace(app))
            {
                _config.ExcludedApps.Add(app.Trim());
            }
        }

        _config.Save();
        _config.ApplyToSession(_session);
        _onStateUpdated?.Invoke();

        UpdateLiveStatusCard();
        Close();
    }

    private void BtnReset_Click(object sender, RoutedEventArgs e)
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
        _config.TrayVnTop = def.TrayVnTop;
        _config.TrayVnBottom = def.TrayVnBottom;
        _config.TrayEnTop = def.TrayEnTop;
        _config.TrayEnBottom = def.TrayEnBottom;
        _config.EnglishKeywords.Clear();
        _config.ForeignInitials = def.ForeignInitials;
        _config.CustomCharsetMap = def.CustomCharsetMap;
        _config.ExcludedApps = def.ExcludedApps;
        _config.Macros = def.Macros;

        LoadConfigData();
    }

    private void DgMacros_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (DgMacros.SelectedItem is MacroItem item)
        {
            TxtMacroKey.Text = item.Key;
            TxtMacroValue.Text = item.Value;
        }
    }

    private void AddOrUpdateMacro(string key, string val)
    {
        if (string.IsNullOrEmpty(key)) return;

        var existing = _macroItems.FirstOrDefault(m => string.Equals(m.Key, key, StringComparison.OrdinalIgnoreCase));
        if (existing != null)
        {
            existing.Value = val;
        }
        else
        {
            _macroItems.Add(new MacroItem { Key = key, Value = val });
        }
    }

    private void BtnAddMacro_Click(object sender, RoutedEventArgs e)
    {
        string key = TxtMacroKey.Text.Trim();
        string val = TxtMacroValue.Text.Trim();
        if (string.IsNullOrEmpty(key)) return;

        AddOrUpdateMacro(key, val);
        DgMacros.Items.Refresh();
        UpdateMacroCount();

        TxtMacroKey.Clear();
        TxtMacroValue.Clear();
    }

    private void BtnDeleteMacro_Click(object sender, RoutedEventArgs e)
    {
        if (DgMacros.SelectedItem is MacroItem item)
        {
            _macroItems.Remove(item);
            UpdateMacroCount();
        }
    }

    private void BtnAddSampleMacros_Click(object sender, RoutedEventArgs e)
    {
        var samples = new (string k, string v)[]
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

        foreach (var (k, v) in samples)
        {
            AddOrUpdateMacro(k, v);
        }
        DgMacros.Items.Refresh();
        UpdateMacroCount();
    }

    private void BtnExportMacros_Click(object sender, RoutedEventArgs e)
    {
        var sfd = new Microsoft.Win32.SaveFileDialog
        {
            Filter = "File văn bản (*.txt)|*.txt|Tất cả các file (*.*)|*.*",
            FileName = "VietFlow_Macros.txt"
        };
        if (sfd.ShowDialog() == true)
        {
            var sb = new StringBuilder();
            sb.AppendLine("# Bảng gõ tắt VietFlow");
            foreach (var item in _macroItems)
            {
                sb.AppendLine($"{item.Key}:{item.Value}");
            }
            File.WriteAllText(sfd.FileName, sb.ToString(), Encoding.UTF8);
        }
    }

    private void BtnImportMacros_Click(object sender, RoutedEventArgs e)
    {
        var ofd = new Microsoft.Win32.OpenFileDialog
        {
            Filter = "File văn bản (*.txt)|*.txt|Tất cả các file (*.*)|*.*"
        };
        if (ofd.ShowDialog() == true)
        {
            var lines = File.ReadAllLines(ofd.FileName);
            foreach (var line in lines)
            {
                string trimmed = line.Trim();
                if (string.IsNullOrWhiteSpace(trimmed) || trimmed.StartsWith('#')) continue;

                int colonIdx = trimmed.IndexOf(':');
                if (colonIdx > 0)
                {
                    string k = trimmed[..colonIdx].Trim();
                    string v = trimmed[(colonIdx + 1)..].Trim();
                    AddOrUpdateMacro(k, v);
                    continue;
                }

                int tabIdx = trimmed.IndexOf('\t');
                if (tabIdx > 0)
                {
                    string k = trimmed[..tabIdx].Trim();
                    string v = trimmed[(tabIdx + 1)..].Trim();
                    AddOrUpdateMacro(k, v);
                }
            }
            DgMacros.Items.Refresh();
            UpdateMacroCount();
        }
    }

    private void BtnClearMacros_Click(object sender, RoutedEventArgs e)
    {
        if (System.Windows.MessageBox.Show("Bạn có chắc chắn muốn xóa toàn bộ danh sách từ gõ tắt không?", "Xác nhận", MessageBoxButton.YesNo, MessageBoxImage.Question) == MessageBoxResult.Yes)
        {
            _macroItems.Clear();
            UpdateMacroCount();
        }
    }

    private void BtnAddApp_Click(object sender, RoutedEventArgs e)
    {
        string app = TxtAppProcess.Text.Trim();
        if (string.IsNullOrEmpty(app)) return;

        if (!app.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
        {
            app += ".exe";
        }

        if (!_excludedAppItems.Any(a => string.Equals(a, app, StringComparison.OrdinalIgnoreCase)))
        {
            _excludedAppItems.Add(app);
            UpdateExcludedAppsCount();
        }

        TxtAppProcess.Clear();
    }

    private void BtnPickRunningProcess_Click(object sender, RoutedEventArgs e)
    {
        var picker = new ProcessPickerWindow { Owner = this };
        if (picker.ShowDialog() == true && !string.IsNullOrEmpty(picker.SelectedExeName))
        {
            string exe = picker.SelectedExeName;
            if (!_excludedAppItems.Any(a => string.Equals(a, exe, StringComparison.OrdinalIgnoreCase)))
            {
                _excludedAppItems.Add(exe);
                UpdateExcludedAppsCount();
            }
        }
    }

    private void BtnBrowseExe_Click(object sender, RoutedEventArgs e)
    {
        var ofd = new Microsoft.Win32.OpenFileDialog
        {
            Filter = "File thực thi (*.exe)|*.exe|Tất cả các file (*.*)|*.*",
            Title = "Chọn file ứng dụng hoặc game để loại trừ"
        };
        if (ofd.ShowDialog() == true)
        {
            string exeName = Path.GetFileName(ofd.FileName);
            if (!_excludedAppItems.Any(a => string.Equals(a, exeName, StringComparison.OrdinalIgnoreCase)))
            {
                _excludedAppItems.Add(exeName);
                UpdateExcludedAppsCount();
            }
        }
    }

    private void BtnClearExcludedApps_Click(object sender, RoutedEventArgs e)
    {
        _excludedAppItems.Clear();
        UpdateExcludedAppsCount();
    }

    private void BtnDeleteApp_Click(object sender, RoutedEventArgs e)
    {
        if (LstExcludedApps.SelectedItem is string app)
        {
            _excludedAppItems.Remove(app);
            UpdateExcludedAppsCount();
        }
    }

    public void RefreshLiveState()
    {
        UpdateLiveStatusCard();
    }

    private void CboTheme_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (CboTheme.SelectedValue is string mode)
        {
            ThemeManager.Apply(mode);
        }
    }

    private void BtnCustomizeTray_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new TrayIconDialog(
            _config.TrayVnTop,
            _config.TrayVnBottom,
            _config.TrayEnTop,
            _config.TrayEnBottom)
        {
            Owner = this
        };

        if (dialog.ShowDialog() == true)
        {
            _config.TrayVnTop = dialog.VnTop;
            _config.TrayVnBottom = dialog.VnBottom;
            _config.TrayEnTop = dialog.EnTop;
            _config.TrayEnBottom = dialog.EnBottom;
            _onStateUpdated?.Invoke();
        }
    }

    private void AboutLink_RequestNavigate(object sender, System.Windows.Navigation.RequestNavigateEventArgs e)
    {
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = e.Uri.AbsoluteUri,
                UseShellExecute = true
            });
        }
        catch
        {
            // Ignore shell failures.
        }
        e.Handled = true;
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
            TxtKeywordsCount.Text = $"{ParseKeywords(TxtEnglishKeywords.Text).Count} từ";
        }
    }

    private void TxtEnglishKeywords_TextChanged(object sender, TextChangedEventArgs e)
    {
        UpdateKeywordsCount();
    }

    private void BtnSaveKeywords_Click(object sender, RoutedEventArgs e)
    {
        var words = ParseKeywords(TxtEnglishKeywords.Text);
        _config.EnglishKeywords = words;
        EnglishPhonetics.SetKeywords(words.Count > 0 ? words : EnglishPhonetics.DefaultKeywords);
        _config.Save();
        UpdateKeywordsCount();
    }

    private void BtnResetKeywords_Click(object sender, RoutedEventArgs e)
    {
        _config.EnglishKeywords.Clear();
        TxtEnglishKeywords.Text = string.Join(Environment.NewLine, EnglishPhonetics.DefaultKeywords);
        EnglishPhonetics.SetKeywords(EnglishPhonetics.DefaultKeywords);
        _config.Save();
        UpdateKeywordsCount();
    }

    private void BtnOpenConfigFolder_Click(object sender, RoutedEventArgs e)
    {
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
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

    protected override void OnClosing(CancelEventArgs e)
    {
        base.OnClosing(e);
        NativeMethods.TrimProcessMemory();
    }
}

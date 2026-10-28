// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System;
using System.Collections.Generic;
using System.IO;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media.Imaging;
using SkiaSharp;
using VietFlow.Configuration;
using VietFlow.Engine;

namespace VietFlow.Windows;

internal sealed class TrayIconManager : IDisposable
{
    private readonly TrayIcon _trayIcon;
    private readonly VietFlowConfig _config;
    private readonly Action _onToggleLanguage;
    private readonly Action<InputMethod> _onSelectMethod;
    private readonly Action<CharsetId> _onSelectCharset;
    private readonly Action<string?> _onOpenSettingsTab;
    private readonly Action _onQuickConvert;
    private readonly Action _onOpenConverter;
    private readonly Action _onCustomizeTray;
    private readonly Action _onRestart;
    private readonly Action _onRestartAdmin;
    private readonly Action _onExit;

    private long _lastClickTicks;
    private bool _firstClickToggled;
    private NativeMenuItem? _toggleItem;
    private NativeMenuItem? _startupItem;
    private NativeMenuItem? _methodMenu;
    private NativeMenuItem? _charsetMenu;
    private readonly Dictionary<InputMethod, NativeMenuItem> _methodItems = new();
    private readonly Dictionary<CharsetId, NativeMenuItem> _charsetItems = new();
    private readonly Dictionary<string, NativeMenuItem> _themeItems = new();
    private string _menuLanguage = string.Empty;
    private bool _disposed;
    private WindowIcon? _iconVn;
    private WindowIcon? _iconEn;
    private string _iconVnKey = string.Empty;
    private string _iconEnKey = string.Empty;

    public TrayIconManager(
        VietFlowConfig config,
        Action onToggleLanguage,
        Action<InputMethod> onSelectMethod,
        Action<CharsetId> onSelectCharset,
        Action<string?> onOpenSettingsTab,
        Action onQuickConvert,
        Action onOpenConverter,
        Action onCustomizeTray,
        Action onRestart,
        Action onRestartAdmin,
        Action onExit)
    {
        _config = config;
        _onToggleLanguage = onToggleLanguage;
        _onSelectMethod = onSelectMethod;
        _onSelectCharset = onSelectCharset;
        _onOpenSettingsTab = onOpenSettingsTab;
        _onQuickConvert = onQuickConvert;
        _onOpenConverter = onOpenConverter;
        _onCustomizeTray = onCustomizeTray;
        _onRestart = onRestart;
        _onRestartAdmin = onRestartAdmin;
        _onExit = onExit;

        _trayIcon = new TrayIcon
        {
            IsVisible = true,
            ToolTipText = "VietFlow"
        };
        // Toggle immediately on a single click: no 500 ms wait for a possible
        // second click. A second click inside the system double-click time opens
        // Settings instead of toggling again.
        uint doubleClickMs = NativeMethods.GetDoubleClickTime();
        long doubleClickWindow = Math.Clamp((int)doubleClickMs, 220, 600);
        _trayIcon.Clicked += (_, _) =>
        {
            long now = Environment.TickCount64;
            if (_lastClickTicks != 0 && now - _lastClickTicks <= doubleClickWindow)
            {
                _lastClickTicks = 0;
                if (_firstClickToggled)
                {
                    // A double click is "open Settings", not "toggle twice".
                    // Undo the immediate first-click toggle so the mode is unchanged.
                    _firstClickToggled = false;
                    _onToggleLanguage();
                }

                _onOpenSettingsTab(null);
                return;
            }

            _lastClickTicks = now;
            _firstClickToggled = true;
            _onToggleLanguage();
        };

        TrayIcon.SetIcons(Application.Current!, new TrayIcons { _trayIcon });

        BuildMenu();
        UpdateState(_config.IsEnabled, _config.Method, _config.Charset);
    }

    private void BuildMenu()
    {
        _menuLanguage = _config.UiLanguage;
        bool en = _config.UiLanguage == "en";
        var menu = new NativeMenu();

        _toggleItem = new NativeMenuItem(en ? "Vietnamese mode" : "Bật tiếng Việt")
        {
            ToggleType = MenuItemToggleType.CheckBox
        };
        _toggleItem.Click += (_, _) => _onToggleLanguage();
        menu.Items.Add(_toggleItem);
        menu.Items.Add(new NativeMenuItemSeparator());

        _methodMenu = new NativeMenuItem(en ? "Input method" : "Kiểu gõ")
        {
            Menu = new NativeMenu()
        };
        AddMethodItem(_methodMenu.Menu!, InputMethod.Telex, "Telex");
        AddMethodItem(_methodMenu.Menu!, InputMethod.SimpleTelex, "Simple Telex");
        AddMethodItem(_methodMenu.Menu!, InputMethod.Vni, "VNI");
        AddMethodItem(_methodMenu.Menu!, InputMethod.Viqr, "VIQR");
        menu.Items.Add(_methodMenu);

        _charsetMenu = new NativeMenuItem(en ? "Charset" : "Bảng mã")
        {
            Menu = new NativeMenu()
        };
        AddCharsetItem(_charsetMenu.Menu!, CharsetId.Unicode, "Unicode");
        AddCharsetItem(_charsetMenu.Menu!, CharsetId.UnicodeComposite, en ? "Unicode Composite (NFD)" : "Unicode tổ hợp (NFD)");
        AddCharsetItem(_charsetMenu.Menu!, CharsetId.Tcvn3, "TCVN3 (ABC)");
        AddCharsetItem(_charsetMenu.Menu!, CharsetId.VniWindows, "VNI Windows");
        AddCharsetItem(_charsetMenu.Menu!, CharsetId.Custom, en ? "Custom..." : "Tùy chỉnh...");
        menu.Items.Add(_charsetMenu);
        menu.Items.Add(new NativeMenuItemSeparator());

        var tools = new NativeMenuItem(en ? "Tools" : "Công cụ")
        {
            Menu = new NativeMenu()
        };
        tools.Menu!.Items.Add(Item(en ? "Convert..." : "Chuyển mã...", _onOpenConverter));
        tools.Menu.Items.Add(Item(en ? "Convert clipboard" : "Chuyển mã Clipboard", _onQuickConvert));
        tools.Menu.Items.Add(Item(en ? "Macros..." : "Bảng gõ tắt...", () => _onOpenSettingsTab("macro")));
        menu.Items.Add(tools);

        var system = new NativeMenuItem(en ? "System" : "Hệ thống")
        {
            Menu = new NativeMenu()
        };

        _startupItem = new NativeMenuItem(en ? "Start with Windows" : "Khởi động cùng Windows")
        {
            ToggleType = MenuItemToggleType.CheckBox,
            IsChecked = StartupHelper.IsEnabled()
        };
        _startupItem.Click += (_, _) =>
        {
            bool enabled = !_startupItem.IsChecked;
            bool success = StartupHelper.SetEnabled(enabled);
            _startupItem.IsChecked = success ? enabled : StartupHelper.IsEnabled();
            _config.StartWithWindows = _startupItem.IsChecked;
            _config.Save();

            if (!success)
            {
                ShowBalloon(
                    "VietFlow",
                    en ? "Could not update the Windows startup setting." : "Không thể cập nhật khởi động cùng Windows.");
            }
        };
        system.Menu!.Items.Add(_startupItem);

        var themeMenu = new NativeMenuItem(en ? "Theme" : "Chế độ màu")
        {
            Menu = new NativeMenu()
        };
        AddThemeItem(themeMenu.Menu!, ThemeManager.SystemMode, en ? "System" : "Theo hệ thống");
        AddThemeItem(themeMenu.Menu!, ThemeManager.LightMode, en ? "Light" : "Sáng");
        AddThemeItem(themeMenu.Menu!, ThemeManager.DarkMode, en ? "Dark" : "Tối");
        system.Menu.Items.Add(themeMenu);

        system.Menu.Items.Add(Item(en ? "Customize tray icon..." : "Tùy chỉnh Tray icon...", _onCustomizeTray));
        system.Menu.Items.Add(new NativeMenuItemSeparator());
        system.Menu.Items.Add(Item(en ? "Restart VietFlow" : "Khởi động lại VietFlow", _onRestart));
        system.Menu.Items.Add(Item(en ? "Restart as Administrator" : "Khởi động lại quyền Admin", _onRestartAdmin));
        menu.Items.Add(system);
        menu.Items.Add(new NativeMenuItemSeparator());

        menu.Items.Add(Item(en ? "Settings..." : "Cài đặt...", () => _onOpenSettingsTab(null)));
        menu.Items.Add(Item(en ? "About VietFlow..." : "Giới thiệu VietFlow...", () => _onOpenSettingsTab("about")));
        menu.Items.Add(new NativeMenuItemSeparator());
        menu.Items.Add(Item(en ? "Exit" : "Thoát", _onExit));

        _trayIcon.Menu = menu;
    }

    private static NativeMenuItem Item(string header, Action action)
    {
        var item = new NativeMenuItem(header);
        item.Click += (_, _) => action();
        return item;
    }

    private void AddMethodItem(NativeMenu menu, InputMethod method, string text)
    {
        var item = new NativeMenuItem(text)
        {
            ToggleType = MenuItemToggleType.CheckBox
        };
        item.Click += (_, _) => _onSelectMethod(method);
        _methodItems[method] = item;
        menu.Items.Add(item);
    }

    private void AddCharsetItem(NativeMenu menu, CharsetId id, string text)
    {
        var item = new NativeMenuItem(text)
        {
            ToggleType = MenuItemToggleType.CheckBox
        };
        item.Click += (_, _) => _onSelectCharset(id);
        _charsetItems[id] = item;
        menu.Items.Add(item);
    }

    private void AddThemeItem(NativeMenu menu, string mode, string text)
    {
        var item = new NativeMenuItem(text)
        {
            ToggleType = MenuItemToggleType.CheckBox
        };
        item.Click += (_, _) =>
        {
            _config.ThemeMode = mode;
            _config.Save();
            ThemeManager.Apply(mode);
            UpdateState(_config.IsEnabled, _config.Method, _config.Charset);
        };
        _themeItems[mode] = item;
        menu.Items.Add(item);
    }

    public void RefreshStartupState()
    {
        if (_startupItem != null)
        {
            _startupItem.IsChecked = StartupHelper.IsEnabled();
        }
    }

    public void UpdateState(bool isEnabled, InputMethod method, CharsetId charset)
    {
        if (!string.Equals(_menuLanguage, _config.UiLanguage, StringComparison.Ordinal))
        {
            BuildMenu();
        }

        UpdateIcon(isEnabled);

        bool en = _config.UiLanguage == "en";

        if (_toggleItem != null)
        {
            string hotkey = GetHotkeyDisplay(_config.ToggleHotkey, _config.UiLanguage);
            _toggleItem.Header = isEnabled
                ? (en ? $"Vietnamese mode [V] ({hotkey})" : $"Đang bật tiếng Việt [V] ({hotkey})")
                : (en ? $"English mode [E] ({hotkey})" : $"Đang tắt tiếng Việt [E] ({hotkey})");
            _toggleItem.IsChecked = isEnabled;
        }

        foreach (var (m, item) in _methodItems)
        {
            item.IsChecked = m == method;
        }

        foreach (var (c, item) in _charsetItems)
        {
            item.IsChecked = c == charset;
        }

        string currentTheme = ThemeManager.Normalize(_config.ThemeMode);
        foreach (var (mode, item) in _themeItems)
        {
            item.IsChecked = string.Equals(mode, currentTheme, StringComparison.OrdinalIgnoreCase);
        }

        if (_methodMenu != null && _methodItems.TryGetValue(method, out var methodItem))
        {
            _methodMenu.Header = (en ? "Input method: " : "Kiểu gõ: ") + methodItem.Header;
        }

        if (_charsetMenu != null && _charsetItems.TryGetValue(charset, out var charsetItem))
        {
            _charsetMenu.Header = (en ? "Charset: " : "Bảng mã: ") + charsetItem.Header;
        }

        string methodStr = method switch
        {
            InputMethod.Telex => "Telex",
            InputMethod.SimpleTelex => "Simple Telex",
            InputMethod.Vni => "VNI",
            InputMethod.Viqr => "VIQR",
            _ => method.ToString()
        };

        string modeText = isEnabled
            ? (en ? "Vietnamese [V]" : "Tiếng Việt [V]")
            : "English [E]";
        _trayIcon.ToolTipText = $"VietFlow 0.1 alpha · {modeText} · {methodStr}";
    }

    public static string GetHotkeyDisplay(string hotkey, string lang = "vi")
    {
        bool en = lang == "en";
        return hotkey switch
        {
            "AltZ" => "Alt + Z",
            "CtrlSpace" => "Ctrl + Space",
            "ShiftSpace" => "Shift + Space",
            "AltShift" => "Alt + Shift",
            "WinSpace" => "Win + Space",
            "CapsLock" => "Caps Lock",
            "RightCtrl" => en ? "Right Control" : "Ctrl phải",
            "Custom" => en ? "Custom Key" : "Phím tùy chỉnh",
            _ => "Ctrl + Shift"
        };
    }

    public void UpdateIcon(bool isVietnamese)
    {
        string top = isVietnamese ? _config.TrayVnTop : _config.TrayEnTop;
        string bottom = isVietnamese ? _config.TrayVnBottom : _config.TrayEnBottom;
        string key = top + "|" + bottom;

        if (isVietnamese)
        {
            if (_iconVn == null || !string.Equals(_iconVnKey, key, StringComparison.Ordinal))
            {
                _iconVn = RenderIcon(top, bottom, "V");
                _iconVnKey = key;
            }

            _trayIcon.Icon = _iconVn;
        }
        else
        {
            if (_iconEn == null || !string.Equals(_iconEnKey, key, StringComparison.Ordinal))
            {
                _iconEn = RenderIcon(top, bottom, "E");
                _iconEnKey = key;
            }

            _trayIcon.Icon = _iconEn;
        }
    }

    private static WindowIcon RenderIcon(string topHex, string bottomHex, string letter)
    {
        const int size = 64;
        SKColor top = ParseColor(topHex, SKColor.Parse("#EF4444"));
        SKColor bottom = ParseColor(bottomHex, SKColor.Parse("#B91C1C"));

        using var bitmap = new SKBitmap(size, size);
        using var canvas = new SKCanvas(bitmap);
        canvas.Clear(SKColors.Transparent);

        // Fill the whole small-icon slot: Windows never gives a tray icon more than
        // 16/20/24 px, so the visual weight has to come from the badge and glyph.
        var rect = new SKRect(0, 0, size, size);
        float radius = size * 0.24f;

        using (var shader = SKShader.CreateLinearGradient(
                   new SKPoint(0, 0),
                   new SKPoint(0, size),
                   new[] { top, bottom },
                   null,
                   SKShaderTileMode.Clamp))
        using (var fill = new SKPaint { Shader = shader, IsAntialias = true })
        {
            canvas.DrawRoundRect(rect, radius, radius, fill);
        }

        using (var border = new SKPaint
               {
                   Style = SKPaintStyle.Stroke,
                   Color = new SKColor(0, 0, 0, 70),
                   StrokeWidth = 1f,
                   IsAntialias = true
               })
        {
            canvas.DrawRoundRect(new SKRect(0.5f, 0.5f, size - 0.5f, size - 0.5f), radius, radius, border);
        }

        bool lightBadge = IsLight(top, bottom);
        SKColor letterColor = lightBadge ? new SKColor(17, 24, 39) : SKColors.White;

        using var typeface = SKTypeface.FromFamilyName("Segoe UI", SKFontStyle.Bold)
                             ?? SKTypeface.Default;
        using var font = new SKFont(typeface, size);

        // Measure the real glyph outline, not the font em box. V and E have
        // different side bearings/vertical bounds, so centering by font metrics
        // pushes V toward the bottom edge.
        using var glyphPath = font.GetTextPath(letter);
        if (glyphPath != null)
        {
            SKRect bounds = glyphPath.Bounds;
            if (bounds.Width > 0 && bounds.Height > 0)
            {
                float target = size * 0.76f;
                float scale = Math.Min(target / bounds.Width, target / bounds.Height);

                glyphPath.Transform(SKMatrix.CreateScale(scale, scale));
                glyphPath.Transform(SKMatrix.CreateTranslation(
                    ((size - (bounds.Width * scale)) / 2f) - (bounds.Left * scale),
                    ((size - (bounds.Height * scale)) / 2f) - (bounds.Top * scale)));

                using var fill = new SKPaint
                {
                    IsAntialias = true,
                    Color = letterColor
                };

                using var shadowPath = new SKPath();
                shadowPath.AddPath(glyphPath);
                shadowPath.Transform(SKMatrix.CreateTranslation(1f, 1f));
                using var shadowPaint = new SKPaint
                {
                    IsAntialias = true,
                    Color = lightBadge ? new SKColor(255, 255, 255, 90) : new SKColor(0, 0, 0, 90)
                };

                canvas.DrawPath(shadowPath, shadowPaint);
                canvas.DrawPath(glyphPath, fill);
            }
        }

        using var image = SKImage.FromBitmap(bitmap);
        using var data = image.Encode(SKEncodedImageFormat.Png, 100);
        using var stream = new MemoryStream(data.ToArray());
        return new WindowIcon(stream);
    }

    private static SKColor ParseColor(string value, SKColor fallback)
    {
        try
        {
            return string.IsNullOrWhiteSpace(value) ? fallback : SKColor.Parse(value);
        }
        catch
        {
            return fallback;
        }
    }

    private static bool IsLight(SKColor top, SKColor bottom)
    {
        double luminance = (0.299 * (top.Red + bottom.Red) +
                            0.587 * (top.Green + bottom.Green) +
                            0.114 * (top.Blue + bottom.Blue)) / 2.0;
        return luminance > 165;
    }

    public void ShowBalloon(string title, string message)
    {
        ToastWindow.Show(title, message);
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _trayIcon.IsVisible = false;
        _trayIcon.Dispose();
    }
}

// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Drawing.Text;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using VietFlow.Configuration;
using VietFlow.Engine;

namespace VietFlow.Windows;

internal sealed class TrayIconManager : IDisposable
{
    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool DestroyIcon(IntPtr hIcon);

    private readonly NotifyIcon _notifyIcon;
    private readonly VietFlowConfig _config;
    private readonly Action _onToggleLanguage;
    private readonly Action<InputMethod> _onSelectMethod;
    private readonly Action<CharsetId> _onSelectCharset;
    private readonly Action<string?> _onOpenSettingsTab;
    private readonly Action _onQuickConvert;
    private readonly Action _onOpenConverter;
    private readonly Action _onRestart;
    private readonly Action _onRestartAdmin;
    private readonly Action _onExit;

    private Icon? _currentIcon;
    private IntPtr _currentHIcon = IntPtr.Zero;
    private ToolStripMenuItem? _toggleItem;
    private readonly Dictionary<InputMethod, ToolStripMenuItem> _methodItems = new();
    private readonly Dictionary<CharsetId, ToolStripMenuItem> _charsetItems = new();
    private readonly Dictionary<string, ToolStripMenuItem> _themeItems = new();
    private string _menuLanguage = string.Empty;

    public TrayIconManager(
        VietFlowConfig config,
        Action onToggleLanguage,
        Action<InputMethod> onSelectMethod,
        Action<CharsetId> onSelectCharset,
        Action<string?> onOpenSettingsTab,
        Action onQuickConvert,
        Action onOpenConverter,
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
        _onRestart = onRestart;
        _onRestartAdmin = onRestartAdmin;
        _onExit = onExit;

        _notifyIcon = new NotifyIcon
        {
            Visible = true,
            Text = "VietFlow 0.1 alpha"
        };

        _notifyIcon.MouseClick += (_, e) =>
        {
            if (e.Button == MouseButtons.Left)
            {
                _onToggleLanguage();
            }
        };

        _notifyIcon.DoubleClick += (_, _) => _onOpenSettingsTab(null);

        BuildContextMenu();
        UpdateState(_config.IsEnabled, _config.Method, _config.Charset);
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

    private void BuildContextMenu()
    {
        _menuLanguage = _config.UiLanguage;
        bool en = _config.UiLanguage == "en";
        var menu = new ContextMenuStrip
        {
            ShowImageMargin = false
        };

        menu.Items.Add(Item(en ? "Convert..." : "Chuyển mã...", _onOpenConverter));
        menu.Items.Add(Item(en ? "Convert Clipboard" : "Chuyển mã Clipboard", _onQuickConvert));
        menu.Items.Add(Item(en ? "Macros..." : "Bảng gõ tắt...", () => _onOpenSettingsTab("macro")));

        var other = new ToolStripMenuItem(en ? "Other options" : "Lựa chọn khác");
        _toggleItem = new ToolStripMenuItem(en ? "Vietnamese mode" : "Bật tiếng Việt");
        _toggleItem.Click += (_, _) => _onToggleLanguage();
        other.DropDownItems.Add(_toggleItem);

        var startup = new ToolStripMenuItem(en ? "Start with Windows" : "Khởi động cùng Windows")
        {
            Checked = TaskSchedulerHelper.IsScheduledToRunAtLogon()
        };
        startup.Click += (_, _) =>
        {
            bool enabled = !startup.Checked;
            TaskSchedulerHelper.SetRunAtLogon(enabled);
            startup.Checked = enabled;
            _config.StartWithWindows = enabled;
            _config.Save();
        };
        other.DropDownItems.Add(startup);

        var themeMenu = new ToolStripMenuItem(en ? "Theme" : "Chế độ màu");
        AddThemeItem(themeMenu.DropDownItems, ThemeManager.SystemMode, en ? "System" : "Theo hệ thống");
        AddThemeItem(themeMenu.DropDownItems, ThemeManager.LightMode, en ? "Light" : "Sáng");
        AddThemeItem(themeMenu.DropDownItems, ThemeManager.DarkMode, en ? "Dark" : "Tối");
        other.DropDownItems.Add(themeMenu);

        menu.Items.Add(other);
        menu.Items.Add(new ToolStripSeparator());

        AddMethodItem(menu.Items, InputMethod.Telex, "Telex");
        AddMethodItem(menu.Items, InputMethod.Vni, "VNI");
        var moreMethods = new ToolStripMenuItem(en ? "More input methods" : "Kiểu gõ khác");
        AddMethodItem(moreMethods.DropDownItems, InputMethod.SimpleTelex, "Simple Telex");
        AddMethodItem(moreMethods.DropDownItems, InputMethod.MicrosoftVi, "Microsoft Vietnamese");
        AddMethodItem(moreMethods.DropDownItems, InputMethod.Viqr, "VIQR");
        menu.Items.Add(moreMethods);
        menu.Items.Add(new ToolStripSeparator());

        AddCharsetItem(menu.Items, CharsetId.Unicode, "Unicode");
        AddCharsetItem(menu.Items, CharsetId.VniWindows, "VNI Windows");
        AddCharsetItem(menu.Items, CharsetId.Tcvn3, "TCVN3 (ABC)");
        var moreCharsets = new ToolStripMenuItem(en ? "More charsets" : "Bảng mã khác");
        AddCharsetItem(moreCharsets.DropDownItems, CharsetId.UnicodeComposite, "Unicode Composite (NFD)");
        AddCharsetItem(moreCharsets.DropDownItems, CharsetId.Viqr, "VIQR");
        AddCharsetItem(moreCharsets.DropDownItems, CharsetId.Custom, en ? "Custom (Advanced)" : "Tùy chỉnh (Nâng cao)");
        menu.Items.Add(moreCharsets);
        menu.Items.Add(new ToolStripSeparator());

        menu.Items.Add(Item(en ? "Restart VietFlow" : "Khởi động lại VietFlow", _onRestart));
        menu.Items.Add(Item(en ? "Restart as Administrator" : "Khởi động lại quyền Admin", _onRestartAdmin));
        menu.Items.Add(Item(en ? "Settings..." : "Cài đặt...", () => _onOpenSettingsTab(null)));
        menu.Items.Add(Item(en ? "About VietFlow..." : "About VietFlow...", () => _onOpenSettingsTab("about")));
        menu.Items.Add(new ToolStripSeparator());
        menu.Items.Add(Item(en ? "Exit" : "Thoát", _onExit));

        _notifyIcon.ContextMenuStrip = menu;
    }

    private static ToolStripMenuItem Item(string text, Action action)
    {
        var item = new ToolStripMenuItem(text);
        item.Click += (_, _) => action();
        return item;
    }

    private void AddMethodItem(ToolStripItemCollection items, InputMethod method, string text)
    {
        var item = new ToolStripMenuItem(text);
        item.Click += (_, _) => _onSelectMethod(method);
        _methodItems[method] = item;
        items.Add(item);
    }

    private void AddCharsetItem(ToolStripItemCollection items, CharsetId id, string text)
    {
        var item = new ToolStripMenuItem(text);
        item.Click += (_, _) => _onSelectCharset(id);
        _charsetItems[id] = item;
        items.Add(item);
    }

    private void AddThemeItem(ToolStripItemCollection items, string mode, string text)
    {
        var item = new ToolStripMenuItem(text);
        item.Click += (_, _) =>
        {
            _config.ThemeMode = mode;
            _config.Save();
            ThemeManager.Apply(mode);
            UpdateState(_config.IsEnabled, _config.Method, _config.Charset);
        };
        _themeItems[mode] = item;
        items.Add(item);
    }

    public void UpdateState(bool isEnabled, InputMethod method, CharsetId charset)
    {
        if (!string.Equals(_menuLanguage, _config.UiLanguage, StringComparison.Ordinal))
        {
            BuildContextMenu();
        }

        UpdateIcon(isEnabled);

        if (_toggleItem != null)
        {
            string hotkey = GetHotkeyDisplay(_config.ToggleHotkey, _config.UiLanguage);
            _toggleItem.Text = isEnabled
                ? (_config.UiLanguage == "en" ? $"Vietnamese mode [V] ({hotkey})" : $"Đang bật tiếng Việt [V] ({hotkey})")
                : (_config.UiLanguage == "en" ? $"English mode [E] ({hotkey})" : $"Đang tắt tiếng Việt [E] ({hotkey})");
            _toggleItem.Checked = isEnabled;
        }

        foreach (var (m, item) in _methodItems)
        {
            item.Checked = m == method;
        }

        foreach (var (c, item) in _charsetItems)
        {
            item.Checked = c == charset;
        }

        string currentTheme = ThemeManager.Normalize(_config.ThemeMode);
        foreach (var (mode, item) in _themeItems)
        {
            item.Checked = string.Equals(mode, currentTheme, StringComparison.OrdinalIgnoreCase);
        }

        string methodStr = method switch
        {
            InputMethod.Telex => "Telex",
            InputMethod.SimpleTelex => "Simple Telex",
            InputMethod.Vni => "VNI",
            InputMethod.MicrosoftVi => "MS Vi",
            InputMethod.Viqr => "VIQR",
            _ => method.ToString()
        };

        bool en = _config.UiLanguage == "en";
        string modeText = isEnabled
            ? (en ? "Vietnamese [V]" : "Tiếng Việt [V]")
            : "English [E]";
        _notifyIcon.Text = $"VietFlow 0.1 alpha · {modeText} · {methodStr}";
    }

    public void UpdateIcon(bool isVietnamese)
    {
        int size = Math.Max(16, SystemInformation.SmallIconSize.Width);
        if (size > 64)
        {
            size = 64;
        }

        using var bitmap = new Bitmap(size, size, PixelFormat.Format32bppArgb);
        using (var g = Graphics.FromImage(bitmap))
        {
            g.SmoothingMode = SmoothingMode.AntiAlias;
            g.TextRenderingHint = TextRenderingHint.AntiAliasGridFit;
            g.InterpolationMode = InterpolationMode.HighQualityBicubic;
            g.PixelOffsetMode = PixelOffsetMode.HighQuality;
            g.Clear(Color.Transparent);

            Color topColor = isVietnamese
                ? ParseColor(_config.TrayVnTop, "#EF4444")
                : ParseColor(_config.TrayEnTop, "#2563EB");
            Color bottomColor = isVietnamese
                ? ParseColor(_config.TrayVnBottom, "#B91C1C")
                : ParseColor(_config.TrayEnBottom, "#1D4ED8");

            var rect = new RectangleF(0.5f, 0.5f, size - 1f, size - 1f);
            int radius = Math.Max(3, size / 5);
            using var path = CreateRoundedRectanglePath(rect, radius);
            using var brush = new LinearGradientBrush(new Rectangle(0, 0, size, size), topColor, bottomColor, 90f);
            g.FillPath(brush, path);

            using var pen = new Pen(Color.FromArgb(60, 0, 0, 0), 1f);
            g.DrawPath(pen, path);

            DrawBadgeLetter(g, isVietnamese ? "V" : "E", size);
        }

        IntPtr newHandle = bitmap.GetHicon();
        var newIcon = Icon.FromHandle(newHandle);

        _notifyIcon.Icon = newIcon;

        var oldIcon = _currentIcon;
        IntPtr oldHandle = _currentHIcon;
        _currentIcon = newIcon;
        _currentHIcon = newHandle;

        oldIcon?.Dispose();
        if (oldHandle != IntPtr.Zero)
        {
            DestroyIcon(oldHandle);
        }
    }

    private static void DrawBadgeLetter(Graphics g, string letter, int size)
    {
        using var family = new FontFamily("Segoe UI");
        using var path = new GraphicsPath();

        path.AddString(letter, family, (int)FontStyle.Bold, size * 2.2f, PointF.Empty, StringFormat.GenericTypographic);

        RectangleF bounds = path.GetBounds();
        if (bounds.Width <= 0f || bounds.Height <= 0f)
        {
            return;
        }

        float target = size * 0.80f;
        float scale = Math.Min(target / bounds.Width, target / bounds.Height);

        using var matrix = new Matrix();
        // GDI+ applies the calls in reverse order: translate to the destination first,
        // then scale, then remove the glyph's origin offset.
        matrix.Translate((size - bounds.Width * scale) / 2f, (size - bounds.Height * scale) / 2f);
        matrix.Scale(scale, scale);
        matrix.Translate(-bounds.X, -bounds.Y);
        path.Transform(matrix);

        using var textBrush = new SolidBrush(Color.White);
        g.FillPath(textBrush, path);
    }

    private static Color ParseColor(string? value, string fallback)
    {
        try
        {
            return ColorTranslator.FromHtml(string.IsNullOrWhiteSpace(value) ? fallback : value);
        }
        catch
        {
            return ColorTranslator.FromHtml(fallback);
        }
    }

    private static GraphicsPath CreateRoundedRectanglePath(RectangleF rect, int radius)
    {
        var path = new GraphicsPath();
        int d = radius * 2;
        path.AddArc(rect.X, rect.Y, d, d, 180, 90);
        path.AddArc(rect.Right - d, rect.Y, d, d, 270, 90);
        path.AddArc(rect.Right - d, rect.Bottom - d, d, d, 0, 90);
        path.AddArc(rect.X, rect.Bottom - d, d, d, 90, 90);
        path.CloseFigure();
        return path;
    }

    public void ShowBalloon(string title, string text, ToolTipIcon icon = ToolTipIcon.Info)
    {
        _notifyIcon.ShowBalloonTip(2000, title, text, icon);
    }

    public void Dispose()
    {
        _notifyIcon.Visible = false;
        _notifyIcon.Dispose();
        _currentIcon?.Dispose();
        _currentIcon = null;

        if (_currentHIcon != IntPtr.Zero)
        {
            DestroyIcon(_currentHIcon);
            _currentHIcon = IntPtr.Zero;
        }
    }
}

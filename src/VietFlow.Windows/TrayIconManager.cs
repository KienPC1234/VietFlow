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
    private readonly Action _onOpenSettings;
    private readonly Action _onQuickConvert;
    private readonly Action _onExit;

    private Icon? _currentIcon;
    private IntPtr _currentHIcon = IntPtr.Zero;
    private ToolStripMenuItem? _toggleItem;
    private readonly Dictionary<InputMethod, ToolStripMenuItem> _methodItems = new();
    private readonly Dictionary<CharsetId, ToolStripMenuItem> _charsetItems = new();

    public TrayIconManager(
        VietFlowConfig config,
        Action onToggleLanguage,
        Action<InputMethod> onSelectMethod,
        Action<CharsetId> onSelectCharset,
        Action onOpenSettings,
        Action onQuickConvert,
        Action onExit)
    {
        _config = config;
        _onToggleLanguage = onToggleLanguage;
        _onSelectMethod = onSelectMethod;
        _onSelectCharset = onSelectCharset;
        _onOpenSettings = onOpenSettings;
        _onQuickConvert = onQuickConvert;
        _onExit = onExit;

        _notifyIcon = new NotifyIcon
        {
            Visible = true,
            Text = "VietFlow - Bộ gõ tiếng Việt hiệu năng cao"
        };

        _notifyIcon.MouseClick += (s, e) =>
        {
            if (e.Button == MouseButtons.Left)
            {
                _onToggleLanguage();
            }
        };

        _notifyIcon.DoubleClick += (s, e) =>
        {
            _onOpenSettings();
        };

        BuildContextMenu();
        UpdateIcon(_config.IsEnabled);
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
        var menu = new ContextMenuStrip();
        bool en = _config.UiLanguage == "en";

        string hotkeyDesc = GetHotkeyDisplay(_config.ToggleHotkey, _config.UiLanguage);
        string toggleText = en ? $"Vietnamese Mode ({hotkeyDesc})" : $"Bật tiếng Việt ({hotkeyDesc})";
        _toggleItem = new ToolStripMenuItem(toggleText, null, (s, e) => _onToggleLanguage());
        menu.Items.Add(_toggleItem);

        menu.Items.Add(new ToolStripSeparator());

        // Bảng mã Submenu
        var charsetMenu = new ToolStripMenuItem(en ? "Character Set" : "Bảng mã");
        AddCharsetItem(charsetMenu, CharsetId.Unicode, en ? "1. Unicode Precomposed (NFC)" : "1. Unicode dựng sẵn (NFC)");
        AddCharsetItem(charsetMenu, CharsetId.UnicodeComposite, en ? "2. Unicode Composite (NFD)" : "2. Unicode tổ hợp (NFD)");
        AddCharsetItem(charsetMenu, CharsetId.Tcvn3, "3. TCVN3 (ABC)");
        AddCharsetItem(charsetMenu, CharsetId.VniWindows, "4. VNI Windows");
        menu.Items.Add(charsetMenu);

        // Kiểu gõ Submenu
        var methodMenu = new ToolStripMenuItem(en ? "Input Method" : "Kiểu gõ");
        AddMethodItem(methodMenu, InputMethod.Telex, "Telex");
        AddMethodItem(methodMenu, InputMethod.SimpleTelex, "Simple Telex");
        AddMethodItem(methodMenu, InputMethod.Vni, "VNI");
        AddMethodItem(methodMenu, InputMethod.MicrosoftVi, "Microsoft Vietnamese");
        AddMethodItem(methodMenu, InputMethod.Viqr, "VIQR");
        menu.Items.Add(methodMenu);

        menu.Items.Add(new ToolStripSeparator());

        string clipText = en ? "Clipboard Toolkit (Ctrl+Shift+F9)" : "Chuyển mã Clipboard (Ctrl+Shift+F9)";
        string settingsText = en ? "Settings... (Ctrl+Shift+F5)" : "Bảng điều khiển... (Ctrl+Shift+F5)";
        string exitText = en ? "Exit" : "Thoát";

        menu.Items.Add(new ToolStripMenuItem(clipText, null, (s, e) => _onQuickConvert()));
        menu.Items.Add(new ToolStripMenuItem(settingsText, null, (s, e) => _onOpenSettings()));

        menu.Items.Add(new ToolStripSeparator());
        menu.Items.Add(new ToolStripMenuItem(exitText, null, (s, e) => _onExit()));

        _notifyIcon.ContextMenuStrip = menu;
    }

    private void AddCharsetItem(ToolStripMenuItem parent, CharsetId id, string text)
    {
        var item = new ToolStripMenuItem(text, null, (s, e) => _onSelectCharset(id))
        {
            Checked = _config.Charset == id
        };
        _charsetItems[id] = item;
        parent.DropDownItems.Add(item);
    }

    private void AddMethodItem(ToolStripMenuItem parent, InputMethod method, string text)
    {
        var item = new ToolStripMenuItem(text, null, (s, e) => _onSelectMethod(method))
        {
            Checked = _config.Method == method
        };
        _methodItems[method] = item;
        parent.DropDownItems.Add(item);
    }

    public void UpdateState(bool isEnabled, InputMethod method, CharsetId charset)
    {
        UpdateIcon(isEnabled);

        if (_toggleItem != null)
        {
            string hotkeyDesc = _config.ToggleHotkey == "AltZ" ? "Alt+Z" : "Ctrl+Shift";
            _toggleItem.Text = isEnabled ? $"Đang bật tiếng Việt ({hotkeyDesc})" : $"Đang tắt tiếng Việt ({hotkeyDesc})";
            _toggleItem.Checked = isEnabled;
        }

        foreach (var (m, item) in _methodItems)
        {
            item.Checked = (m == method);
        }

        foreach (var (c, item) in _charsetItems)
        {
            item.Checked = (c == charset);
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
        string mode = isEnabled
            ? (en ? "Vietnamese [V]" : "Tiếng Việt [V]")
            : "English [E]";
        _notifyIcon.Text = $"VietFlow · {mode} · {methodStr}";
    }

    public void UpdateIcon(bool isVietnamese)
    {
        // Draw at the exact size the shell uses (16x16 at 100%, 20/24 at higher DPI).
        // Rendering at 32x32 and letting Windows downscale was the cause of the blurry icon.
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

            // Fill the whole 16px cell; the shell renders it at the exact small-icon size.
            var rect = new RectangleF(0.5f, 0.5f, size - 1f, size - 1f);
            int radius = Math.Max(3, size / 5);
            using var path = CreateRoundedRectanglePath(rect, radius);
            using var brush = new LinearGradientBrush(new Rectangle(0, 0, size, size), topColor, bottomColor, 90f);
            g.FillPath(brush, path);

            // Subtle dark edge keeps the badge separated from light taskbars without eating the glyph.
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

    /// <summary>
    /// Draws the state letter as a filled vector outline so it stays sharp at 16px.
    /// </summary>
    private static void DrawBadgeLetter(Graphics g, string letter, int size)
    {
        using var family = new FontFamily("Segoe UI");
        using var path = new GraphicsPath();

        // Build the glyph large, then scale it down: this keeps the outline smooth.
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


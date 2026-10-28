// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using Avalonia;
using Avalonia.Controls;
using Avalonia.Markup.Xaml;
using Avalonia.Media;

namespace VietFlow.Windows;

public partial class TrayIconDialog : Window
{
    private string _vnTop;
    private string _vnBottom;
    private string _enTop;
    private string _enBottom;

    public TrayIconDialog(string vnTop, string vnBottom, string enTop, string enBottom)
    {
        InitializeComponent();

        _vnTop = Normalize(vnTop, "#EF4444");
        _vnBottom = Normalize(vnBottom, "#B91C1C");
        _enTop = Normalize(enTop, "#2563EB");
        _enBottom = Normalize(enBottom, "#1D4ED8");

        CpVnTop.Color = Parse(_vnTop);
        CpVnBottom.Color = Parse(_vnBottom);
        CpEnTop.Color = Parse(_enTop);
        CpEnBottom.Color = Parse(_enBottom);

        CpVnTop.ColorChanged += (_, e) => { _vnTop = ToHex(e.NewColor); Refresh(); };
        CpVnBottom.ColorChanged += (_, e) => { _vnBottom = ToHex(e.NewColor); Refresh(); };
        CpEnTop.ColorChanged += (_, e) => { _enTop = ToHex(e.NewColor); Refresh(); };
        CpEnBottom.ColorChanged += (_, e) => { _enBottom = ToHex(e.NewColor); Refresh(); };

        Refresh();
    }

    public string VnTop => _vnTop;
    public string VnBottom => _vnBottom;
    public string EnTop => _enTop;
    public string EnBottom => _enBottom;

    private void Refresh()
    {
        BadgeV.Background = CreateGradient(_vnTop, _vnBottom);
        BadgeE.Background = CreateGradient(_enTop, _enBottom);

        TxtBadgeV.Foreground = LetterBrush(Parse(_vnTop), Parse(_vnBottom));
        TxtBadgeE.Foreground = LetterBrush(Parse(_enTop), Parse(_enBottom));
    }

    private static IBrush LetterBrush(Color top, Color bottom)
    {
        double luminance = (0.299 * (top.R + bottom.R) +
                            0.587 * (top.G + bottom.G) +
                            0.114 * (top.B + bottom.B)) / 2.0;
        return luminance > 165
            ? new SolidColorBrush(Color.FromRgb(17, 24, 39))
            : Brushes.White;
    }

    private static IBrush CreateGradient(string top, string bottom)
    {
        return new LinearGradientBrush
        {
            StartPoint = new RelativePoint(0, 0, RelativeUnit.Relative),
            EndPoint = new RelativePoint(0, 1, RelativeUnit.Relative),
            GradientStops =
            {
                new GradientStop(Parse(top), 0),
                new GradientStop(Parse(bottom), 1)
            }
        };
    }

    private static Color Parse(string value)
    {
        try
        {
            return Color.Parse(value);
        }
        catch
        {
            return Colors.Gray;
        }
    }

    private static string ToHex(Color color) => $"#{color.R:X2}{color.G:X2}{color.B:X2}";

    private static string Normalize(string? value, string fallback)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return fallback;
        }

        try
        {
            _ = Color.Parse(value);
            return value;
        }
        catch
        {
            return fallback;
        }
    }

    private void BtnSave_Click(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
    {
        Close(true);
    }

    private void BtnCancel_Click(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
    {
        Close(false);
    }
}

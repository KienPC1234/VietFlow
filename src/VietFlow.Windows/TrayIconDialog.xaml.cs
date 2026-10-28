// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Windows;
using WinFormsColorDialog = System.Windows.Forms.ColorDialog;
using WinFormsDialogResult = System.Windows.Forms.DialogResult;
using MediaBrush = System.Windows.Media.Brush;
using MediaColor = System.Windows.Media.Color;
using MediaColorConverter = System.Windows.Media.ColorConverter;
using MediaColors = System.Windows.Media.Colors;
using MediaGradientBrush = System.Windows.Media.LinearGradientBrush;

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

        Refresh();
    }

    public string VnTop => _vnTop;
    public string VnBottom => _vnBottom;
    public string EnTop => _enTop;
    public string EnBottom => _enBottom;

    private void Refresh()
    {
        BtnVnTop.Content = _vnTop;
        BtnVnBottom.Content = _vnBottom;
        BtnEnTop.Content = _enTop;
        BtnEnBottom.Content = _enBottom;

        BadgeV.Background = CreateGradient(_vnTop, _vnBottom);
        BadgeE.Background = CreateGradient(_enTop, _enBottom);
    }

    private static MediaBrush CreateGradient(string top, string bottom)
    {
        return new MediaGradientBrush(Parse(top), Parse(bottom), 90);
    }

    private static MediaColor Parse(string value)
    {
        try
        {
            return (MediaColor)MediaColorConverter.ConvertFromString(value);
        }
        catch
        {
            return MediaColors.Gray;
        }
    }

    private static string Normalize(string? value, string fallback)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return fallback;
        }

        try
        {
            _ = (MediaColor)MediaColorConverter.ConvertFromString(value);
            return value;
        }
        catch
        {
            return fallback;
        }
    }

    private string? PickColor(string current)
    {
        using var dialog = new WinFormsColorDialog
        {
            FullOpen = true,
            AnyColor = true,
            Color = System.Drawing.ColorTranslator.FromHtml(current)
        };

        if (dialog.ShowDialog() != WinFormsDialogResult.OK)
        {
            return null;
        }

        return $"#{dialog.Color.R:X2}{dialog.Color.G:X2}{dialog.Color.B:X2}";
    }

    private void BtnVnTop_Click(object sender, RoutedEventArgs e)
    {
        var value = PickColor(_vnTop);
        if (value != null) { _vnTop = value; Refresh(); }
    }

    private void BtnVnBottom_Click(object sender, RoutedEventArgs e)
    {
        var value = PickColor(_vnBottom);
        if (value != null) { _vnBottom = value; Refresh(); }
    }

    private void BtnEnTop_Click(object sender, RoutedEventArgs e)
    {
        var value = PickColor(_enTop);
        if (value != null) { _enTop = value; Refresh(); }
    }

    private void BtnEnBottom_Click(object sender, RoutedEventArgs e)
    {
        var value = PickColor(_enBottom);
        if (value != null) { _enBottom = value; Refresh(); }
    }

    private void BtnSave_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = true;
        Close();
    }

    private void BtnCancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
        Close();
    }
}

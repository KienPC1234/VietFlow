// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Markup.Xaml;
using VietFlow.Engine;
using VietFlow.Tools;

namespace VietFlow.Windows;

public partial class ConvertWindow : Window
{
    private readonly UiOption<CharsetId>[] _charsets;

    public ConvertWindow()
    {
        InitializeComponent();

        _charsets = new[]
        {
            new UiOption<CharsetId>(CharsetId.Unicode, "Unicode (NFC)"),
            new UiOption<CharsetId>(CharsetId.UnicodeComposite, "Unicode (NFD)"),
            new UiOption<CharsetId>(CharsetId.Tcvn3, "TCVN3 (ABC)"),
            new UiOption<CharsetId>(CharsetId.VniWindows, "VNI Windows"),
            new UiOption<CharsetId>(CharsetId.Viqr, "VIQR")
        };

        CboFrom.ItemsSource = _charsets;
        CboTo.ItemsSource = _charsets;
        CboFrom.SelectedItem = Find(CharsetId.Unicode);
        CboTo.SelectedItem = Find(CharsetId.Tcvn3);

        BtnPaste_Click(this, new RoutedEventArgs());
    }

    private UiOption<CharsetId> Find(CharsetId id) =>
        _charsets.First(option => option.Id == id);

    private void BtnPaste_Click(object? sender, RoutedEventArgs e)
    {
        try
        {
            TxtContent.Text = ClipboardHelper.GetText();
        }
        catch
        {
            // Ignore clipboard lock errors.
        }
    }

    private void BtnConvert_Click(object? sender, RoutedEventArgs e)
    {
        if (CboFrom.SelectedItem is not UiOption<CharsetId> from ||
            CboTo.SelectedItem is not UiOption<CharsetId> to)
        {
            return;
        }

        TxtContent.Text = ClipboardConverter.Convert(TxtContent.Text ?? string.Empty, from.Id, to.Id);
    }

    private void BtnCopy_Click(object? sender, RoutedEventArgs e)
    {
        try
        {
            ClipboardHelper.SetText(TxtContent.Text ?? string.Empty);
        }
        catch
        {
            // Ignore clipboard lock errors.
        }
    }

    private void BtnClose_Click(object? sender, RoutedEventArgs e) => Close();
}

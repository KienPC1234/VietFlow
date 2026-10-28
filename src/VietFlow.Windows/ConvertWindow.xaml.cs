// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Windows;
using VietFlow.Engine;
using VietFlow.Tools;
using WpfClipboard = System.Windows.Clipboard;

namespace VietFlow.Windows;

public partial class ConvertWindow : Window
{
    public ConvertWindow()
    {
        InitializeComponent();

        var charsets = new[]
        {
            new { Id = CharsetId.Unicode, Name = "Unicode (NFC)" },
            new { Id = CharsetId.UnicodeComposite, Name = "Unicode (NFD)" },
            new { Id = CharsetId.Tcvn3, Name = "TCVN3 (ABC)" },
            new { Id = CharsetId.VniWindows, Name = "VNI Windows" },
            new { Id = CharsetId.Viqr, Name = "VIQR" }
        };

        CboFrom.ItemsSource = charsets;
        CboFrom.DisplayMemberPath = "Name";
        CboFrom.SelectedValuePath = "Id";
        CboFrom.SelectedValue = CharsetId.Unicode;

        CboTo.ItemsSource = charsets;
        CboTo.DisplayMemberPath = "Name";
        CboTo.SelectedValuePath = "Id";
        CboTo.SelectedValue = CharsetId.Tcvn3;

        BtnPaste_Click(this, new RoutedEventArgs());
    }

    private void BtnPaste_Click(object sender, RoutedEventArgs e)
    {
        try
        {
            if (WpfClipboard.ContainsText())
            {
                TxtContent.Text = WpfClipboard.GetText();
            }
        }
        catch
        {
            // Ignore clipboard lock errors.
        }
    }

    private void BtnConvert_Click(object sender, RoutedEventArgs e)
    {
        if (CboFrom.SelectedValue is not CharsetId from || CboTo.SelectedValue is not CharsetId to)
        {
            return;
        }

        TxtContent.Text = ClipboardConverter.Convert(TxtContent.Text, from, to);
    }

    private void BtnCopy_Click(object sender, RoutedEventArgs e)
    {
        try
        {
            WpfClipboard.SetText(TxtContent.Text);
        }
        catch
        {
            // Ignore clipboard lock errors.
        }
    }

    private void BtnClose_Click(object sender, RoutedEventArgs e)
    {
        Close();
    }
}

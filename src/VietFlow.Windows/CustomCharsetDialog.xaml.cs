// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Collections.ObjectModel;
using System.Windows;
using VietFlow.Tables;

namespace VietFlow.Windows;

public partial class CustomCharsetDialog : Window
{
    public sealed class CharMapRow
    {
        public required string Glyph { get; init; }
        public string Code { get; set; } = string.Empty;
    }

    private readonly ObservableCollection<CharMapRow> _rows = new();

    public CustomCharsetDialog(IReadOnlyDictionary<string, string> existing)
    {
        InitializeComponent();

        for (int i = 0; i < CharsetTables.TotalVnChars; i++)
        {
            string glyph = CharsetTables.UnicodeTable[i].ToString();
            _rows.Add(new CharMapRow
            {
                Glyph = glyph,
                Code = existing.TryGetValue(glyph, out string? value) ? value : string.Empty
            });
        }

        DgMap.ItemsSource = _rows;
    }

    public Dictionary<string, string> Mappings { get; private set; } = new();

    private void BtnFillUnicode_Click(object sender, RoutedEventArgs e)
    {
        foreach (var row in _rows)
        {
            row.Code = row.Glyph;
        }
        DgMap.Items.Refresh();
    }

    private void BtnClear_Click(object sender, RoutedEventArgs e)
    {
        foreach (var row in _rows)
        {
            row.Code = string.Empty;
        }
        DgMap.Items.Refresh();
    }

    private void BtnSave_Click(object sender, RoutedEventArgs e)
    {
        // Commit the cell being edited before reading the rows.
        DgMap.CommitEdit(System.Windows.Controls.DataGridEditingUnit.Cell, true);
        DgMap.CommitEdit(System.Windows.Controls.DataGridEditingUnit.Row, true);

        var map = new Dictionary<string, string>();
        foreach (var row in _rows)
        {
            string code = row.Code?.Trim() ?? string.Empty;
            if (code.Length > 0)
            {
                map[row.Glyph] = code;
            }
        }

        Mappings = map;
        DialogResult = true;
        Close();
    }

    private void BtnCancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
        Close();
    }
}

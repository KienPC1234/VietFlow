// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Markup.Xaml;
using VietFlow.Tables;

namespace VietFlow.Windows;

public partial class CustomCharsetDialog : Window
{
    public sealed class CharMapRow : INotifyPropertyChanged
    {
        private string _code = string.Empty;

        public required string Glyph { get; init; }

        public string Code
        {
            get => _code;
            set
            {
                if (_code == value)
                {
                    return;
                }

                _code = value;
                PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Code)));
            }
        }

        public event PropertyChangedEventHandler? PropertyChanged;
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

    private void BtnFillUnicode_Click(object? sender, RoutedEventArgs e)
    {
        foreach (var row in _rows)
        {
            row.Code = row.Glyph;
        }
    }

    private void BtnClear_Click(object? sender, RoutedEventArgs e)
    {
        foreach (var row in _rows)
        {
            row.Code = string.Empty;
        }
    }

    private void BtnSave_Click(object? sender, RoutedEventArgs e)
    {
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
        Close(true);
    }

    private void BtnCancel_Click(object? sender, RoutedEventArgs e) => Close(false);
}

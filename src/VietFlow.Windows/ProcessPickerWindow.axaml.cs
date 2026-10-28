// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Diagnostics;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Markup.Xaml;

namespace VietFlow.Windows;

public partial class ProcessPickerWindow : Window
{
    private List<ProcessInfo> _all = new();

    public ProcessPickerWindow(string lang = "vi")
    {
        InitializeComponent();

        bool en = Localization.IsEnglish(lang);
        Title = en ? "Select a running process" : "Chọn tiến trình đang chạy";
        LblPickerTitle.Text = en ? "Select a running app or game" : "Chọn ứng dụng hoặc game đang chạy";
        LblPickerDesc.Text = en
            ? "The selected app will be added to the excluded list."
            : "Ứng dụng được chọn sẽ được thêm vào danh sách loại trừ.";
        TxtSearch.PlaceholderText = en ? "Search by app name or .exe" : "Tìm theo tên ứng dụng hoặc file .exe";
        BtnCancel.Content = en ? "Cancel" : "Hủy";
        BtnOk.Content = en ? "Add" : "Thêm";

        LoadProcesses();
    }

    public ProcessInfo? SelectedProcess { get; private set; }
    public string? SelectedExeName => SelectedProcess?.Exe;

    private void LoadProcesses()
    {
        var byExe = new Dictionary<string, ProcessInfo>(StringComparer.OrdinalIgnoreCase);
        foreach (var process in Process.GetProcesses())
        {
            try
            {
                var info = ProcessInfo.FromProcess(process);
                if (!string.IsNullOrWhiteSpace(info.Exe) && !byExe.ContainsKey(info.Exe))
                {
                    byExe[info.Exe] = info;
                }
            }
            catch
            {
                // Some system processes cannot be inspected.
            }
            finally
            {
                process.Dispose();
            }
        }

        _all = byExe.Values
            .OrderBy(item => item.DisplayName, StringComparer.CurrentCultureIgnoreCase)
            .ThenBy(item => item.Exe, StringComparer.OrdinalIgnoreCase)
            .ToList();

        RefreshList();
    }

    private void RefreshList()
    {
        string query = TxtSearch.Text?.Trim() ?? string.Empty;
        LstProcesses.ItemsSource = string.IsNullOrEmpty(query)
            ? _all
            : _all.Where(item =>
                    item.DisplayName.Contains(query, StringComparison.CurrentCultureIgnoreCase) ||
                    item.Exe.Contains(query, StringComparison.OrdinalIgnoreCase))
                .ToList();
    }

    private void TxtSearch_TextChanged(object? sender, TextChangedEventArgs e) => RefreshList();

    private void BtnOk_Click(object? sender, RoutedEventArgs e)
    {
        if (LstProcesses.SelectedItem is ProcessInfo item)
        {
            SelectedProcess = item;
            Close(item);
            return;
        }

        Close(null);
    }

    private void BtnCancel_Click(object? sender, RoutedEventArgs e) => Close(null);
}

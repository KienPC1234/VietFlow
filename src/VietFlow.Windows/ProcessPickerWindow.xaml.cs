// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;

namespace VietFlow.Windows;

public sealed class RunningProcessItem
{
    public string ExeName { get; set; } = string.Empty;
    public string WindowTitle { get; set; } = string.Empty;
    public int Pid { get; set; }
}

public partial class ProcessPickerWindow : Window
{
    private readonly ObservableCollection<RunningProcessItem> _items = new();
    private ICollectionView _collectionView = null!;

    public string? SelectedExeName { get; private set; }

    public ProcessPickerWindow()
    {
        InitializeComponent();
        DgProcesses.ItemsSource = _items;
        _collectionView = CollectionViewSource.GetDefaultView(_items);
        _collectionView.Filter = FilterProcess;
        LoadRunningProcesses();
    }

    private void LoadRunningProcesses()
    {
        _items.Clear();
        int currentPid = Environment.ProcessId;

        var set = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        try
        {
            var processes = Process.GetProcesses();
            foreach (var p in processes)
            {
                try
                {
                    if (p.Id == currentPid || p.MainWindowHandle == IntPtr.Zero)
                    {
                        continue;
                    }

                    string title = p.MainWindowTitle;
                    if (string.IsNullOrWhiteSpace(title))
                    {
                        continue;
                    }

                    string exe = p.ProcessName + ".exe";
                    string key = $"{exe}_{title}";
                    if (!set.Add(key))
                    {
                        continue;
                    }

                    _items.Add(new RunningProcessItem
                    {
                        ExeName = exe,
                        WindowTitle = title,
                        Pid = p.Id
                    });
                }
                catch
                {
                    // Ignore inaccessible processes
                }
                finally
                {
                    p.Dispose();
                }
            }
        }
        catch
        {
        }

        UpdateCount();
    }

    private bool FilterProcess(object obj)
    {
        if (obj is not RunningProcessItem item) return false;
        string query = TxtSearch.Text.Trim();
        if (string.IsNullOrEmpty(query)) return true;

        return item.ExeName.Contains(query, StringComparison.OrdinalIgnoreCase) ||
               item.WindowTitle.Contains(query, StringComparison.OrdinalIgnoreCase);
    }

    private void TxtSearch_TextChanged(object sender, TextChangedEventArgs e)
    {
        _collectionView.Refresh();
        UpdateCount();
    }

    private void BtnRefresh_Click(object sender, RoutedEventArgs e)
    {
        LoadRunningProcesses();
    }

    private void DgProcesses_MouseDoubleClick(object sender, System.Windows.Input.MouseButtonEventArgs e)
    {
        ConfirmSelection();
    }

    private void BtnSelect_Click(object sender, RoutedEventArgs e)
    {
        ConfirmSelection();
    }

    private void ConfirmSelection()
    {
        if (DgProcesses.SelectedItem is RunningProcessItem item)
        {
            SelectedExeName = item.ExeName;
            DialogResult = true;
            Close();
        }
    }

    private void BtnCancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
        Close();
    }

    private void UpdateCount()
    {
        int total = _collectionView.Cast<object>().Count();
        TxtProcessCount.Text = $"{total} tiến trình phù hợp";
    }
}


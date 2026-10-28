// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Avalonia.Threading;
using VietFlow.Configuration;
using VietFlow.Engine;
using VietFlow.Tools;

namespace VietFlow.Windows;

public partial class App : Application
{
    private const string MutexName = "Global\\VietFlow_SingleInstance_Mutex";

    private Mutex? _mutex;
    private bool _isOwned;
    private IClassicDesktopStyleApplicationLifetime _desktop = null!;

    private VietFlowConfig _config = null!;
    private VietFlowSession _session = null!;
    private ForegroundTracker _tracker = null!;
    private TrayIconManager _trayManager = null!;
    private KeyboardHook _hook = null!;
    private SettingsWindow? _settingsWindow;
    private string? _pendingTab;
    private string? _shotPath;
    private readonly UpdateService _updateService = new();

    public override void Initialize()
    {
        AvaloniaXamlLoader.Load(this);
    }

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is not IClassicDesktopStyleApplicationLifetime desktop)
        {
            base.OnFrameworkInitializationCompleted();
            return;
        }

        _desktop = desktop;
        desktop.ShutdownMode = ShutdownMode.OnExplicitShutdown;
        desktop.Exit += (_, _) => Cleanup();

        string[] args = desktop.Args ?? Array.Empty<string>();

        _mutex = new Mutex(true, MutexName, out _isOwned);
        if (!_isOwned)
        {
            if (args.Contains("--restart", StringComparer.OrdinalIgnoreCase))
            {
                for (int i = 0; i < 50 && !_isOwned; i++)
                {
                    Thread.Sleep(100);
                    _isOwned = _mutex.WaitOne(0);
                }
            }

            if (!_isOwned)
            {
                base.OnFrameworkInitializationCompleted();
                var message = new MessageWindow(
                    "VietFlow",
                    "VietFlow đang chạy.\nVietFlow is already running.",
                    showCancel: false);
                message.Closed += (_, _) => desktop.Shutdown();
                message.Show();
                return;
            }
        }

        _config = VietFlowConfig.Load();
        ThemeManager.Apply(_config.ThemeMode);


        foreach (string arg in args)
        {
            if (arg.StartsWith("--tab=", StringComparison.OrdinalIgnoreCase))
            {
                _pendingTab = arg[6..];
            }
            else if (arg.StartsWith("--shot=", StringComparison.OrdinalIgnoreCase))
            {
                _shotPath = arg[7..];
            }



        }

        _session = new VietFlowSession(_config.Method, _config.Charset, _config.ModernStyle);
        _config.ApplyToSession(_session);

        // Dynamic variable @clipboard provider for macros.
        _session.Macros.ClipboardProvider = ClipboardHelper.GetText;

        _tracker = new ForegroundTracker(_config);

        _trayManager = new TrayIconManager(
            _config,
            onToggleLanguage: ToggleLanguage,
            onSelectMethod: SelectMethod,
            onSelectCharset: SelectCharset,
            onOpenSettingsTab: OpenSettings,
            onQuickConvert: QuickClipboardConvert,
            onOpenConverter: OpenConverter,
            onCustomizeTray: CustomizeTrayIcon,
            onRestart: RestartApplication,
            onRestartAdmin: RestartAsAdmin,
            onExit: ExitApplication);

        _hook = new KeyboardHook(
            _session,
            _config,
            _tracker,
            onToggleLanguage: ToggleLanguage,
            onOpenSettings: () => OpenSettings(null),
            onQuickConvert: QuickClipboardConvert);

        _trayManager.UpdateState(_config.IsEnabled, _config.Method, _config.Charset);

        var memTimer = new DispatcherTimer { Interval = TimeSpan.FromMinutes(5) };
        memTimer.Tick += (_, _) =>
        {
            if (_settingsWindow == null)
            {
                NativeMethods.TrimProcessMemory();
            }
        };
        memTimer.Start();

        Dispatcher.UIThread.Post(NativeMethods.TrimProcessMemory, DispatcherPriority.ApplicationIdle);

        if (args.Contains("--settings", StringComparer.OrdinalIgnoreCase) || _pendingTab != null)
        {
            OpenSettings(null);
        }

        base.OnFrameworkInitializationCompleted();
    }

    private void ToggleLanguage()
    {
        _config.IsEnabled = !_config.IsEnabled;
        _config.Save();
        _session.IsEnabled = _config.IsEnabled;

        Dispatcher.UIThread.Post(() =>
        {
            _trayManager.UpdateState(_config.IsEnabled, _config.Method, _config.Charset);
            _settingsWindow?.RefreshLiveState();

            if (_config.NotificationSound)
            {
                try
                {
                    NativeMethods.MessageBeep(NativeMethods.MB_ICONASTERISK);
                }
                catch
                {
                    // Sound is best-effort.
                }
            }
        });
    }

    private void SelectMethod(InputMethod method)
    {
        _config.Method = method;
        _config.Save();
        _session.Method = method;
        _trayManager.UpdateState(_config.IsEnabled, _config.Method, _config.Charset);
    }

    private void SelectCharset(CharsetId charset)
    {
        _config.Charset = charset;
        _config.Save();
        _session.Charset = Charsets.CharsetFactory.GetCharset(charset);
        _trayManager.UpdateState(_config.IsEnabled, _config.Method, _config.Charset);
    }

    private void OpenSettings(string? tab = null)
    {
        Dispatcher.UIThread.Post(() =>
        {
            if (_settingsWindow == null)
            {
                _settingsWindow = new SettingsWindow(_config, _session, () =>
                {
                    _trayManager.UpdateState(_config.IsEnabled, _config.Method, _config.Charset);
                    _trayManager.RefreshStartupState();
                    _tracker.RefreshExcludedApps();
                }, _updateService);
                _settingsWindow.Closed += (_, _) =>
                {
                    _settingsWindow = null;
                    NativeMethods.TrimProcessMemory();
                };
            }

            if (!_settingsWindow.IsVisible)
            {
                _settingsWindow.Show();
            }

            _settingsWindow.WindowState = WindowState.Normal;
            _settingsWindow.Activate();

            string? target = tab ?? _pendingTab;
            if (!string.IsNullOrEmpty(target))
            {
                _settingsWindow.ShowTab(target);
                _pendingTab = null;
            }

            if (!string.IsNullOrEmpty(_shotPath))
            {
                string path = _shotPath;
                _shotPath = null;
                Dispatcher.UIThread.Post(() =>
                {
                    _settingsWindow?.CaptureTo(path);
                    _desktop.Shutdown();
                }, DispatcherPriority.Loaded);
            }
        });
    }

    private void QuickClipboardConvert()
    {
        Dispatcher.UIThread.Post(() =>
        {
            try
            {
                string text = ClipboardHelper.GetText();
                if (!string.IsNullOrEmpty(text))
                {
                    string converted = ClipboardConverter.StripAccents(text);
                    ClipboardHelper.SetText(converted);
                    _trayManager.ShowBalloon(
                        "VietFlow Clipboard Toolkit",
                        "Đã loại bỏ dấu tiếng Việt trong bộ nhớ tạm thành công!");
                }
            }
            catch (Exception ex)
            {
                _trayManager.ShowBalloon("VietFlow lỗi", $"Không thể chuyển mã clipboard: {ex.Message}");
            }
        });
    }

    private void OpenConverter()
    {
        Dispatcher.UIThread.Post(() =>
        {
            var window = new ConvertWindow();
            if (_settingsWindow != null)
            {
                window.ShowDialog(_settingsWindow);
            }
            else
            {
                window.Show();
            }
        });
    }

    private async void CustomizeTrayIcon()
    {
        await Dispatcher.UIThread.InvokeAsync(async () =>
        {
            if (_settingsWindow == null)
            {
                OpenSettings(null);
                await Task.Delay(200);
            }

            if (_settingsWindow == null)
            {
                return;
            }

            var dialog = new TrayIconDialog(
                _config.TrayVnTop,
                _config.TrayVnBottom,
                _config.TrayEnTop,
                _config.TrayEnBottom);

            bool? result = await dialog.ShowDialog<bool?>(_settingsWindow);
            if (result != true)
            {
                return;
            }

            _config.TrayVnTop = dialog.VnTop;
            _config.TrayVnBottom = dialog.VnBottom;
            _config.TrayEnTop = dialog.EnTop;
            _config.TrayEnBottom = dialog.EnBottom;
            _config.Save();

            _trayManager.UpdateState(_config.IsEnabled, _config.Method, _config.Charset);
            _settingsWindow.RefreshTrayPreview();
        });
    }

    private void RestartApplication()
    {
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = Environment.ProcessPath!,
                Arguments = "--restart",
                UseShellExecute = true
            });
        }
        catch
        {
            // Ignore restart failures.
        }

        ExitApplication();
    }

    private void RestartAsAdmin()
    {
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = Environment.ProcessPath!,
                Arguments = "--restart",
                UseShellExecute = true,
                Verb = "runas"
            });
        }
        catch
        {
            _trayManager.ShowBalloon("VietFlow", "Không thể khởi động lại với quyền Admin (UAC bị huỷ).");
            return;
        }

        ExitApplication();
    }

    private void ExitApplication()
    {
        _hook?.Dispose();
        _trayManager?.Dispose();

        if (_isOwned && _mutex != null)
        {
            try
            {
                _mutex.ReleaseMutex();
            }
            catch
            {
                // Ignore mutex release failures.
            }
            _mutex.Dispose();
        }

        _desktop.Shutdown();
    }

    private void Cleanup()
    {
        try
        {
            _hook?.Dispose();
            _trayManager?.Dispose();
        }
        catch
        {
            // Ignore shutdown failures.
        }
    }
}

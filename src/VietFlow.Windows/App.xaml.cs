// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.IO;
using System.Threading;
using System.Windows;
using VietFlow.Configuration;
using VietFlow.Engine;
using VietFlow.Tools;
using WpfClipboard = System.Windows.Clipboard;

namespace VietFlow.Windows;

public partial class App : System.Windows.Application
{
    private const string MutexName = "Global\\VietFlow_SingleInstance_Mutex";
    private Mutex? _mutex;
    private bool _isOwned;

    private VietFlowConfig _config = null!;
    private VietFlowSession _session = null!;
    private ForegroundTracker _tracker = null!;
    private TrayIconManager _trayManager = null!;
    private KeyboardHook _hook = null!;
    private SettingsWindow? _settingsWindow;
    private string? _pendingTab;
    private string? _shotPath;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        _mutex = new Mutex(true, MutexName, out _isOwned);
        if (!_isOwned)
        {
            // Another instance of VietFlow is already active
            Shutdown();
            return;
        }

        _config = VietFlowConfig.Load();
        ThemeManager.Apply(_config.ThemeMode);

        foreach (var arg in e.Args)
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

        // Dynamic variable @clipboard provider for macros
        _session.Macros.ClipboardProvider = () =>
        {
            string text = string.Empty;
            Dispatcher.Invoke(() =>
            {
                try
                {
                    if (WpfClipboard.ContainsText())
                    {
                        text = WpfClipboard.GetText();
                    }
                }
                catch
                {
                    // Ignore clipboard lock errors
                }
            });
            return text;
        };

        _tracker = new ForegroundTracker(_config);

        _trayManager = new TrayIconManager(
            _config,
            onToggleLanguage: ToggleLanguage,
            onSelectMethod: SelectMethod,
            onSelectCharset: SelectCharset,
            onOpenSettings: OpenSettings,
            onQuickConvert: QuickClipboardConvert,
            onExit: ExitApplication);

        _hook = new KeyboardHook(
            _session,
            _config,
            _tracker,
            onToggleLanguage: ToggleLanguage,
            onOpenSettings: OpenSettings,
            onQuickConvert: QuickClipboardConvert);

        _trayManager.UpdateState(_config.IsEnabled, _config.Method, _config.Charset);

        // Periodic background memory trimmer (every 5 minutes)
        var memTimer = new System.Windows.Threading.DispatcherTimer
        {
            Interval = TimeSpan.FromMinutes(5)
        };
        memTimer.Tick += (_, _) =>
        {
            if (_settingsWindow == null)
            {
                NativeMethods.TrimProcessMemory();
            }
        };
        memTimer.Start();

        Dispatcher.BeginInvoke(System.Windows.Threading.DispatcherPriority.ApplicationIdle, new Action(NativeMethods.TrimProcessMemory));

        if (e.Args.Contains("--settings", StringComparer.OrdinalIgnoreCase) || _pendingTab != null)
        {
            Dispatcher.BeginInvoke(new Action(OpenSettings));
        }
    }

    private void ToggleLanguage()
    {
        _config.IsEnabled = !_config.IsEnabled;
        _config.Save();
        _session.IsEnabled = _config.IsEnabled;
        _trayManager.UpdateState(_config.IsEnabled, _config.Method, _config.Charset);

        if (_config.NotificationSound)
        {
            try
            {
                System.Media.SystemSounds.Asterisk.Play();
            }
            catch
            {
                // Sound is best-effort.
            }
        }

        Dispatcher.Invoke(() =>
        {
            _settingsWindow?.RefreshLiveState();
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

    private void OpenSettings()
    {
        Dispatcher.Invoke(() =>
        {
            if (_settingsWindow == null)
            {
                _settingsWindow = new SettingsWindow(_config, _session, () =>
                {
                    _trayManager.UpdateState(_config.IsEnabled, _config.Method, _config.Charset);
                });
                _settingsWindow.Closed += (_, _) =>
                {
                    _settingsWindow = null;
                    NativeMethods.TrimProcessMemory();
                };
            }

            _settingsWindow.Show();
            _settingsWindow.WindowState = WindowState.Normal;
            _settingsWindow.Activate();

            if (!string.IsNullOrEmpty(_pendingTab))
            {
                _settingsWindow.ShowTab(_pendingTab);
                _pendingTab = null;
            }

            if (!string.IsNullOrEmpty(_shotPath))
            {
                string path = _shotPath;
                _shotPath = null;
                Dispatcher.BeginInvoke(new Action(() => CaptureWindow(path)), System.Windows.Threading.DispatcherPriority.ApplicationIdle);
            }
        });
    }

    private void CaptureWindow(string path)
    {
        if (_settingsWindow == null)
        {
            return;
        }

        try
        {
            _settingsWindow.UpdateLayout();
            int width = (int)Math.Ceiling(_settingsWindow.ActualWidth);
            int height = (int)Math.Ceiling(_settingsWindow.ActualHeight);
            var bitmap = new System.Windows.Media.Imaging.RenderTargetBitmap(width, height, 96, 96, System.Windows.Media.PixelFormats.Pbgra32);
            bitmap.Render(_settingsWindow);

            var encoder = new System.Windows.Media.Imaging.PngBitmapEncoder();
            encoder.Frames.Add(System.Windows.Media.Imaging.BitmapFrame.Create(bitmap));
            using var stream = File.Create(path);
            encoder.Save(stream);
        }
        catch
        {
            // Screenshot mode is best-effort.
        }

        Shutdown();
    }

    private void QuickClipboardConvert()
    {
        Dispatcher.Invoke(() =>
        {
            try
            {
                if (WpfClipboard.ContainsText())
                {
                    string text = WpfClipboard.GetText();
                    if (!string.IsNullOrEmpty(text))
                    {
                        // Convert clipboard content (Unicode <-> TCVN3 / VNI-Windows or strip accents)
                        string converted = ClipboardConverter.StripAccents(text);
                        WpfClipboard.SetText(converted);
                        _trayManager.ShowBalloon("VietFlow Clipboard Toolkit", "Đã loại bỏ dấu tiếng Việt trong bộ nhớ tạm thành công!");
                    }
                }
            }
            catch (Exception ex)
            {
                _trayManager.ShowBalloon("VietFlow Lỗi", $"Không thể chuyển mã clipboard: {ex.Message}", System.Windows.Forms.ToolTipIcon.Warning);
            }
        });
    }

    private void ExitApplication()
    {
        _hook.Dispose();
        _trayManager.Dispose();

        if (_isOwned && _mutex != null)
        {
            _mutex.ReleaseMutex();
            _mutex.Dispose();
        }

        Shutdown();
    }

    protected override void OnExit(ExitEventArgs e)
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
                // Ignore if already released
            }
            _mutex.Dispose();
        }

        base.OnExit(e);
    }
}

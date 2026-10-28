// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using Microsoft.Win32;
using System.Windows;

namespace VietFlow.Windows;

/// <summary>
/// Swaps the Light/Dark resource dictionaries and follows the Windows app theme
/// when the configured mode is "System".
/// </summary>
internal static class ThemeManager
{
    public const string SystemMode = "System";
    public const string LightMode = "Light";
    public const string DarkMode = "Dark";

    private static string _mode = SystemMode;

    public static bool IsDark { get; private set; }

    public static event Action? Changed;

    static ThemeManager()
    {
        try
        {
            SystemEvents.UserPreferenceChanged += (_, e) =>
            {
                if (_mode == SystemMode &&
                    (e.Category == UserPreferenceCategory.General ||
                     e.Category == UserPreferenceCategory.Desktop))
                {
                    var app = System.Windows.Application.Current;
                    app?.Dispatcher.BeginInvoke(new Action(() => Apply(SystemMode)));
                }
            };
        }
        catch
        {
            // System events are unavailable in some hosting scenarios.
        }
    }

    public static string Normalize(string? mode) => mode switch
    {
        DarkMode => DarkMode,
        LightMode => LightMode,
        _ => SystemMode
    };

    public static void Apply(string? mode)
    {
        _mode = Normalize(mode);
        IsDark = _mode == DarkMode || (_mode == SystemMode && IsSystemDark());

        var app = System.Windows.Application.Current;
        if (app == null)
        {
            return;
        }

        var dictionaries = app.Resources.MergedDictionaries;
        ResourceDictionary? current = null;
        foreach (var dictionary in dictionaries)
        {
            string? source = dictionary.Source?.OriginalString;
            if (source != null && source.Contains("Themes/", StringComparison.OrdinalIgnoreCase))
            {
                current = dictionary;
                break;
            }
        }

        var theme = new ResourceDictionary
        {
            Source = new Uri(IsDark ? "Themes/Dark.xaml" : "Themes/Light.xaml", UriKind.Relative)
        };

        if (current != null)
        {
            int index = dictionaries.IndexOf(current);
            dictionaries[index] = theme;
        }
        else
        {
            dictionaries.Insert(0, theme);
        }

        Changed?.Invoke();
    }

    private static bool IsSystemDark()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(
                @"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
            return key?.GetValue("AppsUseLightTheme") is int value && value == 0;
        }
        catch
        {
            return false;
        }
    }
}

// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using Avalonia;
using Avalonia.Styling;

namespace VietFlow.Windows;

/// <summary>
/// Applies the configured theme to Avalonia's Fluent theme.
/// "System" follows the Windows theme through ThemeVariant.Default.
/// </summary>
internal static class ThemeManager
{
    public const string SystemMode = "System";
    public const string LightMode = "Light";
    public const string DarkMode = "Dark";

    public static string Normalize(string? mode) => mode switch
    {
        DarkMode => DarkMode,
        LightMode => LightMode,
        _ => SystemMode
    };

    public static void Apply(string? mode)
    {
        if (Application.Current == null)
        {
            return;
        }

        Application.Current.RequestedThemeVariant = Normalize(mode) switch
        {
            LightMode => ThemeVariant.Light,
            DarkMode => ThemeVariant.Dark,
            _ => ThemeVariant.Default
        };
    }
}

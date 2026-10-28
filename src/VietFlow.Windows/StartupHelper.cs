// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Diagnostics;
using Microsoft.Win32;

namespace VietFlow.Windows;

/// <summary>
/// Per-user Windows startup entry. HKCU Run works without elevation and is
/// much more reliable than a schtasks entry for a normal desktop app.
/// </summary>
internal static class StartupHelper
{
    private const string RunKeyPath = @"Software\Microsoft\Windows\CurrentVersion\Run";
    private const string ValueName = "VietFlow";
    private const string LegacyTaskName = "VietFlowAutoStart";

    public static bool IsEnabled()
    {
        if (LegacyTaskExists())
        {
            return true;
        }

        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(RunKeyPath, writable: false);
            if (key?.GetValue(ValueName) is not string value || string.IsNullOrWhiteSpace(value))
            {
                return false;
            }

            string target = Unquote(value);
            string current = Environment.ProcessPath ?? string.Empty;
            if (string.IsNullOrWhiteSpace(current))
            {
                return false;
            }

            return string.Equals(
                Path.GetFullPath(target),
                Path.GetFullPath(current),
                StringComparison.OrdinalIgnoreCase);
        }
        catch
        {
            return false;
        }
    }

    public static bool SetEnabled(bool enabled)
    {
        // Remove the old elevated scheduled-task entry first. If it cannot be
        // removed, do not create a second startup path that would launch twice.
        if (!RemoveLegacyScheduledTask())
        {
            return false;
        }

        try
        {
            using var key = Registry.CurrentUser.CreateSubKey(RunKeyPath, writable: true);
            if (key == null)
            {
                return false;
            }

            if (enabled)
            {
                string exePath = Environment.ProcessPath ?? string.Empty;
                if (string.IsNullOrWhiteSpace(exePath))
                {
                    return false;
                }

                key.SetValue(ValueName, $"\"{exePath}\"", RegistryValueKind.String);
            }
            else
            {
                key.DeleteValue(ValueName, throwOnMissingValue: false);
            }

            return true;
        }
        catch
        {
            return false;
        }
    }

    private static string Unquote(string value)
    {
        string trimmed = value.Trim();
        if (trimmed.Length >= 2 && trimmed[0] == '"')
        {
            int closing = trimmed.IndexOf('"', 1);
            if (closing > 1)
            {
                return trimmed[1..closing];
            }
        }

        return trimmed;
    }

    private static bool LegacyTaskExists()
    {
        try
        {
            return RunSchtasks($"/query /tn \"{LegacyTaskName}\"");
        }
        catch
        {
            return false;
        }
    }

    private static bool RemoveLegacyScheduledTask()
    {
        try
        {
            if (!LegacyTaskExists())
            {
                return true;
            }

            return RunSchtasks($"/delete /tn \"{LegacyTaskName}\" /f");
        }
        catch
        {
            return true;
        }
    }

    private static bool RunSchtasks(string arguments)
    {
        var psi = new ProcessStartInfo("schtasks.exe", arguments)
        {
            CreateNoWindow = true,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };

        using var process = Process.Start(psi);
        if (process == null)
        {
            return false;
        }

        if (!process.WaitForExit(2500))
        {
            try
            {
                process.Kill();
            }
            catch
            {
                // Ignore.
            }

            return false;
        }

        return process.ExitCode == 0;
    }
}

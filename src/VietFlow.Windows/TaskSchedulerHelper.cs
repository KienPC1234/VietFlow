// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Diagnostics;

namespace VietFlow.Windows;

internal static class TaskSchedulerHelper
{
    private const string TaskName = "VietFlowAutoStart";

    public static bool IsScheduledToRunAtLogon()
    {
        try
        {
            var psi = new ProcessStartInfo("schtasks.exe", $"/query /tn \"{TaskName}\"")
            {
                CreateNoWindow = true,
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true
            };
            using var proc = Process.Start(psi);
            if (proc == null) return false;
            proc.WaitForExit(2000);
            return proc.ExitCode == 0;
        }
        catch
        {
            return false;
        }
    }

    public static void SetRunAtLogon(bool enable)
    {
        try
        {
            if (enable)
            {
                string exePath = Environment.ProcessPath ?? Process.GetCurrentProcess().MainModule?.FileName ?? string.Empty;
                if (string.IsNullOrEmpty(exePath)) return;

                // schtasks /create /tn "VietFlowAutoStart" /tr "\"exePath\"" /sc onlogon /rl highest /f
                string args = $"/create /tn \"{TaskName}\" /tr \"\\\"{exePath}\\\"\" /sc onlogon /rl highest /f";
                RunSchtasks(args);
            }
            else
            {
                // schtasks /delete /tn "VietFlowAutoStart" /f
                string args = $"/delete /tn \"{TaskName}\" /f";
                RunSchtasks(args);
            }
        }
        catch
        {
            // Ignore execution errors if not elevated
        }
    }

    private static void RunSchtasks(string arguments)
    {
        var psi = new ProcessStartInfo("schtasks.exe", arguments)
        {
            CreateNoWindow = true,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        using var proc = Process.Start(psi);
        proc?.WaitForExit(3000);
    }
}


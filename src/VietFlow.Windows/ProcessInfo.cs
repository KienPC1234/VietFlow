// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Diagnostics;
using System.Drawing;
using System.Drawing.Imaging;
using Avalonia.Media.Imaging;
using AvaloniaBitmap = Avalonia.Media.Imaging.Bitmap;
using DrawingIcon = System.Drawing.Icon;

namespace VietFlow.Windows;

/// <summary>
/// Display metadata for an executable: process name, friendly name and shell icon.
/// </summary>
public sealed class ProcessInfo
{
    public required string Exe { get; init; }
    public required string DisplayName { get; init; }
    public AvaloniaBitmap? Icon { get; init; }

    public bool HasIcon => Icon != null;

    public bool ShowPlaceholder => Icon == null;

    public string Initial => string.IsNullOrWhiteSpace(DisplayName)
        ? "?"
        : DisplayName[..1].ToUpperInvariant();

    public override string ToString() => $"{DisplayName} ({Exe})";

    public static ProcessInfo FromProcess(Process process)
    {
        string exe = process.ProcessName + ".exe";
        string? path = null;
        try
        {
            path = process.MainModule?.FileName;
        }
        catch
        {
            // Some protected processes do not expose MainModule.
        }

        return FromPath(exe, path);
    }

    public static ProcessInfo FromExe(string exe)
    {
        string path = FindRunningPath(exe) ?? string.Empty;
        return FromPath(exe, path);
    }

    public static ProcessInfo FromPath(string exe, string? path)
    {
        string displayName = Path.GetFileNameWithoutExtension(exe);
        if (!string.IsNullOrWhiteSpace(path))
        {
            try
            {
                var version = FileVersionInfo.GetVersionInfo(path);
                if (!string.IsNullOrWhiteSpace(version.FileDescription))
                {
                    displayName = version.FileDescription!;
                }
                else if (!string.IsNullOrWhiteSpace(version.ProductName))
                {
                    displayName = version.ProductName!;
                }
            }
            catch
            {
                // Version resources are optional.
            }
        }

        return new ProcessInfo
        {
            Exe = exe,
            DisplayName = displayName,
            Icon = LoadIcon(path)
        };
    }

    private static string? FindRunningPath(string exe)
    {
        string processName = Path.GetFileNameWithoutExtension(exe);
        if (string.IsNullOrWhiteSpace(processName))
        {
            return null;
        }

        try
        {
            foreach (var process in Process.GetProcessesByName(processName))
            {
                try
                {
                    string? path = process.MainModule?.FileName;
                    if (!string.IsNullOrWhiteSpace(path))
                    {
                        return path;
                    }
                }
                catch
                {
                    // Ignore protected processes.
                }
                finally
                {
                    process.Dispose();
                }
            }
        }
        catch
        {
            // Process enumeration can fail on restricted systems.
        }

        return null;
    }

    private static AvaloniaBitmap? LoadIcon(string? path)
    {
        if (string.IsNullOrWhiteSpace(path) || !File.Exists(path))
        {
            return null;
        }

        try
        {
            using DrawingIcon? icon = DrawingIcon.ExtractAssociatedIcon(path);
            if (icon == null)
            {
                return null;
            }

            using var systemBitmap = icon.ToBitmap();
            using var stream = new MemoryStream();
            systemBitmap.Save(stream, ImageFormat.Png);
            stream.Position = 0;
            return new AvaloniaBitmap(stream);
        }
        catch
        {
            return null;
        }
    }
}

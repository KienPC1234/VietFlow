// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System;
using Avalonia;
using Velopack;

namespace VietFlow.Windows;

public static class Program
{
    [STAThread]
    public static void Main(string[] args)
    {
        // Velopack startup hook handler: handles install, update, and uninstall lifecycle events.
        VelopackApp.Build().Run();

        BuildAvaloniaApp().StartWithClassicDesktopLifetime(args);
    }

    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder.Configure<App>()
            .UsePlatformDetect()
            .LogToTrace();
}

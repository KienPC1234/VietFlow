// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Avalonia.Threading;

namespace VietFlow.Windows;

internal sealed class ToastWindow : Window
{
    private static ToastWindow? _current;

    private ToastWindow(string title, string message)
    {
        WindowDecorations = WindowDecorations.None;
        CanResize = false;
        ShowInTaskbar = false;
        ShowActivated = false;
        Topmost = true;
        Width = 340;
        SizeToContent = SizeToContent.Height;
        Background = new SolidColorBrush(Color.Parse("#111827"));

        Content = new Border
        {
            Background = new SolidColorBrush(Color.Parse("#111827")),
            BorderBrush = new SolidColorBrush(Color.Parse("#1F2937")),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(10),
            Padding = new Thickness(16, 12),
            Child = new StackPanel
            {
                Children =
                {
                    new TextBlock
                    {
                        Text = title,
                        FontSize = 13,
                        FontWeight = FontWeight.SemiBold,
                        Foreground = new SolidColorBrush(Color.Parse("#E5E7EB"))
                    },
                    new TextBlock
                    {
                        Text = message,
                        FontSize = 11,
                        TextWrapping = TextWrapping.Wrap,
                        Foreground = new SolidColorBrush(Color.Parse("#94A3B8")),
                        Margin = new Thickness(0, 4, 0, 0)
                    }
                }
            }
        };

        Opened += (_, _) =>
        {
            var screen = Screens.Primary;
            if (screen != null)
            {
                PixelRect area = screen.WorkingArea;
                Position = new PixelPoint(
                    Math.Max(area.X, area.Right - (int)Bounds.Width - 16),
                    Math.Max(area.Y, area.Bottom - (int)Bounds.Height - 16));
            }
        };

        var timer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(4) };
        timer.Tick += (_, _) =>
        {
            timer.Stop();
            Close();
        };
        timer.Start();
    }

    public static void Show(string title, string message)
    {
        Dispatcher.UIThread.Post(() =>
        {
            _current?.Close();
            var window = new ToastWindow(title, message);
            _current = window;
            window.Closed += (_, _) =>
            {
                if (ReferenceEquals(_current, window))
                {
                    _current = null;
                }
            };
            window.Show();
        });
    }
}

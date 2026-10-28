// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;

namespace VietFlow.Windows;

internal sealed class MessageWindow : Window
{
    public MessageWindow(string title, string message, bool showCancel)
    {
        Title = title;
        Width = 440;
        SizeToContent = SizeToContent.Height;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterScreen;
        ShowInTaskbar = false;
        Background = new SolidColorBrush(Color.Parse("#0F172A"));

        var text = new TextBlock
        {
            Text = message,
            TextWrapping = TextWrapping.Wrap,
            Foreground = new SolidColorBrush(Color.Parse("#E5E7EB")),
            FontSize = 13,
            Margin = new Thickness(0, 0, 0, 18)
        };

        var ok = new Button
        {
            Content = "OK",
            MinWidth = 84,
            HorizontalContentAlignment = HorizontalAlignment.Center
        };
        ok.Classes.Add("primary");
        ok.Click += (_, _) => Close(true);

        var buttons = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 8
        };

        if (showCancel)
        {
            var cancel = new Button { Content = "Cancel", MinWidth = 84 };
            cancel.Classes.Add("ghost");
            cancel.Click += (_, _) => Close(false);
            buttons.Children.Add(cancel);
        }

        buttons.Children.Add(ok);

        Content = new Border
        {
            Background = new SolidColorBrush(Color.Parse("#111827")),
            BorderBrush = new SolidColorBrush(Color.Parse("#1F2937")),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(10),
            Padding = new Thickness(20),
            Margin = new Thickness(12),
            Child = new StackPanel
            {
                Children =
                {
                    new TextBlock
                    {
                        Text = title,
                        FontSize = 15,
                        FontWeight = FontWeight.SemiBold,
                        Foreground = new SolidColorBrush(Color.Parse("#E5E7EB")),
                        Margin = new Thickness(0, 0, 0, 10)
                    },
                    text,
                    buttons
                }
            }
        };
    }

    public static Task<bool> ConfirmAsync(Window owner, string title, string message) =>
        new MessageWindow(title, message, showCancel: true).ShowDialog<bool>(owner);

    public static Task ShowInfoAsync(Window owner, string title, string message) =>
        new MessageWindow(title, message, showCancel: false).ShowDialog(owner);
}

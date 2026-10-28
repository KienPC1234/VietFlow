// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Runtime.InteropServices;

namespace VietFlow.Windows;

/// <summary>
/// Synchronous Win32 clipboard access. The low-level keyboard hook calls the
/// @clipboard macro provider off the UI thread, so Avalonia's async clipboard
/// API cannot be used there.
/// </summary>
internal static class ClipboardHelper
{
    private const uint CF_UNICODETEXT = 13;
    private const uint GMEM_MOVEABLE = 0x0002;

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool OpenClipboard(IntPtr hWndNewOwner);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseClipboard();

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsClipboardFormatAvailable(uint format);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr GetClipboardData(uint uFormat);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EmptyClipboard();

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SetClipboardData(uint uFormat, IntPtr hMem);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GlobalLock(IntPtr hMem);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GlobalUnlock(IntPtr hMem);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GlobalAlloc(uint uFlags, UIntPtr dwBytes);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GlobalFree(IntPtr hMem);

    public static string GetText()
    {
        if (!IsClipboardFormatAvailable(CF_UNICODETEXT))
        {
            return string.Empty;
        }

        for (int attempt = 0; attempt < 5; attempt++)
        {
            if (!OpenClipboard(IntPtr.Zero))
            {
                Thread.Sleep(10);
                continue;
            }

            try
            {
                IntPtr handle = GetClipboardData(CF_UNICODETEXT);
                if (handle == IntPtr.Zero)
                {
                    return string.Empty;
                }

                IntPtr pointer = GlobalLock(handle);
                if (pointer == IntPtr.Zero)
                {
                    return string.Empty;
                }

                try
                {
                    return Marshal.PtrToStringUni(pointer) ?? string.Empty;
                }
                finally
                {
                    GlobalUnlock(handle);
                }
            }
            finally
            {
                CloseClipboard();
            }
        }

        return string.Empty;
    }

    public static bool SetText(string text)
    {
        text ??= string.Empty;

        for (int attempt = 0; attempt < 5; attempt++)
        {
            if (!OpenClipboard(IntPtr.Zero))
            {
                Thread.Sleep(10);
                continue;
            }

            try
            {
                EmptyClipboard();

                int bytes = (text.Length + 1) * 2;
                IntPtr handle = GlobalAlloc(GMEM_MOVEABLE, (UIntPtr)bytes);
                if (handle == IntPtr.Zero)
                {
                    return false;
                }

                IntPtr pointer = GlobalLock(handle);
                if (pointer == IntPtr.Zero)
                {
                    GlobalFree(handle);
                    return false;
                }

                try
                {
                    Marshal.Copy(text.ToCharArray(), 0, pointer, text.Length);
                    Marshal.WriteInt16(pointer, text.Length * 2, 0);
                }
                finally
                {
                    GlobalUnlock(handle);
                }

                if (SetClipboardData(CF_UNICODETEXT, handle) == IntPtr.Zero)
                {
                    GlobalFree(handle);
                    return false;
                }

                return true;
            }
            finally
            {
                CloseClipboard();
            }
        }

        return false;
    }
}

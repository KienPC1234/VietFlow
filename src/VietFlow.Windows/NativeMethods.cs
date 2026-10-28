// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace VietFlow.Windows;

internal static partial class NativeMethods
{
    public const int WH_KEYBOARD_LL = 13;
    public const int WM_KEYDOWN = 0x0100;
    public const int WM_KEYUP = 0x0101;
    public const int WM_SYSKEYDOWN = 0x0104;
    public const int WM_SYSKEYUP = 0x0105;

    public const uint LLKHF_EXTENDED = 0x01;
    public const uint LLKHF_INJECTED = 0x10;
    public const uint LLKHF_ALTDOWN = 0x20;
    public const uint LLKHF_UP = 0x80;

    public const ushort VK_BACK = 0x08;
    public const ushort VK_TAB = 0x09;
    public const ushort VK_RETURN = 0x0D;
    public const ushort VK_SHIFT = 0x10;
    public const ushort VK_CONTROL = 0x11;
    public const ushort VK_MENU = 0x12; // Alt
    public const ushort VK_PAUSE = 0x13;
    public const ushort VK_CAPITAL = 0x14; // CapsLock
    public const ushort VK_ESCAPE = 0x1B;
    public const ushort VK_SPACE = 0x20;
    public const ushort VK_LSHIFT = 0xA0;
    public const ushort VK_RSHIFT = 0xA1;
    public const ushort VK_LCONTROL = 0xA2;
    public const ushort VK_RCONTROL = 0xA3;
    public const ushort VK_LMENU = 0xA4;
    public const ushort VK_RMENU = 0xA5;

    public const ushort VK_F1 = 0x70;
    public const ushort VK_F2 = 0x71;
    public const ushort VK_F3 = 0x72;
    public const ushort VK_F4 = 0x73;
    public const ushort VK_F5 = 0x74;
    public const ushort VK_F9 = 0x78;

    public const uint INPUT_KEYBOARD = 1;
    public const uint KEYEVENTF_EXTENDEDKEY = 0x0001;
    public const uint KEYEVENTF_KEYUP = 0x0002;
    public const uint KEYEVENTF_UNICODE = 0x0004;

    public const uint PROCESS_QUERY_LIMITED_INFORMATION = 0x1000;

    public delegate IntPtr LowLevelKeyboardProc(int nCode, IntPtr wParam, IntPtr lParam);

    [StructLayout(LayoutKind.Sequential)]
    public struct KBDLLHOOKSTRUCT
    {
        public uint vkCode;
        public uint scanCode;
        public uint flags;
        public uint time;
        public UIntPtr dwExtraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct INPUT
    {
        public uint type;
        public KEYBDINPUT ki;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct KEYBDINPUT
    {
        public ushort wVk;
        public ushort wScan;
        public uint dwFlags;
        public uint time;
        public UIntPtr dwExtraInfo;
        public ulong padding; // Ensure alignment to largest member of union
    }

    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr SetWindowsHookExW(int idHook, LowLevelKeyboardProc lpfn, IntPtr hMod, uint dwThreadId);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool UnhookWindowsHookEx(IntPtr hhk);

    [DllImport("user32.dll")]
    public static extern IntPtr CallNextHookEx(IntPtr hhk, int nCode, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern uint SendInput(uint nInputs, [In] INPUT[] pInputs, int cbSize);

    [DllImport("user32.dll")]
    public static extern short GetAsyncKeyState(int vKey);

    [DllImport("user32.dll")]
    public static extern short GetKeyState(int nVirtKey);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr OpenProcess(uint processAccess, bool bInheritHandle, uint processId);

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool QueryFullProcessImageNameW(IntPtr hProcess, uint dwFlags, [Out] StringBuilder lpExeName, ref uint lpdwSize);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool CloseHandle(IntPtr hObject);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr GetModuleHandleW(string? lpModuleName);

    [DllImport("kernel32.dll")]
    public static extern IntPtr GetCurrentProcess();

    [DllImport("psapi.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool EmptyWorkingSet(IntPtr hProcess);

    /// <summary>
    /// Aggressively collects garbage and trims process working set memory down to minimal footprint (~2MB - 5MB).
    /// </summary>
    public static void TrimProcessMemory()
    {
        try
        {
            GC.Collect(2, GCCollectionMode.Aggressive, true, true);
            GC.WaitForPendingFinalizers();
            GC.Collect(2, GCCollectionMode.Aggressive, true, true);
            EmptyWorkingSet(GetCurrentProcess());
        }
        catch
        {
        }
    }

    [DllImport("user32.dll")]
    public static extern uint MapVirtualKeyW(uint uCode, uint uMapType);

    public static INPUT CreateVkInput(ushort vk, bool keyUp)
    {
        return new INPUT
        {
            type = INPUT_KEYBOARD,
            ki = new KEYBDINPUT
            {
                wVk = vk,
                wScan = (ushort)MapVirtualKeyW(vk, 0),
                dwFlags = keyUp ? KEYEVENTF_KEYUP : 0,
                time = 0,
                dwExtraInfo = UIntPtr.Zero
            }
        };
    }

    public static INPUT CreateUnicodeInput(char c, bool keyUp)
    {
        return new INPUT
        {
            type = INPUT_KEYBOARD,
            ki = new KEYBDINPUT
            {
                wVk = 0,
                wScan = c,
                dwFlags = KEYEVENTF_UNICODE | (keyUp ? KEYEVENTF_KEYUP : 0),
                time = 0,
                dwExtraInfo = UIntPtr.Zero
            }
        };
    }

    /// <summary>
    /// Sends backspaces and replacement Unicode characters reliably to the active window.
    /// For multi-character restorations (e.g. English auto-restore or macro expansions),
    /// backspaces are sent first with pacing to prevent the target application from dropping deletions.
    /// For single-character replacements, an atomic array is dispatched to prevent browser address-bar autocomplete duplication.
    /// </summary>
    public static void SendAtomicBatch(int backspaces, string text)
    {
        if (backspaces <= 0 && string.IsNullOrEmpty(text))
        {
            return;
        }

        if (backspaces > 1)
        {
            var backInputs = new INPUT[backspaces * 2];
            int bIdx = 0;
            for (int i = 0; i < backspaces; i++)
            {
                backInputs[bIdx++] = CreateVkInput(VK_BACK, false);
                backInputs[bIdx++] = CreateVkInput(VK_BACK, true);
            }
            SendInput((uint)backInputs.Length, backInputs, Marshal.SizeOf<INPUT>());
            Thread.Sleep(3);

            if (!string.IsNullOrEmpty(text))
            {
                var charInputs = new INPUT[text.Length * 2];
                int cIdx = 0;
                for (int i = 0; i < text.Length; i++)
                {
                    charInputs[cIdx++] = CreateUnicodeInput(text[i], false);
                    charInputs[cIdx++] = CreateUnicodeInput(text[i], true);
                }
                SendInput((uint)charInputs.Length, charInputs, Marshal.SizeOf<INPUT>());
            }
        }
        else
        {
            int totalInputs = (backspaces * 2) + ((text?.Length ?? 0) * 2);
            var inputs = new INPUT[totalInputs];
            int idx = 0;

            if (backspaces == 1)
            {
                inputs[idx++] = CreateVkInput(VK_BACK, false);
                inputs[idx++] = CreateVkInput(VK_BACK, true);
            }

            if (!string.IsNullOrEmpty(text))
            {
                for (int i = 0; i < text.Length; i++)
                {
                    inputs[idx++] = CreateUnicodeInput(text[i], false);
                    inputs[idx++] = CreateUnicodeInput(text[i], true);
                }
            }

            SendInput((uint)inputs.Length, inputs, Marshal.SizeOf<INPUT>());
        }
    }

    /// <summary>
    /// Gets the process image name of the current foreground window.
    /// </summary>
    public static string GetForegroundProcessName()
    {
        IntPtr hwnd = GetForegroundWindow();
        if (hwnd == IntPtr.Zero) return string.Empty;

        GetWindowThreadProcessId(hwnd, out uint pid);
        if (pid == 0) return string.Empty;

        IntPtr hProc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        if (hProc == IntPtr.Zero)
        {
            try
            {
                using var proc = Process.GetProcessById((int)pid);
                return proc.ProcessName + ".exe";
            }
            catch
            {
                return string.Empty;
            }
        }

        try
        {
            var sb = new StringBuilder(1024);
            uint size = (uint)sb.Capacity;
            if (QueryFullProcessImageNameW(hProc, 0, sb, ref size))
            {
                return Path.GetFileName(sb.ToString());
            }
        }
        finally
        {
            CloseHandle(hProc);
        }

        return string.Empty;
    }
}

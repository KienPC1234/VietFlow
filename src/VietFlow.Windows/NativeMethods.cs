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

    [DllImport("user32.dll", EntryPoint = "SendInput", SetLastError = true)]
    private static extern unsafe uint SendInputPtr(uint nInputs, INPUT* pInputs, int cbSize);

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

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool MessageBeep(uint uType);

    [DllImport("user32.dll")]
    public static extern uint GetDoubleClickTime();

    public const uint MB_ICONASTERISK = 0x00000040;

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
    /// Sends backspaces and replacement Unicode characters to the active window.
    /// Small normal edits are one atomic SendInput batch so the caret does not
    /// visibly jump left and right. Long restores/macros use paced deletion first
    /// so legacy applications do not drop backspaces.
    /// </summary>
    public static void SendAtomicBatch(int backspaces, string text)
    {
        SendAtomicBatch(backspaces, text.AsSpan());
    }

    public static unsafe void SendAtomicBatch(int backspaces, ReadOnlySpan<char> text)
    {
        if (backspaces <= 0 && text.IsEmpty)
        {
            return;
        }

        // Normal Vietnamese edits are small (usually 1-3 backspaces). Send them
        // in ONE SendInput call so the caret never visibly jumps left, waits,
        // then jumps right. Only long restores/macros use the paced fallback.
        const int atomicBackspaceLimit = 8;
        const int atomicCharLimit = 32;

        if (backspaces <= atomicBackspaceLimit && text.Length <= atomicCharLimit)
        {
            int totalInputs = (backspaces * 2) + (text.Length * 2);

            // Zero allocation on the normal typing path. The largest normal
            // batch is ~80 INPUT structs (a few KB), safe for the hook stack.
            Span<INPUT> inputs = totalInputs <= 128
                ? stackalloc INPUT[totalInputs]
                : new INPUT[totalInputs];

            int index = 0;
            for (int i = 0; i < backspaces; i++)
            {
                inputs[index++] = CreateVkInput(VK_BACK, false);
                inputs[index++] = CreateVkInput(VK_BACK, true);
            }

            for (int i = 0; i < text.Length; i++)
            {
                inputs[index++] = CreateUnicodeInput(text[i], false);
                inputs[index++] = CreateUnicodeInput(text[i], true);
            }

            fixed (INPUT* pointer = &inputs[0])
            {
                SendInputPtr((uint)inputs.Length, pointer, sizeof(INPUT));
            }

            return;
        }

        // Large restores (English word recovery / macro expansion): pace the
        // deletion separately so legacy apps do not drop the backspaces.
        var backInputs = new INPUT[backspaces * 2];
        int backIndex = 0;
        for (int i = 0; i < backspaces; i++)
        {
            backInputs[backIndex++] = CreateVkInput(VK_BACK, false);
            backInputs[backIndex++] = CreateVkInput(VK_BACK, true);
        }

        if (backInputs.Length > 0)
        {
            SendInput((uint)backInputs.Length, backInputs, Marshal.SizeOf<INPUT>());
            Thread.Sleep(3);
        }

        if (!text.IsEmpty)
        {
            var charInputs = new INPUT[text.Length * 2];
            int charIndex = 0;
            for (int i = 0; i < text.Length; i++)
            {
                charInputs[charIndex++] = CreateUnicodeInput(text[i], false);
                charInputs[charIndex++] = CreateUnicodeInput(text[i], true);
            }

            SendInput((uint)charInputs.Length, charInputs, Marshal.SizeOf<INPUT>());
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

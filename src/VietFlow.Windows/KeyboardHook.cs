// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Runtime.InteropServices;
using VietFlow.Configuration;
using VietFlow.Engine;

namespace VietFlow.Windows;

internal sealed class KeyboardHook : IDisposable
{
    private readonly VietFlowSession _session;
    private readonly VietFlowConfig _config;
    private readonly ForegroundTracker _tracker;
    private readonly Action _onToggleLanguage;
    private readonly Action _onOpenSettings;
    private readonly Action _onQuickConvert;

    private IntPtr _hookId = IntPtr.Zero;
    private readonly NativeMethods.LowLevelKeyboardProc _hookProc;

    private bool _ctrlPressed;
    private bool _shiftPressed;
    private bool _altPressed;

    public KeyboardHook(
        VietFlowSession session,
        VietFlowConfig config,
        ForegroundTracker tracker,
        Action onToggleLanguage,
        Action onOpenSettings,
        Action onQuickConvert)
    {
        _session = session;
        _config = config;
        _tracker = tracker;
        _onToggleLanguage = onToggleLanguage;
        _onOpenSettings = onOpenSettings;
        _onQuickConvert = onQuickConvert;

        _hookProc = HookCallback;
        Install();
    }

    private void Install()
    {
        using var curProcess = System.Diagnostics.Process.GetCurrentProcess();
        using var curModule = curProcess.MainModule;
        IntPtr hModule = NativeMethods.GetModuleHandleW(curModule?.ModuleName);
        _hookId = NativeMethods.SetWindowsHookExW(NativeMethods.WH_KEYBOARD_LL, _hookProc, hModule, 0);
    }

    private unsafe IntPtr HookCallback(int nCode, IntPtr wParam, IntPtr lParam)
    {
        if (nCode >= 0)
        {
            ref readonly var kbd = ref *(NativeMethods.KBDLLHOOKSTRUCT*)lParam;

            // Skip all injected keys (prevents recursive loops when SendInput generates backspaces or characters)
            if ((kbd.flags & NativeMethods.LLKHF_INJECTED) != 0)
            {
                return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
            }

            int msg = wParam.ToInt32();
            bool isKeyDown = msg is NativeMethods.WM_KEYDOWN or NativeMethods.WM_SYSKEYDOWN;
            bool isKeyUp = msg is NativeMethods.WM_KEYUP or NativeMethods.WM_SYSKEYUP;

            ushort vk = (ushort)kbd.vkCode;

            // Track modifier keys
            if (vk is NativeMethods.VK_LCONTROL or NativeMethods.VK_RCONTROL or NativeMethods.VK_CONTROL)
            {
                _ctrlPressed = isKeyDown;
            }
            else if (vk is NativeMethods.VK_LSHIFT or NativeMethods.VK_RSHIFT or NativeMethods.VK_SHIFT)
            {
                _shiftPressed = isKeyDown;
            }
            else if (vk is NativeMethods.VK_LMENU or NativeMethods.VK_RMENU or NativeMethods.VK_MENU)
            {
                _altPressed = isKeyDown;
            }

            // Global Hotkey: Ctrl + Shift + F5 (Settings)
            if (isKeyDown && vk == NativeMethods.VK_F5 && _ctrlPressed && _shiftPressed)
            {
                _onOpenSettings();
                return (IntPtr)1;
            }

            // Global Hotkey: Ctrl + Shift + F9 (Quick Clipboard Convert)
            if (isKeyDown && vk == NativeMethods.VK_F9 && _ctrlPressed && _shiftPressed)
            {
                _onQuickConvert();
                return (IntPtr)1;
            }

            // Toggle Hotkeys
            if (isKeyDown)
            {
                bool isToggle = false;
                bool suppressKey = false;

                switch (_config.ToggleHotkey)
                {
                    case "CtrlShift":
                        if (_ctrlPressed && (vk is NativeMethods.VK_LSHIFT or NativeMethods.VK_RSHIFT))
                        {
                            isToggle = true;
                        }
                        break;
                    case "AltZ":
                        if (_altPressed && vk == 'Z')
                        {
                            isToggle = true;
                            suppressKey = true;
                        }
                        break;
                    case "CtrlSpace":
                        if (_ctrlPressed && vk == NativeMethods.VK_SPACE)
                        {
                            isToggle = true;
                            suppressKey = true;
                        }
                        break;
                    case "ShiftSpace":
                        if (_shiftPressed && vk == NativeMethods.VK_SPACE)
                        {
                            isToggle = true;
                            suppressKey = true;
                        }
                        break;
                    case "AltShift":
                        if (_altPressed && (vk is NativeMethods.VK_LSHIFT or NativeMethods.VK_RSHIFT))
                        {
                            isToggle = true;
                        }
                        break;
                    case "WinSpace":
                        if (((NativeMethods.GetAsyncKeyState(0x5B) & 0x8000) != 0 || (NativeMethods.GetAsyncKeyState(0x5C) & 0x8000) != 0) && vk == NativeMethods.VK_SPACE)
                        {
                            isToggle = true;
                            suppressKey = true;
                        }
                        break;
                    case "CapsLock":
                        if (vk == NativeMethods.VK_CAPITAL)
                        {
                            isToggle = true;
                            suppressKey = true;
                        }
                        break;
                    case "RightCtrl":
                        if (vk == NativeMethods.VK_RCONTROL)
                        {
                            isToggle = true;
                            suppressKey = true;
                        }
                        break;
                    default:
                        if (_config.CustomToggleVk > 0 && vk == _config.CustomToggleVk)
                        {
                            bool shiftReq = (_config.CustomToggleModifiers & 1) != 0;
                            bool ctrlReq = (_config.CustomToggleModifiers & 2) != 0;
                            bool altReq = (_config.CustomToggleModifiers & 4) != 0;

                            if ((!shiftReq || _shiftPressed) && (!ctrlReq || _ctrlPressed) && (!altReq || _altPressed))
                            {
                                isToggle = true;
                                suppressKey = true;
                            }
                        }
                        break;
                }

                if (isToggle)
                {
                    _onToggleLanguage();
                    if (suppressKey)
                    {
                        return (IntPtr)1;
                    }
                    return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
                }
            }

            // Navigation and mouse interaction resets the word buffer
            if (isKeyDown && (vk is NativeMethods.VK_ESCAPE or NativeMethods.VK_RETURN or NativeMethods.VK_TAB or NativeMethods.VK_BACK or 0x25 or 0x26 or 0x27 or 0x28))
            {
                if (vk == NativeMethods.VK_BACK)
                {
                    var res = _session.ProcessKey('\b');
                    if (res.ShouldSuppressOriginalKey)
                    {
                        NativeMethods.SendAtomicBatch(res.Backspaces, res.Output);
                        return (IntPtr)1;
                    }
                }
                else
                {
                    _session.Engine.Reset();
                }
                return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
            }

            // Game Mode / Excluded Applications check: bypass hook for excluded apps
            if (_tracker.IsForegroundAppExcluded())
            {
                return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
            }

            // If Vietnamese engine is turned off, check if macros are allowed when disabled
            if (!_config.IsEnabled && !_config.AllowMacroWhenDisabled)
            {
                return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
            }

            // Process text characters on KeyDown
            if (isKeyDown)
            {
                // If Ctrl or Alt or Windows key is down, do not process text (e.g. Ctrl+C, Ctrl+V, Alt+Tab, Win+R)
                if (_ctrlPressed || _altPressed || (NativeMethods.GetAsyncKeyState(0x5B) & 0x8000) != 0 || (NativeMethods.GetAsyncKeyState(0x5C) & 0x8000) != 0)
                {
                    _session.Engine.Reset();
                    return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
                }

                char inputChar = VkToChar(vk, _shiftPressed);
                if (inputChar != '\0')
                {
                    var result = _session.ProcessKey(inputChar);
                    if (result.ShouldSuppressOriginalKey)
                    {
                        // Only suppress and SendInput if backspaces > 0 or character was transformed.
                        // For pure unmodified keystrokes, allow native OS message dispatch to pass through directly.
                        // This prevents duplicate character race conditions (e.g. "c" duplicating to "cc").
                        if (result.Backspaces > 0 || result.Output.Length != 1 || result.Output[0] != inputChar)
                        {
                            NativeMethods.SendAtomicBatch(result.Backspaces, result.Output);
                            return (IntPtr)1;
                        }
                        else
                        {
                            return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
                        }
                    }
                }
            }
        }

        return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
    }

    private static char VkToChar(ushort vk, bool shift)
    {
        bool caps = (NativeMethods.GetKeyState(NativeMethods.VK_CAPITAL) & 1) != 0;
        bool isUpper = shift ^ caps;

        if (vk >= 'A' && vk <= 'Z')
        {
            return isUpper ? (char)vk : char.ToLowerInvariant((char)vk);
        }

        if (vk >= '0' && vk <= '9')
        {
            if (!shift) return (char)vk;
            return vk switch
            {
                '1' => '!',
                '2' => '@',
                '3' => '#',
                '4' => '$',
                '5' => '%',
                '6' => '^',
                '7' => '&',
                '8' => '*',
                '9' => '(',
                '0' => ')',
                _ => '\0'
            };
        }

        if (vk == NativeMethods.VK_SPACE) return ' ';

        return vk switch
        {
            0xBA => shift ? ':' : ';',
            0xBB => shift ? '+' : '=',
            0xBC => shift ? '<' : ',',
            0xBD => shift ? '_' : '-',
            0xBE => shift ? '>' : '.',
            0xBF => shift ? '?' : '/',
            0xC0 => shift ? '~' : '`',
            0xDB => shift ? '{' : '[',
            0xDC => shift ? '|' : '\\',
            0xDD => shift ? '}' : ']',
            0xDE => shift ? '"' : '\'',
            _ => '\0'
        };
    }

    public void Dispose()
    {
        if (_hookId != IntPtr.Zero)
        {
            NativeMethods.UnhookWindowsHookEx(_hookId);
            _hookId = IntPtr.Zero;
        }
    }
}


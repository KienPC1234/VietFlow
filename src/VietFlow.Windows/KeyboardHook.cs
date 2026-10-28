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
    private readonly char[] _outputBuffer = new char[512];
    private readonly char[] _displayShadow = new char[512];
    private int _displayShadowLength;

    private bool _ctrlPressed;
    private bool _shiftPressed;
    private bool _altPressed;
    private IntPtr _lastForegroundWindow = IntPtr.Zero;
    private long _lastTextKeyTick;

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

            // A new foreground window means a new word; do not carry the previous
            // app's buffer or display shadow into it.
            IntPtr foreground = NativeMethods.GetForegroundWindow();
            if (foreground != _lastForegroundWindow)
            {
                _lastForegroundWindow = foreground;
                _lastTextKeyTick = 0;
                _session.Engine.Reset();
                ResetDisplayShadow();
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
                    _lastTextKeyTick = 0;
                    _session.Engine.Reset();
                    ResetDisplayShadow();
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
                _lastTextKeyTick = 0;
                if (vk == NativeMethods.VK_BACK)
                {
                    if (_session.IsEnabled)
                    {
                        int written = _session.ProcessKey('\b', _outputBuffer, out int backspaces);
                        if (backspaces > 0 || written > 0)
                        {
                            SendEngineOutput(backspaces, _outputBuffer.AsSpan(0, written));
                            return (IntPtr)1;
                        }

                        RemoveLastDisplayChar();
                    }
                }
                else
                {
                    _session.Engine.Reset();
                    ResetDisplayShadow();
                }
                return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
            }

            // Game Mode / Excluded Applications check: bypass hook for excluded apps
            if (_tracker.IsForegroundAppExcluded())
            {
                _lastTextKeyTick = 0;
                ResetDisplayShadow();
                return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
            }

            // If Vietnamese engine is turned off, check if macros are allowed when disabled
            if (!_config.IsEnabled && !_config.AllowMacroWhenDisabled)
            {
                _lastTextKeyTick = 0;
                ResetDisplayShadow();
                return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
            }

            // Process text characters on KeyDown
            if (isKeyDown)
            {
                // If Ctrl or Alt or Windows key is down, do not process text (e.g. Ctrl+C, Ctrl+V, Alt+Tab, Win+R)
                if (_ctrlPressed || _altPressed || (NativeMethods.GetAsyncKeyState(0x5B) & 0x8000) != 0 || (NativeMethods.GetAsyncKeyState(0x5C) & 0x8000) != 0)
                {
                    _session.Engine.Reset();
                    ResetDisplayShadow();
                    return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
                }

                char inputChar = VkToChar(vk, _shiftPressed);
                if (inputChar != '\0' && _session.IsEnabled)
                {
                    long now = Environment.TickCount64;
                    int timeout = Math.Clamp(_config.TypingTimeoutMs, 300, 10000);
                    if (_lastTextKeyTick != 0 && now - _lastTextKeyTick > timeout)
                    {
                        _session.Engine.Reset();
                        ResetDisplayShadow();
                    }
                    _lastTextKeyTick = now;

                    int written = _session.ProcessKey(inputChar, _outputBuffer, out int backspaces);

                    if (backspaces > 0 || written != 1 || _outputBuffer[0] != inputChar)
                    {
                        SendEngineOutput(backspaces, _outputBuffer.AsSpan(0, written));
                        return (IntPtr)1;
                    }

                    TrackDisplayChar(inputChar);
                    return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
                }
            }
        }

        return NativeMethods.CallNextHookEx(_hookId, nCode, wParam, lParam);
    }

    /// <summary>
    /// The engine often rebuilds a short suffix (for example "vie" -> "viê")
    /// and reports several backspaces. Diffing against what is actually on
    /// screen avoids deleting/typing the unchanged prefix, which is what makes
    /// the caret visibly jump.
    /// </summary>
    private void SendEngineOutput(int backspaces, ReadOnlySpan<char> text)
    {
        if (backspaces < 0)
        {
            backspaces = 0;
        }

        if (backspaces == 0 && text.IsEmpty)
        {
            return;
        }

        bool canDiff = backspaces > 0 &&
                       backspaces <= _displayShadowLength &&
                       _displayShadowLength <= 128 &&
                       text.Length <= 256 &&
                       !ContainsBreak(text);

        if (!canDiff)
        {
            NativeMethods.SendAtomicBatch(backspaces, text);
            UpdateShadowFallback(backspaces, text);
            return;
        }

        int prefixLength = _displayShadowLength - backspaces;
        int newLength = prefixLength + text.Length;
        Span<char> next = stackalloc char[newLength];
        _displayShadow.AsSpan(0, prefixLength).CopyTo(next);
        text.CopyTo(next[prefixLength..]);

        int common = 0;
        while (common < _displayShadowLength &&
               common < newLength &&
               _displayShadow[common] == next[common])
        {
            common++;
        }

        NativeMethods.SendAtomicBatch(_displayShadowLength - common, next[common..]);

        next.CopyTo(_displayShadow);
        _displayShadowLength = newLength;
    }

    private void UpdateShadowFallback(int backspaces, ReadOnlySpan<char> text)
    {
        if (ContainsBreak(text))
        {
            ResetDisplayShadow();
            return;
        }

        if (backspaces > _displayShadowLength)
        {
            ResetDisplayShadow();
        }
        else
        {
            _displayShadowLength -= backspaces;
        }

        if (_displayShadowLength + text.Length > _displayShadow.Length)
        {
            ResetDisplayShadow();
            return;
        }

        text.CopyTo(_displayShadow.AsSpan(_displayShadowLength));
        _displayShadowLength += text.Length;
    }

    private void TrackDisplayChar(char c)
    {
        if (!char.IsLetter(c))
        {
            ResetDisplayShadow();
            return;
        }

        if (_displayShadowLength < _displayShadow.Length)
        {
            _displayShadow[_displayShadowLength++] = c;
        }
        else
        {
            ResetDisplayShadow();
        }
    }

    private void RemoveLastDisplayChar()
    {
        if (_displayShadowLength > 0)
        {
            _displayShadowLength--;
        }
    }

    private void ResetDisplayShadow()
    {
        _displayShadowLength = 0;
    }

    private static bool ContainsBreak(ReadOnlySpan<char> text)
    {
        foreach (char c in text)
        {
            if (char.IsWhiteSpace(c) || char.IsPunctuation(c) || char.IsSymbol(c) || char.IsDigit(c))
            {
                return true;
            }
        }

        return false;
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


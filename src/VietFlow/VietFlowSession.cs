// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Runtime.CompilerServices;
using System.Text;
using VietFlow.Charsets;
using VietFlow.Engine;

namespace VietFlow;

public sealed class VietFlowSession
{
    private readonly VietFlowEngine _engine;
    private bool _isEnabled = true;

    public VietFlowEngine Engine => _engine;

    public bool IsEnabled
    {
        get => _isEnabled;
        set
        {
            if (_isEnabled != value)
            {
                _isEnabled = value;
                if (!_isEnabled)
                {
                    _engine.Reset();
                }
                EnabledChanged?.Invoke(this, _isEnabled);
            }
        }
    }

    public EngineOptions Options
    {
        get => _engine.Options;
        set
        {
            var oldMethod = _engine.Options.Method;
            _engine.Options = value;
            if (oldMethod != value.Method)
            {
                InputMethodChanged?.Invoke(this, value.Method);
            }
        }
    }

    public InputMethod Method
    {
        get => _engine.Options.Method;
        set
        {
            if (_engine.Options.Method != value)
            {
                _engine.Options.Method = value;
                _engine.Reset();
                InputMethodChanged?.Invoke(this, value);
            }
        }
    }

    public IVnCharset Charset
    {
        get => _engine.Charset;
        set => _engine.Charset = value;
    }

    public MacroTable Macros => _engine.Macros;

    public event EventHandler<bool>? EnabledChanged;
    public event EventHandler<InputMethod>? InputMethodChanged;

    public VietFlowSession(
        InputMethod method = InputMethod.Telex,
        CharsetId charsetId = CharsetId.Unicode,
        bool modernStyle = false)
    {
        var options = new EngineOptions
        {
            Method = method,
            ModernStyle = modernStyle
        };
        var charset = CharsetFactory.GetCharset(charsetId);
        _engine = new VietFlowEngine(options, charset);
    }

    public VietFlowSession(VietFlowEngine engine)
    {
        _engine = engine ?? throw new ArgumentNullException(nameof(engine));
    }

    public void ToggleEnabled() => IsEnabled = !IsEnabled;

    /// <summary>
    /// Core processing method for character inputs from WPF (PreviewTextInput), WinForms, or Windows Hooks.
    /// Automatically manages modifiers, hotkeys (Ctrl/Alt/Win), and case sensitivity.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public KeyProcessResult ProcessKey(char c, ModifierKeys modifiers = ModifierKeys.None)
    {
        if (!_isEnabled)
        {
            return KeyProcessResult.Unhandled;
        }

        // Bypasses shortcuts like Ctrl+C, Ctrl+V, Alt+F4, Win+E without corrupting typing state
        if ((modifiers & (ModifierKeys.Control | ModifierKeys.Alt | ModifierKeys.Windows)) != 0)
        {
            _engine.Reset();
            return KeyProcessResult.Unhandled;
        }

        // Apply Shift / CapsLock casing if explicitly specified
        char finalChar = c;
        if (char.IsLetter(c) && (modifiers & (ModifierKeys.Shift | ModifierKeys.CapsLock)) != 0)
        {
            bool shift = (modifiers & ModifierKeys.Shift) != 0;
            bool caps = (modifiers & ModifierKeys.CapsLock) != 0;
            bool shouldUpper = shift ^ caps;
            finalChar = shouldUpper ? char.ToUpperInvariant(c) : char.ToLowerInvariant(c);
        }

        return _engine.ProcessKey(finalChar);
    }

    /// <summary>
    /// Processes low-level virtual key codes from WPF KeyDown, WM_KEYDOWN, or low-level keyboard hook.
    /// </summary>
    public KeyProcessResult ProcessVirtualKey(
        int vkCode,
        char charCode = '\0',
        ModifierKeys modifiers = ModifierKeys.None)
    {
        if (!_isEnabled)
        {
            return KeyProcessResult.Unhandled;
        }

        // Navigation keys reset typing session
        if (IsNavigationKey(vkCode))
        {
            _engine.Reset();
            return KeyProcessResult.Unhandled;
        }

        // Backspace
        if (vkCode == VirtualKeys.Back)
        {
            return ProcessKey('\b', modifiers);
        }

        if (charCode != '\0')
        {
            return ProcessKey(charCode, modifiers);
        }

        return KeyProcessResult.Unhandled;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static bool IsNavigationKey(int vkCode) => vkCode switch
    {
        VirtualKeys.Left or VirtualKeys.Right or VirtualKeys.Up or VirtualKeys.Down => true,
        VirtualKeys.Home or VirtualKeys.End or VirtualKeys.Prior or VirtualKeys.Next => true,
        VirtualKeys.Escape or VirtualKeys.Return or VirtualKeys.Tab or VirtualKeys.Delete => true,
        _ => false
    };

    public void NotifyFocusLost() => _engine.Reset();

    public void NotifyMouseClick() => _engine.Reset();

    public void NotifyCaretMoved() => _engine.Reset();

    public void NotifyNavigationKey(int vkCode)
    {
        if (IsNavigationKey(vkCode))
        {
            _engine.Reset();
        }
    }

    public string ProcessString(ReadOnlySpan<char> input)
    {
        _engine.Reset();
        Span<char> output = stackalloc char[input.Length * 4 + 256];
        int writePos = 0;

        Span<char> keyOut = stackalloc char[512];

        for (int i = 0; i < input.Length; i++)
        {
            char c = input[i];
            int written = _engine.ProcessKey(c, keyOut, out int backspaces);

            if (backspaces > 0)
            {
                writePos = Math.Max(0, writePos - backspaces);
            }

            if (written > 0)
            {
                keyOut[..written].CopyTo(output[writePos..]);
                writePos += written;
            }
        }

        return new string(output[..writePos]);
    }

    public int ProcessUtf8(ReadOnlySpan<byte> utf8Input, Span<byte> utf8Output)
    {
        _engine.Reset();

        int charCount = Encoding.UTF8.GetCharCount(utf8Input);
        Span<char> charInput = charCount <= 256 ? stackalloc char[charCount] : new char[charCount];
        Encoding.UTF8.GetChars(utf8Input, charInput);

        int maxOutChars = charCount * 4 + 256;
        Span<char> charOutput = maxOutChars <= 1024 ? stackalloc char[maxOutChars] : new char[maxOutChars];
        int writePos = 0;
        Span<char> keyOut = stackalloc char[512];

        for (int i = 0; i < charInput.Length; i++)
        {
            char c = charInput[i];
            int written = _engine.ProcessKey(c, keyOut, out int backspaces);

            if (backspaces > 0)
            {
                writePos = Math.Max(0, writePos - backspaces);
            }

            if (written > 0)
            {
                keyOut[..written].CopyTo(charOutput[writePos..]);
                writePos += written;
            }
        }

        return Encoding.UTF8.GetBytes(charOutput[..writePos], utf8Output);
    }

    public void Reset() => _engine.Reset();
}

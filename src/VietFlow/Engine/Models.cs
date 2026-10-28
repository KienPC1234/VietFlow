// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

namespace VietFlow.Engine;

/// <summary>
/// Supported Vietnamese input methods.
/// </summary>
public enum InputMethod : byte
{
    Telex = 0,
    SimpleTelex = 1,
    Vni = 2,
    Viqr = 3,
    UserCustom = 4
}

/// <summary>
/// Character set identifiers supported for output encoding.
/// </summary>
public enum CharsetId : byte
{
    Unicode = 0,               // Precomposed Unicode (NFC)
    UnicodeComposite = 1,      // Decomposed Unicode (NFD)
    Tcvn3 = 2,                 // TCVN3 (ABC)
    VniWindows = 3,            // VNI Windows
    VniWin = 3,                // Alias for VNI Windows
    Viqr = 4,                  // VIQR pure ASCII
    Custom = 5                 // User-defined charset (advanced editor)
}

/// <summary>
/// Output type from engine.
/// </summary>
public enum OutputType : byte
{
    Character,
    KeyStroke,
    Direct
}

/// <summary>
/// Character lexical category.
/// </summary>
public enum CharType : byte
{
    Letter,
    Digit,
    ToneOrAction,
    WordBreak,
    Punctuation,
    Reset
}

/// <summary>
/// Syllable word form status.
/// </summary>
public enum WordForm : byte
{
    NonVn = 0,
    Empty = 1,
    C = 2,
    V = 3,
    CV = 4,
    VC = 5,
    CVC = 6
}

/// <summary>
/// Keyboard modifier flags for GUI and hook integration.
/// </summary>
[Flags]
public enum ModifierKeys : byte
{
    None = 0,
    Shift = 1 << 0,
    Control = 1 << 1,
    Alt = 1 << 2,
    Windows = 1 << 3,
    CapsLock = 1 << 4
}

/// <summary>
/// Virtual key codes for UI framework navigation and dispatching.
/// </summary>
public static class VirtualKeys
{
    public const int Back = 0x08;
    public const int Tab = 0x09;
    public const int Return = 0x0D;
    public const int Escape = 0x1B;
    public const int Space = 0x20;
    public const int Prior = 0x21; // Page Up
    public const int Next = 0x22;  // Page Down
    public const int End = 0x23;
    public const int Home = 0x24;
    public const int Left = 0x25;
    public const int Up = 0x26;
    public const int Right = 0x27;
    public const int Down = 0x28;
    public const int Delete = 0x2E;
}

/// <summary>
/// Engine configuration options.
/// </summary>
public sealed class EngineOptions
{
    /// <summary>
    /// Input method used for Vietnamese typing (default: Telex).
    /// </summary>
    public InputMethod Method { get; set; } = InputMethod.Telex;

    /// <summary>
    /// Allow placing tone marks anywhere in the word (default: true).
    /// </summary>
    public bool FreeMarking { get; set; } = true;

    /// <summary>
    /// Use modern tone placement style: oà, oè, uỳ instead of òa, òe, ùy (default: false).
    /// </summary>
    public bool ModernStyle { get; set; } = false;

    /// <summary>
    /// Enable macro replacement (default: true).
    /// </summary>
    public bool EnableMacro { get; set; } = true;

    public bool MacroEnabled
    {
        get => EnableMacro;
        set => EnableMacro = value;
    }

    /// <summary>
    /// Enable strict Vietnamese spelling checks (default: true).
    /// </summary>
    public bool SpellCheck { get; set; } = true;

    public bool SpellCheckEnabled
    {
        get => SpellCheck;
        set => SpellCheck = value;
    }

    /// <summary>
    /// Auto restore original key strokes when typing non-Vietnamese words (default: true).
    /// </summary>
    public bool AutoNonVnRestore { get; set; } = true;

    /// <summary>
    /// Auto correct Shift key sticking on 2nd letter (e.g. "VIệt" -> "Việt", "TRường" -> "Trường").
    /// </summary>
    public bool FixShiftLag { get; set; } = true;

    /// <summary>
    /// Automatically capitalize the first letter of a sentence after Enter, '.', '!' or '?'.
    /// </summary>
    public bool AutoCapitalize { get; set; } = false;

    /// <summary>
    /// Initial letters treated as non-Vietnamese (tones/marks are suppressed). Default "fjwz".
    /// </summary>
    public string ForeignInitials { get; set; } = "fjwz";

    /// <summary>
    /// Enable consonant shorthand for the initial consonant: f -> ph, j -> gi, w -> qu.
    /// </summary>
    public bool ConsonantShortcuts { get; set; } = false;

    public EngineOptions Clone() => new()
    {
        Method = Method,
        FreeMarking = FreeMarking,
        ModernStyle = ModernStyle,
        EnableMacro = EnableMacro,
        SpellCheck = SpellCheck,
        AutoNonVnRestore = AutoNonVnRestore,
        FixShiftLag = FixShiftLag,
        AutoCapitalize = AutoCapitalize,
        ForeignInitials = ForeignInitials,
        ConsonantShortcuts = ConsonantShortcuts
    };
}

/// <summary>
/// Result of processing a key stroke, ready for GUI / WPF / low-level hook consumption.
/// </summary>
public readonly record struct KeyProcessResult(
    int Backspaces,
    string Output,
    bool Handled,
    bool Restored,
    OutputType OutputType)
{
    public KeyProcessResult(int backspaces, string output, OutputType outputType = OutputType.Character)
        : this(backspaces, output, true, false, outputType)
    {
    }

    /// <summary>
    /// Indicates whether the calling GUI/hook should suppress the original key event (e.Handled = true).
    /// </summary>
    public bool ShouldSuppressOriginalKey => Handled;

    /// <summary>
    /// Indicates whether one or more backspaces must be sent to the target UI element.
    /// </summary>
    public bool HasBackspaces => Backspaces > 0;

    /// <summary>
    /// Indicates whether output text should be sent to the target UI element.
    /// </summary>
    public bool HasOutput => !string.IsNullOrEmpty(Output);

    public static readonly KeyProcessResult Unhandled = new(0, string.Empty, false, false, OutputType.Character);

    public static KeyProcessResult OutputChars(int backspaces, string output, bool restored = false) =>
        new(backspaces, output, true, restored, OutputType.Character);

    public static KeyProcessResult OutputKeys(int backspaces, string output) =>
        new(backspaces, output, true, true, OutputType.KeyStroke);
}

using System.Buffers;
using System.Runtime.CompilerServices;

namespace VietFlow.Engine;

public enum ToneKeyType
{
    None = 0,
    Acute,   // Sắc
    Grave,   // Huyền
    Hook,    // Hỏi
    Tilde,   // Ngã
    Dot,     // Nặng
    Remove   // Xóa dấu
}

public enum ModifyKeyType
{
    None = 0,
    Circumflex, // Â, Ê, Ô
    Breve,      // Ă
    Horn,       // Ơ, Ư
    DMark,      // Đ
    HookAll     // W key: ă, ơ, ư
}

public static class InputProcessor
{
    private static readonly CharType[] BaseCharTypeMap = new CharType[256];
    private static readonly SearchValues<char> WordBreakSearchValues;
    private static readonly byte[][] ToneKeyTable = new byte[6][];
    private static readonly byte[][] ModifyKeyTable = new byte[6][];

    static InputProcessor()
    {
        for (int i = 0; i < 256; i++)
        {
            char c = (char)i;
            if (c is >= 'a' and <= 'z' or >= 'A' and <= 'Z')
            {
                BaseCharTypeMap[i] = CharType.Letter;
            }
            else if (c is >= '0' and <= '9')
            {
                BaseCharTypeMap[i] = CharType.Digit;
            }
            else if (c is ' ' or '\t')
            {
                BaseCharTypeMap[i] = CharType.WordBreak;
            }
            else if (c is '\r' or '\n')
            {
                BaseCharTypeMap[i] = CharType.Reset;
            }
            else if (char.IsPunctuation(c) || char.IsSymbol(c))
            {
                BaseCharTypeMap[i] = CharType.Punctuation;
            }
            else
            {
                BaseCharTypeMap[i] = CharType.Reset;
            }
        }

        WordBreakSearchValues = SearchValues.Create(" \t\r\n.,;:!?()[]{}\"'<>=+-*/\\|@#$%^&~`");

        // Precompute per-method action tables: one array lookup per keystroke.
        for (int m = 0; m <= (int)InputMethod.UserCustom; m++)
        {
            var method = (InputMethod)m;
            var tones = new byte[256];
            var mods = new byte[256];
            for (int i = 0; i < 256; i++)
            {
                tones[i] = (byte)GetToneKeyCore((char)i, method);
                mods[i] = (byte)GetModifyKeyCore((char)i, method);
            }
            ToneKeyTable[m] = tones;
            ModifyKeyTable[m] = mods;
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static CharType GetCharType(char c, InputMethod method)
    {
        if (c < 256)
        {
            CharType baseType = BaseCharTypeMap[c];
            if (method is InputMethod.Vni or InputMethod.MicrosoftVi)
            {
                if (c is >= '0' and <= '9')
                {
                    return CharType.ToneOrAction;
                }
            }
            else if (method is InputMethod.Viqr)
            {
                if (c is '\'' or '`' or '?' or '~' or '.' or '0' or '^' or '+' or '*' or '(')
                {
                    return CharType.ToneOrAction;
                }
            }
            else if (method is InputMethod.Telex or InputMethod.SimpleTelex)
            {
                char lower = ToLowerAscii(c);
                if (lower is 's' or 'f' or 'r' or 'x' or 'j' or 'z' or 'w')
                {
                    return CharType.ToneOrAction;
                }
            }
            return baseType;
        }

        if (char.IsLetter(c)) return CharType.Letter;
        if (char.IsDigit(c)) return CharType.Digit;
        if (char.IsWhiteSpace(c)) return CharType.WordBreak;
        return CharType.Punctuation;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsWordBreakFast(char c)
    {
        return WordBreakSearchValues.Contains(c);
    }

    /// <summary>
    /// ASCII-only lowercase (avoids the culture-aware Unicode casing path on the hot path).
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static char ToLowerAscii(char c)
    {
        return (uint)(c - 'A') <= 'Z' - 'A' ? (char)(c + ('a' - 'A')) : c;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsAsciiUpper(char c)
    {
        return (uint)(c - 'A') <= 'Z' - 'A';
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsLetterFast(char c)
    {
        return c <= 127 ? (uint)((c | 0x20) - 'a') <= 'z' - 'a' : char.IsLetter(c);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ToneKeyType GetToneKey(char c, InputMethod method)
    {
        uint m = (uint)(int)method;
        return c <= 255 && m < 6 ? (ToneKeyType)ToneKeyTable[m][c] : ToneKeyType.None;
    }

    private static ToneKeyType GetToneKeyCore(char c, InputMethod method)
    {
        char lower = ToLowerAscii(c);
        switch (method)
        {
            case InputMethod.Telex:
            case InputMethod.SimpleTelex:
                return lower switch
                {
                    's' => ToneKeyType.Acute,
                    'f' => ToneKeyType.Grave,
                    'r' => ToneKeyType.Hook,
                    'x' => ToneKeyType.Tilde,
                    'j' => ToneKeyType.Dot,
                    'z' => ToneKeyType.Remove,
                    _ => ToneKeyType.None
                };

            case InputMethod.Vni:
                return c switch
                {
                    '1' => ToneKeyType.Acute,
                    '2' => ToneKeyType.Grave,
                    '3' => ToneKeyType.Hook,
                    '4' => ToneKeyType.Tilde,
                    '5' => ToneKeyType.Dot,
                    '0' => ToneKeyType.Remove,
                    _ => ToneKeyType.None
                };

            // Microsoft Vietnamese keyboard layout (UniKey's MsViMethodMapping):
            // 5 = huyền, 6 = hỏi, 7 = ngã, 8 = sắc, 9 = nặng (and their shifted variants).
            case InputMethod.MicrosoftVi:
                return c switch
                {
                    '5' or '%' => ToneKeyType.Grave,
                    '6' or '^' => ToneKeyType.Hook,
                    '7' or '&' => ToneKeyType.Tilde,
                    '8' or '*' => ToneKeyType.Acute,
                    '9' or '(' => ToneKeyType.Dot,
                    _ => ToneKeyType.None
                };

            case InputMethod.Viqr:
                return c switch
                {
                    '\'' => ToneKeyType.Acute,
                    '`' => ToneKeyType.Grave,
                    '?' => ToneKeyType.Hook,
                    '~' => ToneKeyType.Tilde,
                    '.' => ToneKeyType.Dot,
                    '0' => ToneKeyType.Remove,
                    _ => ToneKeyType.None
                };

            default:
                return ToneKeyType.None;
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ModifyKeyType GetModifyKey(char c, InputMethod method)
    {
        uint m = (uint)(int)method;
        return c <= 255 && m < 6 ? (ModifyKeyType)ModifyKeyTable[m][c] : ModifyKeyType.None;
    }

    private static ModifyKeyType GetModifyKeyCore(char c, InputMethod method)
    {
        char lower = ToLowerAscii(c);
        switch (method)
        {
            case InputMethod.Telex:
            case InputMethod.SimpleTelex:
                return lower switch
                {
                    'w' => ModifyKeyType.HookAll,
                    _ => ModifyKeyType.None
                };

            case InputMethod.Vni:
                return c switch
                {
                    '6' => ModifyKeyType.Circumflex,
                    '7' => ModifyKeyType.Horn,
                    '8' => ModifyKeyType.Breve,
                    '9' => ModifyKeyType.DMark,
                    _ => ModifyKeyType.None
                };

            // Microsoft Vietnamese keyboard layout types ă/â/ê/ô/đ directly (map chars),
            // so there are no "modify previous letter" keys.
            case InputMethod.MicrosoftVi:
                return ModifyKeyType.None;

            case InputMethod.Viqr:
                return c switch
                {
                    '^' => ModifyKeyType.Circumflex,
                    '+' or '*' => ModifyKeyType.Horn,
                    '(' => ModifyKeyType.Breve,
                    _ => ModifyKeyType.None
                };

            default:
                return ModifyKeyType.None;
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static VnLexiTone ToVnLexiTone(ToneKeyType tone)
    {
        return tone switch
        {
            ToneKeyType.Acute => VnLexiTone.Acute,
            ToneKeyType.Grave => VnLexiTone.Grave,
            ToneKeyType.Hook => VnLexiTone.Hook,
            ToneKeyType.Tilde => VnLexiTone.Tilde,
            ToneKeyType.Dot => VnLexiTone.Dot,
            ToneKeyType.Remove => VnLexiTone.None,
            _ => VnLexiTone.None
        };
    }

    /// <summary>
    /// Character keys of the Microsoft Vietnamese keyboard layout:
    /// 1 = ă, 2 = â, 3 = ê, 4 = ô, 0 = đ (Shift gives the uppercase form).
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool TryGetMsViMappedChar(char c, out VnLexiName lexi, out bool isUpper)
    {
        switch (c)
        {
            case '1': lexi = VnLexiName.ah; isUpper = false; return true;
            case '!': lexi = VnLexiName.Ah; isUpper = true; return true;
            case '2': lexi = VnLexiName.ar; isUpper = false; return true;
            case '@': lexi = VnLexiName.Ar; isUpper = true; return true;
            case '3': lexi = VnLexiName.er; isUpper = false; return true;
            case '#': lexi = VnLexiName.Er; isUpper = true; return true;
            case '4': lexi = VnLexiName.or; isUpper = false; return true;
            case '$': lexi = VnLexiName.Or; isUpper = true; return true;
            case '0': lexi = VnLexiName.dd; isUpper = false; return true;
            case ')': lexi = VnLexiName.DD; isUpper = true; return true;
            default: lexi = VnLexiName.NonVnChar; isUpper = false; return false;
        }
    }
}


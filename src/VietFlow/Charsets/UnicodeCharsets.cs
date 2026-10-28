using System.Runtime.CompilerServices;
using System.Text;
using VietFlow.Engine;
using VietFlow.Tables;

namespace VietFlow.Charsets;

public sealed class UnicodeNfcCharset : IVnCharset
{
    public static readonly UnicodeNfcCharset Instance = new();

    public CharsetId Id => CharsetId.Unicode;
    public string Name => "Unicode (Pre-composed / NFC)";

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public int Encode(VnLexiCode code, Span<char> destination)
    {
        if (destination.IsEmpty) return 0;

        if (code.IsVnChar)
        {
            uint idx = code.VnIndex;
            if (idx < CharsetTables.TotalVnChars)
            {
                destination[0] = CharsetTables.UnicodeTable[idx];
                return 1;
            }
        }

        destination[0] = (char)code.Value;
        return 1;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public int EncodeUtf8(VnLexiCode code, Span<byte> destination)
    {
        Span<char> chars = stackalloc char[1];
        int count = Encode(code, chars);
        return Encoding.UTF8.GetBytes(chars[..count], destination);
    }

    public string EncodeToString(VnLexiCode code)
    {
        Span<char> buf = stackalloc char[1];
        int len = Encode(code, buf);
        return new string(buf[..len]);
    }

    public bool TryDecode(char c, out VnLexiCode code)
    {
        int idx = Array.IndexOf(CharsetTables.UnicodeTable, c);
        if (idx >= 0)
        {
            code = new VnLexiCode((uint)idx + CharsetTables.VnStdCharOffset);
            return true;
        }
        if (c < 128)
        {
            code = new VnLexiCode(c);
            return true;
        }
        code = default;
        return false;
    }
}

public sealed class UnicodeCompositeCharset : IVnCharset
{
    public static readonly UnicodeCompositeCharset Instance = new();

    public CharsetId Id => CharsetId.UnicodeComposite;
    public string Name => "Unicode Decomposed / Composite (NFD)";

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public int Encode(VnLexiCode code, Span<char> destination)
    {
        if (destination.IsEmpty) return 0;

        if (code.IsVnChar)
        {
            uint idx = code.VnIndex;
            if (idx < CharsetTables.TotalVnChars)
            {
                uint comp = CharsetTables.UnicodeComposite[idx];
                char baseChar = (char)(comp & 0xFFFF);
                char diacritic = (char)(comp >> 16);

                destination[0] = baseChar;
                if (diacritic != 0 && destination.Length > 1)
                {
                    destination[1] = diacritic;
                    return 2;
                }
                return 1;
            }
        }

        destination[0] = (char)code.Value;
        return 1;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public int EncodeUtf8(VnLexiCode code, Span<byte> destination)
    {
        Span<char> chars = stackalloc char[2];
        int count = Encode(code, chars);
        return Encoding.UTF8.GetBytes(chars[..count], destination);
    }

    public string EncodeToString(VnLexiCode code)
    {
        Span<char> buf = stackalloc char[2];
        int len = Encode(code, buf);
        return new string(buf[..len]);
    }

    public bool TryDecode(char c, out VnLexiCode code)
    {
        if (c < 128)
        {
            code = new VnLexiCode(c);
            return true;
        }
        code = default;
        return false;
    }
}


using System.Runtime.CompilerServices;
using System.Text;
using VietFlow.Engine;
using VietFlow.Tables;

namespace VietFlow.Charsets;

public sealed class Tcvn3Charset : IVnCharset
{
    public static readonly Tcvn3Charset Instance = new();

    public CharsetId Id => CharsetId.Tcvn3;
    public string Name => "TCVN3 (ABC)";

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public int Encode(VnLexiCode code, Span<char> destination)
    {
        if (destination.IsEmpty) return 0;

        if (code.IsVnChar)
        {
            uint idx = code.VnIndex;
            if (idx < CharsetTables.TotalVnChars)
            {
                destination[0] = (char)CharsetTables.Tcvn3Table[idx];
                return 1;
            }
        }

        destination[0] = (char)code.Value;
        return 1;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public int EncodeUtf8(VnLexiCode code, Span<byte> destination)
    {
        if (destination.IsEmpty) return 0;

        if (code.IsVnChar)
        {
            uint idx = code.VnIndex;
            if (idx < CharsetTables.TotalVnChars)
            {
                destination[0] = CharsetTables.Tcvn3Table[idx];
                return 1;
            }
        }

        destination[0] = (byte)code.Value;
        return 1;
    }

    public string EncodeToString(VnLexiCode code)
    {
        Span<char> buf = stackalloc char[1];
        int len = Encode(code, buf);
        return new string(buf[..len]);
    }

    public bool TryDecode(char c, out VnLexiCode code)
    {
        int idx = Array.LastIndexOf(CharsetTables.Tcvn3Table, (byte)c);
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

public sealed class VniWindowsCharset : IVnCharset
{
    public static readonly VniWindowsCharset Instance = new();

    public CharsetId Id => CharsetId.VniWindows;
    public string Name => "VNI-Windows";

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public int Encode(VnLexiCode code, Span<char> destination)
    {
        if (destination.IsEmpty) return 0;

        if (code.IsVnChar)
        {
            uint idx = code.VnIndex;
            if (idx < CharsetTables.TotalVnChars)
            {
                ushort w = CharsetTables.VniWinTable[idx];
                destination[0] = (char)(w & 0xFF);

                if ((w & 0xFF00) != 0 && destination.Length > 1)
                {
                    destination[1] = (char)(w >> 8);
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
        if (destination.IsEmpty) return 0;

        if (code.IsVnChar)
        {
            uint idx = code.VnIndex;
            if (idx < CharsetTables.TotalVnChars)
            {
                ushort w = CharsetTables.VniWinTable[idx];
                destination[0] = (byte)(w & 0xFF);

                if ((w & 0xFF00) != 0 && destination.Length > 1)
                {
                    destination[1] = (byte)(w >> 8);
                    return 2;
                }
                return 1;
            }
        }

        destination[0] = (byte)code.Value;
        return 1;
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

public sealed class ViqrCharset : IVnCharset
{
    public static readonly ViqrCharset Instance = new();

    public CharsetId Id => CharsetId.Viqr;
    public string Name => "VIQR";

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public int Encode(VnLexiCode code, Span<char> destination)
    {
        if (destination.IsEmpty) return 0;

        if (code.IsVnChar)
        {
            uint idx = code.VnIndex;
            if (idx < CharsetTables.TotalVnChars)
            {
                uint dw = CharsetTables.ViqrTable[idx];
                destination[0] = (char)(dw & 0xFF);
                int len = 1;

                if ((dw & 0x0000FF00) != 0 && destination.Length > 1)
                {
                    destination[1] = (char)((dw >> 8) & 0xFF);
                    len = 2;

                    if ((dw & 0x00FF0000) != 0 && destination.Length > 2)
                    {
                        destination[2] = (char)((dw >> 16) & 0xFF);
                        len = 3;
                    }
                }
                return len;
            }
        }

        destination[0] = (char)code.Value;
        return 1;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public int EncodeUtf8(VnLexiCode code, Span<byte> destination)
    {
        if (destination.IsEmpty) return 0;

        if (code.IsVnChar)
        {
            uint idx = code.VnIndex;
            if (idx < CharsetTables.TotalVnChars)
            {
                uint dw = CharsetTables.ViqrTable[idx];
                destination[0] = (byte)(dw & 0xFF);
                int len = 1;

                if ((dw & 0x0000FF00) != 0 && destination.Length > 1)
                {
                    destination[1] = (byte)((dw >> 8) & 0xFF);
                    len = 2;

                    if ((dw & 0x00FF0000) != 0 && destination.Length > 2)
                    {
                        destination[2] = (byte)((dw >> 16) & 0xFF);
                        len = 3;
                    }
                }
                return len;
            }
        }

        destination[0] = (byte)code.Value;
        return 1;
    }

    public string EncodeToString(VnLexiCode code)
    {
        Span<char> buf = stackalloc char[4];
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

public static class CharsetFactory
{
    public static IVnCharset GetCharset(CharsetId id) => id switch
    {
        CharsetId.Unicode => UnicodeNfcCharset.Instance,
        CharsetId.UnicodeComposite => UnicodeCompositeCharset.Instance,
        CharsetId.Tcvn3 => Tcvn3Charset.Instance,
        CharsetId.VniWindows => VniWindowsCharset.Instance,
        CharsetId.Viqr => ViqrCharset.Instance,
        _ => UnicodeNfcCharset.Instance
    };
}


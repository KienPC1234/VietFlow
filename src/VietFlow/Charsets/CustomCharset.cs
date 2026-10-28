// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Text;
using VietFlow.Engine;
using VietFlow.Tables;

namespace VietFlow.Charsets;

/// <summary>
/// User-defined charset. Each standard Vietnamese character maps to a user string;
/// unmapped characters fall back to Unicode.
/// </summary>
public sealed class CustomCharset : IVnCharset
{
    public static readonly CustomCharset Instance = new();

    private volatile string?[] _table = new string?[CharsetTables.TotalVnChars];
    private volatile Dictionary<char, VnLexiCode> _reverse = new();

    public CharsetId Id => CharsetId.Custom;
    public string Name => "Custom";

    public void SetMappings(IReadOnlyDictionary<string, string>? map)
    {
        var table = new string?[CharsetTables.TotalVnChars];
        var reverse = new Dictionary<char, VnLexiCode>(CharsetTables.TotalVnChars);

        if (map is { Count: > 0 })
        {
            for (int i = 0; i < CharsetTables.TotalVnChars; i++)
            {
                string glyph = CharsetTables.UnicodeTable[i].ToString();
                if (map.TryGetValue(glyph, out string? value) && !string.IsNullOrEmpty(value))
                {
                    table[i] = value;

                    if (value.Length == 1 && !reverse.ContainsKey(value[0]))
                    {
                        reverse[value[0]] = new VnLexiCode((uint)i + CharsetTables.VnStdCharOffset);
                    }
                }
            }
        }

        _table = table;
        _reverse = reverse;
    }

    public int Encode(VnLexiCode code, Span<char> destination)
    {
        if (destination.IsEmpty)
        {
            return 0;
        }

        if (code.IsVnChar)
        {
            uint index = code.VnIndex;
            if (index < CharsetTables.TotalVnChars)
            {
                string? mapped = _table[index];
                if (mapped != null)
                {
                    int length = Math.Min(mapped.Length, destination.Length);
                    mapped.AsSpan(0, length).CopyTo(destination);
                    return length;
                }

                destination[0] = CharsetTables.UnicodeTable[index];
                return 1;
            }
        }

        destination[0] = (char)code.Value;
        return 1;
    }

    public int EncodeUtf8(VnLexiCode code, Span<byte> destination)
    {
        Span<char> buffer = stackalloc char[16];
        int length = Encode(code, buffer);
        return Encoding.UTF8.GetBytes(buffer[..length], destination);
    }

    public string EncodeToString(VnLexiCode code)
    {
        Span<char> buffer = stackalloc char[16];
        int length = Encode(code, buffer);
        return new string(buffer[..length]);
    }

    public bool TryDecode(char c, out VnLexiCode code)
    {
        if (_reverse.TryGetValue(c, out code))
        {
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

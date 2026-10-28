// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Text;
using VietFlow.Charsets;
using VietFlow.Engine;
using VietFlow.Tables;

namespace VietFlow.Tools;

public static class ClipboardConverter
{
    /// <summary>
    /// Converts input text from a source character set to a target character set.
    /// </summary>
    public static string Convert(string text, CharsetId source, CharsetId target)
    {
        if (string.IsNullOrEmpty(text) || source == target)
        {
            return text;
        }

        var dstCharset = CharsetFactory.GetCharset(target);
        var sb = new StringBuilder(text.Length * 2);
        Span<char> charBuf = stackalloc char[4];

        if (source == CharsetId.VniWindows)
        {
            for (int i = 0; i < text.Length; i++)
            {
                if (i + 1 < text.Length)
                {
                    ushort pair = (ushort)(((byte)text[i + 1] << 8) | (byte)text[i]);
                    int pairIdx = Array.IndexOf(CharsetTables.VniWinTable, pair);
                    if (pairIdx >= 0)
                    {
                        var code = new VnLexiCode((uint)pairIdx + CharsetTables.VnStdCharOffset);
                        int len = dstCharset.Encode(code, charBuf);
                        sb.Append(charBuf[..len]);
                        i++;
                        continue;
                    }
                }

                ushort single = (ushort)(byte)text[i];
                int singleIdx = -1;
                for (int k = 0; k < CharsetTables.VniWinTable.Length; k++)
                {
                    if (CharsetTables.VniWinTable[k] == single && (single > 127 || single == 'A' || single == 'a'))
                    {
                        singleIdx = k;
                        break;
                    }
                }

                if (singleIdx >= 0 && single > 127)
                {
                    var code = new VnLexiCode((uint)singleIdx + CharsetTables.VnStdCharOffset);
                    int len = dstCharset.Encode(code, charBuf);
                    sb.Append(charBuf[..len]);
                }
                else
                {
                    sb.Append(text[i]);
                }
            }
            return sb.ToString();
        }

        var srcCharset = CharsetFactory.GetCharset(source);
        for (int i = 0; i < text.Length; i++)
        {
            char c = text[i];
            if (srcCharset.TryDecode(c, out var code))
            {
                int len = dstCharset.Encode(code, charBuf);
                sb.Append(charBuf[..len]);
            }
            else
            {
                sb.Append(c);
            }
        }

        return sb.ToString();
    }

    /// <summary>
    /// Strips all Vietnamese diacritics and tones from text (e.g. "Tiếng Việt" -> "Tieng Viet").
    /// </summary>
    public static string StripAccents(string text)
    {
        if (string.IsNullOrEmpty(text))
        {
            return text;
        }

        var sb = new StringBuilder(text.Length);
        for (int i = 0; i < text.Length; i++)
        {
            char c = text[i];
            int idx = Array.IndexOf(CharsetTables.UnicodeTable, c);
            if (idx >= 0 && idx < CharsetTables.TotalVnChars)
            {
                int rootIdx = CharsetTables.StdVnRootChar[idx];
                sb.Append(CharsetTables.UnicodeTable[rootIdx]);
            }
            else
            {
                sb.Append(c);
            }
        }

        return sb.ToString();
    }
}


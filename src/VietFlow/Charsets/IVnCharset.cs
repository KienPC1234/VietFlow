using VietFlow.Engine;

namespace VietFlow.Charsets;

public interface IVnCharset
{
    CharsetId Id { get; }
    string Name { get; }
    int Encode(VnLexiCode code, Span<char> destination);
    int EncodeUtf8(VnLexiCode code, Span<byte> destination);
    string EncodeToString(VnLexiCode code);
    bool TryDecode(char c, out VnLexiCode code);
}


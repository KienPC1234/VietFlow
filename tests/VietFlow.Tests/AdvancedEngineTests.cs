using System.Text;
using VietFlow.Charsets;
using VietFlow.Engine;
using Xunit;

namespace VietFlow.Tests;

public class AdvancedEngineTests
{
    [Fact]
    public void TestModernVsClassicTonePlacement()
    {
        var classic = new VietFlowSession(InputMethod.Telex, CharsetId.Unicode, modernStyle: false);
        var modern = new VietFlowSession(InputMethod.Telex, CharsetId.Unicode, modernStyle: true);

        Assert.Equal("hòa", classic.ProcessString("hoaf"));
        Assert.Equal("hoà", modern.ProcessString("hoaf"));

        Assert.Equal("hòe", classic.ProcessString("hoef"));
        Assert.Equal("hoè", modern.ProcessString("hoef"));

        Assert.Equal("tùy", classic.ProcessString("tuyf"));
        Assert.Equal("tuỳ", modern.ProcessString("tuyf"));

        // Closed syllable (with ending consonant) always places tone on nucleus vowel
        Assert.Equal("hoàng", classic.ProcessString("hoangf"));
        Assert.Equal("hoàng", modern.ProcessString("hoangf"));
    }

    [Fact]
    public void TestMacroExpansion()
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Macros.AddOrUpdate("vn", "Việt Nam");
        session.Macros.AddOrUpdate("chxhcnvn", "Cộng hòa Xã hội Chủ nghĩa Việt Nam");

        string result = session.ProcessString("Toi yeu vn rat nhieu.");
        Assert.Equal("Toi yeu Việt Nam rat nhieu.", result);

        string longMacro = session.ProcessString("chxhcnvn ");
        Assert.Equal("Cộng hòa Xã hội Chủ nghĩa Việt Nam ", longMacro);
    }

    [Fact]
    public void TestBackspaceHandling()
    {
        var session = new VietFlowSession(InputMethod.Telex);

        // Type "vieetj" -> "việt", backspace once -> "việ"
        string result = session.ProcessString("vieetj\b");
        Assert.Equal("việ", result);

        // Backspace all characters
        string cleared = session.ProcessString("vieetj\b\b\b\b");
        Assert.Equal("", cleared);
    }

    [Fact]
    public void TestUnicodeCompositeCharset()
    {
        var session = new VietFlowSession(InputMethod.Telex, CharsetId.UnicodeComposite);
        string result = session.ProcessString("Vieetj");

        // Verify that composite combining diacritic (0x0323 dot below) is used
        Assert.Contains('\u0323', result);
    }

    [Fact]
    public void TestLegacyCharsetsEncoding()
    {
        // TCVN3
        var tcvn = new VietFlowSession(InputMethod.Telex, CharsetId.Tcvn3);
        string tcvnOut = tcvn.ProcessString("Vieetj");
        Assert.False(string.IsNullOrEmpty(tcvnOut));

        // VNI-Windows
        var vni = new VietFlowSession(InputMethod.Telex, CharsetId.VniWindows);
        string vniOut = vni.ProcessString("Vieetj");
        Assert.False(string.IsNullOrEmpty(vniOut));

        // VIQR
        var viqr = new VietFlowSession(InputMethod.Telex, CharsetId.Viqr);
        string viqrOut = viqr.ProcessString("Vieetj");
        Assert.False(string.IsNullOrEmpty(viqrOut));
    }

    [Fact]
    public void TestProcessUtf8Pipeline()
    {
        var session = new VietFlowSession(InputMethod.Telex);
        byte[] inputBytes = Encoding.UTF8.GetBytes("Tieengs Vieetj treen .NET 10");

        Span<byte> outputBytes = stackalloc byte[128];
        int written = session.ProcessUtf8(inputBytes, outputBytes);

        string result = Encoding.UTF8.GetString(outputBytes[..written]);
        Assert.Equal("Tiếng Việt trên .NET 10", result);
    }
}

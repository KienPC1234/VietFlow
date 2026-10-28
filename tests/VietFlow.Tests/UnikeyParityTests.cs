// VietFlow - Regression tests against UniKey's UkEngine semantics.
// Licensed under the GNU General Public License v3.0 (GPLv3)

using VietFlow.Charsets;
using VietFlow.Engine;
using Xunit;

namespace VietFlow.Tests;

public class UnikeyParityTests
{
    [Theory]
    // qu / gi are initial consonants, not vowel clusters (UkEngine::appendConsonnant)
    [InlineData("quow ", "quơ ")]
    [InlineData("quowr ", "quở ")]
    [InlineData("thuow ", "thuơ ")]
    [InlineData("thuowr ", "thuở ")]
    [InlineData("thuowngf ", "thường ")]
    [InlineData("gios ", "gió ")]
    [InlineData("giuowngf ", "giường ")]
    [InlineData("giuwax ", "giữa ")]
    [InlineData("ginf ", "gìn ")]
    [InlineData("quans ", "quán ")]
    [InlineData("quyeets ", "quyết ")]
    // Missing VowelSeq entries: uâ, oă, uơ
    [InlineData("tuaanf ", "tuần ")]
    [InlineData("quaan ", "quân ")]
    [InlineData("xuaan ", "xuân ")]
    [InlineData("luaatj ", "luật ")]
    [InlineData("xuaats ", "xuất ")]
    [InlineData("chuaanr ", "chuẩn ")]
    [InlineData("xoawsn ", "xoắn ")]
    // Double-hook undo (processHookWithUO)
    [InlineData("uoww ", "uow ")]
    [InlineData("dduoww ", "đuow ")]
    [InlineData("thuoww ", "thuow ")]
    public void TelexMatchesUnikeyCoreRules(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Telex);
        Assert.Equal(expected, session.ProcessString(input));
    }

    [Theory]
    // isValidCV: k only goes with e/ê/i/y
    [InlineData("kas ", "kas ")]
    // isValidVC / VCPairList: ai+ng and oe+ng are not legal
    [InlineData("aings ", "aings ")]
    [InlineData("oengs ", "oengs ")]
    // Stop codas c/ch/p/t only accept sắc and nặng
    [InlineData("cacr ", "cacr ")]
    [InlineData("cacx ", "cacx ")]
    [InlineData("cacs ", "các ")]
    public void InvalidVietnameseIsRestored(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Telex);
        Assert.Equal(expected, session.ProcessString(input));
    }

    [Theory]
    // Tone marks may follow a coda even when free marking is disabled (UniKey processTone).
    [InlineData("toans", "toán")]
    [InlineData("vietj", "viẹt")]
    [InlineData("vieetj", "việt")]
    public void ToneWorksWithoutFreeMarking(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Options.FreeMarking = false;
        Assert.Equal(expected, session.ProcessString(input));
    }

    [Theory]
    // Microsoft Vietnamese keyboard layout (MsViMethodMapping)
    [InlineData("Vi3t9", "Việt")]
    [InlineData("1n", "ăn")]
    [InlineData("2n", "ân")]
    [InlineData("3m", "êm")]
    [InlineData("4ng", "ông")]
    [InlineData("hoa8", "hóa")]
    [InlineData("hoa5", "hòa")]
    [InlineData("hoa6", "hỏa")]
    [InlineData("hoa7", "hõa")]
    [InlineData("hoa9", "họa")]
    public void MicrosoftVietnameseLayout(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.MicrosoftVi);
        Assert.Equal(expected, session.ProcessString(input));
    }

    [Theory]
    // Telex bracket shortcuts: [ = ơ, ] = ư, { = Ơ, } = Ư
    [InlineData("[", "ơ")]
    [InlineData("]", "ư")]
    [InlineData("{", "Ơ")]
    [InlineData("}", "Ư")]
    [InlineData("t[", "tơ")]
    [InlineData("th[", "thơ")]
    public void TelexBracketShortcuts(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Telex);
        Assert.Equal(expected, session.ProcessString(input));
    }

    [Fact]
    public void MultiWordMacroExpansion()
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Macros.AddOrUpdate("vn", "Việt Nam");
        session.Macros.AddOrUpdate("cty tnhh", "Công ty TNHH");

        Assert.Equal("Công ty TNHH ", session.ProcessString("cty tnhh "));
        Assert.Equal("Toi yeu Việt Nam ", session.ProcessString("Toi yeu vn "));
    }

    [Fact]
    public void DecomposedUnicodeBackspaceUsesDisplayLength()
    {
        var session = new VietFlowSession(InputMethod.Telex, CharsetId.UnicodeComposite);

        Assert.Equal("Việt", session.ProcessString("Vieetj"));
        Assert.Equal("Việ", session.ProcessString("Vieetj\b"));
        Assert.Equal("Vi", session.ProcessString("Vieetj\b\b"));
        Assert.Equal("V", session.ProcessString("Vieetj\b\b\b"));
    }

    [Fact]
    public void ToneUndoTrailingKeyIsBackspacedFirst()
    {
        var session = new VietFlowSession(InputMethod.Telex);

        Assert.Equal("toans", session.ProcessString("toanss"));
        Assert.Equal("toan", session.ProcessString("toanss\b"));
    }

    [Theory]
    // A stuck/repeated tone key must not leak the raw key back into the word at Space.
    [InlineData("custom ", "custom ")]
    [InlineData("cusstom ", "custom ")]
    [InlineData("toanss ", "toans ")]
    [InlineData("cusstom\b", "custo")]
    public void ToneUndoKeepsBufferAndScreenInSync(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Telex);
        Assert.Equal(expected, session.ProcessString(input));
    }

    [Fact]
    public void AutoCapitalizeAfterSentenceEnd()
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Options.AutoCapitalize = true;

        Assert.Equal("xin chao. Ban khoe khong? Toi khoe!", session.ProcessString("xin chao. ban khoe khong? toi khoe!"));
        Assert.Equal("\nHen gap lai.", session.ProcessString("\nhen gap lai."));
    }

    [Theory]
    [InlineData("d9uong72 ", "đường ")]
    [InlineData("gio1 ", "gió ")]
    [InlineData("tua6n ", "tuân ")]
    public void VniUsesSameUnikeyModel(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Vni);
        Assert.Equal(expected, session.ProcessString(input));
    }

    [Theory]
    [InlineData("quo+ ", "quơ ")]
    [InlineData("gio' ", "gió ")]
    [InlineData("tua^`n ", "tuần ")]
    public void ViqrUsesSameUnikeyModel(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Viqr);
        Assert.Equal(expected, session.ProcessString(input));
    }
}

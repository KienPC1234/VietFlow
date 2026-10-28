// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using VietFlow.Engine;
using VietFlow.Tools;
using Xunit;

namespace VietFlow.Tests;

public class KillerFeaturesTests
{
    [Fact]
    public void TestShiftLagCorrection()
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Options.FixShiftLag = true;

        // "VIeetj" should automatically demote the lagging second upper 'I' -> 'i' producing "Việt"
        string result = session.ProcessString("VIeetj");
        Assert.Equal("Việt", result);

        // "TRuwowngf" should automatically demote 'R' -> 'r' producing "Trường"
        string truong = session.ProcessString("TRuwowngf");
        Assert.Equal("Trường", truong);

        // Normal all-caps should stay all-caps: "VIEETJ" -> "VIỆT"
        string allCaps = session.ProcessString("VIEETJ");
        Assert.Equal("VIỆT", allCaps);
    }

    [Fact]
    public void TestConsonantShortcuts()
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Options.ConsonantShortcuts = true;

        // Initial f -> ph
        Assert.Equal("phát", session.ProcessString("fats"));
        Assert.Equal("Phát", session.ProcessString("Fats"));

        // Initial j -> gi
        Assert.Equal("giá", session.ProcessString("jas"));
        Assert.Equal("Giá", session.ProcessString("Jas"));

        // Initial w -> qu
        Assert.Equal("quê", session.ProcessString("wee"));
        Assert.Equal("Quê", session.ProcessString("Wee"));
    }

    [Fact]
    public void TestDynamicMacroVariables()
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Macros.ClipboardProvider = () => "PastedContent";

        session.Macros.AddOrUpdate("clip", "Result: @clipboard");
        session.Macros.AddOrUpdate("today", "Date: @date");

        string clipResult = session.ProcessString("clip ");
        Assert.Equal("Result: PastedContent ", clipResult);

        string dateResult = session.ProcessString("today ");
        string expectedDate = DateTime.Now.ToString("dd/MM/yyyy");
        Assert.Equal($"Date: {expectedDate} ", dateResult);
    }

    [Fact]
    public void TestExtendedMacroVariables()
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Macros.DateFormat = "yyyy-MM-dd";
        session.Macros.TimeFormat = "HH_mm";
        session.Macros.CustomVariables = new Dictionary<string, string>
        {
            ["name"] = "Kien"
        };

        session.Macros.AddOrUpdate("qqq", "@date|@time|@name|@year");
        string result = session.ProcessString("qqq ");

        string expectedDate = DateTime.Now.ToString("yyyy-MM-dd");
        string expectedTime = DateTime.Now.ToString("HH_mm");
        Assert.Equal($"{expectedDate}|{expectedTime}|Kien|{DateTime.Now:yyyy} ", result);

        session.Macros.AddOrUpdate("qnl", "A @newline B");
        Assert.Equal("A \n B ", session.ProcessString("qnl "));

        session.Macros.AddOrUpdate("qqr", "@random");
        string random = session.ProcessString("qqr ").Trim();
        Assert.Equal(6, random.Length);
        Assert.All(random, c => Assert.True(char.IsDigit(c)));
    }

    [Fact]
    public void TestClipboardConverter()
    {
        string original = "Cộng hòa Xã hội Chủ nghĩa Việt Nam";

        // Test Unicode to TCVN3 and back
        string tcvn3 = ClipboardConverter.Convert(original, CharsetId.Unicode, CharsetId.Tcvn3);
        Assert.NotEqual(original, tcvn3);
        string backToUnicode = ClipboardConverter.Convert(tcvn3, CharsetId.Tcvn3, CharsetId.Unicode);
        Assert.Equal(original, backToUnicode);

        // Test Unicode to VNI-Windows and back
        string vniWin = ClipboardConverter.Convert(original, CharsetId.Unicode, CharsetId.VniWindows);
        Assert.NotEqual(original, vniWin);
        string backFromVni = ClipboardConverter.Convert(vniWin, CharsetId.VniWindows, CharsetId.Unicode);
        Assert.Equal(original, backFromVni);

        // Test StripAccents
        string stripped = ClipboardConverter.StripAccents(original);
        Assert.Equal("Cong hoa Xa hoi Chu nghia Viet Nam", stripped);
    }
}

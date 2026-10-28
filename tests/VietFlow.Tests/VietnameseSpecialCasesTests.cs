// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using VietFlow.Charsets;
using VietFlow.Engine;
using Xunit;

namespace VietFlow.Tests;

public class VietnameseSpecialCasesTests
{
    [Theory]
    [InlineData("chuyeen", "chuyên")]
    [InlineData("truyeenf", "truyền")]
    [InlineData("khuyeen", "khuyên")]
    [InlineData("nguyeen", "nguyên")]
    [InlineData("huyeenj", "huyện")]
    [InlineData("thuyeenf", "thuyền")]
    [InlineData("quyeenf", "quyền")]
    [InlineData("chuyeens", "chuyến")]
    [InlineData("nguyeenz", "nguyên")]
    [InlineData("nguyeenx", "nguyễn")]
    [InlineData("hoanf", "hoàn")]
    [InlineData("toans", "toán")]
    [InlineData("khoans", "khoán")]
    [InlineData("ddoanf", "đoàn")]
    [InlineData("hoangf", "hoàng")]
    [InlineData("loangx", "loãng")]
    [InlineData("thoangs", "thoáng")]
    [InlineData("vuownf", "vườn")]
    [InlineData("vuonwf", "vườn")]
    [InlineData("luowjn", "lượn")]
    [InlineData("luonwj", "lượn")]
    [InlineData("muowjn", "mượn")]
    [InlineData("muonwj", "mượn")]
    [InlineData("truwowngf", "trường")]
    [InlineData("phuowng", "phương")]
    [InlineData("huowngs", "hướng")]
    [InlineData("nhieeuf", "nhiều")]
    [InlineData("tieeu", "tiêu")]
    [InlineData("kieeuj", "kiệu")]
    [InlineData("hieeur", "hiểu")]
    [InlineData("xoay", "xoay")]
    [InlineData("loay", "loay")]
    [InlineData("ngoays", "ngoáy")]
    [InlineData("ngoaif", "ngoài")]
    [InlineData("khoais", "khoái")]
    [InlineData("loaij", "loại")]
    [InlineData("xuaan", "xuân")]
    [InlineData("chuaanr", "chuẩn")]
    [InlineData("luaanj", "luận")]
    [InlineData("xuaats", "xuất")]
    [InlineData("luaatj", "luật")]
    public void TestComplexVowelSequences(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Telex);
        Assert.Equal(expected, session.ProcessString(input));
    }

    [Fact]
    public void TestToneAlterationAndRemoval()
    {
        var session = new VietFlowSession(InputMethod.Telex);

        // Replace Sac with Huyen directly
        Assert.Equal("toàn", session.ProcessString("toansf"));

        // Replace Huyen with Hoi directly
        Assert.Equal("toản", session.ProcessString("toanfr"));

        // Remove tone with 'z'
        Assert.Equal("toan", session.ProcessString("toansz"));
        Assert.Equal("viêt", session.ProcessString("vieetjz"));

        // Double tone removes tone and outputs raw key
        Assert.Equal("toans", session.ProcessString("toanss"));
    }

    [Fact]
    public void TestDoubleModifierKeyUndo()
    {
        var session = new VietFlowSession(InputMethod.Telex);

        // aa -> â, aaa -> aa
        Assert.Equal("â", session.ProcessString("aa"));
        Assert.Equal("aa", session.ProcessString("aaa"));

        // ee -> ê, eee -> ee
        Assert.Equal("ê", session.ProcessString("ee"));
        Assert.Equal("ee", session.ProcessString("eee"));

        // oo -> ô, ooo -> oo
        Assert.Equal("ô", session.ProcessString("oo"));
        Assert.Equal("oo", session.ProcessString("ooo"));

        // dd -> đ, ddd -> dd
        Assert.Equal("đ", session.ProcessString("dd"));
        Assert.Equal("dd", session.ProcessString("ddd"));
    }

    [Fact]
    public void TestSimpleTelexVsStandardTelex()
    {
        var standard = new VietFlowSession(InputMethod.Telex);
        var simple = new VietFlowSession(InputMethod.SimpleTelex);

        // In standard Telex, lone 'w' produces 'ư'
        Assert.Equal("ư", standard.ProcessString("w"));
        Assert.Equal("Ư", standard.ProcessString("W"));

        // In Simple Telex, lone 'w' stays 'w'
        Assert.Equal("w", simple.ProcessString("w"));
        Assert.Equal("W", simple.ProcessString("W"));
        Assert.Equal("win", simple.ProcessString("win"));
        Assert.Equal("wifi", simple.ProcessString("wifi"));
        Assert.Equal("wiki", simple.ProcessString("wiki"));
        Assert.Equal("water", simple.ProcessString("water"));
        Assert.Equal("wash", simple.ProcessString("wash"));
        Assert.Equal("wasd", simple.ProcessString("wasd"));
        Assert.Equal("while", simple.ProcessString("while"));
        Assert.Equal("games", simple.ProcessString("games"));

        // But 'uw' and 'ow' still produce 'ư' and 'ơ' in Simple Telex
        Assert.Equal("tư", simple.ProcessString("tuw"));
        Assert.Equal("tơ", simple.ProcessString("tow"));
        Assert.Equal("đường", simple.ProcessString("dduowngf"));
    }

    [Fact]
    public void TestStopConsonantsPhonetics()
    {
        var session = new VietFlowSession(InputMethod.Telex);

        // Words ending in c, ch, p, t take Acute (Sac) or Dot (Nang)
        Assert.Equal("hát", session.ProcessString("hats"));
        Assert.Equal("hạt", session.ProcessString("hatj"));
        Assert.Equal("bác", session.ProcessString("bacs"));
        Assert.Equal("bạc", session.ProcessString("bacj"));
        Assert.Equal("thích", session.ProcessString("thichs"));
        Assert.Equal("thịt", session.ProcessString("thitj"));
        Assert.Equal("đẹp", session.ProcessString("ddejp"));
    }

    [Fact]
    public void TestCapsLockAndUpperCaseCombinations()
    {
        var session = new VietFlowSession(InputMethod.Telex);

        Assert.Equal("VIỆT", session.ProcessString("VIEETJ"));
        Assert.Equal("TOÁN", session.ProcessString("TOANS"));
        Assert.Equal("TRƯỜNG", session.ProcessString("TRUWOWNGF"));
        Assert.Equal("HÒA", session.ProcessString("HOAF"));
    }

    [Fact]
    public void TestModernVsClassicDiphthongs()
    {
        var classic = new VietFlowSession(InputMethod.Telex, CharsetId.Unicode, modernStyle: false);
        var modern = new VietFlowSession(InputMethod.Telex, CharsetId.Unicode, modernStyle: true);

        // oa
        Assert.Equal("hòa", classic.ProcessString("hoaf"));
        Assert.Equal("hoà", modern.ProcessString("hoaf"));

        // oe
        Assert.Equal("khỏe", classic.ProcessString("khoefr"));
        Assert.Equal("khoẻ", modern.ProcessString("khoefr"));

        // uy
        Assert.Equal("tùy", classic.ProcessString("tuyf"));
        Assert.Equal("tuỳ", modern.ProcessString("tuyf"));
    }

    [Fact]
    public void TestVniSpecialCases()
    {
        var session = new VietFlowSession(InputMethod.Vni);

        Assert.Equal("Việt", session.ProcessString("Vie6t5"));
        Assert.Equal("Nam", session.ProcessString("Nam"));
        Assert.Equal("hoàn", session.ProcessString("hoan2"));
        Assert.Equal("toán", session.ProcessString("toan1"));
        Assert.Equal("đường", session.ProcessString("d9uong72"));
    }

    [Theory]
    [InlineData("mix", "mĩ")]
    [InlineData("box", "bõ")]
    [InlineData("six", "sĩ")]
    [InlineData("max", "mã")]
    [InlineData("tax", "tã")]
    [InlineData("car", "cả")]
    [InlineData("bar", "bả")]
    [InlineData("sir", "sỉ")]
    [InlineData("nor", "nỏ")]
    [InlineData("bus", "bú")]
    [InlineData("bust", "bút")]
    [InlineData("rust", "rút")]
    [InlineData("must", "mút")]
    [InlineData("trust", "trút")]
    [InlineData("gust", "gút")]
    [InlineData("cast", "cát")]
    [InlineData("last", "lát")]
    [InlineData("test", "tét")]
    [InlineData("best", "bét")]
    [InlineData("rest", "rét")]
    [InlineData("host", "hót")]
    [InlineData("cost", "cót")]
    [InlineData("list", "lít")]
    [InlineData("most", "mót")]
    [InlineData("boost", "bốt")]
    [InlineData("moon", "môn")]
    [InlineData("noon", "nôn")]
    [InlineData("soon", "sôn")]
    [InlineData("seen", "sên")]
    [InlineData("room", "rôm")]
    [InlineData("sum", "sum")]
    [InlineData("vans", "ván")]
    [InlineData("lens", "lén")]
    public void TestVietnameseWordsNonConflictWithEnglishKeywords(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Options.SpellCheck = true;
        session.Options.AutoNonVnRestore = true;
        Assert.Equal(expected, session.ProcessString(input));
    }

    [Theory]
    [InlineData("fix", "fix")]
    [InlineData("fit", "fit")]
    [InlineData("file", "file")]
    [InlineData("flex", "flex")]
    [InlineData("fox", "fox")]
    [InlineData("fax", "fax")]
    [InlineData("feed", "feed")]
    [InlineData("food", "food")]
    [InlineData("zoom", "zoom")]
    [InlineData("json", "json")]
    [InlineData("java", "java")]
    [InlineData("opencode", "opencode")]
    [InlineData("opencode ", "opencode ")]
    [InlineData("code", "code")]
    [InlineData("daod", "daod")]
    [InlineData("cool ", "cool ")]
    [InlineData("root ", "root ")]
    [InlineData("good ", "good ")]
    [InlineData("custom ", "custom ")]
    [InlineData("customer ", "customer ")]
    [InlineData("system ", "system ")]
    [InlineData("linux ", "linux ")]
    public void TestEnglishAndCodingKeywords(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Options.SpellCheck = true;
        session.Options.AutoNonVnRestore = true;
        Assert.Equal(expected, session.ProcessString(input));
    }

    [Fact]
    public void TestCustomStepByStep()
    {
        var session = new VietFlowSession(InputMethod.Telex);
        session.Options.SpellCheck = true;
        session.Options.AutoNonVnRestore = true;

        var rC = session.ProcessKey('c');
        Assert.Equal(0, rC.Backspaces);
        Assert.Equal("c", rC.Output);

        var rU = session.ProcessKey('u');
        Assert.Equal(0, rU.Backspaces);
        Assert.Equal("u", rU.Output);

        var rS = session.ProcessKey('s');
        Assert.Equal(1, rS.Backspaces); // only deletes 'u', leaves 'c' untouched!
        Assert.Equal("ú", rS.Output);

        var rT = session.ProcessKey('t');
        Assert.Equal(0, rT.Backspaces); // directly appends 't' to 'cú' -> 'cút'
        Assert.Equal("t", rT.Output);

        var rO = session.ProcessKey('o');
        // 'o' after coda 't' is non-Vietnamese syllable boundary; restores raw 'custo' without touching 'c'
        Assert.Equal(2, rO.Backspaces);
        Assert.Equal("usto", rO.Output);

        var rM = session.ProcessKey('m');
        Assert.Equal(0, rM.Backspaces);
        Assert.Equal("m", rM.Output);

        var rSpace = session.ProcessKey(' ');
        // By space, word is already restored! Zero backspaces, zero delay, zero flickering
        Assert.Equal(0, rSpace.Backspaces);
        Assert.Equal(" ", rSpace.Output);

        Assert.Equal("custom ", session.ProcessString("custom "));
    }
}

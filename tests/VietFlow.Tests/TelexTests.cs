using VietFlow.Engine;
using Xunit;

namespace VietFlow.Tests;

public class TelexTests
{
    private readonly VietFlowSession _session = new(InputMethod.Telex);

    [Theory]
    // Vowels with tones
    [InlineData("as", "á")]
    [InlineData("af", "à")]
    [InlineData("ar", "ả")]
    [InlineData("ax", "ã")]
    [InlineData("aj", "ạ")]
    // Circumflex
    [InlineData("aa", "â")]
    [InlineData("aas", "ấ")]
    [InlineData("aaf", "ầ")]
    [InlineData("aar", "ẩ")]
    [InlineData("aax", "ẫ")]
    [InlineData("aaj", "ậ")]
    [InlineData("ee", "ê")]
    [InlineData("ees", "ế")]
    [InlineData("eef", "ề")]
    [InlineData("eer", "ể")]
    [InlineData("eex", "ễ")]
    [InlineData("eej", "ệ")]
    [InlineData("oo", "ô")]
    [InlineData("oos", "ố")]
    [InlineData("oof", "ồ")]
    [InlineData("oor", "ổ")]
    [InlineData("oox", "ỗ")]
    [InlineData("ooj", "ộ")]
    // Breve & Horn with 'w'
    [InlineData("aw", "ă")]
    [InlineData("aws", "ắ")]
    [InlineData("awf", "ằ")]
    [InlineData("awr", "ẳ")]
    [InlineData("awx", "ẵ")]
    [InlineData("awj", "ặ")]
    [InlineData("ow", "ơ")]
    [InlineData("ows", "ớ")]
    [InlineData("owf", "ờ")]
    [InlineData("owr", "ở")]
    [InlineData("owx", "ỡ")]
    [InlineData("owj", "ợ")]
    [InlineData("uw", "ư")]
    [InlineData("uws", "ứ")]
    [InlineData("uwf", "ừ")]
    [InlineData("uwr", "ử")]
    [InlineData("uwx", "ữ")]
    [InlineData("uwj", "ự")]
    // Lone 'w' produces 'ư' in Telex
    [InlineData("w", "ư")]
    [InlineData("ws", "ứ")]
    // Consonant 'dd' -> 'đ'
    [InlineData("dd", "đ")]
    [InlineData("dda", "đa")]
    [InlineData("ddas", "đá")]
    // Words & Syllables
    [InlineData("vieetj", "việt")]
    [InlineData("nam", "nam")]
    [InlineData("tieengs", "tiếng")]
    [InlineData("dduowngf", "đường")]
    [InlineData("nguowif", "người")]
    [InlineData("hoangf", "hoàng")]
    [InlineData("thuowng", "thương")]
    [InlineData("nguyeexn", "nguyễn")]
    [InlineData("chuyeenj", "chuyện")]
    [InlineData("truowfng", "trường")]
    // Sentence
    [InlineData("Tieengs Vieetj laf tieengs mej dder.", "Tiếng Việt là tiếng mẹ đẻ.")]
    public void TestTelexInput(string input, string expected)
    {
        string actual = _session.ProcessString(input);
        Assert.Equal(expected, actual);
    }

    [Theory]
    // Tone undo with double key
    [InlineData("ass", "as")]
    [InlineData("aff", "af")]
    [InlineData("arr", "ar")]
    // Circumflex undo with 3rd key
    [InlineData("aaa", "aa")]
    [InlineData("eee", "ee")]
    [InlineData("ooo", "oo")]
    // 'w' undo
    [InlineData("aww", "aw")]
    [InlineData("oww", "ow")]
    [InlineData("uww", "uw")]
    // 'dd' undo
    [InlineData("ddd", "dd")]
    // Tone removal with 'z'
    [InlineData("asz", "a")]
    [InlineData("vieetjz", "viêt")]
    public void TestTelexUndo(string input, string expected)
    {
        string actual = _session.ProcessString(input);
        Assert.Equal(expected, actual);
    }
}

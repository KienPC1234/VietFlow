using VietFlow.Engine;
using Xunit;

namespace VietFlow.Tests;

public class VniAndViqrTests
{
    [Theory]
    // VNI tones: 1 (sắc), 2 (huyền), 3 (hỏi), 4 (ngã), 5 (nặng), 0 (xóa)
    [InlineData("a1", "á")]
    [InlineData("a2", "à")]
    [InlineData("a3", "ả")]
    [InlineData("a4", "ã")]
    [InlineData("a5", "ạ")]
    [InlineData("a10", "a")]
    // VNI diacritics: 6 (^), 7 (+), 8 ((), 9 (d)
    [InlineData("a6", "â")]
    [InlineData("e6", "ê")]
    [InlineData("o6", "ô")]
    [InlineData("a8", "ă")]
    [InlineData("o7", "ơ")]
    [InlineData("u7", "ư")]
    [InlineData("d9", "đ")]
    // Combinations
    [InlineData("vie6t5", "việt")]
    [InlineData("nam", "nam")]
    [InlineData("tie6ng1", "tiếng")]
    [InlineData("d9u7o7ng2", "đường")]
    public void TestVniInput(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Vni);
        string actual = session.ProcessString(input);
        Assert.Equal(expected, actual);
    }

    [Theory]
    // VIQR tones: ' ` ? ~ .
    [InlineData("a'", "á")]
    [InlineData("a`", "à")]
    [InlineData("a?", "ả")]
    [InlineData("a~", "ã")]
    [InlineData("a.", "ạ")]
    // VIQR diacritics: ^ + (
    [InlineData("a^", "â")]
    [InlineData("e^", "ê")]
    [InlineData("o^", "ô")]
    [InlineData("a(", "ă")]
    [InlineData("o+", "ơ")]
    [InlineData("u+", "ư")]
    public void TestViqrInput(string input, string expected)
    {
        var session = new VietFlowSession(InputMethod.Viqr);
        string actual = session.ProcessString(input);
        Assert.Equal(expected, actual);
    }
}


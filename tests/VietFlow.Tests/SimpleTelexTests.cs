using VietFlow.Engine;
using Xunit;

namespace VietFlow.Tests;

public class SimpleTelexTests
{
    private readonly VietFlowSession _session = new(InputMethod.SimpleTelex);

    [Theory]
    // Lone 'w' must remain 'w' in Simple Telex
    [InlineData("w", "w")]
    [InlineData("web", "web")]
    [InlineData("wiki", "wiki")]
    // 'w' modifies preceding vowels
    [InlineData("aw", "ă")]
    [InlineData("ow", "ơ")]
    [InlineData("uw", "ư")]
    [InlineData("aws", "ắ")]
    [InlineData("dduowngf", "đường")]
    [InlineData("nguowif", "người")]
    // Double keys and tones
    [InlineData("aa", "â")]
    [InlineData("ee", "ê")]
    [InlineData("oo", "ô")]
    [InlineData("dd", "đ")]
    [InlineData("vieetj", "việt")]
    [InlineData("nam", "nam")]
    [InlineData("ddas", "đá")]
    public void TestSimpleTelex(string input, string expected)
    {
        string actual = _session.ProcessString(input);
        Assert.Equal(expected, actual);
    }
}

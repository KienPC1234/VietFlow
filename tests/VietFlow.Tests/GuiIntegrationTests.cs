using VietFlow.Engine;
using VietFlow.Integration;
using Xunit;

namespace VietFlow.Tests;

public class GuiIntegrationTests
{
    [Fact]
    public void TestApplyToTextBufferSimulation()
    {
        var session = new VietFlowSession(InputMethod.Telex);
        string text = "Xin chao ";
        int caret = text.Length;

        // Type 'v', 'i', 'e', 'e', 't', 'j'
        string typed = "vieetj";
        for (int i = 0; i < typed.Length; i++)
        {
            var res = session.ProcessKey(typed[i]);
            GuiIntegration.ApplyToTextBuffer(ref text, ref caret, in res);
        }

        Assert.Equal("Xin chao việt", text);
        Assert.Equal(13, caret);

        // Type space
        var spaceRes = session.ProcessKey(' ');
        GuiIntegration.ApplyToTextBuffer(ref text, ref caret, in spaceRes);
        Assert.Equal("Xin chao việt ", text);

        // Type 'N', 'a', 'm'
        string typed2 = "Nam";
        for (int i = 0; i < typed2.Length; i++)
        {
            var res = session.ProcessKey(typed2[i]);
            GuiIntegration.ApplyToTextBuffer(ref text, ref caret, in res);
        }

        Assert.Equal("Xin chao việt Nam", text);
    }

    [Fact]
    public void TestModifierHotkeyBypassAndReset()
    {
        var session = new VietFlowSession(InputMethod.Telex);

        // Type 'v', 'i'
        session.ProcessKey('v');
        session.ProcessKey('i');

        // User hits Ctrl+C (copy) -> must be unhandled and reset buffer
        var copyRes = session.ProcessKey('c', ModifierKeys.Control);
        Assert.False(copyRes.Handled);

        // Next character 'e' starts a fresh word, not continuing 'vi'
        var nextRes = session.ProcessKey('e');
        Assert.Equal("e", nextRes.Output);
    }

    [Fact]
    public void TestNavigationKeysResetBuffer()
    {
        var session = new VietFlowSession(InputMethod.Telex);

        // Type 'v', 'i'
        session.ProcessKey('v');
        session.ProcessKey('i');

        // User presses Left Arrow (caret moved)
        var navRes = session.ProcessVirtualKey(VirtualKeys.Left);
        Assert.False(navRes.Handled);

        // Next character 'e' starts fresh
        var nextRes = session.ProcessKey('e');
        Assert.Equal("e", nextRes.Output);
    }

    [Fact]
    public void TestShiftAndCapsLockModifiers()
    {
        var session = new VietFlowSession(InputMethod.Telex);

        // Shift pressed -> uppercase 'A'
        var res1 = session.ProcessKey('a', ModifierKeys.Shift);
        Assert.Equal("A", res1.Output);

        session.Reset();

        // CapsLock active -> uppercase 'A'
        var res2 = session.ProcessKey('a', ModifierKeys.CapsLock);
        Assert.Equal("A", res2.Output);

        session.Reset();

        // CapsLock + Shift -> inverts to lowercase 'a'
        var res3 = session.ProcessKey('a', ModifierKeys.CapsLock | ModifierKeys.Shift);
        Assert.Equal("a", res3.Output);
    }

    [Fact]
    public void TestIsEnabledToggle()
    {
        var session = new VietFlowSession(InputMethod.Telex);

        Assert.True(session.IsEnabled);

        // Type 'a', 's' with engine disabled
        session.IsEnabled = false;
        var res = session.ProcessKey('s');
        Assert.False(res.Handled);

        // Enable again
        session.IsEnabled = true;
        session.ProcessKey('a');
        var res2 = session.ProcessKey('s');
        Assert.True(res2.Handled);
        Assert.Equal("á", res2.Output);
    }

    [Fact]
    public void TestWpfModifierConversion()
    {
        // WPF: Control (2) + Shift (4) = 6
        var mods = GuiIntegration.FromWpfModifiers(6, isCapsLockOn: true);

        Assert.True((mods & ModifierKeys.Control) != 0);
        Assert.True((mods & ModifierKeys.Shift) != 0);
        Assert.True((mods & ModifierKeys.CapsLock) != 0);
        Assert.False((mods & ModifierKeys.Alt) != 0);
    }
}

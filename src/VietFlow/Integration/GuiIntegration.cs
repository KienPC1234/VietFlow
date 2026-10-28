// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Runtime.CompilerServices;
using VietFlow.Engine;

namespace VietFlow.Integration;

/// <summary>
/// Helpers for seamless integration into WPF, WinForms, Avalonia, MAUI, or Win32 low-level keyboard hooks.
/// </summary>
public static class GuiIntegration
{
    /// <summary>
    /// Applies a KeyProcessResult directly to an in-memory text buffer and caret position.
    /// Ideal for WPF TextBox, Avalonia, or custom text editing controls.
    /// </summary>
    /// <param name="text">The current text content of the control.</param>
    /// <param name="caretIndex">The current caret position (0-based).</param>
    /// <param name="result">The result from VietFlowSession.ProcessKey.</param>
    /// <returns>True if the buffer was modified; otherwise false.</returns>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool ApplyToTextBuffer(
        ref string text,
        ref int caretIndex,
        in KeyProcessResult result)
    {
        if (!result.Handled)
        {
            return false;
        }

        // 1. Remove backspaced characters before the caret
        if (result.Backspaces > 0)
        {
            int removeCount = Math.Min(result.Backspaces, caretIndex);
            int startIndex = caretIndex - removeCount;
            text = text.Remove(startIndex, removeCount);
            caretIndex = startIndex;
        }

        // 2. Insert new characters at the caret
        if (result.HasOutput)
        {
            text = text.Insert(caretIndex, result.Output);
            caretIndex += result.Output.Length;
        }

        return true;
    }

    /// <summary>
    /// Converts standard WPF ModifierKeys bitmask (System.Windows.Input.ModifierKeys)
    /// to VietFlow ModifierKeys without taking a compile-time dependency on PresentationCore.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ModifierKeys FromWpfModifiers(int wpfModifierInt, bool isCapsLockOn = false)
    {
        // WPF System.Windows.Input.ModifierKeys mapping:
        // None = 0, Alt = 1, Control = 2, Shift = 4, Windows = 8
        ModifierKeys result = ModifierKeys.None;

        if ((wpfModifierInt & 1) != 0) result |= ModifierKeys.Alt;
        if ((wpfModifierInt & 2) != 0) result |= ModifierKeys.Control;
        if ((wpfModifierInt & 4) != 0) result |= ModifierKeys.Shift;
        if ((wpfModifierInt & 8) != 0) result |= ModifierKeys.Windows;
        if (isCapsLockOn) result |= ModifierKeys.CapsLock;

        return result;
    }

    /// <summary>
    /// Converts standard WinForms Keys bitmask (System.Windows.Forms.Keys)
    /// to VietFlow ModifierKeys without taking a compile-time dependency on System.Windows.Forms.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ModifierKeys FromWinFormsModifiers(int winFormsKeysInt, bool isCapsLockOn = false)
    {
        // WinForms Keys mapping:
        // Shift = 0x00010000, Control = 0x00020000, Alt = 0x00040000
        ModifierKeys result = ModifierKeys.None;

        if ((winFormsKeysInt & 0x00010000) != 0) result |= ModifierKeys.Shift;
        if ((winFormsKeysInt & 0x00020000) != 0) result |= ModifierKeys.Control;
        if ((winFormsKeysInt & 0x00040000) != 0) result |= ModifierKeys.Alt;
        if (isCapsLockOn) result |= ModifierKeys.CapsLock;

        return result;
    }
}

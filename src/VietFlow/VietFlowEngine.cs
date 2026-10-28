// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using VietFlow.Charsets;
using VietFlow.Tables;

namespace VietFlow.Engine;

[InlineArray(128)]
public struct WordBuffer
{
    private WordInfo _element0;
}

[StructLayout(LayoutKind.Sequential)]
public struct WordInfo
{
    public char RawChar;
    public VnLexiName Lexi;
    public VnLexiTone Tone;
    public CharType Type;
    public bool IsUpper;
    public bool Modified;
}

public sealed class VietFlowEngine
{
    private const int MaxWordLength = 127;
    private const int MaxMacroTokens = 16;
    private const int MaxMacroChars = 64;
    private const int MacroBufferSize = 128;
    private const int MaxOutputChars = 512;

    private EngineOptions _options;
    private IVnCharset _charset;
    private bool _singleCharCharset;
    private readonly MacroTable _macros;

    private WordBuffer _buffer;
    private int _bufferLength;
    private WordForm _wordForm;

    private readonly char[] _rawChars = new char[128];
    private int _rawCharCount;

    private VnLexiTone _currentTone;
    private bool _hasEndingConsonant;
    private bool _capitalizeNext;

    // Incremental nucleus state: avoids rescanning the buffer on every keystroke.
    private int _vowelStartIndex = -1;
    private int _vowelCount;
    private int _fastVowelKey = ToneRules.EmptyVowelKey; // compact slot key for O(1) validation

    // Number of characters currently rendered on screen for the active word.
    // Differs from _bufferLength for charsets that emit combining sequences (NFD, VNI-Windows, VIQR).
    private int _displayLength;

    // Completed words (rendered text + separators) used for multi-word macro matching,
    // mirroring UniKey's key buffer. Backspaces for a match = (end - start) chars.
    private readonly char[] _macroBuffer = new char[MacroBufferSize];
    private readonly int[] _macroTokenStarts = new int[MaxMacroTokens];
    private int _macroBufferLength;
    private int _macroTokenCount;

    public EngineOptions Options
    {
        get => _options;
        set => _options = value;
    }

    public IVnCharset Charset
    {
        get => _charset;
        set
        {
            var charset = value ?? UnicodeNfcCharset.Instance;
            if (!ReferenceEquals(charset, _charset))
            {
                _charset = charset;
                _singleCharCharset = charset.Id is CharsetId.Unicode or CharsetId.Tcvn3;
                ResetWord();
            }
        }
    }

    public MacroTable Macros => _macros;

    public VietFlowEngine(
        EngineOptions? options = null,
        IVnCharset? charset = null,
        MacroTable? macros = null)
    {
        _options = options ?? new EngineOptions();
        _charset = charset ?? UnicodeNfcCharset.Instance;
        _singleCharCharset = _charset.Id is CharsetId.Unicode or CharsetId.Tcvn3;
        _macros = macros ?? new MacroTable();
        Reset();
    }

    public void Reset()
    {
        ResetWord();
        ClearMacroHistory();
        _capitalizeNext = false;
    }

    private void ClearMacroHistory()
    {
        _macroBufferLength = 0;
        _macroTokenCount = 0;
    }

    private void ResetWord()
    {
        _bufferLength = 0;
        _rawCharCount = 0;
        _wordForm = WordForm.Empty;
        _currentTone = VnLexiTone.None;
        _hasEndingConsonant = false;
        _displayLength = 0;
        ClearVowels();
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public KeyProcessResult ProcessKey(char c)
    {
        Span<char> outSpan = stackalloc char[MaxOutputChars];
        int written = ProcessKey(c, outSpan, out int backspaces);
        return new KeyProcessResult(backspaces, new string(outSpan[..written]), OutputType.Direct);
    }

    public int ProcessKey(char c, Span<char> output, out int backspaces)
    {
        backspaces = 0;

        if (c == '\b')
        {
            return ProcessBackspace(output, out backspaces);
        }

        // Telex bracket shortcuts: [ = ơ, ] = ư, { = Ơ, } = Ư
        if ((_options.Method is InputMethod.Telex or InputMethod.SimpleTelex) &&
            TryBracketShortcut(c, output, out backspaces, out int bracketWritten))
        {
            return bracketWritten;
        }

        if (_bufferLength == 0)
        {
            if (_options.ConsonantShortcuts && (c is 'f' or 'F' or 'j' or 'J' or 'w' or 'W'))
            {
                return ProcessFirstLetter(c, output, out backspaces);
            }

            if (_options.Method == InputMethod.Telex && (c is 'w' or 'W'))
            {
                return ProcessLoneW(c, output, out backspaces);
            }

            if (InputProcessor.IsLetterFast(c))
            {
                return ProcessFirstLetter(c, output, out backspaces);
            }

            if (_options.AutoCapitalize && c is '.' or '!' or '?' or '\r' or '\n')
            {
                _capitalizeNext = true;
            }

            output[0] = c;
            return 1;
        }

        // Try tone key
        ToneKeyType toneKey = InputProcessor.GetToneKey(c, _options.Method);
        if (toneKey != ToneKeyType.None)
        {
            if (TryApplyTone(toneKey, c, output, out backspaces, out int written))
            {
                if (_rawCharCount < MaxWordLength) _rawChars[_rawCharCount++] = c;
                return written;
            }
        }

        // Try modify key
        ModifyKeyType modifyKey = InputProcessor.GetModifyKey(c, _options.Method);
        if (modifyKey != ModifyKeyType.None || IsDoubleKeyModify(c))
        {
            if (TryApplyModify(modifyKey, c, output, out backspaces, out int written))
            {
                if (_rawCharCount < MaxWordLength) _rawChars[_rawCharCount++] = c;
                return written;
            }
        }

        // Word breaks
        if (InputProcessor.IsLetterFast(c))
        {
            return ProcessSubsequentLetter(c, output, out backspaces);
        }

        CharType charType = InputProcessor.GetCharType(c, _options.Method);
        if (charType is CharType.WordBreak or CharType.Reset || InputProcessor.IsWordBreakFast(c))
        {
            return ProcessWordBreak(c, output, out backspaces);
        }

        // Non-letter key ends current word
        return ProcessWordBreak(c, output, out backspaces);
    }

    private int ProcessFirstLetter(char c, Span<char> output, out int backspaces)
    {
        if (_capitalizeNext && _options.AutoCapitalize && char.IsLetter(c))
        {
            c = char.ToUpperInvariant(c);
            _capitalizeNext = false;
        }

        _rawChars[0] = c;
        _rawCharCount = 1;
        backspaces = 0;
        bool isUpper = InputProcessor.IsAsciiUpper(c);
        char lower = InputProcessor.ToLowerAscii(c);

        if (_options.ConsonantShortcuts)
        {
            if (lower == 'f')
            {
                return RebuildShortcut(VnLexiName.p, VnLexiName.h, isUpper, output, out backspaces);
            }
            if (lower == 'j')
            {
                return RebuildShortcut(VnLexiName.g, VnLexiName.i, isUpper, output, out backspaces);
            }
            if (lower == 'w')
            {
                return RebuildShortcut(VnLexiName.q, VnLexiName.u, isUpper, output, out backspaces);
            }
        }

        VnLexiName lexi = CharsetTables.IsoToVnLexi(lower);
        bool isVowel = IsVowelLexi(lexi);

        ref var slot = ref _buffer[_bufferLength++];
        slot.RawChar = c;
        slot.Lexi = isUpper && lexi != VnLexiName.NonVnChar ? (VnLexiName)((int)lexi - 1) : lexi;
        slot.Tone = VnLexiTone.None;
        slot.Type = CharType.Letter;
        slot.IsUpper = isUpper;
        slot.Modified = false;

        _wordForm = isVowel ? WordForm.V : WordForm.C;
        _hasEndingConsonant = false;
        RecalculateVowelState();

        int written = EncodeSlot(ref slot, output);
        _displayLength = written;
        return written;
    }

    private int RebuildShortcut(VnLexiName first, VnLexiName second, bool isUpper, Span<char> output, out int backspaces)
    {
        ref var slot0 = ref _buffer[_bufferLength++];
        slot0.RawChar = isUpper ? char.ToUpperInvariant(CharsetTables.VnLexiToIso[(int)first]) : CharsetTables.VnLexiToIso[(int)first];
        slot0.Lexi = isUpper ? (VnLexiName)((int)first - 1) : first;
        slot0.Tone = VnLexiTone.None;
        slot0.Type = CharType.Letter;
        slot0.IsUpper = isUpper;
        slot0.Modified = false;

        ref var slot1 = ref _buffer[_bufferLength++];
        slot1.RawChar = CharsetTables.VnLexiToIso[(int)second];
        slot1.Lexi = second;
        slot1.Tone = VnLexiTone.None;
        slot1.Type = CharType.Letter;
        slot1.IsUpper = false;
        slot1.Modified = false;

        _wordForm = WordForm.C;
        _hasEndingConsonant = false;
        RecalculateVowelState();

        return RebuildCurrentWord(output, out backspaces);
    }

    private int ProcessLoneW(char c, Span<char> output, out int backspaces)
    {
        _rawChars[0] = c;
        _rawCharCount = 1;
        backspaces = 0;
        bool isUpper = InputProcessor.IsAsciiUpper(c);

        ref var slot = ref _buffer[_bufferLength++];
        slot.RawChar = c;
        slot.Lexi = isUpper ? VnLexiName.Uh : VnLexiName.uh;
        slot.Tone = VnLexiTone.None;
        slot.Type = CharType.Letter;
        slot.IsUpper = isUpper;
        slot.Modified = true;

        _wordForm = WordForm.V;
        _hasEndingConsonant = false;
        RecalculateVowelState();

        int written = EncodeSlot(ref slot, output);
        _displayLength = written;
        return written;
    }

    private int ProcessSubsequentLetter(char c, Span<char> output, out int backspaces)
    {
        backspaces = 0;
        if (_rawCharCount < MaxWordLength) _rawChars[_rawCharCount++] = c;

        if (_bufferLength >= MaxWordLength)
        {
            output[0] = c;
            _displayLength += 1;
            return 1;
        }

        int vowelStart = GetVowelStart();

        bool isUpper = InputProcessor.IsAsciiUpper(c);
        char lower = InputProcessor.ToLowerAscii(c);
        VnLexiName lexi = CharsetTables.IsoToVnLexi(lower);

        if (lexi == VnLexiName.NonVnChar)
        {
            return AppendNonVnLetter(c, output, out backspaces);
        }

        bool isVowel = IsVowelLexi(lexi) && _bufferLength >= vowelStart;

        if (isVowel && _vowelCount > 0 && _vowelStartIndex < vowelStart)
        {
            // "gi" became an initial consonant and a real nucleus vowel follows it.
            ClearVowels();
        }

        ref var slot = ref _buffer[_bufferLength++];
        slot.RawChar = c;
        slot.Lexi = isUpper ? (VnLexiName)((int)lexi - 1) : lexi;
        slot.Tone = VnLexiTone.None;
        slot.Type = CharType.Letter;
        slot.IsUpper = isUpper;
        slot.Modified = false;

        if (isVowel)
        {
            AddVowel(_bufferLength - 1, lexi);

            // "qu" alone: the u is part of the initial consonant, not a nucleus vowel.
            if (_bufferLength == 2 &&
                InputProcessor.ToLowerAscii(_buffer[0].RawChar) == 'q' &&
                InputProcessor.ToLowerAscii(_buffer[1].RawChar) == 'u')
            {
                ClearVowels();
            }
        }

        bool vowelAfterCoda = UpdateFormAfterAppend(isVowel);

        int upgradeFrom = -1;
        if (!isVowel && _wordForm != WordForm.NonVn && _vowelCount == 2)
        {
            upgradeFrom = TryUpgradeUoOnCoda();
        }

        ValidateIncremental(isVowel);

        // "cút" + "o": a vowel after a coda can never belong to the same syllable.
        // Restore the raw keystrokes immediately when a mark was already applied.
        if (vowelAfterCoda && _options.SpellCheck && _options.AutoNonVnRestore && HasAnyVietnameseMark())
        {
            return RestoreRawBufferWithNewChar(c, output, out backspaces);
        }

        // Auto fix Shift key sticking on 2nd letter (e.g. "VIệt" -> "Việt", "TRường" -> "Trường")
        if (_options.FixShiftLag && _bufferLength >= 3)
        {
            ref var slot0 = ref _buffer[0];
            ref var slot1 = ref _buffer[1];
            if (slot0.IsUpper && slot1.IsUpper && !isUpper)
            {
                slot1.IsUpper = false;
                slot1.RawChar = InputProcessor.ToLowerAscii(slot1.RawChar);
                slot1.Lexi = slot1.Lexi.VnToLower();
                RefreshTonePosition();
                return RebuildCurrentWord(output, out backspaces);
            }
        }

        if (upgradeFrom >= 0)
        {
            RefreshTonePosition();
            return RebuildFromIndex(output, out backspaces, upgradeFrom);
        }

        if (_currentTone != VnLexiTone.None && _wordForm != WordForm.NonVn)
        {
            if (RefreshTonePosition())
            {
                return RebuildFromIndex(output, out backspaces, _vowelStartIndex);
            }
        }

        int written = EncodeSlot(ref slot, output);
        _displayLength += written;
        return written;
    }

    /// <summary>
    /// Appends a raw key as a literal character without emitting output (used by tone undo).
    /// </summary>
    private void AppendLiteralKey(char c)
    {
        VnLexiName lexi = CharsetTables.IsoToVnLexi(InputProcessor.ToLowerAscii(c));
        bool isUpper = InputProcessor.IsAsciiUpper(c);
        bool isVowel = IsVowelLexi(lexi) && _bufferLength >= GetVowelStart();

        ref var slot = ref _buffer[_bufferLength++];
        slot.RawChar = c;
        slot.Lexi = lexi != VnLexiName.NonVnChar && isUpper ? (VnLexiName)((int)lexi - 1) : lexi;
        slot.Tone = VnLexiTone.None;
        slot.Type = CharType.Letter;
        slot.IsUpper = isUpper;
        slot.Modified = false;

        if (isVowel)
        {
            AddVowel(_bufferLength - 1, lexi);
        }

        UpdateFormAfterAppend(isVowel);
        ValidateIncremental(isVowel);
    }

    private int AppendNonVnLetter(char c, Span<char> output, out int backspaces)
    {
        backspaces = 0;
        if (_bufferLength >= MaxWordLength)
        {
            output[0] = c;
            _displayLength += 1;
            return 1;
        }

        ref var slot = ref _buffer[_bufferLength++];
        slot.RawChar = c;
        slot.Lexi = VnLexiName.NonVnChar;
        slot.Tone = VnLexiTone.None;
        slot.Type = CharType.Letter;
        slot.IsUpper = InputProcessor.IsAsciiUpper(c);
        slot.Modified = false;

        _wordForm = WordForm.NonVn;
        RecalculateVowelState();

        int written = EncodeSlot(ref slot, output);
        _displayLength += written;
        return written;
    }

    /// <summary>
    /// Appends a directly mapped Vietnamese character (Telex bracket shortcuts).
    /// </summary>
    private int AppendMappedLetter(char rawKey, VnLexiName lexi, bool isUpper, Span<char> output, out int backspaces)
    {
        if (_bufferLength == 0 && _capitalizeNext && _options.AutoCapitalize)
        {
            isUpper = true;
            _capitalizeNext = false;
        }

        backspaces = 0;
        if (_rawCharCount < MaxWordLength) _rawChars[_rawCharCount++] = rawKey;

        if (_bufferLength >= MaxWordLength)
        {
            output[0] = rawKey;
            _displayLength += 1;
            return 1;
        }

        ref var slot = ref _buffer[_bufferLength++];
        slot.RawChar = rawKey;
        slot.Lexi = lexi;
        slot.Tone = VnLexiTone.None;
        slot.Type = CharType.Letter;
        slot.IsUpper = isUpper;
        slot.Modified = true;

        if (_bufferLength == 1)
        {
            _wordForm = IsVowelLexi(lexi) ? WordForm.V : WordForm.C;
            _hasEndingConsonant = false;
            RecalculateVowelState();

            int single = EncodeSlot(ref slot, output);
            _displayLength = single;
            return single;
        }

        RecalculateVowelState();
        bool mappedIsVowel = IsVowelLexi(lexi);
        UpdateFormAfterAppend(mappedIsVowel);
        ValidateIncremental(mappedIsVowel);

        if (_currentTone != VnLexiTone.None && _wordForm != WordForm.NonVn)
        {
            if (RefreshTonePosition())
            {
                return RebuildFromIndex(output, out backspaces, _vowelStartIndex);
            }
        }

        int written = EncodeSlot(ref slot, output);
        _displayLength += written;
        return written;
    }

    private bool TryBracketShortcut(char c, Span<char> output, out int backspaces, out int written)
    {
        backspaces = 0;
        written = 0;

        switch (c)
        {
            case '[':
                written = AppendMappedLetter(c, VnLexiName.oh, false, output, out backspaces);
                return true;
            case '{':
                written = AppendMappedLetter(c, VnLexiName.Oh, true, output, out backspaces);
                return true;
            case ']':
                written = AppendMappedLetter(c, VnLexiName.uh, false, output, out backspaces);
                return true;
            case '}':
                written = AppendMappedLetter(c, VnLexiName.Uh, true, output, out backspaces);
                return true;
            default:
                return false;
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private bool IsDoubleKeyModify(char c)
    {
        if (_options.Method is not (InputMethod.Telex or InputMethod.SimpleTelex))
        {
            return false;
        }

        char lower = InputProcessor.ToLowerAscii(c);
        return lower is 'a' or 'e' or 'o' or 'd';
    }

    private bool TryApplyTone(
        ToneKeyType toneKey,
        char rawKey,
        Span<char> output,
        out int backspaces,
        out int written)
    {
        backspaces = 0;
        written = 0;

        int vowelCount = _vowelCount;
        if (vowelCount == 0 || _wordForm == WordForm.NonVn)
        {
            return false;
        }

        // Simple Telex: Words starting with 'w' are non-Vietnamese (e.g. water, win, wash, wasd)
        if (_options.Method == InputMethod.SimpleTelex && InputProcessor.ToLowerAscii(_buffer[0].RawChar) == 'w')
        {
            return false;
        }

        if (_options.SpellCheck && !_options.ConsonantShortcuts &&
            EnglishPhonetics.IsForeignInitial(_buffer[0].RawChar, _buffer[0].Lexi, _options.ForeignInitials))
        {
            return false;
        }

        int firstVowel = _vowelStartIndex;
        int lastVowel = firstVowel + vowelCount - 1;
        bool hasCoda = lastVowel < _bufferLength - 1;

        VnLexiTone newTone = InputProcessor.ToVnLexiTone(toneKey);

        // Tone removal key: z clears the mark (and is consumed even when no tone is present).
        if (newTone == VnLexiTone.None)
        {
            _currentTone = VnLexiTone.None;
            for (int i = 0; i < vowelCount; i++)
            {
                _buffer[firstVowel + i].Tone = VnLexiTone.None;
            }

            written = RebuildFromIndex(output, out backspaces, firstVowel);
            return true;
        }

        // Stop codas (c, ch, p, t) never take huyền/hỏi/ngã.
        if (hasCoda)
        {
            int codaId = GetCodaIdAt(lastVowel + 1, _bufferLength - 1 - lastVowel);
            if (IsStopCodaId(codaId) &&
                newTone is VnLexiTone.Grave or VnLexiTone.Hook or VnLexiTone.Tilde)
            {
                return false;
            }
        }

        Span<VnLexiName> vowelLexis = stackalloc VnLexiName[vowelCount];
        for (int i = 0; i < vowelCount; i++)
        {
            vowelLexis[i] = _buffer[firstVowel + i].Lexi;
        }

        // UniKey only enforces a complete vowel sequence when free marking is disabled.
        if (_options.SpellCheck && !_options.FreeMarking && !ToneRules.IsValidVowelSequence(vowelLexis))
        {
            return false;
        }

        // Check for tone undo
        if (_currentTone == newTone)
        {
            _currentTone = VnLexiTone.None;
            for (int i = 0; i < vowelCount; i++)
            {
                _buffer[firstVowel + i].Tone = VnLexiTone.None;
            }

            // The second key is a literal character now (UniKey's processAppend(ev)).
            // It must enter the buffer so the screen and the model stay in sync.
            if (_bufferLength < MaxWordLength)
            {
                AppendLiteralKey(rawKey);
                written = RebuildFromIndex(output, out backspaces, firstVowel);
            }
            else
            {
                written = RebuildFromIndex(output, out backspaces, firstVowel);
                if (output.Length > written)
                {
                    output[written++] = rawKey;
                    _displayLength++;
                }
            }

            return true;
        }

        _currentTone = newTone;

        int targetVowelPos = ToneRules.GetTonePosition(
            vowelLexis,
            hasCoda,
            _options.ModernStyle);

        for (int i = 0; i < vowelCount; i++)
        {
            _buffer[firstVowel + i].Tone = (i == targetVowelPos) ? _currentTone : VnLexiTone.None;
        }

        written = RebuildFromIndex(output, out backspaces, firstVowel);
        return true;
    }

    private bool TryApplyModify(
        ModifyKeyType modifyKey,
        char rawKey,
        Span<char> output,
        out int backspaces,
        out int written)
    {
        backspaces = 0;
        written = 0;

        char lower = InputProcessor.ToLowerAscii(rawKey);

        if (_options.SpellCheck && !_options.ConsonantShortcuts &&
            EnglishPhonetics.IsForeignInitial(_buffer[0].RawChar, _buffer[0].Lexi, _options.ForeignInitials))
        {
            return false;
        }

        // Double key handling in Telex (strictly adjacent characters, no consonants between)
        if (_options.Method is InputMethod.Telex or InputMethod.SimpleTelex && _wordForm != WordForm.NonVn)
        {
            if (lower == 'd')
            {
                ref var slot = ref _buffer[_bufferLength - 1];
                if (slot.Lexi is VnLexiName.d or VnLexiName.D or VnLexiName.dd or VnLexiName.DD)
                {
                    if (slot.Modified)
                    {
                        slot.Lexi = slot.IsUpper ? VnLexiName.D : VnLexiName.d;
                        slot.Modified = false;
                        written = RebuildCurrentWord(output, out backspaces);
                        if (output.Length > written)
                        {
                            output[written++] = rawKey;
                            _displayLength++;
                        }
                        return true;
                    }

                    slot.Lexi = slot.IsUpper ? VnLexiName.DD : VnLexiName.dd;
                    slot.Modified = true;
                    written = RebuildCurrentWord(output, out backspaces);
                    return true;
                }
            }

            if (lower is 'a' or 'e' or 'o')
            {
                ref var slot = ref _buffer[_bufferLength - 1];
                char slotChar = InputProcessor.ToLowerAscii(slot.RawChar);

                if (slotChar == lower)
                {
                    if (slot.Modified)
                    {
                        slot.Lexi = CharsetTables.IsoToVnLexi(slotChar);
                        slot.Modified = false;
                        RefreshVowelKey();
                        written = RebuildCurrentWord(output, out backspaces);
                        if (output.Length > written)
                        {
                            output[written++] = rawKey;
                            _displayLength++;
                        }
                        return true;
                    }

                    VnLexiName circumflex = lower switch
                    {
                        'a' => slot.IsUpper ? VnLexiName.Ar : VnLexiName.ar,
                        'e' => slot.IsUpper ? VnLexiName.Er : VnLexiName.er,
                        'o' => slot.IsUpper ? VnLexiName.Or : VnLexiName.or,
                        _ => slot.Lexi
                    };

                    slot.Lexi = circumflex;
                    slot.Modified = true;
                    RefreshVowelKey();
                    written = RebuildCurrentWord(output, out backspaces);
                    return true;
                }
            }
        }

        // 'w' key handling (Telex hook-all)
        if (modifyKey == ModifyKeyType.HookAll || (_options.Method is InputMethod.Telex or InputMethod.SimpleTelex && lower == 'w'))
        {
            if (_vowelCount > 0 && !_options.FreeMarking && !LastVowelIsCurrent())
            {
                return false;
            }

            if (ApplyHookAll(rawKey, output, out backspaces, out written))
            {
                return true;
            }

            // Lone 'w'
            if (_options.Method == InputMethod.Telex)
            {
                written = ProcessLoneW(rawKey, output, out backspaces);
                return true;
            }
        }

        // VNI & VIQR number/symbol modifications
        if (_options.Method is InputMethod.Vni or InputMethod.Viqr)
        {
            if (modifyKey == ModifyKeyType.Horn)
            {
                if (_vowelCount > 0 && !_options.FreeMarking && !LastVowelIsCurrent())
                {
                    return false;
                }

                return ApplyHookAll(rawKey, output, out backspaces, out written);
            }

            int vowelStart = GetVowelStart();

            switch (modifyKey)
            {
                case ModifyKeyType.Circumflex:
                    for (int i = _bufferLength - 1; i >= vowelStart; i--)
                    {
                        ref var slot = ref _buffer[i];
                        char c = InputProcessor.ToLowerAscii(slot.RawChar);
                        if (c is 'a' or 'e' or 'o')
                        {
                            if (!_options.FreeMarking && i != _bufferLength - 1)
                            {
                                return false;
                            }

                            slot.Lexi = c switch
                            {
                                'a' => slot.IsUpper ? VnLexiName.Ar : VnLexiName.ar,
                                'e' => slot.IsUpper ? VnLexiName.Er : VnLexiName.er,
                                'o' => slot.IsUpper ? VnLexiName.Or : VnLexiName.or,
                                _ => slot.Lexi
                            };
                            slot.Modified = true;
                            RefreshTonePosition();
                            written = RebuildCurrentWord(output, out backspaces);
                            return true;
                        }
                    }
                    break;

                case ModifyKeyType.Breve:
                    for (int i = _bufferLength - 1; i >= vowelStart; i--)
                    {
                        ref var slot = ref _buffer[i];
                        if (InputProcessor.ToLowerAscii(slot.RawChar) == 'a')
                        {
                            if (!_options.FreeMarking && i != _bufferLength - 1)
                            {
                                return false;
                            }

                            slot.Lexi = slot.IsUpper ? VnLexiName.Ah : VnLexiName.ah;
                            slot.Modified = true;
                            RefreshTonePosition();
                            written = RebuildCurrentWord(output, out backspaces);
                            return true;
                        }
                    }
                    break;

                case ModifyKeyType.DMark:
                    for (int i = _bufferLength - 1; i >= 0; i--)
                    {
                        ref var slot = ref _buffer[i];
                        if (InputProcessor.ToLowerAscii(slot.RawChar) == 'd')
                        {
                            if (!_options.FreeMarking && i != _bufferLength - 1)
                            {
                                return false;
                            }

                            slot.Lexi = slot.IsUpper ? VnLexiName.DD : VnLexiName.dd;
                            slot.Modified = true;
                            written = RebuildCurrentWord(output, out backspaces);
                            return true;
                        }
                    }
                    break;
            }
        }

        return false;
    }

    /// <summary>
    /// Hook/horn application shared by Telex 'w', VNI '7' and VIQR '+/*'.
    /// Handles u+o pairs, "th" + uo -> thuơ and double-key undo, mirroring UkEngine::processHookWithUO.
    /// </summary>
    private bool ApplyHookAll(char rawKey, Span<char> output, out int backspaces, out int written)
    {
        backspaces = 0;
        written = 0;

        int vowelStart = GetVowelStart();
        int uIdx = -1, oIdx = -1;
        for (int i = vowelStart; i < _bufferLength; i++)
        {
            char ch = InputProcessor.ToLowerAscii(_buffer[i].RawChar);
            if (ch == 'u')
            {
                uIdx = i;
                oIdx = -1;
            }
            else if (ch == 'o' && uIdx >= 0 && i == uIdx + 1)
            {
                oIdx = i;
            }
        }

        if (uIdx >= 0 && oIdx >= 0)
        {
            bool uHorned = _buffer[uIdx].Lexi is VnLexiName.uh or VnLexiName.Uh;
            bool oHorned = _buffer[oIdx].Lexi is VnLexiName.oh or VnLexiName.Oh;

            if (uHorned && oHorned)
            {
                // ươ + w -> uo (undo)
                _buffer[uIdx].Lexi = _buffer[uIdx].IsUpper ? VnLexiName.U : VnLexiName.u;
                _buffer[oIdx].Lexi = _buffer[oIdx].IsUpper ? VnLexiName.O : VnLexiName.o;
                _buffer[uIdx].Modified = false;
                _buffer[oIdx].Modified = false;
                RefreshTonePosition();
                written = RebuildCurrentWord(output, out backspaces);
                if (output.Length > written)
                {
                    output[written++] = rawKey;
                    _displayLength++;
                }
                return true;
            }

            if (oHorned)
            {
                // uơ + w -> uo (undo)
                _buffer[oIdx].Lexi = _buffer[oIdx].IsUpper ? VnLexiName.O : VnLexiName.o;
                _buffer[oIdx].Modified = false;
                RefreshTonePosition();
                written = RebuildCurrentWord(output, out backspaces);
                if (output.Length > written)
                {
                    output[written++] = rawKey;
                    _displayLength++;
                }
                return true;
            }

            if (uHorned)
            {
                // ưo + w -> ươ
                _buffer[oIdx].Lexi = _buffer[oIdx].IsUpper ? VnLexiName.Oh : VnLexiName.oh;
                _buffer[oIdx].Modified = true;
                RefreshTonePosition();
                written = RebuildCurrentWord(output, out backspaces);
                return true;
            }

            // "th" + uo -> thuơ (only o is horned); otherwise uo -> ươ
            bool thSpecial = oIdx == _bufferLength - 1 && uIdx == 2
                && InputProcessor.ToLowerAscii(_buffer[0].RawChar) == 't'
                && InputProcessor.ToLowerAscii(_buffer[1].RawChar) == 'h'
                && !_buffer[0].Modified && !_buffer[1].Modified;

            _buffer[uIdx].Lexi = _buffer[uIdx].IsUpper ? VnLexiName.Uh : VnLexiName.uh;
            _buffer[oIdx].Lexi = _buffer[oIdx].IsUpper ? VnLexiName.Oh : VnLexiName.oh;
            _buffer[uIdx].Modified = true;
            _buffer[oIdx].Modified = true;

            if (thSpecial)
            {
                _buffer[uIdx].Lexi = _buffer[uIdx].IsUpper ? VnLexiName.U : VnLexiName.u;
                _buffer[uIdx].Modified = false;
            }

            RefreshTonePosition();
            written = RebuildCurrentWord(output, out backspaces);
            return true;
        }

        // Check double 'w' undo: only on immediately preceding modified vowel
        ref var lastSlot = ref _buffer[_bufferLength - 1];
        if (lastSlot.Modified && lastSlot.Lexi is VnLexiName.ah or VnLexiName.Ah or VnLexiName.oh or VnLexiName.Oh or VnLexiName.uh or VnLexiName.Uh)
        {
            VnLexiName plain = CharsetTables.IsoToVnLexi(InputProcessor.ToLowerAscii(lastSlot.RawChar));
            lastSlot.Lexi = InputProcessor.IsAsciiUpper(lastSlot.RawChar) ? plain.VnToUpper() : plain;
            lastSlot.Modified = false;
            RefreshTonePosition();
            written = RebuildCurrentWord(output, out backspaces);
            if (output.Length > written)
            {
                output[written++] = rawKey;
                _displayLength++;
            }
            return true;
        }

        // Modify the nearest preceding a, o or u inside the vowel nucleus
        for (int i = _bufferLength - 1; i >= vowelStart; i--)
        {
            ref var slot = ref _buffer[i];
            char c = InputProcessor.ToLowerAscii(slot.RawChar);

            if (c == 'a')
            {
                slot.Lexi = slot.IsUpper ? VnLexiName.Ah : VnLexiName.ah;
            }
            else if (c == 'o')
            {
                slot.Lexi = slot.IsUpper ? VnLexiName.Oh : VnLexiName.oh;
            }
            else if (c == 'u')
            {
                slot.Lexi = slot.IsUpper ? VnLexiName.Uh : VnLexiName.uh;
            }
            else
            {
                continue;
            }

            slot.Modified = true;
            RefreshTonePosition();
            written = RebuildCurrentWord(output, out backspaces);
            return true;
        }

        return false;
    }

    private int ProcessWordBreak(char c, Span<char> output, out int backspaces)
    {
        backspaces = 0;
        int written = 0;

        if (_options.AutoCapitalize && c is '.' or '!' or '?' or '\r' or '\n')
        {
            _capitalizeNext = true;
        }

        bool macroActive = _bufferLength > 0 && _options.EnableMacro && _macros.HasEntries;
        int wordStart = -1;
        int wordLength = 0;

        if (macroActive)
        {
            Span<char> word = stackalloc char[MaxMacroChars + 1];
            wordLength = RenderWord(word);

            if (wordLength <= MaxMacroChars)
            {
                EnsureMacroCapacity(wordLength + 1);
                wordStart = _macroBufferLength;
                word[..wordLength].CopyTo(_macroBuffer.AsSpan(wordStart));
                _macroBufferLength += wordLength;

                if (TryMatchMacro(wordStart, c, output, out backspaces, out written))
                {
                    ClearMacroHistory();
                    ResetWord();
                    return written;
                }
            }
            else
            {
                wordStart = -1;
            }
        }

        if (_bufferLength > 0 && _options.SpellCheck && _options.AutoNonVnRestore &&
            HasAnyVietnameseMark() && !IsValidVietnameseWord())
        {
            // Keep the unchanged ASCII prefix on screen; only rewrite the suffix.
            int matchLen = 0;
            while (matchLen < _bufferLength && matchLen < _rawCharCount &&
                   InputProcessor.ToLowerAscii(_buffer[matchLen].RawChar) == InputProcessor.ToLowerAscii(_rawChars[matchLen]) &&
                   _buffer[matchLen].Tone == VnLexiTone.None &&
                   !_buffer[matchLen].Modified)
            {
                matchLen++;
            }

            int prefixLength = MeasureRange(0, matchLen);
            backspaces = _displayLength - prefixLength;

            int count = _rawCharCount > 0 ? _rawCharCount : _bufferLength;
            for (int i = matchLen; i < count; i++)
            {
                output[written++] = _rawChars[i];
            }

            if (wordStart >= 0)
            {
                // The displayed text is the raw keystroke sequence now.
                _macroBufferLength = wordStart;
                int copy = Math.Min(count, _macroBuffer.Length - wordStart);
                Array.Copy(_rawChars, 0, _macroBuffer, wordStart, copy);
                _macroBufferLength += copy;
                wordLength = copy;
            }

            FinishMacroToken(ref wordStart, c);

            ResetWord();
            if (output.Length > written)
            {
                output[written++] = c;
            }
            return written;
        }

        FinishMacroToken(ref wordStart, c);

        ResetWord();
        output[0] = c;
        return 1;
    }

    private void FinishMacroToken(ref int wordStart, char separator)
    {
        if (wordStart < 0)
        {
            return;
        }

        if (_macroBufferLength < _macroBuffer.Length)
        {
            _macroBuffer[_macroBufferLength++] = separator;
        }

        while (_macroTokenCount >= MaxMacroTokens)
        {
            wordStart -= DropOldestMacroToken();
        }

        _macroTokenStarts[_macroTokenCount++] = wordStart;

        // Keep the search window bounded.
        while (_macroTokenCount > 1 && _macroBufferLength > MaxMacroChars)
        {
            DropOldestMacroToken();
        }
    }

    private bool TryMatchMacro(int wordStart, char c, Span<char> output, out int backspaces, out int written)
    {
        backspaces = 0;
        written = 0;

        int end = _macroBufferLength;

        // Current word first (shortest), then extend backwards word by word.
        if (_macros.TryGet(_macroBuffer.AsSpan(wordStart, end - wordStart), out string? replacement))
        {
            backspaces = end - wordStart;
            return WriteMacro(replacement, c, output, out written);
        }

        for (int i = _macroTokenCount - 1; i >= 0; i--)
        {
            int start = _macroTokenStarts[i];
            int length = end - start;
            if (length > MaxMacroChars)
            {
                break;
            }

            if (_macros.TryGet(_macroBuffer.AsSpan(start, length), out replacement))
            {
                backspaces = length;
                return WriteMacro(replacement, c, output, out written);
            }
        }

        return false;
    }

    private bool WriteMacro(string replacement, char c, Span<char> output, out int written)
    {
        int copyLength = Math.Min(replacement.Length, output.Length);
        replacement.AsSpan(0, copyLength).CopyTo(output);
        written = copyLength;
        if (output.Length > written)
        {
            output[written++] = c;
        }
        return true;
    }

    private void EnsureMacroCapacity(int extra)
    {
        while (_macroBufferLength + extra > MacroBufferSize && _macroTokenCount > 0)
        {
            DropOldestMacroToken();
        }

        if (_macroBufferLength + extra > MacroBufferSize)
        {
            ClearMacroHistory();
        }
    }

    private int DropOldestMacroToken()
    {
        if (_macroTokenCount == 0)
        {
            return 0;
        }

        int cut = _macroTokenCount > 1 ? _macroTokenStarts[1] : _macroBufferLength;
        if (cut <= 0 || cut > _macroBufferLength)
        {
            cut = _macroBufferLength;
        }

        Array.Copy(_macroBuffer, cut, _macroBuffer, 0, _macroBufferLength - cut);
        _macroBufferLength -= cut;

        for (int i = 1; i < _macroTokenCount; i++)
        {
            _macroTokenStarts[i - 1] = _macroTokenStarts[i] - cut;
        }

        _macroTokenCount--;
        return cut;
    }

    private int RenderWord(Span<char> destination)
    {
        return EncodeRange(0, destination);
    }

    private int RestoreRawBufferWithNewChar(char c, Span<char> output, out int backspaces)
    {
        // The newly appended slot has not been written to the screen yet.
        int previousDisplayLength = _displayLength;

        int matchLen = 0;
        while (matchLen < _bufferLength - 1 && matchLen < _rawCharCount &&
               InputProcessor.ToLowerAscii(_buffer[matchLen].RawChar) == InputProcessor.ToLowerAscii(_rawChars[matchLen]) &&
               _buffer[matchLen].Tone == VnLexiTone.None &&
               !_buffer[matchLen].Modified)
        {
            matchLen++;
        }

        int prefixLength = MeasureRange(0, matchLen);
        backspaces = previousDisplayLength - prefixLength;

        int written = 0;
        for (int i = matchLen; i < _rawCharCount; i++)
        {
            output[written++] = _rawChars[i];
        }

        // Reconstruct the buffer from the raw keys in NonVn mode.
        _bufferLength = 0;
        for (int i = 0; i < _rawCharCount; i++)
        {
            ref var slot = ref _buffer[_bufferLength++];
            slot.RawChar = _rawChars[i];
            slot.Lexi = CharsetTables.IsoToVnLexi(InputProcessor.ToLowerAscii(_rawChars[i]));
            slot.Tone = VnLexiTone.None;
            slot.Type = CharType.Letter;
            slot.IsUpper = InputProcessor.IsAsciiUpper(_rawChars[i]);
            slot.Modified = false;
        }

        _currentTone = VnLexiTone.None;
        _wordForm = WordForm.NonVn;
        _hasEndingConsonant = false;
        _displayLength = prefixLength + written;
        ClearVowels();

        return written;
    }

    private bool HasAnyVietnameseMark()
    {
        for (int i = 0; i < _bufferLength; i++)
        {
            if (_buffer[i].Tone != VnLexiTone.None || _buffer[i].Modified)
            {
                return true;
            }
        }
        return false;
    }

    private int ProcessBackspace(Span<char> output, out int backspaces)
    {
        if (_rawCharCount > 0) _rawCharCount--;

        if (_bufferLength == 0)
        {
            backspaces = 1;
            return 0;
        }

        int wordLength = MeasureRange(0, _bufferLength);

        // A raw key was echoed without being buffered (tone undo). Delete it first.
        if (_displayLength > wordLength)
        {
            backspaces = _displayLength - wordLength;
            _displayLength = wordLength;
            return 0;
        }

        int previousDisplayLength = _displayLength;
        bool removedTone = _buffer[_bufferLength - 1].Tone != VnLexiTone.None;
        _bufferLength--;

        if (_bufferLength == 0)
        {
            ResetWord();
            backspaces = previousDisplayLength;
            return 0;
        }

        RecalculateWordForm();

        if (removedTone)
        {
            // The tone mark lived on the deleted character: drop it instead of moving it.
            _currentTone = VnLexiTone.None;
        }
        else
        {
            RefreshTonePosition();
        }

        backspaces = previousDisplayLength;
        int written = EncodeRange(0, output);
        _displayLength = written;
        return written;
    }

    private int RebuildCurrentWord(Span<char> output, out int backspaces)
    {
        backspaces = _displayLength;
        int written = EncodeRange(0, output);
        _displayLength = written;
        return written;
    }

    private int RebuildFromIndex(Span<char> output, out int backspaces, int fromIndex)
    {
        fromIndex = Math.Clamp(fromIndex, 0, _bufferLength);
        int prefixLength = MeasureRange(0, fromIndex);
        backspaces = Math.Max(0, _displayLength - prefixLength);
        int written = EncodeRange(fromIndex, output);
        _displayLength = prefixLength + written;
        return written;
    }

    private int EncodeRange(int from, Span<char> output)
    {
        int written = 0;
        for (int i = from; i < _bufferLength; i++)
        {
            written += EncodeSlot(ref _buffer[i], output[written..]);
        }
        return written;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private int EncodeSlot(ref WordInfo slot, Span<char> destination)
    {
        if (destination.IsEmpty)
        {
            return 0;
        }

        if ((uint)(int)slot.Lexi >= (uint)CharsetTables.TotalVnChars)
        {
            destination[0] = slot.RawChar;
            return 1;
        }

        VnLexiCode code = VnLexi.ToVnLexiCode(slot.Lexi, slot.Tone, slot.IsUpper);
        return _charset.Encode(code, destination);
    }

    private int MeasureRange(int from, int toExclusive)
    {
        if (_singleCharCharset)
        {
            return toExclusive - from;
        }

        int length = 0;
        Span<char> scratch = stackalloc char[4];
        for (int i = from; i < toExclusive; i++)
        {
            length += EncodeSlot(ref _buffer[i], scratch);
        }
        return length;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static bool IsVowelLexi(VnLexiName lexi)
    {
        return (uint)(int)lexi < (uint)CharsetTables.TotalVnChars && CharsetTables.IsVnVowel[(int)lexi];
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private int GetVowelStart()
    {
        if (_bufferLength >= 2)
        {
            char c0 = InputProcessor.ToLowerAscii(_buffer[0].RawChar);
            char c1 = InputProcessor.ToLowerAscii(_buffer[1].RawChar);
            if ((c0 == 'q' && c1 == 'u') || (c0 == 'g' && c1 == 'i'))
            {
                return 2;
            }
        }
        return 0;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private void ClearVowels()
    {
        _vowelStartIndex = -1;
        _vowelCount = 0;
        _fastVowelKey = ToneRules.EmptyVowelKey;
    }

    /// <summary>
    /// Rebuilds the packed key from the actual slot symbols (used after a mark changes a vowel).
    /// </summary>
    private void RefreshVowelKey()
    {
        int key = ToneRules.EmptyVowelKey;
        for (int i = 0; i < _vowelCount; i++)
        {
            key = ToneRules.PackVowelSlot(key, i, ToneRules.GetVowelSlot(_buffer[_vowelStartIndex + i].Lexi));
        }
        _fastVowelKey = key;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private void AddVowel(int index, VnLexiName lexi)
    {
        if (_vowelCount == 0)
        {
            _vowelStartIndex = index;
        }

        if (_vowelCount < 3)
        {
            _fastVowelKey = ToneRules.PackVowelSlot(_fastVowelKey, _vowelCount, ToneRules.GetVowelSlot(lexi));
        }

        _vowelCount++;
    }

    private void RecalculateVowelState()
    {
        ClearVowels();

        int start = GetVowelStart();
        for (int i = start; i < _bufferLength; i++)
        {
            if (IsVowelLexi(_buffer[i].Lexi))
            {
                AddVowel(i, _buffer[i].Lexi);
            }
        }

        // "gi" alone (gì, gỉ, gĩ, gị, gìn): the i is the nucleus.
        if (_vowelCount == 0 && start == 2 && _bufferLength >= 2 &&
            InputProcessor.ToLowerAscii(_buffer[0].RawChar) == 'g' &&
            InputProcessor.ToLowerAscii(_buffer[1].RawChar) == 'i')
        {
            AddVowel(1, _buffer[1].Lexi);
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private bool LastVowelIsCurrent()
    {
        return _vowelCount > 0 && _vowelStartIndex + _vowelCount == _bufferLength;
    }

    /// <summary>
    /// Updates the coarse syllable form after a character was appended.
    /// Returns true when a vowel follows a coda (which can never be the same syllable).
    /// </summary>
    private bool UpdateFormAfterAppend(bool isVowel)
    {
        bool vowelAfterCoda = false;

        if (_wordForm != WordForm.NonVn)
        {
            if (isVowel)
            {
                if (_hasEndingConsonant || _wordForm is WordForm.CVC or WordForm.VC)
                {
                    _wordForm = WordForm.NonVn;
                    vowelAfterCoda = true;
                }
                else
                {
                    _wordForm = _wordForm switch
                    {
                        WordForm.C => WordForm.CV,
                        WordForm.V => WordForm.V,
                        WordForm.CV => WordForm.CV,
                        _ => WordForm.V
                    };
                }
            }
            else
            {
                _hasEndingConsonant = _wordForm is WordForm.CV or WordForm.CVC or WordForm.VC;
                _wordForm = _wordForm switch
                {
                    WordForm.CV => WordForm.CVC,
                    WordForm.V => WordForm.VC,
                    _ => WordForm.C
                };
            }
        }

        return vowelAfterCoda;
    }

    /// <summary>
    /// UniKey converts uơ/ưo to ươ as soon as a coda consonant is appended.
    /// </summary>
    private int TryUpgradeUoOnCoda()
    {
        if (_vowelCount != 2 || _vowelStartIndex + 2 != _bufferLength - 1)
        {
            return -1;
        }

        VnLexiName v0 = _buffer[_vowelStartIndex].Lexi;
        VnLexiName v1 = _buffer[_vowelStartIndex + 1].Lexi;

        bool uThenO = (v0 is VnLexiName.Uh or VnLexiName.uh) && (v1 is VnLexiName.O or VnLexiName.o);
        bool oThenU = (v0 is VnLexiName.U or VnLexiName.u) && (v1 is VnLexiName.Oh or VnLexiName.oh);

        if (!uThenO && !oThenU)
        {
            return -1;
        }

        _buffer[_vowelStartIndex].Lexi = _buffer[_vowelStartIndex].IsUpper ? VnLexiName.Uh : VnLexiName.uh;
        _buffer[_vowelStartIndex + 1].Lexi = _buffer[_vowelStartIndex + 1].IsUpper ? VnLexiName.Oh : VnLexiName.oh;
        _buffer[_vowelStartIndex].Modified = true;
        _buffer[_vowelStartIndex + 1].Modified = true;

        // Update the packed key (uơ/ưo -> ươ).
        RefreshVowelKey();

        return _vowelStartIndex;
    }

    /// <summary>
    /// Recomputes and re-places the active tone mark over the current vowel nucleus.
    /// Returns true when any slot changed.
    /// </summary>
    private bool RefreshTonePosition()
    {
        RefreshVowelKey();

        if (_currentTone == VnLexiTone.None)
        {
            return false;
        }

        int count = _vowelCount;
        if (count == 0)
        {
            _currentTone = VnLexiTone.None;
            return true;
        }

        Span<VnLexiName> vowels = stackalloc VnLexiName[count];
        for (int i = 0; i < count; i++)
        {
            vowels[i] = _buffer[_vowelStartIndex + i].Lexi;
        }

        bool hasCoda = _vowelStartIndex + count < _bufferLength;
        int target = ToneRules.GetTonePosition(vowels, hasCoda, _options.ModernStyle);

        bool changed = false;
        for (int i = 0; i < count; i++)
        {
            VnLexiTone expected = i == target ? _currentTone : VnLexiTone.None;
            if (_buffer[_vowelStartIndex + i].Tone != expected)
            {
                _buffer[_vowelStartIndex + i].Tone = expected;
                changed = true;
            }
        }

        return changed;
    }

    /// <summary>
    /// Cheap incremental validation used on the typing hot path (no buffer scans).
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private void ValidateIncremental(bool isVowel)
    {
        if (_wordForm == WordForm.NonVn)
        {
            return;
        }

        if (isVowel)
        {
            if (_vowelCount == 0)
            {
                // "qu" prefix: the u belongs to the initial consonant, nothing to validate yet.
                return;
            }

            if (_vowelCount == 1 && _vowelStartIndex > 0 && !IsInitialValid())
            {
                _wordForm = WordForm.NonVn;
                return;
            }

            if (!ToneRules.IsValidVowelSlots(_fastVowelKey, _vowelCount))
            {
                _wordForm = WordForm.NonVn;
            }
        }
        else if (_vowelCount > 0)
        {
            int codaId = GetCodaIdAt(_vowelStartIndex + _vowelCount, _bufferLength - _vowelStartIndex - _vowelCount);
            if (!ToneRules.IsValidVowelCodaSlots(_fastVowelKey, _vowelCount, codaId))
            {
                _wordForm = WordForm.NonVn;
            }
        }
    }

    private bool IsInitialValid()
    {
        Span<char> initial = stackalloc char[8];
        int initialLength = BuildInitial(_vowelStartIndex, initial);
        return ToneRules.IsValidInitialWithVowel(initial[..initialLength], _buffer[_vowelStartIndex].Lexi);
    }

    private void ValidateWordStructure()
    {
        if (_wordForm == WordForm.NonVn || _bufferLength == 0)
        {
            return;
        }

        if (_vowelCount == 0)
        {
            _hasEndingConsonant = false;
            return;
        }

        _hasEndingConsonant = _vowelStartIndex + _vowelCount < _bufferLength;

        if (_vowelCount == 1 && _vowelStartIndex > 0)
        {
            Span<char> initial = stackalloc char[8];
            int initialLength = BuildInitial(_vowelStartIndex, initial);
            if (!ToneRules.IsValidInitialWithVowel(initial[..initialLength], _buffer[_vowelStartIndex].Lexi))
            {
                _wordForm = WordForm.NonVn;
                return;
            }
        }

        if (!ToneRules.IsValidVowelSlots(_fastVowelKey, _vowelCount))
        {
            _wordForm = WordForm.NonVn;
            return;
        }

        if (_hasEndingConsonant)
        {
            int codaId = GetCodaIdAt(_vowelStartIndex + _vowelCount, _bufferLength - _vowelStartIndex - _vowelCount);
            if (!ToneRules.IsValidVowelCodaSlots(_fastVowelKey, _vowelCount, codaId))
            {
                _wordForm = WordForm.NonVn;
            }
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private int GetCodaIdAt(int start, int length)
    {
        char c0 = InputProcessor.ToLowerAscii(_buffer[start].RawChar);
        char c1 = length > 1 ? InputProcessor.ToLowerAscii(_buffer[start + 1].RawChar) : '\0';
        return ToneRules.GetCodaId(length, c0, c1);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static bool IsStopCodaId(int codaId)
    {
        return codaId is 0 or 3 or 4 or 5; // c, p, t, ch
    }

    private int BuildInitial(int endExclusive, Span<char> destination)
    {
        int count = 0;
        for (int i = 0; i < endExclusive && count < destination.Length; i++)
        {
            VnLexiName lexi = _buffer[i].Lexi;
            if (lexi is VnLexiName.dd or VnLexiName.DD)
            {
                destination[count++] = 'd';
                if (count < destination.Length)
                {
                    destination[count++] = 'd';
                }
            }
            else
            {
                destination[count++] = InputProcessor.ToLowerAscii(_buffer[i].RawChar);
            }
        }
        return count;
    }

    private void RecalculateWordForm()
    {
        RecalculateVowelState();

        if (_bufferLength == 0)
        {
            _wordForm = WordForm.Empty;
            _hasEndingConsonant = false;
            return;
        }

        _wordForm = WordForm.Empty;

        if (_vowelCount == 0)
        {
            _wordForm = WordForm.C;
            _hasEndingConsonant = false;
            ValidateWordStructure();
            return;
        }

        bool startsAtNucleus = _vowelStartIndex == GetVowelStart();
        bool endsAtEnd = _vowelStartIndex + _vowelCount == _bufferLength;

        _wordForm = startsAtNucleus
            ? (endsAtEnd ? WordForm.V : WordForm.VC)
            : (endsAtEnd ? WordForm.CV : WordForm.CVC);

        _hasEndingConsonant = !endsAtEnd;
        ValidateWordStructure();
    }

    private VnLexiTone GetWordTone()
    {
        for (int i = 0; i < _bufferLength; i++)
        {
            if (_buffer[i].Tone != VnLexiTone.None)
            {
                return _buffer[i].Tone;
            }
        }
        return VnLexiTone.None;
    }

    public bool IsValidVietnameseWord()
    {
        if (_bufferLength == 0) return true;
        if (_wordForm == WordForm.NonVn) return false;

        if (!_options.ConsonantShortcuts && EnglishPhonetics.IsForeignInitial(_buffer[0].RawChar, _buffer[0].Lexi, _options.ForeignInitials))
        {
            return false;
        }

        if (_rawCharCount > 0)
        {
            Span<char> physicalChars = stackalloc char[_rawCharCount];
            for (int i = 0; i < _rawCharCount; i++)
            {
                physicalChars[i] = InputProcessor.ToLowerAscii(_rawChars[i]);
            }

            if (EnglishPhonetics.IsEnglishKeyword(physicalChars))
            {
                return false;
            }
        }

        Span<char> rawChars = stackalloc char[_bufferLength];
        for (int i = 0; i < _bufferLength; i++)
        {
            rawChars[i] = InputProcessor.ToLowerAscii(_buffer[i].RawChar);
        }

        if (EnglishPhonetics.IsEnglishKeyword(rawChars))
        {
            return false;
        }

        if (_vowelCount == 0) return true;

        if (_vowelCount == 1 && _vowelStartIndex > 0)
        {
            Span<char> initial = stackalloc char[8];
            int initialLength = BuildInitial(_vowelStartIndex, initial);
            if (!ToneRules.IsValidInitialWithVowel(initial[..initialLength], _buffer[_vowelStartIndex].Lexi))
            {
                return false;
            }
        }

        if (!ToneRules.IsValidVowelSlots(_fastVowelKey, _vowelCount))
        {
            return false;
        }

        if (_vowelStartIndex + _vowelCount < _bufferLength)
        {
            int codaId = GetCodaIdAt(_vowelStartIndex + _vowelCount, _bufferLength - _vowelStartIndex - _vowelCount);

            if (!ToneRules.IsValidVowelCodaSlots(_fastVowelKey, _vowelCount, codaId))
            {
                return false;
            }

            if (IsStopCodaId(codaId) &&
                GetWordTone() is VnLexiTone.Grave or VnLexiTone.Hook or VnLexiTone.Tilde)
            {
                return false;
            }
        }

        return true;
    }
}

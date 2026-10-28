using System.Collections.Frozen;
using System.Runtime.CompilerServices;
using VietFlow.Tables;

namespace VietFlow.Engine;

public readonly record struct VowelSeq(VnLexiName[] Vowels, int ClassicToneIndex, int ModernToneIndex);

public readonly record struct ConSeq(string Key, bool CanBeInitial, bool CanBeEnding);

public static class ToneRules
{
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static VnLexiName NormalizeVowel(VnLexiName v)
    {
        return v switch
        {
            VnLexiName.a or VnLexiName.A => VnLexiName.A,
            VnLexiName.ar or VnLexiName.Ar => VnLexiName.Ar,
            VnLexiName.ah or VnLexiName.Ah or VnLexiName.ab or VnLexiName.Ab => VnLexiName.Ah,
            VnLexiName.e or VnLexiName.E => VnLexiName.E,
            VnLexiName.er or VnLexiName.Er => VnLexiName.Er,
            VnLexiName.i or VnLexiName.I => VnLexiName.I,
            VnLexiName.o or VnLexiName.O => VnLexiName.O,
            VnLexiName.or or VnLexiName.Or => VnLexiName.Or,
            VnLexiName.oh or VnLexiName.Oh => VnLexiName.Oh,
            VnLexiName.u or VnLexiName.U => VnLexiName.U,
            VnLexiName.uh or VnLexiName.Uh => VnLexiName.Uh,
            VnLexiName.y or VnLexiName.Y => VnLexiName.Y,
            _ => (int)v >= 0 && (int)v < CharsetTables.TotalVnChars
                ? (VnLexiName)CharsetTables.StdVnRootChar[(int)v]
                : v
        };
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int PackKey(VnLexiName v0, VnLexiName v1 = VnLexiName.None, VnLexiName v2 = VnLexiName.None)
    {
        VnLexiName n0 = NormalizeVowel(v0);
        int k1 = v1 == VnLexiName.None ? 0 : (int)NormalizeVowel(v1) + 1;
        int k2 = v2 == VnLexiName.None ? 0 : (int)NormalizeVowel(v2) + 1;
        return ((int)n0 + 1) | (k1 << 8) | (k2 << 16);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int PackKey(ReadOnlySpan<VnLexiName> vowels)
    {
        return vowels.Length switch
        {
            1 => PackKey(vowels[0]),
            2 => PackKey(vowels[0], vowels[1]),
            3 => PackKey(vowels[0], vowels[1], vowels[2]),
            _ => 0
        };
    }

    private static readonly FrozenDictionary<int, VowelSeq> VowelSeqMap;
    private static readonly FrozenDictionary<string, ConSeq> ConsonantMap;
    private static readonly FrozenSet<string> ValidEndingConsonants;
    private static readonly FrozenSet<int> VowelCodaPairs;

    // O(1) lookup tables for the incremental engine path (no hashing per keystroke).
    private const int VowelSlots = 16;
    private const int NoVowelSlot = 12;
    public const int EmptyVowelKey = NoVowelSlot | (NoVowelSlot << 4) | (NoVowelSlot << 8);
    private static readonly bool[] VowelFastTable = new bool[VowelSlots * VowelSlots * VowelSlots];
    private static readonly bool[] CodaFastTable = new bool[VowelSlots * VowelSlots * VowelSlots * 8];
    private static readonly byte[] VowelSlotLookup = new byte[CharsetTables.TotalVnChars];
    private static FrozenDictionary<string, ConSeq>.AlternateLookup<ReadOnlySpan<char>> ConsonantSpanLookup;
    private static FrozenSet<string>.AlternateLookup<ReadOnlySpan<char>> EndingSpanLookup;

    static ToneRules()
    {
        var vowelEntries = new List<KeyValuePair<int, VowelSeq>>
        {
            // Single vowels
            CreateVowelEntry(0, 0, VnLexiName.A),
            CreateVowelEntry(0, 0, VnLexiName.Ar),
            CreateVowelEntry(0, 0, VnLexiName.Ah),
            CreateVowelEntry(0, 0, VnLexiName.E),
            CreateVowelEntry(0, 0, VnLexiName.Er),
            CreateVowelEntry(0, 0, VnLexiName.I),
            CreateVowelEntry(0, 0, VnLexiName.O),
            CreateVowelEntry(0, 0, VnLexiName.Or),
            CreateVowelEntry(0, 0, VnLexiName.Oh),
            CreateVowelEntry(0, 0, VnLexiName.U),
            CreateVowelEntry(0, 0, VnLexiName.Uh),
            CreateVowelEntry(0, 0, VnLexiName.Y),

            // Two vowels
            // AI, AO, AU, AY, EO
            CreateVowelEntry(0, 0, VnLexiName.A, VnLexiName.I),
            CreateVowelEntry(0, 0, VnLexiName.A, VnLexiName.O),
            CreateVowelEntry(0, 0, VnLexiName.A, VnLexiName.U),
            CreateVowelEntry(0, 0, VnLexiName.A, VnLexiName.Y),
            CreateVowelEntry(0, 0, VnLexiName.E, VnLexiName.O),

            // ArU, ArY
            CreateVowelEntry(0, 0, VnLexiName.Ar, VnLexiName.U),
            CreateVowelEntry(0, 0, VnLexiName.Ar, VnLexiName.Y),

            // AhU, AhY
            CreateVowelEntry(0, 0, VnLexiName.Ah, VnLexiName.U),
            CreateVowelEntry(0, 0, VnLexiName.Ah, VnLexiName.Y),

            // EU, ErU, ErI
            CreateVowelEntry(0, 0, VnLexiName.E, VnLexiName.U),
            CreateVowelEntry(0, 0, VnLexiName.Er, VnLexiName.U),
            CreateVowelEntry(0, 0, VnLexiName.Er, VnLexiName.I),

            // IA, IE, IEr
            CreateVowelEntry(0, 0, VnLexiName.I, VnLexiName.A),
            CreateVowelEntry(1, 1, VnLexiName.I, VnLexiName.E),
            CreateVowelEntry(1, 1, VnLexiName.I, VnLexiName.Er),

            // IU
            CreateVowelEntry(0, 0, VnLexiName.I, VnLexiName.U),

            // OA, OE, OI
            CreateVowelEntry(0, 1, VnLexiName.O, VnLexiName.A),
            CreateVowelEntry(0, 1, VnLexiName.O, VnLexiName.E),
            CreateVowelEntry(0, 0, VnLexiName.O, VnLexiName.I),

            // OrI, OhI, OhU
            CreateVowelEntry(0, 0, VnLexiName.Or, VnLexiName.I),
            CreateVowelEntry(0, 0, VnLexiName.Oh, VnLexiName.I),
            CreateVowelEntry(0, 0, VnLexiName.Oh, VnLexiName.U),

            // UA, UE, UEr, UI, UY
            CreateVowelEntry(0, 0, VnLexiName.U, VnLexiName.A),
            CreateVowelEntry(0, 1, VnLexiName.U, VnLexiName.E),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Er),
            CreateVowelEntry(0, 0, VnLexiName.U, VnLexiName.I),
            CreateVowelEntry(0, 1, VnLexiName.U, VnLexiName.Y),

            // UO, UOr, UOh
            CreateVowelEntry(0, 1, VnLexiName.U, VnLexiName.O),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Or),
            CreateVowelEntry(1, 1, VnLexiName.Uh, VnLexiName.Oh),
            CreateVowelEntry(0, 0, VnLexiName.Uh, VnLexiName.A),
            CreateVowelEntry(0, 0, VnLexiName.Uh, VnLexiName.I),
            CreateVowelEntry(0, 0, VnLexiName.Uh, VnLexiName.U),

            // uâ (uar), oă (oab) — missing from the original port: tuần, quân, xuân, luật, xuất, xoắn
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Ar),
            CreateVowelEntry(1, 1, VnLexiName.O, VnLexiName.Ah),

            // ưo (uho), uơ (uoh) — quơ, thuở, huơ
            CreateVowelEntry(0, 0, VnLexiName.Uh, VnLexiName.O),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Oh),

            // uu
            CreateVowelEntry(0, 1, VnLexiName.U, VnLexiName.U),

            // YE, YEr
            CreateVowelEntry(1, 1, VnLexiName.Y, VnLexiName.E),
            CreateVowelEntry(1, 1, VnLexiName.Y, VnLexiName.Er),

            // Three vowels
            // IEU, IErU
            CreateVowelEntry(1, 1, VnLexiName.I, VnLexiName.E, VnLexiName.U),
            CreateVowelEntry(1, 1, VnLexiName.I, VnLexiName.Er, VnLexiName.U),

            // OAI, OAY, OAO, OEO
            CreateVowelEntry(1, 1, VnLexiName.O, VnLexiName.A, VnLexiName.I),
            CreateVowelEntry(1, 1, VnLexiName.O, VnLexiName.A, VnLexiName.Y),
            CreateVowelEntry(1, 1, VnLexiName.O, VnLexiName.Ar, VnLexiName.Y),
            CreateVowelEntry(1, 1, VnLexiName.O, VnLexiName.A, VnLexiName.O),
            CreateVowelEntry(1, 1, VnLexiName.O, VnLexiName.E, VnLexiName.O),

            // UAO, UAY, UEU
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.A, VnLexiName.O),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.A, VnLexiName.Y),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Ar, VnLexiName.Y),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.E, VnLexiName.U),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Er, VnLexiName.U),

            // UIA, UOI, UOrI, UOhI
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.I, VnLexiName.A),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.O, VnLexiName.I),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Or, VnLexiName.I),
            CreateVowelEntry(1, 1, VnLexiName.Uh, VnLexiName.Oh, VnLexiName.I),

            // UOU, UOrU, UOhU
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.O, VnLexiName.U),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Or, VnLexiName.U),
            CreateVowelEntry(1, 1, VnLexiName.Uh, VnLexiName.Oh, VnLexiName.U),

            // uơi (uohi), uơu (uohu), ưoi (uhoi), ưou (uhou)
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Oh, VnLexiName.I),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Oh, VnLexiName.U),
            CreateVowelEntry(0, 0, VnLexiName.Uh, VnLexiName.O, VnLexiName.I),
            CreateVowelEntry(0, 0, VnLexiName.Uh, VnLexiName.O, VnLexiName.U),

            // UYA, UYE, UYEr, UYU
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Y, VnLexiName.A),
            CreateVowelEntry(2, 2, VnLexiName.U, VnLexiName.Y, VnLexiName.E),
            CreateVowelEntry(2, 2, VnLexiName.U, VnLexiName.Y, VnLexiName.Er),
            CreateVowelEntry(1, 1, VnLexiName.U, VnLexiName.Y, VnLexiName.U),

            // YEU, YErU
            CreateVowelEntry(1, 1, VnLexiName.Y, VnLexiName.E, VnLexiName.U),
            CreateVowelEntry(1, 1, VnLexiName.Y, VnLexiName.Er, VnLexiName.U)
        };

        VowelSeqMap = vowelEntries.ToFrozenDictionary();

        for (int i = 0; i < CharsetTables.TotalVnChars; i++)
        {
            VowelSlotLookup[i] = (byte)ComputeVowelSlot((VnLexiName)i);
        }

        foreach (var entry in vowelEntries)
        {
            var seq = entry.Value.Vowels;
            int fastKey = EmptyVowelKey;
            for (int i = 0; i < seq.Length; i++)
            {
                fastKey = PackVowelSlot(fastKey, i, GetVowelSlot(seq[i]));
            }
            VowelFastTable[fastKey] = true;
        }

        var conEntries = new[]
        {
            new ConSeq("b", true, false),
            new ConSeq("c", true, true),
            new ConSeq("ch", true, true),
            new ConSeq("d", true, false),
            new ConSeq("dd", true, false),
            new ConSeq("g", true, false),
            new ConSeq("gh", true, false),
            new ConSeq("gi", true, false),
            new ConSeq("h", true, false),
            new ConSeq("k", true, false),
            new ConSeq("kh", true, false),
            new ConSeq("l", true, false),
            new ConSeq("m", true, true),
            new ConSeq("n", true, true),
            new ConSeq("ng", true, true),
            new ConSeq("ngh", true, false),
            new ConSeq("nh", true, true),
            new ConSeq("p", true, true),
            new ConSeq("ph", true, false),
            new ConSeq("qu", true, false),
            new ConSeq("r", true, false),
            new ConSeq("s", true, false),
            new ConSeq("t", true, true),
            new ConSeq("th", true, false),
            new ConSeq("tr", true, false),
            new ConSeq("v", true, false),
            new ConSeq("x", true, false)
        };

        ConsonantMap = conEntries.ToFrozenDictionary(c => c.Key, c => c, StringComparer.Ordinal);

        ValidEndingConsonants = conEntries
            .Where(c => c.CanBeEnding)
            .Select(c => c.Key)
            .ToFrozenSet(StringComparer.Ordinal);

        ConsonantSpanLookup = ConsonantMap.GetAlternateLookup<ReadOnlySpan<char>>();
        EndingSpanLookup = ValidEndingConsonants.GetAlternateLookup<ReadOnlySpan<char>>();

        // Legal vowel + coda combinations (port of UniKey's VCPairList).
        // Encoding: (packed vowel sequence << 3) | coda id.
        var pairs = new HashSet<int>();
        AddVowelCodaPairs(pairs, "c ch m n ng nh p t", VnLexiName.A);
        AddVowelCodaPairs(pairs, "c m n ng p t", VnLexiName.Ar);
        AddVowelCodaPairs(pairs, "c m n ng p t", VnLexiName.Ah);
        AddVowelCodaPairs(pairs, "c ch m n ng nh p t", VnLexiName.E);
        AddVowelCodaPairs(pairs, "c ch m n nh p t", VnLexiName.Er);
        AddVowelCodaPairs(pairs, "c ch m n nh p t", VnLexiName.I);
        AddVowelCodaPairs(pairs, "c m n ng p t", VnLexiName.O);
        AddVowelCodaPairs(pairs, "c m n ng p t", VnLexiName.Or);
        AddVowelCodaPairs(pairs, "m n p t", VnLexiName.Oh);
        AddVowelCodaPairs(pairs, "c m n ng p t", VnLexiName.U);
        AddVowelCodaPairs(pairs, "c m n ng t", VnLexiName.Uh);
        AddVowelCodaPairs(pairs, "t", VnLexiName.Y);
        AddVowelCodaPairs(pairs, "c m n ng p t", VnLexiName.I, VnLexiName.E);
        AddVowelCodaPairs(pairs, "c m n ng p t", VnLexiName.I, VnLexiName.Er);
        AddVowelCodaPairs(pairs, "c ch m n ng nh p t", VnLexiName.O, VnLexiName.A);
        AddVowelCodaPairs(pairs, "c m n ng t", VnLexiName.O, VnLexiName.Ah);
        AddVowelCodaPairs(pairs, "n t", VnLexiName.O, VnLexiName.E);
        AddVowelCodaPairs(pairs, "n ng t", VnLexiName.U, VnLexiName.A);
        AddVowelCodaPairs(pairs, "n ng t", VnLexiName.U, VnLexiName.Ar);
        AddVowelCodaPairs(pairs, "c ch n nh", VnLexiName.U, VnLexiName.E);
        AddVowelCodaPairs(pairs, "c ch n nh", VnLexiName.U, VnLexiName.Er);
        AddVowelCodaPairs(pairs, "c m n ng p t", VnLexiName.U, VnLexiName.O);
        AddVowelCodaPairs(pairs, "c m n ng t", VnLexiName.U, VnLexiName.Or);
        AddVowelCodaPairs(pairs, "c m n ng p t", VnLexiName.Uh, VnLexiName.O);
        AddVowelCodaPairs(pairs, "c m n ng p t", VnLexiName.Uh, VnLexiName.Oh);
        AddVowelCodaPairs(pairs, "c ch n nh p t", VnLexiName.U, VnLexiName.Y);
        AddVowelCodaPairs(pairs, "m n ng p t", VnLexiName.Y, VnLexiName.E);
        AddVowelCodaPairs(pairs, "m n ng t", VnLexiName.Y, VnLexiName.Er);
        AddVowelCodaPairs(pairs, "n t", VnLexiName.U, VnLexiName.Y, VnLexiName.E);
        AddVowelCodaPairs(pairs, "n t", VnLexiName.U, VnLexiName.Y, VnLexiName.Er);

        VowelCodaPairs = pairs.ToFrozenSet();
    }

    private static void AddVowelCodaPairs(HashSet<int> pairs, string codas, params VnLexiName[] vowels)
    {
        int vowelKey = PackKey(vowels) << 3;

        int fastKey = EmptyVowelKey;
        for (int i = 0; i < vowels.Length; i++)
        {
            fastKey = PackVowelSlot(fastKey, i, GetVowelSlot(vowels[i]));
        }

        foreach (var coda in codas.Split(' '))
        {
            int codaId = GetCodaId(coda);
            if (codaId >= 0)
            {
                pairs.Add(vowelKey | codaId);
                CodaFastTable[(fastKey << 3) | codaId] = true;
            }
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int GetVowelSlot(VnLexiName lexi)
    {
        return (uint)(int)lexi < (uint)CharsetTables.TotalVnChars
            ? VowelSlotLookup[(int)lexi]
            : NoVowelSlot;
    }

    private static int ComputeVowelSlot(VnLexiName lexi)
    {
        return NormalizeVowel(lexi) switch
        {
            VnLexiName.A => 0,
            VnLexiName.Ar => 1,
            VnLexiName.Ah => 2,
            VnLexiName.E => 3,
            VnLexiName.Er => 4,
            VnLexiName.I => 5,
            VnLexiName.O => 6,
            VnLexiName.Or => 7,
            VnLexiName.Oh => 8,
            VnLexiName.U => 9,
            VnLexiName.Uh => 10,
            VnLexiName.Y => 11,
            _ => NoVowelSlot
        };
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int PackVowelSlot(int key, int position, int slot)
    {
        return (key & ~(0xF << (position * 4))) | (slot << (position * 4));
    }

    /// <summary>
    /// O(1) vowel nucleus validation using the compact slot table (no hash lookup).
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsValidVowelSlots(int fastKey, int count)
    {
        return count is >= 1 and <= 3 && VowelFastTable[fastKey];
    }

    /// <summary>
    /// O(1) vowel + coda validation using the compact slot table (no hash lookup).
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsValidVowelCodaSlots(int fastKey, int count, ReadOnlySpan<char> coda)
    {
        return IsValidVowelCodaSlots(fastKey, count, GetCodaId(coda));
    }

    /// <summary>
    /// O(1) vowel + coda validation from a pre-computed coda id.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsValidVowelCodaSlots(int fastKey, int count, int codaId)
    {
        return count is >= 1 and <= 3 && codaId >= 0 && CodaFastTable[(fastKey << 3) | codaId];
    }

    private static int GetCodaId(ReadOnlySpan<char> coda)
    {
        if (coda.Length is < 1 or > 2)
        {
            return -1;
        }

        return GetCodaId(coda.Length, coda[0], coda.Length > 1 ? coda[1] : '\0');
    }

    /// <summary>
    /// Maps a lowercase coda (1-2 letters) to a compact id used by the fast tables.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int GetCodaId(int length, char c0, char c1)
    {
        if (length == 1)
        {
            return c0 switch
            {
                'c' => 0,
                'm' => 1,
                'n' => 2,
                'p' => 3,
                't' => 4,
                _ => -1
            };
        }

        if (length == 2)
        {
            return (c0, c1) switch
            {
                ('c', 'h') => 5,
                ('n', 'g') => 6,
                ('n', 'h') => 7,
                _ => -1
            };
        }

        return -1;
    }

    private static KeyValuePair<int, VowelSeq> CreateVowelEntry(
        int classicTone,
        int modernTone,
        params VnLexiName[] vowels)
    {
        int key = PackKey(vowels);
        return new KeyValuePair<int, VowelSeq>(key, new VowelSeq(vowels, classicTone, modernTone));
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool TryGetVowelSeq(ReadOnlySpan<VnLexiName> vowels, out VowelSeq seq)
    {
        int key = PackKey(vowels);
        return VowelSeqMap.TryGetValue(key, out seq);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsValidVowelSequence(ReadOnlySpan<VnLexiName> vowels)
    {
        int key = PackKey(vowels);
        return VowelSeqMap.ContainsKey(key);
    }

    /// <summary>
    /// Validates a vowel nucleus from its pre-packed key (allocation-free incremental path).
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsValidVowelKey(int vowelKey, int count)
    {
        return count is >= 1 and <= 3 && VowelSeqMap.ContainsKey(vowelKey);
    }

    /// <summary>
    /// Validates a vowel nucleus + coda pair from the packed nucleus key (port of isValidVC).
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsValidVowelCodaKey(int vowelKey, int count, ReadOnlySpan<char> coda)
    {
        if (count is < 1 or > 3)
        {
            return false;
        }

        int codaId = GetCodaId(coda);
        return codaId >= 0 && VowelCodaPairs.Contains((vowelKey << 3) | codaId);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsValidEndingConsonant(string ending)
    {
        return ValidEndingConsonants.Contains(ending.ToLowerInvariant());
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsValidEndingConsonant(ReadOnlySpan<char> ending)
    {
        return ending.Length > 0 && EndingSpanLookup.Contains(ending);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsValidInitialConsonant(string initial)
    {
        return TryGetConsonant(initial.ToLowerInvariant().AsSpan(), out var c) && c.CanBeInitial;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool TryGetConsonant(ReadOnlySpan<char> initial, out ConSeq consonant)
    {
        return ConsonantSpanLookup.TryGetValue(initial, out consonant);
    }

    /// <summary>
    /// Port of UniKey's <c>isValidCV</c>: checks an initial consonant against the first vowel.
    /// </summary>
    public static bool IsValidInitialWithVowel(ReadOnlySpan<char> initial, VnLexiName firstVowel)
    {
        if (initial.IsEmpty)
        {
            return true;
        }

        if (!TryGetConsonant(initial, out var c) || !c.CanBeInitial)
        {
            return false;
        }

        VnLexiName v = NormalizeVowel(firstVowel);

        // "k" only goes with e, ê, i, y (and sequences starting with them)
        if (initial.Length == 1 && (initial[0] == 'k' || initial[0] == 'K'))
        {
            return v is VnLexiName.E or VnLexiName.Er or VnLexiName.I or VnLexiName.Y;
        }

        // gi doesn't go with i, qu doesn't go with u
        if (initial.Length == 2)
        {
            if ((initial[0] == 'g' || initial[0] == 'G') && (initial[1] == 'i' || initial[1] == 'I'))
            {
                return v != VnLexiName.I;
            }
            if ((initial[0] == 'q' || initial[0] == 'Q') && (initial[1] == 'u' || initial[1] == 'U'))
            {
                return v != VnLexiName.U;
            }
        }

        return true;
    }

    /// <summary>
    /// Port of UniKey's <c>isValidVC</c> using the <c>VCPairList</c> table (no allocations).
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsValidVowelCoda(ReadOnlySpan<VnLexiName> vowels, ReadOnlySpan<char> coda)
    {
        if (coda.IsEmpty)
        {
            return true;
        }

        int codaId = GetCodaId(coda);
        return codaId >= 0 && VowelCodaPairs.Contains((PackKey(vowels) << 3) | codaId);
    }

    /// <summary>
    /// Stop codas (c, ch, p, t) only accept the acute and dot tones.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsStopCoda(ReadOnlySpan<char> coda)
    {
        return coda.Length switch
        {
            1 => coda[0] is 'c' or 'p' or 't',
            2 => coda[0] == 'c' && coda[1] == 'h',
            _ => false
        };
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int GetTonePosition(
        ReadOnlySpan<VnLexiName> vowels,
        bool hasEndingConsonant,
        bool modernStyle)
    {
        if (vowels.Length == 0) return 0;
        if (vowels.Length == 1) return 0;

        if (TryGetVowelSeq(vowels, out var seq))
        {
            if (hasEndingConsonant)
            {
                return seq.ModernToneIndex;
            }
            return modernStyle ? seq.ModernToneIndex : seq.ClassicToneIndex;
        }

        return vowels.Length > 1 ? 1 : 0;
    }
}

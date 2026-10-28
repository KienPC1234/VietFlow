// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Collections.Frozen;
using System.Runtime.CompilerServices;
using VietFlow.Tables;

namespace VietFlow.Engine;

/// <summary>
/// High-performance English keyword dictionary and phonetic analysis to prevent
/// English / programming words from getting accidental Vietnamese diacritics.
/// </summary>
public static class EnglishPhonetics
{
    private static readonly FrozenSet<string> Keywords;
    private static FrozenSet<string>.AlternateLookup<ReadOnlySpan<char>> KeywordLookup;
    private static readonly int MaxKeywordLength;

    static EnglishPhonetics()
    {
        var words = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            // Programming & Polysyllabic English keywords with internal 's' (conflicts with Telex 's' acute tone)
            // NOTE: Never include words that are valid Vietnamese syllables (e.g. bus->bú, must->mút, rust->rút, bust->bút, trust->trút, gust->gút, sum->sum)
            "custom", "customer", "customize", "customs",
            "music", "musician", "museum",
            "status", "focus", "virus", "bonus", "minus", "campus", "circus",
            "user", "users", "username", "useful", "useless",
            "push", "support", "super", "sugar", "summer", "submit", "subscribe", "success",
            "suggest", "summary", "surface", "survey", "survive", "suspect",
            "issue", "issues", "result", "results", "resume", "resource", "resources",
            "response", "responses", "request", "requests", "require", "requires",

            // English words ending in x or containing x
            // NOTE: Never include mix->mĩ, box->bõ, six->sĩ, max->mã, tax->tã
            "alex", "linux", "index", "context", "matrix", "complex", "syntax", "prefix",
            "suffix", "vertex", "pixel", "relax", "remix", "apex", "latex", "unix", "posix",

            // Words with foreign consonant endings / English programming keywords
            // NOTE: Never include test->tét, best->bét, rest->rét, host->hót, cost->cót, last->lát, cast->cát, list->lít, most->mót, boost->bốt
            "text", "next", "exist", "select", "project", "object", "export",
            "import", "string", "spring", "struct", "strict", "direct",
            "effect", "correct", "product", "protect", "detect", "connect", "collect",
            "inspect", "inject", "reject", "reflect", "twist", "first",
            "worst", "east", "almost",

            // English words with double letters or non-VN codas
            // NOTE: Never include seen->sên, moon->môn, noon->nôn, soon->sôn, room->rôm, vans->ván, lens->lén
            "class", "press", "process", "access", "address", "express",
            "impress", "stress", "cross", "guess",
            "business", "fitness", "witness", "canvas", "atlas", "radius", "versus", "alias",
            "basis", "crisis", "thesis", "axis", "paris",

            // Programming & technical keywords (with zero Vietnamese homophones)
            "code", "true", "false",
            "null", "type", "void", "char", "bool", "byte",
            "float", "double", "public", "private", "protected", "static", "sealed", "readonly", "override",
            "return", "switch", "while", "break", "continue", "default", "async", "await", "yield",
            "throw", "package", "button", "click", "event", "target", "window",
            "document", "header", "footer", "content", "view", "model", "service", "controller",
            "password", "message", "display", "insert", "update", "delete", "create",
            "docker", "github", "reddit", "python", "react", "rust", "golang", "swift", "kotlin",
            "commit", "checkout", "branch", "merge", "fetch", "pull", "style", "script",

            // Double vowel words ending in non-VN consonants (l, d, k)
            // NOTE: Never include car->cả, bar->bả, sir->sỉ, nor->nỏ
            "cool", "root", "boot", "tool", "pool", "book", "look", "good", "food",
            "feed", "feel", "feet", "free", "zoom", "week", "wood", "wool",
            "loop", "floor", "cook", "hook", "deep", "keep",
            "meet", "need", "seed", "weed", "green", "screen", "sweet", "speed", "steel",

            // Words ending in non-VN codas (polysyllabic r, l, etc.)
            "server", "driver", "water", "filter", "center", "enter",
            "letter", "master", "system", "item", "problem", "config", "debug"
        };

        int maxLength = 0;
        foreach (var word in words)
        {
            if (word.Length > maxLength) maxLength = word.Length;
        }

        MaxKeywordLength = maxLength;
        // Stored lowercase; lookups lowercase the span (ASCII) and use an Ordinal hash,
        // which is much faster than case-insensitive span hashing.
        Keywords = words.ToFrozenSet(StringComparer.Ordinal);
        KeywordLookup = Keywords.GetAlternateLookup<ReadOnlySpan<char>>();
    }

    /// <summary>
    /// Allocation-free, ordinal (fast) keyword lookup.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsEnglishKeyword(ReadOnlySpan<char> word)
    {
        if (word.IsEmpty || word.Length > MaxKeywordLength)
        {
            return false;
        }

        Span<char> lower = stackalloc char[MaxKeywordLength];
        for (int i = 0; i < word.Length; i++)
        {
            lower[i] = InputProcessor.ToLowerAscii(word[i]);
        }

        return KeywordLookup.Contains(lower[..word.Length]);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsForeignInitial(char rawChar, VnLexiName lexi)
    {
        if (lexi is VnLexiName.uh or VnLexiName.Uh) return false;
        char lower = InputProcessor.ToLowerAscii(rawChar);
        return lower is 'f' or 'j' or 'w' or 'z';
    }
}

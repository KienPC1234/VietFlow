// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Collections.Frozen;
using System.Runtime.CompilerServices;

namespace VietFlow.Engine;

/// <summary>
/// English/foreign keyword dictionary used to avoid accidental Vietnamese marks
/// on English or programming words. The word list can be replaced at runtime
/// (Advanced settings) without restarting the engine.
/// </summary>
public static class EnglishPhonetics
{
    private sealed class KeywordSet
    {
        public required FrozenSet<string> Set { get; init; }
        public required FrozenSet<string>.AlternateLookup<ReadOnlySpan<char>> Lookup { get; init; }
        public required int MaxLength { get; init; }
        public required string[] Words { get; init; }
    }

    private static readonly string[] DefaultWords;
    private static volatile KeywordSet _current;

    static EnglishPhonetics()
    {
        DefaultWords = CreateDefaultWords();
        _current = Build(DefaultWords);
    }

    /// <summary>
    /// Built-in word list (lowercase, distinct).
    /// </summary>
    public static IReadOnlyList<string> DefaultKeywords => DefaultWords;

    /// <summary>
    /// Replaces the active keyword list. An empty/null list falls back to the built-in defaults.
    /// </summary>
    public static void SetKeywords(IEnumerable<string>? words)
    {
        var normalized = words?
            .Select(w => w.Trim().ToLowerInvariant())
            .Where(w => w.Length > 0)
            .Distinct(StringComparer.Ordinal)
            .ToArray();

        _current = normalized is { Length: > 0 } ? Build(normalized) : Build(DefaultWords);
    }

    /// <summary>
    /// Allocation-free keyword lookup (ordinal hash after ASCII lowercasing).
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsEnglishKeyword(ReadOnlySpan<char> word)
    {
        var current = _current;
        if (word.IsEmpty || word.Length > current.MaxLength)
        {
            return false;
        }

        Span<char> lower = stackalloc char[current.MaxLength];
        for (int i = 0; i < word.Length; i++)
        {
            lower[i] = InputProcessor.ToLowerAscii(word[i]);
        }

        return current.Lookup.Contains(lower[..word.Length]);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static bool IsForeignInitial(char rawChar, VnLexiName lexi, ReadOnlySpan<char> foreignInitials)
    {
        if (lexi is VnLexiName.uh or VnLexiName.Uh)
        {
            return false;
        }

        char lower = InputProcessor.ToLowerAscii(rawChar);
        return foreignInitials.Contains(lower);
    }

    private static KeywordSet Build(string[] words)
    {
        int maxLength = 0;
        foreach (var word in words)
        {
            if (word.Length > maxLength) maxLength = word.Length;
        }

        var set = words.ToFrozenSet(StringComparer.Ordinal);
        return new KeywordSet
        {
            Set = set,
            Lookup = set.GetAlternateLookup<ReadOnlySpan<char>>(),
            MaxLength = maxLength,
            Words = words
        };
    }

    private static string[] CreateDefaultWords()
    {
        var words = new HashSet<string>(StringComparer.Ordinal)
        {
            // Programming & polysyllabic English keywords with internal 's' (Telex acute conflict)
            // Never include valid Vietnamese syllables: bus->bú, must->mút, rust->rút, bust->bút, sum->sum
            "custom", "customer", "customize", "customs",
            "music", "musician", "museum",
            "status", "focus", "virus", "bonus", "minus", "campus", "circus",
            "user", "users", "username", "useful", "useless",
            "push", "support", "super", "sugar", "summer", "submit", "subscribe", "success",
            "suggest", "summary", "surface", "survey", "survive", "suspect",
            "issue", "issues", "result", "results", "resume", "resource", "resources",
            "response", "responses", "request", "requests", "require", "requires",

            // Words containing x (never mix->mĩ, box->bõ, six->sĩ, max->mã, tax->tã)
            "alex", "linux", "index", "context", "matrix", "complex", "syntax", "prefix",
            "suffix", "vertex", "pixel", "relax", "remix", "apex", "latex", "unix", "posix",

            // Foreign consonant endings / programming keywords
            // Never test->tét, best->bét, rest->rét, host->hót, cost->cót, last->lát, cast->cát
            "text", "next", "exist", "select", "project", "object", "export",
            "import", "string", "spring", "struct", "strict", "direct",
            "effect", "correct", "product", "protect", "detect", "connect", "collect",
            "inspect", "inject", "reject", "reflect", "twist", "first",
            "worst", "east", "almost",

            // Double letters / non-VN codas
            "class", "press", "process", "access", "address", "express",
            "impress", "stress", "cross", "guess",
            "business", "fitness", "witness", "canvas", "atlas", "radius", "versus", "alias",
            "basis", "crisis", "thesis", "axis", "paris",

            // Programming & technical keywords
            "code", "true", "false",
            "null", "type", "void", "char", "bool", "byte",
            "float", "double", "public", "private", "protected", "static", "sealed", "readonly", "override",
            "return", "switch", "while", "break", "continue", "default", "async", "await", "yield",
            "throw", "package", "button", "click", "event", "target", "window",
            "document", "header", "footer", "content", "view", "model", "service", "controller",
            "password", "message", "display", "insert", "update", "delete", "create",
            "docker", "github", "reddit", "python", "react", "rust", "golang", "swift", "kotlin",
            "commit", "checkout", "branch", "merge", "fetch", "pull", "style", "script",

            // Double vowel words ending in non-VN consonants
            "cool", "root", "boot", "tool", "pool", "book", "look", "good", "food",
            "feed", "feel", "feet", "free", "zoom", "week", "wood", "wool",
            "loop", "floor", "cook", "hook", "deep", "keep",
            "meet", "need", "seed", "weed", "green", "screen", "sweet", "speed", "steel",

            // Non-VN codas (r, l, ...)
            "server", "driver", "water", "filter", "center", "enter",
            "letter", "master", "system", "item", "problem", "config", "debug"
        };

        return words.OrderBy(w => w, StringComparer.Ordinal).ToArray();
    }
}

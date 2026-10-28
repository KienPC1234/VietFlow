using System.Collections.Concurrent;
using System.Collections.Frozen;
using System.Text;

namespace VietFlow.Engine;

public sealed class MacroTable
{
    private readonly ConcurrentDictionary<string, string> _macros = new(StringComparer.OrdinalIgnoreCase);

    // Immutable snapshot for allocation-free, lock-free reads (span lookups).
    private FrozenDictionary<string, string>? _snapshot;
    private readonly object _snapshotLock = new();

    public int Count => _macros.Count;

    public bool HasEntries => !_macros.IsEmpty;

    public Func<string>? ClipboardProvider { get; set; }

    /// <summary>Format for @date, using .NET date format syntax.</summary>
    public string DateFormat { get; set; } = "dd/MM/yyyy";

    /// <summary>Format for @time, using .NET date format syntax.</summary>
    public string TimeFormat { get; set; } = "HH:mm";

    /// <summary>Format for @datetime, using .NET date format syntax.</summary>
    public string DateTimeFormat { get; set; } = "dd/MM/yyyy HH:mm";

    /// <summary>User-defined @variables, without the leading @.</summary>
    public IReadOnlyDictionary<string, string>? CustomVariables { get; set; }

    public void AddOrUpdate(string trigger, string replacement)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(trigger);
        ArgumentNullException.ThrowIfNull(replacement);
        _macros[trigger] = replacement;
        _snapshot = null;
    }

    public bool Remove(string trigger)
    {
        if (string.IsNullOrWhiteSpace(trigger)) return false;
        bool removed = _macros.TryRemove(trigger, out _);
        if (removed) _snapshot = null;
        return removed;
    }

    public void Clear()
    {
        _macros.Clear();
        _snapshot = null;
    }

    public bool TryGet(string trigger, out string replacement)
    {
        return TryGet(trigger.AsSpan(), out replacement);
    }

    /// <summary>
    /// Allocation-free macro lookup (span alternate lookup on a frozen snapshot).
    /// </summary>
    public bool TryGet(ReadOnlySpan<char> trigger, out string replacement)
    {
        if (trigger.IsEmpty)
        {
            replacement = string.Empty;
            return false;
        }

        var snapshot = Volatile.Read(ref _snapshot) ?? CreateSnapshot();

        if (snapshot.GetAlternateLookup<ReadOnlySpan<char>>().TryGetValue(trigger, out var rawValue))
        {
            replacement = ExpandDynamicVariables(rawValue);
            return true;
        }

        replacement = string.Empty;
        return false;
    }

    private FrozenDictionary<string, string> CreateSnapshot()
    {
        lock (_snapshotLock)
        {
            return _snapshot ??= _macros.ToFrozenDictionary(StringComparer.OrdinalIgnoreCase);
        }
    }

    private string ExpandDynamicVariables(string template)
    {
        if (!template.Contains('@'))
        {
            return template;
        }

        DateTime now = DateTime.Now;
        var builder = new StringBuilder(template.Length + 16);
        int index = 0;

        while (index < template.Length)
        {
            char current = template[index];
            if (current != '@')
            {
                builder.Append(current);
                index++;
                continue;
            }

            int tokenStart = index + 1;
            int tokenEnd = tokenStart;
            while (tokenEnd < template.Length)
            {
                char c = template[tokenEnd];
                if (!char.IsLetterOrDigit(c) && c != '_')
                {
                    break;
                }

                tokenEnd++;
            }

            if (tokenEnd == tokenStart)
            {
                builder.Append('@');
                index++;
                continue;
            }

            ReadOnlySpan<char> token = template.AsSpan(tokenStart, tokenEnd - tokenStart);
            if (TryResolveDynamicVariable(token, now, out string value))
            {
                builder.Append(value);
            }
            else
            {
                builder.Append('@').Append(token);
            }

            index = tokenEnd;
        }

        return builder.ToString();
    }

    private bool TryResolveDynamicVariable(ReadOnlySpan<char> token, DateTime now, out string value)
    {
        if (token.Equals("date", StringComparison.OrdinalIgnoreCase))
        {
            value = FormatDate(now, DateFormat, "dd/MM/yyyy");
            return true;
        }

        if (token.Equals("time", StringComparison.OrdinalIgnoreCase))
        {
            value = FormatDate(now, TimeFormat, "HH:mm");
            return true;
        }

        if (token.Equals("datetime", StringComparison.OrdinalIgnoreCase) ||
            token.Equals("now", StringComparison.OrdinalIgnoreCase))
        {
            string fallback = $"{FormatDate(now, DateFormat, "dd/MM/yyyy")} {FormatDate(now, TimeFormat, "HH:mm")}";
            value = FormatDate(now, DateTimeFormat, fallback);
            return true;
        }

        if (token.Equals("day", StringComparison.OrdinalIgnoreCase))
        {
            value = now.ToString("dd");
            return true;
        }

        if (token.Equals("month", StringComparison.OrdinalIgnoreCase))
        {
            value = now.ToString("MM");
            return true;
        }

        if (token.Equals("year", StringComparison.OrdinalIgnoreCase))
        {
            value = now.ToString("yyyy");
            return true;
        }

        if (token.Equals("weekday", StringComparison.OrdinalIgnoreCase) ||
            token.Equals("dayname", StringComparison.OrdinalIgnoreCase))
        {
            value = now.ToString("dddd");
            return true;
        }

        if (token.Equals("timestamp", StringComparison.OrdinalIgnoreCase) ||
            token.Equals("unix", StringComparison.OrdinalIgnoreCase))
        {
            value = new DateTimeOffset(now).ToUnixTimeSeconds().ToString();
            return true;
        }

        if (token.Equals("random", StringComparison.OrdinalIgnoreCase))
        {
            value = CreateRandomDigits(6);
            return true;
        }

        if (token.Equals("uuid", StringComparison.OrdinalIgnoreCase) ||
            token.Equals("guid", StringComparison.OrdinalIgnoreCase))
        {
            value = Guid.NewGuid().ToString();
            return true;
        }

        if (token.Equals("user", StringComparison.OrdinalIgnoreCase))
        {
            value = Environment.UserName;
            return true;
        }

        if (token.Equals("computer", StringComparison.OrdinalIgnoreCase))
        {
            value = Environment.MachineName;
            return true;
        }

        if (token.Equals("clipboard", StringComparison.OrdinalIgnoreCase))
        {
            try
            {
                value = ClipboardProvider?.Invoke() ?? string.Empty;
            }
            catch
            {
                value = string.Empty;
            }

            return true;
        }

        if (token.Equals("newline", StringComparison.OrdinalIgnoreCase))
        {
            value = "\n";
            return true;
        }

        if (token.Equals("tab", StringComparison.OrdinalIgnoreCase))
        {
            value = "\t";
            return true;
        }

        if (token.Equals("space", StringComparison.OrdinalIgnoreCase))
        {
            value = " ";
            return true;
        }

        if (CustomVariables is { Count: > 0 })
        {
            string key = token.ToString();
            if (CustomVariables.TryGetValue(key, out string? custom))
            {
                value = custom ?? string.Empty;
                return true;
            }

            // Dictionary comparers are not guaranteed for user-supplied dictionaries.
            foreach (var pair in CustomVariables)
            {
                if (string.Equals(pair.Key, key, StringComparison.OrdinalIgnoreCase))
                {
                    value = pair.Value ?? string.Empty;
                    return true;
                }
            }
        }

        value = string.Empty;
        return false;
    }

    private static string FormatDate(DateTime now, string format, string fallback)
    {
        if (string.IsNullOrWhiteSpace(format))
        {
            return now.ToString(fallback);
        }

        try
        {
            return now.ToString(format);
        }
        catch
        {
            return now.ToString(fallback);
        }
    }

    private static string CreateRandomDigits(int length)
    {
        Span<char> buffer = stackalloc char[length];
        for (int i = 0; i < buffer.Length; i++)
        {
            buffer[i] = (char)('0' + Random.Shared.Next(10));
        }

        return new string(buffer);
    }
}

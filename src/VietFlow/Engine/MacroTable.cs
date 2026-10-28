using System.Collections.Concurrent;
using System.Collections.Frozen;

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

        string result = template;
        if (result.Contains("@date"))
        {
            result = result.Replace("@date", DateTime.Now.ToString("dd/MM/yyyy"));
        }
        if (result.Contains("@time"))
        {
            result = result.Replace("@time", DateTime.Now.ToString("HH:mm"));
        }
        if (result.Contains("@clipboard") && ClipboardProvider != null)
        {
            try
            {
                string clip = ClipboardProvider() ?? string.Empty;
                result = result.Replace("@clipboard", clip);
            }
            catch
            {
                result = result.Replace("@clipboard", string.Empty);
            }
        }

        return result;
    }
}

// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using VietFlow.Configuration;

namespace VietFlow.Windows;

internal sealed class ForegroundTracker
{
    private readonly VietFlowConfig _config;
    private IntPtr _lastHwnd = IntPtr.Zero;
    private string _lastProcessName = string.Empty;
    private bool _lastIsExcluded = false;
    private int _lastExcludedCount = -1;
    private readonly HashSet<string> _excludedSet = new(StringComparer.OrdinalIgnoreCase);

    public ForegroundTracker(VietFlowConfig config)
    {
        _config = config;
        SyncExcludedSet();
    }

    private void SyncExcludedSet()
    {
        if (_lastExcludedCount != _config.ExcludedApps.Count)
        {
            _excludedSet.Clear();
            for (int i = 0; i < _config.ExcludedApps.Count; i++)
            {
                _excludedSet.Add(_config.ExcludedApps[i]);
            }
            _lastExcludedCount = _config.ExcludedApps.Count;
        }
    }

    /// <summary>
    /// Checks whether the currently active foreground window belongs to an excluded app (Game Mode).
    /// Highly optimized with handle-level caching and O(1) hash lookup.
    /// </summary>
    public bool IsForegroundAppExcluded()
    {
        if (_config.ExcludedApps.Count == 0)
        {
            return false;
        }

        IntPtr hwnd = NativeMethods.GetForegroundWindow();
        if (hwnd == IntPtr.Zero)
        {
            return false;
        }

        if (hwnd == _lastHwnd)
        {
            return _lastIsExcluded;
        }

        _lastHwnd = hwnd;
        _lastProcessName = NativeMethods.GetForegroundProcessName();

        if (string.IsNullOrEmpty(_lastProcessName))
        {
            _lastIsExcluded = false;
            return false;
        }

        SyncExcludedSet();
        _lastIsExcluded = _excludedSet.Contains(_lastProcessName);
        return _lastIsExcluded;
    }

    public string CurrentProcessName => _lastProcessName;
}


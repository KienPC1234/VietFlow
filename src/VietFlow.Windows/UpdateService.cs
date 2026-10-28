// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System;
using System.Threading;
using System.Threading.Tasks;
using Velopack;
using Velopack.Sources;

namespace VietFlow.Windows;

public enum UpdateStatus
{
    Idle,
    Checking,
    UpToDate,
    UpdateAvailable,
    Downloading,
    ReadyToRestart,
    Error
}

public sealed class UpdateService
{
    private const string RepoUrl = "https://github.com/KienPC1234/VietFlow";

    private readonly SemaphoreSlim _lock = new(1, 1);
    private UpdateManager? _updateManager;
    private UpdateInfo? _pendingUpdate;

    public UpdateStatus Status { get; private set; } = UpdateStatus.Idle;
    public string StatusMessage { get; private set; } = string.Empty;
    public int DownloadProgress { get; private set; } = 0;
    public string? NewVersion { get; private set; }

    public event Action? StateChanged;

    public bool IsInstalled => GetManager().IsInstalled;
    public string CurrentVersionText => GetManager().CurrentVersion?.ToString() ?? "1.0.4";

    private UpdateManager GetManager()
    {
        if (_updateManager == null)
        {
            var source = new GithubSource(RepoUrl, null, false);
            _updateManager = new UpdateManager(source);
        }
        return _updateManager;
    }

    private void SetState(UpdateStatus status, string message, int progress = 0, string? newVersion = null)
    {
        Status = status;
        StatusMessage = message;
        DownloadProgress = progress;
        if (newVersion != null)
        {
            NewVersion = newVersion;
        }
        StateChanged?.Invoke();
    }

    public async Task<UpdateInfo?> CheckForUpdatesAsync(bool silent = false)
    {
        if (!await _lock.WaitAsync(0))
        {
            return _pendingUpdate;
        }

        try
        {
            SetState(UpdateStatus.Checking, "Đang kiểm tra bản cập nhật mới trên GitHub...");

            var mgr = GetManager();
            var updateInfo = await mgr.CheckForUpdatesAsync();

            if (updateInfo == null)
            {
                SetState(
                    UpdateStatus.UpToDate,
                    $"VietFlow đang ở phiên bản mới nhất ({CurrentVersionText}).");
                return null;
            }

            _pendingUpdate = updateInfo;
            string targetVer = updateInfo.TargetFullRelease.Version.ToString();
            SetState(
                UpdateStatus.UpdateAvailable,
                $"Đã tìm thấy bản cập nhật mới: v{targetVer}.",
                newVersion: targetVer);

            return updateInfo;
        }
        catch (Exception ex)
        {
            string err = silent
                ? "Không thể kiểm tra cập nhật."
                : $"Không thể kiểm tra cập nhật: {ex.Message}";
            SetState(UpdateStatus.Error, err);
            return null;
        }
        finally
        {
            _lock.Release();
        }
    }

    public async Task<bool> DownloadAndInstallUpdateAsync(Action<int>? onProgress = null)
    {
        if (_pendingUpdate == null)
        {
            var update = await CheckForUpdatesAsync();
            if (update == null)
            {
                return false;
            }
        }

        var updateToDownload = _pendingUpdate;
        if (updateToDownload == null)
        {
            return false;
        }

        if (!await _lock.WaitAsync(0))
        {
            return false;
        }

        try
        {
            var mgr = GetManager();
            SetState(UpdateStatus.Downloading, "Đang tải bản cập nhật...", 0);

            await mgr.DownloadUpdatesAsync(updateToDownload, progress =>
            {
                DownloadProgress = progress;
                onProgress?.Invoke(progress);
                SetState(UpdateStatus.Downloading, $"Đang tải xuống: {progress}%", progress);
            });

            SetState(
                UpdateStatus.ReadyToRestart,
                $"Đã tải xong v{NewVersion}. Nhấn 'Khởi động lại' để hoàn tất cập nhật.",
                100);
            return true;
        }
        catch (Exception ex)
        {
            SetState(UpdateStatus.Error, $"Lỗi trong quá trình tải cập nhật: {ex.Message}");
            return false;
        }
        finally
        {
            _lock.Release();
        }
    }

    public void RestartAppToApplyUpdate()
    {
        if (_pendingUpdate == null)
        {
            return;
        }

        try
        {
            var mgr = GetManager();
            mgr.ApplyUpdatesAndRestart(_pendingUpdate);
        }
        catch (Exception ex)
        {
            SetState(UpdateStatus.Error, $"Không thể khởi động lại để cài đặt: {ex.Message}");
        }
    }
}

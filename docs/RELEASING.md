# Releasing VietFlow

Releases are published only for a feature milestone, a breaking change or a critical security fix. Batch small fixes into the next milestone.

## Versioning

`VERSION` and `CHANGELOG.md` are the single source of truth. `build.ps1` syncs the `Cargo.toml` version, and `build.rs` fails when `VERSION` and `Cargo.toml` disagree.

1. Bump `VERSION`.
2. Add a section for that version to `CHANGELOG.md`.
3. Run `.\scripts\build.ps1 -Arch All -Msi`.

The build writes:

- `dist\VietFlow.exe` (x64)
- `dist\VietFlow-x86.exe` (x86)
- `dist\VietFlow-arm64.exe` (ARM64)
- `dist\latest.json`
- `dist\version.txt`
- `dist\CHANGELOG.md`
- `releases\VietFlow-win-x64.msi`
- `releases\VietFlow-win-x86.msi`
- `releases\VietFlow-win-arm64.msi`

## GitHub release

1. Create and publish a release tagged `v<VERSION>`.
2. Upload the x64/x86/ARM64 portable executables and native WiX MSI installers.
3. Upload `latest.json`, `version.txt` and `CHANGELOG.md` as release assets.

The updater reads `releases/latest/download/latest.json`, then `version.txt`, then the GitHub API. `latest.json.downloads.msi_<arch>` points at the matching MSI installer for installed copies, while `latest.json.downloads.<arch>` points at portable binaries. For Portable users, the update checker directly opens the release page for manual download without touching any MSI. For MSI installations, the updater downloads and launches the MSI installer directly to perform a clean upgrade. `latest.json.notes` is the changelog shown in the update dialog.

`scripts/publish-release.ps1` creates the release and uploads the files using the stored Git credential.

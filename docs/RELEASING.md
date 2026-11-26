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

`scripts/publish-release.ps1` (via `scripts/publish_github_release.ps1`) enforces a clean git working tree, `HEAD == origin/main`, and provenance verification across all 3 architectures (`x64`, `x86`, `arm64`).

1. Commit and push `VERSION`, `Cargo.toml`, and `CHANGELOG.md` to `origin/main` first so GitHub Actions builds the matching commit (including `arm64`).
2. Package all 3 architectures into `releases\`:
   - If ARM64 MSVC tools are installed locally: run `.\scripts\build.ps1 -Arch All -Msi`.
   - Otherwise, build `x64` and `x86` locally with `-Msi`, download `VietFlow-arm64` (`VietFlow-arm64.exe` and `vietflow_tsf_arm64.dll`) from the latest `build` workflow run on that commit via `gh run download <run-id> -n VietFlow-arm64`, build `VietFlow-win-arm64.msi` with `.\installer\wix\build_msi.ps1 -Arch arm64`, create `VietFlow-win-arm64-Portable.zip`, ensure `releases\latest.json` lists all 6 download URLs (`x64`, `msi_x64`, `x86`, `msi_x86`, `arm64`, `msi_arm64`), and run:
     ```pwsh
     .\scripts\verify_release_artifacts.ps1 -Arch All -DistDir <dist> -ReleaseDir releases -RequireRelease
     ```
3. Run offline preflight, then publish to GitHub Releases:
   ```pwsh
   .\scripts\publish-release.ps1
   .\scripts\publish-release.ps1 -Publish
   ```

The updater reads `releases/latest/download/latest.json`, then `version.txt`, then the GitHub API. `latest.json.downloads.msi_<arch>` points at the matching MSI installer for installed copies, while `latest.json.downloads.<arch>` points at portable binaries. For Portable users, the update checker directly opens the release page for manual download without touching any MSI. For MSI installations, the updater checks the GitHub Releases API SHA-256 digest and Windows Authenticode signature before launching the MSI installer. `latest.json.notes` is the changelog shown in the update dialog.

## WinGet (`microsoft/winget-pkgs`)

After the GitHub release is published, generate, validate, and submit the WinGet manifest (`KienPC1234.VietFlow`) from the 3 MSIs in `releases\`:

```pwsh
# Generate and validate manifests in installer/winget/KienPC1234.VietFlow/<VERSION>/
.\scripts\update_winget_manifest.ps1

# Generate, validate, and submit a PR to microsoft/winget-pkgs
.\scripts\update_winget_manifest.ps1 -Update -Submit
```

Prerequisites: `winget` CLI, `Microsoft.WingetCreate` (`wingetcreate`), and an authenticated `gh` CLI session (or cached `wingetcreate token --store`).

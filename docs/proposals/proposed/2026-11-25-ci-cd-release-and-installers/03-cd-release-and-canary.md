# 03: Cross-Platform Packaging, Tag Releases, and Rolling Canary Pre-Release

## Scope

Create the reusable 8-target build/packaging workflow and the CD workflow handling:
- CI Artifact uploads for every build (both tag and non-tag runs)
- Stable/versioned GitHub Releases on `v*` tags with collected Git history
- Rolling `canary` pre-release overwritten on every `main` push

## Inputs

- Cross-platform ready crate from `01-cross-platform-build`.
- CI checks from `02-ci-workflow`.

## Implementation details

1. **Reusable packaging workflow (`.github/workflows/build-packages.yml`)**:
   - Accepts `version_label` (e.g. `v0.1.0`, `canary`, or `ci-<short_sha>`).
   - Matrix (8 targets):
     - `ubuntu-latest`: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` (`rust-lld`, static-pie), `aarch64-linux-android`, `x86_64-linux-android` (resolved Android NDK API 24 `clang` linker).
     - `macos-15`: `aarch64-apple-darwin`, `x86_64-apple-darwin`.
     - `windows-latest`: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`.
   - Archive convention:
     - Unix targets (Linux, macOS, Android): `agy-statusline-<version_label>-<target>.tar.gz` containing `agy-statusline` (`chmod 755`) and `README.md`.
     - Windows targets: `agy-statusline-<version_label>-<target>.zip` containing `agy-statusline.exe` and `README.md`.
   - Combine all target archives in a consolidation job, generate `SHA256SUMS`, and upload the combined bundle as a GitHub Actions **CI Artifact** (`agy-statusline-<version_label>`).
2. **Release / CD workflow (`.github/workflows/release.yml`)**:
   - Permissions: `contents: write`.
   - Concurrency group per ref (`release-canary` on `main` with `cancel-in-progress: true`; non-cancelling per tag on `v*`).
   - Triggers:
     - `push.tags: ['v*']` (explicitly excludes `canary` to prevent self-trigger loops)
     - `push.branches: ['main']`
     - `workflow_dispatch`
   - Gates on lint + multi-OS test jobs before publishing.
   - **On `v*` tag**:
     - Verify `v<Cargo.toml version>` matches tag name (fail fast on drift).
     - Collect Git commit history from the previous `v*` tag (`git describe --tags --abbrev=0 --match 'v*' HEAD^` if present, else root commit) into clean Markdown release notes grouped or listed with commit hashes and subjects.
     - Publish GitHub Release via `gh release create` / `gh release edit` with all 8 archives and `SHA256SUMS` (marking `prerelease` automatically if the tag contains `-` such as `v0.2.0-rc.1`).
   - **On `main` push**:
     - Force-move the `canary` tag to `GITHUB_SHA`.
     - Create or overwrite the single `canary` GitHub pre-release (`--prerelease`, never marked `latest`), deleting stale assets if filenames changed, uploading all 8 `agy-statusline-canary-<target>.*` archives + `SHA256SUMS`, and writing commit SHA + commit history since the latest `v*` tag into the pre-release notes.

## Non-goals

- Publishing to crates.io, Homebrew taps, Winget, or Termux APT repositories (deferred to later proposals).

## Done condition

- Pushing `main` uploads CI Artifacts and refreshes the `canary` pre-release with all 8 archives + `SHA256SUMS`.
- Pushing a `v*` tag uploads CI Artifacts and publishes a GitHub Release with all 8 archives, `SHA256SUMS`, and collected Git history.

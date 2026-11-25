# Tag-Driven Releases, Cross-Platform CI/CD, and One-Line Installers

Accepted: 2026-11-25

## Problem

`agy-statusline` currently builds and installs only via manual local commands (`just install` on macOS or ad-hoc cross-compilation pushed over SSH). With `https://github.com/kmou424/agy-statusline.git` now serving as the primary upstream, users and multi-machine workflows (Linux workstations/devboxes, macOS, Windows, and Android Termux) lack:

1. Automated CI verification across operating systems on pull requests and `main` pushes using the pinned Rust toolchain (`1.100.0`).
2. Automated cross-platform build and packaging for common desktop and mobile/Termux targets with OS-idiomatic archive formats and checksums.
3. Automated GitHub Releases on `v*` tags (with collected Git commit history) plus a continuously updated `canary` pre-release channel tracking `main`, while preserving packaged binaries as CI artifacts on every workflow run.
4. One-command installer scripts for Unix (`sh`/`bash` across Linux, macOS, and Android Termux) and Windows (PowerShell) that install into the platform's standard per-user executable directory, verify checksums, configure `~/.gemini/antigravity-cli/settings.json`, and support switching between latest stable release and `--canary`.

Additionally, inspecting `src/lib.rs:27` and `src/theme.rs:64` shows that home-directory resolution currently reads only `HOME`, which prevents `~` path shortening and `colorScheme` settings detection from working natively on Windows (`USERPROFILE`).

## Proposal

Deliver automated CI/CD, cross-platform packaging, and companion installer scripts in four ordered slices targeting `https://github.com/kmou424/agy-statusline`:

### 1. Cross-platform runtime readiness and build targets (`01-cross-platform-build`)

- Add a small cross-platform home-directory helper (`HOME` on Unix; `USERPROFILE` with `HOME` fallback on Windows) used by `src/lib.rs` and `src/theme.rs`, and ensure `~` path shortening handles Windows path separators cleanly.
- Add release profile tuning in `Cargo.toml` (`lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`) for compact, fast-starting release binaries.
- Define the v1 supported release target matrix and OS-idiomatic archive formats:

| OS / Environment | Rust Target | Archive Format | Build Runner / Linker |
|---|---|---|---|
| Linux x86_64 | `x86_64-unknown-linux-musl` | `.tar.gz` | `ubuntu-latest` (`rust-lld`, static-pie) |
| Linux ARM64 | `aarch64-unknown-linux-musl` | `.tar.gz` | `ubuntu-latest` (`rust-lld`, static-pie) |
| macOS Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` | `macos-15` (Apple `ld`) |
| macOS Intel | `x86_64-apple-darwin` | `.tar.gz` | `macos-15` (cross-target, Apple `ld`) |
| Android (Termux) ARM64 | `aarch64-linux-android` | `.tar.gz` | `ubuntu-latest` (Android NDK API 24 `clang`) |
| Android (Termux) x86_64 | `x86_64-linux-android` | `.tar.gz` | `ubuntu-latest` (Android NDK API 24 `clang`) |
| Windows x86_64 | `x86_64-pc-windows-msvc` | `.zip` | `windows-latest` (MSVC) |
| Windows ARM64 | `aarch64-pc-windows-msvc` | `.zip` | `windows-latest` (cross-target, MSVC) |

- Keep `justfile` aligned with optional local cross-build helpers while preserving existing local recipes.

### 2. Automated CI without Blaze (`02-ci-workflow`)

- Add `.github/workflows/ci.yml` triggered on `push` (branches including `main`), `pull_request`, and `workflow_dispatch`.
- Run exclusively on the pinned Rust toolchain from `rust-toolchain.toml` (`1.100.0`) with standard `cargo` — intentionally **not** using `blaze`, because ephemeral CI runners cannot reuse Blaze's in-memory daemon state and pinned `cargo` is the authoritative compiler check.
- Jobs:
  - `lint`: `cargo fmt --all --check` and `cargo clippy --all-targets --all-features -- -D warnings`.
  - `test`: native test matrix on `ubuntu-latest` (installing `clang` + `mold` to verify `.cargo/config.toml`), `macos-15`, and `windows-latest` running `cargo test --locked`.
  - `package-artifacts` (shared reusable workflow): builds the full target matrix on non-tag runs and uploads every packaged archive plus `SHA256SUMS` to GitHub Actions CI Artifacts (`actions/upload-artifact`).

### 3. Tag-based Releases, rolling `canary` pre-release, and CI Artifacts (`03-cd-release-and-canary`)

- Add a reusable build/package workflow (`.github/workflows/build-packages.yml`) and a CD workflow (`.github/workflows/release.yml`) triggered on:
  - `push` to `v*` tags (stable/versioned releases)
  - `push` to `main` (rolling `canary` pre-release)
  - `workflow_dispatch`
- Every run (both `v*` tags and `main` / non-tag builds) runs the CI test gate first, builds the full 8-target matrix, generates a unified `SHA256SUMS` manifest, and uploads all archives + `SHA256SUMS` as **CI Artifacts**.
- **Versioned tag (`v*`) behavior**:
  - Verify tag matches `v<Cargo.toml version>` (unless prerelease suffix is explicitly allowed, or require exact match with `Cargo.toml` version).
  - Archive naming: `agy-statusline-<tag>-<target>.<tar.gz|zip>`.
  - Collect Git commit history since the previous `v*` tag (`git log <prev_tag>..HEAD` formatted into categorized/bulleted release notes, falling back to full history on the first tag) and publish a GitHub Release with all archives and `SHA256SUMS`.
- **Rolling `canary` pre-release (`main` push) behavior**:
  - Archive naming: `agy-statusline-canary-<target>.<tar.gz|zip>` (deterministic filenames for the installer).
  - Force-update a dedicated `canary` Git tag to `main`'s tested commit SHA and overwrite a single GitHub pre-release (`prerelease: true`, `latest: false`) with fresh archives, `SHA256SUMS`, commit SHA, and recent commit log since the last stable `v*` tag.

### 4. Companion one-line installer scripts (`04-installer-scripts`)

- **Unix installer (`scripts/install.sh`)**:
  - Strict POSIX `sh` syntax compatible with both `sh` and `bash` (verified with `shellcheck -s sh` and tested under both shells).
  - Supports Linux (`x86_64`, `aarch64`), macOS (`arm64`/`aarch64`, `x86_64`), and Android Termux (`aarch64`, `x86_64`; detected via `uname -o`, `$TERMUX_VERSION`, or Android system markers).
  - Default install path: `~/.local/bin/agy-statusline` (overridable via `--install-dir`).
  - Default channel: latest stable GitHub Release; `--canary` installs from the rolling `canary` pre-release; `--version <tag>` installs a pinned tag.
  - Downloads archive + `SHA256SUMS` (`curl` or `wget`), verifies SHA-256 before replacing the binary atomically, and updates `~/.gemini/antigravity-cli/settings.json` (`statusLine.type = "command"`, `statusLine.command = "~/.local/bin/agy-statusline"`, `statusLine.enabled = true`, preserving all existing keys; opt-out via `--no-configure`, optional `--stack-with-default`).
- **Windows installer (`scripts/install.ps1`)**:
  - PowerShell 5.1+ / PowerShell 7+ (`pwsh`), supporting `x86_64` and `ARM64`.
  - Default install path: `$env:LOCALAPPDATA\Programs\agy-statusline\agy-statusline.exe` (Windows' standard per-user executable directory analogous to `~/.local/bin`, requiring no admin elevation; overridable via `-InstallDir`).
  - Default channel: latest stable GitHub Release; supports `-Canary` (and `--canary` alias handling) for the rolling `canary` pre-release, plus `-Version`, `-NoConfigure`, `-StackWithDefault`.
  - Verifies SHA-256 via `Get-FileHash`, extracts `.zip` via `Expand-Archive`, installs atomically, and safely merges `statusLine` into `$env:USERPROFILE\.gemini\antigravity-cli\settings.json` via built-in `ConvertFrom-Json`/`ConvertTo-Json` (quoting the command path if it contains spaces for `cmd /c`).

## Alternatives

1. **Use `blaze` inside GitHub Actions CI**
   - Rejected because ephemeral runners start cold on every job, eliminating Blaze's in-memory incremental daemon advantage while introducing an extra toolchain layer that diverges from the pinned Rust `1.100.0` compiler.
2. **Ship glibc-linked Linux binaries (`*-unknown-linux-gnu`) instead of static-pie `musl`**
   - Rejected because `agy-statusline` is pure Rust with zero dynamic C library dependencies; glibc-linked binaries break on older Linux distributions (GLIBC version mismatches) and Alpine/musl hosts, whereas static-pie `musl` runs across all Linux distributions.
3. **Reuse `aarch64-unknown-linux-musl` for Android Termux instead of native `aarch64-linux-android`**
   - Rejected because GitHub's `ubuntu-latest` runner already ships the Android NDK, allowing native Android API 24 bionic builds (`aarch64-linux-android` / `x86_64-linux-android`) with zero Docker overhead and full Termux compatibility.
4. **Use a third-party release framework (`cargo-dist` / `release-plz`)**
   - Rejected for now to avoid opaque generated workflows and extra tooling lock-in; our single-binary crate with 8 targets is simpler and more transparent with focused GitHub Actions workflows and self-contained installer scripts.
5. **Install on Windows to `%USERPROFILE%\.local\bin` instead of `%LOCALAPPDATA%\Programs\agy-statusline`**
   - Rejected as default because `%LOCALAPPDATA%\Programs\<app>` is Windows' established per-user installation convention (matching VS Code, Python, and WinGet user installs), while `-InstallDir` still lets power users choose `%USERPROFILE%\.local\bin` if preferred.

## Acceptance criteria

- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --locked` pass in GitHub Actions across Linux (with `mold`), macOS, and Windows, including new tests verifying Windows `USERPROFILE` home resolution.
- The packaging workflow builds and uploads valid archives + `SHA256SUMS` as CI Artifacts for all 8 targets (`.tar.gz` for Linux/macOS/Android, `.zip` for Windows) on both non-tag and tag runs.
- Pushing to `main` updates the single rolling `canary` pre-release in place with deterministic `agy-statusline-canary-<target>.*` assets and `SHA256SUMS`.
- Pushing a `v*` tag publishes a GitHub Release containing versioned archives, `SHA256SUMS`, and collected Git commit history since the previous `v*` tag.
- `scripts/install.sh` passes `shellcheck -s sh` (and syntax checks under both `sh -n` and `bash -n`), correctly detects OS/arch (including Termux), verifies SHA-256, installs to `~/.local/bin/agy-statusline`, and non-destructively updates `settings.json` for both default (stable) and `--canary` modes (verified with local mock/fixture tests).
- `scripts/install.ps1` passes syntax/unit verification on Windows CI, verifies SHA-256, installs to `$env:LOCALAPPDATA\Programs\agy-statusline\agy-statusline.exe`, and safely merges `statusLine` into `settings.json` for both stable and `-Canary` modes.
- `README.md` documents the one-line install commands (stable and `--canary`) for Unix/Termux and Windows.

## Risks

- **Android NDK path changes on GitHub runners**: `$ANDROID_NDK_LATEST_HOME` is preinstalled on `ubuntu-latest`, but runner image updates can shift its directory name; the workflow should resolve the NDK path defensively (checking `ANDROID_NDK_LATEST_HOME`, `ANDROID_NDK_HOME`, and `/usr/local/lib/android/sdk/ndk/*`).
- **Windows `cmd /c` quoting in `agy` `statusLine.command`**: If `%LOCALAPPDATA%` contains spaces (e.g., user names with spaces), unquoted paths break under `cmd /c`; `scripts/install.ps1` must quote paths containing whitespace cleanly and test that case.
- **Settings JSON editing on minimal Unix/Termux hosts**: `scripts/install.sh` cannot assume `jq` is always installed; supporting `python3`, `jq`, and `node` in priority order (and failing safely without corrupting an existing `settings.json` if none exist) avoids data loss while covering almost all developer and Termux hosts.
- **Rolling `canary` tag force-updates**: Moving the `canary` tag on every `main` push triggers tag-ref updates; `release.yml` must filter `tags: ['v*']` (excluding `canary`) so updating `canary` never re-triggers the workflow in a loop.

## Checklist

- [ ] `01-cross-platform-build`: Cross-platform home resolution, release profile optimization, and local cross-build alignment
- [ ] `02-ci-workflow`: GitHub Actions CI workflow (lint, Linux/macOS/Windows tests without Blaze, CI artifact uploads)
- [ ] `03-cd-release-and-canary`: Reusable 8-target packaging matrix, `v*` tag Release with Git history collection, and rolling `canary` pre-release on `main`
- [ ] `04-installer-scripts`: POSIX `sh`/`bash` installer (`scripts/install.sh`), Windows PowerShell installer (`scripts/install.ps1`), automated installer tests, and `README.md` updates

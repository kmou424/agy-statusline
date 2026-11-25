# 04: Companion One-Line Installer Scripts (Unix/Termux & Windows)

## Scope

Create standalone installer scripts for Unix (`scripts/install.sh`) and Windows (`scripts/install.ps1`) that download, verify, install, and wire `agy-statusline` into Antigravity CLI (`agy`) `settings.json`, supporting both latest stable release and `--canary`.

## Inputs

- Release asset naming and `SHA256SUMS` contract from `03-cd-release-and-canary`.
- Antigravity CLI `statusLine` schema from `docs/research/antigravity-statusline/2026-11-25-agy-statusline-contract.md`.

## Implementation details

1. **Unix installer (`scripts/install.sh`) — POSIX `sh` + `bash` compatible**:
   - Shebang `#!/bin/sh` with `set -eu` (strictly POSIX — no bash-only arrays, `[[`, or `pipefail` — works identically via `curl -fsSL ... | sh` and `curl -fsSL ... | bash`).
   - Platform detection:
     - Android / Termux: detect via `uname -o` matching `*Android*`, non-empty `$TERMUX_VERSION`, or `/system/bin/app_process`; map `aarch64`/`arm64` -> `aarch64-linux-android`, `x86_64`/`amd64` -> `x86_64-linux-android`.
     - macOS (`Darwin`): detect Apple Silicon (`arm64`/`aarch64`, including Rosetta `sysctl.proc_translated == 1`) -> `aarch64-apple-darwin`; `x86_64` -> `x86_64-apple-darwin`.
     - Linux: `x86_64`/`amd64` -> `x86_64-unknown-linux-musl`; `aarch64`/`arm64` -> `aarch64-unknown-linux-musl`.
   - CLI options:
     - `--canary` (download from `releases/download/canary/agy-statusline-canary-<target>.tar.gz` + `SHA256SUMS`)
     - `--version <tag>` (download explicit tag `vX.Y.Z`)
     - Default (no channel flag): resolve latest stable tag from GitHub Releases and download `agy-statusline-<tag>-<target>.tar.gz` + `SHA256SUMS`
     - `--install-dir <dir>` (default: `$HOME/.local/bin`)
     - `--no-configure` (skip editing `settings.json`)
     - `--stack-with-default` (set `stack_with_default: true`)
     - `--help` / `-h`
   - Download & verification: support `curl` and `wget`, verify SHA-256 via `sha256sum` / `shasum -a 256` / `openssl dgst -sha256`, extract in `mktemp -d` with `trap` cleanup, install atomically (`chmod 755` + `mv`).
   - `settings.json` update (`$HOME/.gemini/antigravity-cli/settings.json`):
     - Use `"~/.local/bin/agy-statusline"` when installed to the default `$HOME/.local/bin`, or the explicit installed path when `--install-dir` is customized.
     - Merge non-destructively via `python3`, `jq`, or `node` (whichever is available), writing atomically via temp file + `mv`. If `settings.json` does not exist yet, create a minimal valid JSON object. If `settings.json` exists and none of `python3`/`jq`/`node` is available, leave it untouched and print the exact `/statusline <path>` command.
2. **Windows installer (`scripts/install.ps1`)**:
   - Parameters: `[switch]$Canary`, `[string]$Version`, `[string]$InstallDir = "$env:LOCALAPPDATA\Programs\agy-statusline"`, `[switch]$NoConfigure`, `[switch]$StackWithDefault` (plus graceful handling if invoked with `--canary` in raw `$args`).
   - Architecture detection: `AMD64` -> `x86_64-pc-windows-msvc`, `ARM64` -> `aarch64-pc-windows-msvc`.
   - Downloads `.zip` + `SHA256SUMS`, verifies SHA-256 via `Get-FileHash -Algorithm SHA256`, extracts to temp folder, installs `agy-statusline.exe` atomically.
   - Merges `statusLine` into `$env:USERPROFILE\.gemini\antigravity-cli\settings.json` via built-in `ConvertFrom-Json` / `ConvertTo-Json -Depth 10` (preserving existing keys, quoting the `command` string if the path contains spaces for `cmd /c`).
3. **Verification & documentation**:
   - Add automated tests for `scripts/install.sh` (using local fixture archive + `SHA256SUMS` + isolated temp `HOME`, verifying both `sh` and `bash`, `--canary` URL resolution, checksum mismatch rejection, and non-destructive `settings.json` merging) and PowerShell syntax/config-merge tests on Windows CI.
   - Update `README.md` with one-line install commands (stable and `--canary`) for Linux/macOS/Termux and Windows.

## Non-goals

- Interactive TUI installer menus or uninstalling user-customized `settings.json` keys outside `statusLine`.

## Done condition

- `scripts/install.sh` passes `shellcheck -s sh`, `sh -n`, `bash -n`, and local fixture end-to-end tests.
- `scripts/install.ps1` passes Windows CI validation and fixture tests.
- `README.md` documents stable and `--canary` install commands for Unix/Termux and Windows.

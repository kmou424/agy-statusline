# 02: Automated CI Workflow (Cargo-Only, Multi-OS)

## Scope

Create `.github/workflows/ci.yml` to run formatting, Clippy, and multi-OS Cargo tests on the pinned Rust toolchain (`1.100.0`), without using `blaze`.

## Inputs

- `rust-toolchain.toml`, `Cargo.toml`, `Cargo.lock`, `.cargo/config.toml`.
- Reusable packaging workflow from `03-cd-release-and-canary` for uploading non-tag CI artifacts.

## Implementation details

1. **Triggers**:
   - `push` to `main` (and feature branches if desired; avoid duplicating runs with `release.yml` via clear job separation)
   - `pull_request` targeting `main`
   - `workflow_dispatch`
2. **Toolchain & caching**:
   - Install Rust using `dtolnay/rust-toolchain` (reading `rust-toolchain.toml`, `1.100.0`) and `Swatinem/rust-cache` for Cargo registry/target caching.
   - Explicitly do **not** install or run `blaze` in CI.
3. **Jobs**:
   - `lint` (`ubuntu-latest`): `cargo fmt --all --check` and `cargo clippy --all-targets --all-features -- -D warnings`.
   - `test` matrix (`ubuntu-latest`, `macos-15`, `windows-latest`):
     - On `ubuntu-latest`, install `clang` and `mold` so `.cargo/config.toml` GNU Linux linking is exercised in CI.
     - Run `cargo test --locked`.
   - `shell-lint` (`ubuntu-latest`): run `shellcheck -s sh scripts/install.sh`, `sh -n scripts/install.sh`, and `bash -n scripts/install.sh` (wired once `04-installer-scripts` lands).
   - Call the shared packaging workflow on PRs / non-tag pushes so built archives are uploaded as CI Artifacts (e.g. 14-day retention) for manual testing before tagging.

## Non-goals

- Publishing GitHub Releases or modifying tags (handled exclusively by `03-cd-release-and-canary`).

## Done condition

- Workflow YAML validates cleanly and passes end-to-end on GitHub Actions across Linux, macOS, and Windows runners.

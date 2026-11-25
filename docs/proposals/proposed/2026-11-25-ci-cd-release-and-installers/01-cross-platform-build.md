# 01: Cross-Platform Runtime Readiness and Release Build Profile

## Scope

Prepare the Rust crate for clean compilation, testing, and compact binary packaging across Linux (`musl`), macOS (`darwin`), Android Termux (`android`), and Windows (`msvc`).

## Inputs

- Existing crate code in `src/lib.rs`, `src/theme.rs`, `src/layout.rs`, `Cargo.toml`, `.cargo/config.toml`, `justfile`, and `tests/`.

## Implementation details

1. **Cross-platform home directory helper**:
   - Add a shared helper (e.g. `home_dir() -> Option<PathBuf>` / `Option<String>`) used by both `src/lib.rs` and `src/theme.rs`:
     - On Unix (`#[cfg(not(windows))]`): read non-empty `HOME`.
     - On Windows (`#[cfg(windows)]`): prefer non-empty `USERPROFILE`, falling back to non-empty `HOME` (supports Git Bash / MSYS2 / PowerShell alike).
   - Ensure `src/layout.rs::cwd_part` renders `~` and `~/...` cleanly even when Windows paths use `\` separators (normalizing `\` to `/` after `~/` or preserving native separator consistently, covered by unit tests).
2. **Release profile tuning (`Cargo.toml`)**:
   - Add `[profile.release]` with `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"` to minimize release binary size while keeping link times fast.
3. **Target linker config safety (`.cargo/config.toml`)**:
   - Ensure `[target.'cfg(target_os = "linux")']` does not collide with `musl` cross-linking (`rust-lld`) or `android` NDK linking: scope `clang` + `-fuse-ld=mold` specifically to GNU Linux ELF targets (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`) or `cfg(all(target_os = "linux", target_env = "gnu"))`, leaving `musl` targets to `rust-lld` and `android` targets to the NDK linker!

## Non-goals

- Adding Windows-specific probe daemons or new status line visual segments.
- Changing existing local `just ci` / `just install` behavior on macOS or Linux.

## Done condition

- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --locked` pass cleanly with unit tests covering Windows-style home resolution and path shortening.
- `.cargo/config.toml` allows `cargo build --release --target x86_64-unknown-linux-musl` without requiring manual `RUSTFLAGS` overrides.

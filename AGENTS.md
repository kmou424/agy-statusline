# AGENTS.md

Documentation follows the docs-engineering skill; [docs/README.md](./docs/README.md) maps where each kind of document lives.

## Toolchain and commands

- Rust toolchain and components are pinned in [rust-toolchain.toml](./rust-toolchain.toml) (`1.100.0`, `edition = "2024"` in [Cargo.toml](./Cargo.toml)).
- Linker configuration lives in [.cargo/config.toml](./.cargo/config.toml) (`clang` + `mold` on Linux ELF targets; default system `ld` on macOS).
- Build and test recipes live in [justfile](./justfile):
  - `just test` — fast incremental test loop via `blaze test`
  - `just check` — fast workspace check via `blaze check`
  - `just verify` — formatting check, Clippy, and `blaze test`
  - `just ci` — formatting check, Clippy, and `cargo test` on the pinned toolchain
  - `just build-release` — release build via `cargo build --release`

## Workspace conventions

- Ephemeral scratch files go under `.scratch/` (Git-ignored) and are never cited from project documentation.
- Non-trivial feature work starts as a proposal under `docs/proposals/proposed/` and waits for owner approval (`Accepted: YYYY-MM-DD`) before implementation.
- Never persist or log sensitive fields from `agy` `stdin` payloads (`email`, `transcript_path`, workspace paths, conversation titles).

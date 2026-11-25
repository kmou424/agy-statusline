# agy-statusline

Custom status line renderer for Antigravity CLI (`agy`), written in Rust.

## Prerequisites

- Rust toolchain pinned in [rust-toolchain.toml](./rust-toolchain.toml) (`1.100.0`, with `rustfmt` and `clippy`)
- [`just`](https://github.com/casey/just) command runner
- [`blaze`](https://github.com/indexable-inc/blaze) for fast incremental local tests and checks
- `clang` and [`mold`](https://github.com/rui314/mold) on Linux ELF targets (configured in [.cargo/config.toml](./.cargo/config.toml))

## Common commands

Run `just` to list all recipes defined in [justfile](./justfile):

```bash
just test           # Fast tests via blaze test
just check          # Fast check via blaze check
just verify         # fmt-check + clippy + blaze test
just ci             # fmt-check + clippy + cargo test (pinned toolchain)
just build-release  # Optimized release binary via cargo
```

## Documentation

Project documentation follows the `docs-engineering` layout mapped in [docs/README.md](./docs/README.md).

# agy-statusline

Custom status line renderer for Antigravity CLI (`agy`), written in Rust.

Reads `agy`'s `statusLine` JSON payload from `stdin` and renders a `wow-pi`-aligned two-row status line (working directory, Git branch/status, local runtime/framework, 12-cell context bar, model/effort identity, active `agy` mode badges, and token/cache/cost usage) with terminal-width adaptation and `agy` `colorScheme`-aware ANSI styling.

## Prerequisites for local development

- Rust toolchain pinned in [rust-toolchain.toml](./rust-toolchain.toml) (`1.100.0`, with `rustfmt` and `clippy`)
- [`just`](https://github.com/casey/just) command runner
- [`blaze`](https://github.com/indexable-inc/blaze) for fast incremental local checks/tests
- `clang` and [`mold`](https://github.com/rui314/mold) on Linux ELF targets (configured in [.cargo/config.toml](./.cargo/config.toml))

## Install and use with `agy`

**Linux, macOS, and Android (Termux)**

```sh
curl -fsSL https://raw.githubusercontent.com/kmou424/agy-statusline/main/scripts/install.sh | sh
```

Canary:

```sh
curl -fsSL https://raw.githubusercontent.com/kmou424/agy-statusline/main/scripts/install.sh | sh -s -- --canary
```

**Windows (PowerShell)**

```powershell
irm https://raw.githubusercontent.com/kmou424/agy-statusline/main/scripts/install.ps1 | iex
```

Canary:

```powershell
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/kmou424/agy-statusline/main/scripts/install.ps1))) -Canary
```

The installers configure `agy` automatically. Use `--no-configure` or `-NoConfigure` to install just the binary.

### Build and install locally

Build and install the release binary into `~/.local/bin/agy-statusline`:

```bash
just install
```

Register the binary in Antigravity CLI via `/statusline`:

```text
/statusline ~/.local/bin/agy-statusline
```

Or configure `~/.gemini/antigravity-cli/settings.json` directly:

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.local/bin/agy-statusline",
    "enabled": true
  }
}
```

## Common commands

Run `just` to list all recipes defined in [justfile](./justfile):

```bash
just ci             # fmt-check + clippy + cargo test (pinned toolchain)
just verify         # fmt-check + clippy + blaze test
just test           # Fast tests via blaze test
just check          # Fast check via blaze check
just build-release  # Optimized release binary via cargo
just install        # Build release binary and install to ~/.local/bin/agy-statusline
```

## Documentation

Project documentation follows the `docs-engineering` layout mapped in [docs/README.md](./docs/README.md).

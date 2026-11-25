# `wow-pi`-Aligned Footer Prototype for Antigravity CLI (`agy`)

Accepted: 2026-11-25

## Problem

Antigravity CLI (`agy 1.5.16`) supports replacing or stacking its built-in TUI status line via a `statusLine` shell command that reads a JSON payload from `stdin` and prints ANSI-formatted lines to `stdout` (see `2026-11-25-agy-statusline-contract`). However, `agy`'s built-in status line does not provide the dense, width-adaptive two-row developer footer style from `wow-pi` (workspace path, Git status summary, project runtime/framework, 12-cell context bar, model and effort identity, and token/cache/cost counters; see `2026-11-25-wow-pi-footer-reference`).

Building this renderer for `agy` must work within strict runtime constraints and contract gaps established in our research:

- `agy` runs the status line command as a fresh process on agent-state changes (up to once every 300 ms, 5 s timeout, 30-consecutive-failure session circuit breaker).
- `stdin` guarantees `workspace`, `cwd`, `conversation_title`, `model`, `context_window`, `cost`, `terminal_width`, `vcs.type`, and `agy`-native state fields (`agent_state`, `cycle_mode`, `sandbox`, `quota`, `subagents`, `task_count`, `artifact_count`, `pending_input_count`, `tool_confirmation_pending`, `vim`), while Git branch/status/stash and local runtime/framework metadata must be derived locally.
- Several `wow-pi` footer elements have no `agy` equivalent or differ in accounting: provider display name, Pi-specific badges, extension statuses, captured `belowEditor` widgets, session `Σ` run/compaction/TPS/TTFT telemetry, exact occupied token count, and session-cumulative cache-hit totals.
- `agy` configures `colorScheme` in `~/.gemini/antigravity-cli/settings.json` (default `"terminal"`, plus seven named built-in themes), but does not expose a host theme RGB palette over `stdin`.

We need a fast, single-binary Rust prototype that reproduces `wow-pi`'s core two-row footer aesthetic and responsive behavior under `agy`'s real contract so we can visually inspect and iterate on it inside live `agy` sessions.

## Proposal

Implement an end-to-end v0 prototype of `agy-statusline` aligned with `wow-pi`'s Row 1 + Row 2 footer layout, responsive width rules, and glyph system, paired with an `agy`-aware semantic color layer.

### 1. Input contract and safety boundary

- Parse `stdin` JSON with forward-compatible deserialization (all optional/nullable fields tolerated, unknown fields and unknown enum variants ignored without failing).
- Sanitize untrusted display strings (`cwd`, `workspace.*`, `conversation_title`, Git branch/tag, `model.*`) by stripping VT/ANSI escapes and replacing `\r`, `\n`, `\t` with single spaces before width calculation or printing.
- Never write `stdin` payloads or sensitive fields (`email`, `transcript_path`) to disk, caches, or logs.
- Always exit `0` on recoverable input/probe errors (rendering degraded or empty output rather than non-zero exit codes) so transient issues never trip `agy`'s 30-failure circuit breaker.

### 2. Layout and segments (`wow-pi` Row 1 + Row 2 alignment)

Reproduce `wow-pi`'s pure layout architecture (`footer-layout.ts`) separated from data collection:

- **Row 1 (Left, priority order `0..4`)**:
  - Priority `0` — **Working directory**: prefer `workspace.current_dir`, fall back to `cwd`; shorten `$HOME` prefix to `~` / `~/...`; compact fallback is `basename`.
  - Priority `2` (optional, enabled when non-empty or configurable flag; default aligned with `wow-pi` unless toggled) — **Conversation title** from `conversation_title`.
  - Priority `3` — **Git group**: branch clipped to 20 display cells (detached renders `HEAD`), status counts in `[ ... ]` (`conflicted`, `deleted`, `modified`, `renamed`, `staged`, `untracked`, `stashed`, plus `ahead`/`behind`/`diverged`), derived from bounded local `git status --porcelain=v2 --branch --show-stash`. When `git` is unavailable or times out, fall back gracefully to any optional `vcs.branch` / `vcs.dirty` fields present in `stdin`, or omit cleanly.
  - Priority `4` — **Runtime / framework**: marker-file detection in the active directory (ordered matching `wow-pi`'s `runtime-catalog.ts`) with bounded version probe (e.g. `rustc --version`, `node --version`, `go version`, `python3 --version`, etc.) and framework marker detection.
- **Row 1 (Right)** — **Context usage**:
  - Rendered when `context_window.context_window_size > 0`: icon + 12-cell progress bar (`▰`/`▱` in Nerd mode, `#`/`-` in ASCII mode) + 1-decimal percentage (`used_percentage`) + `·` + compact `used/window` (`used` approximated as `round(used_percentage * context_window_size / 100)` because `agy` does not emit an exact occupied-token counter).
  - Stress thresholds match `wow-pi`: bar fill `< 50%` `success`, `>= 50%` `warning`, `>= 80%` `error`; icon & percent `< 70%` `accent`, `>= 70%` `warning`, `>= 90%` `error`.
  - Collapses to `<icon> <pct>` when full context + compact `cwd` exceeds `terminal_width`.
- **Row 2 (Left)** — **Model identity and compact mode badges**:
  - Model glyph + `model.display_name` (fallback `model.id`) + `[effort]` in `dim` when `model.effort` is present and not `"off"` (avoiding duplicate effort text if `display_name` already carries an effort suffix, or displaying verbatim—verify against live `agy` payloads during prototype testing).
  - Compact `agy`-native indicators appended as subtle `warning`/`accent` badges only when active (`cycle_mode` such as `plan`/`auto`/`accept-edits`, `sandbox.enabled`, `tool_confirmation_pending`), keeping `wow-pi`'s visual weight uncluttered.
- **Row 2 (Right)** — **Usage stats** joined by dim ` | `:
  - Cumulative input tokens (`context_window.total_input_tokens`) and output tokens (`context_window.total_output_tokens`) formatted in compact units (`K`, `M`, `B`).
  - Current-turn cache-hit rate computed from `context_window.current_usage` (`cache_read / (input + cache_read + cache_write)`) when prompt input > 0, styled by `wow-pi` thresholds (`< 30%` `error`, `< 70%` `warning`, `>= 70%` `success`).
  - Estimated cost (`cost.total_usd`, formatted `$0.000`) when `cost` is present.
- **Responsive width & truncation**:
  - Measure visible width in terminal display cells (ANSI-stripped, Unicode East Asian width aware).
  - Follow `wow-pi`'s drop/collapse rules for Row 1 (`runtime` -> `git` -> `title`, keeping `cwd`) and Row 2 (reserve compact model label + badges, truncate usage if needed, drop chrome before model name), finishing with a strict `truncate_to_width(line, width)` pass on every emitted line.
- **Icon mode**:
  - Match `wow-pi`'s `resolveIconMode("auto")` terminal environment heuristic (`HERDR_ENV`, `TERM`, `WT_SESSION`, `TERM_PROGRAM`, `LC_TERMINAL`), with Nerd glyph defaults and clean ASCII fallbacks.

### 3. Color strategy (follow `agy` when possible, custom semantic palette otherwise)

Decouple rendering from raw ANSI codes via a semantic `Palette` matching `wow-pi`'s token roles (`accent`, `md_link`, `text`, `muted`, `dim`, `success`, `warning`, `error`):

1. **Detect `agy` `colorScheme` (best-effort, read-only)**: inspect `colorScheme` in `~/.gemini/antigravity-cli/settings.json` (defaulting to `"terminal"` if missing, unreadable, or unset).
2. **Follow terminal / `agy` default (`"terminal"`)**: emit standard 16-color ANSI SGR codes (`30`–`37`, `90`–`97`, `dim`, `bold`) matching `agy`'s official `statusline.sh`. Because `agy`'s `"terminal"` scheme uses the host terminal palette, this follows `agy` and the user's terminal theme natively on both dark and light backgrounds.
3. **Named `agy` schemes without a published host palette**: for `agy`'s seven named schemes (`light`, `dark`, `solarized light`, `solarized dark`, `tokyo night`, `colorblind-friendly light`, `colorblind-friendly dark`), `agy` exposes no external color table. Provide curated built-in semantic palettes (standard ANSI or 256/truecolor tuned per scheme, with high-contrast adjustments for light and colorblind-friendly modes) so the status line harmonizes with `agy`'s active `colorScheme`, while keeping the `Palette` abstraction ready for user-defined palette overrides in a follow-up proposal.

### 4. Prototype scope and non-goals

In scope for the v0 prototype:
- Single Rust binary reading `stdin` and writing Row 1 + Row 2 to `stdout`.
- Core crates for JSON (`serde`, `serde_json`), Unicode display width (`unicode-width`), and clean modular layout (`input`, `theme`, `icons`, `probes`, `layout`).
- Bounded external probes (tight per-command timeout, e.g. <= 200 ms, skipped cleanly when not in a Git repo or no runtime marker matches).
- Comprehensive unit/fixture tests covering payload variations, width breakpoints, CJK/ANSI sanitization, icon modes, and theme palettes, plus live verification in `agy` via `/statusline` (including `stack_with_default: true` side-by-side comparison).

Explicit non-goals for this prototype (deferred to follow-up proposals after live validation):
- External user config file (`agy-statusline.toml`) and CLI flags beyond minimal smoke/debug switches.
- Persistent cache files or background watcher daemons for Git/runtime probes.
- Reconstructing unavailable `wow-pi` metrics (`Σ` runs/messages/compactions/TPS/TTFT, extension statuses, captured widgets) or parsing undocumented `transcript_path` JSONL files.
- Full `agy` `quota` / `subagents` / `vim` multi-row expansion (only minimal non-intrusive badges in Row 2 for v0).

## Alternatives

1. **Parse `transcript_path` JSONL to reconstruct `wow-pi`'s `Σ` summary row and cumulative cache-hit rate**
   - Rejected for the prototype because `transcript_path` is explicitly undocumented as a status line API (`2026-11-25-agy-statusline-contract`), its format may change without notice, and scanning growing transcript files on every 300 ms invocation risks timeouts and leaks session I/O latency into the TUI.
2. **Hardcode a single dark-mode RGB palette (like a fixed TrueColor theme)**
   - Rejected because `agy`'s default `colorScheme` is `"terminal"` and users run both light and dark terminals or switch `agy` built-in themes; defaulting `"terminal"` to standard 16-color SGR follows the host palette directly, while named `colorScheme` presets cover non-terminal themes.
3. **Run a background daemon or persistent disk cache for Git/runtime probes in v0**
   - Rejected for the prototype to keep installation a single binary invoked by `agy` and establish real baseline latency first. Bounded direct probes validate the UX quickly and tell us whether caching is actually needed.

## Acceptance criteria

- Given representative `agy` JSON fixtures (empty/startup state, typical active session from `sources/agy-statusline-doc.md`, null `model`/`current_usage`, unknown extra fields, unknown `agent_state` values), the binary exits `0` and produces sanitized two-row output without panics or raw control-sequence leakage.
- Width adaptation tests at `160`, `100`, `72`, `50`, and `30` columns verify `wow-pi`'s collapse order (Row 1 context collapse, `cwd` basename fallback, priority drop `runtime` -> `git` -> `title`; Row 2 model preservation and chrome drop) and guarantee every emitted line's visible terminal cell width is `<= terminal_width`, including CJK conversation titles and Nerd icons.
- Palette resolution maps `colorScheme = "terminal"` (or missing settings file) to standard 16-color ANSI output, and resolves each named `agy` `colorScheme` to its corresponding built-in semantic palette without failing on malformed settings files.
- Local Git and runtime probes return structured results inside a real Git/Rust workspace, omit cleanly outside a repo or without markers, and respect their timeout budget without failing the binary.
- `just verify` (`cargo fmt --check`, `cargo clippy`, `blaze test`) and `just ci` (`cargo test` on pinned Rust `1.100.0`) pass cleanly.
- Live manual smoke test in `agy` (via `/statusline` and `stack_with_default: true`) verifies rendered alignment, color harmony, width handling against actual `terminal_width`, and records any real-payload discrepancies (such as `model.display_name` vs `effort` duplication or `total_input_tokens` cache accounting).

## Risks

- **Uncertain usable columns vs `terminal_width`**: `agy` does not document whether horizontal padding/margins subtract columns from `terminal_width`. If live testing shows line wrapping at exact `terminal_width`, we will reserve a small column margin after observing it.
- **Per-invocation Git/runtime subprocess latency**: On large repositories or slow filesystems, spawning `git status` and a runtime version binary on every state change could approach the 300 ms throttle cadence. Tight timeouts prevent hitting the 5 s kill or 30-failure breaker, and real latency measurements from the prototype will guide a follow-up short-TTL cache if needed.
- **No host RGB table for named `agy` themes**: Because `agy` does not expose its internal theme colors, presets for named `colorScheme` values are approximations rather than exact host values and may need visual tuning during live checks.
- **Live payload accounting nuances**: Whether `context_window.total_input_tokens` already includes cached tokens and whether `model.display_name` always embeds the effort level (as in `"Gemini 4 Argon (Medium)"`) can only be confirmed against real `agy` payloads during prototype testing.

## Result

Shipped in `v0.1.0` (`bda659a`, `c7c767d`, `607f5c4`) and verified live in Antigravity CLI (`agy`) TUI by the owner on `2026-11-25`.

- **What shipped**: Single-binary Rust status line renderer (`src/input.rs`, `src/text.rs`, `src/icons.rs`, `src/theme.rs`, `src/probes.rs`, `src/layout.rs`, `src/lib.rs`, `src/main.rs`) with `wow-pi`-aligned Row 1 + Row 2 responsive layout, ANSI/VT sanitization (CSI, OSC, ST-terminated control strings, C1 CSI), PUA-aware terminal cell width, `agy` `colorScheme`-aware semantic palettes (standard 16-color SGR for `"terminal"`, curated TrueColor presets for named schemes), concurrent bounded Git/runtime probes (`<= 200 ms` / `<= 150 ms`, `256 KiB` stdout cap), `just install` targeting `~/.local/bin/agy-statusline`, and 31 passing unit/integration tests (`just ci`).
- **Deviations and notes**:
  - Model effort renders as `Model effort` without square brackets to match `wow-pi`'s actual `footer-layout.ts` output, suppressing duplicate effort text when already parenthesized in `model.display_name`.
  - `terminal_width` is deserialized as `i64` and clamped to `4096`; explicit `<= 0` produces empty output (matching `wow-pi`), while absent `terminal_width` defaults to `80`.
  - Unknown `cycle_mode` strings are ignored; only documented `plan`, `auto`, and `accept-edits` render badges.
  - `blaze test` (`0.20261105.4`) could not verify external crate dependencies against the local `rsproxy-sparse` Cargo mirror replacement (`the load asked for rsproxy-sparse index again after it was fetched and mounted`); pinned Rust `1.100.0` `just ci` (`cargo fmt --check`, `cargo clippy`, `cargo test`) passed all 31 tests.

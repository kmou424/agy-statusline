# `wow-pi` Footer Module Reference and Mapping to `agy` `statusLine`

- **Date**: 2026-11-25
- **Scope**: Objective anatomy of `wow-pi`'s `footer` module and factual feature-by-feature mapping onto Antigravity CLI (`agy 1.5.16`) `statusLine`.
- **Reference repository baseline**: `wow-pi` at Git commit `bc16aea6086f594e2ed4c3ab9a54afb8dd25d129` (verified clean across `packages/wow-ui-shell` and `packages/wow-base-ui` on `2026-11-25`).

## Primary sources

- `wow-pi`: `packages/wow-ui-shell/src/footer.ts` (lines 1–301)
- `wow-pi`: `packages/wow-ui-shell/src/footer-layout.ts` (lines 1–373)
- `wow-pi`: `packages/wow-ui-shell/src/runtime-catalog.ts` (lines 1–108)
- `wow-pi`: `packages/wow-base-ui/src/utilities/icons.ts` (lines 1–148)
- `wow-pi`: `packages/wow-base-ui/src/utilities/format.ts` (lines 1–45)
- Antigravity CLI contract: [2026-11-25-agy-statusline-contract.md](./2026-11-25-agy-statusline-contract.md) and [sources/agy-statusline-doc.md](./sources/agy-statusline-doc.md)

---

## 1. `wow-pi` Footer Anatomy

`wow-pi` splits footer data collection (`registerFooter` in `footer.ts`) from pure text/ANSI layout (`buildFooterLines` in `footer-layout.ts`).

### 1.1 Default configuration flags (`footerDefaults`, `footer-layout.ts:20-33`)

- Enabled by default: `enabled`, `cwd`, `gitBranch`, `gitStatus`, `runtime`, `context`, `tokens`, `cost`, `statusSummary`.
- Disabled by default: `sessionName`, `gitCommit`.
- Default extension-status separator: `"|"` (usage stats on Row 2 always use a dim `" | "` separator).

### 1.2 Row-by-row structure and styling

| Row | Position | Content | Color / Style semantics (`ThemeColor`) |
|---|---|---|---|
| **Row 1** | Left (priority `0`) | Working directory (`~` for `$HOME`, `~/rel` for descendants, raw path otherwise). Compact fallback: `basename(path)`. | Glyph: `mdLink`; path: `accent`. |
| **Row 1** | Left (priority `2`, opt-in) | Session name (`sessionName`), if non-empty. | Glyph: `dim`; text: `text`. |
| **Row 1** | Left (priority `3`) | Git group: branch (clipped to 20 cols; detached renders `"HEAD"`), optional commit/tag (exact tag or 7-char OID; omits `"(initial)"`), status counts in `[ ... ]` (`conflicted`, `deleted`, `modified`, `renamed`, `staged`, `untracked`, `stashed`, plus `ahead`/`behind`/`diverged`). | Branch: `warning` if detached else `mdLink`; commit/tag: `dim`; brackets: `dim`; conflicted/deleted: `error`; modified/renamed: `warning`; staged: `success`; untracked/stashed: `muted`; ahead: `success`; behind/diverged: `warning`. |
| **Row 1** | Left (priority `4`) | Runtime + version + optional framework. | Runtime symbol/name: `success`; version: `muted`; framework: `dim`. |
| **Row 1** | Right | Context usage (when `window > 0`): icon + 12-cell bar + 1-decimal percentage + `·` + `used/window` compact counts (e.g. `14.2% · 150K/1M`). Unknown percent/tokens render `?` and 0 filled cells. | Bar fill (`barStress`): `< 50%` `success`, `>= 50%` `warning`, `>= 80%` `error`; unfilled cells: `dim`. Icon & percent (`stress`): `< 70%` `accent`, `>= 70%` `warning`, `>= 90%` `error`. `used/window`: `text`. Bar glyphs: Nerd `▰`/`▱`, ASCII `#`/`-`. |
| **Row 2** | Left | Model identity joined by dim ` · `: model glyph + provider name + `modelName [thinkingEffort]` + optional badges (`fast/ultrafast on`, `Do not pause`). | Model glyph: `mdLink`; provider: `success` (`openai*`), `accent` (`anthropic`), else `muted`; model name: `text`; effort: `dim` (shown only when `model.reasoning && thinking && thinking !== "off"`); badges: `warning`. |
| **Row 2** | Right | Usage stats joined by dim ` \| `: prompt input (`input + cacheRead + cacheWrite`), output, cumulative cache-hit rate (omitted if prompt input is 0), USD cost (`$0.000`, with extra currency glyph in Nerd mode). | Input: `accent`; output: `success`; cache hit rate: `< 30%` `error`, `< 70%` `warning`, `>= 70%` `success`; cost: `warning`. |
| **Row 3+** | Full width (wrapped) | Extension statuses from host status map (excluding `wow.do-not-pause`), sorted by key, ANSI-stripped and flattened. | Glyph: `mdLink`; items: `muted`; separator: `dim`. |
| **Row 4+** | Full width | Captured `belowEditor` widgets relocated into the footer. | Owned by each widget component. |
| **Last row** | Centered | Session summary: `Σ <runs> runs · <messages> messages · <compactions> compactions · TPS <x.x tok/s | —> · TTFT <elapsed | —>`. | Entire line: `dim`. |

### 1.3 Responsive width & truncation rules (`footer-layout.ts:140-372`)

Width calculations use terminal cell width (`visibleWidth`, ANSI-stripped) rather than string length:

- **Non-positive width**: returns `[]`.
- **Row 1**:
  1. If `visibleWidth(contextText) + visibleWidth(compactCwd) + 1 > width`, collapse `contextText` to `<icon> <pct>` (dropping the 12-cell bar and `used/window`).
  2. Compute `leftBudget = max(0, width - visibleWidth(contextText) - (contextText ? 1 : 0))`.
  3. If full left parts exceed `leftBudget`, switch parts that have a `compact` form (currently `cwd` → `basename`).
  4. While `parts.length > 1` and joined left parts still exceed `leftBudget`, remove the item with the highest `priority` number first (drop order: `runtime (4)` → `git (3)` → `sessionName (2)`, keeping `cwd (0)` last).
  5. Right-align `contextText` against the clipped left string; if `right` alone fills `width`, `right` wins and is truncated to `width`.
- **Row 2**:
  1. Reserve `min(width, visibleWidth(badges) + (badges ? 1 : 0) + min(12, visibleWidth(modelLabel)))` columns for the left side.
  2. If `visibleWidth(cost) + reserved + 1 > width`, truncate `cost` to `max(0, width - reserved - 1)`.
  3. If full `identity` + `cost` + 1 exceeds `width`, drop the model glyph, provider name, and badge text labels, keeping clipped `modelLabel` plus compact badge glyphs only.
  4. Right-align `cost` against `identity`.
- **Final safety pass**: Every emitted line passes through `truncateToWidth(line, width)`.
- **Input sanitization**: `clean()` strips VT control characters, replaces `\n`, `\r`, `\t` with spaces, and trims.

### 1.4 Glyph sets and icon-mode detection (`icons.ts:1-148`)

`resolveIconMode("auto")` checks environment variables rather than querying installed fonts: Nerd mode is enabled if `HERDR_ENV === "1"`, `TERM === "xterm-kitty"`, `WT_SESSION` is truthy, or lowercased `TERM_PROGRAM` / `LC_TERMINAL` matches one of `iterm.app`, `ghostty`, `wezterm`, `kitty`, `rio`, `tabby`, `windowsterminal`, `vscode`, `herdr`.

Footer-relevant glyphs in `icons.ts:34-97, 129-148`:

| Role | Nerd glyph (codepoint) | ASCII fallback |
|---|---|---|
| `cwd` | `U+F07C` | `@` |
| `session` | `U+F02B` | `s` |
| `git` | `U+F418` | `*` |
| `context` | `U+F0C9` | `#` |
| `model` | `U+F121` | `M` |
| `fast` / `doNotPause` | `U+F0E7` / `U+F04B` | `>` / `play` |
| `input` / `output` | `U+F093` / `U+F019` | `↑` / `↓` |
| `cacheHit` / `cost` | `U+F1C0` / `U+F155` | `CH` / `$` |
| `extensions` | `U+F1E6` | `&` |
| Git status (`conflicted`, `deleted`, `modified`, `renamed`, `staged`, `untracked`, `stashed`) | `=`, `✘`, `!`, `»`, `+`, `?`, `$` | `=`, `x`, `!`, `r`, `A`, `?`, `S` |
| Git divergence (`ahead`, `behind`, `diverged`) | `↑`, `↓`, `⇕` | `^`, `v`, `^v` |
| Runtimes (`nodejs`, `rust`, `go`, `python`, `ruby`, `java`, `cpp`, `c`, `swift`, `kotlin`, `deno`, `bun`, `php`, `lua`, `zig`) | `U+E718`, `U+E7A8`, `U+E626`, `U+E73C`, `U+E739`, `U+E256`, `U+E61D`, `U+E61E`, `U+E755`, `U+E634`, `U+E7FB`, `U+E6FB`, `U+E73D`, `U+E620`, `U+E6A9` (fallback `U+F120`) | runtime name string |

Compact number formatting (`format.ts:1-11`): `Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 1, useGrouping: false })` (non-finite or negative values render `"—"`).

### 1.5 Git & runtime probe semantics (`footer.ts:136-189`, `footer-layout.ts:59-82`, `runtime-catalog.ts:5-108`)

- **Git status command**: `git status --porcelain=v2 --branch --show-stash` (2 s timeout in `wow-pi`).
  - `# branch.oid <oid>` → `oid`
  - `# branch.ab +<a> -<b>` → `ahead`, `behind`
  - `# stash <n>` → `stashed`
  - `? ...` → `untracked++`
  - `u ...` → `conflicted++`
  - `1 XY ...` or `2 XY ...`: `X !== "."` → `staged++`; `Y === "M"` → `modified++`; `X === "D" || Y === "D"` → `deleted++`; `X === "R" || Y === "R"` → `renamed++`. (Note: counts can overlap on a single path, e.g. staged deletion increments both `staged` and `deleted`.)
- **Optional Git tag**: `git describe --tags --exact-match HEAD` (only when `gitCommit` is enabled; 2 s timeout).
- **Runtime detection order** (first marker match in `cwd` wins; never executes project scripts):
  1. `bun` (`bun.lock`, `bun.lockb` → `bun --version`)
  2. `deno` (`deno.json`, `deno.jsonc` → `deno --version`)
  3. `nodejs` (`package.json` → `node --version`)
  4. `rust` (`Cargo.toml` → `rustc --version`)
  5. `go` (`go.mod` → `go version`)
  6. `python` (`pyproject.toml`, `requirements.txt`, `Pipfile` → `python3 --version`)
  7. `kotlin` (`build.gradle.kts` → `java --version`)
  8. `java` (`pom.xml`, `build.gradle` → `java --version`)
  9. `ruby` (`Gemfile` → `ruby --version`)
  10. `php` (`composer.json` → `php --version`)
  11. `swift` (`Package.swift` → `swift --version`)
  12. `dotnet` (`global.json`, `Directory.Build.props` → `dotnet --version`)
  13. `elixir` (`mix.exs` → `elixir --version`)
  14. `cpp` (`CMakeLists.txt`, `meson.build` → `c++ --version`)
  Version extraction takes the first dotted numeric sequence (`\d+(?:\.\d+)+`) from `stdout`.
- **Framework detection order** (first marker match in `cwd` wins, only evaluated if a runtime matched):
  `Next.js` (`next.config.{js,mjs,ts}`) → `Nuxt` (`nuxt.config.{ts,js}`) → `SvelteKit` (`svelte.config.{js,ts}`) → `Angular` (`angular.json`) → `Vite` (`vite.config.{ts,js,mjs}`) → `Django` (`manage.py`) → `Laravel` (`artisan`) → `Rails` (`config/application.rb`, `bin/rails`).

---

## 2. Factual Mapping: `wow-pi` Footer vs. `agy` `statusLine`

### 2.1 Segment-by-segment availability

| `wow-pi` Footer segment | Availability in `agy` `statusLine` | Exact `agy` source or technical gap |
|---|---|---|
| Working directory (`cwd`) | **Direct from `stdin`** | `workspace.current_dir` (active) and `cwd` (launch dir). |
| Session name (`sessionName`) | **Direct from `stdin`** | `conversation_title` (omitted until assigned). |
| Git branch / detached / commit / tag / status counts / ahead-behind | **External local derivation** | `stdin` guarantees only `vcs.type` (`git`/`jj`/`hg`). Branch, OID, exact tag, and dirty/stash/ahead/behind counts must be queried locally (e.g. via `git status --porcelain=v2 --branch --show-stash`). No host branch-change push event exists. |
| Runtime / version / framework | **External local derivation** | Not present in `stdin`; must be inferred from filesystem markers and optional version probes. |
| Context bar, percentage, window capacity | **Direct (occupied token count approximate)** | `context_window.used_percentage`, `remaining_percentage`, and `context_window_size` (`0` if unknown) come from `stdin`. No dedicated `occupied_tokens` integer is supplied; `round(used_percentage * context_window_size / 100)` is an approximation, while `total_input_tokens` is session-cumulative rather than window occupancy. |
| Model label & thinking effort | **Direct from `stdin`** | `model.display_name` (or `model.id`) and `model.effort` (`low`–`max`, when present). |
| Provider display name | **Unavailable as separate field** | `model` only provides `id`, `display_name`, and `effort`. |
| Pi `serviceTier` / `doNotPause` badges | **Unavailable (Pi-specific)** | No equivalent keys in `agy` `stdin`. |
| Cumulative input / output tokens | **Direct from `stdin`** | `context_window.total_input_tokens` and `context_window.total_output_tokens`. Note: whether `total_input_tokens` includes cache reads/writes is not explicitly stated in the field table and requires live payload verification. |
| Cumulative cache-hit rate | **Accounting mismatch (per-turn only in `agy`)** | `wow-pi` computes session-cumulative `cacheRead / (input + cacheRead + cacheWrite)` across all entries. `agy` exposes cache counters only under `context_window.current_usage` (`cache_read_input_tokens`, `cache_creation_input_tokens`, `input_tokens`) with no cumulative cache totals and no guaranteed turn/response ID for deduplicating snapshots across repeated invocations. |
| Estimated USD cost | **Direct from `stdin`** | `cost.total_usd` (includes `subagent_usd`; `cost.estimated` indicates whether estimated). Omitted until reported. |
| Extension statuses & captured `belowEditor` widgets | **Unavailable** | `agy` exposes no extension status map or widget capture API. |
| Session `Σ` (`runs`, `messages`, `compactions`, `TPS`, `TTFT`) | **Unavailable from `statusLine` contract** | None of these counters or streaming-timing metrics exist in `stdin`. `transcript_path` is not documented as a stable schema, and 300 ms invocation throttling prevents measuring true TTFT or provider generation TPS from status-line snapshots. |
| Multi-line responsive layout & ANSI colors | **Direct capability** | `agy` explicitly supports multi-line output, ANSI color codes, and supplies live `terminal_width` in `stdin`. |

### 2.2 `agy`-native `statusLine` fields with no `wow-pi` footer counterpart

These fields are provided natively in `agy` `stdin` and have no direct equivalent in `wow-pi`'s footer:

- `quota`: per-bucket remaining quota (`remaining_fraction` or `remaining_amount`), `reset_time`, `reset_in_seconds`, `disabled` (periodically refreshed during long turns as of `v1.5.16`).
- `agent_state`: `idle`, `working`, `tool_use`, `reviewing`, `authenticating`, `initializing`, `error`.
- `cycle_mode`: `accept-edits`, `auto`, `plan` (omitted in default mode).
- `sandbox.enabled`: terminal sandbox active state.
- `subagents`: array of `{ name, role, status, model? }`.
- `task_count`, `artifact_count`, `pending_input_count`, `tool_confirmation_pending`.
- `vim.mode`: `NORMAL`, `INSERT`, `VISUAL`, `VISUAL LINE` (only when `editorMode == "vim"`).
- `plan_tier`, `exceeds_200k_tokens`, `agent.name`, `version`, `product`.

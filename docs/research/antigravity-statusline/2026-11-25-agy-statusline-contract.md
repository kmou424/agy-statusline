# Antigravity CLI (`agy`) `statusLine` Contract and Runtime Constraints

- **Date**: 2026-11-25
- **Scope**: Antigravity CLI custom status line (`statusLine`) contract, `stdin` payload schema, interactive controls, adjacent display settings, and observed runtime constraints.
- **Verification scope**: Static inspection of official documentation, published JSON Schema, official example script, and local `agy --version` / `agy changelog` output. No custom `statusLine` command was executed inside a live `agy` TUI session during this investigation.

## Primary sources

- Local CLI binary checked via `agy --version` on `2026-11-25`: `1.5.16` (macOS arm64 Mach-O executable).
- Local `agy changelog` output (checked `2026-11-25`).
- Official status line guide (`https://antigravity.google/docs/cli/statusline.md`, read `2026-11-25`) — verbatim copy at [sources/agy-statusline-doc.md](./sources/agy-statusline-doc.md).
- Official `/statusline` command guide (`https://antigravity.google/docs/cli/commands/statusline.md`, read `2026-11-25`) — verbatim copy at [sources/agy-cmd-statusline-doc.md](./sources/agy-cmd-statusline-doc.md).
- Official example script (`https://raw.githubusercontent.com/google-antigravity/antigravity-cli/main/examples/statusline/statusline.sh`, read `2026-11-25`) — verbatim copy at [sources/agy-example-statusline-sh.md](./sources/agy-example-statusline-sh.md).
- Official `settings.json` v1 JSON Schema (`https://antigravity.google/schemas/v1/settings.json`, read `2026-11-25`) — verbatim excerpts at [sources/agy-settings-schema-statusline.md](./sources/agy-settings-schema-statusline.md).
- Official settings guide (`https://antigravity.google/docs/settings.md`, read `2026-11-25`) and web changelog (`https://antigravity.google/docs/changelog.md`, read `2026-11-25`).

---

## 1. Configuration contract (`settings.json` & `/statusline`)

Antigravity CLI stores persistent settings in `~/.gemini/antigravity-cli/settings.json` using sparse persistence (only non-default keys are saved; the file is normalized on startup; unrecognized keys are preserved; malformed values open a Settings Error screen before starting).

### `statusLine` object schema

From [sources/agy-statusline-doc.md](./sources/agy-statusline-doc.md) and [sources/agy-settings-schema-statusline.md](./sources/agy-settings-schema-statusline.md):

| Property | Type | Default | Documented behavior |
|---|---|---|---|
| `command` | `string` | — | Shell command executed via `sh -c` (Unix) or `cmd /c` (Windows). Leading `~` expands to the home directory. |
| `type` | `string` | `"command"` | Optional. `"command"` is the only supported enum value in the v1 schema (`additionalProperties: false` on `$defs.statusLine`). |
| `padding` | `integer` (`>= 0`) | `0` | Number of blank lines rendered above the status line. |
| `enabled` | `boolean` | `true` | `false` stops running `command` and shows the built-in status line while retaining `command` in `settings.json`. |
| `stack_with_default` | `boolean` | `false` | When `true`, renders custom output beneath the built-in status line instead of replacing it. |

**Deprecated bare boolean form**: Setting `"statusLine": true` or `"statusLine": false` is deprecated; the CLI reads a boolean as an object without a `command`, so no script executes.

### Interactive `/statusline` commands

From [sources/agy-cmd-statusline-doc.md](./sources/agy-cmd-statusline-doc.md):

- `/statusline` — toggle status line on/off
- `/statusline on` or `/statusline enable` — enable
- `/statusline off` or `/statusline disable` — disable
- `/statusline <command>` — save and immediately activate a custom command
- `/statusline delete` or `/statusline reset` — remove custom command and revert to built-in default
- `/statusline help` — display quick usage reference

---

## 2. Execution, rendering, and failure contract

Documented guarantees from [sources/agy-statusline-doc.md](./sources/agy-statusline-doc.md) and `changelog.md`:

- **Placement**: Rendered below the TUI prompt box (replacing the built-in status line by default, or stacked beneath it when `stack_with_default: true`).
- **Invocation trigger**: Executed whenever agent state changes (not on a fixed wall-clock polling interval).
- **Shell wrapper**: Executed through `sh -c` on Unix and `cmd /c` on Windows.
- **I/O**: Current session state is piped as JSON to `stdin`; `stdout` becomes the rendered status line.
- **Output formatting**: Multi-line output and ANSI color codes are explicitly supported.
- **Throttle**: Invoked at most once every **300 ms**.
- **Timeout**: Any single run exceeding **5 seconds** is terminated.
- **Circuit breaker**: After **30 consecutive failures**, the CLI stops running the script for the rest of the session and displays `⚠ Statusline disabled after 30 consecutive failures.` Running `/statusline enable` re-enables it.
- **Transient notices**: Release notes document that transient CLI notices can temporarily replace the custom status line, which returns once the notice clears.
- **Print / headless mode**: Native `statusLine` scripts are not executed in non-interactive `-p` mode.

### Unspecified (undocumented) behaviors

The published docs and schema do **not** specify:
- A maximum line count or vertical height cap for multi-line output.
- Whether `agy` clips or wraps output lines exceeding `terminal_width`, or whether a horizontal border/margin reduces usable columns below `terminal_width`.
- A guaranteed ANSI color depth (the official example in [sources/agy-example-statusline-sh.md](./sources/agy-example-statusline-sh.md) uses standard 16-color SGR codes only).
- Whether a terminal window resize alone (without an agent-state transition) triggers re-invocation.
- A general output-cache lifetime or last-good-output retention policy on single failures below the 30-failure threshold.
- Any mouse click / interactive callback channel for custom status line text (click-to-expand in alt-screen is a built-in TUI feature, not a hook into custom `statusLine` output).

---

## 3. Documented `stdin` JSON payload schema

All top-level fields documented in [sources/agy-statusline-doc.md](./sources/agy-statusline-doc.md):

| Field | Type | Presence / Nullability | Meaning |
|---|---|---|---|
| `cwd` | `string` | present | Directory the CLI was launched in. |
| `conversation_id` | `string` | present | Active conversation ID (empty string before the first conversation starts). |
| `session_id` | `string` | present | Alias of `conversation_id` kept for compatibility. |
| `conversation_title` | `string` | omitted until assigned | Title of active conversation; updates immediately on rename. |
| `transcript_path` | `string` | present | Path where the conversation transcript is written (file may not exist yet; file format is not documented as a status line API). |
| `model` | `object \| null` | `null` until selected | `{ id, display_name, effort? }`; `effort` (`low`–`max`) appears only for models with multiple effort levels. |
| `workspace` | `object` | present | `{ current_dir, project_dir }`. |
| `version` | `string` | present | CLI version string. |
| `product` | `string` | present | Application name (`"antigravity"`). |
| `context_window` | `object` | present | `{ total_input_tokens, total_output_tokens, context_window_size, used_percentage, remaining_percentage, current_usage }`. `context_window_size` is `0` if unknown; `current_usage` is `null` before the first model response. |
| `context_window.current_usage` | `object \| null` | `null` before first response | Shown in the official example payload with `{ input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens }`. |
| `exceeds_200k_tokens` | `boolean \| null` | `null` before first response | `true` when conversation context exceeds 200,000 tokens. |
| `agent` | `object` | omitted for default agent | `{ name }` of active custom agent. |
| `quota` | `object` | optional | Map from quota bucket ID (e.g. `"gemini-weekly"`) to `{ remaining_fraction? \| remaining_amount?, reset_time?, reset_in_seconds?, disabled? }`. |
| `agent_state` | `string` | present | Documented values: `idle`, `working`, `tool_use`, `reviewing`, `authenticating`, `initializing`, `error`. (Note: [sources/agy-example-statusline-sh.md](./sources/agy-example-statusline-sh.md) also matches `thinking` and includes a wildcard fallback.) |
| `vcs` | `object` | omitted outside a repo | Documented table lists `{ type }` with `"git"`, `"jj"`, or `"hg"`. See schema discrepancy note below. |
| `cycle_mode` | `string` | omitted in default mode | Mode cycled by `Shift+Tab`: `accept-edits`, `auto`, or `plan`. |
| `sandbox` | `object` | present | `{ enabled: boolean }`; `true` while terminal sandbox is active. |
| `subagents` | `array` | optional | Elements with `{ name, role, status }` (`status`: `running` \| `completed`). See changelog note below for optional `model`. |
| `artifact_count` | `integer` | omitted when `0` | Number of artifacts produced in the conversation. |
| `plan_tier` | `string` | optional | Subscription tier of authenticated user (e.g. `"Pro"`). |
| `email` | `string` | optional/present | Signed-in account email address (sensitive personal field). |
| `pending_input_count` | `integer` | omitted when `0` | Number of queued user messages. |
| `tool_confirmation_pending` | `boolean` | omitted unless `true` | `true` while a tool confirmation dialog is shown. |
| `task_count` | `integer` | omitted when `0` | Number of running background tasks. |
| `terminal_width` | `integer` | present | Live interactive terminal width in columns. |
| `vim` | `object` | only when `editorMode == "vim"` | `{ mode }`: `NORMAL`, `INSERT`, `VISUAL`, or `VISUAL LINE`. |
| `cost` | `object` | omitted until reported | `{ total_usd, subagent_usd, estimated }`. List-price estimates (not billed amounts); `total_usd` includes subagents. |

### Observed schema discrepancies and recent changelog evolution

1. **`vcs.branch` / `vcs.dirty` discrepancy**: The official field table and example JSON in [sources/agy-statusline-doc.md](./sources/agy-statusline-doc.md) document only `vcs.type` (`git` / `jj` / `hg`). However, the official GitHub script in [sources/agy-example-statusline-sh.md](./sources/agy-example-statusline-sh.md) attempts to read `.vcs.branch // ""` and `.vcs.dirty // false`. Treat `vcs.branch` and `vcs.dirty` as unguaranteed/optional rather than guaranteed fields.
2. **`subagents[].model`**: Present in CLI release notes (local `agy changelog` and web `changelog.md`) though omitted from the `statusline.md` field table; treat as optional.
3. **Ongoing payload evolution**: Release notes across recent versions show repeated additive changes to the status line payload (`conversation_title`, `cost`, `subagents[].model`, periodic `quota` refreshes during long turns in `v1.5.16`, `--sandbox` reporting fixes, `stack_with_default`). Forward-compatible deserialization (ignoring unknown fields and unknown enum strings) is required to avoid breakage on CLI updates.

---

## 4. Adjacent display settings that interact with `statusLine`

From [sources/agy-settings-schema-statusline.md](./sources/agy-settings-schema-statusline.md) and `settings.md`:

- **`colorScheme`**: Closed enum (`"terminal"`, `"light"`, `"solarized light"`, `"colorblind-friendly light"`, `"dark"`, `"solarized dark"`, `"colorblind-friendly dark"`, `"tokyo night"`; default `"terminal"`). There is no host API exposing the active `colorScheme` or its semantic RGB palette to the `statusLine` script. Standard 16-color ANSI codes follow the host terminal palette when `colorScheme` is `"terminal"`.
- **`altScreenMode`**: `"default"` (inline conversation, alt-screen full-screen panels), `"always"` (full TUI in alternate screen with internal scrollback/mouse), `"never"` (always inline, `settings.json`-only). Inline mode also has `clearScrollbackOnResize` (default `true`).
- **`hideActiveItems`**: Boolean (default `false`). Hides the built-in list of active subagents and background tasks below the prompt box when `true`.

---

## 5. Objective runtime constraints and engineering hazards

1. **Invocation latency under a 300 ms ceiling**: At peak activity, `agy` can spawn the `statusLine` command up to ~3.3 times per second. Spawning multiple subprocesses per paint (e.g., chained shell/jq/git calls or slow interpreter cold starts) directly adds CPU overhead and risks hitting the 5 s timeout or 30-failure circuit breaker on slow disks, large Git worktrees, or remote/SSH filesystems.
2. **External Git / runtime probes are not event-driven**: Unlike an in-process TUI plugin with branch-watch hooks, a `statusLine` process starts fresh per invocation. Running uncached `git` commands or runtime version probes on every invocation repeats I/O at up to 300 ms cadence; conversely, any cache TTL alone cannot trigger a repaint while `agy` is idle because `agy` only invokes `statusLine` on agent-state changes.
3. **Terminal cell width & ANSI safety**:
   - String byte length and Unicode scalar count do not match terminal display width: ANSI SGR escapes take 0 columns, CJK characters and many emoji take 2 columns, combining marks take 0 columns, and Nerd Font Private Use Area glyphs can behave inconsistently across fonts/terminals.
   - Untrusted or arbitrary payload strings (`cwd`, `workspace.*`, `conversation_title`, Git branch/tag names) can contain newlines, tabs, or ESC sequences that corrupt TUI layout if not sanitized before printing.
4. **Sensitive fields in payload**: `email` and `transcript_path` (plus workspace paths and conversation titles) appear in `stdin`. Persisting raw `stdin` payloads in caches, debug logs, or shared outputs risks leaking personal/session data.
5. **Cross-platform shell invocation**: `agy` invokes `command` via `sh -c` on Unix and `cmd /c` on Windows; quoting and argument handling differ across shells if `command` includes arguments or spaces in paths.

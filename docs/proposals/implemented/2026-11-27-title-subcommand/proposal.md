# Built-in `title` Subcommand and Installer Integration

Accepted: 2026-11-27

## Problem

Antigravity CLI (`agy`) does not update the terminal window title (`OSC 0`/`OSC 2`) unless a `"title"` command is configured in `~/.gemini/antigravity-cli/settings.json`. Without one, terminal multiplexers and agent workspace managers such as Herdr keep displaying whatever stale `terminal_title` the launching shell left behind (for example `agy ~/P/R/agy-statusline` or `agy --conversation=3 ~/P/A/Sakiko`), making it difficult to tell multiple `agy` sessions apart.

`agy` already provides a native `"title"` setting parallel to `"statusLine"` that pipes the exact same session JSON payload (including `workspace.current_dir`, `cwd`, and `conversation_title`) to `stdin` and uses the command's single-line plain-text `stdout` as the terminal title. Extending `agy-statusline` (built in `2026-11-25-wow-pi-footer-prototype`) and its cross-platform installers (built in `2026-11-25-ci-cd-release-and-installers`) to serve both `"statusLine"` and `"title"` avoids maintaining a separate `jq`/shell helper script across macOS, Linux, Android (Termux), and Windows.

## Proposal

### 1. CLI contract (`src/main.rs`)

Parse `std::env::args()` directly with zero new crate dependencies:

- **No arguments (`agy-statusline`)**: unchanged; reads `agy` JSON from `stdin`, prints the two-row ANSI status line, and always exits `0`.
- **`agy-statusline title`**: reads the same `agy` JSON from `stdin`, prints a single plain-text window title line to `stdout` (or nothing when empty), and always exits `0` so malformed or partial payloads never cause errors.
- **`-h` / `--help`**: prints concise usage (`agy-statusline [title]`) to `stdout` and exits `0`.
- **`-V` / `--version`**: prints `agy-statusline <version>` to `stdout` and exits `0`.
- **Unknown arguments**: prints a short usage error to `stderr` and exits `2`.

### 2. Title rendering (`render_title` / `render_title_from_input`)

Add a pure, in-memory title renderer that reuses `input::parse_input` and `text::clean`, spawns no subprocesses (`git` or runtime probes), performs no filesystem I/O, and never persists or logs any `stdin` fields (`email`, `transcript_path`, workspace paths, or conversation titles):

- **Directory name (`dir_name`)**:
  - Prefer `input.active_dir()` (`workspace.current_dir` when non-empty, otherwise `cwd`).
  - Strip ANSI/VT escapes and control characters via `text::clean` and collapse whitespace runs to single spaces.
  - Extract the final path segment independently of host OS path rules across both `/` and `\` separators:
    - Strip trailing `/` and `\` (e.g. `/Users/alice/repo/` -> `repo`, `C:\Users\alice\repo\` -> `repo`).
    - Preserve root indicators cleanly (`/` or `///` -> `/`; `C:\` or `C:/` -> `C:`).
    - Handle UNC paths (`\\server\share\repo` -> `repo`, `\\server\share\` -> `share`).
- **Session title (`session_title`)**:
  - Read `input.conversation_title`, sanitize via `text::clean`, and collapse whitespace runs into single ASCII spaces.
- **Formatting** (single line, plain text only, no ANSI styling):
  - Both present: `[<dir_name>] <session_title>`
  - Only `dir_name` present: `[<dir_name>]`
  - Only `session_title` present: `<session_title>`
  - Neither present (including empty or invalid JSON): empty string `""` (prints nothing).

### 3. Installer integration (`scripts/install.sh` & `scripts/install.ps1`)

Update both installers to configure `agy`'s `"title"` block alongside `"statusLine"` by default, while adding an explicit opt-out:

- **Default configuration** (when `--no-configure` / `-NoConfigure` is not set and `--no-title` / `-NoTitle` is not set):
  - Merge `"title": { "type": "command", "command": "<title_command>", "enabled": true }` into `~/.gemini/antigravity-cli/settings.json` alongside `"statusLine"`, preserving all unrelated keys, existing `"statusLine"` options, symlinked `settings.json` targets, and atomic writes.
  - Unix default install directory (`~/.local/bin`): `"~/.local/bin/agy-statusline title"`.
  - Unix custom install directory: `"<shell_quoted_bin_path> title"` (e.g. `'/tmp/custom dir/agy-statusline' title`).
  - Windows: `"<cmd_quoted_or_raw_bin_path> title"` (e.g. `"C:\R&D tools\agy-statusline.exe" title`, leaving `title` unquoted so `cmd /c` does not strip outer quotes).
  - Support all four Unix JSON writers (`python3`, `jq`, `node`, and fresh-file `awk` fallback) plus PowerShell's `ConvertFrom-Json` / `ConvertTo-Json -Depth 100`.
- **New opt-out flag**:
  - `scripts/install.sh`: `--no-title` skips writing/modifying `"title"` while still configuring `"statusLine"`.
  - `scripts/install.ps1`: `-NoTitle` (and `--no-title` compatibility alias) does the same.
- **Existing `--no-configure` / `-NoConfigure`**: continues to skip all `settings.json` edits.
- **No-JSON-tool fallback message** in `scripts/install.sh`: include manual guidance for both `statusLine` and `title` when `settings.json` already exists without `python3`, `jq`, or `node`.

### 4. Documentation and tests

- Add unit tests and CLI binary integration tests for `render_title`, Unix/Windows/UNC/root path extraction, CJK/ANSI/control-character sanitization, fallback combinations, and CLI argument handling (`title`, `--help`, `--version`, invalid args).
- Extend `scripts/test-install.sh` and `scripts/test-install.ps1` to cover default `title` configuration, `sh -c` execution of custom-path `title` commands, `--no-title` / `-NoTitle` leaving existing `"title"` untouched, `--no-configure`, and the `awk` fallback.
- Update `README.md` to document `agy-statusline title`, the `settings.json` `"title"` snippet, and `--no-title` / `-NoTitle`.

## Alternatives

1. **Maintain a separate shell/`jq` script (`~/.gemini/antigravity-cli/title.sh`)**
   - Rejected because it introduces an external `jq` dependency, does not work natively on Windows, and splits installation and updates across two artifacts.
2. **Emit `OSC 0`/`OSC 2` to `/dev/tty` or call `herdr pane report-metadata` inside `statusLine` rendering**
   - Rejected because `statusLine` is a pure rendering hook running up to every 300 ms; adding TTY or Unix-socket side effects couples rendering to host multiplexers, adds latency, breaks over plain SSH or when `/statusline` is disabled, and duplicates `agy`'s built-in `"title"` contract.
3. **Ship a second binary (`agy-title`) in release archives**
   - Rejected because it doubles packaged executables across all 8 build targets and complicates both installers for a small pure-Rust module that shares `input` and `text` directly with `agy-statusline`.

## Acceptance criteria

- `agy-statusline title` renders `[<dir_name>] <session_title>` for standard payloads, `[<dir_name>]` when `conversation_title` is omitted or blank, `<session_title>` when directory is missing, and empty output with exit code `0` on `{}` or malformed JSON, across Unix, Windows, UNC, and CJK/control-character inputs.
- `agy-statusline` with no arguments preserves existing status line output and exit behavior; `--help`/`-h` and `--version`/`-V` exit `0`; unknown arguments exit `2`.
- `just ci` (`fmt-check`, `clippy`, `cargo test`) passes with zero warnings.
- `shellcheck -s sh scripts/install.sh` passes, and `scripts/test-install.sh` passes under both `sh` and `bash`; `scripts/test-install.ps1` passes in CI on Windows.
- Live verification in `agy` confirms `"title": { "type": "command", "command": "~/.local/bin/agy-statusline title", "enabled": true }` updates the pane's `terminal_title` in Herdr.

## Risks

- **Overwriting an existing custom `"title"` configuration**: default installation updates `"title"` in `settings.json` just like `"statusLine"`. Mitigated by providing `--no-title` / `-NoTitle` and `--no-configure` / `-NoConfigure` and documenting them in `--help` and `README.md`.
- **Windows `cmd /c` quoting**: paths with spaces or shell metacharacters require quoting the binary path while keeping the `title` argument unquoted so `cmd /c` does not strip outer quotes; covered in `scripts/test-install.ps1`.
- **Tilde expansion with arguments**: `"~/.local/bin/agy-statusline title"` relies on `agy` / `sh -c` expanding the leading `~` on the command word; verified under `sh -c` and live `agy`.

## Checklist

- [x] Implement `title` renderer, CLI subcommand dispatch (`title`, `--help`, `--version`), and Rust unit/integration tests.
- [x] Integrate `title` configuration and `--no-title` / `-NoTitle` into `scripts/install.sh`, `scripts/install.ps1`, and their offline fixture tests.
- [x] Update `README.md`, run full verification (`just ci`, `shellcheck`, installer tests), and move proposal to `implemented/` upon shipping.

## Result

Shipped in `v0.1.2`:

- Added pure in-memory title rendering (`src/title.rs`, re-exported as `render_title` and `render_title_from_input` from `src/lib.rs`) and zero-dependency CLI argument dispatch (`title`, `-h`/`--help`, `-V`/`--version`, exit code `2` on invalid arguments) in `src/main.rs`, with unit and CLI integration tests in `src/title.rs` and `tests/title.rs`.
- Integrated non-destructive `"title"` configuration and `--no-title` / `-NoTitle` opt-out into `scripts/install.sh`, `scripts/install.ps1`, `scripts/test-install.sh`, `scripts/test-install.ps1`, and `README.md`.
- Deviation from initial draft (added during two-pass review): both installers gate automatic `"title"` configuration on `canary` or semver tags `>= v0.1.2`, skipping `"title"` with a stderr notice when installing legacy `v0.1.0`/`v0.1.1` releases that lack the `title` subcommand.

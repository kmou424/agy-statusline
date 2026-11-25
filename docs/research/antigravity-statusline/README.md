# Antigravity CLI custom status line research

Question: What is Antigravity CLI (`agy`)'s documented `statusLine` customization contract and runtime boundary, and how do the segments, data sources, and layout rules of `wow-pi`'s `footer` module map onto it?

Conclusion: Antigravity CLI (`agy 1.5.16`) provides a first-class `statusLine` command interface in `~/.gemini/antigravity-cli/settings.json` that pipes 26+ JSON session-state fields to `stdin` on agent-state changes and renders multi-line, ANSI-colored `stdout` below the prompt box (with `padding` and `stack_with_default` options, a 300 ms invocation ceiling, a 5 s timeout, and a 30-consecutive-failure session circuit breaker). Most visual elements of `wow-pi`'s `footer` (directory, conversation title, context bar/percentage/capacity, model/effort, cumulative token counts, estimated cost, and responsive width truncation) map directly from `stdin` or can be derived locally via external Git/runtime inspection, while several metrics diverge in accounting or have no `agy` `statusLine` source (exact occupied token count, session-cumulative cache-hit rate, provider label, Pi service tier / autonomy flags, extension status registry, captured below-editor widgets, and session `Σ` run/compaction/TPS/TTFT telemetry). Conversely, `agy` exposes native fields absent from `wow-pi`'s footer (`quota`, `agent_state`, `cycle_mode`, `sandbox.enabled`, `subagents`, `task_count`, `artifact_count`, `pending_input_count`, `tool_confirmation_pending`, `vim.mode`, `plan_tier`, `exceeds_200k_tokens`).

## Reports

- [2026-11-25-agy-statusline-contract.md](./2026-11-25-agy-statusline-contract.md) — Documented `agy` `statusLine` settings schema, `/statusline` commands, execution/throttle/failure contract, `stdin` JSON payload schema, changelog observations, adjacent display settings, and runtime constraints.
- [2026-11-25-wow-pi-footer-reference.md](./2026-11-25-wow-pi-footer-reference.md) — Primary-source anatomy of `wow-pi`'s `footer` module (commit `bc16aea6086f594e2ed4c3ab9a54afb8dd25d129`) and factual segment-by-segment mapping to `agy` `statusLine`.

## Verbatim sources

- [sources/agy-statusline-doc.md](./sources/agy-statusline-doc.md) — Official Antigravity CLI status line guide (retrieved `2026-11-25`).
- [sources/agy-cmd-statusline-doc.md](./sources/agy-cmd-statusline-doc.md) — Official `/statusline` slash command reference (retrieved `2026-11-25`).
- [sources/agy-example-statusline-sh.md](./sources/agy-example-statusline-sh.md) — Official GitHub `statusline.sh` example script (retrieved `2026-11-25`).
- [sources/agy-settings-schema-statusline.md](./sources/agy-settings-schema-statusline.md) — Verbatim `statusLine` and adjacent display-key excerpts from the published `settings.json` v1 JSON Schema (retrieved `2026-11-25`).

## Refreshing this topic

Re-check `agy --version`, `agy changelog` (or `https://antigravity.google/docs/changelog.md` under `## Antigravity CLI`), `https://antigravity.google/docs/cli/statusline.md`, and `https://antigravity.google/schemas/v1/settings.json`; if contracts changed, update verbatim files under `sources/` with a new `Retrieved:` date and add a new dated report rather than editing frozen reports.

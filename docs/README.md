# Documentation profile

This file maps `agy-statusline`'s documentation onto the homes of the `docs-engineering` skill. Read the skill for how documents are placed and how they change; read this file for where they are.

## Entry points and boundaries

- Project instruction file: [AGENTS.md](../AGENTS.md)
- One-screen project entry: [README.md](../README.md)
- Ephemeral scratch directory: `.scratch/` (Git-ignored; excluded from documentation citations)

## Homes

| Home | Location | Language | Holds here |
|---|---|---|---|
| design | `docs/design/` | Chinese or English, one per document | how components and mechanisms fit together |
| guides | `docs/guides/` | Chinese or English, one per document | setup, usage, verification, and operational procedures |
| reference | `docs/reference/` | Chinese or English, one per document | exact specifications, contracts, configuration keys, and lookup facts |
| adr | `docs/adr/` | Chinese or English, one per document | binding constraints and refused alternatives |
| proposals | `docs/proposals/{proposed,implemented,archived}/` | Chinese or English, one per proposal | requests for non-trivial change and their outcomes |
| research | `docs/research/<topic>/` | as written | dated investigations, primary sources, and conclusions |

Only homes with active documents need to exist on disk.

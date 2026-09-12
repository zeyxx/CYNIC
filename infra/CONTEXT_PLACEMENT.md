# Context File Placement (R/R)

One page. Decides where a fact belongs across the four Hermes context files. Wrong placement is debt: it leaks project specifics into global identity or user facts into agent memory.

## The four files

| File | Owns | Written by | Scope | Loaded as |
|---|---|---|---|---|
| `SOUL.md` | Agent identity: tone, style, what to avoid | You, by hand | Global, follows every session | Slot 1 of system prompt, verbatim, from `HERMES_HOME` only |
| `AGENTS.md` | Project instructions: commands, structure, conventions, boundaries | Project author, by hand | This repo and subdirectories | Project context at session start, plus progressive subdirectory discovery |
| `MEMORY.md` | Agent's notes: environment facts, tool quirks, lessons learned | Agent, via memory tool | Global across sessions | Frozen snapshot at session start |
| `USER.md` | User profile: name, role, preferences, expectations | Agent, via memory tool | Global across sessions | Frozen snapshot at session start |

## Routing rule

- Follows you everywhere -> `SOUL.md`.
- Belongs to this project only -> `AGENTS.md`.
- Agent learned it doing work -> `MEMORY.md`.
- It describes the human -> `USER.md`, never `MEMORY.md`.

## Hard boundaries

- `SOUL.md` never holds repo paths, commands, ports, or architecture. Project specifics found there move to `AGENTS.md`.
- `AGENTS.md` never holds personality, tone, or user profile. It stays command-first and under 150 lines; detail lives in executable contracts (`contracts/`, `infra/registry.json`) and their CLI read-back, not in prose.
- `MEMORY.md` never holds user-profile facts. User facts found there move to `USER.md`.
- Memory and context files inject as frozen snapshots at session start. Mid-session saves persist to disk immediately and appear from the next session. Restart the session to pick up edits.
- Keep each file hand-written and pruned. Stale context is worse than no context.

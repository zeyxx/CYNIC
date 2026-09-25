# CYNIC — Codex Adapter

Read:

1. `AGENTS.md`
2. `contracts/foundation.json`
3. `infra/runtime-authority.json`
4. `infra/registry.json`

Default validation:

```bash
make check
```

Do not load deleted/stale surfaces as protocol. `axioms/`, `organs/`, legacy
docs and Python lifecycle gates are no longer runtime authority.

MCP uses the Rust binary configured in `.codex/config.toml`.

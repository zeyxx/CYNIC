# CYNIC Codex Protocol

Codex follows the minimal repo protocol in `AGENTS.md`.

Read order:

1. `AGENTS.md`
2. `contracts/foundation.json`
3. `infra/runtime-authority.json`
4. `infra/registry.json`

Default validation:

```bash
make check
```

Do not treat deleted `axioms/`, `organs/`, legacy docs, or Python lifecycle
gates as authority. The MCP projection is the Rust binary configured in
`.codex/config.toml`.

# Archived Unused Repositories

These 8 JavaScript packages were functional but unused in CYNIC production phase.
Archived 2026-02-22 during architecture cleanup.

## Repositories

1. **anchor/** - Empty Anchor Protocol package, never used
2. **cynic-agent/** - Experimental agent framework, superseded by Python kernel
3. **gasdf-relayer/** - GAS relay service, not integrated into production
4. **llm/** - LLM adapter skeleton, replaced by Ollama integration
5. **mcp/** - MCP server package (core functionality used elsewhere, this version abandoned)
6. **node/** - Node.js SDK skeleton, abandoned in favor of Python
7. **observatory/** - Monitoring skeleton, not used in production
8. **persistence/** - Persistence skeleton, replaced by PostgreSQL + Kysely

## Archival Reason

During the transition to Python kernel (CYNIC β → α), these packages were evaluated:
- None have production imports (verified via grep)
- All functionality either superseded or implemented elsewhere
- Maintaining them adds cognitive overhead without value

## Recovery

To restore any package:

```bash
git restore --source=HEAD -- packages/<name>
```

Example:
```bash
git restore --source=HEAD -- packages/mcp/
```

## Replaced By

- **llm/** → `cynic/services/llm/` (Ollama integration)
- **persistence/** → `cynic/persistence/` (PostgreSQL + Kysely)
- **cynic-agent/** → `cynic/kernel/` (Python kernel)
- **mcp/** → `cynic/mcp/` (core MCP server)
- **node/** → `.claude/` (JavaScript plugin)

## Verification

All 8 packages passed dependency verification:
```bash
grep -r "from packages\|import.*packages" cynic/ --include="*.py"
# Result: ZERO matches (excluding venv)
```

Date: 2026-02-22
Verified by: CYNIC (automated integrity check)

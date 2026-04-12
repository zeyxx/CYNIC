## Cost Discipline

Every token is metabolic energy (§V.3). Route to the cheapest adequate model.

### Model Selection (subagents)

| Task type | Model | Examples |
|-----------|-------|---------|
| Deep reasoning, synthesis, metathinking | opus | Architectural decisions, multi-file diagnosis, cynic-judge |
| Code implementation, exploration, research | sonnet | Explore agent, code changes, refactoring, PR review |
| Mechanical, monitoring, formatting | haiku | Health checks, commit messages, lint, status |

**Default for Agent tool**: `model: "sonnet"` unless the task requires cross-file synthesis or philosophical reasoning. When in doubt, start with sonnet — escalate to opus only if the result is shallow.

**Already enforced structurally:**
- `.claude/agents/health-watcher.md`: model: haiku
- `.claude/agents/kernel-auditor.md`: model: sonnet
- Skills with `disable-model-invocation`: zero model cost

### Session Cost

Track in TODO.md Session Log. Approximate from conversation length:
- Short session (~30 messages): ~$1-2
- Medium session (~80 messages): ~$3-6
- Long session (~150+ messages): ~$8-15

These are rough estimates. Better than nothing (anti-pattern #6).

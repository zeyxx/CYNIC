---
name: cynic-learn
description: "Active learning loop for agents — captures corrections, extracts patterns, retrieves before acting. Triggers on: user correction ('no', 'actually', 'wrong', 'not that'), repeated mistake, task failure, or when starting a task that might repeat past errors. Inspired by self-improving-agent (OpenClaw) and MemSkill (arxiv 2602.02474)."
---

# CYNIC Learn — The Dog That Remembers

*"A dog that bites the same hand twice has learned nothing."*

## The Loop

```
CAPTURE ──▶ EXTRACT ──▶ RETRIEVE ──▶ APPLY
   │                                    │
   └────────── feedback ◀──────────────┘
```

Three operations. Every session. No exceptions.

## 1. RETRIEVE (at session start)

Before any task, read all feedback memories:

```
Read: ~/.claude/projects/<project>/memory/feedback_*.md
```

Scan for patterns relevant to the current task:
- Same file being modified? Check for past corrections on that file.
- Same type of task? (deploy, refactor, test) Check for past mistakes on that task type.
- Same domain? (router, probe, hal) Check for domain-specific learnings.

**If a relevant pattern exists: state it before acting.**
> "I found a past correction about this: [pattern]. Applying it."

**If no relevant pattern: proceed normally.**

## 2. CAPTURE (during work)

Record when ANY of these triggers fire:

| Trigger | Signal | Example |
|---------|--------|---------|
| **User correction** | "No", "Actually", "Wrong", "Not that", "Instead..." | "No, don't mock the database — use integration tests" |
| **Repeated failure** | Same error appears twice | clippy warning on same pattern twice |
| **Task failure** | Command fails, test fails, deploy fails | `cargo test` fails after your change |
| **Better approach discovered** | You or user find a simpler way mid-task | "We could have used `dirs::home_dir()` instead of parsing `$HOME`" |
| **Surprising behavior** | Something works differently than expected | Forgejo sets HOME to its own data dir |

### Capture Format

Write to memory as a `feedback` type file:

```markdown
---
name: feedback_<topic>
description: <one-line — specific enough to match in future searches>
type: feedback
---

**Context:** <what was being done>
**Trigger:** <correction | failure | discovery>
**Wrong approach:** <what was tried and failed>
**Right approach:** <what works>
**Why:** <root cause — not just the fix, but why the fix is correct>
**Scope:** <when to apply this — file, domain, always>
```

### Rules
- **Capture the WHY, not just the WHAT.** "Use `dirs::home_dir()`" is useless without "because `$HOME` doesn't exist on Windows."
- **Be specific about scope.** "Always" is almost never the right scope. "When dealing with file paths on cross-platform code" is better.
- **Update, don't duplicate.** If a feedback memory already exists for this topic, update it. Don't create `feedback_deploy_2.md`.
- **Convert relative dates.** "Yesterday" → "2026-03-11". Memories persist across sessions.

## 3. EXTRACT (periodically)

After accumulating 5+ feedback memories, consolidate:

1. **Group by domain** — Which area has the most corrections? (deploy, git, architecture, testing, etc.)
2. **Find patterns** — Do multiple corrections point to the same root cause?
3. **Promote to rules** — If a pattern appears 3+ times, it's a rule. Write it to the appropriate skill or CLAUDE.md.

```
3+ corrections on same topic → promote to skill or CLAUDE.md rule
Pattern across domains → candidate for universal principle
Single correction → stays as feedback memory
```

### Promotion Targets

| Pattern type | Promote to |
|-------------|-----------|
| Git/deploy mistake | cynic-workflow skill |
| Architecture violation | cynic-kernel skill |
| Code quality issue | CLAUDE.md development rules |
| Cross-platform gotcha | cynic-kernel Windows fixes section |
| Human preference | user memory (not feedback) |

## Anti-Patterns

| Anti-pattern | Why it's wrong | Instead |
|-------------|---------------|---------|
| Saving everything | Noise drowns signal | Only capture triggers above |
| Vague memories | "Be careful with deploys" teaches nothing | Specific: "Stop service BEFORE binary copy — ETXTBSY" |
| Duplicate memories | 3 files about the same thing | Update existing, don't create new |
| Forgetting to retrieve | Starting work without checking past mistakes | RETRIEVE is step 1, always |
| Over-extracting | Turning every correction into a rule | Wait for 3+ occurrences before promoting |

## Confidence

Pattern confidence is φ-bounded:
- 1 occurrence: anecdote (≤ 25%)
- 2 occurrences: coincidence (≤ 40%)
- 3+ occurrences: pattern (≤ 61.8%)

Never promote an anecdote to a rule. Wait for the pattern.

## Connected Mode

This skill uses Claude Code's native memory system (`~/.claude/projects/.../memory/`) as persistence. No custom tools, no external databases, no MCP servers. The memory system IS the learning substrate.

For formal self-evolving agent architectures, see:
- EvoAgentX (github.com/EvoAgentX/EvoAgentX)
- MemSkill (arxiv 2602.02474)
- OpenAI Self-Evolving Agents Cookbook

> *sniff* "The dog that learns from every bite becomes the one that never gets bitten."

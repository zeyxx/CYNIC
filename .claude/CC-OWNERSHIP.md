# CC-OWNERSHIP — Claude Code Harness Map

Living inventory of what Claude Code owns in this project. Not instructions — a map.

**Last updated:** 2026-04-30

## Hook Inventory

| Event | Script | Purpose | Consumer | Async | Timeout |
|-------|--------|---------|----------|-------|---------|
| SessionStart | session-init.sh | Probe kernel, inject crystals+wisdom+sessions | Claude context | no | 12000ms |
| UserPromptSubmit | phi-reminder.sh | Inject φ⁻¹ confidence bound reminder | Claude context | no | - |
| UserPromptSubmit | observe-prompt.sh | POST user prompt to /observe | CCM intake | yes | - |
| SubagentStart | subagent-context.sh | Inject axioms+triad into subagents | Subagent context | no | - |
| SubagentStart | observe-subagent.sh | POST agent dispatch to /observe | CCM intake | yes | - |
| PreToolUse | protect-files.sh | Block access to secrets (.ssh, .env) | Security gate | no | - |
| PreToolUse | coord-claim.sh | Claim kernel modules via /coord | Multi-cortex coord | no | - |
| PostToolUse | observe-tool.sh | POST tool actions to /observe (domain+tags) | CCM intake | yes | - |
| PostToolUse | rustfmt-rs.sh | Auto-format .rs files after Edit/Write | Code quality | no | - |
| Stop | askesis-claude-end.sh | Askesis exercise logging | Askesis system | no | 5000ms |
| Stop | session-stop.sh | Coord release, compliance, session summary, distill check | Kernel /observe | no | 10000ms |
| Stop | dream-trigger.sh | Check if dream consolidation needed | Memory system | no | 5000ms |
| Stop | exercise-scheduler.sh | Schedule next exercise | Askesis system | yes | 5000ms |

## Observer Pipeline (reasoning trail)

```
UserPromptSubmit ─→ observe-prompt.sh ─→ /observe (domain=session, tool=user_prompt)
                                              │
PostToolUse ──────→ observe-tool.sh ───→ /observe (domain=derived, tool=Edit/Bash/Read/...)
                                              │
SubagentStart ────→ observe-subagent.sh → /observe (domain=session, tool=agent_dispatch)
                                              │
                                              ▼
                                     CCM intake → crystals
```

**Domain derivation (observe-tool.sh v2):**
- `*/cynic-kernel/*` → rust
- `*/cynic-python/*` → python
- `*/.claude/*` → harness
- `*/docs/*` → docs
- `*/scripts/*` → ops
- Bash: cargo/make → rust, git/gh → git, curl/systemctl → ops

## Commands

| Command | Purpose | Model invocation | When |
|---------|---------|------------------|------|
| /status | Kernel+infra diagnostic | disabled | System health check |
| /cc-status | Harness diagnostic (hooks, observers, telemetry) | disabled | Harness health check |
| /build | Cargo build+test+clippy | disabled | After kernel changes |
| /deploy | Build release + swap binary | disabled | Production deploy |
| /run | Start kernel locally | disabled | Dev iteration |
| /test-chess | Run chess E2E test | disabled | Crystal benchmark |
| /e2e | Full E2E test suite | disabled | Pre-release |
| /dream | Trigger dream consolidation | disabled | Memory cleanup |
| /loop | Recurring command execution | disabled | Polling/monitoring |
| /cynic-kernel | Kernel architecture reference | disabled | Kernel dev |
| /cynic-workflow | Troubleshooting reference | disabled | Debugging |
| /frontend-dev | Frontend dev helper | disabled | UI work |

## Rules

| File | Scope | Key rules | Enforcement |
|------|-------|-----------|-------------|
| universal.md | All code | R1-R23 (paths, I/O, K15, commit, gates) | make lint-rules, hooks |
| kernel.md | cynic-kernel/ | K1-K17 (domain purity, ports, LLM patterns) | make lint-rules, clippy |
| python.md | cynic-python/ | P1-P15 (types, coverage, stateless, measure) | mypy, pytest |
| cost.md | All | Model routing (opus/sonnet/haiku) | Convention |
| reference.md | All | φ constants, Dog inventory, infra addresses | Reference |
| workflow.md | Sessions | Session protocol, multi-cortex, scientific method | Hooks + convention |

## Data Flows

**Observation pipeline:**
```
Hook fires → observe-*.sh → POST /observe → SurrealDB → CCM intake → crystals
```

**Session handover:**
```
session-stop.sh → POST /observe (tool=session_summary) → SurrealDB
session-init.sh → GET /observations?domain=session → inject into context
```

**Coord lifecycle:**
```
Edit kernel file → coord-claim.sh → POST /coord/claim
Tool use → observe-tool.sh → POST /coord/heartbeat
Session end → session-stop.sh → POST /coord/release
```

## Known Gaps

| Gap | Severity | K15 Status | Remediation |
|-----|----------|------------|-------------|
| SubagentStop not observed | LOW | No consumer | Wire when subagent results matter for crystallization |
| Read observations are noisy | LOW | Consumer exists (CCM) | Consider filtering high-frequency reads |
| Distill rate unmeasured | MEDIUM | /cc-status checks it | Track over time, target ≥ 61.8% |
| observe-tool.sh domain heuristic is coarse | LOW | Good enough | Refine if crystal domain distribution is skewed |

## Governance

- **Who edits:** Any cortex (Claude, Gemini, Codex) that modifies hooks/commands/rules
- **Staleness signal:** `git log -1 --format=%cr -- .claude/CC-OWNERSHIP.md` > 14 days = review
- **Falsification:** `diff <(jq -r '.hooks | to_entries[].value[].hooks[].command' .claude/settings.json | sort) <(ls .claude/hooks/*.sh | sort)` — every wired hook has a script, every script is wired

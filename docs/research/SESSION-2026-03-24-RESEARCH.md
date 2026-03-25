# Session 2026-03-24 — Research & Crystallization

## Context

Session started as RC8→RC2→RC5 industrial hardening. User stopped reactive fixing after RC8+RC2 commits were made too fast without skills/superpowers/research. Session pivoted to deep research on two foundational gaps: naming conventions and workflow infrastructure.

**Commits made (without proper workflow — flagged):**
- `9a5039c` fix(rc8): Makefile status-code health checks, setup-ubuntu.sh safe bind
- `3cf5d35` fix(rc2): honest health — count healthy dogs, not total dogs

---

## Research 1: Naming Convention

### Problem
CYNIC has no naming convention. 12 significant inconsistencies found across env vars, API fields, dog IDs, config files. What appeared as "IPs in scripts" was a symptom of systemic naming incoherence.

### Projects Researched

| Project | Relevance | Key pattern |
|---------|-----------|-------------|
| [Lighthouse](https://github.com/sigp/lighthouse) (Eth consensus, Rust, 19k stars) | Validators reaching consensus — same problem as CYNIC Dogs | `validator_definitions.yml`, pubkey as identity, `#[serde(tag = "type")]` for heterogeneous config |
| [Vector](https://github.com/vectordotdev/vector) (Datadog, Rust) | Infrastructure config, component registration | `[sources.NAME] type = "..."`, `VECTOR_*` env prefix, `STYLE.md` + `RUST_STYLE.md` |
| [OpenFang](https://github.com/RightNow-AI/openfang) (Agent OS, Rust) | Agent config, env var patterns | `OPENFANG_*` prefix, `api_key_env` indirection in TOML, `config.toml` |
| [ai-hedge-fund](https://github.com/virattt/ai-hedge-fund) | Multi-evaluator system | Agents named by strategy (Ben Graham, etc.), not by infrastructure |
| [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/naming.html) | RFC 430, canonical Rust naming | UpperCamelCase types, snake_case values, SCREAMING_SNAKE_CASE consts |
| [Mozilla Application Services](https://mozilla.github.io/application-services/book/naming-conventions.html) | Cross-platform Rust | Follow clippy + rustfmt, cross-language naming |
| [Martin Fowler — Ubiquitous Language](https://martinfowler.com/bliki/UbiquitousLanguage.html) | DDD naming in config | Domain vocabulary in domain contexts, anti-corruption layer for external |

### Naming Audit (current state)

12 significant inconsistencies found by agent audit:

1. `SOVEREIGN_API_KEY` vs `CYNIC_SOVEREIGN_KEY` — same secret, two names
2. `CYNIC_SOVEREIGN_URL` in env but never consumed by kernel
3. `"sovereign"` overloaded — health status AND sovereign infra (LLM on physical hardware)
4. `dogs_used` (String, joined) vs `dog_id` (String) — same concept, different names
5. `id` vs `dog_id` — dog identity has 3 different JSON keys across endpoints
6. `"deterministic-dog"` — only dog with `-dog` suffix (12 string literals scattered in code)
7. `<TAILSCALE_S>` vs `<TAILSCALE_GPU>` — same machine, two placeholder names
8. `SURREALDB_URL` protocol mismatch — `ws://` in setup-ubuntu.sh vs `http://` in deployed env
9. `domain/temporal.rs` — file exists, module commented out (dead architecture)
10. `"VERIFY/FALSIFY"` in API axioms list vs `verify` in domain types
11. `"hf-mistral"` in doc comment vs `"huggingface"` in actual config
12. `BackendInitError` is a struct while all other errors are enums

### Crystallized Decisions

**T1: "sovereign" is overloaded.** Health status → `healthy`/`degraded`/`critical`. "Sovereign" reserved for: LLM running on physical, open-source infrastructure managed by CYNIC.

**T2: Dog identity = generated hex ID + display_name.** Researched from Lighthouse (pubkey), Vector (user IDs), ai-hedge-fund (strategy names). Generated IDs needed for future clusters/federation. Named IDs collide across nodes.

**T3: Config migration `backends.toml` → `dogs.toml`.** Add `kind` field (inference/heuristic). Pattern: `#[serde(tag = "kind")]` serde tagged enum — same as Lighthouse's `SigningDefinition`. Zero new dependencies (toml 0.8 supports it natively).

**T4: `CYNIC_*` prefix for all internal env vars.** Third parties keep their namespace. Ghost vars eliminated.

**T5: `dog_id` everywhere in API.** Not `id`, not `dogs_used`. Frontend not consuming yet → rename is free.

**T6: DeterministicDog exits hardcoded code.** Enters config as `kind = "heuristic"`. Its ID is a generated hex, display_name = "Heuristic scorer". No more string literal `"deterministic-dog"` in 12 places.

**T7: Dog naming convention.** Individual dog names should not be infrastructure-derived (`sovereign-ubuntu`) because they break when infra changes. The identity is the hex ID; the display_name is descriptive and changeable.

### Output Document

`docs/NAMING-CONVENTION.md` — full convention with research sources, migration paths, current violations, mechanical enforcement plan.

---

## Research 2: Workflow Infrastructure

### Problem

Agent (Claude) repeatedly skips skills and commits without review. 4+ sessions of identical feedback. Workflow documented in CLAUDE.md (300+ lines, 32 rules, "Mandatory Tool Use") but nothing enforces it mechanically. Same pattern as code: documented rules without mechanical gates = hope engineering.

### Current `.claude/` state

```
CONFIGURED:
  .claude/commands/     → 9 commands (build, deploy, status, etc.)    ✓
  ~/.claude/commands/   → cynic-skills/ (empirical, burn, judge, etc.) ✓
  ~/.claude/plugins/    → superpowers, code-review, commit-commands    ✓
  ~/.claude/hooks/      → rtk-rewrite (token compression)             ✓

NOT CONFIGURED:
  .claude/rules/        → EMPTY
  .claude/agents/       → DOES NOT EXIST
  .claude/hooks/        → EMPTY (project-level)
  CLAUDE.md             → 300+ lines (too long, carries everything)
```

### Projects Researched

| Project | Key patterns |
|---------|-------------|
| [ChrisWiles/claude-code-showcase](https://github.com/ChrisWiles/claude-code-showcase) | **Skill evaluation hook**: `UserPromptSubmit` → JS engine matches prompts against `skill-rules.json` (keyword/path/intent scoring). `PreToolUse` blocks edits on main. `PostToolUse` auto-formats + auto-tests. `agents/code-reviewer.md` with model=opus. |
| [feiskyer/claude-code-settings](https://github.com/feiskyer/claude-code-settings) | 7 specialized agents: pr-reviewer, deep-reflector, instruction-reflector, etc. Tools restricted per agent (`tools: Read, Grep, Glob, Bash(gh:*)`). Plugin marketplace structure. |
| [alirezarezvani/claude-skills](https://github.com/alirezarezvani/claude-skills) | 192+ skills library organized by domain. |
| [Anthropic official guide](https://code.claude.com/docs/en/skills) | Canonical skill/agent/command structure. |

### Key Discovery: Skill Routing Hook

The showcase project's `skill-eval.js` is the missing piece. It's a `UserPromptSubmit` hook that:
1. Reads the user's prompt via stdin
2. Matches keywords, file paths, intent patterns against `skill-rules.json`
3. Scores each skill match (keyword=2, path=4, directory=5, intent=4)
4. Outputs a `user-prompt-submit-hook` feedback suggesting which skills to activate

This makes skill activation MECHANICAL — the agent doesn't need to remember which skills exist. The hook tells it. Same principle as compiler enforcement for code rules.

### Hook Architecture (from showcase)

```
UserPromptSubmit:
  → skill-eval: analyze prompt → suggest skills

PreToolUse(Edit|Write):
  → branch protection: block edits on main
  → (CYNIC: verify coord claim)

PostToolUse(Edit|Write):
  → auto-format (prettier → cargo fmt for Rust)
  → auto-test (jest → cargo check for Rust)
  → auto-typecheck (tsc → cargo clippy for Rust)
```

### Agent Design (from showcase + feiskyer)

```yaml
# agents/code-reviewer.md
---
name: code-reviewer
description: MUST BE USED PROACTIVELY after writing or modifying code.
model: sonnet   # opus for complex, sonnet for routine
tools: Read, Grep, Glob, Bash(cargo:*), Bash(git:*)
---
# Rust-specific review: clippy, unsafe, Rule violations, port trait compliance
```

### Crystallized Truths

**T1:** A workflow documented in CLAUDE.md without mechanical enforcement is hope engineering. 4+ sessions of identical feedback prove it. **→ Migrate conventions to rules/, steps to hooks, reviews to agents.**

**T2:** CLAUDE.md must go under 200 lines: principles + pointers to rules/. Everything else splits into path-scoped rules. **→ Immediate split needed.**

**T3:** The agent (Claude) is the weak link. Enforcement must be BELOW the agent (hooks, auto-invoked agents), not AT the agent (CLAUDE.md instructions). Same logic as compiler enforcement for code. **→ Hooks pre-commit, agents auto-invoked, rules path-scoped.**

**T4:** The tools already exist (19 skills, plugins, commands). The problem is not tooling — it's ROUTING to tooling. **→ Skill routing hook is the fix.**

---

## Next Sessions (ordered)

### Session A: Workflow Infrastructure (prerequisite for everything)
- Split CLAUDE.md → < 200 lines + rules/ files
- Create agents/ (code-reviewer, pre-commit-auditor)
- Configure hooks in project settings.json:
  - `UserPromptSubmit` → skill router (bash, match keywords → suggest cynic-skills)
  - `PreToolUse(Edit|Write)` → block on main, verify coord claim
  - `PostToolUse(Edit|Write)` → `cargo check` on .rs files
- Test the workflow on a small change

### Session B: Naming Convention Implementation
- With workflow enforced, implement `docs/NAMING-CONVENTION.md`:
  - `sovereign` → `healthy` in system_health_status
  - `backends.toml` → `dogs.toml` with `kind` field
  - Dog ID migration (hex IDs)
  - Env var cleanup
  - API field alignment (`dog_id` everywhere)

### Session C: RC Fixes (RC5 + review RC8/RC2)
- With workflow + naming in place, implement RC5 (silent failures)
- Review RC8/RC2 commits through the new code-reviewer agent
- Continue RC3→RC7 per audit compound order

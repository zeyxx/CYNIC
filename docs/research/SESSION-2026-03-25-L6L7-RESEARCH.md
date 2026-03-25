# L6-L7 Research Findings — CLAUDE.md + .claude/rules/ Design

*2026-03-25. 5 research agents, crystallized. Ready to implement next session.*

## Critical Bug

**`paths:` frontmatter in .claude/rules/ is BROKEN.** Use `globs:` instead.
- GitHub issues: #17204, #16299, #23478, #21858, #23569
- Safe format: `globs: ["cynic-kernel/**/*.rs"]` (quoted, in array)
- Do NOT use: `paths: ["cynic-kernel/**"]`

## Academic Findings

**Instruction saturation (NeurIPS 2025, arXiv 2507.11538):**
- Frontier LLMs: 68% accuracy at 500 instructions, ~100-150 = practical threshold
- Claude Code system prompt uses ~50 slots → 100-150 left for CLAUDE.md + rules
- CYNIC currently: 60-80+ instructions → AT the threshold

**"Lost in the Middle" (ACL 2024, arXiv 2307.03172):**
- 30%+ compliance penalty for rules in mid-position
- Best attention: positions 1-5 and last 5

**Context rot (Chroma 2025):**
- Degradation at EVERY input length increment, not just near limit
- U-shaped: best at start and end, worst in middle

**Primacy effect (arXiv 2507.13949):**
- Fine-tuning amplifies positional bias
- Most-violated rules → position 1 AND last position (double reinforcement)

## Practical Findings

**Positive framing > negative (~50% violation reduction):**
- "Use `?` for error propagation" > "Don't use unwrap()"

**2-3 IMPORTANT markers max per file.** More = noise.

**Prompt caching:** CLAUDE.md content auto-cached by Claude Code (~90% savings on repeated turns). Threshold: 2048 tokens (Sonnet), 4096 (Opus).

**Session start token budget:**

| Source | Tokens |
|--------|--------|
| CLAUDE.md | ~4,300 |
| MEMORY.md index | ~5,700 |
| MCP tool schemas | ~2,000 |
| session-init output | ~300 |
| Top crystals | ~200 |
| **Total before work** | **~12,700** |

## CLAUDE.md Audit — What to Cut

**Delete entirely (100% redundant with L0-L3):**
- Rule 3 (conventional commits) — commit-msg hook enforces
- Rule 13 (compiler is enforcement) — meta-rule, not actionable
- "Before every commit verify no secrets" grep — lint-rules Makefile

**Shorten (grep already in lint-rules, remove "Falsifiable: grep X" suffixes):**
- Rules 8, 17, 19, 22, 32 — just say "enforced by `make lint-rules`"

**Move to reference doc (data, not instructions):**
- API endpoint table (21 lines, ~400 tokens)
- Dogs table (8 lines)
- φ constants (7 lines)
- Infrastructure table (7 lines)

## CLAUDE.md Target Structure (~40 lines)

```
Lines 1-2:   Identity ("CYNIC is an epistemic immune system")
Lines 3-12:  Security (non-negotiable: no IPs, no keys, no names, placeholders)
Lines 13-15: Ownership zones (T.=kernel, S.=frontend)
Lines 16-28: Skill routing table (rewritten with specific triggers)
Lines 29-35: Top 3 most-violated rules (primacy position)
Lines 36-40: Canonical references + pointer to .claude/rules/
```

## .claude/rules/ Structure (4 files)

```
.claude/rules/
├── universal.md          # Judgment rules always loaded (~50 lines)
│                         # Rules: 1,2,5,6,8,9,12,14-16,18,20-21,24-28,31,33
├── kernel.md             # Rust-specific, lazy loaded
│   globs: ["cynic-kernel/**"]
│   # Rules: 4,10-11,17,22,32 + build notes
├── workflow.md           # Tool use triggers, always loaded (~30 lines)
│                         # Improved BEFORE/AFTER/ON with specific triggers
└── reference.md          # API table, Dogs, φ, infra — always loaded
                          # Pure reference data, no instructions
```

## Skill Routing Triggers — Rewritten

**BEFORE triggers (current ~30% compliance → rewritten for specificity):**

| Current (vague) | Rewritten (specific/falsifiable) |
|-----------------|----------------------------------|
| "Before building something new" | "Before adding a new module, crate dependency, or Cargo.toml change" |
| "Before a decision under uncertainty" | "Before choosing between two architecturally different approaches" |

**Missing triggers to add:**
- "Before adding a Cargo.toml dependency → cynic-empirical"
- "When a test fails on first run → grep codebase for same pattern (Rule #12)"

**AFTER trigger fix:**
- "After significant session work" → "After modifying >5 files or >100 lines"

## Hermes Agent Insights

**SOUL.md pattern:** Identity in one file, rules separate, skills injected. Maps to: CLAUDE.md (identity) / rules/ (rules) / skills (injected).

**Tool routing = tool calling.** Subagent descriptions ARE tool schemas. Write them with function-signature precision: what it does, when to call, what it needs, when NOT to use.

**Imperative keywords that work:** PROACTIVELY, MUST BE USED, Invoke IMMEDIATELY — these are RLHF training signals, not just caps for emphasis.

**Hermes periodic nudges:** Agent nudges itself to persist knowledge. Maps to CYNIC's /distill skill.

## Claude Agent SDK (for future sessions)

Same engine as CLI, programmatic interface. Key capabilities beyond CLI:
- In-process hooks (Python/TS callbacks, not shell scripts)
- `tool_choice` forcing (guarantee specific tool called)
- `max_budget_usd` (cost cap per session)
- Session forking (A/B testing)
- In-process custom tools (no MCP subprocess needed)
- `settingSources: ["project"]` required — SDK doesn't load .claude/ by default

## Rule-to-Layer Mapping (33 rules)

| Rule | Enforcement Layer | Status |
|------|-------------------|--------|
| 1 Diagnose before fixing | L6 rules/ (judgment) | Keep in universal.md |
| 2 Two fix attempts max | L6 rules/ (judgment) | Keep in universal.md |
| 3 Conventional commits | L2 git hook (mechanical) | **DELETE from CLAUDE.md** |
| 4 Domain purity #[cfg] | L1 lint-rules grep | Keep in kernel.md (judgment: why) |
| 5 Port contracts first | L6 rules/ (judgment) | Keep in universal.md |
| 6 Bounded everything | L6 rules/ (judgment) | Keep in universal.md |
| 7 Zero hardcoded paths | L1 lint-rules grep | Shorten (grep exists) |
| 8 Never discard fallible I/O | L0 clippy + L1 grep | Shorten (partially mechanical) |
| 9 Wire or delete | L0 dead_code lint | Shorten (compiler catches) |
| 10 Timeout background await | L6 rules/ (judgment) | Keep in kernel.md |
| 11 Display implies Error | L6 rules/ (judgment) | Keep in kernel.md |
| 12 Fix class not instance | L6 rules/ (judgment) | Keep in universal.md |
| 13 Compiler is enforcement | Meta-rule | **DELETE** (philosophy, not instruction) |
| 14 One value one source | L6 rules/ (judgment) | Keep in universal.md |
| 15 HTTP status codes | L6 rules/ (judgment) | Keep in universal.md |
| 16 Scripts thin, logic in kernel | L6 rules/ (judgment) | Keep in universal.md |
| 17 Adapters through port trait | L1 lint-rules grep | Shorten |
| 18 SQL queries have tests | L6 rules/ (judgment) | Keep in kernel.md |
| 19 No logic duplication | L1 lint-rules grep | Shorten |
| 20 Research testing patterns | L6 workflow.md | Keep |
| 21 No dead architecture | L6 rules/ (judgment) | Keep in universal.md |
| 22 No trait name collisions | L1 lint-rules grep | Shorten |
| 23 Validate feedback loop | L6 workflow.md | Keep |
| 24 Name things for what they ARE | L6 rules/ (judgment) | Keep in universal.md |
| 25 Fix→Test→Gate→Verify | L6 rules/ (meta-rule) | Keep in universal.md + CLAUDE.md (primacy) |
| 26 Strong > no > weak foundation | L6 rules/ (judgment) | Keep in universal.md |
| 27 Compound organically | L6 rules/ (judgment) | Keep in universal.md |
| 28 Agents use the platform | L6 rules/ (judgment) | Keep in universal.md |
| 29 Deploy from main only | L1 Makefile (partial) | Keep — gap: no branch gate |
| 30 Commit before completing | L3 session-stop hook | Shorten (hook warns) |
| 31 Measure before and after | L6 rules/ (judgment) | Keep in universal.md |
| 32 No cross-layer type leakage | L1 lint-rules grep | Shorten |
| 33 Every producer needs consumer | L6 rules/ (judgment) | Keep in universal.md |

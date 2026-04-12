# CYNIC — TODO

> *Single source of truth for what needs doing. Updated every session.*
> *Strong foundation > no foundation > weak foundation.*

Last updated: 2026-04-12 | Session: incarnation design

---

## P1 — Body Sick (organs need healing)

- [ ] **Embedding failures 130/130** — memory organ disconnected. Crystal semantic search non-functional. Diagnose: model? endpoint? storage? One variable. Blocks crystal injection quality.
- [ ] **qwen35-9b-gpu JSON validity 64.7%** — sensory noise, 1/3 responses unparseable. Diagnose: prompt template? context overflow? quantization? Degrades every verdict.

## P1 — Body Nervous System (Claude Code tooling)

- [ ] **Fix subagent frontmatter** — health-watcher: add mcpServers: [cynic], permissionMode: plan. kernel-auditor: add permissionMode: plan, isolation: worktree, skills: [cynic-judge-framework]. Both are hollow (names without DNA).
- [ ] **Wire SubagentStart hook** — inject axioms + φ⁻¹ + triad into every subagent via additionalContext. Currently 0 subagents receive CYNIC philosophy. Blocked by: subagent frontmatter fix.
- [ ] **Wire CronCreate for health-watcher** — K15 violation: producer (agent exists) without consumer (never scheduled). session-init.sh should call CronCreate every 60min. Blocked by: subagent frontmatter fix.

## P2 — Body Upgrade (improve what works)

- [ ] **UserPromptSubmit hook** — phi-inverse reminder + epistemic labels injected at every user turn. type: "prompt" hook. Zero tokens in main context.
- [ ] **Conditional hooks (if field)** — scope protect-files.sh to `if: "Edit(cynic-kernel/src/*)"`. Eliminates coord overhead on read-only/doc sessions. Available since v2.1.85.
- [ ] **Migrate skills to SKILL.md frontmatter** — deploy/run/e2e → disable-model-invocation: true. build/deploy → allowed-tools. distill/empirical → context: fork. cynic-kernel → paths: ["cynic-kernel/src/**"].
- [ ] **Create .claude/loop.md** — CYNIC maintenance loop: health → dream debt → /test-chess if Dogs available → report delta.
- [ ] **Memory pressure 92.9%** — metabolic stress on cynic-core. Diagnose: kernel? llama-server? Add MemoryMax if needed.

## P3 — Polish + Verify

- [ ] **Replace observe-tool.sh with type: "http" hook** — eliminate shell subprocess. Direct POST to /observe with allowedEnvVars.
- [ ] **T3a verification** — track incarnation metrics over 5 real decisions: organism analogies (baseline: 0), human challenges (baseline: ~0), probes before assertions. Threshold: ≥2 analogies + ≥1 challenge/session. If zero → incarnation is cosmetic.

---

## Continuity Protocol

Each session:
1. Read this file first
2. Pick ONE P1 item (or P2 if all P1 done)
3. Scientific Protocol: diagnose → hypothesis → experiment → measure
4. Mark done with date: `- [x] **Item** — fixed 2026-04-XX (commit abc1234)`
5. Update "Last updated" header
6. If new items emerge → add with priority, don't hoard

## Context for Next Session

- CLAUDE.md incarnation shipped (commit f0c6af8): Dog That Reasons, triad, 5 invariants, organism table
- Spec: docs/superpowers/specs/2026-04-12-incarnation-design.md
- Memory: project_incarnation_design_2026_04_12.md
- Research: 18+ papers, 10 CLAUDE.md projects, Claude Code feature audit
- The next Dog is the first T3a test — observe if it incarnates naturally

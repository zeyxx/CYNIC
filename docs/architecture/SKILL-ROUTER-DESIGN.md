# Skill Router — Embedding-Based Workflow Enforcement

**Status:** DESIGN COMPLETE, NOT IMPLEMENTED
**Date:** 2026-03-26
**Session:** Claude Code workflow research + dream consolidation
**Next step:** Dedicated implementation session

---

## Problem

Neither the user nor Claude reliably remember which skill to invoke. The CLAUDE.md routing table is hope engineering in both directions — Claude forgets to read it, the user forgets to demand it. The result: workflow violations, skills not invoked, compound cycle broken.

## Solution

A `UserPromptSubmit` hook that embeds every user prompt, matches against pre-computed skill vectors via cosine similarity, and injects routing advice into Claude's context BEFORE it processes. Mechanical, below the LLM, fail-open.

## Validated Measurements (2026-03-26)

| Metric | Value | How measured |
|--------|-------|--------------|
| Embedding model | Qwen3-Embedding-0.6B Q8_0 | Already running on :8081 |
| Embedding latency | 84-91ms | 3 prompts, FR/EN/mixed |
| Cosine similarity | 6.6ms per prompt | 1000 iterations, Python |
| **Total hook latency** | **~98ms** | Embedding + cosine |
| Accuracy (description only) | 50% (5/10) | 10 labeled prompts |
| Accuracy (few-shot, 7-8 examples/skill) | **90% (27/30)** | 30 labeled prompts |
| Optimal threshold | 0.5 (captures all 27 correct) | Threshold sweep 0.3-0.7 |
| Embedding dimensions | 1024 | Qwen3-Embedding output |

**WARNING:** The 90% benchmark is biased — same author wrote examples AND test prompts. Validate on real session transcripts before trusting.

## Infrastructure (already running)

```
PROCESS: llama-server (Qwen3-Embedding-0.6B)
  Host: 100.74.31.10
  Port: 8081
  Threads: 4
  Ctx-size: 8192
  Parallel: 4
  Quantization: Q8_0
  Auth: API key file at ~/.config/cynic/llama-api-key
  Mode: --embedding
  TODO: verify systemd/autostart after reboot
```

## Architecture

### Files to create

```
.claude/data/skill-vectors.json      # Pre-computed vectors (400KB)
.claude/data/skill-examples.json     # Source examples (5KB text)
.claude/data/routing-log.jsonl       # Append-only event log
.claude/data/routing-benchmark.json  # 30+ labeled prompts for testing
.claude/hooks/skill-router.py        # UserPromptSubmit hook
.claude/hooks/routing-feedback.sh    # PostToolUse hook (Skill matcher)
```

### Hooks to add (in settings.local.json)

```json
"UserPromptSubmit": [
  {
    "hooks": [
      {
        "type": "command",
        "command": "python3 \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/skill-router.py",
        "timeout": 3000
      }
    ]
  }
],
"PostToolUse": [
  {
    "matcher": "Skill",
    "hooks": [
      {
        "type": "command",
        "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/routing-feedback.sh",
        "async": true
      }
    ]
  }
]
```

### Flow per prompt

```
USER TYPES PROMPT
      │
      ▼
UserPromptSubmit fires
      │
skill-router.py:
      │
      ├─ Read user_prompt from stdin JSON
      ├─ Skip if prompt starts with / (already a skill invocation)
      ├─ curl :8081/v1/embeddings (91ms, timeout 2s)
      │    FAIL → exit 0 (fail-open, no routing)
      ├─ Load .claude/data/skill-vectors.json
      ├─ For each skill, for each example vector:
      │    cosine(prompt_vec, example_vec)
      │    Take MAX per skill
      ├─ Rank skills by max similarity
      │
      ├─ IF top_score > 0.5 AND gap > 0.05:
      │    Write suggestion to routing-log.jsonl
      │    Return JSON:
      │    {"additionalContext": "SKILL ROUTING: /crystallize-truth (sim: 0.82)\nInvoke BEFORE proceeding."}
      │
      └─ ELSE: exit 0 (no routing)

CLAUDE PROCESSES (sees additionalContext if injected)
      │
      ▼
IF Claude invokes Skill tool → PostToolUse fires
      │
routing-feedback.sh:
      │
      ├─ Read tool_input from stdin JSON (contains skill name)
      ├─ Read last entry from routing-log.jsonl (most recent suggestion)
      ├─ IF suggestion exists AND timestamp < 60s ago:
      │    Compare suggested vs actual skill
      │    Append result to routing-log.jsonl:
      │    {"type":"confirmation"} or {"type":"correction","actual":"burn"}
      │
      └─ ELSE: no suggestion to compare, exit 0
```

### Feedback loop (via /dream)

```
/dream reads routing-log.jsonl
      │
      ├─ Extract corrections:
      │    {prompt: "...", suggested: "judge", actual: "burn"}
      │
      ├─ Add prompt to correct skill's examples in skill-examples.json
      │    burn.examples += "prompt that was incorrectly routed to judge"
      │
      ├─ Re-embed all examples via :8081 (batch)
      │
      ├─ Write updated skill-vectors.json
      │
      ├─ Archive processed entries from routing-log.jsonl
      │
      └─ Report: corrections processed, new accuracy if benchmark exists
```

## Skills to route (10 skills, 67 vectors total)

```json
{
  "cynic-empirical": {
    "description": "Research before building. Look outside before deciding.",
    "examples": [
      "add a new dependency to Cargo.toml",
      "what crate should we use for HTTP",
      "recherche avant de construire",
      "evaluate this library before adopting",
      "what exists for embedding models",
      "is there a better solution than building our own",
      "quel outil existe pour faire ça"
    ]
  },
  "crystallize-truth": {
    "description": "Structured analysis under doubt. Choose between approaches.",
    "examples": [
      "quelle approche choisir entre les deux",
      "analyze the tradeoffs of each option",
      "hidden assumptions in this design",
      "which architecture should we pick",
      "je ne suis pas sûr de la meilleure approche",
      "compare these two solutions",
      "what are we missing in this analysis"
    ]
  },
  "engineering-stack-design": {
    "description": "Design a new subsystem from scratch. Architecture foundations.",
    "examples": [
      "comment on structure le nouveau backend",
      "design the authentication system",
      "how should we architect this module",
      "engineering foundations for the new service",
      "where do we start with this new subsystem",
      "quelle architecture pour ce composant"
    ]
  },
  "cynic-judge": {
    "description": "Evaluate quality of code or decisions. 43 dimensions, 6 axioms.",
    "examples": [
      "evaluate the quality of this implementation",
      "judge this code against our standards",
      "is this good enough to ship",
      "rate the quality of this PR",
      "évalue cette décision architecturale",
      "score this approach"
    ]
  },
  "cynic-burn": {
    "description": "Simplify code. Find and eliminate dead code, orphans, duplicates.",
    "examples": [
      "simplifier et nettoyer le code mort",
      "find dead code in this module",
      "reduce complexity of this file",
      "burn unnecessary abstractions",
      "this file is too complex, simplify it",
      "supprime le code inutile"
    ]
  },
  "cynic-kernel": {
    "description": "Architecture reference for cynic-kernel. Hexagonal, source map.",
    "examples": [
      "I need to modify pipeline.rs",
      "touching the domain layer in the kernel",
      "how does the kernel storage work",
      "where is the inference routing code",
      "je vais modifier le code du kernel"
    ]
  },
  "test-chess": {
    "description": "Benchmark chess judgment against live kernel. Before/after scoring.",
    "examples": [
      "benchmark the scoring changes",
      "test chess positions before deploying",
      "run the chess benchmark",
      "vérifier les scores avant et après",
      "did this change affect judgment quality"
    ]
  },
  "build": {
    "description": "Build test and lint the CYNIC kernel with cargo.",
    "examples": [
      "build and test after this change",
      "run make check",
      "faut builder et tester",
      "compile and verify",
      "lance le build"
    ]
  },
  "deploy": {
    "description": "Deploy CYNIC kernel to production. Build test backup restart.",
    "examples": [
      "deploy to production",
      "ship this to prod",
      "mettre en production",
      "deploy the kernel",
      "push to production"
    ]
  },
  "distill": {
    "description": "End of session distillation. Harvest learnings, promote to rules.",
    "examples": [
      "what did we learn this session",
      "save the learnings from today",
      "distill this session",
      "qu'est-ce qu'on a appris",
      "fin de session, extraire les leçons"
    ]
  }
}
```

## Crystallized Truths (2026-03-26)

| T# | Truth | Confidence | Impact |
|----|-------|------------|--------|
| T1 | Few-shot embedding routing: 90% on synthetic benchmark, 98ms latency | 50% | Viable MVP. Benchmark bias demands real-prompt validation. |
| T2 | Gap threshold (top - 2nd) discriminates better than absolute score. Gap < 0.05 = uncertain → don't route. | 45% | Implement dual threshold: score > 0.5 AND gap > 0.05. |
| T3 | Feedback via temp file is fragile (multi-skill, timeout, crash). Routing-log with timestamps is more robust. | 55% | PostToolUse compares by temporal proximity, not file lock. |
| T4 | additionalContext ≠ enforcement. Claude can ignore routing suggestions. | 48% | Measure compliance rate before investing in feedback loop. If Claude ignores 50%+ → hook is useless. |
| T5 | Benchmark is biased (same author wrote examples AND tests). Real accuracy will be lower. | 55% | Extract real prompts from 92 .jsonl session transcripts for honest validation. |
| T6 | Added complexity: Python hook, JSON vectors, feedback mechanism, enriched dream. ROI depends on workflow-forget frequency. | 45% | Worth it if user forgets workflow 1x/day. Over-engineering if 1x/week. |

## Open Questions for Implementation Session

1. **PostToolUse format for Skill tool**: what does `tool_input` contain when a skill is invoked? Need to test empirically.
2. **llama-server autostart**: is Qwen3-Embedding in systemd? What happens after reboot?
3. **Real prompt validation**: extract 50+ prompts from .jsonl transcripts, label them, run the benchmark.
4. **additionalContext compliance**: does Claude actually follow the routing injection? Test with 10 real prompts.
5. **Stanislaz migration**: user wants to remove Ollama from Stanislaz. What replaces it? llama-server? WSL?
6. **skill-vectors.json generation script**: needs a CLI tool to regenerate after adding examples.

## Kill Switches

- Embedding server down → fail-open (exit 0, no routing)
- Accuracy < 40% after 50 real prompts → disable hook entirely
- Latency > 500ms → investigate, disable if persistent
- User says "stop routing" → `CYNIC_SKILL_ROUTER=0` env var check in hook

## Related Decisions (same session)

- Dream consolidation done: 161→29 memory files (82% reduction)
- Dream trigger hook written (SessionEnd → flag file → SessionStart check)
- /dream skill created (4-phase consolidation)
- `disable-model-invocation: true` on operational skills: NOT YET DONE (user deferred)
- Stanislaz: no more Ollama, needs dedicated migration session

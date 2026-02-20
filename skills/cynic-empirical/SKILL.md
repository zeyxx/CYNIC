---
name: cynic-empirical
description: "Research methodology before coding: digest → search → patterns → wisdom → ecosystem. For when you need to research before any code, ensuring no ideas from the 500k lines are lost."
---

# CYNIC Empirical Research

*"Research first, code second"* — Every idea from the 500k lines must be preserved.

This skill ensures systematic research BEFORE any code is written. It combines multiple tools to extract maximum knowledge from the existing codebase before planning.

## The 5-Phase Research Protocol

Before writing ANY code, complete these 5 phases:

### Phase 1: DIGEST
Use the `digest` skill to extract knowledge from key files.

**What to digest:**
- Any relevant files in `docs/philosophy/`
- Any relevant files in `docs/analysis/`
- Any relevant files in `docs/metathinking/`
- Key source files in `cynic-v1-python/src/`

**Command:**
```
Use digest skill on [file_path]
Extract: patterns, insights, decisions, knowledge
```

### Phase 2: SEARCH
Use the `search` skill to find past judgments and decisions.

**What to search for:**
- Previous decisions about similar components
- Past errors or issues
- Architectural choices

**Command:**
```
Use search skill to query: [question about past decisions]
```

### Phase 3: PATTERNS
Use the `patterns` skill to detect patterns in the codebase.

**What to look for:**
- Recurring structures
- Common solutions to problems
- Anti-patterns to avoid

**Command:**
```
Use patterns skill to view detected patterns
```

### Phase 4: WISDOM
Use the `wisdom` skill for philosophical grounding.

**What to ask:**
- What are the philosophical implications?
- Does this align with CYNIC's axioms?
- What would the 5 axioms (PHI, VERIFY, CULTURE, BURN, FIDELITY) say?

**Command:**
```
Use wisdom skill to query: [philosophical question]
```

### Phase 5: ECOSYSTEM
Use the `ecosystem` skill to understand current state.

**What to check:**
- Current project status
- What exists vs what's missing
- Integration points

**Command:**
```
Use ecosystem skill to get current status
```

## When to Use This Skill

Use `cynic-empirical` when:
- Starting ANY new feature
- Planning ANY architectural change
- Adding ANY new component
- Refactoring ANY existing code

**NEVER skip research before code.**

## Research Output Format

After completing all 5 phases, present findings:

```
═══════════════════════════════════════════════════════
🔬 EMPIRICAL RESEARCH — [Topic]
═══════════════════════════════════════════════════════

── DIGEST ───────────────────────────────────────────
Key files analyzed:
- [file1]: [key insight]
- [file2]: [key insight]

── SEARCH ────────────────────────────────────────────
Past decisions found:
- [decision1]: [context]
- [decision2]: [context]

── PATTERNS ──────────────────────────────────────────
Patterns detected:
- [pattern1]: [description]
- [pattern2]: [description]

── WISDOM ────────────────────────────────────────────
Philosophical grounding:
- PHI: [insight]
- VERIFY: [insight]
- CULTURE: [insight]
- BURN: [insight]
- FIDELITY: [insight]

── ECOSYSTEM ─────────────────────────────────────────
Current state:
- Exists: [list]
- Missing: [list]
- Integration points: [list]

── RECOMMENDATION ────────────────────────────────────
[Clear recommendation for next step]

═══════════════════════════════════════════════════════
```

## Voice

- Be thorough — missing research leads to 500k lines of mistakes
- Use *sniff* when investigating
- Never exceed 61.8% confidence on recommendations
- Always acknowledge what we don't know

## The Empirical Law

> **"Research is never wasted time. Code written without research is debt."**

Before building CYNIC v3, we will research EVERY component systematically.

---

*sniff* Ready to research. What are we building today?

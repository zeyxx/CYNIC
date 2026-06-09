---
name: cynic-empirical
description: "Research reflex — look OUTSIDE before building or deciding. Hypothesis-driven investigation using external sources. Use when entering unfamiliar territory, evaluating build-vs-adopt, or before any decision that depends on what exists beyond your codebase."
---

# CYNIC Empirical — The Dog That Sniffs Before It Digs

*"Don't build what you don't understand. Don't understand what you haven't investigated."*

## When to Use

- Before building something that might already exist
- Entering unfamiliar domain, library, or practice
- Evaluating a technology choice that locks in a dependency
- When the answer depends on what's OUT THERE, not what's in your head

**NOT for:** Analyzing what you already know (→ crystallize-truth), architecting a system (→ engineering-stack-design), judging quality (→ cynic-judge).

## The 5 Steps

### 1. QUESTION
State exactly what you're trying to find out. One sentence, falsifiable.
Vague questions produce vague research.

### 2. HYPOTHÈSE
What do you EXPECT to find? State it before searching.
Research without a hypothesis is browsing.

### 3. CHERCHE DEHORS
Go look. Use what's available:
- **context7 MCP** — up-to-date library docs
- **WebSearch** — GitHub repos, papers, forums, awesome lists
- **Live system** — probe infrastructure directly (ts_exec, curl)
- **Git history** — what was tried before and why it was abandoned

**Rules:** Follow citation chains. Check recency (last commit, last release). Don't stop at the first result.

### 4. CHALLENGE
What would falsify your conclusion? Apply the strongest counterargument.
If you can't find one, you haven't searched hard enough.

### 5. DÉCIDE
Three options, pick one:

| Decision | When |
|----------|------|
| **Adopt** | Exists, fits, maintained, not single-maintainer |
| **Build** | Core domain, or needs heavy adaptation, or nothing fits |
| **Skip** | Not needed yet — don't solve hypothetical problems |

State confidence (max 61.8%). If inconclusive after 2-3 search rounds: state uncertainty, pick lowest-risk option, move on.

## Anti-Patterns

- **Post-hoc research** — deciding first, then searching for confirmation
- **First-result bias** — adopting the first thing that looks right
- **Infinite research** — 2-3 rounds max, then decide. Research is not the work.
- **Ignoring recency** — a 2023 project may be dead. Check pulse before adopting.
- **Building ego** — "I can build it better" is not a reason when a maintained solution exists

## Output

No rigid template. Just answer:
1. What did you find?
2. Build, adopt, or skip?
3. Why? (with confidence)

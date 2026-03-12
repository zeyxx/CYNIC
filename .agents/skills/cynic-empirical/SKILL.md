---
name: cynic-empirical
description: "5-phase research protocol before building — DIGEST, SEARCH, PATTERNS, WISDOM, ECOSYSTEM. Use when entering an unfamiliar domain, evaluating a technology, or before making architecture decisions that depend on external knowledge. Triggers on: 'research this', 'what exists', 'before we build', 'is there a better way', 'state of the art', 'what does the ecosystem offer'."
---

# CYNIC Empirical — The Dog That Sniffs Before It Digs

*"Don't build what you don't understand. Don't understand what you haven't investigated."*

## When to Use

- Entering a domain you haven't worked in before
- Evaluating whether to build vs adopt
- Before architecture decisions that depend on external knowledge
- When the user asks "what exists?" or "is there a better way?"
- Before any technology choice that locks in a dependency

**NOT for:** Internal code decisions, refactoring existing code, debugging. Those have their own skills.

## The 5 Phases

```
DIGEST ──▶ SEARCH ──▶ PATTERNS ──▶ WISDOM ──▶ ECOSYSTEM
  │                                               │
  └──── loop back if gaps found ◀────────────────┘
```

Complete each phase before starting the next. Do not collapse them.

---

### Phase 1 — DIGEST

**Goal:** Understand the raw material. What do we actually know?

1. **Read the primary sources** — Not summaries. Not blog posts. The actual docs, papers, READMEs, source code.
2. **Extract claims** — What does this technology/approach claim to do?
3. **Separate fact from marketing** — "Blazing fast" is marketing. "50ms p99 on 10K QPS" is a fact.
4. **Note what's NOT said** — Limitations, failure modes, what the docs avoid mentioning.

**Output:** A list of verified claims with sources.

```
CLAIM: [specific, falsifiable statement]
SOURCE: [URL or document reference]
VERIFIED: [yes/no/partially — how?]
```

---

### Phase 2 — SEARCH

**Goal:** Find everything relevant. Cast wide, filter later.

Search across:
- **GitHub** — repos, issues, discussions, stars, recent activity
- **arxiv / papers** — academic research if applicable
- **skills.sh / awesome-agent-skills** — existing agent skills
- **Awesome lists** — curated community collections
- **Forums** — HN, Reddit, Discord, specific community forums
- **Official docs** — always the primary reference

**Search strategy:**
1. Start with the obvious keywords
2. Follow citation chains — what do the best repos reference?
3. Check "Alternatives" sections in READMEs
4. Look at who stars/forks — what else do they build?
5. Check recency — a 2024 solution may be obsolete in 2026

**Output:** A categorized list of discoveries.

```
| Discovery | Type | Relevance | Recency | URL |
|-----------|------|-----------|---------|-----|
```

---

### Phase 3 — PATTERNS

**Goal:** What patterns emerge from the search? What does the field converge on?

1. **Group discoveries by approach** — How many independent teams arrived at the same solution?
2. **Identify convergence** — If 5 projects use the same pattern, it's probably correct.
3. **Identify divergence** — Where projects disagree, that's where the interesting tradeoffs live.
4. **Map to our needs** — Which patterns solve OUR problem vs a similar-but-different problem?

**Output:** Pattern map.

```
PATTERN: [name]
CONVERGENCE: [X out of Y projects use this]
DIVERGENCE: [where/why projects disagree]
FIT: [how this maps to our specific problem]
TRADEOFF: [what we give up by adopting this]
```

---

### Phase 4 — WISDOM

**Goal:** Apply judgment. What should we actually do?

1. **Filter by sovereignty** — Does this create a dependency we can't control? If yes, stronger justification required.
2. **Filter by complexity** — Is the simplest option good enough? Don't adopt complexity for its own sake.
3. **Filter by maturity** — How battle-tested is this? Stars don't equal stability.
4. **Build vs Adopt decision:**

| Factor | Build | Adopt |
|--------|-------|-------|
| Exists and fits perfectly | | X |
| Exists but needs heavy adaptation | X (often cheaper) | |
| Doesn't exist | X | |
| Core domain (our competitive advantage) | X | |
| Commodity infrastructure | | X |
| Maintained by 1 person | X (bus factor) | |

5. **State the recommendation** — With confidence (φ-bounded, max 61.8%).

---

### Phase 5 — ECOSYSTEM

**Goal:** How does our decision fit in the broader ecosystem?

1. **Upstream dependencies** — What do we depend on? How healthy are those projects?
2. **Downstream impact** — Who might use what we build? Could this be a contribution back?
3. **Community alignment** — Are we swimming with or against the current?
4. **Future trajectory** — Where is the field heading? Are we building for today or tomorrow?

**Output:** Ecosystem map and final recommendation.

---

## Output Format

```
*sniff* Empirical research complete.

═══════════════════════════════════════════════════════
EMPIRICAL ANALYSIS — [topic]
═══════════════════════════════════════════════════════

── DIGEST ──────────────────────────────────────────────
  [Key verified claims, 3-5 bullets]

── SEARCH ──────────────────────────────────────────────
  [Top discoveries with relevance, categorized]

── PATTERNS ────────────────────────────────────────────
  [Convergence/divergence patterns, 2-4 patterns]

── WISDOM ──────────────────────────────────────────────
  [Build vs Adopt for each component]
  [Recommendation with confidence]

── ECOSYSTEM ───────────────────────────────────────────
  [Fit assessment, upstream/downstream, trajectory]

Confidence: XX% (φ-bounded, max 61.8%)
═══════════════════════════════════════════════════════
```

## Anti-Patterns

| Anti-pattern | Fix |
|-------------|-----|
| Searching only GitHub | Search papers, forums, skills.sh, awesome lists too |
| Stopping at Phase 2 | SEARCH without PATTERNS is a link dump, not research |
| Adopting the first thing found | Complete all 5 phases. The first result is rarely the best. |
| Ignoring recency | A 2023 project may be abandoned. Check last commit date, issue activity. |
| Building when adoption fits | If it exists, works, and is maintained — adopt. Ego is not a reason to build. |
| Adopting when building fits | If it's our core domain or needs heavy modification — build. |
| Skipping sovereignty check | Every external dependency is a potential capture vector |

## Connected Mode

This skill produces research artifacts that feed into:
- **crystallize-truth** — When research reveals tensions that need crystallization
- **engineering-stack-design** — When research informs technology choices
- **cynic-kernel** — When research validates or challenges architecture decisions

> *sniff* "The empirical dog digs in many yards before choosing where to bury the bone."

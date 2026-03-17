---
name: crystallize-truth
description: Use when analyzing complex problems before design decisions, architectural choices, or documentation — especially when assumptions might be hidden, when intuition might be wrong, or when the stakes of getting it wrong are high.
---

# Crystallize Truth — Structured Analysis Under Doubt

*"The map is not the territory. The model is not the system. Test both."*

Three independent layers. Applied sequentially. Never collapse them into one pass.
Each layer questions the previous. Stop when conclusions stop changing.

---

## Layer 1 — 10 Modes of Thinking

Apply each mode as a distinct lens. Do not merge them. Write findings per mode.

| # | Mode | Core Question |
|---|---|---|
| 1 | **Causal** | What causes what? Root causes vs symptoms — never confuse them |
| 2 | **Abstract** | What is the highest-level principle? Strip all specifics |
| 3 | **Nonlinear** | What feedback loops exist? What amplifies? What's the observer effect? |
| 4 | **Recursive** | Does this structure appear at other scales? Is it self-referential? |
| 5 | **Epistemic** | What do we KNOW vs ASSUME vs IGNORE? Name each category explicitly |
| 6 | **Heuristic** | What rules of thumb emerge? Are they learnable or must they be hardcoded? |
| 7 | **Bayesian** | What is the prior? What evidence updates it? What is the posterior? |
| 8 | **Dialectical** | State thesis. State antithesis. Force a synthesis. What reverses intuition? |
| 9 | **Integrative** | How do all elements interconnect? What hidden dependencies exist? |
| 10 | **Probabilistic** | Replace every point estimate with a distribution. Quantify uncertainty |

---

## Layer 2 — Recursive Descent

Take the 3–5 most important findings from Layer 1.

For each finding, descend:
```
Level 1: State the finding clearly
Level 2: What would falsify this? Apply the strongest counterargument
Level 3: Does it hold? If yes → crystallized. If no → reformulate or discard
Level N: Repeat until conclusions stop changing (minimum 3 levels)
```

**Rule**: A finding that cannot survive its own counterargument is not a truth — it is a bias.

---

## Layer 3 — Metathinking

Question the analysis itself, not its conclusions:

1. Are these the **right questions** to ask, or am I answering the wrong problem?
2. What has been **systematically ignored** — not by accident, but by the shape of my thinking?
3. What would a **hostile expert** say about this analysis?
4. Am I being **realistic and objective**, or constructing an elegant story?
5. Is the **problem framed correctly** — or does the frame itself create blind spots?

Metathinking may invalidate entire sections of Layer 1 or Layer 2. That is its purpose.

---

## Output — TRUTH Statements

```
| T# | Truth | Confidence | Design impact |
|----|-----------------------------|------------|---------------|
| T1 | [Falsifiable statement]     | XX%        | [Concrete consequence] |
| T1a| [Sub-truth refining T1]     | XX%        | [Specific consequence] |
| T2 | [Another truth]             | XX%        | [Concrete consequence] |
```

**Sub-truths (T1a, T1b, ...):** When a truth has multiple distinct design impacts or needs refinement, use sub-truths. This prevents artificial flattening where complex findings get compressed into one row. A truth with 3 sub-truths is more useful than 3 vaguely related truths.

**Rules:**
- Maximum confidence = φ⁻¹ = 61.8% — epistemic humility is not optional
- Every truth must be **falsifiable** — if it cannot be proven wrong, it is not a truth
- Every truth must have a **concrete design impact** — if it changes nothing, it is trivia
- Truths that **reverse intuition** are more valuable than those that confirm it
- When two truths conflict: design to distinguish them empirically, never suppress one
- No artificial ceiling on truth count — let the problem determine the number

---

## Validation Before Writing

Before committing any truth to a document:

```
□ Can I state one clear objection to each truth?
□ Is each truth grounded in measurement or logic — not assumption?
□ Have I been realistic, not optimistic?
□ Does each truth change the design in a specific, traceable way?
□ Would I defend this under hostile questioning?
```

Fail any check → return to Layer 1 for that truth.

---

## When to Stop

Stop when:
- Conclusions stop changing between descent levels
- Every truth survives its counterargument
- The metathinking pass finds no new blind spots

Do not stop because:
- It feels complete
- The document looks full
- Time pressure exists

*"Better a short document of crystallized truths than a long document of elegant assumptions."*

---

## Example — Concrete Crystallization

**Problem:** "Should we use a message queue or direct HTTP calls between services?"

**Layer 1 (selected modes):**
- *Causal:* Direct HTTP creates temporal coupling — caller blocks until callee responds. Queue decouples.
- *Epistemic:* We KNOW our traffic is bursty. We ASSUME the queue won't become a bottleneck. We IGNORE queue operational cost.
- *Dialectical:* Thesis: queues are always better for async. Antithesis: queues add complexity, failure modes, and a new dependency. Synthesis: queues for fire-and-forget, HTTP for request-response.

**Layer 2 (recursive descent on top finding):**
- L1: "Queues decouple services temporally" — true
- L2: Counterargument: queues ADD a new failure mode (queue itself goes down). Now you have 3 failure modes instead of 1.
- L3: Does it hold? Partially — queues decouple but don't eliminate failure. They MOVE the coupling from temporal to operational.

**Layer 3 (metathinking):**
- Are we asking the right question? Maybe. The real question might be "what's our failure budget?" not "queue vs HTTP?"

**Output:**

| T# | Truth | Confidence | Design impact |
|----|-------|------------|---------------|
| T1 | Queues move coupling from temporal to operational — they don't eliminate it | 55% | Budget for queue monitoring and dead-letter handling, not just "add a queue" |
| T1a | For request-response patterns, HTTP is simpler and the coupling is acceptable | 52% | Use HTTP for inference requests (caller needs the response). Queue for telemetry events (fire-and-forget). |

---

## Common Failure Modes

| Failure | Symptom | Fix |
|---|---|---|
| Mode collapse | All 10 modes say the same thing | Force dialectical opposition on the dominant view |
| Shallow recursive | Only 1 level of descent | Keep asking "what would falsify this?" |
| Missing metathinking | Never questioned the problem frame | Apply Layer 3 as separate session, not a footnote |
| Point estimate disease | Specific numbers without distributions | Replace every number with (expected, φ⁻¹ percentile, φ⁻² percentile) |
| Confirmation bias | Only truths that confirm the design | Actively seek truths that require redesign |
| Epistemic cowardice | Vague truths that offend no one | Name the assumption. Commit to a falsifiable claim |

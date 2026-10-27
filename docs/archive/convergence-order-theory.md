# Convergence Order Theory

## Formal Model for Multi-Branch Patch Convergence

**Status**: Design document, derived from observed C1-C6 contradictions
**Date**: 2026-10-08
**Worktree**: /tmp/cynic-convergence

---

## 1. Motivation

When converging N divergent branches into a single target (e.g., `main`), we observe:

- **Contradictions**: Two branches modify the same domain with different approaches
- **Duplicates**: The same logical change exists on multiple branches (different SHAs)
- **Dependencies**: Some patches require others to be applied first
- **Independence**: Some patches commute (order does not matter)

This document formalizes these observations into a mathematical model that can drive an automated convergence engine.

---

## 2. Definitions

### 2.1 Patch

A **patch** is a commit that transforms the codebase from one state to another.

```
Patch = (sha, branch, date, author, subject, files[])
```

### 2.2 Patch Space

Let **P** be the set of all patches across all branches under convergence.

### 2.3 Recency

```
recency: P → ℝ
recency(p) = unix_timestamp(p.date)
```

### 2.4 Completeness

```
completeness: P → ℕ
completeness(p) = |p.files|
```

### 2.5 Dominance (Pareto)

Patch **a** dominates patch **b** (written **a ≻ b**) iff:

```
recency(a) ≥ recency(b) ∧ completeness(a) ≥ completeness(b) ∧ (recency(a) > recency(b) ∨ completeness(a) > completeness(b))
```

**Observation (C2)**: When a ≻ b, patch b is obsolete and can be dropped.

**Observation (C1)**: When a and b are incomparable (neither dominates), both must be merged.

---

## 3. Dependency Graph

### 3.1 Dependency Types

| Type | Symbol | Meaning | Detection |
|---|---|---|---|
| **Ancestor** | a → b | b is a descendant of a | `git merge-base --is-ancestor a b` |
| **Semantic** | a ⇒ b | b references symbols introduced by a | Static analysis (grep, AST) |
| **File Overlap** | a ∩ b | a and b modify the same file | Set intersection |
| **Independent** | a ⊥ b | No dependency | None of the above |

### 3.2 Dependency Graph

```
G = (P, E)
E = {(a, b) | a → b ∨ a ⇒ b ∨ a ∩ b}
```

### 3.3 Topological Order

A **topological order** of G is a linear ordering of P such that:

```
∀(a, b) ∈ E: position(a) < position(b)
```

**Observation (C3)**: Ancestor dependencies produce a natural topological order.

**Observation (C6)**: Semantic dependencies produce hidden edges not visible in file overlap.

**Observation (C4, C5)**: Independent patches (a ⊥ b) can be applied in any order.

---

## 4. Independence and Commutativity

### 4.1 Independence Relation

```
a ⊥ b ⇔ ¬(a → b) ∨ (b → a) ∨ (a ⇒ b) ∨ (b ⇒ a) ∨ (a ∩ b)
```

### 4.2 Commutativity

If a ⊥ b, then:

```
apply(a, apply(b, S)) = apply(b, apply(a, S))
```

for any codebase state S.

### 4.3 Equivalence Classes

The independence relation partitions P into **equivalence classes**:

```
P / ⊥ = {C₁, C₂, ..., Cₖ}
```

where each Cᵢ is a maximal set of mutually independent patches.

**Observation (C4, C5)**: Patches in the same equivalence class can be applied in any order.

---

## 5. Invariants

### 5.1 Invariant Definition

An **invariant** is a predicate on codebase state that must hold after each patch application.

```
Invariant: State → {true, false}
```

### 5.2 CYNIC Invariants

| Invariant | Check | Failure Mode |
|---|---|---|
| **Compilation** | `cargo check` passes | Type error, missing import |
| **Test Suite** | `cargo test` passes | Regression |
| **Architecture** | `cynic audit` passes | Boundary violation |
| **Schema** | JSON validates against schema | Schema drift |

### 5.3 Invariant Preservation

A patch set **P'** is **valid** iff:

```
∀p ∈ P': Invariant(apply(p, state_after_previous_patches))
```

---

## 6. Convergence Algorithm

### 6.1 Input

- Set of branches B = {b₁, b₂, ..., bₙ}
- Target branch T (e.g., `main`)
- Base SHA (common ancestor)

### 6.2 Algorithm

```
1. CENSUS: For each branch b ∈ B, compute unique patches (git cherry)
2. CLASSIFY: For each patch p, compute (recency, completeness, files)
3. DETECT DUPLICATES: Group patches by content similarity (patch-id)
4. BUILD GRAPH: For each pair (a, b), determine dependency type
5. RESOLVE CONTRADICTIONS: For each contradiction, apply decision rule
6. TOPOLOGICAL SORT: Compute linear order respecting dependencies
7. PARTITION: Group independent patches into equivalence classes
8. EXECUTE: Apply patches in topological order, checking invariants
9. VERIFY: Run full invariant suite on final state
```

### 6.3 Decision Rules

| Condition | Decision | Rationale |
|---|---|---|
| a ≻ b | ADOPT_A | b is obsolete |
| b ≻ a | ADOPT_B | a is obsolete |
| a → b | ADOPT_B | b includes a |
| a ⇒ b | ADOPT_A_B, order [a, b] | b depends on a |
| a ∩ b, no semantic dep | MERGE | Both needed, integrate |
| a ⊥ b | ADOPT_A_B, any order | Independent |
| a ≈ b (duplicate) | ADOPT one, DROP other | Same content |

---

## 7. Formal Properties

### 7.1 Termination

The algorithm terminates because:
- The patch set is finite
- Each step reduces the number of unapplied patches
- Topological sort is finite

### 7.2 Soundness

If the algorithm produces a valid patch order, then:
- All dependencies are satisfied
- All invariants hold
- The final state is consistent

### 7.3 Completeness

If a valid convergence exists, the algorithm finds it:
- The dependency graph captures all constraints
- Topological sort finds a valid order if one exists
- Invariant checking ensures correctness

---

## 8. Observed Contradictions (C1-C6)

### C1: Plane Vocabulary (MERGE)

```
A = 43431403 (core-extraction, 2026-10-06, 3 files)
B = f5be5013 (network-convergence, 2026-10-05, 14 files)

A ⊁ B (A more recent, less complete)
B ⊁ A (B more complete, less recent)
→ Incomparable → MERGE
```

### C2: Mesh Model (ADOPT_A)

```
A = a28d1735 (core-extraction, 2026-10-06, 10 files)
B = 69e34e60 (network-convergence, 2026-10-05, 3 files)

A ≻ B (A more recent AND more complete)
→ ADOPT_A
```

### C3: CLI Surface (ADOPT_B)

```
A = b0287cf6 (core-extraction, 2026-10-05)
B = 06289d56 (scan-exclusions, 2026-10-07)

A → B (A is ancestor of B)
→ ADOPT_B (B includes A)
```

### C4: Audit Engine (ADOPT_A_B)

```
A = 6f3e9cc0 (core-extraction, debt ledger)
B = dc268c6d (scan-exclusions, drop fields)

A ⊥ B (no file overlap, no semantic dependency)
→ ADOPT_A_B, any order
```

### C5: Secrets Vault (ADOPT_A_B)

```
A = 59646b51 (core-extraction, token resolution)
B = 1593dedb (core-extraction, sealed install)

A ⊥ B (different files, different imports)
→ ADOPT_A_B, any order
```

### C6: Build Policy (ADOPT_A_B)

```
A = 25749045 (persistence-drift-rebase, build-policy.json)
B = 50294409 (core-extraction, Makefile)

A ⇒ B (A's test references CYNIC_SOURCE_BIN from B)
→ ADOPT_A_B, order [B, A]
```

---

## 9. Derived Models

### 9.1 Patch Lattice

The dominance relation defines a **lattice** on P:

```
a ∨ b = max(a, b)  (dominant patch)
a ∧ b = min(a, b)  (dominated patch)
```

### 9.2 Convergence Monoid

The set of valid convergence sequences forms a **monoid**:

```
Identity: empty sequence
Composition: concatenation of valid sequences
Associativity: (a · b) · c = a · (b · c)
```

### 9.3 Invariant Topology

The set of valid states forms a **topological space**:

```
Open sets: states where invariants hold
Closure: limit of convergent sequences
```

---

## 10. Future Work

### 10.1 Automated Detection

- **Semantic dependency**: Use Rust AST analysis to detect symbol references
- **Duplicate detection**: Use `git patch-id` for content-based deduplication
- **Invariant checking**: Integrate `cargo check`, `cargo test`, `cynic audit`

### 10.2 Formal Verification

- Prove termination, soundness, completeness
- Model check the convergence algorithm
- Verify invariant preservation

### 10.3 Tooling

- `cynic converge` command that implements this model
- JSON output for machine-readable convergence reports
- Integration with Forgejo for PR creation and merge

---

## 11. References

- `convergence-tracker.json` — Observed data (C1-C6)
- `convergence-tracker.schema.json` — Machine-readable schema
- `cynic-contradiction-distiller` skill — Contradiction resolution protocol
- `cynic-bootstrap-experiment` skill — Bootstrap experiment protocol

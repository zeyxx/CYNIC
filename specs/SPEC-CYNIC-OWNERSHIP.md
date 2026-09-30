# SPEC-CYNIC-OWNERSHIP: Two Primitives, Four Rights

- **Status**: Draft
- **Author**: Titouan Corneloup (zeyxx) / Hermes Agent
- **Date**: 2026-09-30
- **Target Crates**: `cynic-protocol`, `cynic-cli` (contract enforcement), `cynic-intel-escore` (future weight derivation)
- **Related Requirements**: `CLM-0001`, `CLM-0002` (`contracts/claims.json`)
- **Contract**: `contracts/ownership.json`

---

## 1. Context and Problem Statement

Two reference projects died from the same structural defect, and it was not a technical
defect.

**Eliza Labs (August 2026).** The token migrated from `ai16z` to `ELIZAOS` on a 1:6 swap
that expanded supply from 1.1B to 11B. A Burwick Law class action (SDNY, filed 2026-04-20)
alleged misrepresentation of the governance model — in substance, that the project was
represented as autonomously governed while insiders retained actual control. The token
passed from ~$2.5B to ~$2.3M. The foundation was wound down and the entire remaining
treasury was surrendered in settlement.

**Steem (March 2020).** HF20 moved voting from stake-weight to reputation-weight. When
TRON's acquisition of Steemit brought capital without reputation, the community nullified
the acquirer's vote by fork, then hard fork 0.23 seized 23.6M STEEM (~$6.3M) from 64
accounts, on the stated ground that they had contributed to a threat against the chain. The
chain fractured into HIVE.

The two cases are mirror images, and together they bound the design:

- ElizaOS shows that **governance power which is not provable to participants is
  confiscatory in fact**, regardless of the stated rules.
- Steem shows that **removing capital's veto does not remove capital; it converts a
  settlement into a fork**, and a fork destroys the value it was meant to defend.

The existing substrate already enforces authority, admission, leases, fencing, and read-back
proof. What it does not have is a rule stating **who owns what, and what governance can never
touch.** `CYNIC_TOKEN_EQUITY_FIREWALL.md:20-26` states that token is not equity and equity is
not protocol access, but it does not state the operative consequence: *equity cannot be
confiscated by governance.* That missing clause is what both reference projects lacked.

`ACCOUNTABILITY_RECOVERED_UNATTRIBUTATED` (`crates/cynic-workload/src/types.rs:75`) is a
terminal state that admits an attribution could not be resolved. It records a gap; it does not
prevent it. An ownership model must prevent, not record.

---

## 2. Decision and Formal Specification

Authority over a system is split along one axis: **what can be lost, and by whom.**

| | Right | Character | Revocable |
|---|---|---|---|
| **P1 — incorruptible claim** | **R-A** claim on material possession | inviolable | never, by no vote |
| | **R-B** right of exit, carrying what one can carry | inviolable | never |
| **P2 — delegated authority** | **R-C** governance weight | earned, derived | yes, on ceasing contribution |
| | **R-D** obligations attached, and forfeit on defection | conditional | yes, by defection |

### 2.1 Formal Invariants

1. **INV-01 (Material claim is inviolable)**: No aggregate of governance weight R-C may
   extinguish, dilute, or transfer a material claim R-A. There is no weight at which this
   becomes permissible.
2. **INV-02 (Exit is inviolable)**: Any actor may exit and remove its own material claim
   without admission, without cause, and without the cooperation of any other actor. R-B
   admits no counterparty.
3. **INV-03 (Weight is derived, never stored)**: Governance weight R-C is a pure function of
   *observed* contribution. No actor may assign, grant, sell, or inherit weight. Weight is
   recomputed, never read from a field.
4. **INV-04 (Revocation transfers the charge)**: Revoking a capability transfers its
   obligation R-D to a named successor that accepts it with observable proof. Revocation may
   never extinguish an obligation. An actor may not discharge a duty by delegating it.
5. **INV-05 (Weight is not purchasable)**: No quantity of material claim R-A may be
   converted into governance weight R-C. The two are not commensurable.
6. **INV-06 (Seniority penalises defection, never counts votes)**: Tenure may modify R-D
   (the cost of defection). Tenure is never an input to R-C (the vote tally). A cohort
   that is merely old holds no additional votes.
7. **INV-07 (Unconsumed contribution scores zero)**: Contribution with no observable
   consumption by a third party has zero weight. Activity is not value.

### 2.2 The Discriminator

The whole model reduces to one question, which is executable:

> **Does the holder of the largest material share hold a vote over a decision that affects
> another actor's material claim?**

If yes, the design is ElizaOS. If no, it is coherent. There is no third case.

`INV-01` through `INV-05` are precisely the conditions under which the answer is forced to
*no*.

### 2.3 The Two Asymmetries

Non-totalitarianism is not a property of the parts; it is a property of two non-derivations:

- **A1** — R-C cannot be acquired with R-A. *(kills ElizaOS: power not provable to
  participants)*
- **A2** — R-A and R-B cannot be overridden by any sum of R-C and R-D. *(kills Steem:
  property overwritten by earned authority)*

### 2.4 Why R-B Is Not Optional

Without an unconditional right of exit, governance has no effective counterweight, because
every other counterweight is contestable by the governed. Departure is the only check that is
itself unfalsifiable and costless to exercise. A community whose members cannot leave is not
governed, it is held. R-B is the constraint that makes R-A credible.

### 2.5 Mathematical Model

Contribution weight over a decaying window, saturating and derived:

```
retention(i, Δ)   = |{ consumers of i's output over Δ }| / |{ consumers over lifetime }|
cost(i, Δ)        = Σ observed resources consumed by i over Δ
                   (CPU ms, RAM bytes·s, VRAM bytes, joules, wall ms)

weight(i, Δ)      = Σ_Δ  retention(i, Δ) / cost(i, Δ)  ·  exp(-λ·(now - Δ))
                    capped at saturation s_i
```

- `cost` is measured by sensor. It is never declared. No consensus layer is required to
  observe it.
- `retention` is revealed preference, not stated preference. It is the only contribution
  signal that survives an actor optimising the measurement system.
- The exponential term is INV-06's *mechanism* — weight decays unless maintained, so tenure
  confers a forfeited stake (R-D) rather than votes (R-C).
- `saturation` prevents an actor from converting unbounded labour into unbounded control, and
  is what keeps INV-07 meaningful.

**A note on the physics analogy, which is a heuristic and must not justify a constant.** The
`attention as energy` framing fails in two specific ways: attention is not conserved, so no
conservation law can be derived from it, and `voltage × intensity` computes energy, not power,
because the intensity of contribution is a flow while its accumulated volume is a stock. The
model above takes the time derivative. Any constant of this shape — including the harmonic
1.618 previously refuted in `docs/archive/reports/2026-09-02-mathematics-geometry-audit.md:221`
as *structurally aesthetic choices, not demonstrated epistemic laws* — is inadmissible unless
it carries a calibration and a falsifier.

### 2.6 Provenance Carries Capacity

Authority over a created artifact follows its authorship, not its ownership by capital:

> **A created artifact carries a verifiable, non-transferable authorship claim, and authority
> over that artifact follows its author.**

`R-C` weight is granted by provenance, never purchased. This is why no KYC is required for
entry: identity enters the system through work, whose proof is the output itself.

---

## 3. Top-Down Causal Chain Impact

- **Constituent / Operator Intent**: The sovereign defines the constitutive rules; the rules
  bind the sovereign identically. This is the inversion of ElizaOS, where the announced rule
  and the actual control diverged.
- **Protocol Types & Contracts**: `contracts/ownership.json` declares R-A..R-D, INV-01..07,
  A1, A2, and the discriminator. `contracts/claims.json` indexes the claims this spec rests
  on. `OneShotCapability` (`crates/cynic-protocol/src/one_shot_capability.rs:23`) already
  provides atomic single-use consumption and is the natural carrier for INV-04.
- **Execution & Sandboxing Boundary**: Governance executes through the same admission path as
  any other mutation — `admit_agent_workload` (`crates/cynic-workload/src/admission.rs:12`).
  There is no privileged governance channel. A governance decision that cannot be admitted is
  not a decision.
- **Read-Back Proof & Observation**: A governance act produces a `RuntimeProofBundle`
  (`crates/cynic-protocol/src/runtime/proofs.rs:11`) whose `validate_for_subject` refuses
  non-satisfied status, empty proofs, duplicate gates, stale proofs, and future-dated proofs.
  Weight derivation consumes that bundle. This is the property that makes R-C unwritable.

---

## 4. Blast Radius and Non-Goals

- **In-Scope**: the four rights, the two asymmetries, the derivation of weight from
  observed contribution, and the single executable discriminator.
- **Explicit Non-Goals**:
  - Naming any token, community, market, or economic instrument.
  - Any on-chain settlement. PoH append-only yields *contradiction*, not falsification; a
    revokable governance weight must remain locally verifiable.
  - Calibration of `λ` or `s_i`. They stay `null` until a measured corpus exists. An
    uncalibrated constant is a belief, and this spec does not ship beliefs as numbers.
  - The economic policy split (buyback / community proposals / infrastructure maintenance).
    It is expressible as a machine-readable policy with a read-back proof, but it belongs to
    a NationKit surface, not to CYNIC.
- **Forbidden Interactions**:
  - A vote count that mixes `retention` and tenure, or that mixes capital and contribution in
    a single scalar. Flow and stock of the same quantity must never govern the same power.
  - Any revocation path that extinguishes rather than transfers (INV-04).
  - Any schema where governance weight is a stored field (INV-03).

---

## 5. Verification and Property Tests

| ID | Property | Expected |
|---|---|---|
| PROP-01 | Majority supply holder votes on a decision affecting another's claim | must be structurally impossible |
| PROP-02 | Actor grants itself weight with zero observed contribution | rejected; weight stays 0 |
| PROP-03 | Actor revokes its own capability to avoid R-D | refused; obligation transfers (INV-04) |
| PROP-04 | Contributor whose output has no consumers | weight = 0 (INV-07) |
| PROP-05 | Tenure raises the top-20 vote share | must not change (INV-06) |
| PROP-06 | Any schema with a stored weight field | compile-time rejected |

**Read-back probe.** A governance act is admissible only when its bundle satisfies
`status == Converged`, `blockers.is_empty()`, at least one proof family `Converged`, and
`observed_at_unix_ms` within `max_age_ms` of now. Otherwise the weight derived from it is
`null` and surfaces as `unknown` — never as zero-by-default, which would read as a passing
grade.

**Current executable coverage.** `contracts/ownership.json` is read and validated by
`ownership_claims_are_indexed` and `no_normative_spec_without_invariant_and_falsifier` in
`crates/cynic-cli/tests/architecture_boundary.rs`. The constitutional side of the
discriminator (PROP-01) has **no executable check yet** and is declared
`partial` in `contracts/foundation.json → constitutional.boundary_coverage`. That is the
honest state: the rule is written and the check is missing.

---

## 6. Migration and Rollout Plan

1. **Land `contracts/ownership.json`** as `status: draft`, plus the two enforcement tests.
   No runtime behaviour changes.
2. **Declare the boundary** in `contracts/foundation.json` as a fifteenth hard boundary, at
   `partial` coverage with the discriminator named as the missing check.
3. **Derive weight, never store it.** Wire `ChatTrace` (currently
   `Option<u64> = None`, `crates/cynic-inference/src/domain/chat_trace.rs:26`) to a live
   `TraceSink` so `prompt_tokens`/`gen_tokens` are observed. Until then weight derives to
   `unknown`, which is the correct and honest state.
4. **Add PROP-01 as a type-level check.** The discriminator should be unrepresentable, not
   merely untested.
5. **Leave `λ` and `s_i` null.** They are filled by measurement, or not at all.

---

## 7. What Would Falsify This Spec

| | Falsifier |
|---|---|
| The whole | A constructed system where a majority-supply holder cannot be given any path over another's claim, and where exit remains free, but which is *not* compatible with R-C being derived from observed contribution. That would falsify the necessity of INV-03. |
| INV-03 | A working system with stored, self-assignable weight that does not fall to capture. |
| INV-07 | A system where unconsumed contribution reliably predicts survival, i.e. where activity is a better measure than consumption. |
| R-B | Evidence that binding exit makes the community measurably more extractive, not less. |

---

*Related: `docs/notes/amm-cynic-mapping.md` (invariant-over-launchpad principle),
`CYNIC_TOKEN_EQUITY_FIREWALL.md:20-26` (the clause this spec makes operative),
`contracts/claims.json` (claim index).*

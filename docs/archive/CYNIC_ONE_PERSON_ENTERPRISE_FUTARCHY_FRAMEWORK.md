# CYNIC One-Person Enterprise Futarchy Framework

Status: working foundation draft.

This document distills the current foundation canon and internal Odysseum /
ASDFASDFA drafts into one product and interface framework for the ultimate
one-person enterprise.

It does not replace:

- `docs/foundation/TALARIA_CYNIC_NAMING.md`
- `docs/foundation/CYNIC_ONE_PERSON_FUTARCHY.md`
- `docs/foundation/CYNIC_TOKEN_EQUITY_FIREWALL.md`
- `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md`
- `docs/foundation/CYNIC_E_SCORE.md`
- `docs/foundation/CYNIC_PROTOCOL_REVENUE_WATERFALL.md`
- `docs/foundation/CYNIC_LEGAL_GENESIS_PLAN.md`
- `docs/foundation/ODYSSEUM_MANIFEST.md`
- `axioms/public-futarchy-manifest-v1.json`

It is the bridge between those contracts and the public interface.

---

## 1. Core Thesis

The ultimate one-person enterprise is not a founder with more charisma. It is a
single operator whose work, decisions, risks, boundaries and outcomes are made
legible enough that strangers can coordinate around them without pretending to
own the company.

Canonical loop:

```text
Work
  -> Proof
  -> Question
  -> Market signal
  -> Decision
  -> Outcome
  -> E-Score / Waterfall update
```

The interface is not decoration. The interface is the enterprise's public
coordination layer.

---

## 2. Five Rails

The framework has five rails. A public claim must say which rail owns it.

| Rail | Owns | Source of truth | Interface emotion |
|------|------|-----------------|-------------------|
| Legal rail | TALARIA SAS, IP, contracts, cap table, tax | legal documents, contracts, accounting | trust |
| Protocol rail | smart contracts, treasuries, rewards, burns, votes | Solana, multisigs, published protocol rules | agency |
| Operator rail | work, compute, uptime, security, integration | logs, commits, invoices, reports, Kernel observations | control |
| Market rail | prices, proposal signals, ROI, liquidity, demand | on-chain markets, DEX/CEX data, realized PnL | tension |
| Narrative rail | cult energy, recruiting, public pressure, attention | public posts, manifest, build proofs, reactions | belonging |

Rules:

- the legal rail cannot be replaced by token votes;
- the protocol rail cannot promise corporate profits;
- the operator rail cannot be rewarded without measured work;
- the market rail is signal, not truth;
- the narrative rail can create attention, but not rights.

---

## 3. Public Product Shape

The public surface should begin with four routes:

```text
/manifest
/build-log
/decisions
/outcomes
```

### `/manifest`

Purpose: explain the game without promising entitlement.

Required answers:

- What is TALARIA SAS?
- What is CYNIC?
- What is Odysseum?
- What is `$ASDFASDFA`?
- What is `$CYNIC`, if discussed?
- What is not promised?
- What can a stranger do now?

### `/build-log`

Purpose: convert invisible work into public proof.

Every proof card must answer:

- what changed?
- what artifact proves it?
- which rail does it affect?
- what is still unknown?
- what action becomes possible now?

### `/decisions`

Purpose: expose futarchy-ready questions.

A decision is valid only if the answer changes execution.

Good shape:

```text
Should we do X before Y, given evidence E, with outcome metric M, by deadline D?
```

Bad shape:

```text
Will this pump?
Are we the future?
Do people believe?
```

### `/outcomes`

Purpose: close loops publicly.

Every outcome must state:

- decision taken;
- market signal observed;
- execution performed;
- evidence source;
- what was falsified;
- E-Score, waterfall or roadmap consequence.

---

## 4. Proof Object Model

The primitive public unit is not a tweet, card or dashboard row. It is a proof
object.

Minimum shape:

```json
{
  "schema": "cynic.public_proof.v1",
  "id": "proof-id",
  "rail": "legal|protocol|operator|market|narrative",
  "claim": "What is being asserted.",
  "primary_source": "file, transaction, contract, command output, market data or signed report",
  "kernel_interpretation": "optional judgment, never primary truth",
  "status": "public|private|draft|blocked|unknown",
  "public_action": "What a stranger can do with this now.",
  "forbidden_reading": "The interpretation this proof must not create."
}
```

The key field is `forbidden_reading`. It prevents proof cards from becoming
accidental securities, fake liveness, fake decentralisation or fake entitlement.

---

## 5. Emotional Contract

The interface must transmit emotions through mechanisms, not copy.

| Emotion | Mechanism | UI signal |
|---------|-----------|-----------|
| Clarity | hard separation of rails | boundary map, labels, forbidden readings |
| Trust | public primary evidence | proof cards, source links, redaction labels |
| Control | visible operator loop | next actions, blockers, owner, deadline |
| Agency | decisions that change execution | active questions, options, consequences |
| Tension | market signal without worship | price/signal panels with uncertainty |
| Ambition | institutional polish plus on-chain proofs | corporate surface linked to protocol evidence |
| Belonging | cult can pressure and falsify | public challenges, replies, proposal intake |

Forbidden emotional patterns:

- mystique as substitute for evidence;
- urgency as substitute for decision quality;
- corporate polish hiding missing proof;
- crypto excitement implying yield or equity;
- founder sacrifice framed as entitlement.

---

## 6. Claim Ladder

Every public claim must sit on this ladder.

| Level | Meaning | Public wording |
|-------|---------|----------------|
| L0 | raw idea | "Hypothesis" |
| L1 | internal draft | "Draft" |
| L2 | artifact exists | "Built / documented" |
| L3 | independently inspectable | "Public proof" |
| L4 | used by someone else | "Observed usage" |
| L5 | market-priced | "Market signal" |
| L6 | legally/protocol-final | "Resolved by source of truth" |

Rule:

```text
No L2 claim may be presented as L5 or L6.
```

This is the main antidote to fake liveness.

---

## 7. Futarchy Decision Contract

Futarchy starts after the boundary is clean.

Minimum decision object:

```json
{
  "schema": "cynic.futarchy_decision.v1",
  "question": "Should X happen before Y?",
  "rail": "protocol|operator|market|narrative",
  "excluded_rails": ["legal"],
  "options": ["A", "B"],
  "evidence": ["proof ids"],
  "metric": "observable outcome metric",
  "deadline": "date or block height",
  "execution_change": "what changes if A wins vs B wins",
  "guardrails": ["no equity promise", "no yield promise"],
  "resolution_source": "market plus primary source"
}
```

Futarchy may decide:

- next public demo;
- order of product milestones;
- branch prioritisation;
- operator admission criteria;
- grant allocation;
- market experiment sequencing;
- public case-study selection.

Futarchy must not directly decide:

- TALARIA SAS cap table;
- statutory governance;
- legal dividend distribution;
- IP transfer;
- private counsel decisions;
- hidden launch timing.

---

## 8. Odysseum Translation

The Odysseum narrative is powerful when translated carefully.

Allowed product claim:

```text
Odysseum aims to make community token revenues inspectable and governable by
routing eligible fee flows into public vaults and futarchy-style decision
surfaces.
```

Needs proof before strong claim:

- "Odysseum-Locked" as a standard;
- Pump.fun / PumpSwap native integration;
- ARR and take-rate projections;
- franchise / M&A mechanisms;
- Ignis buyback/burn execution;
- GASdf privacy or gasless guarantees.

Forbidden compression:

- "memecoin holders own a company";
- "stock" as legal equity unless a regulated security-token route exists;
- "buyback returns passive value" without legal review and waterfall context;
- "zero human" if humans still hold upgrade, treasury or legal powers.

Preferred abstraction:

```text
Stocks on Solana = proof-priced operating communities, not legal shares by
default.
Options on Solana = market-priced decisions, not uncontrolled speculation by
default.
```

---

## 9. One-Person Enterprise Operating Rhythm

The OPE must avoid disappearing into private build cycles.

Weekly minimum:

```json
{
  "build_proofs": 2,
  "active_decision_updates": 1,
  "public_replies_or_distribution_events": 3,
  "outcome_or_blocker_update": 1
}
```

Weekly public review:

1. What did the operator build?
2. What proof exists?
3. What decision is open?
4. What did the market or community signal?
5. What changed because of that signal?
6. What remains private, blocked or unknown?

The operator does not need to expose ordinary execution. The operator must
expose power, promises, conflicts, proof and decision loops.

---

## 10. Organism Archetype Map

The Boris Cherny archetypes are useful for CYNIC if they are treated as
operating modes, not job titles. In a one-person enterprise, the same operator
and organism must cycle through all five modes deliberately.

| Archetype | CYNIC function | Current organism expression | Failure mode |
|-----------|----------------|-----------------------------|--------------|
| Prototyper | Generates new products, mechanisms, narratives and market questions | Odysseum, GASdf, Ignis, ASDFASDFA/CYNIC maps, new axioms | Too many futures, not enough public proof |
| Builder | Turns a promising prototype into production-grade infra or product | Kernel, organs, MCP/REST coordination, foundation contracts, public-ui scaffold | Shipping internal machinery that nobody can inspect |
| Sweeper | Simplifies, deletes, clarifies, compresses and makes the system legible | Topology policy, forbidden readings, claim ladder, interface contract | Missing muscle: complexity accumulates faster than trust |
| Grower | Improves PMF, distribution, onboarding, proof loops and user action | bagwork loop, build-proof stream, decision board, Odysseum ICP | Distribution dies while engineering compounds privately |
| Maintainer | Keeps mature surfaces secure, reliable, fast and legally coherent | gate system, source-of-truth hierarchy, fair launch, token/equity firewall | Stability consumes all energy before product clarity emerges |

Rule:

```text
The organism is healthy when every week produces visible evidence of Builder,
Sweeper and Grower work, while Prototyper and Maintainer energy stay bounded.
```

For the current stage, CYNIC is pre-PMF. The required mix is:

```text
Prototyper + Builder + Sweeper
```

But the observed asymmetry is:

```text
Prototyper strong.
Builder strong.
Maintainer unusually strong for stage.
Grower intermittent.
Sweeper underdeveloped.
```

The interface must therefore act as a Sweeper organ before it acts as a growth
surface. It should delete ambiguity, expose boundaries, compress proofs and
make the next decision obvious.

### Archetypes As Weekly Accounting

Every weekly public review should classify work by archetype:

| Question | Archetype |
|----------|-----------|
| What new possibility was created? | Prototyper |
| What became real and usable? | Builder |
| What became simpler, clearer or deleted? | Sweeper |
| What reached users, contributors or markets? | Grower |
| What became safer, faster or more reliable? | Maintainer |

If a week has no Sweeper output, the next public interface task should be
cleanup, compression or deletion before new feature work.

### Sweeper Definition For CYNIC

Sweeper work is not cosmetic. It is the work that converts internal power into
external trust.

Valid Sweeper outputs:

- one confusing term deleted or renamed;
- one claim moved to the correct rail;
- one proof card compressed from raw work;
- one private/draft/unknown boundary made explicit;
- one route or screen removed;
- one legal/protocol/narrative ambiguity blocked;
- one artifact made inspectable by a stranger;
- one "organism" metaphor translated into operational language.

Invalid Sweeper outputs:

- adding another dashboard;
- adding explanatory prose without removing ambiguity;
- hiding missing proof behind better visuals;
- polishing a claim that should be downgraded or forbidden.

---

## 11. CYNIC Axiom Mapping

The archetypes are downstream operating modes. They do not replace the CYNIC
axioms. They translate the axioms into weekly product behavior.

Canonical judge axioms:

```text
FIDELITY
PHI
VERIFY
CULTURE
BURN
SOVEREIGNTY
```

Extended / observed principles:

```text
KENOSIS = observed axiom candidate, not final Q-Score input yet.
FRACTAL = topology principle, not ordinary judge axis.
```

### Six Active Axioms

| Axiom | Product question | Interface obligation | Archetype pressure |
|-------|------------------|----------------------|-------------------|
| FIDELITY | Does the system say what it actually is? | Names, claims and roles must match the real rail. | Sweeper |
| PHI | Is the system proportionate and harmonized? | Screens must show relationships, not isolated hype objects. | Sweeper + Maintainer |
| VERIFY | Can a stranger inspect the claim? | Every claim needs a primary source or must be downgraded. | Builder + Sweeper |
| CULTURE | Does this preserve the community's living pattern? | The interface must leave room for cult energy, replies, pressure and lore. | Grower |
| BURN | Is the structure efficient and non-extractive? | Remove routes, copy, visuals and claims that do not move proof or decisions. | Sweeper |
| SOVEREIGNTY | Does this preserve autonomous decision capacity? | Separate legal, protocol, operator, market and narrative power. | Maintainer + Builder |

The current stage requires the axioms to be read through Sweeper work first:

```text
FIDELITY = rename or downgrade false claims.
PHI = rebalance the rails.
VERIFY = attach proof.
BURN = delete noise.
SOVEREIGNTY = expose boundaries.
CULTURE = keep the public loop alive.
```

### Kenosis As The Seventh Candidate

Kenosis is the axiom of intentional space. It matters for the one-person
enterprise because not every gap should be filled by automation, explanation or
control.

Interface reading:

```text
KENOSIS asks: what must the interface intentionally not decide, not explain, or
not capture so the other side can choose freely?
```

Valid product uses:

- do not force wallet connection before proof inspection;
- do not force investment framing before public artifacts exist;
- do not force a market on questions that do not change execution;
- leave unknowns visible instead of filling them with narrative;
- let community pressure emerge from proof rather than coercive calls to action.

Kenosis must not become an excuse for missing Sweeper work. If silence hides
confusion, it is not Kenosis. It is debt.

### Kenosis Against Overproduction

LLM agents tend to overproduce structure. The human operator can also overthink,
over-accumulate physical objects, tabs, drafts, documents and unresolved
possibilities. Both failure modes look like productivity while draining the
system.

Kenosis is therefore an operator safety law:

```text
When the next useful action is deletion, rest or exposure to reality, adding
more structure is a violation.
```

Trigger signals:

- a new JSON contract is being added before its canonical owner is known;
- a package-local source of truth appears under a product projection;
- the operator keeps reasoning after the next action is already obvious;
- the physical workspace starts reducing energy or attention;
- the interface adds explanation instead of reducing ambiguity;
- the agent proposes "one more abstraction" without naming what will be deleted.

Required reset:

```text
30 minutes.
No screen.
Body first.
Sun, air, walk, breath, cleanup or temperature reset.
Return only to answer: what should be deleted, downgraded or left empty?
```

This is not wellness decoration. It is a control loop that prevents the organism
from confusing accumulation with evolution.

### Fractal As The Eighth Topological Principle

Fractal is not a normal scoring axis. It is the rule that the same pattern must
remain aligned across scales.

OPE pattern:

```text
raw event -> judgment -> public projection
```

It appears at every scale:

| Scale | Raw event | Judgment | Public projection |
|-------|-----------|----------|-------------------|
| Code | commit / test / diff | Kernel or human review | build proof |
| Product | user action | `/judge` / metrics | decision board |
| Protocol | transaction | contract / indexer / market | proof ledger |
| Company | contract / invoice / statute | counsel / accounting | corporate boundary |
| Community | post / reply / market signal | CYNIC interpretation | outcome log |

The interface should not flatten these dimensions into one master story. It
should preserve the pattern while showing which source of truth owns each layer.

### Archetype To Axiom Matrix

| Archetype | Primary axioms | What it must prove |
|-----------|----------------|-------------------|
| Prototyper | CULTURE, PHI, KENOSIS | New possibilities are alive, but not overclaimed. |
| Builder | VERIFY, SOVEREIGNTY, FIDELITY | A prototype became inspectable infrastructure or product. |
| Sweeper | FIDELITY, BURN, PHI | Complexity was reduced and claims now match reality. |
| Grower | CULTURE, VERIFY, KENOSIS | Distribution creates agency without coercion or fake promises. |
| Maintainer | SOVEREIGNTY, BURN, VERIFY | The system remains reliable, bounded and auditable. |

Weekly accounting should therefore record both:

```text
archetype_mode = what kind of work happened
axiom_effect = what principle became more true
```

Example:

```json
{
  "archetype_mode": "Sweeper",
  "axiom_effect": ["FIDELITY", "BURN", "VERIFY"],
  "artifact": "downgraded Odysseum claim from L5 market signal to L1 draft",
  "public_effect": "reduced legal ambiguity before public interface work"
}
```

---

## 12. Interface Scaffold

The first interface should be contract-first.

Screens:

1. Boundary map: TALARIA / CYNIC / Odysseum / ASDFASDFA / future `$CYNIC`.
2. Proof ledger: proof objects filtered by rail and claim level.
3. Decision board: active, draft, blocked and resolved questions.
4. Outcome log: what happened after decisions.
5. Emotional map: what the interface is supposed to make users feel and why.

Design direction:

- Solana Explorer patterns for inspectability: search, statuses, tables, source
  identifiers, cluster/state labels.
- Corporate patterns for legitimacy: legal boundary, entity role, commitments,
  counsel/accounting pending status.
- No landing-page hero until the proof system is legible.
- No fake live data.
- No ornamental organism language as the first viewport.

---

## 13. Definition Of Done

The framework is ready for a first public surface when:

- every major claim is assigned to a rail;
- the manifest has one active decision;
- the build-log has at least one proof object with a primary source;
- the decision board has at least one market-resolvable question;
- the outcome log can distinguish shipped, falsified, blocked and unknown;
- all token/equity and fair-launch guardrails are visible;
- Odysseum claims are split into proven, draft and forbidden readings;
- the interface transmits clarity, control, trust and ambition without implying
  equity, yield, guaranteed buyback or hidden insider power.
- the weekly review can classify visible work across Prototyper, Builder,
  Sweeper, Grower and Maintainer modes.
- each visible work item can name its axiom effect across the six active axioms,
  with Kenosis and Fractal used only at their correct status.

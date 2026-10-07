# Knowledge Architecture — doctrine, canonicalisation, and authority homes

- **schema**: `cynic.knowledge.architecture.v1`
- **status**: `proposed` — design gate. No file is moved and no citation is rewritten until this
  document is ratified.
- **method**: observation only. The corpus was measured (citations, facets, existing headers,
  authority homes) before any rule was written. No production file was mutated.
- **authority_order applied**: observed runtime state > `contracts/foundation.json` >
  `infra/runtime-authority.json` > `infra/directory-tree.json` > `infra/registry.json` >
  workspace `Cargo.toml` > canonical crates > `infra/agent-exposure.json`
- **companions**: `infra/DIRECTORY_STRUCTURE_AUDIT.md` (declared vs observed arborescence),
  `infra/CANONICAL_CRATE_MAP.md` (crate taxonomy), `spaces/operator/README.md` (operator space).
- **claim**: `claim_hermesknowledgearchitect_1791210019` (node-local arbiter; the kernel refused
  credentials for `/coord/claim` — an explicit unknown, not a grant).

---

## 0. The doctrine

> **Code cites machine-readable authority only.**
> Contracts, constitutions, policies, and taxonomies are executable and checkable. Markdown is for
> plans, designs, and audits — it is normative for humans and is **never** execution authority.
> A `.md` file is not "law", no matter what its header claims.

This is not tidiness. It is the rule that makes the rest decidable: if a rule matters to the code,
it must live where a gate can read it.

### 0.1 The doctrine is violated, measured

- **40 markdown citations from Rust**, across 18 distinct `.md` files.
- **17 of them declare `Authority:` / `Specifications:`** — the code names prose as its law.

| markdown cited as authority | sites | what it carries |
|---|---|---|
| `AGENTS.md` | 11 | secret classes table, structural invariants, zero-I/O rule |
| `protocols/machine-enrollment-federation.md` | 7 | proof schemas, federation wiring |
| `infra/CANONICAL_CRATE_MAP.md` | 4 | crate taxonomy |
| `protocols/cynic-stage2-peer-and-ledger.md` | 3 | consensus specs |

### 0.2 The sharpest case, and the general defect

Five Rust files cite **"AGENTS.md secret classes table"** as their authority
(`cynic-secrets/src/protocol.rs:6`, `cynic-secrets/src/lib.rs:11`,
`cynic-secrets/src/adapters/keyring.rs:11`, `cynic-protocol/src/secret_reference.rs:9`,
`cynic-host/src/adapters/local_secret_use_gate.rs:14`).

**That table does not exist.** `AGENTS.md` is 96 lines, mentions "secret" four times, and contains
no table. The real definition lives in `crates/cynic-protocol/src/secret_use.rs`. The code cites a
phantom section, and no gate can detect it — a markdown citation has no existence check, which is
exactly why prose is the wrong home for a rule.

**And the rule has multiple homes.** "Who may use a secret class" is expressed in at least:
`AGENTS.md` (cited, absent), `crates/cynic-protocol/src/secret_use.rs` (the real definition),
`infra/agent-roles.json` (RBAC), `contracts/foundation.json` (mention only). Four homes, no single
authority, no reconciliation path. This is the fragmentation that must be canonicalised before
anything is moved.

### 0.3 The machine-readable corpus is itself fragmented

**50 authority files** exist (16 `contracts/*.json`, 34 `infra/*.json`), with schema families that
do not converge: `cynic.foundation.v1`, `cynic.ownership.v1`, `cynic.pruning-policy.v1`,
`cynic.release_domain_spec.v1`, `cynic.runtime_authority.v1`… Some carry a `schema`, some do not
(`agent-roles.json`, `rust-toolchain.json`, `source-lifecycle.json`). There is no taxonomy saying
what kind of document each one is, so a reader cannot tell a constitution from a policy from a
registry. **Before prose can be promoted into machine-readable form, the target must be defined.**

---

## 1. The model — CYNIC as a Wikipedia

A Wikipedia is one corpus where every fact has **one home**, where an article **cites** its sources
and is not itself the source, and where meta material is attached to its subject rather than piled
beside it. Transposed, with the invariant Wikipedia does **not** have and CYNIC must keep:

> **The corpus preserves authority stratification.** A pre-legislative note and a normative law
> must never be confusable. Flattening everything into one vault would destroy exactly the property
> the corpus exists to protect.

### 1.1 Authority tiers (the physical axis)

| tier | authority | content | machine-readable? | Obsidian |
|---|---|---|---|---|
| **law** | normative, runtime | `contracts/`, `infra/*.json`, `infra/surface-registry/` | yes — cited by code | read-only (graph) |
| **canon** | normative, human | `AGENTS.md`, `CANONICAL_CRATE_MAP.md`, `ARCHITECTURE_CONSTELLATIONS.md`, RFCs, designs | no | editable, links → law |
| **evidence** | **zero execution authority** | audits, debt ledgers, decisions, briefs | no | the vault, dated |
| **exploration** | **zero execution authority, pre-legislative** | `spaces/operator/` | no | the vault |

`AGENTS.md` stays at the repository root: it is an agent entry point, not corpus.

### 1.2 Facets (everything else)

A directory per `domain × kind × lifecycle` is a combinatorial explosion and churns on every
promotion. The physical tree carries only the axis that must be a **boundary** (authority); the rest
are facets, indexed by Obsidian properties and by the audit.

| facet | values | source of truth |
|---|---|---|
| `authority` | `law` · `canon` · `evidence` · `exploration` | §1.1 — the physical axis |
| `kind` | `audit`, `design`, `rfc`, `map`, `rules`, `taxonomy`, `method`, `plan`, `ledger`, `brief`, `paper`, `manifesto`, `index` | derived from the corpus |
| `domain` | crate families: `host`, `network`, `storage`, `inference`, `market`, `agent`, `security`, `data`, `governance`, … — **multi-valued** | `CANONICAL_CRATE_MAP.md` L0-L4, `PLANE_TAXONOMY.md` |
| `lifecycle` | `draft` · `proposed` · `ratified` · `superseded` · `deprecated` | resolves RFC-003 and orphaned specs |

Plus `method` (`observed | derived | specified | assumed` — the provenance vocabulary already used
by the PEF pipeline) and `crates` (which crates the document governs).

**`domain` is a facet, not a directory.** A document on network inference has two domains; forcing
it into one directory betrays one of them. A multi-valued facet betrays neither.

---

## 2. The physical tree

```
infra/                     ← tier LAW only: machine-readable, cited by code. Unchanged paths.
spaces/
  canon/                   ← tier CANON: normative prose (design/ rfc/ map/ rules/ taxonomy/ method/ plan/)
  evidence/                ← tier EVIDENCE: audits, ledgers, briefs (audit/ ledger/ brief/), dated
  operator/                ← tier EXPLORATION (exists): manifesto/ traditions/ whitepaper/ concepts/
AGENTS.md                  ← agent entry point, root, not corpus
```

Two levels under `spaces/`, maximum. Any deeper nesting is a facet that belongs in the frontmatter.

---

## 3. The two rules

1. **One fact, one home.** Any other mention is a **link**, never a copy. This is what kills the
   five-home split, the RFC-003 collision, and the four homes of "secret classes".
2. **A link is a citation, never a promotion.** A document rises `exploration → evidence → canon →
   law` only through the promotion gateway, and only the human operator admits a promotion.

### 3.1 Promotion gateway

- `exploration → canon`: distilled into a specification with explicit invariants, bounds, and
  testable properties (`spaces/operator/README.md` already names this gateway).
- `canon → law`: **the rule becomes a contract, a Rust type, or a test** — never a promoted
  markdown. If it cannot be expressed machine-readably, it stays canon.
- No promotion is implicit. Every promotion is a recorded decision.

### 3.2 Routing rule (where a new document goes)

| question | answer |
|---|---|
| Is it machine-readable and cited by code? | `law` — `infra/` or `contracts/` |
| Does it state a normative rule a human must follow? | `canon` — `spaces/canon/<kind>/` |
| Does it record an observation, a measurement, or a decision? | `evidence` — `spaces/evidence/<kind>/` |
| Is it a hypothesis, an intuition, or a draft? | `exploration` — `spaces/operator/` |

---

## 4. The document contract — `cynic.document.v1`

Frontmatter is mandatory. It generalises the header the audits already carry (`schema`, `status`,
`method`, `authority_order`) rather than inventing a new one.

```yaml
---
schema: cynic.document.v1
title: Directory Structure Audit
authority: evidence            # law | canon | evidence | exploration
kind: audit                    # §1.2
domain: [host, storage]        # multi-valued, crate families
lifecycle: ratified            # draft | proposed | ratified | superseded | deprecated
method: observed               # observed | derived | specified | assumed
date: 2026-09-30
crates: [cynic-host, cynic-cli]
supersedes: null               # path, if this document supersedes another
---
```

`authority` is the only field that must match the directory. Every other field is a facet.

### 4.1 The metric

A rule, `audit docs`, reports `conforming / non-conforming / unclassified`. **The unclassified list
is the work queue** — not an estimate, a measurement. Same movement as `audit paths`: make the
discomfort visible instead of sorting blind.

---

## 5. The sequence — audit and canonicalise before moving

Ordered by dependency. **Steps 0-3 come before any file moves**; step 4 is terminal.

### 0. DOCTRINE — write the law

Ratify §0 and record it where a gate can read it: a `code_cites_prose` entry in
`contracts/foundation.json` (the doctrine must dogfood itself — a doctrine in markdown would
violate the doctrine).

### 1. AUDIT — make the violation visible

Add the rule to `crates/cynic-host/src/audit_engine`: **a Rust source may not cite a `.md` path as
`Authority:`**. It emits the 40 current citations as the work queue. Until this exists, every
canonicalisation step is unverifiable — the same reason `audit paths` had to exist before any move.

Two violations found by `audit paths` are already live and independent of this plan:
`cynic-host/.../worktree_observer.rs:120` → `infra/worktree_layout` and
`cynic-inference/src/domain/chat.rs:69` → `references/llms-101-mapping.md`, neither existing.

### 2. CANONICALISE — unify each cited source into machine-readable form

For each of the 18 cited markdown files, in citation-count order (worst first):

1. **Locate every home** of the rule it carries (the four homes of "secret classes" are the
   template).
2. **Elect the machine-readable home** — an existing contract, a Rust type, or a new taxonomy
   **only if** `foundation.json → no_contract_forest` permits it.
3. **Reconcile** the copies into that home; the losing homes become links or are deleted.
4. **Repoint the citations** from the markdown to the machine-readable home.
5. **Prove it** — `audit paths` green and `code_cites_prose` green for that file.

The first target is `AGENTS.md` §secret classes (11 citations, a phantom section, four competing
homes). The rule already exists as `cynic-protocol::secret_use`; the work is to make the
machine-readable home authoritative and delete the phantom citation.

### 3. TAXONOMY — consolidate the machine-readable corpus

Define what kind of document each of the 50 authority files is, and give each kind one home:
**constitution** (`foundation.json`), **policy**, **taxonomy**, **registry**, **ledger**. Add the
missing `schema` fields (`agent-roles.json`, `rust-toolchain.json`, `source-lifecycle.json`). This
step is the prerequisite for step 2 whenever the elected home is a new file — a new authority needs
a defined kind, or it enlarges the fragmentation it was meant to remove.

### 4. REORGANISE — the markdown corpus (terminal)

Only now, with zero prose cited as authority, can files move safely.

1. **Migrate by tier**, one tier at a time, `audit docs` + `audit paths` green after each.
   `infra/` keeps the machine-readable tier; canon moves to `spaces/canon/`; evidence to
   `spaces/evidence/`.
2. **Unify the five homes** (§0): fold `specs/` and `protocols/` into the tier tree, give every
   superseded document a `supersedes`/`lifecycle` record, retire the duplicate RFC-003.
3. **Index in Obsidian**: one MOC per tier, wikilinks from canon to law, backlinks as the proof that
   the graph resolves.

---

## 6. Rulings recorded

1. **Canon home**: `spaces/canon/` (operator ruling, 2026-10-05). `infra/` stays machine-readable.
2. **`docs/` end state**: absorbed by the tier tree. It is a contradiction today — declared
   "not law" and machine-guarded as a quarantine, yet `contracts/claims.json` holds 7 claims whose
   `home` is under `docs/`, and `infra/storage-taxonomy.json` cites `docs/notes/…` and
   `docs/foundation/…` as evidence. Absorbing means routing by tier and **rehoming those claims**,
   then deleting `docs/`.
3. **`protocols/` disposition**: folded into `spaces/canon/rfc/`, with the 14 code citation sites
   repointed — but only **after** step 2 has decided whether each cited rule is canon or a contract.
   13 of the 18 files have zero references and move freely.
4. **Migration scope**: not "the uncited ones move". Every prose document moves by tier; each move
   updates its citations and `audit paths` proves zero regression. Only the machine-readable tier
   (`contracts/`, `infra/*.json`) stays in place.

---

## 7. Open questions

1. **The taxonomy kinds** (§5.3): not invented here. Derived by the protocol in §8 — the naive
   classifier (citation count, `schema` presence, structural shape, rule vocabulary) already
   **failed** on this corpus: 29 of 50 files fall into one coarse bucket. That failure is the
   finding: the feature set must include the file's **observed usage** (how the code consumes it),
   not only its shape. Name the clusters only after the clustering separates them.
2. **`docs/` end state**: not a tombstone, not an archive. Rehome its 9 live references (7 claims in
   `contracts/claims.json`, 2 citations in `infra/storage-taxonomy.json`), then **delete the
   directory and retire the `docs_quarantine` gate** — a gate that guards a directory that should
   not exist is itself the defect. The need for archiving disappears with `lifecycle: superseded`:
   a superseded document keeps **one** home and points at its successor. That kills the four
   competing retention schemes (`archive/`, `docs/archive/`, `docs/quarantine/`, `specs/`) at once.
3. **Vault structure**: emerged from the data, not designed. `spaces/operator/index.md` references
   29 files of which **27 are broken** — the `canon/ → exploratory/` rename was never propagated,
   so the index is the ghost of a structure that no longer exists. The real clusters, by link
   density: `exploratory/traditions` (30 files, **120 wikilinks** — a genuinely connected cluster),
   `exploratory/manifesto` (10 files, 0 links — a linear book), `exploratory/whitepaper` (13 files,
   0 links — linear), `concepts` (2 files). Language is a real axis: 55 FR / 2 EN.

---

## 8. Method — how the taxonomy and the vault structure are derived

The taxonomy (§5.3) and the vault layout (§6.3) are **not designed**. They are derived by a
protocol that can be re-run, and whose failure modes are visible. The same shape applies to both.

### 8.1 Protocol

1. **State the hypothesis.** For the taxonomy: "there exist N document kinds such that the partition
   is exhaustive, mutually exclusive, and **stable under refactoring**." For the vault: "the
   structure is the connected components of the reference graph."
2. **Extract objective features — including observed usage.** Shape alone is insufficient (measured,
   §7.1). Features: citation count; presence of `schema`; structural signature (does the file
   enumerate entries or state rules?); rule vocabulary (`rule`/`policy`/`invariant`/`forbid`);
   `generated_at` / `report` markers; `shrink` markers; date literals; **and the observed
   consumption** — how the code reads it (`read` vs `resolve` vs `parse` vs `load`), which
   distinguishes a boot-time SSOT from a validated policy from a loaded registry.
3. **Cluster without supervision.** Normalise the features, run hierarchical clustering (or k-means),
   sweep k, pick k by silhouette score. **Do not fix k in advance.**
4. **Name the clusters — human, and it is a promotion.** The machine proposes the partition; the
   operator names each cluster and admits the taxonomy. This is the gateway of §3.1 applied to the
   corpus itself.
5. **Validate.** A kind is real only if it is **predictive**: the files in one cluster share
   consumers, change cadence, and validation rules. If they do not, the cluster is an artefact of
   the features and is rejected.

### 8.2 Applying it to the vault (§6.3)

1. **Build the graph** from wikilinks and file references (`spaces/operator`, 150 wikilinks).
2. **Compute components and hubs.** Connected components are the natural clusters; a hub is a MOC.
   `traditions` is one component (120 links); `manifesto` and `whitepaper` are linear chains with no
   cross-links — a different topology, and therefore a different treatment.
3. **Fix the index from the graph**, not by hand: `index.md` is regenerated from the graph, so it
   cannot drift from the structure the way it did (27 of 29 references broken).
4. **Let `kind`, `domain`, `lifecycle` be Obsidian properties**, not folders — the tree carries only
   the authority axis (§1.1), the facets are indexed.

### 8.3 What the method forbids

- Inventing a taxonomy from taste. If the features do not separate the corpus, the taxonomy is not
  yet found — that is a result, not a reason to force one.
- Fixing the number of kinds before clustering.
- Deriving the vault layout by hand. A hand-built index drifts; a generated one cannot.
- Treating "unclassified" as a failure. It is the work queue (§4.1).

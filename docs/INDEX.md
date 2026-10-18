# CYNIC Documentation Index & Authority Hierarchy

Status: Normative  
Authority: contracts/foundation.json §docs

---

## 1. Documentation Authority Hierarchy

1. **`docs/foundation/`** : Constitutional authority and philosophical foundation.
   - `CONSTITUTION.md` — Fundamental laws of governance, separation of powers, fail-closed principles.
   - `MANIFESTO.md` (and FR `manifeste.md`) — Epistemology: sovereignty, anti-monopoly, proof-over-claims.
   - `WHITEPAPER.md` (and FR `whitepaper.md`) — System architecture, economic model, and consensus.
   - `CRATE_RULES.md` — Rust crate boundaries, pure logic vs I/O separation.

2. **`docs/specs/`** : Active normative technical specifications.
   - `2026-10-12-ROADMAP-REFACTOR-CHANTIERS.md` — Single source of truth for ongoing refactoring and milestones.
   - `SPEC-CYNIC-STORAGE-V1.md` — Storage hierarchy (K1–K8) and evidence journal contracts.
   - `SOLANA_ANCHOR_ARCHITECTURE_SPEC.md` — Anchor on-chain settlement and integration.
   - `MULTI_AGENT_COORDINATION_SPEC.md` — Multi-agent claim, heartbeat, release lifecycle.
   - `EPISTEMIC_QSCORE_CONSENSUS_SPEC.md` — Epistemic validation and scoring engine.

3. **`docs/design/`** : System design proposals, RFCs, and architecture records.
   - `AGENT_FLEET_HOSTING_DESIGN.md` — Isolated agent fleet hosting architecture.
   - `MULTI_AGENT_ISOLATION_DESIGN.md` — Worktree and agent sandbox boundaries.
   - `session-report-knowledge-canon.md` — Epistemic distillation and knowledge base.
   - `CI_EVOLUTION.md` — CI suite survivability, the five adoption metrics, and the branch-protection step.

---

## 2. Governance Lifecycle Policy

- **Active** : Directly enforced by automated tests in `crates/` or contracts in `contracts/`.
- **Superseded / Historical** : Archived to `/home/user/Bureau/data_system/archive_cynic_cold/` to preserve a light, clean working repository.
- **Rule** : No design or specification document may declare paths or capabilities that have no executing code or test.

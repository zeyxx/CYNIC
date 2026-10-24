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
   - `CYNIC_SOVEREIGN_GPS_SPEC.md` — Master architectural compass, 4-layer sovereign stack, crate quadrant topology, and convergence milestones.
   - `2026-10-12-ROADMAP-REFACTOR-CHANTIERS.md` — Single source of truth for ongoing refactoring and milestones.
   - `SPEC-CYNIC-STORAGE-V1.md` — Storage hierarchy (K1–K8) and evidence journal contracts.
   - `SOLANA_ANCHOR_ARCHITECTURE_SPEC.md` — Anchor on-chain settlement and integration.
   - `MULTI_AGENT_COORDINATION_SPEC.md` — Multi-agent claim, heartbeat, release lifecycle.
   - `EPISTEMIC_QSCORE_CONSENSUS_SPEC.md` — Epistemic validation and scoring engine.

3. **`docs/design/`** : System design proposals, RFCs, and architecture records.
   - `AGENT_FLEET_HOSTING_DESIGN.md` — Isolated agent fleet hosting architecture.
   - `MULTI_AGENT_ISOLATION_DESIGN.md` — Worktree and agent sandbox boundaries.
   - `CI_SOVEREIGNTY.md` — CI role split (Forgejo blocking gate vs GitHub advisory hygiene), runner pool and label design, single-writer rule for `main` with the divergence watchdog spec.
   - `session-report-knowledge-canon.md` — Epistemic distillation and knowledge base.
   - `CI_EVOLUTION.md` — CI suite survivability, the five adoption metrics, and the branch-protection step.
   - `NETWORK_ENROLLMENT.md` — Mandatory network enrollment gate: status machine, per-plane probes, registry record. No IP/route/peer/workload before enrollment.
   - `POOL_ELIGIBILITY.md` — Build pool eligibility: declared pool labels + proven toolchain. A node may not take a build job unless both bricks pass.
   - `BASE_ENROLLMENT.md` — Base enrollment as the precondition for any role: identity, reachability, accountability. No role, no label, no workload before it.

4. **`docs/public/` & `docs/articles/`** : Public Epistemic Surface & Autobiographical Canon.
   - `docs/public/public_surface.html` — Live sovereign public surface (standalone dark terminal HTML, zero external CDN): 35 repos across 4 eras, 453 PRs (26 external across 9 ecosystems), 8 issues (Anatoly Yakovenko bounties, upstream Rust compiler bug), calibrated skills matrix, and verified claims ledger.
   - `docs/public/public_surface.md` — GitHub-flavored markdown export of the 4-era odyssey, interaction graph, and verified claims ledger (`cynic.facts.live.v1`).
   - `docs/public/facts.live.json` — Machine-verifiable JSON ledger of live metrics and physical substrate proofs.
   - `docs/articles/odyssey/ERROR_404_AUTOPSY_OF_A_HACKER_FR.md` — Foundational manifesto / autopsy (French).
   - `docs/articles/odyssey/ERROR_404_AUTOPSY_OF_A_HACKER_EN.md` — Mirror parity English translation.

---

## 2. Governance Lifecycle Policy

- **Active** : Directly enforced by automated tests in `crates/` or contracts in `contracts/`.
- **Superseded / Historical** : Archived to `/home/user/Bureau/data_system/archive_cynic_cold/` to preserve a light, clean working repository.
- **Rule** : No design or specification document may declare paths or capabilities that have no executing code or test.

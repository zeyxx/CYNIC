# Branch Convergence — map and order toward main

Status: `proposed` — tranche order needs operator sign-off; no merge runs
before it. Read-only inventory date: 2026-10-06.
Schema: `cynic.branch_convergence.v1`
Mandate: converge all agent work toward `main`, consolidate unfinished
chantiers (hypervisor/cynic-proxmox, secrets migrations, vocabulary).

## 1. Inventory (observed, not asserted)

| Branch | Behind/Ahead vs main | Date | Content (headline) |
|---|---|---|---|
| core-extraction | 1 / 57 | 10-06 | This session: secrets tri-state + sealed install, runtime-update contract + activation/rollback/GC, gate self-sabotage fix, lock refresh, roles + infra + convergence docs |
| knowledge-canon | 1 / 83 | 10-06 | **cynic-proxmox extraction (L1, 6 adapters + 4 ports)**, cynic secrets CLI phase 3, storage-governance contract, proxmox consolidation plan |
| network-mycelium-unification | 1 / 78 | 10-06 | Registry authority + reconciliation contracts (57 files vs canon, +3983/-614) |
| network-convergence | 1 / 88 | 10-06 | (largest — content TBD in tranche review) |
| network-vocabulary | 1 / 59 | 10-06 | Vocabulary migration tranche (this session pushed here) |
| debt-priorities-architecture | 1 / 12 | 10-03 | Small, recent enough to merge fast |
| plane-taxonomy-v1 | 186 / 13 | 10-06 | Ancient base, 13 commits — superseded candidate, delete unless claimed |
| multi-agent-experiment | 36 / 1 | 09-30 | Stale experiment, 1 commit — delete unless claimed |
| public-surface | unpushed (local only) | — | **At-risk work**: exists only in CYNIC-public-surface worktree. Push first, review second. |

Worktrees: main CYNIC (core-extraction, 84 foreign-modified files — other
agents' WIP, never touch), canon / network-worktree / public-surface
(clean). Builder: network-vocabulary @16ee545d (gate proof in flight),
source repo knowledge-canon (ACTIVE — HEAD moved 49e73f6d→7f8e9599 during
this session, dirty), kernel-graisse checkout (dirty, solar-pro4 branch).

## 2. Unfinished chantiers and where they live

- **cynic-proxmox + rename**: lives in knowledge-canon. Rename target
  still undeclared by operator.
- **Secrets migrations**: this session (core-extraction/network-vocabulary).
  Headscale + Freebox in OS store, cleartext purge + sealed migration gated
  on canary deploy.
- **Registry / vocabulary**: network-vocabulary + network-mycelium-unification
  overlap (mycelium adds registry_authority + reconciliation contracts).
  Reconcile canon↔mycelium before either touches main.
- **/cloud-api quarantine, kernel home, watchdog**: decided/recorded in
  runtime-roles + production-infrastructure (this session), execution pending.

## 3. Tranche order (proposed)

1. **Secure at-risk work**: push `public-surface` to origin (backup, no
   review implied). Zero merge risk.
2. **Drop or claim the dead**: plane-taxonomy-v1 (13 on ancient base),
   multi-agent-experiment (1 stale commit). Default = delete after
   operator confirms nobody owns them.
3. **Small first**: debt-priorities-architecture (12 commits) → review →
   merge to main. Proves the lane.
4. **Reconcile the overlap**: knowledge-canon ↔ network-mycelium-unification
   (registry contracts touch the same ground). One reconciled tree, then:
5. **Land the reconciled stack + convergence + vocabulary + core-extraction**
   onto main, smallest diff first, each with builder proof (gate green per
   the fixed gate).
6. **Freeze rule during 3–5**: one agent, one tranche, claims held;
   foreign WIP in the shared worktree is never staged (`git add -A`
   interdit — already law).

## 4. Standing risks (explicit)

- Shared worktree + moving main: every tranche rebases onto fresh main and
  re-proves. A green proof on a stale base is worthless.
- The source repo and kernel-graisse checkout are ACTIVE (other agents).
  Tranches 4–5 coordinate with them or collide.
- 84 foreign-modified files in the main worktree: their owners must commit
  or stash before any merge touches those paths.

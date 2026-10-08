# CYNIC Convergence Tracker

**Created**: 2026-10-08
**Branch**: `agent/hermes/convergence-triage`
**Worktree**: `/tmp/cynic-convergence`
**Status**: IN_PROGRESS

Full picture: all branches, all findings, all emergences, all steps.
Check items off as they are executed. Add new items as they emerge.

---

## 1. Branch Inventory

### Local branches (12 + main)

| Branch | Unique | Last commit | Verdict |
|---|---|---|---|
| `persistence-drift-rebase` | 26 | 8h | ✅ **Clean** — rebased on origin/main |
| `scan-exclusions` | 118 | **23s** | 🔴 **Active** — do not touch |
| `persistence-drift` | 94 | 26h | Diverged (old base) |
| `core-extraction` | 67 | 32h | ⭐ **Merge** — richest (release, secrets, audit) |
| `knowledge-canon` | 54 | 2j | Local-only, remote gone |
| `public-surface` | 54 | 2j | Local-only, diverged |
| `network-convergence` | 88 | 2j | Local-only, remote gone |
| `network-mycelium-unification` | 78 | 3j | Local-only, remote gone |
| `network-vocabulary` | 35 | 3j | Local-only, diverged |
| `recovery/core-clean-20261006` | — | — | Snapshot |
| `recovery/pre-reboot-20261006` | — | — | Snapshot |
| `backup/hermes-pre-rebase-20261004-132317` | — | — | Snapshot |
| `backup/hermes/persistence-drift` | — | — | Snapshot |

### Remote branches (4 + 2 recovery)

`debt-a3`, `persistence-drift`, `persistence-drift-rebase`, `scan-exclusions`, `recovery/core-clean-20261006`, `recovery/pre-reboot-20261006`

### Branches to archive (4)

| Branch | Reason |
|---|---|
| `network-vocabulary` | 0 unique commits |
| `network-mycelium-unification` | 6 unique, duplicate of convergence |
| `knowledge-canon` | 2 unique, absorbed |
| `public-surface` | 3 unique (CV docs), rest duplicate |

### Branches to merge (3)

| Branch | Unique | Approach |
|---|---|---|
| `core-extraction` | 67 | **Merge** origin/main (rebase = cascade conflicts) |
| `network-convergence` | 88 | Merge origin/main |
| `persistence-drift-rebase` | 26 | Already clean — direct PR |

### Branches delegated (1)

| Branch | Scope | Agent | Plan |
|---|---|---|---|
| `scan-exclusions` | `infra/data_system/` (19 commits) | Agent scan-exclusions | MERGED into main (PR #49, commit a533ecd5). 19 commits, 155 tests, Triad Charter & Bridge Spec sealed. |

---

## 2. Findings from Non-Merged Branches (F1-F6)

These problems have been **diagnosed and fixed** in branches not yet merged.
They must land before protocol consolidation is meaningful.

### F1 — Binary path fragmentation (5 shapes coexist)

**Commit**: `db9340c4` (in `core-extraction`, `persistence-drift`, `public-surface`, `scan-exclusions`)

| # | Source | Path |
|---|---|---|
| 1 | `infra/directory-tree.json:73` (SSOT) | `/var/lib/cynic/runtime/current/bin/cynic` |
| 2 | `cynic-pkg plan_release_deployment` | `<root>/current/<bin>` + `<root>/bin/<bin>` |
| 3 | `cynic-host framework_bootstrap` | `$RUNTIME_INSTALL_ROOT/current/<bin>` |
| 4 | `infra/systemd webhook unit` | `/var/lib/cynic/bin/cynic` |
| 5 | `cynic-protocol runtime_seed.rs` (L0!) | `/opt/cynic/runtime` |

**Measured**: Canonical path **absent on every host probed**. On Cortex, none of
`/var/lib/cynic/runtime/current`, `/var/lib/cynic/current`, `/var/lib/cynic/bin`
exist. On PVE (10.10.10.1), none of the four candidate paths exist.

**Impact**: `audit.deployed` is fail-closed — absent install is explicit unknown.

**Tasks**:
- [ ] Reconcile 5 path shapes into one canonical path
- [ ] Update `cynic-protocol runtime_seed.rs` to use SSOT path
- [ ] Update `infra/systemd` units to use SSOT path
- [ ] Deploy binary to canonical path on all hosts

### F2 — Binary installed at wrong location

**Observation**: `cynic` at `~/.local/bin/cynic` (`user:user`), not at
canonical `/usr/local/bin/cynic` (`cynic:cynic`) per `infra/runtime-basket.json`.

**Impact**: CLI runs as `user`, runtime runs as `cynic`. Secrets at
`/etc/cynic/secrets/` are `root:cynic 600` — unreadable by `user`.

**Tasks**:
- [ ] Move binary to `/usr/local/bin/cynic` under `cynic:cynic`
- [ ] Or: make `cynic source deploy` the canonical install path
- [ ] Update `PATH` references in scripts and docs

### F3 — Token resolution already fixed (not merged)

**Commits**: `59646b51`, `e0d76071` (in `core-extraction`)

- `feat(cli)`: resolve VCS token from env → file → OS keyring (fail closed)
- `fix(cli)`: fail closed on missing VCS token with named sources

**Status**: Code in `core-extraction`, not in `main`. Current binary uses old resolution.

### F4 — Secret presence tri-state (not merged)

**Commit**: `bdd03bcc` (in `core-extraction`)

- `fix(secrets)`: tri-state presence (`unknown` behind permission boundary)
- Install seals via vault port

**Impact**: Without this, permission-denied secret read is indistinguishable
from absent secret. Tri-state makes the boundary explicit.

### F5 — Sealed per-secret data key (not merged)

**Commit**: `2fad1576` (in `core-extraction`)

- `fix(secrets)`: sealed per-secret data key, fail-closed read
- System principal seed path

**Impact**: Each secret encrypted with own data key, sealed by KEK.
Compromised secret does not compromise others.

### F6 — Sealed install via vault port (not merged)

**Commit**: `1593dedb` (in `core-extraction`)

- `fix(release)`: capture rollback target before switch
- Sealed install via vault port

**Impact**: Release deployment uses vault port for secret injection,
never exposing secret in process list or environment.

---

## 3. Emergences → Consolidation Backlog (E1-E8)

### E1 — Forgejo token fragmentation

**Observation**: Token at `/etc/cynic/secrets/forgejo-token` (40 bytes).
Permissions were `root:root 600` — unreadable by `user`. Correctif immédiat
exécuté par l'opérateur (2026-10-08) mais pas encore effectif sur le binaire
courant (`cynic source pr list` échoue encore).

**Root cause**: Binary at `~/.local/bin/cynic` runs as `user`, not `cynic`.
Token is `root:cynic` — only `cynic` user can read it.

**Impact**: `cynic source pr list/open/merge` all fail. PR lifecycle requires
manual `curl` fallback.

**Tasks**:
- [ ] Rebuild binary after correctif to verify token resolution works
- [ ] Audit all secret path references across cynic-secrets adapters
- [ ] Unify on registry-declared `credential_ref` paths
- [ ] Add `cynic secrets resolve --ref forgejo-token` surface
- [ ] Wire `cynic source pr` to use cynic-secrets resolution

### E2 — No native `cynic branch create`

**Observation**: No direct branch creation command. `cynic source worktree
--allocate` covers it (creates branch + worktree), but no
`cynic git branch create` for simple branch operations.

**Impact**: Minor — `worktree --allocate` covers the use case.

**Tasks**:
- [ ] Decide: add `cynic git branch create` or document `worktree --allocate` as canonical

### E3 — No `cynic release` surface

**Observation**: `cynic source deploy` deploys pre-built binaries but no
`cynic release` command for full release lifecycle (build → sign → publish → verify).

**Impact**: Release process is manual (SSH to builder, scp binary).

**Tasks**:
- [ ] Design `cynic release` family (build, sign, publish, verify)
- [ ] Integrate with `cynic build` for proof
- [ ] Add `cynic source deploy` as the deploy arm

### E4 — No git hooks enforcement

**Observation**: No `pre-commit`, `commit-msg`, or `pre-push` hooks. Agent
identity not enforced. Claim check not enforced. `git add -A` not blocked.

**Impact**: Protocol rules are documentation-only, not enforced.

**Tasks**:
- [ ] Create `commit-msg` hook: verify agent identity (not `jeanterre552`)
- [ ] Create `pre-commit` hook: verify `.agent-claims.json` has active claim
- [ ] Create `pre-push` hook: verify builder proof exists for commit
- [ ] Install hooks via `core.hooksPath` or `.git/hooks/`

### E5 — `main` local never fast-forwarded

**Observation**: Local `main` is 218 commits behind `origin/main`. No
automation keeps it in sync.

**Impact**: Divergence silent. Agents branching from local `main` get stale base.

**Tasks**:
- [ ] Add `post-merge` hook to FF local `main`
- [ ] Or: document `git branch -f main origin/main` as mandatory pre-branch step
- [ ] Or: add `cynic git ff-main` surface

### E6 — 127 dirty files unattributed

**Observation**: Shared worktree has 127 dirty files (69 M, 31 D, 15 ??, 12 MM)
with no active claims. Cannot determine ownership or safety.

**Impact**: Risk of data loss. Cannot safely clean or merge.

**Tasks**:
- [ ] Classify dirty files by agent (git log + file ownership)
- [ ] Classify by concern (crate, infra, docs)
- [ ] Commit or archive recoverable slices
- [ ] Report unattributed files to operator

### E7 — No merge gate native

**Observation**: `cynic source pr merge` exists but no
`cynic merge --gate builder` that enforces builder proof before merge.

**Impact**: Merges can happen without builder validation.

**Tasks**:
- [ ] Add builder proof check to `cynic source pr merge`
- [ ] Add `--gate` flag requiring green proof
- [ ] Block merge if proof missing or red

### E8 — cynic-secrets path resolution fragmented

**Observation**: `file.rs` uses `base_dir.join(reference.id)` but `base_dir`
is set differently across surfaces. `with_legacy_dir` exists but not
consistently used. Registry `credential_ref` paths not always respected.

**Impact**: Secrets work on some surfaces, fail on others.

**Tasks**:
- [ ] Audit all `FileSecretStore::new()` calls
- [ ] Unify on registry-declared paths
- [ ] Add `cynic secrets doctor` surface
- [ ] Add integration test: resolve from registry path

---

## 4. Execution Plan

### Phase 1 — Branch Triage

- [ ] Archive `agent/hermes/network-vocabulary` (0 unique commits)
- [ ] Archive `agent/hermes/network-mycelium-unification` (6 unique, duplicate)
- [ ] Archive `agent/hermes/knowledge-canon` (2 unique, absorbed)
- [ ] Extract 3 CV docs from `public-surface` → `core-extraction`, then archive `public-surface`
- [ ] Merge `origin/main` into `core-extraction` (NOT rebase — cascade conflicts)
- [ ] Merge `origin/main` into `network-convergence`
- [x] Merged `persistence-drift-rebase` into main (PR #48, commit b82e88b8)

### Phase 2 — Builder Validation

- [ ] Submit `core-extraction` to builder via `cynic build submit`
- [ ] Submit `network-convergence` to builder via `cynic build submit`
- [ ] Submit `persistence-drift-rebase` to builder via `cynic build submit`
- [ ] Read back `cynic build proofs` — all 3 must be green

### Phase 3 — PR Lifecycle (native surfaces)

- [ ] Open PR for `core-extraction` via `cynic source pr open`
- [ ] Open PR for `network-convergence` via `cynic source pr open`
- [ ] Open PR for `persistence-drift-rebase` via `cynic source pr open`
- [ ] Human review each PR
- [ ] Merge each PR via `cynic source pr merge` (after builder gate)

### Phase 4 — Deployment

- [ ] Build release binary on builder (`cargo build --release -p cynic-cli`)
- [ ] Deploy via `cynic source deploy --apply`
- [ ] Verify deployed binary version

### Phase 5 — Worktree Hygiene

- [ ] `git worktree prune` (remove stale `/tmp/debt-a3`)
- [ ] Fast-forward local `main` to `origin/main`
- [ ] Classify 127 dirty files in shared worktree (by agent/concern)

### Phase 6 — Protocol Consolidation

- [ ] `infra/BRANCH_PROTOCOL.md` — single source of truth
- [ ] Update `cynic-branch-lifecycle` skill with enforced rules
- [ ] Update `cynic-worktree-governance` skill with new surfaces
- [ ] Remove duplicate rules from skills (consolidate into protocol doc)
- [ ] Install git hooks (E4)
- [ ] Add merge gate (E7)
- [ ] Fix binary path (F1, F2)
- [ ] Fix token resolution (E1, F3)

---

## 5. Merge Strategy

### Why merge, not rebase

Rebase of `core-extraction` (67 unique commits) on `origin/main` produces
cascade conflicts across 6+ files (Makefile, cynic-vcs/Cargo.toml, lib.rs,
policy.rs, ports.rs, architecture_boundary.rs). This is the documented
merge chaos pattern.

**Merge approach**: One conflict resolution, history preserved, all fixes land.

### Merge order

| Priority | Branch | Reason |
|---|---|---|
| 1 | `core-extraction` | Contains F1-F6 (secrets, deploy, path, token) |
| 2 | `network-convergence` | Network refactor (6 unique commits) |
| 3 | `persistence-drift-rebase` | Already clean (26 ahead, rebased) |
| 4 | `scan-exclusions` | Active — do not touch |

### What each merge unlocks

| Merge | Unlocks |
|---|---|
| `core-extraction` | Token resolution, tri-state presence, sealed install, canonical path |
| `network-convergence` | Network refactor, canonical plane accessors |
| `persistence-drift-rebase` | Architecture boundary test fix |

---

## 6. Documented Contradictions (6 cases) — ALL RESOLVED

Each contradiction is a design conflict where multiple branches solve the same
problem differently. Resolved via `cynic-contradiction-distiller` protocol.

| # | Domain | Severity | Decision | Lesson |
|---|---|---|---|---|
| C1 | network-refactor | HIGH | MERGE (B then A) | More recent ≠ more complete |
| C2 | network-refactor | HIGH | ADOPT A | More recent + more complete = supersedes |
| C3 | cli-surface | MEDIUM | ADOPT B (A then B) | Descendant = dependency order |
| C4 | audit-engine | MEDIUM | ADOPT A+B | No overlap = coexist |
| C5 | secrets-vault | MEDIUM | ADOPT A+B | Different files = independent |
| C6 | build-policy | LOW | ADOPT A+B (B then A) | Semantic dependency (test → code) |

### C1 — network-refactor: Two approaches to plane vocabulary (HIGH) ✅ RESOLVED

**Approach A**: `43431403` (core-extraction) — "trio-primary plane resolution with declared legacy aliases"
- Date: 2026-10-06 03:10 | Author: hermes | Files: 3

**Approach B**: `f5be5013` (network-convergence) — "make plane vocabulary canonical with legacy aliases"
- Date: 2026-10-05 15:51 | Author: jeanterre552 | Files: 14

**Dependency**: None (parallel development)

**Decision**: **MERGE** — cherry-pick B first (more complete, 14 files), then A (more recent, adds trio-primary).

**Lesson**: When two branches diverge, the more recent branch has more context but not always more completeness. Evaluate both dimensions.

### C2 — network-refactor: Two approaches to mesh model (HIGH) ✅ RESOLVED

**Approach A**: `a28d1735` (core-extraction) — "trio-primary reads across node/host/cli/kernel"
- Date: 2026-10-06 03:12 | Author: hermes | Files: 10

**Approach B**: `69e34e60` (network-convergence) — "eliminate duplicate MeshRegistry model"
- Date: 2026-10-05 04:45 | Author: jeanterre552 | Files: 3

**Dependency**: None (parallel development)

**Decision**: **ADOPT A** — cherry-pick `a28d1735` only. `69e34e60` is obsolete (written before, less complete).

**Lesson**: When one branch is both more recent AND more complete, it supersedes the other. No merge needed.

### C3 — cli-surface: Mask/judge unification vs persistence audit (MEDIUM) ✅ RESOLVED

**Approach A**: `b0287cf6` (core-extraction) — "unify mask, judge and judgment into one mask surface"
- Date: 2026-10-05 22:37 | Author: jeanterre552 | Files: 10

**Approach B**: `06289d56` (scan-exclusions) — "wire drift predicate to surface"
- Date: 2026-10-07 13:53 | Author: hermes:persistence-audit | Files: 3

**Dependency**: A is ancestor of B (proven via `git merge-base --is-ancestor`). B depends on A.

**Decision**: **ADOPT B** — cherry-pick A first (unified surface), then B (persistence audit). No contradiction, just dependency order.

**Lesson**: When B is a descendant of A, there is no contradiction — just a cherry-pick order.

### C4 — audit-engine: Debt ledger vs drop fields (MEDIUM) ✅ RESOLVED

**Approach A**: `6f3e9cc0` (core-extraction) — "the debt ledger — declared, counted, shrink-only"
- Date: 2026-10-04 15:31 | Author: CYNIC Builder | Files: 3 (debt_ledger.rs, mod.rs, audit-debt-ledger.json)

**Approach B**: `dc268c6d` (scan-exclusions) — "drop four fields the WorkspaceSourceIndex test helper does not have"
- Date: 2026-10-07 17:03 | Author: hermes:path004 | Files: 1 (prose_citation_rule.rs)

**Dependency**: None (no file overlap, no semantic dependency)

**Integration check**: A does not reference any of the 4 dropped fields (`unit_files`, `unit_contents`, `exec_starts`, `claim_store`). A mentions `WorkspaceSourceIndex` only in a comment. A does not use `prose_citation_rule`. B only modifies a test helper.

**Decision**: **ADOPT A+B** — cherry-pick both, no order required. Completely independent changes.

**Lesson**: Not every contradiction is a conflict. Sometimes A + B simply coexist. Always check integration before assuming conflict.

### C5 — secrets-vault: Token resolution vs sealed install (MEDIUM) ✅ RESOLVED

**Approach A**: `59646b51` (core-extraction) — "resolve VCS token from env, file, then OS keyring"
- Date: 2026-10-07 10:53 | Author: hermes | Files: 1 (source_surface.rs)

**Approach B**: `1593dedb` (core-extraction) — "capture rollback target before switch; sealed install via vault port"
- Date: 2026-10-06 22:52 | Author: hermes | Files: 3 (secret_surface.rs, lib.rs, release_deployment.rs)

**Dependency**: B is ancestor of A (A is more recent). No file overlap.

**Integration check**: A modifies `source_surface.rs` (token resolution). B modifies `secret_surface.rs` (sealed install). Both use `cynic_secrets` but with different imports (`OsCredentialStoreSecretProvider` vs `CompositeSecretProvider`) in different files. No cross-reference.

**Decision**: **ADOPT A+B** — cherry-pick both, no order required. Completely independent changes.

**Lesson**: Two commits in the same branch can be independent if they touch different files.

### C6 — build-policy: Build policy JSON vs gate split (LOW) ✅ RESOLVED

**Approach A**: `25749045` (persistence-drift-rebase) — "make infra/build-policy.json the single source"
- Date: 2026-10-07 21:32 | Author: hermes:ssot-jobs | Files: 3 (architecture_boundary.rs, build_policy_coherence.rs, build-policy.json)

**Approach B**: `50294409` (core-extraction) — "split the gate binary by subject"
- Date: 2026-10-06 12:25 | Author: jeanterre552 | Files: 1 (Makefile)

**Dependency**: No ancestry. No file overlap. **Semantic dependency**: A adds a test in `architecture_boundary.rs` that asserts the Makefile uses `CYNIC_SOURCE_BIN` — a variable introduced by B. Cherry-pick B first, then A.

**Decision**: **ADOPT A+B** — cherry-pick B (Makefile) first, then A (build-policy.json + tests).

**Lesson**: Even without file overlap, there can be a semantic dependency (test → code). Always check test assertions for references to introduced symbols.

---

## 7. Execution Log

| Timestamp | Action | Result |
|---|---|---|
| 2026-10-08 | Worktree created | `/tmp/cynic-convergence` |
| 2026-10-08 | Branch triage analyzed | 4 to archive, 3 to keep |
| 2026-10-08 | Token search | FOUND at `/etc/cynic/secrets/forgejo-token` (40 bytes) |
| 2026-10-08 | Token permissions | `root:root 600` — unreadable by `user` |
| 2026-10-08 | Correctif immédiat | Exécuté par l'opérateur |
| 2026-10-08 | Token resolution test | `cynic source pr list` échoue — binaire pas à jour |
| 2026-10-08 | Non-merged branch analysis | 6 findings (F1-F6) from 4 branches |
| 2026-10-08 | Rebase attempt | ABANDONED — cascade conflicts (6+ files) |
| 2026-10-08 | Merge attempt | ABANDONED — 35 files in conflict |
| 2026-10-08 | Domain classification | 196 commits, 18 domains, 4 branches |
| 2026-10-08 | Contradiction analysis | 9 duplicates + 6 contradictions |
| 2026-10-08 | scan-exclusions delegated | Agent handles own data_system PR |
| 2026-10-08 | cynic-contradiction-distiller created | Skill for formal contradiction resolution |

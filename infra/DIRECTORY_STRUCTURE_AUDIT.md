# Directory Structure Audit — declared arborescence vs observed filesystem

- **schema**: `cynic.audit.directory_structure.v1`
- **status**: `read_only_audit`
- **method**: observation only. No production file was mutated. Coordination was available and a
  single-file claim was taken to write this report
  (`claim_opencodedirectoryaudit_1790591527`, target `infra/DIRECTORY_STRUCTURE_AUDIT.md`).
- **authority_order applied**: observed runtime state > `contracts/foundation.json` >
  `infra/runtime-authority.json` > `infra/directory-tree.json` > `infra/registry.json` > workspace
  `Cargo.toml` > canonical crates > `infra/agent-exposure.json`
- **companions**: `infra/CLI_SURFACE_AUDIT.md` (command reachability),
  `infra/WORKSPACE_ORPHAN_DECISIONS.md` (module-graph orphans inside crates). This report covers
  neither. It covers the **filesystem arborescence**: what `infra/directory-tree.json` declares
  versus what exists, and which top-level directories carry authority they do not hold.
- **method limit**: git history was unreadable for most of this audit. See §0.1.

---

## 0. Why this audit exists

The operator observed that the repository is "absolutely fragmented" and asked where the blind
spots are. The premise is **partly wrong and importantly right**, and the distinction matters:

- The **crate taxonomy is not fragmented.** 47 crates, 47 in `Cargo.toml` `members`, 47 in
  `AGENTS.md` L0-L4, 47 in `CANONICAL_CRATE_MAP.md`. Zero drift in any direction.
- The **filesystem around the crates is fragmented.** 14 top-level directories, 21 dot-dirs and
  22 files at the root are not declared in the path SSOT. Six of them are pure residue. Three of
  them are cited as authority by production Rust code.
- The **enforcement is fragmented.** No existing gate validates that a *declared path exists* or
  that a *cited path resolves*. Every finding in §3 comes from that single missing capability.

### 0.1 Git object-store corruption — root cause, and why it was self-perpetuating

**Symptom.** 16 loose objects in `.git/objects` were **0 bytes**. One of them,
`737c34ee2f027e6256a478eea331d6589d902460`, was the value of
`refs/heads/agent/opencode-inference-audit/runtime-surface-wiring` — the checked-out branch — so
`git status`, `git diff` and `git commit` all failed with `fatal: bad object HEAD`. The
`AGENTS.md` git-isolation protocol was structurally inapplicable.

**Why it is worse than it looks.** Git's `write_loose_object()` begins with
`if (freshen_packed_object(oid) == 0) return 0;` — if a loose object file *exists at that path*,
git concludes the object is present and **skips the write without verifying content or size**. A
0-byte file is therefore a *stuck cache entry*: once it exists, no future `write-tree`,
`hash-object -w` or `commit` will ever repair it. The corruption is self-perpetuating, and every
retry makes it look unchanged.

**Second-order failure, and the dangerous one.** A poisoned object short-circuits silently rather
than erroring. `git diff-index HEAD` returned **rc=0 with empty output** while `HEAD:crates`
— the largest subtree, ~2 200 files — was unreadable. `git status` reported a clean tree. Git
was asserting "no changes" about a tree it could not read. Any audit that trusts `git status`
output on this repository was reading a lie.

**Composition of the 16:** 9 tree objects plus 7 blobs, all belonging to one aborted write — a
`cynic-inference` chat-trace slice (`chat_trace_sink.rs`, `chat_trace.rs`, `trace_sink_port.rs`,
`chat_service.rs`, and 4 `mod.rs`). The aborted commit's own object was among them. The
`19b80535` and `2e6f7210` trees are fully intact; `git ls-tree -r` reads 2 856 and 2 865 files
respectively with zero errors.

**Repair performed** (all steps reversible; nothing lost):

1. Branch ref moved off the 0-byte tip to the last reflog-valid commit. The reflog preserved the
   bad SHA, so the lost commit object itself is unrecoverable but its *content* survives in the
   worktree.
2. All 16 zero-byte files moved to a quarantine directory rather than deleted — they are not valid
   objects, so nothing can legitimately depend on their content, and keeping them preserves
   evidence of the incident.
3. The 7 lost blobs were regenerated from the worktree. This required `touch` first: the index
   entries were *racily clean* (stat data matched), so `git add` skipped re-hashing and therefore
   never rewrote the objects. All 7 came back under **byte-identical hashes**, which is the proof
   that the previous session's staged work survived intact.
4. `git write-tree` then produced a fully readable 2 860-file tree.
5. `git fsck` reports only dangling objects — no corruption, no missing objects.

**The lesson worth carrying.** The trigger was an aborted write (a full disk, a killed process, or
a filesystem-level interruption) at 12:04. Git has no self-healing for a truncated loose object
and no diagnostic that distinguishes "object exists" from "object is empty". A 0-byte object is
indistinguishable from a legitimately-written one to every git command that short-circuits on
existence. **A single `find .git/objects -type f -empty` is the only detector, and nothing in the
`AGENTS.md` protocol, the Makefile, or CI runs it.**

That check is cheap, has no dependencies, and belongs in `make check`. It is listed in §8 as a
prerequisite, not as an optional hardening.

---

## 1. Declared versus observed

`infra/directory-tree.json` is `status: normative` and declares itself the SSOT for all paths.
Reverse-mapping every top-level entry on disk against it:

| Top-level entry | On disk | In `directory-tree.json` | In `runtime-authority.json` | Verdict |
|---|---|---|---|---|
| `crates/` (47 crates) | yes | yes | yes | **ACTIVE** |
| `contracts/` (12) | yes | yes | yes | **ACTIVE** — authority rank 2 |
| `infra/` (200) | yes | yes (self) | yes | **ACTIVE** |
| `scripts/` (20 tracked) | yes | yes | yes | **MIXED** — 2 of 20 have real consumers |
| `evidence/` | yes | yes (as `/var/lib/cynic/evidence`) | yes | **COLLISION** — see §4.3 |
| `vault/` (61) | yes | yes (as `/var/lib/cynic/vault`) | yes | **COLLISION** — see §4.3 |
| `archive/` (18) | yes | yes | no | **DEAD** — self-declared superseded |
| `references/` (2) | yes | yes | no | **DEAD** |
| `tests/` (1 file) | yes | **NO** | no | **ACTIVE but undeclared** — see §2.1 |
| `docs/` (195) | yes | **NO** | yes (as quarantine) | **QUARANTINE** — enforced, see §4.1 |
| `protocols/` (18) | yes | **NO** | no | **ACTIVE but undeclared** — see §2.2 |
| `.codex/` (2) | yes | **NO** | no | **ACTIVE** (configured MCP projection) |
| `.cortex/` (42) | yes | **NO** | no | **DEAD** except 1 broken hook — §4.2 |
| `apps/` (39) | yes | **NO** | no | **DEAD** — §4.4 |
| `programs/` (5) | yes | **NO** | no | **ORPHAN CRATE** — §4.5 |
| `specs/` (4) | yes | **NO** | no | **UNCLEAR** — §4.6 |
| `bounty/` (8) | yes | **NO** | no | **DEAD** — §4.7 |
| `verify/` (2) | yes | **NO** | no | **DEAD** — gitignored ICE scratch |
| `recruiter/` (2) | yes | **NO** | no | **DEAD** — marketing prose |
| `.hermes/`, `.agents/`, `.cynic/`, `.cargo-home/` | yes | partial | no | gitignored residue |
| `hermes/` (no dot) | yes, **empty** | **NO** | no | **RESIDUE** |
| `.ruff_cache/`, `target*/` | yes | **NO** | no | build residue |

**14 top-level directories are absent from the path SSOT. Six are dead. Three are cited as
authority by running Rust code.**

### 1.1 The SSOT declares a path that does not exist

`infra/directory-tree.json` lists `crates/cynic-host/src/lifecycle/runtime_paths.rs`.
`crates/cynic-host/src/lifecycle/` does not exist. The canonical path registry points into a
directory the repository does not contain.

### 1.2 The SSOT is listed in `.gitignore`

`.gitignore:37` contains `/infra/directory-tree.json`, filed under `# Runtime artifacts`. The file
is currently tracked, so it survives — but any `git rm --cached`, any re-add, any fresh
materialisation outside this worktree silently drops the rank-4 authority file. Commit
`f01a4706 "chore(governance): adopt 8-level authority chain — directory-tree.json versioned"`
shows the intent was to version it. The ignore rule contradicts that intent and no gate detects
the contradiction.

---

## 2. Undeclared but load-bearing

### 2.1 `tests/` — one file, nine consumers, not in the SSOT

`tests/support/orphan_sources.rs` (11.7 KB) is the shared reachability detector for orphan `.rs`
files. It is included **by path** from 10 test binaries across 9 crates
(`#[path = "../../../tests/support/orphan_sources.rs"]` in
`cynic-{cli,host,inference,kernel,node,observer,protocol,zeroclaw-solana}/tests/orphan_sources_ratchet.rs`,
plus `cynic-cli/tests/crate_loc_ratchet.rs:36`).

Its existence is itself asserted by `crates/cynic-protocol/tests/architecture_boundary.rs:193-198`
("the shared detector at tests/support/orphan_sources.rs is missing; every crate ratchet includes
it by path"). Deleting it breaks 10 test binaries. It also anchors the workspace-root resolution
contract at `tests/support/orphan_sources.rs:36-45` (an ancestor must hold `Cargo.toml` +
`crates/` + `contracts/foundation.json`).

**The single most structurally load-bearing non-crate directory is absent from the path SSOT.**

### 2.2 `protocols/` — the most code-coupled prose directory, undeclared in all three authorities

18 normative design documents cited as `Authority:` from production Rust:

- `crates/cynic-protocol/src/machine_identity.rs:13` → `protocols/machine-enrollment-federation.md` D1
- `crates/cynic-node/src/usecases/federation_membership_uc.rs:16` → §3.2
- `crates/cynic-node/src/adapters/kernel_plane_membership_transport.rs:11` → step 3 wiring
- `crates/cynic-node/src/config/types.rs:66` → D3
- `crates/cynic-host/src/wireguard_readback.rs:7` → step 2
- `crates/cynic-protocol/src/intent.rs:15` → `protocols/h1-intent.md` §Digest ambiguity
- `crates/cynic-vcs/src/source_policy.rs:5` → "rules that were previously only *prose* in
  `protocols/agent-protocol.md`"

Its subordination is expressed by exactly one sentence, in one file:
`infra/agent-source-protocol.json:3` — "Rules here are authoritative; prose in `protocols/` is
projection." 13 of its 18 files have zero references from anywhere.

---

## 3. Duplications

### 3.1 `cynic-intel-escore` is a path-rewrite clone of `cynic-intel`, with zero consumers

Verified by `diff` after stripping `use` lines — the sole difference in `merkle.rs` is:

```
crate::escore::types::…    →    cynic_intel::escore::types::…
```

Six modules are clones: `aggregator.rs` (423/423), `decay.rs` (210/210), `merkle.rs` (196/196),
`metrics.rs` (104/104), and in futarchy `cpmm.rs` (138/138) plus `vault.rs` (69/69).
`cynic-intel` ∩ `cynic-intel-escore` = **20 shared public names**
(`EScoreMetrics`, `MerkleProof`, `Blake3Hash`, `AggregatorError`, `TokenEScore`,
`merkle_root`, `build_proof`, `verify_proof`, `calculate_decay`, `aggregate_wallet`, …).
`cynic-intel` ∩ `cynic-intel-futarchy` = 5 (`LiquidityVault`, `create_market`, `execute_bet`, …).

Three compounding defects:

1. **Inverted extraction.** Both sub-crates declare `cynic-intel = { path = "../cynic-intel" }`
   and `pub use cynic_intel::escore::types::*`. The "extracted" crate depends on the crate that
   still contains the code. Nothing was removed from `cynic-intel`.
2. **`cynic-intel-escore` has no consumer.** Verified: the only manifests naming it are the root
   `Cargo.toml` `members` list and its own manifest. ~933 duplicated LOC reachable from nothing.
3. **Violates R7** of `infra/CANONICAL_CRATE_MAP.md` — "One crate per cohesive functional domain —
   no micro-crate cascades." `escore` and `futarchy` are already internal modules of `cynic-intel`.

`cynic-intel-futarchy` has exactly one consumer (`cynic-cli`).

### 3.2 `cynic-vcs` / `cynic-vcs-forgejo` — name collision with no bridge

| Module | `cynic-vcs` | `cynic-vcs-forgejo` | shared public names |
|---|---|---|---|
| `forgejo_token.rs` | 195 | **999** | 0 |
| `pull_request.rs` | 207 | 144 | 0 |
| `forgejo.rs` | 358 | — | — |
| `forgejo_port.rs` | 48 | — | — |
| `forgejo_webhook.rs` | — | 201 | — |
| `forgejo_webhook_server.rs` | — | 401 | — |

`comm -12` on extracted `pub` names = **0**. `diff` shows 1 154 changed lines for
`forgejo_token.rs` and 337 for `pull_request.rs`. Two independent `ForgejoToken` and
`PullRequest` type families for one domain concept, with no reconciliation path. Harder to repair
than a clone: a duplicate is deletable, a divergence is a merge.

### 3.3 Three "specs" homes and an RFC number collision

- `specs/` — 4 files, **0** references from `crates/`, `Makefile`, `contracts/`, or `infra/`
- `docs/specs/` — 18 files, inside the machine-enforced quarantine
- `protocols/` — 18 files, 14 code citation sites, undeclared in all authority files

And a number collision: `specs/RFC-003-SOVEREIGN-DATA-CENTRIC-DISTILLATION.md` versus
`infra/RFC_003_AGENT_PARCEL_ENVELOPE.md`. Both claim RFC-003. The numbering is not continuous
across the two homes (RFC-002 lives in `infra/`, RFC-001 and RFC-003 in `specs/`).

`specs/SPEC-CYNIC-STORAGE-V1.md:3,7` declares itself "Ratified, Hardened & Sealed (2026-09-11)"
and "Supersedes: `cynic-kernel/src/storage/` (SurrealDB network coupling, SPOF F2)". Nothing reads
it. `infra/storage-layering.json` and `infra/storage-taxonomy.json` are the executable successors —
so this is a ratified spec silently outranked by JSON contracts, with no deprecation record.

`vault/README.md:9` designates `specs/RFC-XXX.md` as the mandatory gateway out of the vault. The
gateway has two live RFCs.

### 3.4 A contract cites the quarantine that the same contract declares

`infra/storage-taxonomy.json:149` → `docs/notes/d2-vault-integrity-scan-strategy.md`
`infra/storage-taxonomy.json:163` → `docs/foundation/D1-cold-evidence-storage.md`

Both targets are in `docs/`, which `contracts/foundation.json:106,108` declares "historical and not
runtime authority" and which is machine-enforced as a quarantine by
`crates/cynic-cli/src/agent_exposure_audit.rs:22,72` →
`crates/cynic-cli/src/agent_exposure_audit/checks.rs:375-389` (emits a **critical**
`docs_quarantine_missing` finding if `docs/README.md` is absent).

A K-class storage decision is anchored to prose the same repository repudiates and actively guards
against.

### 3.5 Three retention schemes for non-authority material

`archive/` (18 files, self-declares supersession at
`archive/architecture-pre-constellations/README.md:3-4`), `docs/archive/` (75 files, no
declaration), `docs/quarantine/` (28 files, includes `v2-architecture-canon/`). Plus
`specs/` for ratified-but-orphaned work. Four locations, no routing rule.

`archive/architecture-pre-constellations/CRATE_MAP.md` duplicates
`infra/CANONICAL_CRATE_MAP.md`. `archive/legacy-canon/tradition-data-draft.json` (4.5 KB) is the
superseded seed of `vault/canon/data/tradition-data.json` (20.7 KB).

---

## 4. Blind spots

### 4.1 `programs/cynic-anchor` — a crate the build can never see

`programs/cynic-anchor/Cargo.toml:2` declares `name = "cynic-anchor"`. The workspace `members`
list (`Cargo.toml:2-49`) contains only `crates/*`. There is **no `exclude` section**. The crate is
therefore neither a member nor excluded: `cargo check --workspace` and `cargo test --workspace` —
the CI gate at `.github/workflows/ci.yml` — never compile or test it. Its 5 `.rs` files
(`src/lib.rs`, `src/state.rs`, `src/cpmm_tests.rs`, plus the manifest pair) are invisible.

This is exactly the failure class `tests/support/orphan_sources.rs:3-8` exists to catch
("An undeclared `.rs` file is never compiled … invisible rather than broken") — but that detector
only walks crates inside the workspace, so it structurally cannot reach a crate outside it.

Referenced only from quarantined prose: `docs/specs/SOLANA_ANCHOR_ARCHITECTURE_SPEC.md:11`,
`docs/specs/ANCHOR_CLIENT_INTEGRATION_SPEC.md:19`, `docs/notes/amm-cynic-mapping.md:77`. It
duplicates the CPMM/AMM domain owned by `crates/cynic-market` and `cynic-solana-pay`.

### 4.2 `.cortex/zones.json` — a documented boot source that cannot boot

Three files assert the config is loaded from the repo at boot:

- `crates/cynic-kernel/src/domain/zones.rs:2` — "Loaded from .cortex/zones.json at boot"
- `crates/cynic-kernel/src/api/rest/types.rs:123,357` — `has_cortex`

The loader disagrees. `crates/cynic-kernel/src/bootstrap/services.rs:163-176`
(`selected_zones_path`) reads only `$CYNIC_ZONES_PATH`, else `/etc/cynic/zones.json`, else `None`.
`CYNIC_ZONES_PATH` appears **exactly once in the entire repository** — at that definition site —
and is set nowhere.

Worse, even if the variable were exported, `zones.rs:41-49` makes declared paths absolute
**relative to the config file's own directory**. `.cortex/zones.json` declares
`cynic-kernel/src/api/`, which would resolve to `.cortex/cynic-kernel/src/api/` — a directory that
does not exist. The correct runtime path is `/etc/cynic/zones.json`, consistent with
`infra/directory-tree.json:50`.

Two path SSOTs, one broken bridge, and a doc comment that is an executable lie.

The other 41 files in `.cortex/` are residue of a superseded agent-organism system:
`rules/python-lifecycle.md` (11 KB) + `rules/python.md` (4 KB) describe a Python lifecycle gate
that `AGENTS.md` lists as a **deleted authority**; `organs/judge/MANIFEST.json` implements
`organs/`, also deleted; `projects/-home-user-Bureau-CYNIC/` is a stale absolute path from a
*different* checkout root (actual: `-zeyxx-CYNIC`); `session-proof.json` is **0 bytes**.

`.cortex/mcp/run-cynic-mcp.sh` is a strict runtime-gating wrapper (refuses PATH fallback, probes
`--contract-json`) superseded by the unguarded root `.mcp.json` and `.mcp.cynic-gpu.json`, which
invoke `target/debug/cynic-mcp` and `ssh cynic-host-gpu cynic-mcp` directly. **The strict version
is the one nobody uses.**

### 4.3 Three `Supervisor` name collisions

| Name | Meaning A | Meaning B |
|---|---|---|
| `evidence/` | repo-local rustc-ICE crash corpus (18 files, 1 tracked) | `/var/lib/cynic/evidence/*.jsonl` — K1 immutable runtime evidence |
| `vault/` | Obsidian knowledge vault, 61 files, "pre-legislative" | `/var/lib/cynic/vault/models` — K5 GGUF model store |
| `.cynic/` | repo-local empty `staging-surrealdb-data/` | `~/.cynic` — the real operator state root |

A code path uses meaning B exclusively: `crates/cynic-inference/src/adapters/empirical_store/fs_empirical_store.rs:79`
and `src/infra/domain_task_queue.rs:120` both fall back to `home.join(".cynic/evidence/…")`;
`crates/cynic-inference/src/application/inference_gateway.rs:50` uses
`vault_dir: /var/lib/cynic/vault/models`. Declared at `infra/directory-tree.json:73,87-91,106-108`.

`vault/README.md:11-15` contains a "Name Collision Declaration (2026-09-28)" asserting that "no
governance file references this tree". That is now false: the collision is real and declared in
`infra/directory-tree.json`. The direction (repo vault is non-authority) remains correct; the
factual claim does not.

The repo-local `.cynic/` is the sharpest risk: if any consumer ever resolves a relative path
against CWD, it silently reads the wrong tree.

### 4.4 `apps/kyon-mobile` — a whole Android app, and a third `kyon`

31 tracked files, `so.cynic.kyon.*`. Zero references from `crates/`, `Makefile`, `contracts/`,
`infra/`, or CI. Not a workspace member; no Gradle invocation anywhere in the build system.

The name collides with an unrelated Rust domain: `cynic-eval/src/kyon/` is a "decision engine over
judgments" (`mod.rs:7`, `types.rs:50` `KyonAction`). Same name, unrelated meaning, zero linkage.
The only plausible integration surface is `KernelSyncWorker.kt` / `KernelObserveClient.kt`, and
nothing in Rust points back.

8 of the 39 on-disk files are **committed Gradle daemon state**
(`apps/kyon-mobile/.gradle/8.11.1/{checksums,executionHistory,fileChanges,fileHashes,gc.properties,cache.properties}`,
`.gradle/vcs-1/gc.properties`). `.gitignore` has `.gradle/` and `**/.gradle-user-home/` but the
files predate the rule.

### 4.5 `bounty/` — an expired external engagement, 1.1 MB

`bounty/veilo/INDEX.md:3` — "**Deadline**: 2026-08-20T22:59:59 UTC", "Agent access: HUMAN_ONLY".
Zero references from code, Makefile, contracts, or infra.

The directory measures 1.1 MB but holds ~20 KB of source. The remainder is
`bounty/veilo/poc-swap-skim/target/debug/incremental/poc_swap_skim-*/…/{dep-graph.bin (647 KB),
query-cache.bin (342 KB), work-products.bin}`. `.gitignore` covers `/target/` and
`crates/*/target/` but **no nested `target/` outside `crates/`**, so the pattern is structurally
incomplete.

### 4.6 `scripts/` — 2 live out of 20

Genuine consumers:

- `scripts/cargo` — asserted by `crates/cynic-cli/src/dev_surface/check.rs:217,407,440`
- `scripts/surface-audit-matrix` — `crates/cynic-cli/src/dev_surface/surface_matrix.rs` + `Makefile:236`

Cited in prose but effectively dead: `scripts/intent-seal-TEMP.py` (the name says TEMP; cited in
`crates/cynic-cli/src/agent_surface/intent.rs`). Cited in a contract but **nonexistent**:
`infra/benchmark-kv-sweep-plan.json:228` → `"execution_script": "scripts/benchmark-kv-sweep.sh"`.

Zero references anywhere: `benchmark-all-models.py`, `simulate_bts_collusion.py`, `simulate_pow.py`,
`surface-registry-assemble`, `disk-audit`, `runtime-evidence-bundle`, `upstream-sync-reconcile`,
`vault_sweep.py`, `schema_coverage_check.py`, `validate-verticals.py`, `run-matrix-benchmark.py`,
`run_qwen38_bench.sh`, `run_qwen38_matrix.sh`, `run_task_pipeline_test.py`, `cargo-vendor-bootstrap`,
`deploy-synapse-dns.sh`, `isolated_runner.sh`.

`.cortex/rules/python-lifecycle.md` + `python.md` describe a Python lifecycle gate that no script
in `scripts/` implements and no Makefile target invokes — the directory's own governance is
orphaned relative to its own content.

`scripts/__pycache__/` (2 `.pyc`, 30 KB) is **tracked** despite `.gitignore` carrying
`__pycache__/` and `**/__pycache__/`; the files predate the rule.

### 4.7 Other residues

- `verify/` — 2 standalone `.rs` type-check stubs that `include!` real CLI sources to escape the
  rustc ICE. `verify/fleet_surface_check.rs:1-4` says so. Fully subsumed by
  `crates/cynic-cli/src/fleet_surface.rs`, already covered by
  `crates/cynic-cli/tests/architecture_boundary.rs:1175-1181`. Gitignored, 0 tracked files.
- `hermes/` (no dot) — **completely empty**, 0 files, not gitignored, not declared, not referenced.
- `evidence/forensics/rustc-ice/README.md` hardcodes host `10.10.10.119`, violating the
  "never hardcode hosts" rule.
- `crates/cynic-host/target/` — **265 MB** of build output inside a source crate, including two
  generated `.rs` files (`serde_core-*/out/private.rs`, `serde-*/out/private.rs`) that a naive LOC
  census counts as source.
- 13 commits' worth of branch namespaces in `.git/refs/heads/agent/hermes/*` from a superseded
  agent generation.

### 4.8 Referenced-but-absent paths (the single class no gate detects)

| Reference | Target exists |
|---|---|
| `crates/cynic-mcp/src/helpers_split/misc.rs:86` → `contracts/consumer-profiles` | NO |
| `crates/cynic-mcp/src/helpers_split/openapi_route_contract.rs:5` → `docs/api/openapi.json` | NO |
| `crates/cynic-mcp/src/helpers_split/api_topology_contract.rs:4,121` → same | NO |
| `infra/benchmark-kv-sweep-plan.json:228` → `scripts/benchmark-kv-sweep.sh` | NO |
| `contracts/observability-loop.json:7` → `contracts/registry` | NO |
| `infra/directory-tree.json` → `crates/cynic-host/src/lifecycle/runtime_paths.rs` | NO |
| `crates/cynic-protocol/src/capacity.rs:3` → `.hermes/plans/2026-08-24_….md` | NO |
| `crates/cynic-protocol/src/task_pipeline.rs:3` → `.hermes/plans/2026-08-24_….md` | NO |
| `crates/cynic-protocol/src/agent_host_sandbox.rs:3` → `.hermes/plans/2026-09-01_….md` | NO |
| `crates/cynic-host/src/organic_opportunity/session_observer.rs:32` → `.hermes/sessions` (**live default**, gitignored) | NO |
| `contracts/foundation.json:107`, `CODEX.md:13`, `.codex/CODEX-PROTOCOL.md` → `make build`, `make build-release` | NO (targets) |

> **Grep trap**: the abundant `agent/hermes/…` hits across `crates/cynic-vcs/`,
> `crates/cynic-host/src/domain/git/`, and `worktree_observer.rs:243` are **git branch names**, not
> path references. Likewise `hf-cli` hits in
> `crates/cynic-host/src/domain/ecosystem_package/integration_apply.rs:158,399,413` are the runtime
> bootstrap path `/var/lib/cynic/.local/share/cynic/bootstrap/hf-cli-install.sh`, unrelated to the
> repo-local `.agents/skills/hf-cli/`.

---

## 5. Findings in the crate layer (summary; full census in `CANONICAL_CRATE_MAP.md`)

### 5.1 God-object ceiling: 6 of 47 in constitutional violation

Ceiling is **25 000 LOC** (`contracts/foundation.json:74`,
`strict_dag_and_single_responsibility`; superseded 40k figure in
`CANONICAL_CRATE_MAP.md` header). `AGENTS.md` still cites the stale 40k number.

| Crate | src LOC | × ceiling |
|---|---|---|
| `cynic-host` | 89 851 | **3.6×** |
| `cynic-node` | 58 327 | **2.3×** |
| `cynic-kernel` | 40 458 | 1.6× |
| `cynic-protocol` | 34 800 | 1.4× |
| `cynic-cli` | 34 735 | 1.4× |
| `cynic-inference` | 25 611 | 1.02× |

`CANONICAL_CRATE_MAP.md` v3 §2 is accurate: 41/47 exact, 6 off by ≤294 lines, total 387 851
declared vs 388 747 measured (+0.2%). The map is trustworthy and does not overstate compliance.

### 5.2 The ratchet is declared and not enforced

The 6 violating crates simultaneously carry **~105 one-line stub files** (`cynic-host` 45/497,
`cynic-node` 32/346, `cynic-kernel` 28/206) and **30+ masking attributes**:

| Location | Directive | Scope |
|---|---|---|
| `crates/cynic-cli/src/lib.rs:1` | `#![allow(clippy::print_stdout, clippy::print_stderr, dead_code)]` | **entire crate** |
| `crates/cynic-node/src/lib.rs:2`, `src/main.rs:2-3` | `#![allow(dead_code, …)]` | entire crate + binary |
| `crates/cynic-observer/src/lib.rs:1` | `#![allow(unused_imports)]` | entire crate |

`cynic-node` alone carries **19 `allow` attributes across 14 modules**, most of them whole-file
`#![allow(dead_code)]`: `watchdog.rs`, `watchdog/{kernel_bridge,host_runtime,sampler}.rs`,
`config_watch.rs`, `runtime_supervision.rs`, `runtime_supervision/lifecycle.rs`, `system_probe.rs`,
`announce.rs`, `supervise.rs`, `websocket.rs`. Entire subsystems are declared dead while the crate
breaches the ceiling at 2.3×. The ratchet is bypassed from both ends.

`AGENTS.md`: "Never: … mask unverified code with `allow(dead_code)`."

Four `cynic-kernel` files are 1-LOC residue of the 2026-09-28 `cynic-lifecycle` extraction
(`src/domain/lifecycle_{activation_evidence,activation_requests,diagnostics,workflow_state}.rs`):
the kernel shims were emptied but not pruned, against the explicit "dead branches pruned
destructively" directive.

### 5.3 `cynic-inference-runtime` is a 6-LOC shell with 14 phantom edges

`src/lib.rs` is 6 lines of `//!` prose. **0 public items, no `mod`, no `fn`, no `struct`.** Its
`Cargo.toml` declares 14 dependencies (`tokio` full, `reqwest`, `anyhow`, `ulid`, `sha2`,
`chrono`, `serde_json`) that are all unused because the crate exposes nothing.
`CANONICAL_CRATE_MAP.md` labels it "composition shell (new in v3; stub, 6 LOC)". It contributes 14
phantom edges to the build graph and nothing else.

### 5.4 11 of 47 boundary tests are cosmetic

Every crate has `tests/architecture_boundary.rs` (47/47) and `src/lib.rs` (47/47). But 11 files are
20-24 LOC containing **one test** that does a textual `contains(dep)` on `Cargo.toml`:

```rust
let forbidden = ["cynic-kernel", "cynic-node", "cynic-cli", "cynic-mcp"];
assert!(!content.contains(dep), …);
```

Affected: `cynic-mcp`, `cynic-observer`, `cynic-intel`, `cynic-market`, `cynic-inference-runtime`,
`cynic-discovery-host`, `cynic-host-probes`, `cynic-vcs-forgejo`, `cynic-agent-bus-server`,
`cynic-agent-bus-coordinator`, `cynic-zeroclaw-solana`. Four of these are among the largest crates
in the workspace. A substring match on a manifest cannot see a **transitive** upward edge, and
yields false negatives if a forbidden name appears in a comment.

The good pattern already exists in the repo: `cynic-domain-askesis` declares a 29-entry
`FORBIDDEN_DEPS` allowlist and asserts that `lib.rs` is a thin facade. That should be the template
for generated, graph-resolved boundary tests.

### 5.5 Self-contradiction in the crate map

`CANONICAL_CRATE_MAP.md` R3 states "No layer prefixes (-domain, -ports, -adapter, -application)".
Four crates carry the banned prefix — `cynic-domain-askesis`, `cynic-domain-ops`,
`cynic-domain-cloudflare`, `cynic-domain-porkbun` — and the same table marks three of them
`constellation ✓`. `cynic-domain-askesis` is a 124-LOC, 3-public-item, one-function shim
(`archive_zone_backup`) over the 2 486-LOC `cynic-askesis`; zero shared names, dependency pointing
the "wrong" way for a `-domain-` adapter. 13 renames and 8 crate creations from §3 of the map are
planned and **none executed**.

### 5.6 Authority drift

`crates/cynic-intel-soc/src/lib.rs:3` declares "Canonical domain owner: `cynic-intel` (SoC
foundation §481)". `crates/cynic-kernel/src/introspection/mod.rs:3` declares the opposite: "Pure
types and detection functions live in `cynic-intel-soc`." The dependency graph agrees with the
kernel: `cynic-intel-soc` does not depend on `cynic-intel`, and three crates consume it. The
docstring is stale.

### 5.7 What is genuinely healthy

47/47 crates have `lib.rs` + `architecture_boundary.rs`. Zero drift between `AGENTS.md` taxonomy
L0-L4 and `crates/`. Zero `todo!()`, zero `unimplemented!()` workspace-wide, 12 total
`TODO`/`FIXME` comments. Exactly one `cfg(feature)` in all of `src/` — no feature sprawl.
`tests/support/orphan_sources.rs` is included by 10 test binaries and its existence is asserted.
`contracts/` is referenced from code (`cynic-host/src/repo_access.rs:449,667` even as a
workspace-root marker), from `Makefile:143` (`jq empty`), and from
`.github/workflows/ci.yml:32`.

---

## 6. The pattern behind the findings

### 6.1 Six recurring patterns

1. **The ratchet is declared, not enforced.** The ceiling exists, the crate map exists, the
   extraction plan exists (13 renames, 8 creations, §3 of the map). 0 executed. Meanwhile the
   violating crates grow and accumulate one-line stubs and whole-file `allow`s.
2. **Guards are hand-written and cosmetic.** 47 hand-maintained boundary test files, 11 of them
   single-substring greps. The right shape exists once, in `cynic-domain-askesis`, and was not
   propagated.
3. **Silent authority drift.** Stale ownership docstrings, two conflicting LOC ceilings
   (25k vs 40k), a self-contradicting R3 rule, a `vault/README.md` collision declaration that
   contradicts `directory-tree.json`, an RFC claiming ratification with zero readers. No gate
   compares a claim against the executable state.
4. **CI cannot see the filesystem outside `crates/`.** `.github/workflows/ci.yml` runs `jq empty`
   on 5 JSON files then `cargo check/test --workspace`. Nothing in it touches `apps/`,
   `programs/`, `verify/`, `scripts/`, or `tests/support`. None of §4 can fail that gate. The CI
   also tests *more* than the sanctioned gate — `AGENTS.md` declares `make test` non-canonical, and
   it delegates to `check.layers`, not `--workspace` — while relaxing
   `CYNIC_ALLOW_LOCAL_DEV_BUILD: "1"` an admission check `contracts/foundation.json:92` says must
   "refuse compiling actions".
5. **Contamination in the tracked tree.** `scripts/__pycache__/` (tracked `.pyc`),
   `apps/kyon-mobile/.gradle/8.11.1/` (tracked daemon state),
   `bounty/veilo/poc-swap-skim/target/` (990 KB, untracked but escaping an incomplete
   `.gitignore`), `crates/cynic-host/target/` (265 MB inside a source crate), and 13 tracked files
   with non-ASCII names escaped as octal (`vault/canon/traditions/*.md`,
   `docs/archive/*.pptx`) which are non-portable across checkouts and locales.
6. **Name collisions without a bridge.** `evidence/`, `vault/`, `.cynic/`, `kyon`, `archive/`,
   `specs/`, RFC-003. Each is individually explicable; collectively they mean **no reader can
   resolve a path from prose without first knowing which of several homes won**.

### 6.2 The single missing capability

**No audit surface validates that a declared path exists, or that a cited path resolves.**

Every existing gate tests *form*: the module graph, `lib.rs` presence, manifest substrings, the
presence of `docs/README.md`. None tests *reference*. A surface that reads
`infra/directory-tree.json`, `infra/runtime-authority.json`, `infra/registry.json` and
`contracts/*.json`, extracts every repo-relative path literal from `crates/**/*.rs` and from
`Makefile`, `scripts/`, and the JSON contracts, and asserts existence-in-repo or
existence-in-declared-runtime-space would, on its own, close §1.1, §4.1, §4.2, §4.3, §4.5, §4.6,
and §4.8 — seven of the findings in this report.

This is the same shape as `tests/support/orphan_sources.rs` applied to paths instead of modules, and
it is the natural extension of the existing `cynic audit` family.

---

## 7. External standards: is there a canonical DDD structure to adopt?

The operator asked whether external projects have a canonical DDD directory structure to copy.
The short answer, grounded:

**No. There is no canonical DDD directory structure, and the industry has largely stopped
treating the folder tree as the boundary. The boundary that is enforced is the one that matters,
and in every ecosystem it is enforced by a *tool*, not by a convention.**

| Ecosystem | Enforced boundary mechanism | Canonical layout? |
|---|---|---|
| Go | `internal/` — **compiler-enforced** visibility. The only language-level boundary in mainstream use. | Partial: `cmd/`, `internal/`. `pkg/` is now widely treated as an anti-pattern. |
| Bazel / monorepo build | `visibility` + `package_group` + `--check_visibility_under` — **build-graph-enforced**. | Layout is project-defined; the graph is law. |
| Rust | **Nothing.** Cargo has no `internal/`, no dep visibility, no module privacy across crates. `[workspace] members`/`exclude` is the only structural tool, and `crates/*` is convention, not rule. | `crates/*` is the de-facto convention, per the [Cargo Book](https://doc.rust-lang.org/cargo/reference/workspaces.html) and the [workspace member auto-discovery Pre-RFC](https://internals.rust-lang.org/t/pre-rfc-workspace-member-auto-discovery/15368) ("Today, the conventional folder seems to be `crates`"). |
| JVM | Package visibility is **not hierarchical** — a sub-package is not nested in its parent — so the compiler cannot help. [Spring Modulith](https://docs.spring.io/spring-modulith/reference/fundamentals.html) supplies the missing enforcement via ArchUnit/jMolecules. | Package-by-feature (vertical slice), explicitly over package-by-layer. |
| .NET | No tool-level enforcement by default; `Clean Architecture` and `Vertical Slice` templates are both conventions. | `src/{Domain,Application,Infrastructure,WebApi}` is a template, not a standard. |

Two things this table makes concrete about CYNIC:

**(a) DDD-as-folder-tree is a widespread misreading.** The `domain/ application/ infrastructure/
interfaces/` skeleton that DDD examples show is package-by-*layer*; the community consensus
(strongly represented in the DDD literature itself) is package-by-*feature* — package by bounded
context, then by layer *inside* the context. CYNIC already does this correctly at the workspace
level: 47 crates named after domains, not after layers. A `src/{domain,application,adapters}`
skeleton would be a **regression**, not an alignment.

**(b) The only thing CYNIC is missing is the Spring-Modulith-shaped enforcement layer.** Modulith's
verification is three rules — no cycles between modules, efferent access via declared API packages
only, and an explicit per-module allow-list of permitted dependencies. CYNIC's `AGENTS.md` already
states exactly these three rules, and `CANONICAL_CRATE_MAP.md` already encodes the allow-lists as
its L0-L4 taxonomy. **The specification is done. Only the enforcement is missing** — and 11 of 47
boundary tests are one-line substring greps instead of it.

The convergent move, therefore, is *not* a new directory convention. It is to replace 47
hand-maintained `architecture_boundary.rs` files with **one generated check** driven by the
existing L0-L4 taxonomy: resolve the real dependency graph via `cargo metadata`, assert acyclicity,
assert each crate's dependency set is a subset of its declared layer's allowed set, and assert no
lower-layer-violating edge. That is Modulith's `verify()` with `cargo metadata` as the ArchUnit
substitute — and it converts findings §5.2, §5.4, §3.1 and §5.5 from "observed" to "impossible".

**One genuine divergence from every external standard, and it is correct:** CYNIC separates three
path spaces — `git_sources` (repo), `installed_system` (`/etc/cynic`), and runtime state
(`/var/lib/cynic`). That is the same build-time-vs-runtime separation as Nix, systemd and
Kubernetes, and it is the right shape. The defect is not the model; it is that the boundary between
those three spaces is declared in a JSON file that `.gitignore` tries to discard and that no gate
validates.

---

## 8. Recommended sequence

Ordered by dependency, not by severity. Steps 0-2 are prerequisites; 3-5 are the durable repairs.

0. **Repair the branch ref** (§0.1) — **DONE**. Root cause identified, 0-byte objects quarantined,
   7 blobs regenerated under identical hashes, `git fsck` clean. Residual: add
   `find .git/objects -type f -empty` to `make check`, because nothing else detects this class and
   it silently produces false "no changes" verdicts.
1. **Close the missing capability** (§6.2): a path-existence audit surface reading the three
   authority files plus path literals from Rust and JSON. It retires seven findings mechanically
   and is the only step that prevents recurrence of the whole class.
2. **Generate the boundary tests** from the L0-L4 taxonomy via `cargo metadata` (§5.4, §7b).
   Retires the 11 cosmetic guards and makes §3.1 and §5.5 detectable.
3. **Delete the confirmed dead trees**: `bounty/`, `recruiter/`, `references/`, `verify/`,
   `hermes/` (empty), `.cortex/` minus the one `zones.rs` docstring fix, `archive/legacy-canon/`.
   Roughly 1.2 MB and 100+ tracked files of zero-authority mass.
4. **Reconcile the three `Supervisor` collisions** (§4.3): rename repo-local `evidence/`, and
   remove the empty repo-local `.cynic/`. Then fix the four false claims that resulted
   (`vault/README.md`, `.hermes/` citations, `session_observer.rs:32` default, `cynic-intel-soc`
   docstring).
5. **Rule on the four duplications** (§3): delete `cynic-intel-escore` (zero consumers), decide
   `cynic-intel-futarchy`, merge or rename `cynic-vcs`/`cynic-vcs-forgejo`, fold
   `cynic-domain-askesis` into `cynic-askesis`, and give `specs/` a deprecation record. These
   require operator sign-off, not an auditor's ruling.

### Explicitly out of scope for this report

- LOC reduction against the 25k ceiling. `CANONICAL_CRATE_MAP.md` owns the extraction plan; this
  report only records that 6 crates violate it and that the ratchet is bypassed.
- The 390 orphaned production `.rs` files inside crates. `infra/WORKSPACE_ORPHAN_DECISIONS.md` owns
  that, with a per-crate disposition matrix.
- CLI command reachability (91 of 165 advertised commands unreachable). `infra/CLI_SURFACE_AUDIT.md`
  owns that.
- The pinned rustc ICE paths. `infra/RUST_COMPILER_ALPHAS.md` owns the build policy.

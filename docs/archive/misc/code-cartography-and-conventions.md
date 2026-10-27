# Code Cartography & Engineering Conventions

Status: `diagnosis + proposal` — §1–§2 measured, §3 proposed, awaiting operator validation.
Schema: `cynic.code_cartography.v1`
Method: every CYNIC number below was produced by a command that was actually run.
Upstream claims are sourced and labelled as *observed upstream*, never as measured here.

---

## 1. CYNIC measured — the current situation

### 1.1 Crate graph (declared SSOT: `infra/crate-topology.json`)

| Metric | Value |
|---|---|
| Crates | **49** |
| Edges | **155** |
| Cycles | **0** ✅ |

The crate-level graph is **acyclic and correctly layered**. The defect is not the crate DAG.

**Fan-out — who pulls in the most:**

| Crate | Layer | Depends on |
|---|---|---|
| `cynic-cli` | L4 | **26** |
| `cynic-host` | L1 | **20** |
| `cynic-node` | L3 | **17** |
| `cynic-kernel` | L3 | **14** |

`cynic-cli` is a **projection** (L4) that depends on 26 crates including `cynic-node` (L3) and `cynic-host` (L1). A projection consuming implementations rather than ports.

**Fan-in — who everything depends on:**

| Crate | Dependents |
|---|---|
| `cynic-protocol` | **36** |
| `cynic-secrets` | 10 |
| `cynic-finance` | 9 |
| `cynic-intel` | 8 |

`cynic-protocol` is the hub: 36 of 49 crates depend on it. That is the intended shape (constitutional law), but it also means **any change to it invalidates the whole workspace** — the reason a full build takes minutes.

**Orphans — 15 crates with fan-in 0** (nothing depends on them):

```
cynic-agent-bus-coordinator  cynic-agent-bus-server  cynic-channel
cynic-cli      cynic-domain-askesis     cynic-host-probes
cynic-identity cynic-inference-runtime  cynic-intel-escore
cynic-kernel   cynic-market             cynic-mcp
cynic-observer cynic-pool               cynic-zeroclaw-solana
```

Justified ones (binary or leaf): `cynic-cli`, `cynic-kernel`, `cynic-mcp`. The other 12 are either binaries (`cynic-pool`) or **declared debt** (`pending_disposition` in the topology already names 8 of them).

### 1.2 Module graph — where the spaghetti actually lives

Crate-level is clean. **Module-level is not.**

| Crate | Modules | Inter-module edges | **2-module cycles** |
|---|---|---|---|
| `cynic-host` | 427 | 285 | **14** |
| `cynic-node` | 314 | 162 | **10** |
| `cynic-cli` | 141 | 147 | **1** |
| `cynic-protocol` | 138 | 134 | **2** |

**27 module-level cycles** across the four largest crates. Examples in `cynic-host`:

```
conformance            <-> vertical_conformance
process_observation    <-> runtime_layout
hardware_conformance   <-> ports
bridge                 <-> execution
domain                 <-> ports
```

In `cynic-node`:

```
observe        <-> usecases
domain         <-> ports
ports          <-> websocket
usecases       <-> watchdog
network_mesh_plan <-> ports
```

These are the spaghetti. A `domain` ↔ `ports` cycle means the domain and its own port declarations are mutually entangled — the hexagon is not a hexagon.

### 1.3 Public surface — no encapsulation

| Crate | `pub` items |
|---|---|
| `cynic-host` | **1 537** |
| `cynic-protocol` | **1 003** |
| `cynic-network` | 99 |
| `cynic-core` | 11 |

`cynic-host` exposes 1 537 items. **A crate with 1 537 public items has no boundary** — `pub` is being used as the default rather than as a commitment.

`cynic-protocol` at 1 003 is the same defect on the crate that is supposed to be law: a constitution with 1 003 articles is not a constitution.

### 1.4 Artefacts — 12 built, 5 declared, 3 installed

| Set | Count | Members |
|---|---|---|
| Compiler produces | **12** | see below |
| `registry.lifecycle.kernel_release.runtime_artifacts` declares | **5** | `cynic`, `cynic-kernel`, `cynic-lxc-exec-bridge`, `cynic-mcp`, `cynic-node` |
| Actually installed | **3** | `cynic`, `cynic-node`, `cynic-bin` (symlink) |

**Three sets, no comparator.** Plus:

- **`autobins` is declared nowhere** → autodiscovery is active → **any file dropped in `src/bin/` silently becomes a deployment artefact**. This is how `cynic-host` produces 3 executables nobody asked for (`cynic-topology`, `cynic-host-exec`, `cynic-audit`) with no declaration anywhere.
- **No duplicate binaries** (checked: `enroll_grant.rs` and `demo.rs` are each named by an explicit `[[bin]]`, so the declared name differs from the file stem — not a duplicate).
- **Three dead binaries** (0 external consumers): `cynic-network-actuator`, `cynic-pool`, `zeroclaw-solana-demo`.
- **The 3 `cynic-host` binaries are undeclared**, which is the real artefact defect: they exist only because autodiscovery is on.

### 1.5 Toolchain posture — what already exists and is under-used

| Practice | State |
|---|---|
| `edition` | 2024 ✅ |
| `workspace.dependencies` | present ✅ |
| `publish = false` | **48/49** ✅ |
| `workspace.lints.rust` | `unsafe_code = forbid`, `dead_code = deny`, `unused_imports = deny` ✅ |
| `unreachable_pub` | **`warn`** — should be `deny` |
| `rust-version` | **absent** |
| `[features]` | 4 crates, all **empty** (`default = []`, `cli = []`) |
| `xtask` | **absent** — build tasks live in `Makefile` + shell |
| `thiserror` | 30 crates ✅ |
| `anyhow` in libraries | **4** — error type is not a contract |
| `cargo-machete` | not installed — dead deps undetected |

The workspace is **better configured than most**. The lints are real. The gap is enforcement and a few sharp edges.

---

## 2. What large Rust projects do — observed upstream

### 2.1 rust-analyzer (~50 crates) — the reference for discipline

Source: `rust-lang/rust-analyzer` `docs/book/src/contributing/architecture.md`.

| Practice | Detail | Transposable to CYNIC |
|---|---|---|
| **Three directories** | `lib/` (published, semver), `crates/` (internal, `version 0.0.0`), `xtask/` (build) | CYNIC has `publish=false` everywhere → equivalent to `crates/`; missing the `xtask/` half |
| **Numbered layers** | Layer 0 Foundation → Layer 3 Semantic Analysis, each depends only on layers below | CYNIC has this (`crate-topology.json` L0–L4) ✅ |
| **"Architecture Invariant" per crate** | Every crate documents **what is deliberately absent**: *"base-db doesn't know about file system and file paths"*, *"syntax knows nothing about salsa or LSP"* | CYNIC has `NEGATIVE BOUNDARY` prose in some modules, not systematic |
| **"API Boundary" marked explicitly** | `syntax`, `hir`, `ide` are declared boundaries; *"hir-* crates are not, and will never be, an api boundary"* | CYNIC marks none |
| **Only the binary knows the protocol** | *"rust-analyzer is the only crate that knows about LSP and JSON serialization"*; `ide` types are **not serializable by design** | **This is the composition-root law CYNIC violates** |
| **Serialization is a commitment** | *"serializable types impose significant backwards compatibility constraints... types in ide, base-db and below are not serializable by design"* | CYNIC derives `Serialize` broadly |
| **Errors: `(T, Vec<Error>)` not `Result`** | Core never fails; only I/O parts can | CYNIC: `anyhow` in 4 libs |
| **Tests data-driven, three boundaries** | Test at `hir` (inner), `ide` (middle), LSP (outer); avoid testing the API | CYNIC tests are ad-hoc |
| **xtask as build system** | Codegen, release, install — in Rust, not shell; *"we avoid bootstrapping"* | CYNIC has Makefile + shell |

### 2.2 Agave / Solana (~120 crates) — the reference for artefacts

Source: `anza-xyz/agave` root `Cargo.toml`.

| Practice | Detail | Transposable to CYNIC |
|---|---|---|
| **`[workspace.metadata.agave-build-lists]`** | Binaries grouped **by audience**: `dev`, `end-user`, `val-op`, `dcou`, `deprecated`, `dcou-tainted-packages`. Consumed by `scripts/agave-build-lists.sh`. | **Exactly the artefact contract CYNIC needs.** CYNIC's `runtime_artifacts` is a flat 5-entry list that disagrees with reality |
| **Explicit `deprecated` list** | Deprecated binaries are *named*, not deleted silently | Matches CYNIC's `pending_disposition` pattern ✅ |
| **`exclude` for tooling** | `ci/xtask`, `dev-bins`, `programs/sbf` excluded from the workspace | CYNIC has no exclusion mechanism |
| **`agave-unstable-api` feature on every internal crate** | An explicit **instability marker** per crate | CYNIC's 4 empty `[features]` blocks are the vestige of wanting this |
| **Crate names = responsibility** | `accounts-db`, `banking-stage-ingress-types`, `leader-schedule`, `unified-scheduler-pool` | CYNIC already states this in `pruning-policy.json` ✅ |
| **Strict clippy denies at workspace level** | `arithmetic_side_effects`, `default_trait_access`, `manual_let_else`, `uninlined_format_args`, `used_underscore_binding` — all `deny` | CYNIC denies fewer |

### 2.3 rustc — the reference for invariants

Source: `rustc-dev-guide` conventions chapter.

| Rule | Verbatim |
|---|---|
| **No duplicated truth** | *"Avoid duplicated sources of truth — trying to keep data in sync between two different places is a code smell."* |
| **Types over checks** | *"Use types to enforce invariants — make invalid states unrepresentable, not just checked at construction time."* |
| **Small independently testable steps** | *"Run the relevant tests after every meaningful change, so you know where you first went wrong."* |
| **Refactors isolated** | *"Isolate pure refactorings into their own commit."* |

CYNIC violates the first one four times over: `registry.json` ↔ `verticals/*/intent.json` ↔ `network/topology.json` ↔ `directory-tree.json`.

---

## 3. The conventions CYNIC must adopt

Each convention below has **an executable guard**. A convention without a guard is prose, and prose is what produced the current state.

### Group A — Artefacts (what the compiler produces)

| # | Convention | Guard |
|---|---|---|
| **A1** | `autobins = false` workspace-wide. Every `[[bin]]` is declared explicitly. A file in `src/bin/` never becomes an artefact silently. | `grep -c 'autobins' Cargo.toml` must be 1 and set to `false`; a test asserts every `src/bin/*.rs` is covered by a declared `[[bin]]` |
| **A2** | **One binary per lifecycle boundary, never per capability.** A new capability nests as a subcommand. | `contracts/artifacts.json` lists every artefact with `lifecycle`, `privilege`, `consumers`; a test fails when the compiler produces a binary absent from the contract |
| **A3** | **Artefacts grouped by audience** (adopt Agave's model): `operator` (end-user), `service` (supervised daemons), `privileged` (root bridges), `tooling` (dev-only, not deployed). | same test as A2; `tooling` artefacts are excluded from release manifests |
| **A4** | **The binary is the only place that knows concrete adapters.** Libraries depend on ports. | test: no `lib` crate depends on a provider crate (`*-forgejo`, `*-cloudflare`, `*-porkbun`, `*-solana`) |

### Group B — Modules and visibility

| # | Convention | Guard |
|---|---|---|
| **B1** | **`pub` is a commitment.** A crate's public surface is declared, and growth requires a reason. | `unreachable_pub = "deny"` (currently `warn`) + a per-crate `public_api.json` compared with `cargo public-api` output |
| **B2** | **No module-level cycles.** A `domain` ↔ `ports` cycle means the hexagon is broken. | test walking `crate::` references, failing on any 2-cycle in `domain`/`ports`/`usecases` |
| **B3** | **Every crate states its Architecture Invariant** — what is deliberately absent — and its boundary status. | test: every `Cargo.toml` `description` is non-empty and every `lib.rs` carries a `//! Architecture Invariant:` section |
| **B4** | **`rust-version` declared** (pinned toolchain, matching `rust-toolchain.toml`). | workspace key present; test compares with `rust-toolchain.toml` |

### Group C — Errors and serialization

| # | Convention | Guard |
|---|---|---|
| **C1** | **Libraries use typed errors** (`thiserror`); `anyhow` is binary-only. | test: `anyhow` absent from every `lib` crate's `[dependencies]` |
| **C2** | **Serialization is a commitment.** Types below the projection layer do not derive `Serialize` unless they cross a declared boundary. | test: `Serialize` derives in domain crates must be listed in an allow-list |

### Group D — Configs and documents

| # | Convention | Guard |
|---|---|---|
| **D1** | **One responsibility, one document.** No two documents hold the same desired state. | test: hash every JSON under `contracts/` and `infra/`; fail on byte-identical duplicates |
| **D2** | **A document with zero readers is dead.** Declared or deleted, never left silent. | test: every JSON has ≥1 Rust reader, or is listed in `retired` with a reason |
| **D3** | **Declared authority is reconciled.** For every pair in `authority_order`, the lower document must not contradict the higher. | new `cynic audit authority` — pure comparison, fails on divergence |
| **D4** | **Every comparator runs in the gate.** A rule that exists but is not invoked is a phantom gate. | test: every registered `AuditRule` id appears in at least one `make` target |

### Group E — Toolchain

| # | Convention | Guard |
|---|---|---|
| **E1** | **Build tasks are Rust, not shell.** Adopt `xtask`. | `xtask/` exists; `make` targets delegate to `cargo xtask <verb>` |
| **E2** | **Dead dependencies are detected.** | `cargo-machete` in the gate |
| **E3** | **No kebab-case CLI.** A composed concept descends one level. | test: no clap subcommand name contains `-` |

---

## 4. What CYNIC already has right (do not rebuild)

Being precise about this matters, because the correct action for most of the above is **enforce what exists**, not write new machinery.

| Already correct | Evidence |
|---|---|
| Crate DAG acyclic, layered | 49 crates, 155 edges, 0 cycles |
| `pruning-policy.json` is well designed | one-responsibility rule, `suspect_over_splitting`, naming rules, `enforced_by` map |
| Shrink-only ledgers | `layer_debt`, `pending_disposition` — the right pattern |
| `crate-topology.json` has a **generator** | `cynic-topology --check/--write`, deterministic, refuses to bless an unjustified upward edge |
| `workspace.lints` real | `unsafe_code=forbid`, `dead_code=deny`, `unused_imports=deny` |
| `thiserror` adopted | 30 crates |
| `publish = false` | 48/49 |
| Edition 2024, `workspace.dependencies` | present |
| Per-crate `architecture_boundary.rs` | present on core crates (audit enforces it) |

**The generator `cynic-topology` is the single best artefact in this repo.** It derives what can be derived, preserves authored policy, and **refuses to write** a document ARCH-004 would flag. That is the model every other convention should copy.

---

## 5. The finding that matters most

Everything in §1 and the earlier diagnoses (D1–D7, 19 dead verticals, 6 unread contracts, 110 dead files) is **one defect**:

> **CYNIC has comparators and does not run them.**

| Comparator | Exists | Runs in a gate |
|---|---|---|
| `cynic-topology --check` | ✅ | ❌ |
| ARCH-004 | ✅ | ❌ (not in `audit all`) |
| `crate_topology_coherence` test | ✅ | ❌ (fails on `main`) |
| `layer_debt` ledger | ✅ | ❌ (ARCH-004 never reads it) |
| `directory-tree.json` | ✅ | ❌ (no comparator at all) |
| `pruning-policy.json` | ✅ | partial |

The conventions in §3 are mostly **already written** — in `pruning-policy.json`, in `crate-topology.json`, in `architecture_boundary.rs`. What is missing is that **nothing fails when they are violated**.

---

## 6. Proposed order

| Phase | Action | Why first |
|---|---|---|
| **0** | `autobins = false` + `contracts/artifacts.json` + A2 test | Mechanical, freezes artefact drift immediately, reveals the 12 vs 5 discrepancy as a red gate |
| **1** | Wire the existing comparators into `make check`/`make audit` (D4) | Makes every existing defect visible at once — the honest red |
| **2** | B2 (module cycles) + B1 (`unreachable_pub = deny`) | These are the actual spaghetti: 27 cycles, 1 537 `pub` |
| **3** | D1/D2/D3 (config reconciliation) | Resolves the 4-way truth conflict |
| **4** | A4 + C1/C2 (composition root, typed errors) | The architectural inversion |
| **5** | E1 (`xtask`) + E3 (kebab CLI) | Mechanical, last |

---

## 7. Open decisions

1. **Artefact audiences** — adopt Agave's exact grouping (`operator`/`service`/`privileged`/`tooling`), or CYNIC-specific names?
2. **`contracts/artifacts.json`** — new contract, or extend `registry.lifecycle.kernel_release.runtime_artifacts` (which already exists and is read by `build_attestation.rs`)?
3. **Public-API guard** — `cargo-public-api` (adds a toolchain dependency) or a lighter self-written extractor over `rust_source.rs` (already exists in `cynic-cli`)?
4. **`xtask`** — adopt now, or after the comparators are wired?
5. **Module-cycle budget** — fix all 27 now, or ledger them shrink-only (matching `layer_debt`) and fix incrementally?

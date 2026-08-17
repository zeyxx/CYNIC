# MCP → Kernel Workload Receipt Recalibration Plan

> **For Hermes:** execute bottom-up, one proof boundary at a time; never activate a scheduler or invent an admission grant while this plan is incomplete.

**Goal:** Turn the already-compiling MCP/protocol receipt primitives into one bounded, persistence-backed proof slice: authenticated kernel session → delegation/workload → fenced claim → typed execution receipt → kernel read-back.

**Architecture:** The protocol receipt remains a typed join record, not a second log. The kernel owns workload persistence and must validate/bind the receipt when accepting a completion. MCP remains a stateless transport projection backed by its internal kernel session client. Node scheduler activation and PoH atomicity remain explicitly outside this first slice.

**Tech Stack:** Rust 2024, `cynic-protocol`, `cynic-kernel`, `cynic-mcp`, existing workload storage port and REST API, serde JSON.

---

**Objective:** Locate the protocol publication definition, validation, storage trait, Surreal adapter, and existing tests that own completion semantics.
## Audit findings (observed 2026-08-16)

L'analyse concrète des sources révèle que la plupart des blocs sont déjà implémentés et reliés. Le défaut est plus ciblé qu'un "absence de receipt".

- `validate_agent_workload_delivery` (delivery.rs) valide déjà une terminalité complète: terminal_record_digest sha256, terminal_record cohérent, completion admissisé via `admit_completion`, et completion_evidence validé.
- `complete_agent_workload_delivery` (delivery.rs) construit la transition Accepted → Terminal en copiant `durable_record` et `completion` depuis la publication.
- Le storage Surreal (`workload_delivery::complete`, surreal.rs:607) applique une `UPDATE ... WHERE record_json = '{observed_json}'` — c'est déjà un fence optimiste par enregistrement entier.
- `completion::reconcile` (node UC) persiste les receipts via `AgentWorkloadCompletionReceiptPort`: `contains(digest)` et `persist(&delivery)` sont implémentés localement (`LocalAgentWorkloadCompletionReceipt`) comme fichier content-addressed + read-back synchrone.
- La route kernel `/agent-workloads/{id}` (GET) et `/agent-workloads/{node}/{workload}/completion` (POST) existent dans `agent_workloads.rs` et sont enregistrées dans le routeur REST.

- **Ce qui manque concrètement :** le lien receipt↔kernel n'est jamais exercé par MCP. MCP n'a aucun outil projetant la receipt `cynic.agent_execution_receipt.v1`. Le storage Surreal persiste la delivery terminal (fichier/Surreal selon l'adapter) mais `completion::reconcile` appelle `receipts.persist(&read_back)` **après** le storage; la receipt n'est jamais écrite dans SurrealDB comme entité indexée (CAS/hash), seulement comme fichier local par le runtime node.
- `AgentExecutionReceipt` (agent_execution_provenance.rs) est exportée par le protocole mais n'est importée nulle part dans le kernel ou le node storage — elle n'est qu'un type mort.
- La validation du kernel REST `completion_handler` ne valide pas une `receipt_digest` explicite dans la publication; elle ne peut pas rejeter une publication dont la receipt est absente ou inconnue.

## Non-goals

- Do not deploy `build_scheduler`, run a real builder workload, or use Cortex as fallback.
- Do not create, fake, or bypass `CYNIC_BUILD_ADMISSION_GRANT`.
- Do not add Solana, ZeroClaw, inference, or laptop integration dependencies.
- Do not implement PoH atomic multi-writer semantics in this slice; retain it as the next kernel persistence milestone after receipt binding is proven.
- Do not commit or push.

## Decision

1. **Receipt authority:** la receipt est une preuve déposée par le runtime node après un completion_storage réussi. Le kernel REST doit exposer `POST /agent-workloads/{node}/{workload}/completion` (déjà présent) **en y exigeant une `receipt_digest` sha256 dans `AgentWorkloadCompletionPublication`**, et persister/lire cette receipt dans le storage kernel comme entité CAS (table `agent_execution_receipt`).
2. **Storage ownership:** étendre le trait kernel `StoragePort` + adapter Surreal avec `persist_agent_execution_receipt` et `agent_execution_receipt`. La validation pure reste dans le protocole.
3. **Attestation boundary:** `validate_agent_workload_completion_publication` rejette une publication sans `receipt_digest` sha256 valide ou dont la receipt persistée ne correspond pas.
4. **Transport projection:** MCP ajoute un outil `cynic_agent_workload_completion_read_back` projetant la delivery terminal + receipt read-back. Le kernel client (stateless HTTP) reste le seul transport.

## Task 1: RED — test de publication sans receipt_digest (protocole)

L'objectif est TDD : faire échouer d'abord, puis implément.

**Fichiers :**
- `crates/cynic-protocol/src/agent_workload/delivery.rs` — type + validateur.
- `crates/cynic-protocol/src/agent_workload/tests.rs` — tests existants.

**Étapes :**
1. Ajouter un test qui construit une `AgentWorkloadCompletionPublication` sans `receipt_digest` (nouveau champ optionnel → requis pour la validation).
2. Lancer `cargo test -p cynic-protocol` → échoue car le champ n'existe pas / validation ne rejette pas l'absence.
3. Ajouter `receipt_digest: Option<String>` à `AgentWorkloadCompletionPublication` et rejeter l'absence/malformation dans le validateur.

## Task 2: GREEN — persistance/lecture receipt dans le kernel storage (TDD)

**Fichiers :**
- `crates/cynic-kernel/src/domain/storage/mod.rs` — trait `StoragePort`.
- `crates/cynic-kernel/src/storage/surreal.rs` + `workload_delivery.rs` — nouveau module `receipt.rs`.
- `crates/cynic-kernel/src/storage/reconnectable.rs` + `null.rs` — adapter manquants.

**Étapes :**
1. Écrire un test Surreal/local qui persiste une receipt et la relit par digest → RED (trait manquant).
2. Ajouter `persist_agent_execution_receipt` + `agent_execution_receipt` au trait `StoragePort` et `ProofOfHistoryStorage` (ou sous-trait dédié).
3. Implément dans Surreal (`INSERT ONLY` CAS sur `record_id = hash`) et read-back.
4. Branchon le `complete_agent_workload_delivery` Surreal pour appeler la persistance receipt **avant** le read-back final, et rejeter si la receipt persistée ne correspond pas.
5. Lire la receipt dans `complete_agent_workload_delivery` Surreal pour le read-back final.

## Task 3: MCP projection stateless — outil receipt read-back

**Fichiers :**
- `crates/cynic-mcp/src/routers/local/agent_surfaces.rs` — nouvel `#[tool]`.
- `crates/cynic-mcp/src/helpers_split/misc.rs` — helper d'URL.
- `crates/cynic-mcp/src/handler_tests/infrastructure_contract_tests.rs` — test de l'outil.

**Étapes :**
1. Ajouter `cynic_agent_workload_completion_read_back(workload_id) ->` projete `delivery.terminal_record_digest` + `receipt` lu depuis le kernel REST (GET `/agent-workloads/{id}` inclut désormais la receipt).
2. Test du helper de formatage d'URL + test du tool déclenchant un mock kernel HTTP (stub `KernelSessionClient` ou `reqwest` mock).
3. Valider que MCP reste stateless : aucune session locale, seulement projection du kernel.

## Task 4: Validation intégrale restreinte au scope

- `make check` (safe 256M / jobs=1).
- Tests protocol `receipt_digest` + Surreal `persist_agent_execution_receipt`.
- Tests MCP receipt read-back + régression `runtime_contract` + DNS.
- `git diff --check` + `rustfmt --check` sur les fichiers touchés.

## Stop conditions (réaffirmées)
- Un défaut empêche le CAS Surreal sur `agent_execution_receipt` → rester en `degraded_local_observation`, ne pas monter l'abstraction.
- Une modification exige un scheduler réel, un grant, ou Solana → rejeter.

## Task 2: Define receipt-to-completion binding in the protocol (TDD)

**Objective:** Make receipt binding an explicit, serializable part of the workload completion contract.

**Files:**
- Modify: exact protocol workload completion type located in Task 1
- Modify: `crates/cynic-protocol/src/agent_execution_provenance.rs` only if a reusable binding validator belongs there
- Test: adjacent protocol workload/provenance test module

**Steps:**
1. Add a failing test for a terminal completed publication missing the receipt or referencing a receipt with mismatched workload ID.
2. Add the smallest typed binding: receipt payload or canonical receipt digest/reference, chosen by Task 1’s ownership analysis.
3. Validate schema, execution receipt validity, workload ID, node ID, lease ID, fencing token, and terminal disposition rules without reaching I/O.
4. Add tests for: valid fenced completion; stale fence; wrong workload; wrong node; degraded/recovered receipt with explicit unknowns; missing evidence digest.
5. Run the targeted protocol tests with the repo’s low-memory validation settings.

**Acceptance:** A completed workload cannot be represented as valid unless the receipt joins exactly to the accepted claim; degraded/recovered are structured non-promotable outcomes.

## Task 3: Persist and read back the receipt in the kernel storage owner (TDD)

**Objective:** Store the receipt together with its authoritative workload completion and return the read-back object, not a success claim.

**Files:**
- Modify: existing kernel storage trait defining completion
- Modify: its Surreal adapter and types discovered in Task 1
- Modify: `crates/cynic-kernel/src/api/rest/agent_workloads.rs` only to project the use-case result
- Test: existing kernel storage/API completion tests

**Steps:**
1. Add a failing adapter or use-case test that completes a claimed workload and asserts the stored receipt digest/content is returned on a subsequent read.
2. Extend the storage contract with an atomic completion input/result containing the typed receipt binding.
3. Within the existing persistence transaction/conditional update, reject receipt linkage that differs from delivery node/workload/lease/fence.
4. Read back the persisted delivery/completion record and verify receipt identity/digest before returning `terminal`.
5. Map unavailable storage to an explicit unavailable/blocked error; never synthesize `completed` from an unpersisted receipt.

**Acceptance:** The kernel is the first durable owner that can prove a receipt was bound to the completion it reports.

## Task 4: Add MCP projection only after kernel read-back exists

**Objective:** Let MCP submit/read the bounded receipt-backed path while retaining stateless transport and kernel authority.

**Files:**
- Modify only if required: MCP local agent workload/router module discovered through call-site search
- Modify only if new agent-facing command/tool is exposed: `infra/agent-exposure.json`
- Test: targeted MCP router/handler tests

**Steps:**
1. Use `KernelSessionClient`’s authenticated client to call the kernel’s completion/read endpoint.
2. Project the kernel’s read-back receipt reference, status, and residual unknowns verbatim; do not recalculate promotion or lease policy in MCP.
3. Add a test that a revoked/stale session or fence cannot produce a terminal receipt-backed projection.
4. Add a test that runtime unavailability produces an explicit blocked/degraded projection with retained unknowns.
5. Run targeted MCP tests, then `./scripts/cargo test -p cynic-mcp --lib` under the known-safe environment.

**Acceptance:** MCP transports and renders kernel proof; it cannot manufacture a terminal completion or a receipt.

## Task 5: Reassess PoH and scheduler only after the slice is green

**Objective:** Decide the next vertical based on proof, not roadmap momentum.

**Steps:**
1. Run focused protocol, kernel and MCP tests; run `git diff --check` and `rustfmt --check` on touched files.
2. Confirm a read-back completion contains a matching typed receipt and survives a separate read endpoint call.
3. If the slice is green, design atomic PoH append/read-back as a separate kernel persistence task.
4. Only after PoH read-back and real admission authority are proven, revisit registry-selected `build_runner` activation. Do not schedule a canary before then.

**Acceptance:** The next action is selected from verified residual gaps: PoH atomicity first, scheduler deployment last.

## Validation sequence

```bash
# Focused, after each crate-local change (adapt test paths to the located owners)
CYNIC_BUILD_ADMISSION_VERIFIED=1 CYNIC_LOCAL_VALIDATION=1 \
RUST_MIN_STACK=8589934592 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
CARGO_BUILD_JOBS=1 RUSTUP_TOOLCHAIN=1.96.0 \
./scripts/cargo test -p cynic-protocol <target>

# Kernel slice after receipt persistence is implemented
CYNIC_BUILD_ADMISSION_VERIFIED=1 CYNIC_LOCAL_VALIDATION=1 \
RUST_MIN_STACK=8589934592 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
CARGO_BUILD_JOBS=1 RUSTUP_TOOLCHAIN=1.96.0 \
./scripts/cargo test -p cynic-kernel <target>

# MCP regression after its projection changes
CYNIC_BUILD_ADMISSION_VERIFIED=1 CYNIC_LOCAL_VALIDATION=1 \
RUST_MIN_STACK=8589934592 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
CARGO_BUILD_JOBS=1 RUSTUP_TOOLCHAIN=1.96.0 \
./scripts/cargo test -p cynic-mcp --lib

git diff --check
```

## Stop conditions

- A required contract cannot be bound without duplicating workload state: stop and redesign the protocol shape.
- Storage cannot make the completion/receipt link conditional and durable: return explicit blocked/degraded state; do not proceed to MCP projection.
- An implementation requires a synthetic grant, fallback runner, concrete Solana adapter, or host mutation: stop; it violates this slice’s authority boundary.

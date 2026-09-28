# CYNIC — CANONICAL CRATE MAP v3

> **Supersedes** : CANONICAL_CRATE_MAP v2 (2026-09-23), CYNIC_EXTRACTION_GLOBALE.md, CYNIC_CONSTELLATIONS_HEXAGONALES.md
> **Authority** : canonical taxonomy source for workspace crates. Referenced by `AGENTS.md` (protocol), `README.md`, and `contracts/foundation.json` (hard boundary `strict_dag_and_single_responsibility`).
> **Pattern** : `cynic-<family>-<role>` — no exceptions.
> **External validation** : Agave/Solana workspace (117 crates), Bevy (60 crates), Substrate (500+ crates).
> **God-object ceiling** : 25,000 LOC — `contracts/foundation.json` hard boundary and `infra/ARCHITECTURE_CONSTELLATIONS.md` rule 5. Supersedes the 40k figure recorded in v2.
> **Measured** : 2026-09-28 from `cargo metadata` (47 workspace crates) and `src/` LOC census. Total: 388,393 LOC.

---

## 1. GUIDING PRINCIPLES

```
cynic-<family>              → legitimate singleton (cross-cutting or isolated)
cynic-<family>-<role>      → member of a constellation

R1. Every crate with at least one sibling in its family MUST follow the pattern.
R2. The role names WHAT THE CRATE DOES, not its technical layer.
R3. No layer prefixes (-domain, -ports, -adapter, -application).
    Conventional role patterns (Agave):
      -types     → pure types only (e.g. cynic-rpc-types)
      -callback  → traits/callbacks only
      -interface → protocol definition only
R4. Pure domain first — types + ports before adapters.
R5. Strict downward DAG. No cycles. kernel does not depend on node.
R6. no_std + alloc wherever deps allow.
    The no_std/std boundary IS the domain/adapter boundary.
    Enforcement: the compiler (not a test, not a convention).
R7. One crate per cohesive functional domain — no micro-crate cascades.
    Sub-structure (types, parser, scrubber, slots...) lives in INTERNAL
    MODULES of the domain crate, not in separate crates. SOTA model:
    solana-runtime, tokio+feature-flags — NOT 12 sub-crates per module.
    Practical band: ~5-15k LOC per crate; 25k = god-object ceiling.
R8. Vertical extraction only. Extraction proceeds bottom-up by tier
    (see section 5), never horizontally by technical layer. Special
    decomposition proposals in `infra/ARCHITECTURE_CONSTELLATIONS.md`
    (e.g. Slice 5 cynic-trace-* / cynic-eval-* splits) are bounded
    operator-veto items under R7 — they never create micro-crates by
    default. Confirmed by operator decision 2026-09-28.
```

---

## 2. CURRENT STATE — MEASURED LOC (2026-09-28)

```
cynic-host           89 851    GOD OBJECT (extraction session 3 in progress)
cynic-node           58 327    GOD OBJECT (extraction session 4 pending)
cynic-kernel         40 434    GOD OBJECT (grew from 6.3K — organ/senses/probe/introspection modules)
cynic-protocol       34 779    GOD OBJECT (extraction sessions 1-2 partial)
cynic-cli            34 735    GOD OBJECT (extraction session 6 pending)
cynic-inference      25 317    GOD OBJECT (borderline; runtime-authority status: extracting)
cynic-mcp            13 588    ok (5-15k band; projection singleton)
cynic-intel          10 844    ok (absorbs cynic-market per v3 verdict)
cynic-build           8 737    ok
cynic-observer        8 558    ok (rename target: cynic-solana-observer)
cynic-vcs             6 343    ok
cynic-workload        6 195    cross-cutting
cynic-node-protocol   5 658    ok (created from protocol, 2026-09-26)
cynic-eval            4 347    ok (consolidated per R7, created wave B)
cynic-storage         4 024    cross-cutting
cynic-network         3 464    cross-cutting
cynic-pkg             3 213    bad name (→ cynic-host-pkg)
cynic-lifecycle      6 072    ok (absorbed activation/evidence/diagnostics/workflow/forge-task/runtime-paths modules from cynic-host, 2026-09-28)
cynic-domain-ops      2 892    constellation ✓
cynic-askesis         2 373    bad name (→ cynic-kernel-askesis)
cynic-consensus       2 332    bad name (→ cynic-node-consensus)
cynic-vcs-forgejo     1 757    constellation ✓
cynic-forensics       1 221    bad name (→ cynic-host-forensics)
cynic-lxc             1 175    bad name (→ cynic-host-lxc)
cynic-host-probes     1 129    ✓ pattern OK
cynic-market          1 059    ORPHAN (merge into cynic-intel-futarchy)
cynic-sandbox         1 013    bad name (→ cynic-host-sandbox)
cynic-intel-escore      944    constellation ✓
cynic-secrets           783    cross-cutting ✓
cynic-pool              744    ORPHAN (rename: cynic-agent-pool)
cynic-agent-bus         729    constellation ✓
cynic-intel-soc         597    constellation ✓
cynic-release         1 020    ok (absorbed release_gate_adapter + release_relay_adapter + secret_lint from cynic-host, 2026-09-28)
cynic-domain-cloudflare 555    constellation ✓
cynic-solana-pay        471    created 2026-09-24 ✓ (payment vocabulary, not anchor)
cynic-agent-bus-coordinator 457 constellation ✓
cynic-domain-porkbun    431    constellation ✓
cynic-zeroclaw-solana   402    bad name (→ cynic-solana-zeroclaw)
cynic-discovery         352    bad name (→ cynic-node-discovery)
cynic-types             321    pure leaf — DAG root (new in v3; zero-dep primitives)
cynic-channel           283    ORPHAN (rename: cynic-agent-channel)
cynic-discovery-host    220    bad name (→ cynic-host-discovery)
cynic-intel-futarchy    213    constellation ✓
cynic-identity          185    pure leaf ✓ (ed25519, blake3, NodeId)
cynic-agent-bus-server   91    constellation ✓ (projection REST)
cynic-domain-askesis     67    constellation ✓
cynic-inference-runtime   6    composition shell (new in v3; stub over cynic-inference)
```

Ceiling references: GOD OBJECT > 25,000 LOC; MONOLITH band 15-25k; ok band 5-15k; small < 5k.
Six crates exceed the 25k ceiling: host, node, kernel, protocol, cli, inference.

---

## 3. TARGET TAXONOMY — BY FAMILY

### FAMILY: agent
*Multi-agent coordination, economy, transport.*
```
cynic-agent-bus              ✓ exists — types, ports, ledger, claim
cynic-agent-bus-server       ✓ exists — REST projection
cynic-agent-bus-coordinator  ✓ exists — fleet orchestrator
cynic-agent-channel          ← rename cynic-channel (bilateral HTLC economy)
cynic-agent-pool             ← rename cynic-pool (PoUW, PPLNS, rewards)
cynic-agent-coordination     → CREATE — ONE consolidated crate (R7):
                                         coordination_lease + registry_claim_audit +
                                         agent_session/ + agent_coordination
                                         (from protocol). Internal modules.
```

### FAMILY: eval
*Scoring, judgment, verdict, crystals.*
```
cynic-eval                   → CREATE — ONE consolidated crate (R7):
                                         evidence + eval/ + judgment + kyon +
                                         emergence_gallery (from protocol, ~2.8k LOC).
                                         Internal modules: evidence.rs, dog.rs,
                                         judgment.rs, kyon/, gallery.rs.
```

### FAMILY: kernel
*Sovereign orchestrator, coordination bus.*
```
cynic-kernel                 ✓ exists — residue ~5K after corrections (plan);
                                         measured 40.4K — organ/senses/probe growth
                                         must be reversed (session 5)
cynic-kernel-askesis         ← rename cynic-askesis
```

### FAMILY: host
*OS substrate, hypervisor, system adapters.*
```
cynic-host                   ✓ exists — host orchestrator residue
cynic-host-probes            ✓ exists — hardware probes
cynic-host-lxc               ← rename cynic-lxc
cynic-host-pkg               ← rename cynic-pkg
cynic-host-sandbox           ← rename cynic-sandbox
cynic-host-lifecycle         ← rename cynic-lifecycle
cynic-host-release           ← rename cynic-release
cynic-host-forensics         ← rename cynic-forensics
cynic-host-discovery         ← rename cynic-discovery-host
cynic-host-substrate         → CREATE — host_substrate/ cpu/disk/memory/hypervisor
                                         (extract from cynic-protocol)
```

### FAMILY: node
*Single node, supervision, autonomy.*
```
cynic-node                   ✓ exists — supervisor residue
cynic-node-consensus         ← rename cynic-consensus
cynic-node-discovery         ← rename cynic-discovery
cynic-node-protocol          ✓ created 2026-09-26 — node_activation, node_enrollment,
                                         node_control, node_identity,
                                         machine_enrollment, pre_os (from protocol)
cynic-node-autonomy          → CREATE — emergence, local_autonomy
                                         (extract from cynic-protocol)
```

### FAMILY: runtime
*Placement, seed, takeover.*
```
cynic-runtime                → CREATE — ONE consolidated crate (R7):
                                         runtime + runtime_placement + runtime_seed +
                                         runtime_takeover + runtime_takeover_activation
                                         (from protocol, ~3.2k LOC). Internal modules.
```

### FAMILY: fleet
*Multi-node, heartbeat.*
```
cynic-fleet-heartbeat        → CREATE — fleet_heartbeat, fleet_discovery (from protocol)
```

### FAMILY: federation
*Multi-organization, federated bootstrap.*
```
cynic-federation             → CREATE — ONE consolidated crate (R7):
                                         federation_membership + federation_authz +
                                         federation_envelopes + bootstrap_federation
                                         (from protocol). Internal modules.
```

### FAMILY: inference
*Sovereign LLM, routing, verdict cache.*
```
cynic-inference              ✓ exists — router residue (25.3K, extracting)
cynic-inference-runtime      ✓ new — composition shell over domain contracts (stub, 6 LOC)
cynic-inference-llm          → CREATE — llm_port, fleet_inference/ (from protocol)
```

### FAMILY: vcs
*Source control, Git, Forgejo.*
```
cynic-vcs                    ✓ exists — absorbs git_topology, git_hygiene,
                                         source_control, source_worktree* as
                                         INTERNAL MODULES (R7, wave B)
cynic-vcs-forgejo            ✓ created — vcs/forgejo, forgejo_token (adapters + transport)
```

### FAMILY: storage
*Persistence, ports.*
```
cynic-storage                ✓ exists — ports residue + absorbs storage/ from protocol
```

### FAMILY: network
*Mesh, transport.*
```
cynic-network                ✓ exists — absorbs network.rs, transport_* from protocol
```

### FAMILY: build
*CI/CD, test harness.*
```
cynic-build                  ✓ exists — absorbed harness.rs + harness_authority
                                         (2026-09-24, wave A)
```

### FAMILY: intel
*Intelligence, scoring, markets.*
```
cynic-intel                  ✓ exists — absorbs cynic-market
cynic-intel-escore           ✓ exists
cynic-intel-futarchy         ✓ exists — absorbs cynic-market
cynic-intel-soc              ✓ exists
```

### FAMILY: solana
*Solana integration, observer, plugins.*
```
cynic-solana-observer        ← rename cynic-observer (solana_archive = 80%)
cynic-solana-zeroclaw        ← rename cynic-zeroclaw-solana
cynic-solana-anchor          → DEFERRED — anchor_client.rs purged 2026-09-24
                                         (251 LOC, 10/10 symbols zero external
                                         consumers). Recreate only when an
                                         on-chain branch is actually wired.
cynic-solana-pay             ✓ CREATED 2026-09-24 (ex-solana/ from protocol:
                                         constants + ops — payment/SOP
                                         vocabulary, not anchor). AllowanceGrant
                                         stays in protocol residue (shared
                                         vocabulary via agentic_tool_call);
                                         payment behavior lives here as
                                         AllowanceGrantPaymentExt.
```

### FAMILY: domain
*DNS operations, providers.*
```
cynic-domain-ops             ✓ exists
cynic-domain-cloudflare      ✓ exists
cynic-domain-porkbun         ✓ exists
cynic-domain-askesis         ✓ exists
```

### CROSS-CUTTING SINGLETONS
*Legitimate without a family. No split planned.*
```
cynic-types                  ✓ pure leaf — AgentId, Hash32, Budget, Scope,
                                         TaskId, Timestamp (zero-dep DAG root)
cynic-protocol               → residue of atomic types ~3K (after all extractions)
cynic-identity               ✓ pure leaf (ed25519, blake3, NodeId)
cynic-secrets                ✓ cross-cutting (6 consumers, no clear family)
cynic-workload               ✓ cross-cutting (6 consumers)
cynic-mcp                    ✓ agent projection, singleton
cynic-cli                    → thin residue ~3K (after extractions)
cynic-pool                   → cynic-agent-pool (see agent family)
```

---

## 4. ACTION VERDICTS

### RENAME (Cargo.toml + all consumers)
```
cynic-askesis          → cynic-kernel-askesis
cynic-consensus        → cynic-node-consensus
cynic-discovery        → cynic-node-discovery
cynic-discovery-host   → cynic-host-discovery
cynic-forensics        → cynic-host-forensics
cynic-lifecycle        → cynic-host-lifecycle
cynic-lxc              → cynic-host-lxc
cynic-pkg              → cynic-host-pkg
cynic-release          → cynic-host-release
cynic-sandbox          → cynic-host-sandbox
cynic-channel          → cynic-agent-channel
cynic-pool             → cynic-agent-pool
cynic-observer         → cynic-solana-observer
cynic-zeroclaw-solana  → cynic-solana-zeroclaw
```

### MERGE (absorb then delete the source)
```
cynic-market           → into cynic-intel-futarchy (futarchic market = extension)
```

### CREATE — extract from cynic-protocol (see section 5 extraction order)
```
cynic-eval (consolidated: evidence+dog+judgment+kyon+gallery)
cynic-agent-coordination (consolidated: coord+session)
cynic-node-protocol            ✓ done 2026-09-26
cynic-node-autonomy
cynic-host-substrate
cynic-runtime (consolidated: placement+seed+takeover)
cynic-fleet-heartbeat
cynic-federation (consolidated)
cynic-inference-llm
```

### CREATE — extract from cynic-host
```
(depends on the renames above — the cynic-host-* destination crates
already exist; their code is still inside cynic-host/src/)
```

---

## 5. EXTRACTION ORDER — BOTTOM-UP BY TIER

```
RULE: never extract B until all of its dependencies are clean.

TIER 0 — absolute leaves (zero-dep workspace)
  cynic-types  cynic-identity  cynic-protocol  cynic-kernel-askesis (post-rename)

SESSION 1 : Explode cynic-protocol
──────────────────────────────────────────────────────────
  WAVE A — modules without intra-protocol imports (internal leaves)
    vcs/         → cynic-vcs (absorb) + cynic-vcs-forgejo (created; git modules → cynic-vcs internal, R7)
    storage/     → cynic-storage (absorb)
    forensics.rs → cynic-host-forensics (absorb)
    peer/        → cynic-node-consensus (absorb)
    network.rs + transport_* → cynic-network (absorb)
    harness.rs   → cynic-build (absorbed 2026-09-24 with harness_authority.rs;
                    schemas cynic.build.harness_*)
    solana/ + anchor_client.rs → cynic-solana-pay (created 2026-09-24);
                    anchor_client purged (dead code, DEFERRED)
    ✅ WAVE A COMPLETE 2026-09-24 (commit 06c19797
       Proof: cargo check -p 11 crates EXIT=0, 697✓/3 held by other agent)
    Lessons: (1) SecurityEvidenceState stays in residue — shared proof
    vocabulary 40 files/5 crates; (2) AllowanceGrant = cross-family
    vocabulary (agentic_tool_call) → residue, behavior as extension trait
    in the domain crate (orphan rule); (3) coordination_{fault,scaling}_harness
    purged (0 external consumers).

  WAVE B — import A-leaves only
    evidence.rs (→ digest_json)           → cynic-eval (CONSUMED, shared vocab resides in protocol)
    eval/ + judgment + kyon +              → cynic-eval (CREATED, wave B)
    convergence/ + evidence_journal +       evidence_projection +
    judge_patch + emergence_gallery        emergence_gallery + agent_execution_provenance
    (= R7 consolidation, NOT cynic-eval-<X> split — user vetoes 2026-09-25)
    convergence/ (→ evidence)             → cynic-node-consensus (absorb)
    node_identity/enrollment/control/activation → cynic-node-protocol (created 2026-09-26)
    runtime_placement/seed/takeover        → cynic-runtime-* (to create)
    local_autonomy + emergence             → cynic-node-autonomy (to create)
    host_substrate/ (cpu/disk/memory)      → cynic-host-substrate (to create)
    ⚠ WAVE B PARTIAL — cynic-eval ✓, cynic-node-protocol ✓, rest pending

  WAVE C — import B
    (eval/judgment/kyon already extracted into cynic-eval wave B)
    agent_session/ + agent_coordination   → cynic-agent-coord + cynic-agent-session
    fleet_heartbeat/discovery             → cynic-fleet-heartbeat
    llm_port + fleet_inference/           → cynic-inference-llm

  WAVE D — import C (plus coupled)
    federation_* + bootstrap_federation   → cynic-federation-* (to create)
    organic_opportunity + framework       → cynic-cli (absorb, projections)

  PROTOCOL RESIDUE : ~3K LOC
    error_catalog, digest_json, sanitize, integrity, residual_unknown,
    phi, secret_reference, freshness, sbom, temporal, convergence types

SESSION 2 : Renames (Cargo.toml workspace + consumers)
──────────────────────────────────────────────────────────
  14 renames listed in section 4. Mechanical, no code movement.
  Validate: cargo check --workspace after each rename.

SESSION 3 : Explode cynic-host (92.9K → ~10 crates) — IN PROGRESS: release adapters extracted to cynic-release (2026-09-28)
──────────────────────────────────────────────────────────
  After clean protocol. Destination crates exist (post-rename).
  Move host/src/ modules to their respective cynic-host-* crates.

SESSION 4 : Explode cynic-node (58K → ~4 crates)
──────────────────────────────────────────────────────────
  After clean host.
  cynic-node-protocol, cynic-node-autonomy, cynic-node-consensus absorb.

SESSION 5 : Fix kernel inversions
──────────────────────────────────────────────────────────
  kernel → node  : DELETE (architectural inversion)
  kernel → host  : REPLACE with HostPort traits in cynic-protocol residue
  Kernel residue : ~5K LOC, pure orchestrator.
  ⚠ Measured reality (2026-09-28): kernel at 40.4K — organ/senses/probe/
  introspection/pipeline modules must be extracted or purged before the
  ~5K residue target is reachable.

SESSION 6 : Explode cynic-inference + cynic-cli
──────────────────────────────────────────────────────────
  Last — these crates depend on everything else.
```

---

## 6. TARGET DAG (simplified)

```
Leaves (zero-dep workspace) :
  cynic-types  cynic-identity  cynic-protocol  cynic-kernel-askesis

Tier 1 (→ protocol/leaves only) :
  cynic-workload  cynic-secrets  cynic-network  cynic-storage
  cynic-vcs  cynic-host-forensics  cynic-host-lifecycle  cynic-host-release
  cynic-host-lxc  cynic-host-pkg  cynic-host-sandbox  cynic-agent-bus
  cynic-node-consensus  cynic-eval  cynic-node-protocol
  cynic-host-substrate  cynic-runtime-*  cynic-node-autonomy
  cynic-fleet-heartbeat  cynic-agent-channel  cynic-agent-pool

Tier 2 (→ T1) :
  cynic-build  cynic-inference  cynic-intel-soc
  cynic-eval  cynic-agent-coord
  cynic-agent-coordination  cynic-vcs-forgejo

Tier 3 (→ T2) :
  cynic-intel  cynic-host-lifecycle  cynic-domain-ops

Tier 3b (→ intel) :
  cynic-intel-escore  cynic-intel-futarchy  cynic-intel-soc
  cynic-solana-observer  cynic-solana-zeroclaw  cynic-solana-anchor

Tier 4 (→ T1-T3) :
  cynic-host (residue)  cynic-federation-*

Tier 5 (→ T4) :
  cynic-node (residue)
  cynic-kernel (residue, no dep on node post-correction)

Tier 6 (→ everything) :
  cynic-cli (residue)  cynic-mcp
```

---

## 7b. no_std — STRATIFICATION BY DESTINATION

```
TIER no_std + alloc  (on-chain Solana, sandboxes, WASM)
  Compatible deps : ed25519-dalek ✓  blake3 ✓  serde ✓  sha2 ✓  hex ✓  thiserror v2 ✓

  TODAY (zero logic change) :
    cynic-identity        → #![no_std] ready immediately
    cynic-types           → #![no_std] ready (pure primitives)
    cynic-agent-channel   → #![no_std] ready (sha2 + serde)
    cynic-agent-pool      → #![no_std] ready (serde + sha2)

  AT CREATION (session 1 extraction) :
    cynic-protocol residue
    cynic-eval            (already #![no_std] ready — eval/dog/judgment/kyon)
    cynic-agent-types     (pure types extracted from cynic-agent-bus)

  TO VERIFY (may hide std deps) :
    cynic-workload        (serde_json requires alloc → ok)
    cynic-node-protocol   (async_trait → blocks no_std, to isolate)
    cynic-runtime-*       (verify module by module)

TIER std + feature-gated  (pure off-chain, controlled access)
  Agave pattern : feature "cynic-internal" for test-private items
  #[cfg_attr(feature = "cynic-internal", pub)]
  Crates : cynic-agent-coord, cynic-agent-session, cynic-node-protocol

TIER full std  (adapters, applications, I/O)
  cynic-host-*  cynic-vcs-git  cynic-vcs-forgejo  cynic-kernel
  cynic-inference  cynic-storage  cynic-network  cynic-mcp  cynic-cli
  cynic-node (residue)
```

```toml
# Cargo.toml workspace — add after session 1 extraction
[[bans.deny]]
name = "cynic-protocol"
# Forbid importing the residual god object directly from std crates.
# Force imports through the extracted crates.
wrappers = ["cynic-protocol"]  # only the residue allowed internally
```

## 7. ENFORCEMENT RULES

```
E1. Naming pattern
    Every new crate MUST follow cynic-<family>-<role>.
    Violation → PR blocked.

E2. Acyclic DAG
    Any upward edge (kernel→node, host→kernel) → cargo deny.
    Every crate has tests/architecture_boundary.rs.

E3. LOC ratchet (25k ceiling — foundation.json hard boundary)
    Any crate > 25k LOC → WARN in cynic dev crate-audit.
    2026-09-28 measurement: six crates breach (host 93K, node 58K,
    kernel 40K, protocol 35K, cli 35K, inference 25.3K).
    No new code in monoliths under extraction.

E4. Orphan
    Any crate without a workspace consumer → WARN.
    Exception : binaries (main.rs), final projection crates (cli, mcp).

E5. Misplaced module
    Any module whose ubiquitous language belongs to another family
    → open issue, extraction planned in this doc.

E6. Zero global re-export
    lib.rs does not do pub use other_crate::*.
    Imports are explicit.
```

---

## 8. AMBIGUOUS MODULES — DECISIONS TAKEN

| Module (in protocol) | Belongs to | Reason |
|----------------------|------------|--------|
| `harness.rs` | cynic-build | `cynic.build.*` schemas, CI test scenarios |
| `organic_opportunity` | cynic-cli | read-only, "no I/O", operator projection |
| `framework.rs` | cynic-cli | machine self-description, output artifact |
| `emergence.rs` | cynic-node-autonomy | coordination without kernel, NODE domain |
| `local_autonomy.rs` | cynic-node-autonomy | coupled to emergence (mutual re-export) |
| `emergence_gallery.rs` | cynic-eval | imports EvidenceEnvelope + CynicErrorCodeCode |
| `sensory.rs` | cynic-host-probes | SensoryPort trait, snapshot captures |

---

*v3 — 2026-09-28. English-only invariant restored; 47-crate census; 25k god-object*
*ceiling aligned with foundation.json; R8 vertical-extraction policy; kernel growth*
*(6.3K → 40.4K) recorded as a god-object breach; cynic-types and*
*cynic-inference-runtime added to the inventory; referenced from AGENTS.md.*

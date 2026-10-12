# Inference Universe — Boundaries of `cynic-inference`

**Status:** proposed (awaits operator validation before any code moves).
**Parent verdict:** `docs/design/CANONICAL_CRATE_MAP.md` §3 FAMILY inference —
`cynic-inference` is *router residue* (25.3K LOC, extracting); family owns
*Sovereign LLM, routing, verdict cache*. This document defines the universe
precisely and assigns every current module a verdict.

## 1. Definition

The inference universe is **sovereign local LLM serving**: run models,
serve completions, supply weights, route requests, observe serving quality.

Five sub-domains, all serving-shaped:

| # | Sub-domain | Owns | Examples |
|---|-----------|------|----------|
| S1 | **Run** | local runtime lifecycle: spawn, supervise, pool, schedule slots, plan launches | `local_runtime_service`, `process_supervisor`, `systemd_user`, `model_process_pool`, `slot_*`, `inference_queue`, `runtime_planner`, `engine_selector` |
| S2 | **Serve** | completions over backends: chat, embeddings, summarization, tool loop, traces | `chat_service`, `chat_endpoint`, `http_chat_client`, `openai_compat`, `embedding_runtime`, `summarization_service`, `token_entropy` |
| S3 | **Supply** | weights in, models registered: store, catalog, discovery, download, publish of *model records* | `model_store_service`, `model_catalog`, `model_sync_pipeline`, `hf_discovery`, `model_registry`, `model_retention`, `vault_consolidate*`, `profile_bootstrap` |
| S4 | **Route** | request → backend: endpoint resolution, proxy/fleet routing policy, verdict cache | `inference_gateway`, `chat_endpoint`, `routing_policy`, `routing_calc`, `proxy_routing`, `fleet_routing`, `verdict_cache` |
| S5 | **Observe** | serving-quality evidence: traces, token uncertainty, empirical perf, failure records | `chat_trace_sink`, `token_entropy`, `empirical_store`, `failure_gallery`, `backend_verification`, `math/*` |

Membership test: *would this module still make sense if CYNIC served exactly
one model, locally, with no fleet, no benchmarks, no agents, no external
products?* Yes → inside. No → foreign (named destination below).

## 2. Module census with verdicts

Measured 26,018 LOC. Legend: **KEEP** (inside), **MOVE** (foreign, destination
named), **SHRINK** (keep port, move product-specific impl).

### 2.1 KEEP — serving core (S1/S2/S5)

`domain/{chat,chat_session,chat_trace,embedding,summarization,inference,
backend_config,backend_identity,engine_caps,hardware_profile,local_runtime,
token_entropy,supervision}`, `ports/{chat,chat_session,embedding,inference,
local_runtime,summarization,token_entropy,trace_sink,empirical_store,
hardware_probe,filesystem_capacity,routing_event,model_registry,
model_store,model_publisher}` (port definitions), `adapters/{inference/*,
embedding/*,empirical_store,local/*,discovery}`, `application/{chat_service,
chat_endpoint,embedding_runtime,summarization_service,inference_service,
local_runtime_service,model_process_pool,runtime_planner,engine_selector,
inference_gateway,backend_verification}`, `infra/{backend_*,gguf_meta,
model_layout,model_registry,principal_paths,runtime_plan,slot_*,inference_queue,
domain_task_queue,routing}`, `math/*`, `config_loader`, `model_layout` glue.

### 2.2 KEEP — model supply (S3)

`domain/{model_store,model_layout_report,model_retention,hf_catalog}`,
`application/{model_store_service,model_catalog,model_selection,
model_sync_pipeline,hf_discovery,profile_bootstrap,vault_consolidate*}`,
`adapters/publishers/{mod,inference_toml,null}` (port + vault-native impls).

Supply consumes `cynic-huggingface` for Hub download **only**. Hub upload,
jobs, spaces, webhooks, endpoints are NOT supply — they live in
`cynic-huggingface` and are surfaced by `cynic-cli` directly (see §4).

### 2.3 KEEP — routing (S4)

`application/{fleet_routing,proxy_routing,routing_policy}`, `routing_calc.rs`,
`verdict_cache.rs` (family verdict explicitly keeps *verdict cache* here;
it reads `cynic_eval` verdict types, which is a downward read, not ownership).

### 2.4 MOVE — benchmark and eval harness → `cynic-eval`

`application/agent_harness/*` (9 files), `application/{matrix_live_runner,
matrix_sweep_executor,bench_basket_launcher,multi_agent_service,
failure_gallery}`, `domain/{matrix_sweep/*,debate}`, `ports` used only by
them. Rationale: the eval family owns scoring, judgment, galleries
(canonical map §3). `failure_gallery` joins the eval gallery; debate and
multi-agent evaluation are judgment workloads, not serving.

### 2.5 MOVE — node and fleet operations → `cynic-node`

`application/{node_runtime_ops,peer_attach}`, `domain/node_contribution`
(contribution accounting belongs to the node/market side; inference keeps
only a read projection if needed).

### 2.6 MOVE — finance glue → `cynic-finance`

`finance_adapters.rs` implements the finance `ContextMeter` port. The port
lives in `cynic-finance`; the impl reads inference trace types. Destination
decision (impl moves to finance, or finance reads traces directly) is
phase-4 detail — but the file does not belong to the serving crate.

### 2.7 SHRINK — product-specific publisher impls out, ports stay

`adapters/publishers/open_webui.rs` (registers into external Open WebUI),
`adapters/ods/*` + `ods_discovery.rs` (Osmantic ODS peer merge): the
`ModelPublisherPort` stays; product-specific impls move to host-ecosystem
glue. Peer endpoint *discovery* as data stays iff expressed as inference
backends, not as ODS objects.

### 2.8 DECIDE — open questions (operator)

- **Q1** `hermes_runtime_service` + `domain/hermes_runtime` + hermes
  executors/supervisor: Hermes agent-runtime lifecycle. No hermes crate
  exists in the canonical map. Stay (Hermes is served *by* inference?) or
  new home?
- **Q2** `domain/supervision` (HITL policy): serving-shaped (who may approve
  a tool call) — proposed KEEP, confirm.
- **Q3** `progress.rs`, `verdict_cache.rs` plumbing: KEEP (operator-facing
  serving output), confirm.

## 3. Boundary rules (steady state)

1. `cynic-inference` depends on `cynic-huggingface` for **model download and
   catalog discovery only**. Any new Hub capability (upload, jobs, spaces,
   webhooks, endpoints) is consumed directly from `cynic-huggingface` by its
   own surface — never re-exported through inference.
2. No `cynic_inference::Hf*` re-exports of provider surface. The
   `pub use cynic_huggingface::{...}` block in `lib.rs` shrinks to the
   download/discovery subset actually used by `model_store_service` and
   `hf_discovery`.
3. Benchmarks never spawn from serving paths. The closed benchmark loop
   (`matrix_sweep` → `empirical_store`) splits: the *store* stays as
   serving observation (S5); the *sweep executors* move to eval.
4. Rule 2 of the refactor skill holds: the CLI imports inference only via
   serving ports and pure types.

## 4. Staged plan

- **Phase 0 (approved):** `cynic-cli` → `cynic-huggingface` direct dep, new
  `hub` CLI family (upload, jobs; spaces/webhooks/endpoints when adapters
  land). Prune provider re-exports from `cynic-inference/lib.rs`.
- **Phase 1:** spike-extract eval harness (§2.4) into `cynic-eval` on a
  minimal subset; full move only if the spike converges on the builder.
- **Phase 2:** node ops (§2.5) → `cynic-node`.
- **Phase 3:** finance glue (§2.6) + publisher impls (§2.7).
- **Phase 4:** Q1–Q3 resolutions.

Each phase: one design-validated slice, builder `check` proof, no `main`
push without operator release.

## 5. Phase-1 extraction map — eval perimeter, zero orphans

Scope: §2.4 (agent_harness ×9, matrix_live_runner, matrix_sweep_executor,
bench_basket_launcher, multi_agent_service, failure_gallery,
domain/matrix_sweep ×6, domain/debate) plus their exclusive satellites.
Verified by live grep on this branch; every edge below must be re-routed
or the extraction slice is rejected.

### 5.1 Outgoing edges (moving code needs from the staying side)

| # | Edge | Resolution |
|---|------|-----------|
| O1 | `agent_harness` → `domain::local_runtime::LocalModelProfile` | move the type use to a minimal eval-local view, or move the type with the harness (spike decides; type is serving-shaped, prefer a thin view) |
| O2 | `matrix_*`, `bench_basket` → `domain::matrix_sweep::*` | move together — same slice |
| O3 | `matrix_*`, `bench_basket` → `ports::empirical_store_port` | port MOVES to eval with the perimeter; staying `fs_empirical_store` imports it from eval (direction `inference→eval` already exists via `verdict_cache`) |
| O4 | `bench_basket_launcher` → `ports::local_runtime_port::LocalInferenceRuntimePort` (launch fn takes `&dyn` at line 186) | open seam: eval must not depend on inference. Spike options: (a) move a minimal launch port with the perimeter, implemented by the staying side; (b) keep launcher in inference. NO cycle (`eval→inference` dep) is acceptable |
| O5 | `multi_agent_service` → `adapters::inference::harness_executor::HttpHarnessExecutor` (reqwest-only impl) | impl MOVES with the perimeter (it implements the moving `HarnessExecutor` trait) |
| O6 | `multi_agent_service` → `domain::debate` | move together — same slice |
| O7 | staying `hermes_*` adapters → `agent_harness::{HarnessExecutor, InferenceReply, ScenarioSpec, TaskSpec, canonicalize_hermes_runtime_id}` | staying files import these from `cynic-eval` after the move (allowed direction) |
| O8 | staying `fs_empirical_store` → `domain::matrix_sweep::{HardwareBudget, empirical_store::{...}, types::StreamDynamicsReport}` | shared vocabulary MOVES to eval; staying store imports from eval |
| O9 | staying `inference_gateway` → `domain::matrix_sweep::{HardwareBudget, ConstraintFilter, OptimizationGoal}` + `matrix_sweep_executor::config_to_llama_argv` (line 183) | `config_to_llama_argv` (sweep config → llama argv) is serving-shaped: EXTRACT it into the staying side (near `runtime_plan`) in the same slice; gateway imports the rest from eval |
| O10 | staying `profile_bootstrap::forge_profile_from_benchmark_report` → `agent_harness::BenchmarkReport` | the forge fn CONSUMES an eval type: MOVE the fn (+ `ForgeProfileReport`) to eval; CLI Forge arm calls `cynic_eval` directly (CLI already depends on eval) |
| O11 | `domain::matrix_sweep::empirical_store` → `math::pareto` | `pareto` (multi-objective selection) MOVES with the perimeter; `roofline`/`tensor_geometry` (sizing) STAY; `ucb_sampler` (benchmark scheduling) MOVES |
| O12 | wire compat: `cynic-protocol/src/inference_matrix.rs` carries only the schema string `cynic.inference.matrix_sweep.v2` | keep byte-identical on move; never rename to the destination crate |

### 5.2 Incoming edges (consumers that must be re-routed)

| # | Consumer | Resolution |
|---|----------|-----------|
| I1 | `cynic-cli/.../local_runtime.rs:476` calls `forge_profile_from_benchmark_report` | call moves to `cynic_eval::...` (O10); CLI already deps eval |
| I2 | `cynic-inference/tests/debate_live_test.rs` uses `cynic_inference::MultiAgentService` | test MOVES to eval, rewritten to `cynic_eval::...` |
| I3 | `lib.rs` re-exports: `bench_basket_launcher::{build_basket_from_ledger, launch_basket}` (L36), `multi_agent_service::MultiAgentService` (L53), `debate::{...}` (L79), `matrix_sweep::{empirical_store::{...}, types::HardwareBudget}` (L86/89) | REMOVE from inference `lib.rs`; re-export from `cynic-eval` lib; audit `cynic-cli` + host/intel/kernel/node for each name before removal (only I1 found outside) |
| I4 | `application/mod.rs` (`agent_harness`, `bench_basket_launcher`, `failure_gallery`, `matrix_live_runner`, `matrix_sweep_executor`) + `domain/mod.rs` (`debate`, `matrix_sweep`) declarations | REMOVE in the same slice (schema_warden: no orphan `pub mod`) |
| I5 | `cynic-host/src/agent_matrix.rs`, `cynic-host/tests/multi_agent_e2e_simulation.rs` | verified: NO code imports of the perimeter (surface-name strings / lease strings only) — no action |
| I6 | staying `hermes_run_executor`, `hermes_process_supervisor`, `harness_executor`(if staying) | re-import harness traits from eval (O7); `HttpHarnessExecutor` moves per O5 |

### 5.3 Full extraction — CONVERGED (all three slices + spike)

| slice | moved to eval | proofs |
|---|---|---|
| spike (vocab) | `sweep/` vocab + `debate` + `empirical_store_port` + `pareto` (~1.4 k) | eval@d203e926, inference@74f34c24 |
| 1 (harness) | `harness/` ×10 + `BenchModelProfile` view (O1); `failure_gallery` burned (zero users) | eval@f7b61517, inference@18dd3f74 |
| 2 (executors) | `sweep::{matrix_live_runner, matrix_sweep_executor}` (+tokio/reqwest/futures edges) | eval@29c214dd, inference@7402efa6 |
| 3 (bench+agents) | `bench/` (basket + forge, seams injected) + `agents/` + `HttpHarnessExecutor` + debate test | eval@2877590f, inference@36826c8e, cli@36826c8e |

Seam resolutions: O4 → `BenchLauncherPort` declared eval-side, implemented
for `dyn LocalInferenceRuntimePort` in inference; O10 → forge takes
`profiles_dir: &Path`, `alias_from_artifact_id` moved to eval (pure string
fn), bootstrap ensure-path calls eval's `write_loopback_profile`; secret
fallback via `cynic-secrets::resolve_secret` (downward, no cycle); `roofline_inputs`
carried as opaque JSON. All wire schemas byte-identical
(`matrix_sweep.v2`, `bench_basket.v1`, `forge_profile.v1`,
`multi_agent_debate.v1`).
Sizes: inference 30 k → 20.8 kLOC; eval 4.4 k → 9.6 kLOC (well under the
15 kLOC band — no monolith moved, growth absorbed in four bounded slices).
Zero orphans verified by workspace-wide grep.
Remaining burn candidate: `math::ucb_sampler` (declared + re-exported, zero
users) — untouched, out of slice.

### 5.4 Phase-2 partial — CONVERGED (falsified scope)

Falsification of "P2 blocked": the boundary test
(`kernel_inference_router_delegates_node_operations_to_cynic_inference_application`)
pins ONLY `node_runtime_ops` dog fns. `peer_attach` is absent from the test;
`node_contribution` is a zero-user stub. Moved without touching the test:
`peer_attach.rs` → `cynic-node::peer_attach` (serving imports rewired to
`cynic_inference::` paths, all downward), `node_contribution` →
`cynic-node::domain`. CLI attach arm calls `cynic_node::peer_attach`.
Proofs: node@91c56eda, inference@888aaacf, cli@888aaacf. Inference −285
lines. `node_runtime_ops` STAYS until the operator amends the boundary test.

### 5.5 Suspended (evidence-backed, awaiting operator release)

- §2.5 remainder (`node_runtime_ops` dog fns): CONFIRMED KEEP (operator
  decision R). Technically movable (imports only `std::time::Instant`) but
  the boundary test is deliberate kernel law; the node façade already owns
  the surface and only two modules consume the impl — moving gains nothing
  mechanically. Revisit only if dog ops acquire fleet-side state (leases,
  fleet truth) forcing colocation.
- §2.6 (`finance_adapters.rs`): impl parses inference trace files with zero
  inference types; every crate already owns its `finance_adapters` impl and
  the port lives in `cynic-finance`. Current state already satisfies FCIS —
  no move recommended.
- §2.7 (publisher impls out): infeasible as written. `peer_attach` (now
  node-owned, still serving-constructed) and staying `ods_discovery` build
  these impls directly; `cynic-inference` has no upward dep to host/node to
  re-import them from. Impls stay; ports were already the boundary.
- Q1 (hermes home): RESOLVED by deletion (tranche A — zero users).
- Q2 (supervision keep), Q3 (progress/verdict keep):
  open operator decisions; no code changed.

### 5.7 Tranches B/C/D — CONVERGED

- B: burned dead `cli` backend (242); split `chat_service` 894 → 719 +
  `chat_support` 186 (mock/tools doubles extracted, `now_ms` moved with
  them, API unchanged). Proofs: inference@48e4e5f2, cli@48e4e5f2.
- C: burned dead pool/slot/queue cluster (1 645): `model_process_pool`,
  `slot_semaphore`, `inference_queue`, `slot_tracker` + unpinned
  `select_with_slot_tracker` removed from pinned `proxy_routing` (legal
  surface intact). Proof: inference@36d0fa56.
- D: burned dead empirical store (307) + `AutoRecoveryEmbedding` (75) +
  `FixedEmbedding` (22); `NullEmbedding` impl preserved (host uses it —
  caught and restored my own over-deletion by diff). Proof:
  inference@808b3669.
- Total: inference 30 k → 15.6 kLOC (−48 %). Q1 resolved by deletion.

### 5.6 Tranche A (internal burn) — CONVERGED

Burned 18 files, −2 457 lines (all verified zero-user, unpinned):
hermes cluster ×5 (`hermes_run_executor` 458, supervisor 338, service 81,
domain 70, port 22 — resolves Q1 by deletion), `inference_gateway` 440,
dead routing cluster (`infra/routing`, `routing_policy`, `model_selection`,
`fleet_routing`, `routing_calc`, ~650), pass-throughs (`inference_service`,
`summarization_service`), `embedding_runtime`, `engine_selector`,
`chat_trace_sink` (ledger consts moved to `ChatTrace`), `filesystem_capacity`
+ its port. Pinned-but-unused KEPT: `proxy_routing`, `model_catalog`,
`openai_compat`, embedding adapter, dog fns. `RoutingEventPort` stays
(host orphan baseline lists it). Proofs: inference@260b9515, cli@260b9515.

Eval-slice standing non-goals (unchanged): `verdict_cache`, `fs_empirical_store`
impl, `math::{roofline,tensor_geometry}`, `ports::local_runtime_port` itself,
finance glue, ODS publishers.
ODS publishers — untouched. `MultiAgentService`'s `RuntimeKind::DirectLlama`
dependency resolves via O5 (executor moves with it).

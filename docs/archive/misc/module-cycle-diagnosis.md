# Module Cycle Diagnosis

Status: `diagnosis` — measured, with one hypothesis tested and falsified.
Schema: `cynic.module_cycle_diagnosis.v1`
Method: Tarjan strongly-connected components over the module graph, with the
`#[cfg(test)]` regions excluded. Every number below comes from a script that ran.

---

## 0. Correction to an earlier figure

An earlier pass reported **"27 two-module cycles"** (cynic-host 14, cynic-node 10,
cynic-cli 1, cynic-protocol 2). That was a naive count of mutually-importing
*pairs*. It counted symptoms, not structure, and it was wrong in both directions:
it over-counted cynic-cli and cynic-protocol (which have **zero** cycles) and
under-counted the real entanglements.

The real structure, by strongly-connected component:

| Crate | Modules | Edges | **SCCs** | Largest |
|---|---|---|---|---|
| `cynic-host` | 118 | 252 | **2** | **14 modules** |
| `cynic-node` | 54 | 145 | **1** | **23 modules** |
| `cynic-cli` | 70 | 97 | **0** | — |
| `cynic-protocol` | 104 | 120 | **0** | — |

**Two of the four largest crates are not tangled at all.** `cynic-protocol` —
1 003 `pub` items, 104 modules — is acyclic. That is worth stating: the defect is
not "CYNIC is spaghetti". It is concentrated in two places.

---

## 1. The cycles

### 1.1 `cynic-host` — SCC of 14

```
adapters <-> agent_access <-> bridge <-> domain <-> endpoint_observation
<-> execution <-> fleet_disk_observer <-> ports <-> pre_os_control
<-> process_observation <-> repo_access <-> runtime_layout
<-> service_supervisor_observation <-> transport
```

44 internal edges. The cycle is dominated by **two hubs**:

| Module | Out | In |
|---|---|---|
| `adapters` | 12 | 5 |
| `ports` | 3 | 13 |
| `domain` | 4 | 12 |
| `execution` | 6 | 7 |

### 1.2 `cynic-host` — SCC of 3

```
conformance <-> runtime_proof <-> vertical_conformance
```

4 internal edges. Small, self-contained.

### 1.3 `cynic-node` — SCC of 23

```
adapters, config, dns_activation_status, dns_bundle, domain, network_activation_chain,
network_actuator, network_actuator_status, network_evidence, network_mesh_apply_plan,
network_mesh_bundle, network_mesh_plan, network_repair, network_root_policy,
network_runtime_status, network_shadow_proof, observe, ports, provider_fallback_hosts,
provider_fallback_status, usecases, watchdog, websocket
```

88 internal edges. Hubs: `adapters` (out 7, in 10), `usecases` (out 23, in 3),
`ports` (out 4, in 19), `domain` (out 2, in 16), `observe` (out 0, in 12).

---

## 2. A correction to my own layer model

Before the falsification result is meaningful, the model it used has to be
right — and it was not.

**What I assumed.** `domain` is layer 0, `ports` is layer 1, so every
`domain -> ports` edge is a lower layer depending on a higher one: an inversion.

**What is actually true.** In hexagonal architecture the ports *are* the
domain's own interface. In this crate `domain/shared.rs` declares
`SubstrateDomainContract`, and `domain/{services,privilege,packages,principal}.rs`
implement it. A module cannot implement a trait without the trait in scope, so
`domain -> ports` is **intra-layer**. Confirming the direction:
`ports/mod.rs` imports `domain` **10 times** — ports depend on domain, which is
correct, and it means they belong to the same layer.

**Consequence.** 4 of the 12 edges I called inversions were not:

| Edge | Verdict |
|---|---|
| `domain -> ports` (4 files: `services.rs:10`, `privilege.rs:12`, `packages.rs:14`, `principal.rs:11`) | **intra-layer, legal** — `use crate::ports::HostOperation` to implement the contract |

And the residual `domain <-> ports` cycle that survives every repair is a **Rust
language artifact**: a trait and its implementations cannot live in separate
modules without mutual imports. Many Rust hexagonal codebases carry it. The
correct treatment is to declare it permitted intra-layer, not to fight it.

**Corrected model, used for the rest of this document:**

```
inside  : domain + ports     the hexagon's interior and its own interface
app     : usecases           the application layer
outside : adapters + every infrastructure module
```

An inversion is **inside -> outside** or **app -> outside**. Nothing else.

---

## 2b. The inversion hypothesis, re-tested with the corrected model

Re-running the falsification with the right model gives a different and sharper
answer.

### `cynic-host` — 7 real inversions

| Edge | Evidence |
|---|---|
| `domain -> adapters` | `domain/network.rs:3` — re-exports `HttpFreeboxProbeAdapter`. **Dead: 0 consumers of the re-export.** |
| `domain -> execution` | `domain/ecosystem_package/integration_apply.rs:2`, `integration_connect.rs:1`, `integration_apply_types.rs:3` |
| `domain -> runtime_layout` | `domain/services.rs:11`, `domain/privilege.rs:13`, `domain/packages.rs:15` |
| `ports -> bridge` | `ports/mod.rs:6` — `PrivilegeBridgeInstallSpec`, `PrivilegeBridgeObservedState` |
| `ports -> execution` | `ports/mod.rs:17` — `AdmittedHostOperation` |
| `usecases -> execution` | `usecases/execution.rs:6`, `usecases/tests.rs:22` |
| `usecases -> runtime_layout` | `usecases/tests.rs:29` |

Removing them: **1 SCC -> 4 SCCs.** Residual: `domain <-> ports` (the language
artifact), `process_observation <-> runtime_layout`, `bridge <-> execution`,
`adapters <-> agent_access <-> fleet_disk_observer`.

### `cynic-node` — 23 real inversions, and 18 of them are one file

| Edge group | Count | Evidence |
|---|---|---|
| `usecases -> network_*` | **15** | **`usecases/network_surfaces_uc.rs:10-25`** imports `dns_activation_status`, `dns_bundle`, `network_activation_chain`, `network_actuator`, `network_actuator_status`, `network_evidence`, `network_mesh_apply_plan`, `network_mesh_bundle`, `network_mesh_plan`, `network_repair`, `network_root_policy`, `network_runtime_status`, `network_shadow_proof`, `provider_fallback_hosts`, `provider_fallback_status` — **directly** |
| `usecases -> observe` | 3 | `node_network_recovery_uc.rs:22`, `network_surfaces_uc.rs:25`, `observe_node_uc.rs:19` |
| `usecases -> adapters` | 3 | `storage_controller_uc.rs:6`, `build_workload_grant_uc.rs:10`, `node_enrollment_lifecycle_uc.rs:5` |
| `usecases -> config` | 2 | `node_enrollment_lifecycle_uc.rs:7`, `builder_runtime.rs:5` |
| `usecases -> watchdog` | 1 | `network_watchdog_uc.rs:19` |
| `ports -> network_actuator` | 1 | `network_command_runner_port.rs:2` |
| `ports -> network_mesh_plan` | 1 | `network_mesh_registry_port.rs:3` |
| `ports -> websocket` | 1 | `runtime_federation_queue_store_port.rs:3` |
| `domain -> adapters` | 2 | `observation_runtime.rs:4`, `observation_plan.rs:4` |

**18 of 23 inversions are the application layer reaching directly into 15
infrastructure modules, and most of them are in ONE file:**
`usecases/network_surfaces_uc.rs`.

That is not 18 mistakes. It is one missing port. The application layer has no
`NetworkSurfacePort` to call, so it calls the infrastructure modules themselves.

Removing them: **1 SCC -> 2 SCCs.** Residual: `domain <-> ports` (the language
artifact) and a 6-member infrastructure tangle
(`adapters <-> network_actuator <-> network_mesh_plan <-> network_root_policy
<-> observe <-> websocket`).

---

## 2c. What the falsification actually proved

The first run said "removing the inversions does not dissolve the SCCs" and I
read that as "the inversions are not the cause". That reading was too strong.

With the corrected model the accurate statement is:

| Claim | Verdict |
|---|---|
| The SCCs are caused by a few type misplacements | **false** |
| The inversions are irrelevant | **false** — removing them splits the tangles |
| One SCC is one missing port plus one unextracted subsystem | **true** |
| `domain <-> ports` is a defect | **false** — a Rust language artifact |

The repair target is therefore not "move 12 types". It is:
1. **one port** (`NetworkSurfacePort`) replacing 15 direct imports in one file
2. **one extraction** (the network subsystem) removing the 6-member tangle
3. **one declaration** that `domain <-> ports` is permitted intra-layer



**Hypothesis.** The SCCs are caused by a small number of type misplacements: a
`domain` module importing its own `ports`, a `ports` module importing an adapter.
Move those types to their correct layer and the graph becomes a DAG.

**Method.** Assign layers (`domain` 0, `ports` 1, `usecases` 2, `adapters` 3),
treat infrastructure modules as above `domain`/`ports`, mark every edge pointing
from a lower layer to a higher one as an inversion, remove exactly those edges,
recompute the SCCs.

**12 inversions found**, with their source lines:

`cynic-host`
| Edge | Evidence |
|---|---|
| `domain -> adapters` | `domain/network.rs:3` — `pub use crate::adapters::http_freebox_probe_adapter::HttpFreeboxProbeAdapter;` |
| `domain -> ports` | `domain/services.rs:10`, `domain/privilege.rs:12`, `domain/packages.rs:14`, `domain/principal.rs:11` — all `use crate::ports::HostOperation;` |
| `domain -> execution` | `domain/ecosystem_package/integration_apply.rs:2` |
| `domain -> runtime_layout` | `domain/services.rs:11`, `domain/privilege.rs:13` |
| `ports -> bridge` | `ports/mod.rs:6` — `use crate::bridge::{PrivilegeBridgeInstallSpec, ...};` |
| `ports -> execution` | `ports/mod.rs:17` — `use crate::execution::AdmittedHostOperation;` |

`cynic-node`
| Edge | Evidence |
|---|---|
| `domain -> adapters` | `domain/observation_runtime.rs:4`, `domain/observation_plan.rs:4` |
| `domain -> ports` | `domain/mesh_coordination.rs:5`, `domain/observation_plan.rs:5` |
| `ports -> network_actuator` | `ports/network_command_runner_port.rs:2` |
| `ports -> network_mesh_plan` | `ports/network_mesh_registry_port.rs:3` |
| `ports -> websocket` | `ports/runtime_federation_queue_store_port.rs:3` |
| `usecases -> adapters` | `usecases/storage_controller_uc.rs:6`, `usecases/build_workload_grant_uc.rs:10` |

**Result: falsified.**

| Crate | SCCs before | After removing 12 inversions |
|---|---|---|
| `cynic-host` | 2 | **4** — the big one splits, but does not dissolve |
| `cynic-node` | 1 | **1** — 23 members → **20**. Barely moves. |

The residual SCCs in `cynic-host`:

```
process_observation <-> runtime_layout
bridge <-> execution
adapters <-> agent_access <-> fleet_disk_observer
conformance <-> runtime_proof <-> vertical_conformance
```

**Conclusion.** The inversions are real defects — `domain` reaching for
`HttpFreeboxProbeAdapter` is an inversion by any definition — and moving those 12
types is worth doing. But **they are not the cause of the SCCs.** Fixing them
alone converts one large tangle into four smaller ones.

---

## 3. What the cycles actually are

Each SCC is a **self-contained subsystem with internal cycles**: a domain that
has not been extracted, whose modules reach sideways because no layer assignment
exists yet.

| Crate | Subsystem | Modules | Internal edges |
|---|---|---|---|
| `cynic-node` | network / dns / observe / watchdog / websocket / provider_fallback | **27** | **48** |
| `cynic-host` | bridge / execution / runtime_layout / process_observation / transport / repo_access / pre_os_control / fleet_disk_observer / service_supervisor_observation / endpoint_observation | **10** | **14** |

`cynic-node`'s network modules, in full:

```
dns_activation, dns_activation_status, dns_bundle, dns_contract, dns_plan,
network_activation_chain, network_actuator, network_actuator_contract,
network_actuator_status, network_command_surface, network_diagnostic_surface,
network_evidence, network_host_substrate, network_mesh_apply_plan,
network_mesh_bundle, network_mesh_plan, network_repair, network_root_policy,
network_runtime_proof, network_runtime_status, network_shadow_proof,
network_surface_inventory, observe, provider_fallback_hosts,
provider_fallback_status, watchdog, websocket
```

**27 modules, 48 edges between them, and this is one bounded context.** The
23-member SCC is the loopiest part of it. That is not 23 mistakes; it is one
missing boundary.

**This converges with an earlier diagnosis.** `docs/design/fleet-network-architecture.md`
already found 158 fleet/mesh/wireguard/network types scattered across 10 crates,
and proposed 5 bounded contexts. The cycle analysis independently identifies the
same subsystem from a different direction — dependency structure rather than type
inventory. Two methods, one conclusion: **the network plane is a bounded context
living inside a crate that does not own it.**

### Greedy feedback arc set — the size of the job

Minimum FAS is NP-hard, so this is the standard greedy approximation over short
cycles. Reported as an estimate, not a proven minimum.

| Crate | Edges to re-point |
|---|---|
| `cynic-host` | **11** |
| `cynic-node` | **10** |

The first edges each pass picks, in order, are exactly the inversions from §2 —
which confirms the inversions are the right *start*, even though they are not the
whole cause. After them the picks are infrastructure-to-infrastructure:
`bridge -> execution`, `execution -> ports`, `adapters -> agent_access`,
`adapters -> fleet_disk_observer`, `network_mesh_plan -> ports`, `usecases -> watchdog`.

---

## 4. What this means for the repair

**Do not fix 27 cycles.** Fix boundaries.

| Phase | Action | Why |
|---|---|---|
| **C1** | Extract the `cynic-node` network subsystem (27 modules) as a bounded context | It is one domain, and it is already known to be fragmented across 10 crates. Extracting it removes 23 of the 23 SCC members at once. |
| **C2** | Extract the `cynic-host` infra modules (10 modules) as a bounded context | Same shape, smaller. |
| **C3** | Move the 12 inversion types to their correct layer | Small, mechanical, and they are the first edges any repair must touch anyway. |
| **C4** | Ledger the residue shrink-only | `conformance <-> runtime_proof <-> vertical_conformance` is 3 modules and self-contained; it may not deserve extraction. |

C1 and C2 are the same work as the big-bang extraction already planned for
`cynic-host` and `cynic-observer`. The cycle analysis says **which part of that
extraction matters most**, which the type inventory alone did not.

---

## 5. Open decisions

1. **C1 scope** — extract all 27 network modules, or only the 23 in the SCC?
2. **C1 target** — a new crate (`cynic-network-plane`?), or into the existing
   `cynic-network` (which already holds the mesh types and the Headscale adapter)?
   Careful: `cynic-network` is L1, `cynic-node` is L3. Moving L3 modules into L1
   inverts the DAG unless the extracted modules are pure.
3. **C2 target** — the same question for the 10 `cynic-host` infra modules.
4. **C3 now or after C1/C2?** — the 12 inversions are cheap and independent.
5. **C4** — does `conformance <-> runtime_proof <-> vertical_conformance` earn an
   extraction, or a ledger entry?

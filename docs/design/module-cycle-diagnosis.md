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

## 2. The inversion hypothesis, tested and falsified

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

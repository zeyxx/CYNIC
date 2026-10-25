# CLI Family Scope — machine portability and federation

> Status: audit draft (read-only). Non-authoritative narrative; authority remains
> `contracts/foundation.json`, `infra/registry.json`, and `infra/external-project-registry.json`.
> Companion: `docs/design/GASDF_INGESTION_PLAN.md`, `docs/specs/machine-enrollment-federation.md`.

## 0. Intent

The CYNIC CLI grew by domain (host, network, federation, runtime, node, …)
without an explicit declaration of **which machine a family targets** and
**how it scales to N heterogeneous machines**. This document fixes that gap:
it maps every family to its machine scope, lists the overlaps, and proposes
the target model.

## 1. Machine scopes

| Scope | Definition | Examples |
|---|---|---|
| `local` | The machine running the CLI right now | cortex, builder, laptop |
| `federation` | A logical view across enrolled nodes | mesh, peers, fleet |
| `remote` | A specific enrolled node, reached via its service plane | a VPS, an agent host |
| `mobile` | A constrained device (Android) acting as light agent | phone |
| `external` | A machine CYNIC does not control (no root, no enrollment) | third-party VPS, contributor |

## 2. Family → scope map

| Family | Scope | Domain | Overlap / ambiguity |
|---|---|---|---|
| `host` | local | OS substrate, services, hardware, network baseline | `host network` vs `network` |
| `host service` | local | systemd reconciliation | vs `daemon observe`, `supervise observe`, `node storage-reconcile` |
| `host network` | local | network baseline + actuation | vs `network` (federation) |
| `network` | federation | mesh, peers, DNS, WG, reconcile, observe | vs `host network`, `federation` |
| `federation` | federation | status, enroll | vs `network mesh`, `node consensus-status` |
| `daemon` | local | kernel/node daemons (long-running) | vs `supervise run` |
| `supervise` | local | declared-role observation + parent control | vs `daemon observe` |
| `privilege` | local | typed root execution (catalog/admit/exec) | clear boundary |
| `runtime` | local | paths, registry, conformance, disk, memory, CPU, network, storage | vs `host`, `node host` |
| `node` | local + federation | identity, enrollment, consensus, compute, storage | very broad, fuzzy boundary |
| `node host` | local | hardware probe (CPU, memory, disk, capacity) | vs `runtime {disk,memory,cpu}`, `host hardware` |
| `fleet` | federation | runners, capacity | vs `federation status` |
| `source` | federation | source sync, worktrees, PRs | clear |
| `build` | federation (builder) | builds, tunnels, webhook dispatch | clear |
| `storage` | local + federation | SurrealDB, credentials | vs `runtime storage` |
| `inference` | local | models, vault, hardware | clear |
| `agent` | federation | governance: identify, register, claim, release, status | vs `coordination`, `task` |
| `coordination` | federation | fire-and-forget agent log | vs `agent` |
| `task` | federation | async domain-partitioned queue | vs `agent` |
| `idea` | federation | idea ledger | clear |
| `discovery` | federation | capabilities, nodes, events | vs `federation`, `node` |
| `audit` | local + federation | crate, secret, node host, paths, branch, all, conformance, metrics, harness, external, gate receipt | clear |
| `secret` | local | secret posture, check, resolve, install, use, audit | clear |
| `models` | local | LLM model profiles | clear |
| `doctor` | local | read-only diagnosis | clear |

## 3. Key overlaps (consolidation candidates)

1. **Services** — four surfaces for the same object (systemd units):
   - `host service reconcile`
   - `daemon observe`
   - `supervise observe`
   - `node storage-reconcile`

2. **Network** — four families for the network:
   - `host network {baseline, actuate}`
   - `network {chain, runtime, proof, observe, watchdog, notify, sync, reconcile, …}`
   - `runtime network {status, preflight}`
   - `node host network`

3. **Federation** — four surfaces for fleet state:
   - `federation {status, enroll}`
   - `network mesh {plan, bundle, apply, status, coordination}`
   - `node consensus-status`
   - `fleet observe`

4. **Hardware / resources** — three families for the same hardware:
   - `runtime {disk, memory, cpu, network}`
   - `node host {cpu, memory, disk, capacity}`
   - `host hardware`

5. **Inter-agent governance** — four families for coordination:
   - `agent {identify, register, claim, release, status}`
   - `coordination`
   - `task`
   - `idea`

## 4. Scaling model

The current model (`--host <name>`) does not scale to N heterogeneous machines.
Three candidate models:

| Model | Principle | Scale machines | Scale diversity | Complexity |
|---|---|---|---|---|
| A — By machine | Each family targets one machine | Poor (N machines = N scopes) | Poor | Low |
| B — By role | Machines declare a role (host, builder, agent, mobile, VPS) | Good | Medium | Medium |
| C — By capability | Machines expose capabilities (compute, storage, inference, relay); federation resolves | Excellent | Excellent | High |
| D — Hybrid (role + capability) | Role declared + capabilities observed; CLI filters by `--role` or `--capability`; federation resolves the machine | Excellent | Excellent | High but incremental |

**Target (R): Model D — hybrid role + capability.**

Rationale: the federation pattern already provides the building blocks
(`MachineEnrollment`, `FleetRole`, `AutonomyMode`, peer resolver, emergence).
Model D uses them: a machine joins via enrollment (no root required),
declares its role and capabilities, and the CLI filters by role/capability
instead of by host name. The machine is resolved by the federation, not by
the CLI.

What this changes:
- No more `--host cortex` in host/network commands. Instead: `--role host` or `--capability compute`.
- Machines without root (Android, third-party VPS) enroll as light agents
  (`mobile`, `contributor`), not as full hosts.
- The registry declares roles and capabilities; observation verifies them.
- Scaling is transparent: adding a machine = adding a registry entry with
  role + capabilities + enrollment.

What already exists:
- `infra/registry.json`: nodes with `role` (string) and `capabilities` (minimal, on 2 nodes).
- `cynic federation`: status, enroll.
- `cynic node`: identity, enrollment, consensus.
- `cynic discovery`: capabilities, nodes, events.
- `FleetRole` enum (6 variants, closed) in `cynic-protocol/src/federation_membership.rs`.
- `MachineEnrollment` in `cynic-protocol/src/secret_reference.rs`.

What is missing:
- A standardized `capabilities` field in the registry (functional capabilities,
  not just cpu/ram/gpu).
- A `--role` / `--capability` filter transversal to host/network/runtime families.
- Distinction between full host and light agent (mobile, VPS without root)
  in the enrollment model.
- `FleetRole` enum does not cover all registry roles (mesh_coordinator,
  embedding_runner, workstation_portable, agent_host are missing).

## 5. Sequencing (avoid dispersion)

The following chantiers are in flight. Sequence them to avoid dispersion:

1. **This document** (done, read-only audit).
2. **FleetRole extension** — extend the enum to cover missing roles + new ones
   (mobile, vps, contributor). Controlled contract change.
3. **Registry capabilities standardization** — add functional capabilities
   to all nodes, not just 2.
4. **GASDF declaration** — add the 4 GASDF units to the cortex host profile
   in the registry, then reconcile.
5. **CLI family consolidation** — merge the 5 overlap groups (services,
   network, federation, hardware, governance) into clearer families.

Each step is a separate PR on Forgejo. No step blocks the others, but the
order matters for coherence.

## 6. Rules (portability)

- A family MUST declare its machine scope in `--help` (one line).
- A family that targets `local` MUST NOT silently reach remote machines.
- A family that targets `federation` MUST accept `--registry` and emit
  JSON by default.
- New families MUST NOT be top-level hyphenated commands (per
  `cynic-cli-thin-surface` law). They must nest under an owning family.
- Machine identity MUST come from the registry, never from hardcoded IPs.
- Root-required operations MUST go through `cynic privilege exec`, never
  through direct `sudo` in scripts.

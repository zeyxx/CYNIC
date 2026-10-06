# Runtime Roles — kernel, node, and everything else

Status: `proposed` — awaiting operator validation before any code moves.
Schema: `cynic.runtime_roles.v1`
Evidence date: 2026-10-06. Every claim below names its source file or the
observed host. Anything without a source is marked `unknown`, never asserted.

This document joins three authorities that each know half the picture and
never meet: `infra/runtime-authority.json` (who owns what),
`infra/runtime-basket.json` (which artifact runs where, for whom), and
`infra/registry.json` (which nodes exist, in which role). The join is the
role model. Nothing here rebuilds those files; it reads them.

---

## 1. The five runtime surfaces (declaring authority: runtime-authority.json)

| Surface | Owner crate | Artifact | Job in one line |
|---|---|---|---|
| `kernel_bus` | cynic-kernel | `cynic-kernel` | Coordination, observations, health, runtime read-back. Explicitly NOT mutation execution authority. |
| `node_runtime` | cynic-node | `cynic-node` | Node-local supervision, runtime probes, network/DNS status, node proof emission. Consumes cynic-host + agent_workload protocol. |
| `host_substrate` | cynic-host | (none — library + `cynic-host-exec` seed, `cynic-topology`, `cynic-audit`) | OS principal policy, filesystem/package/service projection, bounded process execution, host proof. |
| `operator_cli` | cynic-cli | `cynic` | Operator command grammar, automation JSON, bounded local audits. Non-supervised. |
| `protocol_contracts` | cynic-protocol | (none — types only) | Shared no-I/O types, workload admission, pure lifecycle transitions. Never executes. |

Sixth surface, `runtime_basket` (`infra/runtime-basket.json`): the artifact
catalog itself — which binary ships to which role, for which audience, with
which proofs. It declares; `contracts/runtime-update.json` executes.

## 2. The four audiences (declaring authority: runtime-basket.json)

| Audience | Supervised | Privilege | Install root | Meaning |
|---|---|---|---|---|
| `operator` | no | user | `/usr/local/bin` | Invoked by human or agent. Never a daemon. |
| `service` | yes (systemd) | `cynic` | `/usr/local/bin` | Long-running daemon. Never invoked interactively. |
| `privileged` | no | root | `/usr/local/libexec` | Root-boundary bridge (seed, exec bridge). One root binary must not expose every subcommand. |
| `tooling` | no | user | (none — never deployed) | Dev/audit tools. A tooling artifact that grows a deployment consumer must be reclassified, not silently shipped. |

Current artifact map: `cynic` (operator) → cortex_host, hypervisor,
kernel_primary, build_runner. `cynic-node` (service) → same four roles.
`cynic-kernel` (service) → kernel_primary only. `cynic-mcp` (service) →
cortex_host. `cynic-lxc-exec-bridge` (privileged) → hypervisor.
Tooling (never deployed): `cynic-node-enroll`, `cynic-network-actuator`
(recorded DEAD, candidate for deletion), `cynic-topology`, …

## 3. Kernel vs node vs the rest — the distinction that matters

- **Kernel** (`cynic-kernel`, audience `service`, role `kernel_primary`):
  the fleet's memory and judge. It coordinates, keeps the workload journal,
  scores evidence, serves read-back. It does not actuate mutations —
  coordination messages are not execution authority. One kernel per fleet
  scope; every node trusts the same journal or there is no fleet.
- **Node** (`cynic-node`, audience `service`, all host roles): the hands of
  one machine. It supervises node-local services, probes runtime state,
  reports network/DNS status, emits proofs upward. It never decides fleet
  policy; it reconciles desired state (registry) with observed state (host)
  and reports the delta.
- **Host substrate** (`cynic-host` library + `cynic-host-exec` seed): the
  muscle under the node. OS-level observation and bounded actuation behind
  the privilege boundary. The node asks; the host executes or refuses.
- **Operator CLI** (`cynic`): the mouth. Grammar + JSON, no supervision, no
  daemon semantics. An agent driving `cynic` is an operator, not a service.
- **MCP projection** (`cynic-mcp`): a replaceable agent-facing projection,
  never a bootstrap prerequisite (AGENTS.md). Projections do not own.
- **Enrollment tooling** (`cynic-node-enroll`): runs once per node admission,
  never as a daemon. Admission is an event, not a service.
- **Watchdogs/timers** (`cynic-hypervisor-watchdog.timer` on PVE): time-based
  supervision, owned by whoever declares the timer — currently nobody
  declares it in any contract (`unknown` ownership, see §5).

Rule of thumb: **kernel remembers, node acts, host executes, CLI asks,
protocol constrains, projections translate.** Anything that mixes two of
these in one binary is a layering violation to extract, not a shortcut to
keep.

## 4. Observed reality vs declared model (2026-10-06)

| Node | Declared role | Actually running | Verdict |
|---|---|---|---|
| cynic-builder (10.10.10.119) | `build_runner` | `cynic-node.service` → `/var/lib/cynic/bin/cynic-node` (via `current`→v0.8.0) + socat proxy unit | Conformant. Proxy unit is not a CYNIC artifact — must never enter a restart list. |
| cynic-host-proxmox | `hypervisor` | `cynic-cloud.service` (`/usr/local/bin/cynic-cloud-api`, 34 MB, Jun 12) + watchdog timer + dead `cynic-node` binary (Jul 1, no unit) | **Non-conformant.** Cloud API has zero provenance in repo, basket, or contracts. No releases dir — direct `/usr/local/bin` install, outside the update model. |
| local-kernel (10.10.10.100) | `kernel_primary` | `candidate` — no observed kernel process anywhere; builder proxy forwards 3030 → 192.168.100.2 (cortex) | `unknown`: where the kernel actually runs is unobserved. |
| forgejo / headscale / surrealdb | LXC roles | `candidate` | Unobserved this session. |

Port owners `unknown` (no passwordless sudo where observed, or never
probed): builder `:9090`, PVE `:80`/`:443`/`:4000`/`:53`/`:3128`.

## 5. Decisions required (operator)

1. **Orphan `cynic-cloud-api`**: identify source (which repo/commit built
   it) or quarantine it. A root-service binary with no provenance on the
   internet-ingress hypervisor is the fleet's largest single risk.
2. **Kernel home**: confirm which machine runs `cynic-kernel` for
   `kernel_primary`, declare its unit + health probe, enroll it in the
   update model.
3. **Watchdog ownership**: declare `cynic-hypervisor-watchdog` in a contract
   (owner, scope, alert path) or remove it.
4. **`cynic-network-actuator`**: basket already marks it DEAD — approve
   deletion.
5. **PVE enrollment**: PVE has no releases dir and no passwordless sudo for
   remote actuation. Decide the remote-deploy transport (ssh+sudo grant vs
   node-initiated pull) before PVE enters `fleet_order`.

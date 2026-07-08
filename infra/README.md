# CYNIC Infra Topology

`infra/` contains organism-wide desired state, deployment assets and low-level
operator bridges. It must not become a dumping ground for organ runtime state or
ad hoc scripts.

Normative parent policy:
`docs/foundation/CYNIC_REPO_TOPOLOGY_POLICY.md`.

Machine contract: `axioms/repo-topology-v1.json`.

## Canonical Boundaries

| Path | Concern | Authority |
|------|---------|-----------|
| `registry.json` | Fleet topology, lifecycle placement, network policy and organ declarations | Source of truth |
| `agent-exposure.json` | MCP/tool exposure projected to cortex clients | Source of truth |
| `systemd/` | Unit files and timers for concrete service placement | Deployment projection |
| `surrealdb/` | Database schema and migrations | Storage contract |
| `docker/` | Container packaging for bounded app surfaces | Deployment adapter |
| `firewall/` | Host firewall policy | Deployment adapter |
| `sudoers/` | Optional rendered least-privilege operator projections | Deployment projection |
| `monitoring/` | Legacy/bootstrap provider probes | Transitional adapter |

## Rules

1. Desired topology belongs in `registry.json`, not in shell scripts.
2. Network allocations belong in `registry.json:network_policy`.
3. Runtime evidence belongs in `organ-fleet`, kernel health, or structured
   observations, not committed state snapshots.
4. Provider actions such as Proxmox create/start/exec are adapter operations.
   Agents must consume the semantic contract in
   `axioms/infrastructure-mcp-v1.json` once implemented.
5. New infra concerns should first choose their owner:
   - `infra/registry.json` for organism-wide desired state.
   - `organs/<role>/` for a persistent subsystem.
   - `infra/systemd/` for deployment projection.
   - `organs/<role>/src/entrypoints/` for executable organ operations.
   - `infra/` contracts or projections for deployment-only concerns.

## Current Debt Markers

- `infra/monitoring/*` is bootstrap evidence, not the canonical fleet source.
- Raw Proxmox MCP tools are provider-level and must be wrapped by canonical
  fleet MCP operations before they are considered frictionless for agents.
- Storage LXC activation is blocked until canonical Synapse provisioning
  (`cynic-node` evidence + `organ-fleet`/network-actuator execution) and
  `fleet.node.exec` cover LXC deterministically.
- Dedicated storage activation artifacts belong to `organ-keep`; use
  `make storage-primary-bundle out=<dir>` for a non-mutating bundle. Legacy
  storage scripts remain blocked fallback only.

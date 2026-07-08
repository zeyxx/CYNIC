# CYNIC Organs Topology

`organs/` contains persistent organism subsystems. An organ is not a script
folder; it owns a bounded function, a manifest, ports/adapters, tests and a
runtime/deployment projection when active.

Normative parent policy:
`docs/foundation/CYNIC_REPO_TOPOLOGY_POLICY.md`.

Machine contract: `axioms/repo-topology-v1.json`.

## Canonical Organ Shape

```text
organs/<role>/
├── MANIFEST.yaml
├── README.md
├── pyproject.toml            # Python organs only, when executable
├── src/
│   ├── core/                 # entities, ports, use cases; no external I/O
│   ├── adapters/             # Proxmox, registry, kernel, DNS, systemd, etc.
│   ├── entrypoints/          # CLI/daemon wrappers
│   └── tests/
├── systemd/                  # organ-owned unit templates/projections
└── state/                    # local runtime state; not source of truth
```

## Current Infrastructure Organs

| Organ | Role | Canonical responsibility | Must not own |
|-------|------|--------------------------|--------------|
| `organ-contracts` | Shared contract kernel | universal command/report surface contracts and registry primitives | organ-specific business logic |
| `organ-fleet` | Fleet truth aggregator | desired/observed/effective state | provisioning mutations |
| `organ-wireguard` | Synapse mesh/IPAM | service-plane DNS, WireGuard, network policy validation | Proxmox lifecycle |
| `organ-forge` | Release lifecycle | build/deploy request payloads and gates | raw provider provisioning |
| `organ-lifecycle` | Transmutation candidate | future provider/runtime adapters | canonical release flow before e2e proof |
| `organ-headscale` | Access mesh | operator/agent reachability control plane | Synapse service-plane policy |

## Separation Rules

1. `core/` defines semantics and ports only.
2. `adapters/` implement external I/O and provider specifics.
3. `entrypoints/` wire adapters to use cases; they do not define policy.
4. Executable organ operations live under `src/entrypoints/`; organ-local
   `scripts/` directories are legacy drift unless explicitly quarantined.
5. A repo-affecting organ task must expose coordination targets before running.
6. An organ is active only when registry status and runtime evidence agree.

## Infrastructure MCP Mapping

The canonical agent-facing infrastructure contract is
`axioms/infrastructure-mcp-v1.json`.

- `organ-wireguard` backs `network.ipam.*` and `network.mesh.*`.
- `organ-fleet` backs `fleet.node.status` and effective-state evidence.
- Provider adapters under `organ-lifecycle` or a future dedicated provider organ
  back `fleet.node.provision` and `fleet.node.exec`.
- `organ-forge` backs release lifecycle operations, not generic infrastructure
  provisioning.

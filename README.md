# CYNIC

[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](rust-toolchain.toml)
[![Architecture](https://img.shields.io/badge/architecture-FCIS%20%7C%20L0--L4-green.svg)](infra/CANONICAL_CRATE_MAP.md)
[![Verification](https://img.shields.io/badge/proof-read--back%20verified-purple.svg)](contracts/foundation.json)

**CYNIC** is a low-layer, sovereign Rust runtime for autonomous multi-agent systems, epistemic consensus, and bounded host execution.

It coordinates human intent, operating systems, source matter, builds, artifacts, agents, inference runtimes, hosts, and distributed nodes without collapsing their authorities into a centralized, opaque controller. In CYNIC, **no mutation occurs without verifiable read-back proof**.

---

## Key Capabilities

- **Sovereign Multi-Agent Coordination**: Bounded delegation trees, lease fencing, monotonic budget caps, and conflict-free concurrent workspaces.
- **Epistemic Consensus & Proof Engine**: Pure state reconciliation with machine-readable read-back proofs for all host, network, and source mutations.
- **Functional Core, Imperative Shell (FCIS)**: Strict downward DAG architecture; zero-I/O domain cores decoupled from side-effecting adapters.
- **Host Substrate Instrumentation**: Portable host probes, process isolation, cgroups, LXC management, and multi-plane network mesh orchestration.
- **Inference & Execution Membrane**: Pluggable local model supervisors, agent delegation envelopes, and model-agnostic inference runtimes.

---

## Architectural Taxonomy

The runtime is structured into 44 workspace crates across 5 distinct abstraction layers:

```
┌────────────────────────────────────────────────────────┐
│  L4: Projections                                       │
│  cynic-cli · cynic-mcp                                │
└──────────────────────────┬─────────────────────────────┘
                           │
┌──────────────────────────▼─────────────────────────────┐
│  L3: Orchestration                                     │
│  cynic-kernel · cynic-node                             │
└──────────────────────────┬─────────────────────────────┘
                           │
┌──────────────────────────▼─────────────────────────────┐
│  L2: Domain Runtimes                                   │
│  storage · inference · build · vcs · network · pool    │
│  consensus · eval · secrets · workload · market        │
└──────────────────────────┬─────────────────────────────┘
                           │
┌──────────────────────────▼─────────────────────────────┐
│  L1: Host Substrate & Probes                           │
│  cynic-host · cynic-sandbox · cynic-lxc · cynic-pkg    │
└──────────────────────────┬─────────────────────────────┘
                           │
┌──────────────────────────▼─────────────────────────────┐
│  L0: Pure Core Types                                   │
│  cynic-protocol · cynic-types                          │
└────────────────────────────────────────────────────────┘
```

### Architectural Invariants

1. **Strict DAG Downward**: Higher layers depend on lower layers, never the reverse. Pure domain crates never depend on orchestrators or host primitives.
2. **Zero-I/O Domain Core**: State machines, parsers, planners, and predicates are 100% pure (zero `std::process`, `tokio::spawn`, or network I/O). Side effects are strictly confined behind explicit traits in `adapters/`.
3. **Coherence Loop**:
   $$\text{Desired State} \longrightarrow \text{Observe} \longrightarrow \text{Reconcile} \longrightarrow \text{Bounded Action} \longrightarrow \text{Read-Back Proof}$$

---

## Quickstart & Verification

### Prerequisites

- **Rust toolchain** (Rust 2024 edition compatible, stable 1.85+)
- `cargo`, `make`, `jq`

### Compilation Check

To check the workspace without debuginfo overhead:

```bash
RUSTFLAGS="-C debuginfo=0" cargo check --workspace
```

### Targeted Crate Check

```bash
cargo check -p cynic-protocol --lib
cargo check -p cynic-kernel --lib
```

### Secret & Invariant Audits

CYNIC includes built-in verification surfaces for credential leak prevention and architectural boundary audits:

```bash
# Verify zero secrets, credentials, or private keys are exposed
cynic audit secret --json

# Verify crate architecture and boundaries
cynic audit crate --json
```

### What These Produce

Every surface above emits machine-readable JSON, and every one of them is
read-only. This is the actual output of the two audits, on this workspace:

```json
{
  "schema": "cynic.host.secret_lint_observation.v1",
  "files_scanned": 2790,
  "findings": []
}
```

```json
{
  "schema": "cynic.cli.crate_audit.v1",
  "status": "ok",
  "rule": "core crates must keep executable boundary tests, bounded entrypoints, and explicit visibility into source hotspots",
  "crate_count": 46,
  "ok_count": 46,
  "blocked_count": 0
}
```

`status: "ok"` is not a claim — it is the exit status of a check that ran.
A surface that could not run reports `unknown`, never `ok`.

### The Full Surface

The CLI exposes 24 top-level arms. The audit pair above is the verification
subset; the rest are the runtime surfaces:

| Arm | What it does |
|---|---|
| `cynic audit` | Architectural invariants, secret scan, contract verification |
| `cynic build` | Build submission, proof recording, runner dispatch |
| `cynic agent` | Claim/release scopes, coordination, governance observation |
| `cynic source` | Source sync — observe, reconcile, apply across layers |
| `cynic node` | Node identity, enrollment certificates, lifecycle activation |
| `cynic host` | Local substrate operations, capability-gated mutation |
| `cynic network` | Mesh planning, peer consensus, registry audit, SOC audit |
| `cynic inference` | Local inference plane, vault, model store, Hub access |
| `cynic storage` | SurrealDB status, queries, capability and credential sync |
| `cynic doctor` | Read-only diagnosis of local LLM, network, sovereign Git |
| `cynic court` | Constitutional court — audit policy alignment across surfaces |
| `cynic judge` | Judge extracted patches against `origin/main` before adding |

Full list: `cynic --help`, or `cynic commands`.

---

## Workspace Map

| Layer | Crates | Focus |
|---|---|---|
| **L0** | `cynic-protocol`, `cynic-types` | Pure types, predicates, coordination contracts |
| **L1** | `cynic-host`, `cynic-sandbox`, `cynic-lxc`, `cynic-pkg`, `cynic-forensics`, `cynic-lifecycle` | Host OS observation, cgroups, sandboxing, packages |
| **L2** | `cynic-network`, `cynic-storage`, `cynic-inference`, `cynic-build`, `cynic-vcs`, `cynic-secrets`, `cynic-pool`, `cynic-market`, `cynic-consensus` | Domain execution, secure vaults, network mesh, contribution pool |
| **L3** | `cynic-kernel`, `cynic-node` | Runtime boot, orchestration bus, node supervision |
| **L4** | `cynic-cli`, `cynic-mcp` | Operator CLI surface, Model Context Protocol (MCP) projection |

---

## Authority & Governance

When declarations disagree, authority resolves strictly in this order:

1. **Observed runtime state** (actual hardware/OS evidence)
2. `contracts/foundation.json` (constitutional runtime contract)
3. `infra/runtime-authority.json` (operational authority mapping)
4. `infra/registry.json` (service and placement registry)
5. `Cargo.toml` (workspace dependencies and packages)
6. Canonical Rust source code under `crates/`

---

## Contributing & Security

- **Contributing**: Please review [CONTRIBUTING.md](CONTRIBUTING.md) for architectural guidelines, branching model, and code invariants.
- **Security**: For vulnerability disclosure and credential governance, see [SECURITY.md](SECURITY.md).
- **Agent Protocol**: Guidelines for automated agent contributors live in [AGENTS.md](AGENTS.md).

---

## License

CYNIC is licensed under either of:

- Apache License, Version 2.0 ([LICENSE](LICENSE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
- MIT License ([http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

at your option.

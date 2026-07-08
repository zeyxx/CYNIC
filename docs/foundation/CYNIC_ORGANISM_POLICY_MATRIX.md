# CYNIC Organism Policy Matrix

Status: normative foundation.

This document defines how CYNIC policies are split by dimension, scale and
runtime layer. It exists to prevent mixed concerns, hidden shortcuts and
transport-specific abstractions disguised as organism policy.

The layer precedence contract lives in
`docs/foundation/CYNIC_META_LAYER_CONTRACTS.md` and the machine contract lives
in `axioms/meta-layer-contract-v1.json`.

The empirical multi-scale rule lives in
`docs/foundation/CYNIC_MULTI_SCALE_EMPIRICAL_RULE.md` and the machine map
lives in `axioms/multiscale-organism-property-map-v1.json`.

## 1. Core Rule

Every durable rule must declare:

- dimension: what concern the rule governs.
- scale: where the rule applies.
- layer: where the rule is represented and enforced.
- authority: which source is allowed to change it.
- enforcement: what makes violation mechanically visible.
- exception path: how a temporary fallback is approved and retired.

If a rule lacks these fields, it is not organism policy. It is a local
implementation note.

## 2. Scales

| Scale | Meaning | Examples | Allowed authority |
|-------|---------|----------|-------------------|
| Cell | One process, container or systemd unit | `cynic-mcp`, `surrealdb`, one daemon | Unit file, local config, health probe |
| Adapter | Transport or runtime bridge | Slack, REST client, MCP proxy, Hermes CLI | Adapter contract + tests |
| Organ | Persistent subsystem | `organ-forge`, `organ-slack`, `organ-twitter` | `infra/registry.json`, organ README, manifest |
| Cortex | Episodic reasoning client | Codex, Claude, Antigravity | `AGENTS.md`, agent adapter docs |
| Kernel | Coordination, memory and judgment bus | `/coord`, `/agent-tasks`, `/observe`, `/health` | kernel API contract + storage model |
| Repo | Organism DNA | source tree, docs, gates, hooks | Git + source-of-truth hierarchy |
| Fleet | Deployment topology | nodes, Tailscale, services | `infra/registry.json` |
| Consumer project | External or downstream repo | `privacy-ethereum-framework` | consumer contract profile |
| Superorganism | Future inter-organism coordination | DAOs, external agents, protocols | explicit protocol contract only |

## 3. Layers

| Layer | Responsibility | Must not contain |
|-------|----------------|------------------|
| Semantic contract | Meaning of operations and events | Slack channels, local paths, shell commands |
| Policy contract | Permissions, privacy, identity, scope | transport formatting, retry loops |
| Kernel contract | Persistence, queries, claims, tasks, observations | social delivery decisions |
| MCP surface | Portable tool exposure and validation | project-specific hardcoded routing |
| Runtime adapter | Concrete transport or agent runtime | canonical semantics, ownership decisions |
| Deployment | systemd, service placement, env wiring | business rules, semantic policy |
| Observability | events, health, audit, compliance | hidden state mutation |

Rule: a lower layer may implement a higher-layer rule, but it must not invent
one.

## 4. Policy Dimensions

| Dimension | Question answered | Primary source | Enforcement |
|-----------|-------------------|----------------|-------------|
| Layer precedence | Which layer is allowed to decide? | `docs/foundation/CYNIC_META_LAYER_CONTRACTS.md` + `axioms/meta-layer-contract-v1.json` | review, topology, contract tests |
| Multi-scale projection | How do organism properties project across scales? | `docs/foundation/CYNIC_MULTI_SCALE_EMPIRICAL_RULE.md` + `axioms/multiscale-organism-property-map-v1.json` | empirical evidence, review, runtime checks |
| Source of truth | Where is truth anchored? | `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md` | review, audit, protocol tests |
| Repo topology | Where does each concern live? | `docs/foundation/CYNIC_REPO_TOPOLOGY_POLICY.md` + `axioms/repo-topology-v1.json` | topology validator, review, gate-0 target |
| Theme topology | Which domain owns TALARIA/CYNIC/Odysseum/runtime/data claims? | `docs/foundation/CYNIC_THEME_TOPOLOGY_POLICY.md` + `axioms/organism-theme-topology-v1.json` | topology validator, foundation index, review |
| Organ layer topology | Which primary layer and rank owns each organ? | `axioms/organ-layer-topology-v1.json` | layer validator, topology review |
| Identity | Who is acting? | `AGENTS.md`, registry, service identity | `/coord/register`, service table, audit |
| Permissions | What may this actor do? | policy contract + exposure manifest | MCP validation, REST auth, deny by default |
| Coordination | How do agents avoid collisions? | `AGENTS.md`, `/coord/*` | claims, pre-commit, task targets |
| Memory | What survives a session? | Kernel mempool + committed source | `/agent-tasks`, `/observe`, git |
| Privacy | What can leave local context? | policy contract + consumer profile | sensitivity flags, local-first defaults |
| Transport | How is information delivered? | adapter config | adapter tests, routing table |
| Network/IPAM | How nodes and services become reachable? | `infra/registry.json:network_policy` + `axioms/infrastructure-mcp-v1.json` | registry validation, `cynic-node` evidence, `organ-fleet` tests, MCP preflight |
| Execution | How does work run? | organ contracts + runtime ports | daemon lifecycle, tests, systemd |
| Supply chain | How builds/deploys happen | `infra/registry.json` | lifecycle dispatch, gates |
| Observability | How violations are seen? | health/audit/compliance contract | `/health`, observations, completed read-back |
| Exception | How temporary fallback is allowed? | this matrix + explicit expiry | issue/task, owner, removal condition |

## 5. Reference Models

CYNIC uses external infrastructure standards as reference models, not as
branding. When policy is ambiguous, map the concern to these models before
adding tools or files:

| Reference model | CYNIC use | Typical cloud equivalent |
|-----------------|-----------|--------------------------|
| OSI / TCP-IP | Distinguish link, routing, transport, service and application concerns | VPC/VNet, subnets, route tables, security groups, load balancers |
| Landing Zone | Establish baseline topology, identity, audit, secrets and guardrails | AWS Landing Zone, Azure Landing Zone, Google Cloud foundation |
| IAM / least privilege | Separate human, cortex, organ, dog and service identities | IAM roles, service accounts, managed identities |
| IaC desired state | Treat infra as declared state plus convergence evidence | Terraform/OpenTofu, CloudFormation, Bicep, Pulumi |
| Controller reconcile loop | Compare desired, observed and effective state continuously | Kubernetes controllers, operators, CRDs/status conditions |
| SRE | Make failures actionable and reduce toil | SLI/SLO, runbooks, postmortems, error budgets |
| Zero Trust | Mesh-only, explicit permissions, no implicit LAN trust | NIST 800-207, private endpoints, BeyondCorp-style access |
| Observability | Structured evidence for every mutation and health state | OpenTelemetry, audit logs, health/readiness probes |

For infrastructure, this means raw provider tools are not the canonical agent
interface. Agents consume semantic MCP operations from
`axioms/infrastructure-mcp-v1.json`; provider adapters implement those
operations underneath.

## 6. Non-Negotiable Separation Rules

1. Slack is a transport adapter, not a coordination model.
2. REST is a transport fallback, not a weaker semantic contract.
3. MCP is a portable client surface, not the organism source of truth.
4. The Kernel is the bus and memory, not legal/protocol/market truth.
5. A script is acceptable only as an adapter, diagnostic or operator wrapper.
   It is not allowed to become canonical policy.
6. Project routing belongs to project/channel/thread mapping, not to proxy
   code, prompts or agent-local config.
7. Repo-affecting agents must have declared targets before execution.
8. Persistent agents such as Hermes must satisfy the same coordination
   guarantees as cortex clients, even if they use REST instead of MCP.
9. Runtime state must not be committed as durable source unless explicitly
   classified as source, fixture or operator contract.
10. No "temporary" fallback is valid without an owner, expiry condition and
    replacement path.
11. Human-supervised cortex agents and autonomous organic agents are different
    authority classes. The boundary is `axioms/agent-autonomy-boundary-v1.json`.

## 7. Client Classes

| Class | Examples | Interface | Required guarantees |
|-------|----------|-----------|---------------------|
| Interactive cortex | Codex, Claude, Antigravity with human in the loop | MCP first, REST fallback | register, read attention, claim before edit, observe material facts, release |
| Supervised cortex delegate | bounded spawned cortex task | MCP or REST through parent context | declared targets, parent intent, return result to parent/proposal queue |
| Autonomous organ agent | Hermes Agent, MCP freshness controller | executor-owned REST or future MCP | claim task, register runtime agent, claim declared targets, release in finally path, emit proposals/proofs rather than direct DNA mutation |
| Service actuator | build runner, deploy actuator, provider adapter | Kernel REST + local/provider ports | execute approved/registry-authorized work, emit proof, no product intent invention |
| Organ service | Slack, Twitter, Forge | Kernel REST + local ports | bounded scope, local state discipline, health, observations |
| Dog | deterministic or LLM validator | Kernel-invoked scoring | stateless scoring, no repo writes, bounded confidence |
| Consumer repo | privacy framework and future repos | MCP coordination contract | project-scoped operations, repo-first artifacts, no Slack dependency |

Hermes is not exempt because it is persistent. The executor is its adapter, and
the adapter must provide the same safety boundary that an interactive cortex
provides manually through MCP.

MCP freshness is autonomous maintenance, not cortex preference management. A
new REST API, MCP source change or `infra/agent-exposure.json` capability change
should become an `autonomous_organ_agent` task that dispatches the registry
`mcp-reload` profile and proves freshness. It must not silently grant itself
public/operator authority beyond the declared exposure contract.

When the MCP/Kernel/build-runner path is itself broken, this rule degrades to a
cortex-local repair build. The repair build may restore local tooling and prove
patches, but it is not release validation, not deploy authority, and not a
substitute for the registry executor once the bus is reachable again.

## 8. Canonical File Ownership

| Concern | Canonical file or family | Notes |
|---------|--------------------------|-------|
| Agent session protocol | `AGENTS.md` | Operational entrypoint for agents |
| Agent autonomy boundary | `axioms/agent-autonomy-boundary-v1.json` | Human-supervised cortex vs autonomous organ agent authority |
| Organism policy matrix | this file | Cross-cutting policy authority |
| Source-of-truth hierarchy | `docs/foundation/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md` | Truth anchoring |
| Theme topology | `axioms/organism-theme-topology-v1.json` | TALARIA/CYNIC/Odysseum/runtime/data thematic ownership |
| Coordination semantics | `axioms/coordination-protocol-v1.json` | Transport-agnostic payloads |
| Infrastructure semantics | `axioms/infrastructure-mcp-v1.json` | Deterministic infra/network MCP operations |
| Repo topology | `axioms/repo-topology-v1.json` | Machine-readable filesystem concern boundaries |
| Organ layer topology | `axioms/organ-layer-topology-v1.json` | Primary layer/rank inventory for every immediate organ |
| Coordination architecture | `docs/architecture/COORDINATION-MCP-FOUNDATION.md` | Current implementation map |
| Agent exposure | `infra/agent-exposure.json` | MCP exposure source of truth |
| Runtime placement | `infra/registry.json` | Fleet and lifecycle source |
| Hermes executor contract | `docs/organs/HERMES_AGENT.md` | Persistent-agent adapter contract |
| Kernel APIs | `crates/cynic-kernel/` | Memory, coordination, health, storage |
| MCP proxy | `crates/cynic-mcp/` | Portable tool surface |
| Python persistent-agent runtimes | `services/{coordination,observability,reliability}-runtime/src/` | declared runtime membranes consumed by organs and Hermes surfaces |

If a concern appears in two places, the lower-level file may only project or
implement the higher-level rule. It must not redefine it.

## 9. Fallback Policy

A fallback is allowed only when all fields are explicit:

```json
{
  "fallback": "operator wrapper or degraded transport",
  "reason": "canonical layer unavailable",
  "owner": "human or organ",
  "scope": "exact files/services/projects affected",
  "expires_when": "test passes, service is restored, or registry path exists",
  "observability": "health/read-back/audit proving it was used",
  "replacement": "canonical path to restore"
}
```

Forbidden fallback patterns:

- hardcoded local paths in portable proxies;
- Slack channel IDs as semantic identifiers;
- direct shell deploy presented as canonical supply chain;
- local repair build presented as registry release validation;
- raw provider operation presented as canonical infra lifecycle;
- manually selected LAN or Proxmox guest address presented as service contract;
- prompt-only coordination for repo-affecting agents;
- runtime JSON state committed as organism memory;
- hidden env requirements not declared by service contract.

## 10. Review Checklist

Before adding or changing an organ, MCP tool, adapter, script or service:

1. Identify the scale.
2. Identify the policy dimension.
3. Identify the semantic contract.
4. Identify the transport adapter.
5. Identify the source of configuration.
6. Identify privacy/sensitivity handling.
7. Identify the mechanical enforcement.
8. Identify observability and read-back.
9. Identify fallback status, if any.
10. Delete or quarantine obsolete paths instead of leaving competing truths.

This checklist is mandatory for foundational components: kernel, MCP, registry,
agent exposure, Hermes executor, organ-forge, deployment and coordination.

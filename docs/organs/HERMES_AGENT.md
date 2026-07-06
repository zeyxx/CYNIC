# CYNIC — Hermes Agent Adapter

Hermes Agent is the default agentic execution framework for CYNIC organ work.
It is an organic persistent agent runtime, not a cortex CLI. It is launched by
systemd through `infra/systemd/hermes-agent-executor.service` and executed via
the new Ouroboros lifecycle module at
`services/ouroboros-runtime/src/ouroboros_runtime/core/daemon.py`.

Default does not mean coupled. CYNIC consumes Hermes through contracts,
capability scans, process boundaries, and adapters. CYNIC core, organ core
logic, policy, coordination, memory, evidence, and human gates remain
framework-independent.

## Default Execution Policy

When an organ needs autonomous agentic execution, the default route candidate is
`NousResearch/hermes-agent` if all of the following are true:

- the required capability is observed or declared in
  `cynic.agent_framework_capabilities.v1`;
- the task can be represented by `cynic.agent_execution_envelope.v1` or another
  declared CYNIC schema;
- the side-effect policy is compatible with the task risk;
- repo-affecting tasks declare coordination targets before launch;
- results can be normalized back into Kernel task results, observations, or
  evidence bundles.

Before implementing new agent runtime behavior in CYNIC, check whether Hermes
already provides the capability through CLI, gateway, ACP, MCP, scheduler,
subagents, toolsets, skills, browser/computer-use, terminal backends, memory, or
session search. Prefer adapting the existing Hermes surface over rebuilding it.

Fork policy:

- keep upstream `NousResearch/hermes-agent` as the source of truth by default;
- maintain CYNIC integration as adapters and contracts first;
- create a CYNIC fork only when an adapter cannot enforce sovereignty, policy,
  security, or required local runtime behavior;
- if a fork exists, it remains an execution substrate behind the same CYNIC
  contracts, not a new organism authority.

Anti-coupling rules:

- do not import Hermes internals into CYNIC core or organ core logic;
- do not make Hermes memory the organism memory;
- do not let Hermes toolsets decide coordination claims, policy, gates, or
  evidence redaction;
- do not commit Hermes runtime state, cookies, sessions, profiles, or generated
  skills into the repo;
- do not duplicate Hermes capabilities in CYNIC unless a contract, sovereignty,
  or safety requirement proves the adapter route is insufficient.

## Bloodstream Contract

Hermes execution is not allowed to remain a black box. Every CYNIC-owned Hermes
adapter must emit structured bloodstream events into the Kernel ledger while it
runs.

Minimum events:

- `task_received`
- `kanban_created` or equivalent upstream task/session id creation
- `decompose_attempted` when Hermes planning/delegation is invoked
- `kanban_status` or equivalent progress/status events during execution
- `kanban_completed` or a terminal non-success event
- `task_finished`

Minimum result shape:

```json
{
  "schema": "cynic.hermes.execution_result.v1",
  "task_id": "...",
  "domain": "...",
  "executor": "hermes-kanban",
  "hermes_kanban_task_id": "...",
  "status": "completed",
  "output": "..."
}
```

Downstream organs must consume this normalized result or Kernel observations,
not raw Hermes terminal prose. If an upstream Hermes feature cannot emit enough
evidence through the adapter, the task must be treated as integration debt
before it becomes cashflow-critical automation.

## Identity

Use role-based identities:

- Organ family: `organ-{role}`
- Hermes instance: `organ-{role}-hermes-agent`
- Runtime task agent id: `hermes-agent-{domain}-{task_id}` truncated to 64 characters

Examples:

- `organ-anvil-hermes-agent` for repo lifecycle work
- `hermes-agent-organ-anvil-<task_id>` for one executor task
- `hermes-agent-hermes-x-<task_id>` for one X/social task

## Required Task Contract

Any Hermes task that can modify this repo must declare its targets before execution. Accepted fields:

- `targets`
- `repo_targets`
- `coord_targets`
- `files`

Each target is a repo-relative file path or coordination zone. A task may declare up to 20 targets.

Repo-affecting tasks without declared targets are invalid and must be requeued or failed before `hermes chat` starts.

## Executor Lifecycle

For each task, the executor must:

1. Poll `/agent-tasks?status=pending`.
2. Merge JSON `content` into the task object if `content` is a JSON object.
3. Create the local task lock.
4. If repo-affecting, register `hermes-agent-{domain}-{task_id}` with `/coord/register`.
5. Claim declared repo targets with `/coord/claim-batch`.
6. Treat any returned conflict as a blocker, even though the kernel stores claims as signals.
7. Launch `hermes chat` only after claims are clean.
8. Release all repo claims with `/coord/release` in a `finally` path.
9. Release the local task lock according to task success/failure.

## Prompt Boundary

The executor, not the model prompt, owns repo coordination. The prompt may remind Hermes to touch only claimed files, but the deterministic boundary is before `hermes chat` starts.

## Kernel Endpoints

Use the REST API with `Authorization: Bearer $CYNIC_API_KEY`:

- `POST /coord/register`
- `POST /coord/claim-batch`
- `POST /coord/release`
- `GET /coord/who`

`/agents` is system health/status. `/coord/who` is the coordination view.

## Failure Rules

- Kernel unreachable + repo-affecting task: fail or requeue; do not edit blindly.
- Claim conflict: fail or requeue; do not edit blindly.
- Non-repo task: may proceed without repo claims.
- Missing auth: same as kernel unreachable for repo-affecting tasks.

## Scope

This adapter defines Hermes Agent execution and repo coordination. Hermes X
browsing, curation, and observation behavior remains governed by organ
`SKILL.md`, manifest files, `organ-browser`, and framework capability contracts.

## Framework Evolution & Mercury Alternative

L'écosystème autour de Hermes est en évolution continue. L'architecture
hexagonale (Ports & Adapters) de CYNIC (via `Ouroboros` et
`adapters/hermes_cli.py`) permet d'isoler le cœur organique de CYNIC des
changements très rapides upstream du CLI Hermes.

Hermes reste le framework agentique d'exécution par défaut tant qu'il couvre les
capacités requises via des contrats CYNIC. Les alternatives ne sont pas des
remplacements opportunistes ; elles sont des routes de fallback quand Hermes est
indisponible, insuffisant, trop coûteux, ou impossible à borner proprement.

### Alternative de Migration : Mercury Agent
Si un jour nous décidons de remplacer Hermes Agent pour certains organes, *Mercury Agent* (`cosmicstack-labs/mercury-agent`) est un excellent candidat. 
- **"Soul-driven"** : Il est nativement conçu pour tourner 24/7.
- **Écosystème TypeScript** : Utile pour se rapprocher de `packages/`.
- **Interopérabilité** : Le repo `mercury-agent-skills` propose un pont natif pour réutiliser des skills Hermes.

Grâce à notre refactoring hexagonal, ajouter Mercury comme fallback ne devrait
pas nécessiter de modifier le daemon (`ouroboros.py`). Il faudrait créer un
`adapters/mercury_cli.py` et le faire passer par les mêmes contrats d'exécution.

# CYNIC — Hermes Agent Adapter

Hermes Agent is an organic persistent agent, not a cortex CLI. It is launched by systemd through `infra/systemd/hermes-agent-executor.service` and executed via the new Ouroboros lifecycle module at `services/cynic-python/organs/core/daemon.py`.

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

This adapter defines Hermes Agent repo coordination only. Hermes X browsing, curation, and observation behavior remains governed by organ `SKILL.md` and manifest files.

## Framework Evolution & Mercury Alternative

L'écosystème autour de Hermes est en évolution continue (notamment via `hermes-agent-self-evolution` et `atropos`). L'architecture Hexagonale (Ports & Adapters) de CYNIC (via `Ouroboros` et `adapters/hermes_cli.py`) permet d'isoler le cœur organique de CYNIC des changements très rapides "upstream" du CLI de Hermes.

### Alternative de Migration : Mercury Agent
Si un jour nous décidons de remplacer Hermes Agent pour certains organes, *Mercury Agent* (`cosmicstack-labs/mercury-agent`) est un excellent candidat. 
- **"Soul-driven"** : Il est nativement conçu pour tourner 24/7.
- **Écosystème TypeScript** : Utile pour se rapprocher de `packages/`.
- **Interopérabilité** : Le repo `mercury-agent-skills` propose un pont natif pour réutiliser des skills Hermes.

Grâce à notre refactoring hexagonal, migrer de Hermes vers Mercury ne nécessiterait aucune modification du daemon (`ouroboros.py`). Il suffirait de créer un `adapters/mercury_cli.py`.

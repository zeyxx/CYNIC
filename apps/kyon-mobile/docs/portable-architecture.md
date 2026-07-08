# Kyon Portable Architecture Contract

This contract defines the minimum structure that must remain stable for portability.
If these invariants are respected, Kyon can be transplanted into a different repo
with low coupling and low entropy.

## 1) Core principle

Kyon is a Cortex-mobile implementation of a privacy event pipeline with:
- capture (observability + shield),
- policy (use cases),
- persistence,
- egress (sync).

The framework must never import infra details into business logic.

## 2) Fixed boundaries

- `core/*` contains domain models and pure interfaces (`ports`) only.
- `application/*` contains use cases and orchestration, no Android/IO details.
- `adapters/*` contains Android, network, and database integrations.
- `bootstrap/*` is the composition root.
- `ui/*` is a shell that delegates to application/use-case results.
- `services/*` and Android components stay thin and delegate to bootstrap outputs.

## 3) Portability invariants

### Invariant A — Ports-first
All external dependencies enter through `ports` interfaces.
No direct `Room`/`WorkManager`/`HTTP` call should appear in `application` code.

### Invariant B — Config is environment-owned
Kernel URLs, auth tokens, and feature flags are loaded from `SharedPreferences`
or injected settings, never hardcoded in class constants.

### Invariant C — One composition root
`bootstrap/KyonContainer` (or equivalent) builds every dependency graph.
All services/Workers/Activities use this root instead of manual wiring.

### Invariant D — Observability is first-class
Any external behavior (sync attempts, policy decisions, errors) must be routed
through an `ObservationSink`-style port.

### Invariant E — Testable at boundaries
Each use case in `application/*` must be instantiable with mocked ports
for unit tests.

## 4) Topology to keep in any repo

At minimum, the following folders are required:

```text
app/
  src/main/java/<base>/
    core/                 # entities, config records, abstractions
    ports/                # contracts
    application/          # use cases/orchestration
    adapters/             # concrete implementations
    bootstrap/            # composition root
    ui/                   # presentation shell
    mirror/               # capture domain
    shield/               # policy domain
    service/              # optional scheduled/background workers
```

## 5) Repository-level portability matrix

When moving to a new repo:

1. Keep package path and folder topology.
2. Keep `ports` and `application` with same API names or provide adapter shims.
3. Replace only these adapters:
   - storage adapters (`Room*` or equivalent),
   - sync adapter (`KernelObservationSink` replacement),
   - system integrations (`NotificationListener`, `CallScreening`, `SMS`),
   - sync worker implementation.
4. Keep `build.gradle.kts` dependencies explicit and minimal.
5. Keep app-level constants in env/settings files.

## 6) Non-goals

- This contract does not lock cryptography.
- This contract does not lock backend protocol.
- This contract does not lock notification UI or permission text.

Only topology, dependency direction, and injection discipline are locked.


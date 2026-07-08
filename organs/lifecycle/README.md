# organ-lifecycle

Bounded lifecycle organ for supply-chain analysis, runtime convergence, and local SSOT updates.

## Canonical surfaces

- `src/core/` contains registry-backed supply-chain analysis and runtime orchestration.
- `src/adapters/` contains filesystem and kernel transport adapters.
- `src/entrypoints/` contains the bounded CLI, runtime, and controller entrypoints.
- `systemd/` contains the deployment projection for the controller loop.

## Runtime contract

- `runtime-cycle` emits one observation from declared supply-chain truth.
- `controller.py` reads desired state, runs one convergence pass, and updates `state/SSOT.json`.
- The durable deployment projection lives in `infra/systemd/organ-lifecycle.service` and `infra/systemd/organ-lifecycle.timer`.

## Guardrails

- No cortex-local build is required for this organ.
- Registry truth is read through the registry port.
- The controller only updates local state under `state/`.

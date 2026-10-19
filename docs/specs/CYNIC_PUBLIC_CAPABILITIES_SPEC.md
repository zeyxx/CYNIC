# CYNIC Public Capabilities Spec — v0

> Citizens may consume CYNIC capabilities, but only through declared contracts.
> CYNIC exposes; citizens call. Authority remains runtime state.

## 1. Scope

Capabilities published by CYNIC for external projects:
- health and readiness probe
- registry snapshot (peers, pins, addresses)
- evidence ingestion (append-only)
- citizen capability lookup
- policy/config snapshots (non-secret)

## 2. Contract

- Versioned endpoints (REST), machine-readable JSON.
- WebSocket optional for event streams.
- gRPC/Yellowstone only where evidence or data plane requires it.
- Stable error shape with codes.

## 3. Authentication

- Scoped, revocable tokens (reference + capability, never value).
- Token scope named in `infra/registry.json` policy.
- No privileged surface without explicit approval and proof.

## 4. Rate and isolation

- Defaults conservative; higher limits only by evidence of demand.
- Read-before-write enforced.
- Quarantine on abuse.

## 5. First capability for projects

Start with read-only surfaces:
- `/health`, `/registry`, `/pins` snapshots.
- evidence append as opt-in after contract checks.

## 6. North star

First contact proves value on both sides before opening more.
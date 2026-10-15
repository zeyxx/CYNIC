# External Project Governance Spec — v0

> Status: foundation draft. Authority remains
> `contracts/foundation.json`, `infra/registry.json`, `infra/external-pins.json`.
> Organic first: observe before abstracting.

## 1. Purpose

Manage external projects and their stacks without coupling CYNIC to their runtime.
CYNIC consumes **observed outputs**, never owns their process, data or secrets.

## 2. Identity

- Every external project is pinned in `infra/external-pins.json`
  (slug, repo, url, sha, observed_at).
- Refresh is explicit and dated, never automatic.
- No live GitHub in loops.

## 3. Surfaces (machine-readable only)

A project may expose any subset of:
- REST (JSON, versioned)
- WebSocket (reconnect + backoff, heartbeat)
- RPC/gRPC (e.g. Solana Geyser, Yellowstone)
- files in a documented layout (optional, read-only)

CYNIC never requires a specific language. It requires a documented contract and
a way to read it.

## 4. Consumption contract

- CYNIC pulls by explicit poll or by event stream.
- Pull failures are degraded, never fatal.
- CYNIC records what it observed (hash, timestamp, source pin).

## 5. Secrets and keys

- External secrets never enter CYNIC repo or prompts.
- CYNIC holds references and capabilities, never values.
- If a project must call CYNIC, it uses a scoped token from `infra/registry.json`
  / secret store, revocable.

## 6. Health and quarantine

- A project starts as `observed` (read-only evidence).
- It graduates to `usable` only after contract checks and at least one successful
  exchange.
- It is quarantined on repeated contract violations or stale pin.

## 7. Anti-coupling ratchets

- No external runtime dependency in CYNIC crates.
- No direct DB/Redis/WS client baked into domain modules.
- Stack-specific code lives behind adapters or in the project itself.

## 8. Pending decisions

1. Which surfaces are mandatory for new projects.
2. Heartbeat cadence and quarantine thresholds.
3. How evidence from streams is folded into canon.
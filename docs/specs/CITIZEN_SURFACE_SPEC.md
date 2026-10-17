# Citizen Surface Spec — v0

> A citizen project is observable by CYNIC if it exposes these minimal
> machine-readable surfaces. Optional surfaces only emerge from evidence.

## 1. Minimum

- `GET /health` — liveness/readiness, JSON, stable fields `status`, `version`,
  `uptime_s`.
- `GET /v1/stats` — aggregate JSON (counts, last success/error, timestamps).
- Response headers include `X-Cynic-Citizen` slug and `X-Cynic-Surface-Version`.

## 2. Event surface (opt-in)

- WebSocket or SSE stream with reconnect and backoff.
- Envelope: `{"id","ts","kind","payload"}`; `kind` allowlist published by the
  project.
- CYNIC treats it as observed evidence, not command authority.

## 3. First contact protocol

1. Register pin in `infra/external-pins.json`.
2. First pull is read-only health check.
3. Graduation to `usable` requires one successful health + one stats read.
4. Failure keeps `observed`; repeated failure quarantines.

## 4. Rules

- No secrets in payloads.
- No language or stack assumption.
- Version header is required for contract drift detection.
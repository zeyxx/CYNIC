# Provider audit standard — know what connects CYNIC to its models

## Statut

Foundation rule. Inference providers are CYNIC's oxygen: every pool,
account, and credential path is audited with this grid, and the output
stays LOCAL (account emails are personal data — never committed to git).

## Sources of truth (read-only, secrets never printed)

- `~/.hermes/auth.json` → `credential_pool`: per-entry `last_status`,
  `last_error_*`, `failure_reason`, expiry fields, `request_count`.
- `~/.hermes/antigravity-accounts.json`: account roster.
- `~/.hermes/config.yaml`: which pool+model each role uses
  (`delegation.model`, fallbacks, `x_search.model`).

Audit tools print allowlisted metadata keys only. A tool that can print
a secret is not an audit tool.

## Verdict taxonomy (one per pool entry)

| Verdict | Rule |
|---|---|
| CONNECTED | `request_count > 0` (usage is the aliveness proof) |
| IDLE | entry present, `request_count == 0`, no failure recorded |
| EMPTY | pool exists with 0 entries — disconnected or never configured |
| DEGRADED | `failure_reason` or `last_error_reason` set (reason quoted) |
| EXPIRED | an `expires_at*` timestamp in the past |
| DUPLICATE | same identity usable through two pools/paths (name both) |

`rate_limit` alone is transient, not a disconnection — it is noted, not
verdict-changing.

## First conforming run (2026-10-18, manual)

- CONNECTED: `nous` oauth (348 req; agent key expires tonight 23:04 —
  refresh expected via `refresh_token`, re-run after),
  `openai-codex` Titouan (2 req), `antigravity` 4 oauth accounts
  (titouan.corneloup ×2, cynic@talaria.build, jeanterre9713) +
  env ACCESS_TOKEN (25 req), `opencode-go` (2 req).
- IDLE: `copilot` (0 req, never used), `openai-codex` ×2 spare entries
  (0 req), `agy` + `gemini-oauth` (same env token, 0 req — DUPLICATE
  of the antigravity env path).
- EMPTY (6): `custom:cynic-local` (yet referenced in config.yaml —
  dead path), `cloudcode`, `google-antigravity`, `google-code-assist`,
  `google-oauth`, `custom:antigravity-proxy`.
- No EXPIRED at run time; no DEGRADED beyond transient rate_limits.

## Standing cleanups (operator-gated, one HITL each)

1. `custom:cynic-local`: referenced but empty — configure or remove
   the reference.
2. `openai-codex` ×2 spare entries: keep or prune.
3. `agy` / `gemini-oauth` vs `antigravity`: one path per identity.
4. `google-*` + `cloudcode` empties: delete if superseded by antigravity.
5. Re-run after 23:04 to confirm the `nous` refresh.

## Cadence

On demand, and mandatory before changing any role's model
(`delegation.model` included): no model switch on an unaudited pool.

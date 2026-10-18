# Model metrology — measure, then route

## Statut

Spec. CYNIC routes each task to the fittest substrate (subscription,
free, local fleet) on measured evidence, never on opinion. This is the
metabolic loop of the scientific engine: sessions become knowledge,
runs become allocation.

## Metric catalog (all recorded per run)

| Metric | How | Why it matters |
|---|---|---|
| `accuracy` | exact-match vs adjudicated oracle, or judge score | fitness per task |
| `latency_s` | wall time, p50/p99 over repeats | agent iteration speed |
| `prompt_tokens` / `completion_tokens` | proxy `usage` field | real cost driver |
| `quota_share` | % of weekly subscription quota per task | subscriptions cost quota, not $ |
| `availability` | rate-limit / error events with `retry_after` | a blocked model is infinitely slow |
| `variance` | dispersion over 3 repeats, same prompt | agents need determinism |
| `french_quality` | explicit score on French outputs | non-negotiable here |
| `model_pin` | exact model ID + date of every measure | IDs drift, deprecate, change |
| `judge_cost` | tokens spent measuring quality | measuring costs too |

## Battery rules (learned from calibration v0/v1, 2026-10-19)

- B1. Every task has a machine-checkable oracle. Labels are
  double-adjudicated: calibration v1 showed the oracle itself can be
  wrong (t10 `unknown` vs `redirect` — genuinely ambiguous).
- B2. Calls go through `hermes proxy` (direct model HTTP), never
  agent-mode: agent scaffolding measured >5min on a trivial
  classification (v0 TIMEOUT), proxy measures the model at 2–5s (v1).
  Agent overhead is a separate metric, measured separately.
- B3. Model IDs are validated against the proxy before any matrix run:
  rate-limit error messages suggest alternates that 404
  (`qwen/qwen3.8-27b`, `deepseek/deepseek-v4-flash-0731`).
- B4. Free tier first: calibration burns no subscription quota.
  Subscription legs need explicit HITL (they spend weekly %).
- B5. Re-measure monthly: models deprecate, quotas reset, behavior drifts.
- B6. Every metered call logs full `usage` (prompt/completion/reasoning
  tokens) to the consumption ledger (`kanban/evidence/model_consumption.jsonl`,
  local only). Fixed fees are known ($10 Go, $4.99 AI Plus); our own
  token flow is what makes unit cost computable. Probe prompts keep
  `max_tokens` high enough not to truncate reasoning (v1 seed hit the
  ceiling at 60 — blind spot noted, not hidden).

## Artifacts

- Measures: append-only JSONL, local only (vault/captures, never git).
- Routing table `infra/model-routing.json`: task family → ranked
  (model, accuracy, p50, quota_share). Versioned, PR-gated.
- Calibration probes live in `/tmp`, never committed; the committed
  runner (Rust, later) follows this spec.

## Calibration v1 (2026-10-19, classification 10 turns, proxy direct)

- `meituan/longcat-2.5-preview:free`: **9/10, p50 2s**, 495 in / 22 out
  tok for 10 calls, 0 errors. Only miss is the ambiguous t10.
- `stepfun/step-5-preview:free`: 429 fair-share, retry_after 398s
  (availability metric in action).
- Suggested alternates 404 (see B3).

## Standing questions for the matrix

1. Second task family with test oracle (code micro-fix) — same harness.
2. Subscription legs (antigravity 100% weekly, codex under pressure):
   run only after HITL, tiny battery first.
3. Local fleet leg: blocked on RAM (13G, 11 used) — small GGUF only,
   or fleet nodes with headroom.

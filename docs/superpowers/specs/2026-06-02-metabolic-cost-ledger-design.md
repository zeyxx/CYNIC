# Metabolic Cost Ledger — Design Spec
**Date:** 2026-06-02  
**Status:** Approved  
**Reviewed by:** Claude (primary) + Gemini CLI (independent validator, GROWL → schema revisions)  
**Confidence:** 0.618 (φ⁻¹) — design is sound, instrumentation coverage unproven until Phase 1 complete

---

## Problem

CYNIC has `estimated_cost_usd: 0.0` in `/health` — a placeholder that has never been filled. The organism cannot observe its own metabolic cost. Consequences:

- Human cannot make resource decisions based on evidence
- LLM Dogs cannot self-regulate (they don't know how expensive they are)
- Futarchy proposals have no cost anchor — voting on "increase curation frequency" without knowing what it currently costs is pure speculation
- Dollar-centrism is wrong: the real cost dimensions are tokens, sovereignty debt, and latency

---

## Non-Goals

- Energy (Wh) measurement — future, requires GPU power telemetry not yet available
- Dollar conversion — derived from tokens if needed, never primary
- Real-time dashboarding — ledger is append-only; aggregation is periodic

---

## Schema

Every cost event written to `~/.cynic/metabolism/cost_ledger.jsonl`:

```json
{
  "ts": "2026-06-02T14:01:00Z",
  "session_id": "cortex-abc123",
  "trace_id": "verdict-xyz789",
  "feature_id": "spike_detector",
  "component": "spike_detector",
  "operation": "fetch_ohlcv",
  "compute_class": "external_api",
  "tokens_in": 0,
  "tokens_out": 0,
  "latency_ms": 230,
  "provider": "geckoterminal"
}
```

### Field contracts

| Field | Type | Values | Purpose |
|-------|------|--------|---------|
| `ts` | ISO-8601 | — | Timestamp of the call |
| `session_id` | str | cortex branch name or `cron-<service>` | Who incurred this cost |
| `trace_id` | str\|null | verdict_id, task_id, or null | Causality chain — what triggered this cost |
| `feature_id` | str | `spike_detector`, `x_ingest`, `gemini_briefing`, `hermes_agent`, `kernel_infer`, `x_poster` | Feature-level aggregation for futarchy ROI |
| `component` | str | Phase 1: always equal to `feature_id`. Phase 2: may differ for nested sub-calls (e.g. hermes_agent calling kernel). | Instrumentation source |
| `operation` | str | `fetch`, `infer`, `judge`, `observe`, `search`, `post` | Operation type |
| `compute_class` | enum | `local_gpu` \| `local_cpu` \| `tailnet` \| `external_api` | Sovereignty gradient — objective, not a float |
| `tokens_in` | int | 0 for non-LLM | Input tokens consumed |
| `tokens_out` | int | 0 for non-LLM | Output tokens generated |
| `latency_ms` | int | — | Wall-clock time of the call |
| `provider` | str | `qwen36-27b-gpu`, `qwen25-7b-core`, `geckoterminal`, `helius`, `x-com`, `gemini-api` | Specific backend |

**`compute_class` rationale (vs Gemini's float sovereignty_score):**  
Gemini proposed `[0.0-0.618]` float — rejected as requiring subjective calibration with no clear falsification test. The enum is objective: `local_gpu` = hardware you own, `tailnet` = sovereign network but remote machine, `external_api` = third-party dependency. A float can always be disputed; an enum is a fact.

---

## Architecture

```
Python callers                    Storage                  Kernel
─────────────────                 ──────────────────────   ──────────────
spike_detector.py   ─emit──►      cost_ledger.jsonl        /health
x_ingest_daemon.py  ─emit──►      (append-only)     ─────► estimated_cost
gemini_briefing.py  ─emit──►           │                   (real value)
hermes_agent.py     ─emit──►           │ flush every 30min
kernel (Rust)       ─emit──►      cost_aggregator.py ─POST► /observe
                                       │              domain=metabolism
                                  cost_summary CLI
                                  (human-readable)
```

---

## Components

### 1. `cynic-python/metabolism/cost_tracker.py`

The emitter. Single function all callers import:

```python
def emit(
    feature_id: str,
    operation: str,
    compute_class: str,       # "local_gpu"|"local_cpu"|"tailnet"|"external_api"
    provider: str,
    latency_ms: int,
    tokens_in: int = 0,
    tokens_out: int = 0,
    trace_id: str | None = None,
) -> None
```

- Writes to `~/.cynic/metabolism/cost_ledger.jsonl` (append)
- Reads `CYNIC_SESSION_ID` from env (set by session-init.sh) for `session_id`
- Never throws — all errors are logged and swallowed (cost tracking must not break callers)
- Thread-safe append via `fcntl.flock`

### 2. Instrumentation points (5)

| Caller | What to wrap | Sovereign? |
|--------|-------------|-----------|
| `spike_detector.py` | GeckoTerminal HTTP calls | `external_api` |
| `x_ingest_daemon.py` | X.com dataset fetch loop | `external_api` |
| `gemini_briefing_consumer.py` | Gemini subprocess calls | `external_api` |
| `hermes_agent_task_executor.py` | `subprocess.run(["hermes", ...])` | `tailnet` |
| Kernel Rust `/judge` handler | Surface `response.usage` from llama-server | `local_gpu` or `local_cpu` |

For the Rust kernel: llama-server already returns `usage.prompt_tokens` and `usage.completion_tokens` in its OpenAI-compat response. The kernel must read these and emit a cost observation via `/observe domain=metabolism` internally.

### 3. `cynic-python/metabolism/cost_aggregator.py`

Flush daemon — runs every 30min via systemd timer.

- Reads `cost_ledger.jsonl` from a cursor position (tracks offset)
- Aggregates new events: total tokens by provider, sovereign_call_count by provider, P50/P99 latency by feature_id
- POSTs to kernel `/observe domain=metabolism`:

```json
{
  "tool": "cost_aggregator",
  "domain": "metabolism",
  "context": "tokens_in:4120 tokens_out:890 sovereign_calls:47 [geckoterminal:23,helius:18,x-com:6] p50_latency_ms:380 p99_latency_ms:4800",
  "tags": ["cost-flush", "session-abc123"]
}
```

### 4. `cynic-python/metabolism/cost_summary.py`

CLI for human visibility:

```bash
python3 cost_summary.py --since 24h
python3 cost_summary.py --session cortex-abc123
python3 cost_summary.py --feature spike_detector --since 7d
```

Output:
```
Session cortex-abc123 (2026-06-02)
  Tokens:    in=12,450  out=2,890  total=15,340
  Sovereign: geckoterminal×47  helius×23  x-com×12  gemini×3
  Latency:   P50=380ms  P99=4,800ms
  Top cost:  kernel_infer 89% of tokens | spike_detector 71% of sovereign calls
```

---

## K15 Compliance

**Phase 1 consumer:** `cost_aggregator.py` flushes to kernel `/observe domain=metabolism` — this updates `/health.estimated_cost_usd` and metabolism section with real data. The human can now make resource decisions.

**Phase 2 consumer (Metabolic Governor — NOT in this spec):**  
If `tokens_per_verdict` exceeds threshold → kernel auto-downgrades dog tier (27B → 7B). This is the "teeth" Gemini identified. Deferred until 14+ days of cost data exist to set meaningful thresholds. Premature governance = wrong thresholds = broken organism.

**Falsification for K15:** After Phase 1 deploy, `/health.estimated_cost_usd` must be non-zero within 30min. If still 0.0 → consumer not firing.

---

## Data Retention

- `cost_ledger.jsonl` rotates weekly (keep 4 weeks rolling) — handled by logrotate config
- Kernel observations persist in SurrealDB with normal TTL
- Futarchy can query `/observations?domain=metabolism&feature_id=spike_detector&since=14d` for ROI calculation

---

## Phase Plan

**Phase 1 (this spec):**
1. `cost_tracker.py` — emitter module
2. Instrument 4 Python callers (spike_detector, x_ingest, gemini_briefing, hermes_agent)
3. `cost_aggregator.py` + systemd timer
4. `cost_summary.py` CLI
5. Kernel Rust: surface `response.usage` from llama-server responses

**Phase 2 (future, separate spec):**
- Metabolic Governor: threshold-based dog tier auto-selection
- Per-feature ROI for futarchy proposals
- Energy (Wh) via GPU power telemetry

---

## Open Questions

1. `CYNIC_SESSION_ID` — currently not set by session-init.sh. Needs to be injected or derived from git branch name at emit time.
2. Kernel Rust change scope — surfacing `response.usage` may require a small PR to the inference pipeline. Needs rust-guardian review.
3. Logrotate config — does it already exist for `.cynic/` paths? To verify before deploy.

---
name: stats
description: "Display CYNIC telemetry dashboard: usage stats, latencies, frictions, patterns"
---

# /stats - Telemetry Dashboard

Display real-time CYNIC telemetry and usage statistics.

## Instructions

1. Load telemetry data from the TelemetryCollector
2. Display as a formatted TUI dashboard
3. Highlight any frictions or performance issues
4. Show trends if historical data is available

## Dashboard Format

```
═══════════════════════════════════════════════════════════════
📊 CYNIC TELEMETRY - "φ mesure tout"
═══════════════════════════════════════════════════════════════

── SESSION ────────────────────────────────────────────────────
   ID: {session_id}
   Uptime: {uptime}s
   Actions: {action_count}

── EVENTS BY CATEGORY ─────────────────────────────────────────
   {category}: {bar} {count}
   ...

── LATENCY (ms) ───────────────────────────────────────────────
   {metric}          avg:{avg} p95:{p95} p99:{p99}
   ...

── FRICTIONS ──────────────────────────────────────────────────
   Total: {count}
   Recent:
   {time} [{severity}] {name}
   ...

── LLM USAGE ──────────────────────────────────────────────────
   Calls: {total}
   Tokens: {input_tokens} in / {output_tokens} out
   Cache hits: {cache_hits}%
   Avg latency: {avg_latency}ms

── JUDGMENTS ──────────────────────────────────────────────────
   Total: {count}
   By verdict:
     HOWL: {count}  WAG: {count}  GROWL: {count}  BARK: {count}
   Avg Q-Score: {avg_score}
   Avg confidence: {avg_confidence}%

── SYSTEM ─────────────────────────────────────────────────────
   Memory: {heap_used}MB / {heap_total}MB
   CPU: {cpu_usage}%

═══════════════════════════════════════════════════════════════
```

## Data Sources

- `getTelemetry()` from `@cynic/persistence/services`
- `telemetry.export()` for full data
- `telemetry.getStats()` for summary

## Output

Display the dashboard in the console with proper formatting and colors based on CYNIC TUI protocol.

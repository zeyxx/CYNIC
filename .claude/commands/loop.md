---
name: loop
description: CYNIC maintenance loop — health check, dream debt, test-chess if Dogs available, report delta.
disable-model-invocation: true
context: fork
model: haiku
allowed-tools: Bash(curl *) Bash(source *)
---

CYNIC maintenance loop. Run each step, report findings.

First: `source ~/.cynic-env 2>/dev/null`

## 1. Health Check

```
curl -s -H "Authorization: Bearer ${CYNIC_API_KEY}" "http://${CYNIC_REST_ADDR}/health"
```

Extract and report:
- Status (sovereign/degraded/critical)
- Dogs: count + circuit state (closed/open/half-open)
- Storage: connected/down + metrics if available
- Embedding: sovereign/degraded/null
- Alerts: list any active
- Uptime, total requests, verdict cache size

## 2. Dream Debt

Check dream session counter:
```
cat "$CLAUDE_PROJECT_DIR/.dream-state" 2>/dev/null || echo "0"
```

If >= 5: flag for /dream consolidation.

## 3. Dog Quality (if Dogs available)

If status is sovereign or degraded with >= 2 dogs:

```
curl -s -H "Authorization: Bearer ${CYNIC_API_KEY}" "http://${CYNIC_REST_ADDR}/health" | python3 -c "
import sys, json
d = json.load(sys.stdin)
for q in d.get('organ_quality', []):
    print(f\"{q['dog']}: json_valid={q['json_valid_rate']:.0%} capability_limit={q['capability_limit_rate']:.0%} calls={q['total_calls']}\")
"
```

Flag any Dog with json_valid_rate < 0.8 or capability_limit_rate > 0.3.

## 4. Delta Report

One-line summary if all healthy:
```
LOOP OK — sovereign, N/M dogs, storage connected, dream debt X, no quality flags
```

Multi-line if issues found:
```
LOOP ALERT:
  ⚠ [issue 1]
  ⚠ [issue 2]
  Action: [suggested next step]
```

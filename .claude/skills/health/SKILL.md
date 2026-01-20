---
name: health
description: Display CYNIC system health dashboard. Use when asked about system status, health check, diagnostics, or to see if CYNIC services are running properly.
user-invocable: true
---

# /health - CYNIC System Health

*"A healthy dog is a happy dog"*

## Quick Start

```
/health
```

## What It Does

Shows comprehensive system status:
- **Node Status**: CYNIC node health
- **Judge Stats**: Judgment metrics
- **Collective**: The 11 Dogs status
- **Storage**: Database connectivity
- **PoJ Chain**: Blockchain integrity

## Dashboard Sections

### Core Services
| Service | What It Shows |
|---------|---------------|
| Node | Connection status, uptime |
| Judge | Judgments made, accuracy |
| Persistence | PostgreSQL/Redis status |

### The Collective (11 Dogs)
| Dog | Role | Status |
|-----|------|--------|
| Guardian | Protection | active/idle |
| Analyst | Analysis | active/idle |
| Scholar | Learning | active/idle |
| Architect | Design | active/idle |
| Sage | Wisdom | active/idle |
| ... | ... | ... |

### PoJ Chain
| Metric | Description |
|--------|-------------|
| Height | Current block number |
| Integrity | Chain verification |
| Pending | Judgments awaiting block |

## Implementation

Use the `brain_health` MCP tool:

```javascript
brain_health({
  verbose: true  // Include detailed stats
})
```

## Metrics Available

Additional metrics via `brain_metrics`:

```javascript
brain_metrics({
  action: "collect"  // Raw metrics
  // or "prometheus" for Prometheus format
  // or "html" for dashboard
})
```

## Quick Checks

| Check | Tool |
|-------|------|
| Overall health | `brain_health` |
| PoJ chain | `brain_poj_chain({ action: "status" })` |
| Collective | `brain_collective_status` |
| Learning | `brain_learning({ action: "state" })` |

## See Also

- `/trace` - Trace specific judgments
- `/patterns` - View detected patterns
- `/ecosystem` - Ecosystem status

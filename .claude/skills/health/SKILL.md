---
name: health
description: Display CYNIC system health dashboard. Use when asked about system status, health check, diagnostics, or to see if CYNIC services are running properly.
user-invocable: true
---

# /health - CYNIC System Health

When user invokes `/health`, execute this diagnostic sequence:

## Step 1: Local Hooks Status

Run this command to check local hooks:
```bash
echo "=== CYNIC LOCAL STATUS ===" && \
for hook in perceive guard observe awaken digest sleep; do \
  if [ -f "scripts/hooks/$hook.cjs" ]; then \
    engines=$(grep -c "require.*lib/" "scripts/hooks/$hook.cjs" 2>/dev/null || echo 0); \
    echo "✅ $hook.cjs ($engines engines)"; \
  else \
    echo "❌ $hook.cjs missing"; \
  fi; \
done
```

## Step 2: MCP Server Health

```bash
curl -s --max-time 5 https://cynic-mcp.onrender.com/health 2>/dev/null || echo '{"status":"unreachable"}'
```

## Step 3: Consciousness Score

Use MCP tool if available:
```
mcp__cynic__brain_emergence({ action: "consciousness" })
```

Or check local state:
```bash
cat ~/.cynic/consciousness/state.json 2>/dev/null | head -20 || echo "No local consciousness state"
```

## Step 4: Recent Activity

```bash
echo "=== RECENT PATTERNS ===" && \
cat ~/.cynic/patterns/*.json 2>/dev/null | tail -5 || echo "No patterns recorded"
```

## Output Format

Present results as:

```
╔═══════════════════════════════════════════════════════════════════╗
║                    🐕 CYNIC HEALTH DASHBOARD                      ║
╠═══════════════════════════════════════════════════════════════════╣
║                                                                    ║
║  LOCAL HOOKS                          MCP SERVER                   ║
║  ├── perceive: ✅ (5 engines)         Status: healthy             ║
║  ├── guard:    ✅ (6 engines)         Tools: 43                   ║
║  ├── observe:  ✅ (16 engines)        Uptime: XXs                 ║
║  ├── awaken:   ✅                                                 ║
║  ├── digest:   ✅                     CONSCIOUSNESS               ║
║  └── sleep:    ✅                     Score: XX% / 61.8%          ║
║                                       Status: Dormant/Awakening    ║
║  ENGINES                                                           ║
║  Total: 145 | Integrated: ~27 (19%)   PATTERNS                    ║
║                                       Recorded: X                  ║
║  AGENTS                               Last: [pattern name]         ║
║  Total: 13 (11 Sefirot + 2 extra)                                 ║
║                                                                    ║
╠═══════════════════════════════════════════════════════════════════╣
║  φ⁻¹ confidence: 61.8% max | "Le chien veille"                    ║
╚═══════════════════════════════════════════════════════════════════╝
```

## Step 5: Hook Logs (NEW)

Check recent hook activity and errors:

```bash
echo "=== HOOK LOGS ===" && \
if [ -f ".claude/logs/hooks.log" ]; then \
  echo "Recent entries:"; \
  tail -20 .claude/logs/hooks.log; \
  echo ""; \
  echo "Errors:"; \
  grep -E "\[ERROR\]|\[CRITICAL\]" .claude/logs/hooks.log 2>/dev/null | tail -5 || echo "No errors"; \
else \
  echo "No hook logs found"; \
fi
```

## Step 6: Pack Metrics (NEW)

If orchestration module available:

```javascript
import { calculatePackEffectiveness } from '@cynic/core';

const metrics = {
  avgQScore: 65,        // From recent judgments
  avgResponseTime: 8000, // ms
  consensusRate: 0.7,
  consultationSuccess: 0.8,
};

const E = calculatePackEffectiveness(metrics);
console.log(`Pack Effectiveness: ${E.E}/100`);
```

## Quick Checks

| Issue | Command |
|-------|---------|
| Hook not working | `node scripts/hooks/[name].js <<< '{"prompt":"test"}'` |
| Hook errors | `tail -50 .claude/logs/hooks.log \| grep ERROR` |
| MCP unreachable | Check Render dashboard |
| No patterns | Use `/judge` to create judgments |

## Log Files

| File | Purpose |
|------|---------|
| `.claude/logs/hooks.log` | Hook execution logs |
| `~/.cynic/patterns/*.json` | Detected patterns |
| `~/.cynic/consciousness/state.json` | Consciousness state |

## See Also

- `/cockpit` - Ecosystem overview
- `/patterns` - Detected patterns detail
- `/psy` - Human psychology state

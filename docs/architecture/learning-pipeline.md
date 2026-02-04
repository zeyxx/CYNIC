# CYNIC Learning Pipeline

> "φ apprend de tout, mais doute de ses propres connaissances"

The Learning Pipeline is CYNIC's adaptive system for improving Dog selection and judgment accuracy over time. All ratios are governed by φ (golden ratio).

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        LEARNING PIPELINE FLOW                            │
└─────────────────────────────────────────────────────────────────────────┘

USER ACTION (tool call)
    │
    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 1. OBSERVATION (hooks/observe.js)                                        │
│    └── ImplicitFeedbackDetector extracts signals                        │
│    └── HarmonicFeedbackSystem.processFeedback()                         │
│    └── ConsciousnessMonitor.observe()                                   │
└─────────────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 2. Q-LEARNING EPISODE                                                    │
│    └── startEpisode() - extract features from context                   │
│    └── recordAction() - track which Dog responded                       │
│    └── endEpisode() - calculate reward, update Q-table                  │
└─────────────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 3. PERSISTENCE (PostgreSQL)                                              │
│    └── qlearning_episodes - individual episode records                  │
│    └── qlearning_state - Q-table, exploration rate, stats               │
│    └── patterns - detected patterns with Fisher importance              │
└─────────────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 4. EWC++ CONSOLIDATION (session end)                                     │
│    └── Calculate Fisher importance scores                               │
│    └── Lock patterns where Fisher ≥ φ⁻¹ (61.8%)                         │
│    └── Prevent catastrophic forgetting                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

## φ-Governed Parameters

| Parameter | Value | Derivation |
|-----------|-------|------------|
| Max Confidence | 61.8% | φ⁻¹ |
| Learning Rate (α) | 61.8% | φ⁻¹ |
| Discount Factor (γ) | 38.2% | φ⁻² |
| Lock Threshold | 61.8% | φ⁻¹ |
| Prune Threshold | 23.6% | φ⁻³ |
| Exploration Decay | per episode | φ⁻³ |

## Q-Learning Algorithm

The Q-Learning update follows the Bellman equation:

```
Q(s,a) = Q(s,a) + α[r + γ·max(Q(s',a')) - Q(s,a)]

where:
  s = state (task features)
  a = action (Dog selected)
  r = reward (+1.0 success, -0.5 failure, +0.8 blocked)
  α = φ⁻¹ (learning rate)
  γ = φ⁻² (discount factor)
```

## State Features

The system extracts features from context to determine state:

| Feature Category | Examples |
|-----------------|----------|
| Task Type | TASK_SECURITY, TASK_CODE_CHANGE, TASK_ANALYSIS |
| Tool Used | TOOL_BASH, TOOL_WRITE, TOOL_READ |
| Context | CONTEXT_ERROR, CONTEXT_URGENT, CONTEXT_COMPLEX |

## API Reference

### QLearningService

```javascript
import { QLearningService } from '@cynic/node/orchestration/learning-service';

const qlearning = new QLearningService({
  persistence: pool,      // PostgreSQL pool
  serviceId: 'hooks',     // Service identifier
});

// Start an episode
const episodeId = qlearning.startEpisode({
  taskType: 'security',
  tool: 'Bash',
  content: 'rm -rf /',
});

// Record Dog actions
qlearning.recordAction('guardian', {
  success: true,
  blocked: true,
  decision: 'block',
});

// End episode with outcome
const result = qlearning.endEpisode({
  success: true,
  type: 'blocked',
});
```

### Hooks Integration

```javascript
// scripts/hooks/lib/index.js
import { getQLearningServiceWithPersistence } from './lib/index.js';

const qlearning = getQLearningServiceWithPersistence();

// For guaranteed persistence before continuing:
const qlearning = await getQLearningServiceWithPersistenceAsync();
```

## Database Schema

### qlearning_episodes

```sql
CREATE TABLE qlearning_episodes (
  episode_id TEXT PRIMARY KEY,
  service_id TEXT NOT NULL,
  features JSONB,
  actions JSONB,
  outcome JSONB,
  reward NUMERIC,
  duration_ms INTEGER,
  created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### qlearning_state

```sql
CREATE TABLE qlearning_state (
  service_id TEXT PRIMARY KEY,
  q_table JSONB,
  exploration_rate NUMERIC,
  stats JSONB,
  updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

## Monitoring

### Observatory Endpoints

- `GET /api/learning/proof` - Learning proof with 5 signals
- `GET /api/qlearning` - Q-Learning state and statistics
- `GET /metrics` - Prometheus metrics including `cynic_qlearning_episodes_total`

### Health Indicators

| Signal | Healthy | Warning | Critical |
|--------|---------|---------|----------|
| Episode Count | > 10 | 1-10 | 0 |
| Exploration Rate | Decreasing | Stable | Increasing |
| Q-Convergence | > 0.5 | 0.2-0.5 | < 0.2 |
| Pattern Retention | > φ⁻¹ | φ⁻²-φ⁻¹ | < φ⁻² |

## Testing

```bash
# Run learning pipeline tests
node --test packages/node/test/learning-pipeline.integration.test.js

# Run all learning-related tests
node --test packages/node/test/learning-*.test.js
```

## Troubleshooting

### Episodes not persisting

1. Check `CYNIC_DATABASE_URL` is set
2. Verify PostgreSQL connection
3. Check hooks load dotenv correctly

```javascript
// Debug: Check if pool is available
const { getPool } = require('@cynic/persistence');
const pool = getPool();
console.log('Pool available:', !!pool);
```

### Fisher scores not rising

- Patterns need repeated observations to increase frequency
- Fisher = (gradient)² over all episodes
- Minimum observations: 3 per pattern for significance

### Learning proof shows "not_learning"

- Need avg score ≥ φ⁻² (38.2%) for "stable"
- Need avg score ≥ φ⁻¹ (61.8%) for "learning"
- Check: reward trend, exploration decay, Q-convergence

---

*"Le chien apprend par résonance, pas par force."*

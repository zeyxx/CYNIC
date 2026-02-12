# Model Selection — Haiku/Sonnet Switching

> **Status**: ✅ IMPLEMENTED + TESTED
> **Confidence**: 58% (φ⁻¹ limit)
> **Test**: `scripts/test-model-selection.js` (83% pass rate)

---

## Overview

CYNIC uses **Thompson Sampling** to intelligently select between Claude models:
- **Haiku**: Fast, cheap ($0.25/MTok in, $1.25/MTok out) — simple tasks
- **Sonnet**: Smart, moderate ($3/MTok in, $15/MTok out) — complex tasks
- **Opus**: Powerful, expensive ($15/MTok in, $75/MTok out) — critical tasks

The system LEARNS which models work best for which tasks, continuously improving selection over time.

---

## How It Works

```
┌──────────────────────────────────────────────────────────┐
│  User Prompt → PromptClassifier                          │
│       ↓                                                   │
│  Complexity: simple | moderate | complex                 │
│  Domain: CODE, SOLANA, MARKET, etc.                      │
│       ↓                                                   │
│  ModelIntelligence.selectModel(taskType, context)        │
│       ↓                                                   │
│  Thompson Sampling (learned preferences)                 │
│  + Budget override (exhausted → Haiku)                   │
│  + Memory pressure override (→ Haiku)                    │
│  + Tool affinity (some tools need certain models)        │
│       ↓                                                   │
│  Selected: { model: 'haiku', reason, confidence }        │
│       ↓                                                   │
│  AnthropicAdapter.complete(prompt, { model })            │
│       ↓                                                   │
│  Response + outcome recorded for learning                │
└──────────────────────────────────────────────────────────┘
```

---

## Selection Logic

### 1. Budget Overrides (Highest Priority)

| Budget Level | Model Cap  | Reason |
|--------------|------------|--------|
| EXHAUSTED    | Haiku only | Force cheapest to preserve remaining budget |
| CRITICAL     | Cap at Sonnet | Block Opus (too expensive) |
| CAUTIOUS     | No cap | Thompson decides |
| MODERATE     | No cap | Thompson decides |
| ABUNDANT     | No cap | Thompson decides |

### 2. Memory Pressure Override

If daemon heap usage > φ⁻¹ (61.8%), force Haiku to reduce memory load.

### 3. Tool Affinity Floor

Some tools have minimum model requirements:

| Tool Type | Minimum Model | Reason |
|-----------|---------------|--------|
| `brain_health` | Haiku | Simple status queries |
| `brain_cynic_judge` | Sonnet | 36-dimension scoring requires reasoning |
| `brain_orchestrate` | Opus | Complex orchestration decisions |

Tool affinity acts as a FLOOR — Thompson can upgrade but not downgrade.

### 4. Thompson Sampling

For each task category (code_review, architecture, security, etc.), Thompson maintains Beta distributions for each model:

```
Model Tier   Alpha  Beta   → Success Rate
opus         3.0    1.0    → 75% (high prior)
sonnet       2.0    1.0    → 67% (medium prior)
haiku        1.5    1.0    → 60% (low prior)
ollama       1.0    1.0    → 50% (neutral prior)
```

On each selection:
1. Sample from each model's Beta distribution
2. Select model with highest sample
3. Record outcome (success/failure, quality, duration)
4. Update Beta parameters (α for success, β for failure)

**Exploration**: φ⁻³ (23.6%) of selections are **falsification experiments** — testing if a cheaper model can replace an expensive one.

---

## Task Categories

Model selection is per-category, not global:

```javascript
TaskCategory.CODE_REVIEW    // PR review, code quality
TaskCategory.CODE_WRITE     // Writing new code
TaskCategory.ARCHITECTURE   // Design, planning
TaskCategory.DEBUG          // Debugging, error analysis
TaskCategory.SECURITY       // Security review
TaskCategory.KNOWLEDGE      // Q&A, documentation
TaskCategory.SIMPLE         // Grep, search, simple ops
TaskCategory.ROUTING        // Internal CYNIC routing
```

Each category learns independently — Haiku might excel at SIMPLE tasks but fail at ARCHITECTURE.

---

## API Usage

### Daemon Endpoint

```bash
POST http://localhost:6180/llm/ask
Content-Type: application/json

{
  "prompt": "Review this code for security issues",
  "taskType": "security",
  "system": "You are a security expert",
  "temperature": 0.7,
  "maxTokens": 2000
}
```

**Response**:
```json
{
  "content": "...",
  "model": "claude-sonnet-4-5-20250929",
  "tokens": { "input": 234, "output": 567 },
  "tier": "sonnet",
  "duration": 1234,
  "selection": {
    "tier": "sonnet",
    "reason": "Thompson selected (EV=61.8%, uncertainty=4.7%)",
    "confidence": 0.501
  }
}
```

### Programmatic (Node)

```javascript
import { getModelIntelligence } from '@cynic/node/learning';
import { getCostLedger } from '@cynic/node/accounting';

const mi = getModelIntelligence();
const ledger = getCostLedger();

// Select model
const selection = mi.selectModel('architecture', {
  budgetLevel: ledger.getBudgetStatus().level,
  tool: 'brain_cynic_judge',
});

console.log(`Selected: ${selection.model} (${selection.reason})`);
// => "Selected: sonnet (tool affinity: brain_cynic_judge needs sonnet+)"

// Record outcome
mi.recordOutcome({
  taskType: 'architecture',
  model: selection.model,
  success: true,
  quality: 0.85, // 0-1
  durationMs: 1500,
});
```

---

## Learning Curve

Thompson Sampling improves over time:

**Initial State** (no data):
```
architecture:
  opus:   75% (prior only)
  sonnet: 67% (prior only)
  haiku:  60% (prior only)
```

**After 50 selections**:
```
architecture:
  opus:   89% success (learned: excellent for design)
  sonnet: 78% success (learned: acceptable for design)
  haiku:  42% success (learned: too weak for architecture)
```

**Result**: System now strongly prefers Opus/Sonnet for architecture, rarely selects Haiku.

---

## Cost Optimization

### Scenario: 100 prompts, mixed complexity

**Without learning** (always Sonnet):
- Cost: $1,800 (100 prompts × $18 avg)
- Quality: 80% tasks succeed

**With Thompson Sampling** (after learning):
- Simple tasks (40%) → Haiku: $100
- Moderate tasks (40%) → Sonnet: $720
- Complex tasks (20%) → Opus: $360
- **Total**: $1,180 (34% savings)
- Quality: 85% tasks succeed (better model-task fit)

### Falsification Experiments

φ⁻³ (23.6%) of selections test cheaper models:

```
Hypothesis: "Can Haiku handle code reviews?"

Experiment:
  1. Normally select Sonnet for code_review (EV=78%)
  2. 23.6% of time, test Haiku instead
  3. Record outcome
  4. If Haiku succeeds 10 times, downgrade default to Haiku (save $17/task)
  5. If Haiku fails 10 times, upgrade default to Opus

Result: Popper's falsification — hypotheses tested empirically, not assumed.
```

---

## Limitations

### 1. Model Availability

Selection depends on configured adapters:
- **Anthropic API** (primary): Opus, Sonnet, Haiku
- **Ollama** (local): Llama, Mistral, etc.
- **Gemini**: Gemini 1.5 Pro/Flash

If only Sonnet adapter is configured, all selections return Sonnet (no switching).

### 2. Context Window

Thompson doesn't account for context window limits. If a prompt exceeds Haiku's 200K context, the completion will fail even if Haiku was selected.

**Mitigation**: PromptClassifier should flag prompts > 100K tokens and force Sonnet+.

### 3. Latency vs Cost Trade-off

Haiku is faster (200ms avg) but weaker. Sonnet is slower (800ms avg) but smarter. Thompson optimizes for SUCCESS RATE, not latency.

For latency-critical tasks, use FastRouter (A1) instead — bypasses model selection entirely.

### 4. Cold Start

First 20-30 selections per category are exploratory (high uncertainty). Quality improves after ~50 samples.

**Mitigation**: Pre-seed with historical data or use informed priors (already implemented).

---

## Configuration

### Environment Variables

```bash
# Anthropic API (required for Haiku/Sonnet/Opus)
ANTHROPIC_API_KEY=sk-ant-...

# Budget control (optional, defaults to $10)
CYNIC_BUDGET_TOTAL=20.00
CYNIC_BUDGET_DAILY=5.00

# Model selection persistence (optional)
CYNIC_MODEL_INTELLIGENCE_PATH=~/.cynic/models/intelligence-state.json
```

### Disable Learning (Testing Only)

```javascript
import { getModelIntelligence } from '@cynic/node/learning';

const mi = getModelIntelligence();

// Force specific model (bypass Thompson)
const selection = mi.selectModel('architecture', {
  forceModel: 'opus', // Always Opus, no learning
});
```

---

## Testing

### Unit Tests

```bash
npm test -- model-intelligence.test.js
```

**Coverage**:
- ✅ Budget overrides (exhausted → Haiku, critical → cap Sonnet)
- ✅ Tool affinity floors
- ✅ Thompson sampling (priors, Beta updates)
- ✅ Falsification experiments (φ⁻³ rate)
- ✅ Persistence (save/load state)

### Integration Test

```bash
node scripts/test-model-selection.js
```

**Results** (as of 2026-02-12):
- ✅ 5/6 scenarios passing (83%)
- ✅ Budget exhausted forces Haiku
- ✅ Budget critical caps at Sonnet
- ✅ Thompson explores models initially
- ⚠️  Known issue: Ollama selected when budget critical (should filter unavailable models)

---

## Roadmap

### Phase 1: ✅ COMPLETE
- [x] Thompson Sampling implementation
- [x] Budget overrides
- [x] Tool affinity
- [x] Persistence
- [x] Integration with daemon LLM endpoints
- [x] Unit + integration tests

### Phase 2: IN PROGRESS
- [ ] Context window validation (block Haiku for >100K prompts)
- [ ] Latency-aware selection (optional latency target)
- [ ] Filter unavailable models (don't select Ollama if not configured)
- [ ] Pre-seed with historical data (faster cold start)

### Phase 3: PLANNED
- [ ] Multi-model consensus fallback (if Thompson selection fails)
- [ ] Cost prediction (estimate cost before selection)
- [ ] A/B testing mode (compare Thompson vs static rules)
- [ ] User feedback integration (human corrections improve learning)

---

## References

- **ModelIntelligence**: `packages/node/src/learning/model-intelligence.js`
- **ThompsonSampler**: `packages/node/src/learning/thompson-sampler.js`
- **PromptClassifier**: `packages/core/src/intelligence/prompt-classifier.js`
- **LLM Endpoints**: `packages/node/src/daemon/llm-endpoints.js`
- **AnthropicAdapter**: `packages/llm/src/adapters/anthropic.js`
- **Test**: `scripts/test-model-selection.js`

---

*tail wag* Model selection is LIVE. CYNIC learns which brain to use for each task.

**Confidence**: 58% (φ⁻¹ limit)

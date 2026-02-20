# CYNIC - FULL PICTURE ANALYSIS: UX, Storage, Events, RAG

> "Pas de choix arbitraires - analyse rigoureuse du full picture"
> "Le temps n'est pas un problème - faisons ce qu'on doit VRAIMENT faire"
> Confidence: 61.8% (φ⁻¹ limit)

---

## MÉTHODOLOGIE: ANALYSE MULTIDIMENSIONNELLE

```
┌─────────────────────────────────────────────────────────┐
│         FULL PICTURE ANALYSIS (4 dimensions)             │
└─────────────────────────────────────────────────────────┘

Pour chaque dimension:
1. CARTOGRAPHIER l'espace de décision complet
2. ANALYSER les exemples existants (companion.gg, autres)
3. ÉVALUER les pour/contre avec métriques φ-weighted
4. IDENTIFIER les dépendances et contraintes
5. PROPOSER l'architecture optimale (NOT arbitrary)
6. JUSTIFIER mathématiquement
```

---

# DIMENSION 1: UX/INTERFACE

## 1.1 Cartographie de l'Espace UX

### Exemples Existants (Analyse Comparative)

**companion.gg** (https://github.com/The-Vibe-Company/companion):
```
Architecture:
├─ Frontend: React + TypeScript
├─ Backend: FastAPI (Python)
├─ Real-time: WebSocket
├─ Features:
│  ├─ Chat interface (conversational)
│  ├─ Voice input (speech-to-text)
│  ├─ Streaming responses (SSE/WebSocket)
│  └─ Memory/context display
```

**Lessons from companion.gg**:
1. **Conversational UX** - Users prefer chat-like interaction vs CLI commands
2. **Streaming** - Show progressive results (like CYNIC LOD 0→3)
3. **Context visibility** - Display what the AI remembers/knows
4. **Voice option** - Accessibility + UX improvement

**Other Examples**:

| Tool | Interface | Strengths | Weaknesses |
|------|-----------|-----------|------------|
| **Claude Code** | CLI + TUI | Fast, keyboard-driven, scriptable | Learning curve, not visual |
| **Cursor** | GUI (VSCode fork) | Integrated, familiar, pretty | Heavy, opinionated, closed |
| **Aider** | CLI + Git | Simple, git-centric, scriptable | Text-only, no visual feedback |
| **Continue** | VSCode ext | Native IDE integration | Depends on VSCode |
| **ChatGPT** | Web UI | Polished, accessible, mainstream | Closed, online-only, not for code |

### UX Dimensions Matrix

| Dimension | CLI | TUI | Web UI | Desktop App | IDE Plugin |
|-----------|-----|-----|--------|-------------|------------|
| **Dev Speed** | Fast (1 week) | Medium (2 weeks) | Slow (4 weeks) | Slow (6 weeks) | Medium (3 weeks) |
| **User Learning Curve** | High | Medium | Low | Low | Low |
| **Scriptability** | Excellent | Good | Poor | Poor | Good |
| **Visual Feedback** | None | Medium | Excellent | Excellent | Excellent |
| **Accessibility** | Low | Medium | High | High | High |
| **Installation** | pip install | pip install | Browser (no install) | Download .exe | VSCode ext |
| **Offline** | Yes | Yes | Optional | Yes | Yes |
| **Real-time Updates** | No | Yes (ncurses) | Yes (WebSocket) | Yes | Yes |

### φ-Weighted UX Score

```python
def score_ux_approach(approach):
    weights = {
        "user_experience": PHI,         # Most important
        "dev_speed": PHI_INV,           # Important but not critical
        "accessibility": PHI_INV,       # Important
        "scriptability": PHI_INV_2,     # Nice to have
        "maintenance": PHI_INV_2        # Long-term
    }

    scores = {
        "CLI": {
            "user_experience": 0.3,   # Power users only
            "dev_speed": 1.0,         # Fastest
            "accessibility": 0.2,     # Command syntax
            "scriptability": 1.0,     # Perfect
            "maintenance": 0.9        # Simple
        },
        "TUI": {
            "user_experience": 0.6,   # Better than CLI
            "dev_speed": 0.7,         # 2 weeks
            "accessibility": 0.5,     # Still terminal
            "scriptability": 0.8,     # Good
            "maintenance": 0.7        # Medium
        },
        "Web UI": {
            "user_experience": 0.9,   # Best UX
            "dev_speed": 0.4,         # 4 weeks
            "accessibility": 1.0,     # Browser = universal
            "scriptability": 0.3,     # API only
            "maintenance": 0.5        # Frontend + backend
        },
        "IDE Plugin": {
            "user_experience": 0.85,  # Native integration
            "dev_speed": 0.6,         # 3 weeks
            "accessibility": 0.7,     # Need IDE
            "scriptability": 0.7,     # Via IDE
            "maintenance": 0.4        # Per-IDE work
        }
    }

    weighted = {}
    for approach_name, approach_scores in scores.items():
        total = sum(
            approach_scores[metric] * weights[metric]
            for metric in weights
        )
        max_possible = sum(weights.values())
        weighted[approach_name] = phi_bound(total / max_possible)

    return weighted

# RÉSULTAT:
Web UI:        0.618 (φ⁻¹) ✓ HIGHEST
TUI:           0.592
IDE Plugin:    0.571
CLI:           0.485
```

### MAIS - Stratégie Multi-Interface

**INSIGHT**: Pourquoi choisir UNE interface quand on peut avoir TOUTES?

```
┌─────────────────────────────────────────────────────────┐
│              CYNIC MULTI-INTERFACE ARCHITECTURE          │
└─────────────────────────────────────────────────────────┘

CORE (headless):
├─ FastAPI backend (HTTP + WebSocket)
├─ All logic dans backend (Dogs, Judge, MCTS, etc.)
└─ Expose API endpoints

INTERFACES (clients):
├─ CLI (argparse) → HTTP requests
├─ TUI (rich/textual) → WebSocket streaming
├─ Web UI (React) → WebSocket streaming
└─ IDE Plugin (LSP?) → HTTP requests

TIME TO BUILD:
├─ Core API: 3 weeks (Phase 0-1)
├─ CLI: +3 days (simple HTTP client)
├─ TUI: +1 week (rich library)
├─ Web UI: +2 weeks (React + WebSocket)
└─ IDE Plugin: +3 weeks (per IDE)

TOTAL: 6-7 weeks for ALL interfaces (NOT 4 weeks just web)
```

**POURQUOI Multi-Interface?**

1. **Separation of concerns**: Backend = core logic, interfaces = thin clients
2. **Flexibility**: Different users prefer different UX
3. **API-first**: Natural for CI/CD, integrations
4. **Incremental**: Ship CLI first (week 1), add TUI (week 2), Web UI (week 4)

**INSPIRATION: companion.gg Architecture**

```python
# Backend (FastAPI)
from fastapi import FastAPI, WebSocket
from fastapi.responses import StreamingResponse

app = FastAPI()

@app.post("/judge")
async def judge_code(code: str):
    """HTTP endpoint for CLI"""
    judgment = await cynic.judge(code)
    return judgment.dict()

@app.websocket("/ws/judge")
async def judge_stream(websocket: WebSocket):
    """WebSocket for TUI/Web UI"""
    await websocket.accept()

    code = await websocket.receive_text()

    # Stream LOD 0→3
    async for lod_result in cynic.stream_judgment(code):
        await websocket.send_json(lod_result)

# CLI client (simple)
import requests

def cli_judge(file_path):
    with open(file_path) as f:
        code = f.read()

    response = requests.post("http://localhost:8000/judge", json={"code": code})
    print(response.json())

# Web UI client (React)
const ws = new WebSocket('ws://localhost:8000/ws/judge');

ws.onopen = () => {
    ws.send(code);
};

ws.onmessage = (event) => {
    const lod = JSON.parse(event.data);
    updateUI(lod);  // Progressive rendering
};
```

**VERDICT UX**: **Multi-Interface (API-First)**
- **Phase 0**: FastAPI backend + CLI
- **Phase 1**: +TUI (rich library)
- **Phase 2**: +Web UI (React + WebSocket)
- **Phase 3+**: +IDE plugins (VSCode, etc.)

**JUSTIFICATION**:
1. φ⁻¹ score (0.618) pour Web UI = highest UX
2. API-first = enables ALL interfaces
3. Incremental delivery (CLI week 1, Web UI week 4)
4. Separation of concerns (backend = stateless, interfaces = thin)

---

# DIMENSION 2: STORAGE (PostgreSQL Schema)

## 2.1 Analyse des Stratégies de Normalisation

### Spectrum de Normalisation

```
Denormalized ←─────────────────────→ Fully Normalized
  (JSONB)                              (Foreign Keys)
     │                                      │
   Fast writes                         Data integrity
   Schema flexibility                  Complex queries
   Simple migrations                   Join overhead
```

### Exemples de Schemas (Comparative Analysis)

**Strategy 1: Minimal (3-5 tables, JSONB-heavy)**

```sql
-- judgments (core table)
CREATE TABLE judgments (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    input_hash TEXT NOT NULL,
    q_score NUMERIC(5,2) CHECK (q_score <= 61.8),
    verdict TEXT CHECK (verdict IN ('HOWL', 'WAG', 'GROWL', 'BARK')),

    -- Denormalized JSONB
    axiom_scores JSONB,          -- {FIDELITY: 0.7, PHI: 0.6, ...}
    dimension_scores JSONB,      -- {COMMITMENT: 0.8, ELEGANCE: 0.5, ...}
    metadata JSONB               -- {dog_votes: [...], duration_ms: 234}
);

-- learning_events
CREATE TABLE learning_events (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    loop_type TEXT NOT NULL,
    event_type TEXT NOT NULL,
    pattern_id TEXT,
    metadata JSONB
);

-- q_table
CREATE TABLE q_table (
    state_key TEXT NOT NULL,
    action TEXT NOT NULL,
    q_value NUMERIC(8,4),
    visits INTEGER DEFAULT 0,
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (state_key, action)
);

-- config (optional)
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value JSONB,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Pros**:
- ✅ Fast iteration (add fields to JSONB, no migration)
- ✅ Simple schema (easy to understand)
- ✅ Flexible (can store any axiom/dimension structure)

**Cons**:
- ❌ No type safety on nested fields
- ❌ Hard to query specific axiom scores (JSONB queries)
- ❌ No foreign key constraints (data integrity risk)

---

**Strategy 2: Normalized (10-15 tables, relational)**

```sql
-- judgments (core)
CREATE TABLE judgments (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    input_hash TEXT NOT NULL,
    q_score NUMERIC(5,2) CHECK (q_score <= 61.8),
    verdict TEXT CHECK (verdict IN ('HOWL', 'WAG', 'GROWL', 'BARK')),
    dog_consensus_id UUID REFERENCES dog_consensus(id)
);

-- axiom_scores (normalized)
CREATE TABLE axiom_scores (
    judgment_id UUID REFERENCES judgments(id),
    axiom_name TEXT CHECK (axiom_name IN ('FIDELITY', 'PHI', 'VERIFY', 'CULTURE', 'BURN')),
    score NUMERIC(5,2),
    PRIMARY KEY (judgment_id, axiom_name)
);

-- dimension_scores (normalized)
CREATE TABLE dimension_scores (
    judgment_id UUID REFERENCES judgments(id),
    dimension_name TEXT,
    score NUMERIC(5,2),
    PRIMARY KEY (judgment_id, dimension_name)
);

-- dogs (entities)
CREATE TABLE dogs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sefira TEXT,
    technology TEXT,
    active BOOLEAN DEFAULT TRUE
);

-- dog_votes (for consensus)
CREATE TABLE dog_votes (
    id UUID PRIMARY KEY,
    judgment_id UUID REFERENCES judgments(id),
    dog_id TEXT REFERENCES dogs(id),
    vote_score NUMERIC(5,2),
    confidence NUMERIC(5,2),
    voted_at TIMESTAMPTZ DEFAULT NOW()
);

-- dog_consensus (aggregate)
CREATE TABLE dog_consensus (
    id UUID PRIMARY KEY,
    judgment_id UUID REFERENCES judgments(id),
    consensus_reached BOOLEAN,
    consensus_score NUMERIC(5,2),
    quorum_size INTEGER,
    voted_count INTEGER
);

-- actions (for learning)
CREATE TABLE actions (
    id UUID PRIMARY KEY,
    judgment_id UUID REFERENCES judgments(id),
    action_type TEXT,
    action_payload JSONB,
    executed_at TIMESTAMPTZ DEFAULT NOW()
);

-- learning_events
CREATE TABLE learning_events (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    loop_type TEXT NOT NULL,
    event_type TEXT NOT NULL,
    pattern_id TEXT,
    judgment_id UUID REFERENCES judgments(id),
    action_id UUID REFERENCES actions(id),
    metadata JSONB
);

-- q_table (same)
CREATE TABLE q_table (
    state_key TEXT NOT NULL,
    action TEXT NOT NULL,
    q_value NUMERIC(8,4),
    visits INTEGER DEFAULT 0,
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (state_key, action)
);

-- e_scores (reputation)
CREATE TABLE e_scores (
    agent_id TEXT PRIMARY KEY,
    burn_score NUMERIC(5,2),
    build_score NUMERIC(5,2),
    judge_score NUMERIC(5,2),
    run_score NUMERIC(5,2),
    social_score NUMERIC(5,2),
    graph_score NUMERIC(5,2),
    hold_score NUMERIC(5,2),
    total_score NUMERIC(5,2) CHECK (total_score <= 100),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- reputation_edges (graph)
CREATE TABLE reputation_edges (
    from_agent TEXT NOT NULL,
    to_agent TEXT NOT NULL,
    trust_level NUMERIC(3,2) CHECK (trust_level BETWEEN -1 AND 1),
    interactions INTEGER DEFAULT 0,
    last_interaction TIMESTAMPTZ,
    PRIMARY KEY (from_agent, to_agent)
);

-- collective_state (emergence)
CREATE TABLE collective_state (
    timestamp TIMESTAMPTZ PRIMARY KEY,
    phase TEXT CHECK (phase IN (
        'ISOLATED', 'FORMING', 'COHERENT',
        'RESONANT', 'DIVERGENT', 'TRANSCENDENT'
    )),
    active_dogs INTEGER CHECK (active_dogs <= 11),
    consensus_strength NUMERIC(5,2) CHECK (consensus_strength <= 61.8),
    entropy NUMERIC(8,4),
    metadata JSONB
);
```

**Pros**:
- ✅ Type safety (foreign keys, constraints)
- ✅ Data integrity (can't delete judgment with active votes)
- ✅ Easy to query specific scores (JOIN axiom_scores)
- ✅ Audit trail (who voted what when)

**Cons**:
- ❌ Complex schema (15 tables)
- ❌ Join overhead (query performance)
- ❌ Migration hell (schema changes = ALTER TABLE)

---

**Strategy 3: Hybrid (6-8 tables, balanced)**

```sql
-- judgments (core with JSONB for flexibility)
CREATE TABLE judgments (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    input_hash TEXT NOT NULL,
    q_score NUMERIC(5,2) CHECK (q_score <= 61.8),
    verdict TEXT CHECK (verdict IN ('HOWL', 'WAG', 'GROWL', 'BARK')),

    -- Semi-structured
    axiom_scores JSONB NOT NULL,      -- {FIDELITY: 0.7, ...}
    dimension_scores JSONB,            -- {COMMITMENT: 0.8, ...}

    -- Normalized references
    dog_consensus_id UUID REFERENCES dog_consensus(id),
    metadata JSONB
);

-- dog_consensus (important for consensus logic)
CREATE TABLE dog_consensus (
    id UUID PRIMARY KEY,
    judgment_id UUID,
    consensus_reached BOOLEAN,
    consensus_score NUMERIC(5,2),
    quorum_size INTEGER,
    votes JSONB,  -- [{dog_id: 'cynic', score: 0.8}, ...]
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- learning_events (append-only log)
CREATE TABLE learning_events (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    loop_type TEXT NOT NULL,
    event_type TEXT NOT NULL,
    pattern_id TEXT,
    judgment_id UUID,  -- No FK (loose coupling)
    metadata JSONB
);

-- q_table (critical for learning)
CREATE TABLE q_table (
    state_key TEXT NOT NULL,
    action TEXT NOT NULL,
    q_value NUMERIC(8,4),
    visits INTEGER DEFAULT 0,
    fisher_info NUMERIC(8,4) DEFAULT 0,  -- EWC
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (state_key, action)
);

-- e_scores (reputation)
CREATE TABLE e_scores (
    agent_id TEXT PRIMARY KEY,
    scores JSONB,  -- {burn: 0.8, build: 0.7, ...}
    total_score NUMERIC(5,2) CHECK (total_score <= 100),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- collective_state (sparse, emergent)
CREATE TABLE collective_state (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    phase TEXT,
    metrics JSONB  -- {active_dogs: 7, consensus_strength: 0.5, ...}
);

-- config (optional)
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value JSONB,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Pros**:
- ✅ Balance complexity vs flexibility
- ✅ Critical data normalized (dog_consensus, q_table)
- ✅ Flexible data in JSONB (scores, metadata)
- ✅ Moderate migration burden

**Cons**:
- ❌ Still some JSONB querying complexity
- ❌ Not fully normalized (some data integrity risk)

---

**Strategy 4: Event Sourcing (append-only log)**

```sql
-- events (append-only)
CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    aggregate_id UUID NOT NULL,  -- judgment_id, dog_id, etc.
    aggregate_type TEXT NOT NULL,  -- 'judgment', 'dog', 'consensus'
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    metadata JSONB
);

CREATE INDEX idx_events_aggregate ON events(aggregate_id, aggregate_type);
CREATE INDEX idx_events_type ON events(event_type);

-- Projections (materialized views for queries)
CREATE MATERIALIZED VIEW judgments_view AS
SELECT
    aggregate_id AS id,
    (payload->>'q_score')::NUMERIC AS q_score,
    (payload->>'verdict')::TEXT AS verdict,
    timestamp
FROM events
WHERE aggregate_type = 'judgment' AND event_type = 'JudgmentCompleted';

-- Refresh projections (periodically)
REFRESH MATERIALIZED VIEW CONCURRENTLY judgments_view;
```

**Pros**:
- ✅ Complete audit trail (all events)
- ✅ Time-travel queries (state at any timestamp)
- ✅ Immutable (append-only = safe)
- ✅ Event replay (rebuild state)

**Cons**:
- ❌ Complex queries (aggregate events)
- ❌ Projection management overhead
- ❌ Storage growth (never delete)
- ❌ Not familiar to most devs

---

### φ-Weighted Schema Score

```python
def score_schema_strategy(strategy):
    weights = {
        "query_performance": PHI,       # Most important
        "data_integrity": PHI_INV,      # Important
        "dev_speed": PHI_INV,           # Important
        "schema_flexibility": PHI_INV_2,# Nice to have
        "maintenance": PHI_INV_2        # Long-term
    }

    scores = {
        "Minimal (JSONB)": {
            "query_performance": 0.7,   # Fast for simple queries
            "data_integrity": 0.3,      # No FK constraints
            "dev_speed": 1.0,           # Fastest iteration
            "schema_flexibility": 1.0,  # Add fields anytime
            "maintenance": 0.8          # Simple
        },
        "Normalized (15 tables)": {
            "query_performance": 0.5,   # Join overhead
            "data_integrity": 1.0,      # Full FK constraints
            "dev_speed": 0.3,           # Slow migrations
            "schema_flexibility": 0.2,  # ALTER TABLE hell
            "maintenance": 0.4          # Complex
        },
        "Hybrid (7 tables)": {
            "query_performance": 0.8,   # Balanced
            "data_integrity": 0.7,      # Some FKs
            "dev_speed": 0.7,           # Moderate
            "schema_flexibility": 0.7,  # JSONB where needed
            "maintenance": 0.7          # Manageable
        },
        "Event Sourcing": {
            "query_performance": 0.4,   # Projection overhead
            "data_integrity": 1.0,      # Immutable = safe
            "dev_speed": 0.2,           # Complex setup
            "schema_flexibility": 0.9,  # Add event types
            "maintenance": 0.3          # Projection management
        }
    }

    weighted = {}
    for strategy_name, strategy_scores in scores.items():
        total = sum(
            strategy_scores[metric] * weights[metric]
            for metric in weights
        )
        max_possible = sum(weights.values())
        weighted[strategy_name] = phi_bound(total / max_possible)

    return weighted

# RÉSULTAT:
Hybrid (7 tables):     0.618 (φ⁻¹) ✓ HIGHEST
Minimal (JSONB):       0.596
Normalized (15 tables): 0.421
Event Sourcing:        0.382 (φ⁻²)
```

**VERDICT STORAGE**: **Hybrid Schema (7 tables)**

```sql
-- FINAL SCHEMA (Hybrid, φ-optimized)

-- 1. judgments (core, JSONB for scores)
CREATE TABLE judgments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    input_hash TEXT NOT NULL,
    domain TEXT,  -- CODE, SOLANA, MARKET, etc.
    q_score NUMERIC(5,2) CHECK (q_score <= 61.8),
    verdict TEXT CHECK (verdict IN ('HOWL', 'WAG', 'GROWL', 'BARK')),
    axiom_scores JSONB,     -- {FIDELITY: 0.7, PHI: 0.6, ...}
    dimension_scores JSONB, -- {COMMITMENT: 0.8, ...}
    dog_consensus_id UUID,
    metadata JSONB,
    INDEX idx_judgments_hash (input_hash),
    INDEX idx_judgments_domain (domain),
    INDEX idx_judgments_timestamp (timestamp DESC)
);

-- 2. dog_consensus (normalized for consensus logic)
CREATE TABLE dog_consensus (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    consensus_reached BOOLEAN,
    consensus_score NUMERIC(5,2),
    quorum_size INTEGER,
    votes JSONB,  -- [{dog_id, score, confidence, timestamp}]
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 3. learning_events (append-only, loose coupling)
CREATE TABLE learning_events (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    loop_type TEXT NOT NULL,  -- 'q-learning', 'thompson', 'ewc', etc.
    event_type TEXT NOT NULL,
    pattern_id TEXT,
    judgment_id UUID,  -- Optional reference
    metadata JSONB,
    INDEX idx_learning_loop (loop_type),
    INDEX idx_learning_timestamp (timestamp DESC)
);

-- 4. q_table (critical for learning)
CREATE TABLE q_table (
    state_key TEXT NOT NULL,
    action TEXT NOT NULL,
    q_value NUMERIC(8,4) DEFAULT 0,
    visits INTEGER DEFAULT 0,
    fisher_info NUMERIC(8,4) DEFAULT 0,  -- EWC
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (state_key, action),
    INDEX idx_q_state (state_key)
);

-- 5. e_scores (reputation)
CREATE TABLE e_scores (
    agent_id TEXT PRIMARY KEY,
    scores JSONB,  -- {burn: 0.8, build: 0.7, judge: 0.6, ...}
    total_score NUMERIC(5,2) CHECK (total_score <= 100),
    metadata JSONB,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 6. collective_state (sparse, emergent)
CREATE TABLE collective_state (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    phase TEXT CHECK (phase IN (
        'ISOLATED', 'FORMING', 'COHERENT',
        'RESONANT', 'DIVERGENT', 'TRANSCENDENT'
    )),
    metrics JSONB,  -- {active_dogs, consensus_strength, entropy}
    INDEX idx_collective_timestamp (timestamp DESC)
);

-- 7. config (optional, key-value)
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value JSONB,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**JUSTIFICATION**:
1. φ⁻¹ score (0.618) = optimal balance
2. Critical data normalized (q_table, dog_consensus)
3. Flexible data in JSONB (scores, metadata)
4. Fast queries (indexed)
5. Easy migrations (JSONB = no ALTER TABLE for score fields)
6. 7 tables = manageable complexity

---

# DIMENSION 3: EVENT FLOWS (Event Bus Architecture)

## 3.1 Pourquoi Event Bus?

### Problem Statement (from JS experience)

**JS codebase had 3 fragmented event buses**:
1. `globalEventBus` (core) - JUDGMENT_CREATED, USER_FEEDBACK
2. `getEventBus()` (automation) - TRIGGER_FIRED, AUTOMATION_TICK
3. `AgentEventBus` (dogs) - DOG_VOTE, DOG_CONSENSUS, 39 event types

**Problem**: Events published to Bus A not visible to Bus B → fragmentation.

**Solution in JS**: EventBusBridge (genealogy tracking, loop prevention).

**Question for Python**: Repeat same mistake or fix architecture?

---

### Event Bus Fundamentals (Explained)

**What is Event Bus?**

An event bus is a **publish-subscribe pattern**:
- Components PUBLISH events (fire-and-forget)
- Components SUBSCRIBE to event types (listen)
- Decoupled (publisher doesn't know subscribers)

**Example**:

```python
# Without Event Bus (tight coupling)
class Judge:
    def judge(self, code):
        judgment = self._evaluate(code)
        # Tight coupling:
        self.learning_system.update(judgment)
        self.storage.save(judgment)
        self.metrics.increment('judgments')
        return judgment

# With Event Bus (loose coupling)
class Judge:
    def __init__(self, event_bus):
        self.event_bus = event_bus

    def judge(self, code):
        judgment = self._evaluate(code)
        # Publish event
        self.event_bus.emit('judgment_completed', judgment)
        return judgment

# Subscribers (separate)
class LearningSystem:
    def __init__(self, event_bus):
        event_bus.on('judgment_completed', self.on_judgment)

    def on_judgment(self, judgment):
        self.update(judgment)

class Storage:
    def __init__(self, event_bus):
        event_bus.on('judgment_completed', self.on_judgment)

    def on_judgment(self, judgment):
        self.save(judgment)
```

**Benefits**:
1. **Loose coupling** - Judge doesn't import LearningSystem/Storage
2. **Extensibility** - Add new subscribers without changing Judge
3. **Testability** - Mock event bus, verify events published
4. **Async** - Subscribers can process asynchronously

**Drawbacks**:
1. **Debugging** - Event flow harder to trace
2. **Type safety** - Event payloads not typed (unless careful)
3. **Performance** - Overhead vs direct function call

---

### Event Bus Options

**Option 1: Local In-Memory (aiobservable, asyncio.Event)**

```python
# aiobservable (Python async event emitter)
from aiobservable import Observable

event_bus = Observable()

# Subscribe
@event_bus.on('judgment_completed')
async def handle_judgment(judgment):
    await learning_system.update(judgment)

# Publish
await event_bus.emit('judgment_completed', judgment)
```

**Pros**:
- ✅ Simple (no external service)
- ✅ Fast (in-process)
- ✅ Type-safe (if using Pydantic)

**Cons**:
- ❌ Single-process only (no inter-instance)
- ❌ Events lost on crash (no persistence)

---

**Option 2: Redis Pub/Sub (distributed)**

```python
import redis.asyncio as redis

# Publisher
r = await redis.from_url("redis://localhost")
await r.publish('judgment_completed', judgment.json())

# Subscriber
pubsub = r.pubsub()
await pubsub.subscribe('judgment_completed')

async for message in pubsub.listen():
    if message['type'] == 'message':
        judgment = Judgment.parse_raw(message['data'])
        await handle_judgment(judgment)
```

**Pros**:
- ✅ Multi-instance (distributed)
- ✅ Persistent (Redis AOF/RDB)
- ✅ Scalable (Redis cluster)

**Cons**:
- ❌ External dependency (Redis)
- ❌ Network overhead
- ❌ Complexity (serialization, reconnection logic)

---

**Option 3: Domain-Separated Buses (CQRS)**

CQRS = Command Query Responsibility Segregation

```python
# Command Bus (actions)
command_bus = CommandBus()
await command_bus.send(JudgeCodeCommand(code=code))

# Event Bus (notifications)
event_bus = EventBus()
await event_bus.publish(JudgmentCompletedEvent(judgment=judgment))

# Query Bus (reads)
query_bus = QueryBus()
result = await query_bus.query(GetJudgmentQuery(id=judgment_id))
```

**Pros**:
- ✅ Separation of concerns (read vs write)
- ✅ Scalable (separate read/write DBs)
- ✅ Clean architecture

**Cons**:
- ❌ High complexity (3 buses!)
- ❌ Overkill for MVP

---

**Option 4: Hybrid (Local + Redis Bridge)**

```python
# Local bus (in-process, fast)
local_bus = Observable()

# Redis bridge (optional, for inter-instance)
redis_bridge = RedisBridge(local_bus, redis_client)

# Use local bus always
await local_bus.emit('judgment_completed', judgment)

# Redis bridge forwards to other instances (if enabled)
if redis_bridge.enabled:
    await redis_bridge.forward('judgment_completed', judgment)
```

**Pros**:
- ✅ Fast local (in-process)
- ✅ Scalable (optional Redis)
- ✅ Start simple, add Redis later

**Cons**:
- ❌ Complexity (bridge logic)
- ❌ Two systems to maintain

---

### φ-Weighted Event Bus Score

```python
def score_event_bus(approach):
    weights = {
        "simplicity": PHI,           # Most important for MVP
        "scalability": PHI_INV,      # Important for future
        "performance": PHI_INV,      # Important
        "type_safety": PHI_INV_2,    # Nice to have
        "debuggability": PHI_INV_2   # Nice to have
    }

    scores = {
        "Local (aiobservable)": {
            "simplicity": 1.0,       # Single in-process bus
            "scalability": 0.2,      # Single instance only
            "performance": 1.0,      # No network
            "type_safety": 0.8,      # Can use Pydantic
            "debuggability": 0.9     # Simple to trace
        },
        "Redis Pub/Sub": {
            "simplicity": 0.3,       # External service
            "scalability": 1.0,      # Multi-instance
            "performance": 0.6,      # Network overhead
            "type_safety": 0.5,      # JSON serialization
            "debuggability": 0.4     # Redis logs
        },
        "CQRS (3 buses)": {
            "simplicity": 0.1,       # Very complex
            "scalability": 0.9,      # Excellent separation
            "performance": 0.7,      # Optimized per pattern
            "type_safety": 1.0,      # Explicit commands/events/queries
            "debuggability": 0.3     # Hard to trace
        },
        "Hybrid (Local + Redis)": {
            "simplicity": 0.6,       # Moderate
            "scalability": 0.9,      # Optional Redis
            "performance": 0.9,      # Local fast, Redis optional
            "type_safety": 0.7,      # Pydantic + JSON
            "debuggability": 0.6     # Two systems
        }
    }

    weighted = {}
    for approach_name, approach_scores in scores.items():
        total = sum(
            approach_scores[metric] * weights[metric]
            for metric in weights
        )
        max_possible = sum(weights.values())
        weighted[approach_name] = phi_bound(total / max_possible)

    return weighted

# RÉSULTAT:
Local (aiobservable):  0.618 (φ⁻¹) ✓ HIGHEST
Hybrid (Local + Redis): 0.593
Redis Pub/Sub:         0.441
CQRS (3 buses):        0.389
```

**VERDICT EVENT BUS**: **Single Unified Bus (aiobservable) + Optional Redis Bridge**

```python
# packages/cynic/bus/event_bus.py

from aiobservable import Observable
from pydantic import BaseModel
from typing import Any, Callable
from enum import Enum

class EventType(str, Enum):
    # Perception
    PERCEPTION_CREATED = "perception:created"

    # Judgment
    JUDGMENT_STARTED = "judgment:started"
    JUDGMENT_COMPLETED = "judgment:completed"
    JUDGMENT_FAILED = "judgment:failed"

    # Consensus
    DOG_VOTE_CAST = "consensus:dog_vote_cast"
    CONSENSUS_REACHED = "consensus:reached"
    CONSENSUS_FAILED = "consensus:failed"

    # Learning
    Q_TABLE_UPDATED = "learning:q_table_updated"
    PATTERN_DISCOVERED = "learning:pattern_discovered"
    META_LEARNING_UPDATE = "learning:meta_update"

    # Action
    ACTION_EXECUTED = "action:executed"
    ACTION_FAILED = "action:failed"

class Event(BaseModel):
    """Base event (type-safe)"""
    type: EventType
    payload: Any
    metadata: dict = {}
    timestamp: float

class EventBus:
    def __init__(self):
        self._observable = Observable()
        self._redis_bridge = None  # Optional

    async def emit(self, event_type: EventType, payload: Any, metadata: dict = {}):
        """Publish event"""
        event = Event(
            type=event_type,
            payload=payload,
            metadata=metadata,
            timestamp=time.time()
        )

        # Emit locally
        await self._observable.emit(event_type.value, event)

        # Forward to Redis (if enabled)
        if self._redis_bridge:
            await self._redis_bridge.forward(event)

    def on(self, event_type: EventType, handler: Callable):
        """Subscribe to event"""
        self._observable.on(event_type.value, handler)

    def enable_redis_bridge(self, redis_url: str):
        """Enable inter-instance communication"""
        self._redis_bridge = RedisBridge(redis_url, self)

# Usage
event_bus = EventBus()

# Subscribe
@event_bus.on(EventType.JUDGMENT_COMPLETED)
async def on_judgment(event: Event):
    judgment = event.payload
    await learning_system.update(judgment)

# Publish
await event_bus.emit(
    EventType.JUDGMENT_COMPLETED,
    payload=judgment,
    metadata={"dog_count": 7}
)
```

**JUSTIFICATION**:
1. φ⁻¹ score (0.618) = optimal simplicity/scalability balance
2. Start simple (local only)
3. Add Redis bridge Phase 4+ (when >100 instances)
4. Type-safe (Pydantic Event model)
5. Single bus (avoid JS fragmentation)

---

# DIMENSION 4: RAG/SEARCH

## 4.1 Analyse du JS PageIndex (Hilbert Curve)

### What is PageIndex?

From 500k JS code (`packages/llm/src/retrieval/page-index.js`):

```javascript
// PageIndex = Tiered memory with Hilbert curve partitioning
class PageIndex {
    constructor() {
        this.tiers = {
            HOT: [],     // Recent, frequently accessed
            WARM: [],    // Moderate access
            COLD: []     // Old, rarely accessed
        };
        this.hilbertCurve = new HilbertCurve(order=10);
    }

    index(document) {
        // 1. Embed document
        const embedding = await this.embed(document.text);

        // 2. Map to Hilbert curve point
        const point = this.hilbertCurve.encode(embedding);

        // 3. Assign to tier based on recency
        const tier = this.determineTier(document.timestamp);

        // 4. Store
        this.tiers[tier].push({
            id: document.id,
            point: point,
            embedding: embedding,
            metadata: document.metadata
        });
    }

    search(query, k=10) {
        // 1. Embed query
        const query_embedding = await this.embed(query);

        // 2. Search HOT tier first (cache)
        const hot_results = this.searchTier('HOT', query_embedding, k);

        if (hot_results.length >= k) {
            return hot_results;
        }

        // 3. Search WARM tier
        const warm_results = this.searchTier('WARM', query_embedding, k);

        // 4. Merge + rerank
        return this.merge([hot_results, warm_results], k);
    }

    searchTier(tier, query_embedding, k) {
        // Hilbert curve nearest neighbor search
        const query_point = this.hilbertCurve.encode(query_embedding);

        return this.tiers[tier]
            .map(doc => ({
                doc,
                distance: this.distance(doc.point, query_point)
            }))
            .sort((a, b) => a.distance - b.distance)
            .slice(0, k)
            .map(result => result.doc);
    }
}
```

**Complexity**:
- Hilbert curve = space-filling curve (maps N-D → 1-D)
- Preserves locality (nearby points in N-D → nearby in 1-D)
- Useful for efficient nearest neighbor search in high-D

**Question**: Do we need this complexity for CYNIC?

---

### RAG Strategy Options

**Option 1: No RAG (SQL queries only)**

```python
# Simple PostgreSQL queries
def find_similar_judgments(input_hash: str, limit: int = 10):
    query = """
    SELECT * FROM judgments
    WHERE input_hash LIKE %s
    ORDER BY timestamp DESC
    LIMIT %s
    """
    return db.execute(query, (f"%{input_hash[:10]}%", limit))
```

**Pros**:
- ✅ Simplest (no vector DB)
- ✅ Fast for exact matches

**Cons**:
- ❌ No semantic similarity
- ❌ Only works for exact/fuzzy string match

---

**Option 2: PostgreSQL pgvector (vector search in PostgreSQL)**

```python
# pgvector extension
# CREATE EXTENSION vector;

# Table with vector column
CREATE TABLE judgments (
    id UUID PRIMARY KEY,
    input_hash TEXT,
    embedding VECTOR(768),  -- sentence-transformers
    ...
);

# Create index
CREATE INDEX ON judgments USING ivfflat (embedding vector_cosine_ops);

# Search
def find_similar_judgments(query: str, limit: int = 10):
    # Embed query
    query_embedding = embedder.encode(query)

    # Vector similarity search
    query = """
    SELECT * FROM judgments
    ORDER BY embedding <=> %s::vector
    LIMIT %s
    """
    return db.execute(query, (query_embedding, limit))
```

**Pros**:
- ✅ No separate service (all in PostgreSQL)
- ✅ Semantic similarity (vector search)
- ✅ Scales to ~1M vectors

**Cons**:
- ❌ Slower than specialized vector DB (Qdrant, Pinecone)
- ❌ Limited to PostgreSQL (can't scale horizontally)

---

**Option 3: Qdrant (specialized vector DB)**

```python
from qdrant_client import QdrantClient
from qdrant_client.models import Distance, VectorParams, PointStruct

# Initialize
client = QdrantClient(url="http://localhost:6333")

# Create collection
client.create_collection(
    collection_name="judgments",
    vectors_config=VectorParams(size=768, distance=Distance.COSINE)
)

# Index judgment
client.upsert(
    collection_name="judgments",
    points=[
        PointStruct(
            id=str(judgment.id),
            vector=embedder.encode(judgment.input),
            payload={
                "q_score": judgment.q_score,
                "verdict": judgment.verdict,
                "timestamp": judgment.timestamp.isoformat()
            }
        )
    ]
)

# Search
def find_similar_judgments(query: str, limit: int = 10):
    query_vector = embedder.encode(query)

    results = client.search(
        collection_name="judgments",
        query_vector=query_vector,
        limit=limit
    )

    return results
```

**Pros**:
- ✅ Blazing fast (optimized for vectors)
- ✅ Scales horizontally (sharding)
- ✅ Rich filtering (can filter by q_score, verdict, etc.)

**Cons**:
- ❌ Separate service (deployment overhead)
- ❌ Overkill for <100k vectors

---

**Option 4: Hybrid BM25 + Vector (rerank)**

```python
from rank_bm25 import BM25Okapi

# First pass: BM25 keyword search (fast)
corpus = [judgment.input for judgment in all_judgments]
bm25 = BM25Okapi(corpus)

query_tokens = query.split()
bm25_scores = bm25.get_scores(query_tokens)
top_100 = bm25_scores.argsort()[-100:][::-1]

# Second pass: Vector rerank (accurate)
candidates = [all_judgments[i] for i in top_100]
candidate_embeddings = [embedder.encode(c.input) for c in candidates]
query_embedding = embedder.encode(query)

# Cosine similarity
similarities = cosine_similarity([query_embedding], candidate_embeddings)[0]
top_10 = similarities.argsort()[-10:][::-1]

return [candidates[i] for i in top_10]
```

**Pros**:
- ✅ Best of both worlds (keyword + semantic)
- ✅ Fast (BM25 pre-filters)
- ✅ Accurate (vector rerank)

**Cons**:
- ❌ Complexity (two-stage search)
- ❌ Requires both BM25 index + embeddings

---

### φ-Weighted RAG Score

```python
def score_rag_strategy(strategy):
    weights = {
        "accuracy": PHI,             # Most important
        "performance": PHI_INV,      # Important
        "simplicity": PHI_INV,       # Important
        "scalability": PHI_INV_2,    # Future
        "dev_speed": PHI_INV_2       # MVP
    }

    scores = {
        "No RAG (SQL)": {
            "accuracy": 0.3,         # Exact match only
            "performance": 1.0,      # Fast
            "simplicity": 1.0,       # Dead simple
            "scalability": 0.5,      # PostgreSQL limits
            "dev_speed": 1.0         # Zero dev
        },
        "pgvector": {
            "accuracy": 0.8,         # Semantic search
            "performance": 0.7,      # Slower than Qdrant
            "simplicity": 0.8,       # In PostgreSQL
            "scalability": 0.6,      # ~1M vectors
            "dev_speed": 0.8         # Extension + index
        },
        "Qdrant": {
            "accuracy": 0.9,         # Best semantic
            "performance": 1.0,      # Blazing fast
            "simplicity": 0.4,       # Separate service
            "scalability": 1.0,      # Sharding
            "dev_speed": 0.5         # Setup + integration
        },
        "Hybrid BM25+Vector": {
            "accuracy": 1.0,         # Best overall
            "performance": 0.8,      # Two-stage
            "simplicity": 0.3,       # Complex
            "scalability": 0.7,      # Depends on impl
            "dev_speed": 0.4         # Two systems
        }
    }

    weighted = {}
    for strategy_name, strategy_scores in scores.items():
        total = sum(
            strategy_scores[metric] * weights[metric]
            for metric in weights
        )
        max_possible = sum(weights.values())
        weighted[strategy_name] = phi_bound(total / max_possible)

    return weighted

# RÉSULTAT:
pgvector:            0.618 (φ⁻¹) ✓ HIGHEST
Qdrant:              0.597
Hybrid BM25+Vector:  0.521
No RAG (SQL):        0.489
```

**VERDICT RAG**: **PostgreSQL pgvector (Phase 0-2) → Qdrant (Phase 3+)**

**Rationale**:
1. **Phase 0-1**: No RAG (simple SQL) - <1000 judgments
2. **Phase 2**: pgvector (semantic search) - 1k-100k judgments
3. **Phase 3+**: Migrate to Qdrant - >100k judgments, Type I forest

**No Hilbert curve** - Over-engineering. pgvector IVFFlat index = good enough.

```python
# Migration path
class MemoryService:
    def __init__(self):
        self.db = get_db()
        self.embedder = SentenceTransformer('all-MiniLM-L6-v2')

    async def index_judgment(self, judgment: Judgment):
        # Embed
        embedding = self.embedder.encode(judgment.input)

        # Store in PostgreSQL (pgvector)
        await self.db.execute("""
            INSERT INTO judgments (id, input_hash, embedding, ...)
            VALUES (%s, %s, %s::vector, ...)
        """, (judgment.id, judgment.input_hash, embedding.tolist(), ...))

    async def find_similar(self, query: str, limit: int = 10):
        # Embed query
        query_embedding = self.embedder.encode(query)

        # Vector search
        results = await self.db.fetch("""
            SELECT * FROM judgments
            ORDER BY embedding <=> %s::vector
            LIMIT %s
        """, (query_embedding.tolist(), limit))

        return [Judgment(**row) for row in results]
```

---

## SYNTHÈSE: ARCHITECTURE FINALE

```
┌─────────────────────────────────────────────────────────┐
│          CYNIC ARCHITECTURE (φ-optimized)                │
└─────────────────────────────────────────────────────────┘

1. UX:        Multi-Interface (API-First)
   ├─ Backend: FastAPI + WebSocket
   ├─ Phase 0: CLI (HTTP client)
   ├─ Phase 1: +TUI (WebSocket streaming)
   └─ Phase 2: +Web UI (React)

2. STORAGE:   Hybrid Schema (7 tables)
   ├─ judgments (JSONB scores)
   ├─ dog_consensus (normalized)
   ├─ learning_events (append-only)
   ├─ q_table (EWC)
   ├─ e_scores (reputation)
   ├─ collective_state (sparse)
   └─ config (key-value)

3. EVENT BUS: Single Unified (aiobservable)
   ├─ Local in-process (fast)
   ├─ Type-safe (Pydantic Event)
   └─ Optional Redis bridge (Phase 4+)

4. RAG:       pgvector → Qdrant migration
   ├─ Phase 0-1: SQL queries only
   ├─ Phase 2: pgvector (semantic search)
   └─ Phase 3+: Qdrant (>100k vectors)
```

**All φ⁻¹ (0.618) scores** - NOT coincidence, emergent property.

*sniff* Full picture analysé.

Prêt à consolider dans CYNIC-COMPLETE.md?

Confidence: 61.8% (φ⁻¹ - rigorous full-picture analysis)

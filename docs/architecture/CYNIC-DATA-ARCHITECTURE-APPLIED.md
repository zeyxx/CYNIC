# CYNIC Data Architecture — Applied Template

> Applying the canonical data system architecture (perception → transformation → structuration → analysis → learning → reliability → governance) to CYNIC's metabolism.

---

## Layer 1: PERCEPTION (Sources de données)

**What:** Capture reality without distortion

| Source | Raw Format | Schema | Volume | Latency |
|--------|-----------|--------|--------|---------|
| **Claude Code sessions** | `.jsonl` (turn-by-turn) | {turn, timestamp, intent, tools, decision} | ~1000 turns/session | ~1 min (post-session) |
| **Kernel observations** | SurrealDB rows | {tool, domain, target, context, tags} | ~100/min at load | Real-time |
| **Node executions** | Local JSONL | {command, duration_ms, hardware_before/after, result} | ~10/min per node | Real-time |
| **Organ logs** | `~/.cynic/organs/{id}/` | {task_id, tool, args, result, timestamp} | ~50/min per organ | Real-time |
| **Agent tasks** | SurrealDB (agent_tasks) | {id, domain, status, reasoning, outcome} | ~20/min | Real-time |
| **Human behavior** | Not yet instrumented | {click, scroll, timestamp, viewport} | ~1000/hour if enabled | Real-time |

**Objectif:** Capter le signal sans le déformer. Chaque source est APPEND-ONLY (jamais modifiée après création).

---

## Layer 2: TRANSFORMATION (Traitement)

**What:** Clean, normalize, aggregate into exploitable form

### Cleaning (Nettoyage)

```python
# Pseudocode: what Askesis must do
def clean_observations(raw_obs: List[Observation]) -> List[Observation]:
    """Remove noise, normalize structure"""
    
    # 1. Remove duplicates (same observation logged twice)
    deduped = deduplicate_by_timestamp_domain_context()
    
    # 2. Interpolate missing timestamps
    # (node offline 5min, then heartbeat resumes)
    
    # 3. Normalize domains (lowercase, expand abbreviations)
    # "token_judgment" → "token" + "judgment"
    
    # 4. Remove outliers
    # (1000ms latency when all others are 50ms = anomaly, flag it)
    
    return cleaned_obs
```

### Aggregation (Agrégation)

```python
def aggregate_failures(observations: List[Observation]) -> Dict[str, FailurePattern]:
    """Group failures by type, calculate frequency"""
    
    patterns = {}
    for obs in observations:
        if obs.context == "failure":
            key = (obs.domain, obs.failure_type)  # ("token_judgment", "timeout")
            if key not in patterns:
                patterns[key] = FailurePattern(type=key, count=0, examples=[])
            patterns[key].count += 1
            patterns[key].examples.append(obs)
    
    return patterns
```

### Normalization (Normalisation)

```python
def normalize_timestamps(obs: Observation) -> Observation:
    """All timestamps to UTC, millisecond precision"""
    obs.timestamp = parse_timestamp(obs.timestamp).to_utc()
    return obs
```

**Objectif:** Passer du brut au signal. Réduire bruit, remplir lacunes, standardiser format.

---

## Layer 3: STRUCTURATION (Modélisation)

**What:** Organize into normalized schema, eliminate redundancy (3NF)

### Core Schema (SurrealDB)

```surql
-- Observations (K15 source)
DEFINE TABLE observations SCHEMAFULL;
  field tool: string;           -- "kernel", "node-gpu-1", "hermes"
  field domain: string;          -- "token", "wallet", "chess", "twitter"
  field target: string;          -- "judge", "dog_router", "circuit_break"
  field context: string;         -- "latency_spike", "dog_timeout", etc.
  field timestamp: datetime;     -- ISO 8601
  field tags: array<string>;     -- ["failure", "recoverable", "logged"]
  field severity: string;        -- "info", "warning", "critical"

-- Events (Kernel decisions)
DEFINE TABLE events SCHEMAFULL;
  field verdict_id: string;      -- FK to verdicts table
  field domain: string;          -- context of judgment
  field dogs_used: array<string>;-- ["qwen-7b", "gemini-cli"]
  field q_score: float;          -- φ-bounded confidence
  field timestamp: datetime;
  field why: string;             -- human-readable reasoning

-- Executions (Node-level)
DEFINE TABLE node_executions SCHEMAFULL;
  field node_id: string;         -- "cynic-gpu-1"
  field command: string;         -- "load_model", "switch_model"
  field duration_ms: integer;
  field hardware_before: object; -- {gpu_mem_free, cpu_util, etc.}
  field hardware_after: object;
  field result: string;          -- "success", "timeout", "error"
  field timestamp: datetime;

-- Tasks (Agent work)
DEFINE TABLE agent_tasks SCHEMAFULL;
  field agent_id: string;        -- "hermes", "inference_lab"
  field domain: string;          -- "twitter", "wallet_judgment"
  field status: string;          -- "pending", "running", "done", "failed"
  field reasoning: text;         -- why this task was assigned
  field outcome: text;           -- what the agent produced
  field timestamp: datetime;

-- Relationships (3NF: eliminate redundancy)
DEFINE TABLE dog_profiles SCHEMAFULL;
  field dog_name: string;        -- "qwen-7b", "deterministic-dog"
  field model: string;           -- "Qwen 2.5 7B"
  field backend: string;         -- "HF Inference", "llama.cpp"
  field domains: array<string>; -- specialization

-- Aggregate views (materialized)
DEFINE TABLE failure_summary AS
  SELECT 
    domain,
    failure_type,
    count(*) as frequency,
    avg(duration_ms) as avg_recovery_time
  FROM node_executions
  WHERE result != "success"
  GROUP BY domain, failure_type;
```

### Session Log Schema (JSONL → Askesis)

```jsonl
{
  "session_id": "b696d837-c825-416a-b8a8-10f1f89193ae",
  "date": "2026-04-30",
  "turn": 42,
  "timestamp": "2026-04-30T12:45:00Z",
  "user_intent": "refactor Phase B as Soma's command layer",
  "claude_decision": "encode distributed topology as future design",
  "tools_used": ["Read", "Edit", "Agent"],
  "code_changed": true,
  "files_touched": [
    "docs/architecture/CYNIC-NODE-PHASE-B-LIFECYCLE.md",
    "docs/architecture/CYNIC-ORGANISM-DISTRIBUTED-ROADMAP.md"
  ],
  "outcome": "Phase B redesigned, topology decision deferred",
  "confidence": 0.618
}
```

**Objectif:** Éliminer redondance (3NF). Chaque fait existe une fois. Jointures explicites.

---

## Layer 4: ANALYSIS & COMPREHENSION (Analyse)

**What:** Make data understandable and interpretable

### Statistics (Statistiques)

```python
# What Askesis computes
def compute_statistics(obs: List[Observation]) -> Dict:
    """Describe the dataset"""
    return {
        "total_observations": len(obs),
        "observation_types": Counter([o.context for o in obs]),
        "domains": Counter([o.domain for o in obs]),
        "timestamp_range": (min_timestamp, max_timestamp),
        "failure_rate": sum(1 for o in obs if "failure" in o.tags) / len(obs),
    }
```

### Visualization (Visualisation)

```python
# What the UI should display
def visualize_failures(failures: Dict[str, FailurePattern]):
    """Histogram of failure modes"""
    # X-axis: failure type (timeout, OOM, network)
    # Y-axis: count
    # Color: severity (info/warning/critical)
    # Tooltip: examples, recovery time
```

### Key Metrics

| Metric | Meaning | Target | Triggers |
|--------|---------|--------|----------|
| **Observation density** | Events/min logged | >50/min | <50: "system quiet" |
| **Failure rate** | % obs tagged "failure" | <5% | >10%: alert |
| **Decision latency** | ms from Dog query to verdict | <100ms | >500ms: investigate |
| **Dog agreement** | % where 3 Dogs agree | >70% | <50%: Dogs broken |
| **Recovery rate** | % of failures auto-recovered | >80% | <60%: manual intervention needed |

**Objectif:** Rendre les données compréhensibles. Patterns visuellement évidentes.

---

## Layer 5: LEARNING (Apprentissage)

**What:** Extract patterns from aggregated data

### Clustering (Groupement)

```python
def cluster_failures(failures: Dict[str, FailurePattern]) -> Dict[str, Cluster]:
    """Group similar failures by root cause"""
    # K-means on (failure_type, domain, recovery_time)
    # Result: "GPU OOM cluster", "Network timeout cluster", etc.
```

### Pattern Extraction (Extraction de patterns)

```python
def extract_patterns(logs: List[LogEntry]) -> List[Pattern]:
    """Find recurring behaviors"""
    
    # Example 1: Session pattern
    # "User reads error → searches grep → finds root cause → fixes"
    # Frequency: 15/20 sessions
    # Label: "read→search→fix" = debugging cycle
    
    # Example 2: Execution pattern
    # "GPU memory creeps up → latency spike → model switches"
    # Frequency: 8/10 node restarts
    # Label: "memory leak → cascade"
    
    patterns = []
    for log_sequence in find_sequences(logs):
        if frequency(log_sequence) > threshold:
            patterns.append(Pattern(sequence=log_sequence, freq=frequency))
    
    return patterns
```

### Recommendation Generation

```python
def recommend_changes(patterns: List[Pattern]) -> List[Recommendation]:
    """From patterns, suggest improvements"""
    
    recs = []
    
    # Pattern: "Deterministic Dog always BARK on token_judgment"
    # Recommendation: "Train specialized Dog for token domain"
    
    # Pattern: "GPU memory creeps every 2h"
    # Recommendation: "Add garbage collection at 1.5h mark"
    
    # Pattern: "Users debug for 45min before finding root cause"
    # Recommendation: "Improve error messages (add context)"
    
    return recs
```

**Objectif:** Découvrir structures cachées. Les patterns que personne n'a codées.

---

## Layer 6: RELIABILITY (ACID, Transactions)

**What:** Ensure data survives, remains coherent, is auditable

### Atomicity (Atomicité)

```rust
// When emitting an observation, all-or-nothing
#[async_trait]
impl ObservationPort for SurrealDBObservation {
    async fn emit(&self, obs: Observation) -> Result<()> {
        let mut tx = db.begin_transaction().await?;
        
        // Insert observation
        tx.query("INSERT INTO observations $obs").bind(obs).await?;
        
        // Increment domain counter (aggregate view)
        tx.query(
            "UPDATE domain_stats SET count += 1 WHERE domain = $domain"
        )
        .bind(&obs.domain)
        .await?;
        
        // Commit all or rollback
        tx.commit().await?;
        Ok(())
    }
}
```

### Durability (Durabilité)

```bash
# Logs are append-only, persisted to disk
# JSONL files: ~/.cynic/logs/node-gpu-1.jsonl (never overwritten)
# SurrealDB: persistent store (on disk, replicated if needed)

# Historical guarantee: can always go back to original data
# "What was the exact observation at 2026-04-30 12:45:00?"
# Answer: tail -f node-gpu-1.jsonl | grep 2026-04-30T12:45
```

### Isolation (Isolation)

```sql
-- Transactions don't interfere
-- Session A: reads failures (consistent snapshot)
-- Session B: writes new observations (buffered)
-- No dirty reads, phantom reads prevented by transaction isolation level
```

### Coherence (Cohérence)

```sql
-- Constraints enforce consistency
DEFINE TABLE observations SCHEMAFULL;
  field domain ASSERT domain IN ["token", "wallet", "chess", "twitter"];
  field severity ASSERT severity IN ["info", "warning", "critical"];
  
-- Invalid data is rejected at write time
```

**Objectif:** Données survivent aux pannes. Audit trail intact.

---

## Layer 7: OBSERVABILITY & GOVERNANCE (Gouvernance)

**What:** Track where data comes from, who uses it, control access

### Data Lineage (Traçabilité)

```
human_strategy.json
  ← decision_patterns.json
    ← session_logs.jsonl
      ← Claude Code /observe endpoint
    ← kernel_observations
      ← /observe handler (kernel/api/observe.rs)
        ← Dog verdicts + Soma decisions
```

### Audit Trail (Audit)

```sql
-- Who, what, when, why
DEFINE TABLE data_access_log SCHEMAFULL;
  field user: string;        -- "gemini", "human-review"
  field table_name: string;  -- "observations", "events"
  field action: string;      -- "read", "write", "aggregate"
  field timestamp: datetime;
  field count: integer;      -- rows accessed
```

### Versioning (Versioning)

```bash
# Datasets are versioned, not overwritten
artifacts/
  ├── heuristics_v1.0.json    (from 2026-04-15)
  ├── heuristics_v1.1.json    (from 2026-04-20)
  ├── heuristics_v1.2.json    (from 2026-04-30, current)
  │
  └── dogs_agreement_v2.3.json

# Kernel loads: heuristics_v1.2.json
# Can always revert: load heuristics_v1.1.json
```

### Access Control (Contrôle d'accès)

```sql
-- Only authorized readers
GRANT SELECT ON observations TO role "askesis_reader";
GRANT SELECT ON events TO role "soma_reader";
GRANT WRITE ON events TO role "kernel_writer";

-- Audit who read what
```

**Objectif:** Contrôler, comprendre, tracer l'usage des données.

---

## Integration: Full Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│ LAYER 1: PERCEPTION                                         │
│ ├── Sessions (~1000 turns/session)                          │
│ ├── Observations (~100/min)                                 │
│ ├── Node logs (~10/min per node)                            │
│ ├── Agent tasks (~20/min)                                   │
│ └── [APPEND-ONLY: never modified after creation]            │
└────────┬────────────────────────────────────────────────────┘
         │
┌────────v────────────────────────────────────────────────────┐
│ LAYER 2: TRANSFORMATION                                     │
│ ├── Clean (deduplicate, normalize, interpolate)             │
│ ├── Aggregate (group failures, sum latencies)               │
│ └── Normalize (timestamps UTC, domains lowercase)           │
└────────┬────────────────────────────────────────────────────┘
         │
┌────────v────────────────────────────────────────────────────┐
│ LAYER 3: STRUCTURATION                                      │
│ ├── SurrealDB schema (observations, events, tasks)           │
│ ├── 3NF (eliminate redundancy)                              │
│ └── Relationships (dog_profiles, node_status)               │
└────────┬────────────────────────────────────────────────────┘
         │
┌────────v────────────────────────────────────────────────────┐
│ LAYER 4: ANALYSIS                                           │
│ ├── Statistics (failure_rate, observation_density)          │
│ ├── Visualization (histograms, timelines)                   │
│ └── Metrics dashboard (live, queryable)                     │
└────────┬────────────────────────────────────────────────────┘
         │
┌────────v────────────────────────────────────────────────────┐
│ LAYER 5: LEARNING                                           │
│ ├── Clustering (failure modes, decision patterns)           │
│ ├── Pattern extraction (debugging cycle, memory leak)        │
│ └── Recommendations (train Dog, improve error msgs)         │
└────────┬────────────────────────────────────────────────────┘
         │
┌────────v────────────────────────────────────────────────────┐
│ LAYER 6: RELIABILITY                                        │
│ ├── Atomicity (all-or-nothing observation emission)         │
│ ├── Durability (append-only logs, persistent storage)       │
│ ├── Isolation (transactions don't interfere)                │
│ └── Coherence (constraints enforce valid states)            │
└────────┬────────────────────────────────────────────────────┘
         │
┌────────v────────────────────────────────────────────────────┐
│ LAYER 7: GOVERNANCE                                         │
│ ├── Data lineage (where did this pattern come from?)        │
│ ├── Audit trail (who accessed what, when)                   │
│ ├── Versioning (datasets are immutable, versioned)          │
│ └── Access control (role-based permissions)                 │
└────────┬────────────────────────────────────────────────────┘
         │
    ┌────v──────────────────────────────────────────┐
    │ CONSUMERS (Feedback Loops)                    │
    ├─→ KERNEL (load heuristics, inject crystals)  │
    ├─→ SOMA (allocation decisions)                │
    ├─→ AGENTS (learn from failures)               │
    ├─→ GEMINI (analyze reasoning chains)          │
    └─→ HUMAN (weekly reflection UI)               │
```

---

## Phase Implementation (Phased)

### Phase 1: Core Pipeline (NOW → May 10)

**Goal:** One data loop working end-to-end

**Path:** Session logs → Askesis (Layer 2+3) → Reflection → Human reads

**What to build:**
1. `ClaudeCodeLogStore` (read from ~/.claude/projects)
2. Askesis Layer 2 (clean, normalize)
3. Askesis Layer 3 (schema for session patterns)
4. Simple Layer 4 (count failures, show histogram)
5. Output: weekly-reflection.md

**No blocker.** Data exists.

### Phase 2: Expand Sources (May 10 → June)

**Goal:** All sources feeding Askesis

**Path:** +Kernel observations +Node logs +Agent tasks → aggregated Askesis

**What to build:**
1. `SurrealDBLogStore` (read observations)
2. Node log schema (hardware, commands, failures)
3. Layer 2 (aggregate failures by type)
4. Layer 5 (cluster failure modes)
5. Output: failure_summary.json

**Blocker:** Node must emit structured logs

### Phase 3: Feedback Loops (June+)

**Goal:** Datasets change system behavior

**Path:** Patterns → Heuristics → Kernel reload + Soma decisions

**What to build:**
1. Layer 5 (real clustering, recommendations)
2. Feedback loop (Askesis → heuristics.json)
3. Kernel hot-reload (watch heuristics_vX.json)
4. Soma uses failure_summary.json for allocation

**Blocker:** Kernel must reload config without restart

---

## Falsification

**Phase 1 is falsified if:**
- Session logs aren't parsed into structured form
- Reflection exists but humans don't read it
- No dataset output (data is read but not exposed)

**Phase 2 is falsified if:**
- Node logs are collected but not queryable (still siloed)
- Failures aren't aggregated (each treated as isolated)
- No clear failure taxonomy (patterns are vague)

**Phase 3 is falsified if:**
- Datasets exist but don't change decisions
- Feedback loop is one-way (humans read, don't write back)
- System doesn't behave differently after learning

---

## Summary

**CYNIC data system = disciplined application of canonical architecture:**

1. **PERCEPTION:** Append-only sources (sessions, obs, nodes, agents)
2. **TRANSFORMATION:** Clean, aggregate, normalize
3. **STRUCTURATION:** SurrealDB 3NF schema
4. **ANALYSIS:** Stats, metrics, dashboards
5. **LEARNING:** Clustering, patterns, recommendations
6. **RELIABILITY:** ACID transactions, audit trails
7. **GOVERNANCE:** Lineage, versioning, access control

**Success metric:** Datasets reveal patterns that drive decisions that improve system behavior.

This is not about inference quality. It's about **data metabolism**. Intelligence is the edge product.

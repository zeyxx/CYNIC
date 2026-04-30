# CYNIC as a Data System — Course Concepts ↔ Production Implementation

> Every concept from the data systems curriculum has a concrete equivalent in CYNIC.
> CYNIC is not an illustrative project — it is a production system with real data, real bugs, and real measurements.

---

## 1. PERCEPTION (Data Acquisition)

### Course
- CSV files with `pd.read_csv()`
- PHP cookies `$_COOKIE`
- Sessions `session_start()`

### CYNIC
| Sensor | Source | File | Data captured |
|--------|--------|------|---------------|
| observe-tool.sh | Claude Code PostToolUse | `.claude/hooks/observe-tool.sh` | Every action: file edited, command executed, search launched |
| observe-prompt.sh | Claude Code UserPromptSubmit | `.claude/hooks/observe-prompt.sh` | Developer questions/instructions |
| observe-subagent.sh | Claude Code SubagentStart | `.claude/hooks/observe-subagent.sh` | AI agent dispatches (type, objective) |
| session-init.sh | SessionStart | `.claude/hooks/session-init.sh` | System state at startup (kernel, git, crystals) |
| session-stop.sh | Stop | `.claude/hooks/session-stop.sh` | Session summary (commits, duration, compliance) |
| Hermes (Organ X) | Web browser (X/Twitter) | `scripts/hermes-x/` | Tweets, engagement, social signals |
| Tailscale probes | Mesh network | `cynic-kernel/src/backends/` | GPU/CPU node state, latency |

**Difference from course:** CYNIC does not read CSVs — it captures in real-time via hooks (event-driven, not batch). Equivalent to an IoT system where every tool is a sensor.

---

## 2. INGESTION

### Course
- **Batch:** periodic loading (CSV)
- **Streaming:** real-time data flow

### CYNIC
| Mode | Implementation | Throughput |
|------|---------------|------------|
| Streaming | `POST /observe` (fire-and-forget, async) | Every action = 1 observation |
| Batch | Nightshift (cron, periodic evaluations) | ~20 verdicts/cycle |
| Hybrid | `D*_curated.jsonl` (manually enriched static files) | 233 curated signals |

**Ingestion architecture:**
```
Hook (bash) → curl POST /observe → Kernel REST API → SurrealDB
                                         ↓
                              Validation (JSON schema)
                              Domain derivation (path → domain)
                              Tags derivation (tool → action type)
```

**Reliability trade-off:** Fire-and-forget with `--max-time 2`. If the kernel is down, the observation is lost (trade-off: no local queue, zero hook latency). The course would say "data loss risk" — CYNIC accepts this loss because observations are reproducible (same action = same observation).

---

## 3. STORAGE

### Course
- Raw (unmodified)
- Clean (validated)
- Curated (enriched)
- "Never lose the raw data"

### CYNIC
| Layer | SurrealDB table | Content | Retention |
|-------|----------------|---------|-----------|
| **Raw** | `observations` | All raw observations (1498+) | Unlimited |
| **Processed** | `verdicts` | Dog judgments (q_score, axiom scores) | Unlimited |
| **Curated** | `crystals` | Crystallized knowledge (forming → canonical) | Unlimited |
| **Static curated** | `D*_curated.jsonl` | High-confidence domain signals | Git-tracked |

**Principle upheld:** raw data (observations) is never deleted. Crystals are derived, not replacements.

**Database choice:** SurrealDB (document-graph hybrid, not SQL relational). Why not PostgreSQL? Sovereignty — SurrealDB is embeddable, no cloud dependency.

---

## 4. TRANSFORMATION (Processing)

### Course
- `df['col']`, `groupby()`, `describe()`
- Drop NaN columns, interpolation

### CYNIC
| Operation | Course equivalent | Implementation |
|-----------|------------------|----------------|
| Semantic slug | `groupby()` | Observation → semantic slug (grouping by meaning) |
| FNV hash | Index/primary key | Slug → deterministic hash → crystal ID |
| Domain gate | `dropna()` | Filter observations without domain (domain="general" = noise) |
| Quorum gate | `count() >= N` | N >= 2 concordant observations required to form a crystal |
| Epistemic gate | Statistical threshold | Filter contested crystals (disagreement between Dogs) |
| q_score computation | `mean()` / scoring | Geometric weighted mean of axiom scores |

**Transformation pipeline:**
```python
# Pseudo-code equivalent
observations = db.query("SELECT * FROM observations WHERE domain != 'general'")
grouped = observations.groupby(semantic_slug)
for slug, group in grouped:
    if len(group) >= 2:  # quorum gate
        if disagreement(group) < threshold:  # epistemic gate
            crystal = Crystal(
                content=synthesize(group),
                confidence=geometric_mean(group.scores),
                state="forming"
            )
            db.store(crystal)
```

File: `cynic-kernel/src/pipeline/crystal_observer.rs`

---

## 5. STRUCTURING (Data Modeling)

### Course
- 3NF (anti-redundancy)
- SQL joins
- Triggers, procedures, functions
- CROSS JOIN = danger

### CYNIC
| Concept | Implementation |
|---------|---------------|
| **Normalization** | Separation of observations / verdicts / crystals (3 tables, not one) |
| **Joins** | Verdict → observation (via domain + timestamp), crystal → observations (via semantic slug) |
| **Triggers** | Claude Code hooks = triggers on events (PostToolUse, SessionStart, Stop) |
| **Procedures** | `post_verdict_observation()` = procedure that POSTs a verdict as an observation |
| **Functions** | `compute_qscore()` = pure function (input → output, no side effects) |
| **CROSS JOIN danger** | K15 rule: every producer must have a consumer. Without K15, observations accumulate without being consumed (equivalent of a CROSS JOIN: combinatorial explosion of useless data) |

**Anti-pattern observed in CYNIC:** before K15 enforcement, 1498 observations produced, 0 consumed by CCM. Exactly the CROSS JOIN problem — data that exists but serves no purpose.

---

## 6. ANALYSIS & VISUALIZATION

### Course
- Matplotlib, Seaborn
- Histograms, distribution

### CYNIC
| Tool | Function | Endpoint/File |
|------|----------|---------------|
| `/status` | Kernel dashboard (Dogs, storage, latency) | `.claude/commands/status.md` |
| `/cc-status` | Harness dashboard (hooks, observers, telemetry) | `.claude/commands/cc-status.md` |
| `/health` | Real-time state (JSON) | `GET /health` |
| `/metrics` | Prometheus counters | `GET /metrics` |
| Token calibration | Confusion matrix Dogs vs CultScreener | `cynic-python/calibration/` |

**Programmatic visualization:**
```python
# cynic-python/calibration/calibrate_conviction.py
# Equivalent: histogram of Dog score distribution vs CultScreener conviction
# Result: 77.8% accuracy (token 100%, twitter/wallet need real data)
```

---

## 7. MACHINE LEARNING

### Course
- K-Means (unsupervised clustering)

### CYNIC
| ML Concept | CYNIC Implementation |
|-----------|---------------------|
| **Clustering** | Dogs = independent evaluators. Each Dog produces a score. Consensus = natural cluster. |
| **Unsupervised** | Dogs have no "good/bad" labels — they score on 6 axes (axioms) |
| **Supervised** | CultScreener conviction = ground truth for calibration (score 0-100) |
| **Ensemble** | Multi-Dog evaluation: deterministic + Qwen 7B + Qwen 9B GPU + Qwen 9B CPU |
| **Feature engineering** | Stimulus context: domain + crystal context + wisdom signals injected into Dog prompts |

**Difference from K-Means:** CYNIC uses an ensemble of models (Dogs) instead of a single algorithm. Each Dog is a "clusterer" with its own perspective. The final q_score is a geometric mean (not arithmetic — penalizes low scores more heavily).

```
q_score = geometric_mean(fidelity, phi, verify, culture, burn, sovereignty)
```

If any single axiom is 0, q_score is 0. Stricter than `mean()`.

---

## 8. DATA EXPOSITION (API)

### Course
- APIs, dashboards, web applications

### CYNIC
| Endpoint | Method | Auth | Function |
|----------|--------|------|----------|
| `/health` | GET | None (T1 gap) | System state |
| `/judge` | POST | Bearer | Submit stimulus for evaluation |
| `/observe` | POST | Bearer | Record an observation |
| `/crystals` | GET | Bearer | List crystals |
| `/verdicts` | GET | Bearer | List verdicts |
| `/metrics` | GET | None | Prometheus counters |
| `/coord/*` | POST/GET | Bearer | Multi-agent coordination |

**Full API contract:** `API.md` (OpenAPI-like, git-tracked)

---

## 9. RELIABILITY — ACID

### Course
- Atomicity, Consistency, Isolation, Durability
- COMMIT, ROLLBACK, REVOKE
- `password_hash()`

### CYNIC
| ACID Property | Implementation |
|--------------|----------------|
| **Atomicity** | K15: a verdict is either stored AND posted as observation, or neither |
| **Consistency** | Epistemic gates: a crystal cannot be "canonical" without quorum + no contestation |
| **Isolation** | Multi-cortex coord: only one agent can modify a module at a time (`/coord/claim`) |
| **Durability** | SurrealDB disk persistence + backups (`~/.surrealdb/backups/`) |

| Command | CYNIC Equivalent |
|---------|-----------------|
| COMMIT | `store_verdict()` → SurrealDB |
| ROLLBACK | Verdict rejected by epistemic gate → no crystal formed |
| REVOKE | `/coord/release` — releases claims on modules |

**Security:**
- `Bearer $CYNIC_API_KEY` on all endpoints (except /health — T1 gap)
- Secrets in `~/.cynic-env` (never committed)
- Tailscale mesh (zero-trust network, no open ports)
- Funnel exposure = documented attack surface (`workflow.md`)

---

## 10. OBSERVABILITY

### Course
- Data quality monitoring
- Query performance
- Anomaly detection

### CYNIC
| Signal | Producer | Consumer | Metric |
|--------|----------|----------|--------|
| Tool actions | observe-tool.sh | CCM intake | `cynic_observations_total` |
| User prompts | observe-prompt.sh | CCM intake | Tagged `user-prompt` |
| Agent dispatches | observe-subagent.sh | CCM intake | Tagged `agent-dispatch` |
| Dog health | `/health` | /status dashboard | Circuit state per Dog |
| Storage latency | SurrealDB | `/health` | `avg_latency_ms` |
| Verdict quality | `/judge` | `/metrics` | `cynic_verdicts_total`, q_score distribution |

**Meta-circularity:** the observability system observes itself. observe-tool.sh posts observations about its own executions. This is the equivalent of calling `describe()` on the entire system.

---

## 11. DATA GOVERNANCE

### Course
- Access management
- Traceability (data lineage)
- Versioning
- Audit

### CYNIC
| Aspect | Implementation |
|--------|---------------|
| **Access** | `settings.local.json`: 200+ allow rules, 20+ deny rules |
| **Traceability** | Every observation has: agent_id, session_id, tool_use_id, timestamp, domain, tags |
| **Lineage** | observation → verdict → crystal (fully traceable chain) |
| **Versioning** | Git (code), SurrealDB (data), crystal states (forming→crystallized→canonical) |
| **Audit** | `/coord/who` (who modifies what), state-history (hash-chained log), CC-OWNERSHIP.md |
| **Axioms** | 6 axioms = system invariants. FOGC test = verification that axioms are load-bearing |

---

## 12. ANTI-PATTERNS & RISKS

### Course
| Anti-pattern | Risk |
|-------------|------|
| Cascading triggers | Unpredictable side effects |
| Massive CROSS JOIN | Data explosion |
| Too many NaN | Biased analyses |
| Bad hashing | Compromised security |

### CYNIC (observed in production)
| Anti-pattern | Real observation | Remediation |
|-------------|-----------------|-------------|
| **K15 violation** (producer without consumer) | 1498 observations produced, 0 consumed | Wiring CCM intake, domain gates |
| **Crystal poisoning** | Hermes heartbeat in wallet-judgment domain | DB cleanup, domain gate |
| **Dog confusion** | Token-specialized Dogs judging Twitter → incoherent scores | Domain dispatch (dispatch.rs) |
| **Blind tuning** (B1) | Changing prompts without measuring before/after | Rule R7: measure before AND after |
| **Stale binary** (B2) | Code edited but never deployed | `/deploy` command, pre-push hook |
| **Build > Use** | 20 days of infrastructure without user experiment | Rule R22: USE before architecture |

---

## 13. SYSTEM INVARIANTS

### Course
- Consistency, traceability, resilience, scalability, observability

### CYNIC (6 axioms)
| Course invariant | CYNIC Axiom | Verification |
|-----------------|-------------|--------------|
| Consistency | FIDELITY | Are the Dogs telling the truth? |
| Scalability | PHI | Is the structure proportional? |
| Traceability | VERIFY | Can every claim be verified/falsified? |
| Culture | CULTURE | Are existing patterns honored? |
| Performance | BURN | Is the system efficient? |
| Autonomy | SOVEREIGNTY | Does the system preserve its independence? |

**FOGC test:** "If I replace the 6 axioms with their inverses, does any other line of code need to change?" If yes → axiom logic is leaking into infrastructure. If no → axioms are correctly isolated.

---

## 14. COMPLETE PIPELINE (course equivalent)

```
Course:
  User data → Ingestion → Raw storage → Cleaning →
  Feature engineering → Model → API → Interface

CYNIC:
  Hook fires → POST /observe → SurrealDB observations (raw) →
  CCM intake (domain gate, quorum gate) → Crystal formation (curated) →
  Dog prompt injection (feature engineering) → /judge verdict (model) →
  /health + /status + /cc-status (interface) →
  post_verdict_observation() → back to beginning (compound loop)
```

**Fundamental difference:** CYNIC is a closed-loop system. The course describes a linear pipeline (input → output). CYNIC loops: outputs (verdicts) become inputs (observations), which form crystals, which improve subsequent verdicts.

This is an autopoietic system — it produces itself.

---

## File References

| Concept | CYNIC File |
|---------|-----------|
| Perception | `.claude/hooks/observe-*.sh` |
| Ingestion | `cynic-kernel/src/api/rest/routes.rs` (POST /observe) |
| Storage | `cynic-kernel/src/domain/storage.rs` |
| Transformation | `cynic-kernel/src/pipeline/crystal_observer.rs` |
| ML / Dogs | `cynic-kernel/src/domain/dog.rs` |
| Domain dispatch | `cynic-kernel/src/domain/dispatch.rs` |
| API | `API.md` |
| Governance | `.claude/CC-OWNERSHIP.md`, `.claude/rules/` |
| Dashboard | `.claude/commands/status.md`, `.claude/commands/cc-status.md` |
| Calibration | `cynic-python/calibration/` |
| Curated data | `cynic-python/curation/D*_curated.jsonl` |

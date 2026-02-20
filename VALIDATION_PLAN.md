# CYNIC Docker Validation Plan (2026-02-20)

> "Tu n'as pas la bonne réalité de ma machine, j'ai ollama avec des models, cynic tourne sur docker et c'est sur ça qu'on doit travailler tout le temps"

**Status**: 6 Functional Layers × 2-3 Sub-tests each = ~14 hours total validation

---

## LAYER 1: PERCEPTION (4h) — What can CYNIC sense?

CYNIC has 8 perceive workers running continuously. Each should report state.

### L1.1: Filesystem Watchers (1h)
**Goal**: Verify 8 perceive workers are reporting data
**Access Point**: `GET http://localhost:8000/consciousness`
**Expected**: `percieve_workers: 8` with recent timestamps

- [ ] git_watcher: last update < 5min
- [ ] health_watcher: CPU/memory reading fresh
- [ ] market_watcher: (may be stubbed for local env)
- [ ] solana_watcher: (may be stubbed for local env)
- [ ] disk_watcher: reporting filesystem state
- [ ] memory_watcher: system memory metrics
- [ ] social_watcher: (may be stubbed)
- [ ] self_watcher: CYNIC self-state

**Test Command**:
```bash
curl -s http://localhost:8000/consciousness | python3 -m json.tool | grep -A 50 "perceive_workers"
```

### L1.2: Event Bus & Watchers Real-time (1h)
**Goal**: Verify watchers emit events continuously
**Access Point**: `GET http://localhost:8000/health` (consciousness.timers)
**Expected**: REFLEX cycles should show samples > 50

- [ ] REFLEX cycles > 50 (means perceive_workers firing)
- [ ] REFLEX health != "UNKNOWN"
- [ ] No hung workers (p50_ms reasonable)

**Test Command**:
```bash
curl -s http://localhost:8000/health | python3 -m json.tool | grep -A 30 '"REFLEX"'
```

### L1.3: Local Git State Access (1h)
**Goal**: Verify CYNIC can read repo state (essential for coding tasks)
**Access Point**: `POST http://localhost:8000/perceive` with `level: REFLEX`

**Test Payload**:
```json
{
  "level": "REFLEX",
  "include_context": true
}
```

**Expected**: guidance.json contains git status, branch info, recent commits

- [ ] Git state accessible from Docker
- [ ] Repo branch detected
- [ ] Recent commit hashes in response

### L1.4: State Persistence (1h)
**Goal**: Verify ~/.cynic files are readable and persistent
**Access Point**: Host filesystem + Docker volume mount

- [ ] ~/.cynic directory mounted in Docker
- [ ] State files not corrupted (alive.json, battles.jsonl, etc.)
- [ ] Can read consciousness.json from host

**Test Command**:
```bash
ls -la ~/.cynic/*.json | head -10
cat ~/.cynic/alive.json | python3 -m json.tool | head -20
```

---

## LAYER 2: DATA SOURCES (2h) — Can CYNIC access LLMs?

### L2.1: Ollama Model Discovery (30min)
**Goal**: Verify CYNIC discovers loaded models and can score with them
**Access Point**: `GET http://localhost:8000/introspect`
**Expected**: `llm_adapters: ["OllamaAdapter"]` with gemma2:2b available

- [ ] OllamaAdapter initialized
- [ ] Models detected: gemma2:2b, nomic-embed-text
- [ ] Can call `/v1/generate` on ollama:11434

**Test Command**:
```bash
curl -s http://localhost:8000/introspect | python3 -m json.tool | grep -A 20 "llm"
```

### L2.2: Scoring with Ollama (1h)
**Goal**: Verify CYNIC can generate judgments using Ollama
**Access Point**: `POST http://localhost:8000/judge`

**Test Payload**:
```json
{
  "subject": "Test code snippet",
  "code": "def hello(): print('world')",
  "context": "Simple Python function"
}
```

**Expected**: Q-Score returned, dogs voting, verdict (HOWL/WAG/GROWL/BARK)

- [ ] Judge endpoint returns 200
- [ ] Q-Score in valid range [0, 100]
- [ ] Verdict present (not null)
- [ ] Response time < 5s (Ollama inference)

### L2.3: Embeddings / Vector Search (30min)
**Goal**: Verify nomic-embed-text for similarity search
**Access Point**: Database queries + introspect

- [ ] nomic-embed-text model loaded
- [ ] Embeddings stored in PostgreSQL (pgvector)
- [ ] Can retrieve similar items by vector

**Test Command**:
```bash
docker compose exec postgres-py psql -U cynic -d cynic_py -c "SELECT COUNT(*) FROM vector_search LIMIT 1;"
```

---

## LAYER 3: COGNITION (3h) — Do 11 dogs vote correctly?

### L3.1: Individual Dog Scores (1h)
**Goal**: Verify each dog can score independently
**Access Point**: `POST http://localhost:8000/perceive` response (guidance.json)

**Test Input**: Commit a deliberate "bad" code (obvious style issues)
**Expected**: ANALYST, ARCHITECT, GUARDIAN should vote lower; others neutral

- [ ] ANALYST: checks code quality
- [ ] ARCHITECT: checks design/patterns
- [ ] GUARDIAN: checks security
- [ ] Others: report as expected

**Test Command**:
```bash
curl -s -X POST http://localhost:8000/perceive \
  -H "Content-Type: application/json" \
  -d '{"level":"REFLEX"}' | jq '.guidance.dogs'
```

### L3.2: Dog Consensus (1h)
**Goal**: Verify φ-bounded consensus mechanism works
**Access Point**: `/health` (consciousness active_level)

**Expected**:
- Active level: MACRO (all dogs consensus)
- Gradient: visible progression (REFLEX→MICRO→MACRO→META)
- Q-Score clamped at φ⁻¹ = 61.8% max for confidence

- [ ] No Q-Score > 100
- [ ] No confidence > 61.8%
- [ ] Consensus achievable (level progression)

### L3.3: Dog Learning / Feedback Loop (1h)
**Goal**: Verify Q-Table learns from feedback
**Access Point**: `POST http://localhost:8000/feedback`

**Test**:
1. Submit judgment with Q-Score 50
2. Send feedback: rating -1 (bad), correction provided
3. Re-query same state → Q-Score should shift

**Expected**: Q-Table updated, learning_signals recorded

- [ ] Feedback endpoint accepts POST
- [ ] Q-Table state changes after feedback
- [ ] Temporal Decay visible (older judgments less influenced)

---

## LAYER 4: DECISION (2h) — Can CYNIC propose & choose actions?

### L4.1: DecideAgent Proposals (1h)
**Goal**: Verify CYNIC proposes actions from judgments
**Access Point**: `GET http://localhost:8000/actions`

**Expected**: Pending actions list with priority, type, context

- [ ] Actions queued: INVESTIGATE, REFACTOR, ALERT, MONITOR
- [ ] Priority assigned (1=highest)
- [ ] Metadata present (reasoning, confidence)

**Test Command**:
```bash
curl -s http://localhost:8000/actions | python3 -m json.tool | head -30
```

### L4.2: Action Acceptance / Rejection (1h)
**Goal**: Verify human-in-the-loop decision gating
**Access Point**: `POST http://localhost:8000/actions/{id}/accept` or `/reject`

**Test**:
1. Get action ID from /actions
2. POST accept
3. Check status → should be EXECUTING or EXECUTED

**Expected**: Action marked, feedback recorded to Q-Table

- [ ] Accept endpoint works (200 response)
- [ ] Status changes to EXECUTING
- [ ] Learning signal recorded

---

## LAYER 5: ACTION (1h) — Can CYNIC execute via Claude Code?

### L5.1: ClaudeCodeRunner Integration (1h)
**Goal**: Verify CYNIC can invoke Claude Code CLI
**Access Point**: `/act/execute` (requires Claude CLI installed)

**Prerequisites**:
- `claude --version` returns successfully
- `claude --help` works

**Test**:
```json
POST /act/execute
{
  "action_id": "test-001",
  "action_type": "INVESTIGATE",
  "prompt": "List 5 Python files in the current repo"
}
```

**Expected**:
- Response: `success: true`, `output: "..."`
- CYNIC spawned Claude Code subprocess
- Got back result

- [ ] Claude CLI accessible from Docker
- [ ] Subprocess execution works
- [ ] Output captured and returned

---

## LAYER 6: LEARNING (2h) — Does CYNIC improve over time?

### L6.1: Q-Table Persistence (1h)
**Goal**: Verify Q-Table survives restarts
**Access Point**: PostgreSQL + QTable snapshots

**Test**:
1. Check Q-Table state before restart: `SELECT COUNT(*) FROM qtable;`
2. Kill containers, restart: `docker compose restart`
3. Check Q-Table state after: `SELECT COUNT(*) FROM qtable;`

**Expected**: Rows preserved, learning continues

- [ ] PostgreSQL persists across restarts
- [ ] Q-Table entries not lost
- [ ] Total_updates counter increments

### L6.2: Learning Curves (1h)
**Goal**: Verify Thompson Sampling + EWC improve scores over time
**Access Point**: `/mirror` (decision trace + learning metrics)

**Test**: Submit 10 judgments with feedback, check convergence

**Expected**:
- Early judgments: high variance
- Later judgments: lower variance
- Confidence increases as visits increase

- [ ] /mirror endpoint available
- [ ] Learning curve metrics visible
- [ ] Thompson Sampling working (Q-Score variance decreases)

---

## Summary

**Total Time**: ~14 hours (2 hours/layer)

**Critical Blockers** (if any fail):
- L2.1: No Ollama adapters → can't score (BLOCKER)
- L5.1: No Claude CLI → can't execute actions (BLOCKER)
- L1.1: No perceive workers → no sensing (BLOCKER)

**Next Steps After Validation**:
1. **All L1-L6 GREEN** → CYNIC ready for empirical MCP testing
2. **L2-L5 RED** → Identify gaps, file issues
3. **Confidence Assessment** → Update φ-bounded confidence based on actual results

---

**Last Updated**: 2026-02-20
**Environment**: Docker (cynic:latest, ollama:latest, pgvector:pg16)
**Target Confidence**: 61.8% (φ⁻¹) after full validation

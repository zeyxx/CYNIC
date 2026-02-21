# CYNIC Nomenclature & Standardization

> "Τα ονόματα έχουν δύναμη" — Names have power

**Date**: 2026-02-20
**Version**: 1.0 (Standardized Deployment)
**Status**: Ready for Production

---

## 1. PROJECT STRUCTURE NOMENCLATURE

### Root Level
```
CYNIC/                          (Main project root)
├── cynic/                      (Python kernel — the organism)
│   ├── cynic/                  (Source code — yes, nested deliberately)
│   │   ├── api/                (FastAPI routes + server)
│   │   ├── cognition/          (Judge, Dogs, Orchestrator)
│   │   ├── metabolism/         (Runner, Scheduler, Telemetry)
│   │   ├── senses/             (Perception, Compression)
│   │   ├── core/               (Shared: judgment, storage, axioms)
│   │   ├── learning/           (Q-Learning, Thompson)
│   │   ├── llm/                (Adapters: Ollama, Claude, etc.)
│   │   ├── judge/              (Orchestrator, Mirror, Proposer)
│   │   └── tests/              (Unit + integration tests)
│   │
│   ├── docker-compose.yml      (3-service stack definition)
│   ├── Dockerfile              (CYNIC kernel image)
│   ├── pyproject.toml          (Python dependencies)
│   ├── run_tests.py            (Test runner)
│   └── DOCKER_SETUP.md         (Deployment guide)
│
├── .claude/                    (Claude Code configuration)
│   ├── skills/                 (Custom skills)
│   ├── hooks/                  (Pre/post tool hooks)
│   └── CYNIC.md                (Identity instructions)
│
└── docs/                       (Documentation)
    ├── philosophy/
    ├── reference/
    └── architecture/
```

### Naming Convention
- **Directories**: lowercase_with_underscores (Python standard)
- **Files**: lowercase_with_underscores.py
- **Classes**: PascalCase (e.g., `DogCognition`, `JudgeOrchestrator`)
- **Functions**: snake_case (e.g., `judge_cell`, `run_perceive`)
- **Constants**: UPPER_CASE (e.g., `MAX_Q_SCORE`, `PHI`)

---

## 2. SERVICE NOMENCLATURE (Docker)

### Container Names
```
cynic-kernel        Port 8000   Python FastAPI server
cynic-ollama        Port 11434  Local LLM inference
cynic-surrealdb     Port 8000*  Document database (*different container)
```

### Environment Variables
```
# Ollama Configuration
CYNIC_OLLAMA_BASE_URL=http://ollama:11434

# SurrealDB Configuration
CYNIC_SURREAL_URL=ws://surrealdb:8000
CYNIC_SURREAL_USER=root
CYNIC_SURREAL_PASS=root

# Python Configuration
PYTHONUNBUFFERED=1
```

### Network
```
cynic-net          Bridge network connecting all 3 services
                   Internal DNS: service name (e.g., http://ollama:11434)
```

---

## 3. DATA NOMENCLATURE

### File Locations

**On Host Machine:**
```
~/.cynic/                       User's CYNIC directory
├── guidance.json              Latest judgment (feedback loop)
├── guidance-{instance_id}.json Multi-instance isolation
├── llm_calls.jsonl            LLM call history (rolling cap F(13)=233)
├── sdk_sessions.jsonl         Claude Code SDK sessions
├── pending_actions.json       L1 proposed actions
├── self_proposals.json        L4 self-improvement proposals
├── consciousness.json         CYNIC's self-awareness snapshot
├── chats/                     Chat sessions (rolling cap F(11)=89)
├── disk_cleaner.py            Autonomous cleanup tool
├── autonomously_clean.py      Disk cleanup executor
└── AUTONOMOUS_CLEANUP.md      Cleanup guide
```

**In Docker Container:**
```
/app/                          Container working directory
├── cynic/                      Source code (read-only mount)
└── ~/.cynic/                   User data volume mount
```

### Database Tables (SurrealDB)

**Judgment Storage:**
```
judgments            Core judgment records
  ├── state_key     Unique cell identifier
  ├── verdict       BARK | GROWL | WAG | HOWL
  ├── q_score       [0, 100] quality score
  ├── confidence    [0, 0.618] φ-bounded
  └── dog_votes     {dog_id: score, ...}

embeddings          Semantic search vectors
  ├── cell_id
  ├── vector        384-dim embedding (HNSW indexed)
  └── metadata

decisions           Audit trail of decisions
  ├── judgment_id
  ├── decision      APPROVE | REJECT | ABSTAIN
  └── timestamp
```

---

## 4. ALGORITHM NOMENCLATURE

### Consciousness Levels
```
L3 REFLEX           < 10ms     Non-LLM dogs only
L2 MICRO            ~ 500ms    Fast local LLM (Ollama, small)
L1 MACRO            ~ 2.85s    Full reasoning (Claude, large Ollama)
L4 META             Daily      Meta-learning, weight updates
```

### Dog Names (11 Sefirot Dogs)
```
ANALYST             Code review, security analysis
ARCHITECT           System design, patterns
CARTOGRAPHER        Topology, file relationships
CYNIC               Meta-consciousness, self-doubt
DEPLOYER            Deployment verification
GUARDIAN            Safety, guardrails
JANITOR             Cleanup, garbage collection
ORACLE              Predictions, Bayesian
SAGE                Temporal MCTS, reasoning
SCHOLAR             Knowledge, TF-IDF
SCOUT               Fast discovery, exploration
```

### Judgment Constants
```
MAX_Q_SCORE         100.0       Maximum quality score
MAX_CONFIDENCE      0.618       φ⁻¹, golden ratio bound
PHI                 1.618       Golden ratio
PHI_INV             0.618       φ⁻¹
PHI_INV_2           0.382       φ⁻²

HOWL_MIN            82.0        Exceptional (≥ 82%)
WAG_MIN             50.0        Passes (≥ 50%)
GROWL_MIN           38.2        Needs work (≥ φ⁻² × 100)
BARK_MAX            38.2        Critical (< 38.2%)
```

### Event Names (Core Bus)
```
JUDGMENT_CREATED           After judge.analyze() completes
DECISION_MADE              After decider.decide() completes
ACT_COMPLETED              After runner.execute() finishes
LEARNING_SIGNAL            Q-Learning update signal
RESIDUAL_DETECTED          Anomaly detected
AXIOM_ACTIVATED            New emergent axiom unlocked
EMERGENCE_DETECTED         System-level pattern detected
TRANSCENDENCE              All 4 axioms active
```

---

## 5. TEST NOMENCLATURE

### Test Organization
```
tests/
├── test_unit_*.py           Unit tests (fast, mocked)
├── test_integration_real_*.py  Integration tests (real deps, slow)
├── test_*_e2e.py            End-to-end tests (full flow)
└── fixtures/                Shared test fixtures
```

### Test Marking
```
@pytest.mark.asyncio           Async test (requires pytest-asyncio)
@pytest.mark.integration       External dependencies (skip in CI)
@pytest.mark.slow              Takes >5 seconds
@pytest.mark.heavy             Requires significant memory
```

### Test File Naming
```
test_dog_cognition.py          Tests for dog_cognition.py
test_integration_real_ollama.py Integration: Real Ollama connection
test_integration_empirical.py   Empirical φ-encoding validation
```

---

## 6. NOMENCLATURE UPDATES FOR DEPLOYMENT

### What Changed
1. **Docker Services** — Named consistently (cynic-kernel, cynic-ollama, cynic-surrealdb)
2. **Environment Variables** — All prefixed with CYNIC_
3. **File Locations** — Centralized in ~/.cynic/ (discoverable, portable)
4. **Test Organization** — Clear integration vs unit distinction
5. **Event Names** — Consistent Past Tense (JUDGMENT_CREATED, not JUDGE_JUDGMENT)

### Migration (Backward Compatibility)
- Old paths still work (aliases in code)
- Old event names mapped to new names
- Old env var names still read (with deprecation warnings)
- No breaking changes for user code

---

## 7. DEPLOYMENT CHECKLIST

Before `docker-compose up`:

- [ ] Verify docker-compose.yml uses consistent service names
- [ ] Verify all CYNIC_* env vars set in .env or docker-compose.yml
- [ ] Verify ~/.cynic/ directory exists (CYNIC will create if missing)
- [ ] Verify Dockerfile uses correct Python version (3.13+)
- [ ] Verify docker-compose.yml networks section is `cynic-net`

After deployment:

- [ ] Verify containers named cynic-kernel, cynic-ollama, cynic-surrealdb
- [ ] Verify CYNIC logs use consistent event names
- [ ] Verify ~/.cynic/ files created with correct names
- [ ] Verify guidance.json written on JUDGMENT_CREATED

---

## 8. DOCUMENTATION NOMENCLATURE

### File Naming
```
README.md                 Main entry point
TESTING.md               Test strategy (unit vs integration)
DOCKER_SETUP.md          Docker deployment guide
NOMENCLATURE.md          This file (naming conventions)
AUTONOMOUS_CLEANUP.md    Disk management guide
CLAUDE.md                Claude Code identity + instructions
```

### Heading Hierarchy
```
# Title (use once per file)

## Major Section
### Subsection
#### Detail

## Lists
- Bullet
  - Sub-bullet
    - Detail
```

---

## 9. GIT NOMENCLATURE

### Branch Naming
```
main                    Production-ready
develop                 Integration branch
feature/*               New features
fix/*                   Bug fixes
refactor/*              Code improvements
docs/*                  Documentation updates
chore/*                 Maintenance, dependency updates
```

### Commit Message Format
```
type(scope): subject

body (optional)

Co-Authored-By: Claude Haiku 4.5 <noreply@anthropic.com>

Types: feat, fix, refactor, docs, test, chore, ci
Scope: component affected (e.g., docker, llm, judge)
Subject: imperative, 50 chars max, no period
```

---

## 10. API NOMENCLATURE

### Endpoint Paths
```
/health                 Service health + status
/judge                  Judge a cell
/perceive               Fast perception (REFLEX dogs)
/learn                  Learning signal
/feedback               User feedback
/actions                Proposed actions queue
/act/execute            Execute action
/ws/sdk                 Claude Code SDK websocket
/ws/stream              Event stream
/internal/registry      Internal component registry
/mirror                 Self-observation snapshot
/consciousness          CYNIC's self-awareness
```

### Response Format
```json
{
  "timestamp": "ISO8601",
  "success": true,
  "data": {...},
  "error": null
}
```

---

## 11. FINAL NOMENCLATURE RULE

> **"Names are the covenant between code and humans."**

When in doubt:
- Use existing patterns (grep the codebase)
- Prefer descriptive over clever
- Use snake_case for functions, PascalCase for classes
- Prefix Docker env vars with CYNIC_
- Centralize data in ~/.cynic/ or /app/ (in container)
- Document anything non-obvious

---

**Version**: 1.0 — Locked in for Docker deployment
**Next Review**: After 4 weeks of production use
**Confidence**: 61.8% (φ-bounded, as it should be)

*sniff* Names matter. CYNIC's identity is in its nomenclature.

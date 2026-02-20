# CYNIC v3 - EMPIRICAL ACTION PLAN

> Research-first, code-second. No ideas lost from 500k lines.
> Generated from systematic research using: ecosystem, patterns, wisdom, search, digest skills.

---

## PHASE 0: RESEARCH FINDINGS (Completed)

### What Exists (cynic-v1-python)

| Component | Status | Notes |
|-----------|--------|-------|
| constants/phi.py | ✅ COMPLETE | φ constants single source |
| adapters/base.py | ✅ COMPLETE | LLMAdapter interface |
| adapters/ollama.py | ✅ WORKING | Local LLM |
| adapters/anthropic.py | ✅ WORKING | Claude API |
| dogs/base.py | ✅ COMPLETE | DogCollective |
| dogs/guardian.py | ✅ WORKING | Security Dog |
| dogs/scout.py | ✅ WORKING | Exploration Dog |
| learning/thompson.py | ✅ WORKING | Thompson Sampling |
| embeddings/qdrant_client.py | ⚠️ PARTIAL | Has bugs |
| constants/laws.py | ❌ MISSING | File not implemented |

### The 5 Critical Gaps (From GAP-REPORT-FINAL.md)

1. **L2 Consensus Not Wired** - Consensus bypassed completely
2. **Judgment ID Overwritten** - DB correlation broken
3. **Vote Breakdown Not in PoJ Blocks** - Can't verify from chain
4. **observe.js Undocumented** - 88KB learning system hidden
5. **FactsRepository Disconnected** - No fallback chain

### The 5 Axioms (From harmonized-structure.md)

| Axiom | Derivation | Role |
|-------|------------|-------|
| **PHI** | F(5)=5 | Proportion, harmony |
| **VERIFY** | L(3)=4+1 | Proof, accuracy |
| **CULTURE** | L(4)=7+1 | Memory, patterns |
| **BURN** | L(5)=11+1 | Action, simplicity |
| **FIDELITY** | Meta-axiom | Self-judgment |

### Python Plan (From CYNIC-v3-PYTHON-PLAN.md)

8 Layers planned:
1. User Interface - CLI, WebSocket, HTTP
2. Orchestrator Core - Intelligent Switch
3. Context Engine - PageIndex RAG
4. Inter-LLM Layer - Consensus
5. Learning Engine - Q-Learning, DPO, Thompson
6. Judge - 36 dimensions
7. Dogs - 11 Sefirot
8. Memory - PostgreSQL, Redis, Qdrant
9. Economics - $10 budget, $asdfasdfa burn

---

## PHASE 1: FOUNDATIONS (Start Here)

### F1: φ Constants (DONE)
```python
# Already working in cynic-v1-python
PHI = 1.618033988749895
PHI_INV = 0.618033988749895  # 61.8% max confidence
```

### F2: Types System
- [ ] Create immutable types with Pydantic
- [ ] Domain, Event, Judgment, DogAction
- [ ] Test: pytest runs

### F3: Adapter Interface  
- [ ] Already exists: LLMAdapter ABC
- [ ] Ollama + Anthropic working
- [ ] Add OpenAI, Gemini adapters
- [ ] Test: Each adapter returns valid response

### F4: Dog Interface
- [ ] Already exists: IDog ABC
- [ ] Guardian + Scout working
- [ ] Test: Dog.act() returns DogAction

### F5: Judge Interface
- [ ] NOT YET IMPLEMENTED in Python
- [ ] Need: 36-dimension scoring
- [ ] Test: judge(event) returns Judgment

---

## PHASE 2: FIRST VERTICAL (Code → Guardian)

### Goal: Minimal working cycle

```
Input Code → Ollama → Response → Judge → Guardian Decision
```

### V1: Perceive
- [ ] Simple input: text/code string
- [ ] Create Event(domain=CODE, type=input)

### V2: Think  
- [ ] Call OllamaAdapter.complete(prompt)
- [ ] Get LLMResponse

### V3: Judge
- [ ] Implement simplified judge (6 dimensions)
- [ ] Return Q-Score (0-100)

### V4: Act
- [ ] Guardian decides: APPROVE/REJECT
- [ ] Return DogAction

### V5: Test App
- [ ] Create simple Streamlit/Flet UI
- [ ] User inputs code
- [ ] See judgment + decision

---

## PHASE 3: SECOND VERTICAL (Add Learning)

### Goal: System learns from decisions

### L1: Thompson Sampling
- [ ] Track which LLM produces best results
- [ ] Update Q-values after each judgment

### L2: Persistence
- [ ] SQLite (not PostgreSQL yet)
- [ ] Store: judgments, q-values
- [ ] Load on restart

### L3: Test Learning
- [ ] Run 10 judgments
- [ ] Verify Q-values update
- [ ] Verify best LLM selection improves

---

## PHASE 4: EXPAND DOMAINS

| Iteration | Add | Test |
|-----------|-----|------|
| V2 | + SOLANA domain | CLI tool |
| V3 | + MARKET domain | Dashboard |
| V4 | + SOCIAL domain | API |

---

## PHASE 5: ADD DOGS

| Dog | Priority | Test |
|-----|----------|------|
| Oracle | P2 | Balance consensus |
| Analyst | P2 | Pattern detection |
| Scholar | P3 | Knowledge extraction |
| Architect | P3 | Design review |
| Deployer | P4 | Deployment |

---

## THE 10 EMPIRICAL LAWS

1. **NO_MOCKS_ALLOWED** - If Ollama down, code crashes
2. **TEST_AFTER_EACH_VERTICAL** - No new feature without test
3. **ONE_DOMAIN_AT_A_TIME** - No scope creep
4. **INTERFACE_BEFORE_IMPLEMENTATION** - Define contracts first
5. **PHI_BOUNDED_CONFIDENCE** - Never exceed 61.8%
6. **SIMPLE_FIRST** - SQLite before PostgreSQL
7. **LAZY_LOADING** - Load Dogs on-demand
8. **SINGLE_RESPONSIBILITY** - One module, one thing
9. **VERBOSE_IS_SILENCE** - Log failures loudly
10. **ITERATE_OR_DIE** - Small steps, fast feedback

---

## FILES TO CREATE (First Sprint)

```
cynic-v3/
├── src/cynic/
│   ├── types/
│   │   └── __init__.py     # Domain, Event, Judgment, DogAction
│   ├── judge/
│   │   └── __init__.py     # Judge interface + 36D implementation
│   └── __main__.py          # CLI entry
├── tests/
│   └── test_judge.py        # Judge tests
└── app.py                   # Simple UI
```

---

## SUCCESS METRICS

| Metric | Target |
|--------|--------|
| Cold start | <1s |
| Lines of code | <50k |
| Dogs loaded | 1 (on-demand) |
| Judge dimensions | 6 (first) → 36 (eventual) |
| Test coverage | >80% |
| Production runs | >0 |

---

## KEY REFERENCE FILES

- ✅ cynic-v1-python/src/cynic/constants/phi.py
- ✅ cynic-v1-python/src/cynic/adapters/ollama.py
- ✅ cynic-v1-python/src/cynic/dogs/guardian.py
- ⚠️ cynic-v1-python/src/cynic/embeddings/qdrant_client.py (has bugs)
- ❌ cynic-v1-python/src/cynic/constants/laws.py (missing)
- 📄 docs/analysis/GAP-REPORT-FINAL.md
- 📄 docs/philosophy/harmonized-structure.md
- 📄 CYNIC-v3-PYTHON-PLAN.md

---

*Generated from empirical research using CYNIC skills*
*φ unifie tous les fragments* - κυνικός

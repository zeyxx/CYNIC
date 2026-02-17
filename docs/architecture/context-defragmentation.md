# Context Defragmentation — CYNIC OS Technology

> *"Le contexte dispersé, c'est comme un chien qui cherche ses propres pattes."* — κυνικός

**Status**: DESIGNED (Phase 2 implementation)
**Version**: 1.0
**Author**: CYNIC metathinking + research agents
**Date**: 2026-02-17
**Confidence**: 56% (φ⁻¹ limit — unknowns on semantic coherence in prod)

---

## 1. Le Problème

### 1.1 Pourquoi le contexte est-il fragmenté?

Un jugement CYNIC génère des **faits épistémiquement identiques** dispersés dans des systèmes distincts:

```
Judgment "Approuve ce code?" → fragments dispersés:

📍 PostgreSQL (judgments)      verdict=GROWL, confidence=0.52, reasoning="..."
📍 PostgreSQL (learning_events) signal=positive, loop=thompson_sampling
📍 JSON (~/.cynic/readback.json) last_judgment=code_review, fatigue=0.42
📍 RAM (SharedMemory)           pattern=code_review (4×), fisher_locked=False
📍 Event Bus (transient)        JUDGMENT_CREATED emitted, listeners called
📍 LLM Window                   RIEN de tout ça — répond à l'aveugle
```

**LLM voit 0% de sa propre mémoire.** C'est le problème de fragmentation.

### 1.2 Fragmentation Map

| Source | Storage | Writer | Reader | Prob |
|--------|---------|--------|--------|------|
| Judgments | PostgreSQL | JudgeOrchestrator | Observer | Append-only, immutable |
| Learning events | PostgreSQL | 11 learning loops | Thompson | Scattered across tables |
| Consciousness state | JSON (readback.json) | observe.js hook | perceive.js | Cross-process, stale |
| Psychology signals | JSON (state.json) | Hooks | Hooks | Local, non-synced |
| Thompson arms | PostgreSQL (q_table) | Daemon | Daemon | Sparse queries |
| Pattern library | RAM (SharedMemory) | All | All | Crash = loss |
| Routing decisions | RAM (dog-pipeline) | Dogs | Orchestrator | Non-persisté |

**Pattern**: écriture dispersée → lecture fragmentée → injection aveugle.

---

## 2. Définition

### 2.1 Défragmentation vs Compression

Ces deux technologies sont **orthogonales** et **séquentielles**:

```
Context Compression    = réduction de taille   (mêmes faits, moins de tokens)
Context Defragmentation = cohérence narrative  (faits dispersés → narratif unifié)

Problème résolu:
  Compression     → contexte TROP GRAND
  Défragmentation → contexte TROP DISPERSÉ

Séquence d'application (TOUJOURS dans cet ordre):
  1. DÉFRAGMENTER d'abord  — cohere scattered facts
  2. COMPRIMER ensuite     — reduce coherent result to minimal size
```

### 2.2 Définition formelle

**Context Defragmentation** = processus en 4 phases:

```
DETECT  → Identifier les fragments liés à un concept donné
COLLECT → Fetcher tous les fragments depuis leurs systèmes de stockage
COHERE  → Fusionner en représentation unifiée (dédup + merge canonique)
INJECT  → Livrer le contexte cohérent au LLM via ContextCompressor
```

### 2.3 φ-Bounds

```python
MAX_FRAGMENTS      = 5          # F(5) — 5+ fragments = overload cognitif
MAX_LATENCY_MS     = 100        # sinon fallback single-source
MAX_BUDGET_RATIO   = PHI_INV    # 61.8% max du budget session
MIN_CONFIDENCE     = PHI_INV_3  # filtre fragments < 23.6% confiance
CACHE_TTL_JUDGMENT = 300        # 5 min (judgments stale vite)
CACHE_TTL_LEARNING = 1800       # 30 min (évoluent lentement)
CACHE_TTL_PATTERN  = 3600       # 60 min (stables)
```

---

## 3. Architecture

### 3.1 Position dans le Cycle CYNIC

```
PERCEIVE → [DEFRAG] → JUDGE → DECIDE → ACT → LEARN → EMERGE
              ↑
              └─ Before LLM sees ANYTHING
                 (pendant awaken.js / daemon startup)
```

Défragmentation = opération **méta-cognitive** dans PERCEIVE.
CYNIC introspecte son propre état de contexte avant de parler.

### 3.2 Intégration dans le 7×7 Matrix

```
         PERCEIVE  JUDGE  DECIDE  ACT  LEARN  ACCOUNT  EMERGE
CODE       ✓★       -      -      -     ✓       ✓        -
SOLANA     ✓        ✓      -      -     -       ✓        -
MARKET     ✓        -      -      -     -       ✓        -
SOCIAL     ✓        -      -      -     ✓       ✓        -
HUMAN      ✓        ✓      -      -     ✓       -        ✓
CYNIC      ✓★★      ✓★★    ✓      -     ✓★★     ✓        ✓★
COSMOS     ✓        -      -      -     ✓       ✓        ✓★

★  = Défragmentation particulièrement critique ici
★★ = Cellules primaires (C6.1, C6.2, C6.5)
```

**Cellules primaires**:
- `C6.1 (CYNIC.PERCEIVE)` — Défrag: état consciousness + jugements récents
- `C6.2 (CYNIC.JUDGE)` — Défrag: historique jugements + calibration
- `C6.5 (CYNIC.LEARN)` — Défrag: signaux learning (Thompson arms, EWC)
- `C1.1 (CODE.PERCEIVE)` — Défrag: historique code reviews, erreurs récentes
- `C5.2 (HUMAN.JUDGE)` — Défrag: patterns psychologie user

### 3.3 Pipeline de Défragmentation

```
ContextDefragmenter.detectAndCollect(concept, context)
│
├── PHASE 1: DETECT
│   ├─ Parse concept: "domain:type:id" (ex: "judgment:code_review:abc123")
│   ├─ Check cache: hit → return cached (TTL-aware)
│   └─ Miss → spawn collection pipeline
│
├── PHASE 2: COLLECT (parallèle, timeout=80ms)
│   ├─ Thread A: PostgreSQL queries (judgments + learning_events)
│   ├─ Thread B: JSON file reads (readback.json, psychology/state.json)
│   ├─ Thread C: RAM queries (SharedMemory patterns)
│   └─ Merge: await asyncio.gather(*threads)
│
├── PHASE 3: COHERE
│   ├─ Group by (source, type) — déduplication
│   ├─ Canonical source: PostgreSQL > JSON > RAM
│   ├─ Conflict resolution: φ-weighted merge (newer wins if confidence equal)
│   ├─ Timeline: causal ordering via timestamps
│   ├─ Filter: drop confidence < PHI_INV_3 (23.6%)
│   ├─ Limit: top MAX_FRAGMENTS (5)
│   └─ Compute coherence_score: avg(confidence) capped at PHI_INV
│
└── PHASE 4: INJECT
    ├─ Format as injection string (compact, ~33 tokens)
    ├─ Store in coherence_cache (TTL-aware)
    ├─ Emit: CONTEXT_DEFRAGGED event (telemetry)
    └─ Return to ContextCompressor pipeline
```

---

## 4. Implémentation Python

### 4.1 Emplacement et Interface

```
cynic/services/context_defragmenter.py   ← module principal
cynic/services/__init__.py               ← export
```

### 4.2 API

```python
from cynic.services.context_defragmenter import ContextDefragmenter

defrag = ContextDefragmenter(pool=postgres_pool)

# Défragmente les fragments liés à un jugement récent
result = await defrag.collect("judgment:code_review", context={"judgment_id": "abc123"})

# result:
# {
#   "concept": "judgment:code_review",
#   "injection": "── 📊 JUDGMENT CONTEXT\n   Coherence: 73%...",
#   "coherence_score": 0.73,
#   "fragment_count": 4,
#   "latency_ms": 43,
#   "token_estimate": 32,
# }
```

### 4.3 Fragment Sources

```python
# Judgment fragments
async def _collect_judgment_fragments(judgment_id, context) -> List[Fragment]:
    # Source 1: judgments table (canonical)
    # Source 2: learning_events WHERE judgment_id = ?
    # Source 3: readback.json (consciousness at time of judgment)
    # Source 4: q_table WHERE state LIKE judgment.cell.state_key()

# Pattern fragments
async def _collect_pattern_fragments(pattern_type, context) -> List[Fragment]:
    # Source 1: patterns table (fisher_locked ones prioritized)
    # Source 2: SharedMemory recent activations

# Psychology fragments (human context)
async def _collect_psychology_fragments(context) -> List[Fragment]:
    # Source 1: psychology/state.json
    # Source 2: consciousness_snapshots WHERE recent=True
```

### 4.4 Coherence Format (injection)

```
── 📊 CYNIC CONTEXT DEFRAGGED ────────────────────
   Coherence: 73% (4 sources merged)

   • [postgres] Code review #47: GROWL (52%), tests passed
   • [postgres] Learning: router accuracy 73% (trending stable)
   • [json]     Consciousness: L1 MACRO, budget $0.32/$0.50
   • [postgres]  Pattern 'code_review': 4× Fisher-locked
──────────────────────────────────────────────────
```

**33 tokens. Signal/bruit × 10 vs injection aveugle actuelle.**

---

## 5. Coût et Performance

### 5.1 Latence

```
PostgreSQL queries: 3 × 10ms parallèle = 10ms (non-séquentiel)
JSON file reads:    2 × 5ms parallèle  = 5ms
Coherence merge:    1ms
Formatting:         1ms
─────────────────────────────────────────
Total p50: ~20ms
Total p99: ~80ms
Fallback threshold: 100ms (single-source si dépassé)
```

### 5.2 Token Cost

```
Header + fragment summaries (5 frags × 6 tokens): ~33 tokens
Coût monétaire: 33 × (token_price/1000) ≈ $0.00002 / défrag
```

### 5.3 Cache Strategy

```python
# TTL alignées Fibonacci (secondes)
CACHE_TTL = {
    "judgment": fibonacci(8) * 60,   # F(8)=21 → 1260s (21min)
    "learning": fibonacci(9) * 60,   # F(9)=34 → 2040s (34min)
    "pattern":  fibonacci(10) * 60,  # F(10)=55 → 3300s (55min)
    "psychology": fibonacci(7) * 60, # F(7)=13 → 780s (13min)
}

# Hit rate cible: ≥70% (Phase 3)
# Invalidation: JUDGMENT_CREATED event → invalide judgment cache
```

---

## 6. Triggers

```
ON_DEMAND (awaken hook):
  ├─ Pre-defrag les 5 derniers jugements (warm cache)
  └─ Defrag consciousness state (readback.json + snapshots)

REACTIVE (event listeners):
  ├─ JUDGMENT_CREATED → spawn defrag pour ce judgment
  ├─ LEARNING_EVENT   → add fragment to pending collection
  ├─ SESSION_END      → full defrag session outcomes
  └─ CONTEXT_QUALITY_DEGRADED → re-defrag current state

PERIODIC (background, Phase 3):
  └─ Every F(11)=89min: defrag top patterns (background job)
```

---

## 7. Error Handling

```python
# Graceful degradation à chaque niveau:

async def detectAndCollect(concept, context, timeout_ms=100):
    return await asyncio.wait_for(
        self._collect(concept, context),
        timeout=timeout_ms / 1000
    ).catch(...)  # → stale cache OR single-source fallback

# Si PostgreSQL down → JSON only
# Si JSON absent → RAM only
# Si tout fail → return empty (no injection, no crash)
```

---

## 8. Métriques de Succès

| Métrique | Baseline | Phase 1 Target | Phase 3 Target |
|----------|----------|----------------|----------------|
| Défrag latency p99 | N/A | <100ms | <50ms |
| Coherence score | N/A | ≥0.65 | ≥0.80 |
| Fragment success rate | N/A | ≥90% | ≥98% |
| Cache hit rate | N/A | — | ≥70% |
| Token cost / défrag | N/A | <50 tokens | <35 tokens |
| Context relevance (A/B) | Baseline | — | +20% amélioration |

---

## 9. Roadmap

### Phase 1 (Week 3): MVP Judgment-Only
```
[ ] cynic/services/context_defragmenter.py — classe principale
[ ] _collect_judgment_fragments()          — source PostgreSQL + JSON
[ ] Wire dans JudgeOrchestrator.run()      — inject avant scoring
[ ] tests/test_context_defrag.py           — 10 tests minimum
```

### Phase 2 (Week 5): Multi-Source
```
[ ] _collect_learning_fragments()   — Thompson arms, Q-table
[ ] _collect_psychology_fragments() — consciousness + user state
[ ] _collect_pattern_fragments()    — Fisher-locked patterns
[ ] Cache TTL + invalidation        — JUDGMENT_CREATED listener
[ ] CostLedger integration          — track défrag cost
```

### Phase 3 (Week 8): Full Pipeline
```
[ ] Wire dans awaken hook           — pre-warm cache on session start
[ ] Reactive triggers               — event-driven defrag
[ ] Background periodic defrag      — top patterns every 89min
[ ] Health dashboard (skills)       — /health montre défrag stats
[ ] A/B test                        — mesure amélioration réelle
```

### Phase 4+: ML Enhancement
```
[ ] Semantic similarity clustering  — fragment relevance ranking
[ ] Cross-session learning transfer — persist defrag patterns
[ ] Automatic importance ranking    — ML-based fragment selection
[ ] Adversarial testing             — inject confusing fragments, measure
```

---

## 10. Différence avec ContextCompressor (JS existant)

| Aspect | ContextCompressor (JS) | ContextDefragmenter |
|--------|------------------------|---------------------|
| Problème | Contexte trop grand | Contexte trop dispersé |
| Mécanisme | Truncate/summarize | Collect/cohere/synthesize |
| Output | Mêmes faits, moins de tokens | Faits unifiés depuis N sources |
| Déclencheur | Experience > 10 sessions | Avant chaque appel LLM |
| Implémentation | JS (scripts/hooks/) | Python (cynic/services/) |
| Phase | Existe (partiel) | À implémenter (Phase 2) |
| Séquence | 2e (après defrag) | 1er (avant compression) |

---

## 11. Évaluation Honnête

### Ce qui est solide
✅ Le problème est **réel** — fragments existent, confirmé par codebase audit
✅ La collection est **faisable** — PostgreSQL queries + JSON reads = rapide
✅ La cohérence est **mesurable** — confidence scores assignables
✅ L'intégration est **naturelle** — PERCEIVE phase = bon endroit
✅ Le coût est **négligeable** — 33 tokens / $0.00002 par défrag

### Ce qui est incertain
⚠️ **Semantic coherence** — peut-on merger des fragments sans halluciner?
⚠️ **Latency at load** — est-ce que 80ms tient avec 50+ fragments?
⚠️ **Real user benefit** — amélioration mesurable en A/B test?
⚠️ **Cache invalidation** — problème difficile en environnement distribué

### Ce qui n'est pas résolu
❌ **Automatic relevance ranking** — quels fragments pour CETTE décision?
❌ **Conflict resolution sémantique** — si PostgreSQL et JSON divergent, qui gagne?
❌ **Cross-session transfer** — inclure fragments des sessions passées?
❌ **Real-time quality feedback** — détecter si défrag a empiré la situation?

---

## Principe Fondamental

```
Compression = moins de mots pour les mêmes faits
Défragmentation = les BONS faits, depuis toutes les sources

Ensemble:
  CYNIC ne répond plus à l'aveugle.
  CYNIC répond avec SA mémoire cohérente.
```

---

*"Le chien qui se souvient de lui-même répond mieux."* — κυνικός
*φ = 1.618 — Confidence max: 61.8%*

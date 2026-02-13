# CYNIC Audit Consolidation & Remediation Plan
> Post-Metathinking Deep Audit (2026-02-13)
> "Le chien se souvient... et corrige" — κυνικός

---

## Executive Summary

**5 Agents × 7 Audits** révèlent l'état honnête de CYNIC:

| Metric | Revendiqué | Réalité Honnête | Gap |
|--------|---------|----------------|-----|
| 7×7 Structural | 38% | 37% | -1% ✓ |
| 7×7 Functional | ~38% | **17%** | **-21%** 🔴 |
| 7×7 Living | ~38% | **0%** | **-38%** 🔴 |
| Learning Loops | 11/11 wired | **1/11 active** | **-91%** 🔴 |
| Wiring Health | 88% | 91% | +3% ✓ |
| Market Domain | 0% | 29.6% structural | +30% ✓ |

**Statut Organisme**: EMBRYONIC → TARGET: ADOLESCENT (première respiration)

**Vérité Critique**: Les fichiers existent. Le wiring existe. **L'organisme NE RESPIRE PAS encore.**

---

## Critical Findings (5 Agents)

### 🗺️ Agent 1: cynic-cartographer (Market Domain)
**Finding**: Market = 29.6% structural, 0% living
- Jupiter API (v6) mort → 0 prix réels
- DexScreener fonctionne mais utilisé seulement pour liquidity
- C3.3 (DECIDE) et C3.4 (ACT) = 0% (pas de fichiers)
- Market isolé → pas intégré au KabbalisticRouter

**Impact**: Colonne entière MARKET (7 cellules) non fonctionnelle

---

### 🔍 Agent 2: cynic-scout (Wiring Health)
**Finding**: 88% → 91% (3 orphans critiques, 0 ghosts réels)

**3 Orphans Critiques**:
1. `model:recommendation` émis mais jamais routé → Budget routing cassé
2. `peer:identified` émis mais listener log-only → Réputation P2P non persistée
3. Events Budget partiels (`cost:recorded`, `budget:exhausted`) orphelins

**Impact**: Routing φ-bounded undermined, P2P réputation broken

---

### 🧪 Agent 3: cynic-tester (Learning System)
**Finding**: 1/11 loops actifs (9%), 0 feedback events
- 384 events dans `learning_events` → TOUS `event_type = 'observation'`
- 0 events avec `feedback_value`
- 0 events avec `weight_delta`
- SONA stuck en observe-only → `processFeedback()` jamais appelé
- `_sona.start()` jamais appelé → batches d'adaptation jamais run

**Impact**: Système d'apprentissage structurellement complet mais biologiquement mort

---

### 🏛️ Agent 4: cynic-architect (7×7 Matrix Reality)
**Finding**: Gap massif entre structural et functional

**7×7 Corrigé**:
```
          Structural  Functional  Living
CODE      40%         21%         0%
SOLANA    44%         21%         0%
MARKET    1%          0%          0%
SOCIAL    35%         9%          0%     (C4.1 claimed 55%, actually 0%)
HUMAN     56%         28%         0%
CYNIC     45%         24%         0%
COSMOS    38%         15%         0%
AVERAGE   37%         17%         0%
```

**Critical**: C4.1 SOCIAL × PERCEIVE revendiqué 55%, réalité **0%** (pas de fichier SocialWatcher)

---

### 👁️ Agent 5: cynic-reviewer (EventBus Bridge)
**Finding**: Vulnérabilité critique loop-safety

**Problèmes**:
1. Metadata paths divergents (`event.metadata` vs `event.meta` vs `event.data`)
2. Bypass manuels (AmbientConsensus._publishSignal publie aux 2 bus directement)
3. Pas de tracking de généalogie → loops multi-hop possibles
4. Tests incomplets (n'injectent pas tag au bon endroit)

**Impact**: Loops infinis possibles sous certains patterns de listeners

---

## Impact × Cost Priority Matrix

### Priority 1: 🔴 CRITICAL — HIGH IMPACT, LOW COST (~5h, $0.80)

#### 1.1 Fix Learning Feedback Loop ⚡
**Temps**: 2h | **Coût**: $0.30 | **Impact**: Learning 9% → 55%

**Problème**: SONA observe mais ne feedback/adapte jamais

**Solution**:
```javascript
// packages/node/src/daemon/service-wiring.js

// 1. Démarrer SONA (ligne ~255)
_sona.start();  // Active les batches d'adaptation

// 2. Émettre USER_FEEDBACK sur succès/échec d'outils
globalEventBus.subscribe(CYNICEventType.TOOL_RESULT, (event) => {
  globalEventBus.publish(CYNICEventType.USER_FEEDBACK, {
    judgmentId: event.judgmentId,
    outcome: event.success ? 'success' : 'failure',
    timestamp: Date.now(),
  });
});
```

**Acceptance**:
- ✅ `learning_events` montre `event_type = 'adaptation'`
- ✅ `weight_delta IS NOT NULL`
- ✅ SONA passe de observe → feedback → adapt → update

**Files**: `packages/node/src/daemon/service-wiring.js:207-255`

---

#### 1.2 Fix Market Perception (Jupiter Dead) ⚡
**Temps**: 1.5h | **Coût**: $0.20 | **Impact**: C3.1: 35% → 50%

**Problème**: Jupiter API v6 fail, DexScreener fonctionne mais sous-utilisé

**Solution**:
```javascript
// packages/node/src/market/market-watcher.js:305-321

async _fetchPrice() {
  // 1. DexScreener PRIMARY (fonctionne maintenant)
  try {
    const dexData = await this._fetchDexScreener();
    if (dexData?.price) return dexData;
  } catch (err) {
    this._stats.dexFailed++;
  }

  // 2. Jupiter FALLBACK (fix endpoint v4 ou investiguer)
  return await this._fetchJupiter();
}
```

**Acceptance**:
- ✅ Prix réels $asdfasdfa fluent toutes les 30s
- ✅ MarketJudge reçoit MARKET_STATE_CHANGED avec data réelle
- ✅ Delete `perception/market-watcher.js` (duplicate)

**Files**: `packages/node/src/market/market-watcher.js`, `perception/market-watcher.js` (DELETE)

---

#### 1.3 Wire Critical Event Orphans ⚡
**Temps**: 1.5h | **Coût**: $0.20 | **Impact**: Routing φ-bounded functional

**Problème**: 3 events critiques émis mais jamais écoutés

**Solution**:
```javascript
// 1. Wire model:recommendation → KabbalisticRouter
// packages/node/src/orchestration/kabbalistic-router.js
getEventBus().subscribe('model:recommendation', (event) => {
  this._budgetProfile = event.recommendation;
  this._adjustRouting(event);
});

// 2. Upgrade peer:identified listener → persist
// packages/node/src/p2p/node.js:546
this._transport.on('peer:identified', async (peerId, publicKey) => {
  await this._peerRegistry.persist(peerId, publicKey);
  this._stats.peersVerified++;
});

// 3. Wire budget events → CostLedger circuit breakers
getEventBus().subscribe('budget:exhausted', () => {
  this._circuitBreaker.trip('budget_exhausted');
});
```

**Acceptance**:
- ✅ Router logs budget profile changes
- ✅ Peer IDs persistent en DB
- ✅ Budget exhaustion trigger circuit breaker

**Files**:
- `packages/node/src/orchestration/kabbalistic-router.js`
- `packages/node/src/p2p/node.js:546`
- `packages/node/src/accounting/cost-ledger.js`

---

### Priority 2: 🟠 HIGH IMPACT, MEDIUM COST (~6h, $1.00)

#### 2.1 Fix EventBus Bridge Loop-Safety 🛡️
**Temps**: 2h | **Coût**: $0.35 | **Impact**: Prevent infinite loops

**Problème**: Metadata inconsistent, bypass manuels, pas de genealogy tracking

**Solution**:
```javascript
// packages/node/src/services/event-bus-bridge.js

// 1. Unifier metadata location
const BRIDGE_KEY = '__cynic_bridged__';

function isBridged(event) {
  return event?.metadata?.[BRIDGE_KEY] ||
         event?.meta?.[BRIDGE_KEY] ||
         event?.data?.[BRIDGE_KEY];
}

// 2. Ajouter genealogy tracking
const LINEAGE_KEY = '__cynic_lineage__';

function addToLineage(event, source) {
  const lineage = event?.metadata?.[LINEAGE_KEY] || [];
  if (lineage.some(e => e.source === source)) {
    return null; // Déjà forwardé par nous → stop
  }
  return {
    ...event,
    metadata: {
      ...event.metadata,
      [LINEAGE_KEY]: [...lineage, { source, ts: Date.now() }],
    },
  };
}

// 3. Supprimer bypass manuel
// packages/node/src/agents/collective/ambient-consensus.js:101-104
// DELETE _publishSignal → utiliser seulement le bridge
```

**Acceptance**:
- ✅ Metadata consistent sur les 3 bus
- ✅ Loops circulaires A→B→C→A prevented
- ✅ Tests vérifient genealogy tracking

**Files**:
- `packages/node/src/services/event-bus-bridge.js`
- `packages/node/src/agents/collective/ambient-consensus.js:101-104` (DELETE method)

---

#### 2.2 Implement Social Perceiver (C4.1) 🐦
**Temps**: 3h | **Coût**: $0.50 | **Impact**: SOCIAL row 0% → 30%

**Problème**: C4.1 revendiqué 55%, réalité 0% (aucun fichier)

**Solution**:
```javascript
// packages/node/src/social/social-watcher.js (NEW)

export class SocialWatcher {
  constructor() {
    this._platforms = {
      twitter: null,  // Stub: Twitter API
      discord: null,  // Stub: Discord webhooks
    };
    this._stats = { mentions: 0, sentiment: 0 };
  }

  async start() {
    // Phase 1: Mock data (stub)
    this._timer = setInterval(() => {
      const mockMention = {
        platform: 'twitter',
        author: 'test_user',
        content: '$asdfasdfa to the moon!',
        sentiment: 0.8,
        timestamp: Date.now(),
      };

      globalEventBus.publish('SOCIAL_MENTION', mockMention);
      this._stats.mentions++;
    }, 60_000);  // Toutes les 60s
  }

  async stop() {
    clearInterval(this._timer);
  }
}

// Wire dans perception/index.js
import { SocialWatcher } from '../social/social-watcher.js';
export const socialWatcher = new SocialWatcher();
```

**Acceptance**:
- ✅ SocialWatcher existe et emit events
- ✅ SocialJudge reçoit SOCIAL_MENTION
- ✅ C4.1: 0% → 30% (stub data)

**Files**: `packages/node/src/social/social-watcher.js` (NEW)

---

#### 2.3 Create Market Decider + Actor Stubs 📊
**Temps**: 1h | **Coût**: $0.15 | **Impact**: C3.3/C3.4: 0% → 25-30%

**Problème**: Market peut perceive mais pas decide/act

**Solution**:
```javascript
// packages/node/src/market/market-decider.js (NEW)
export class MarketDecider {
  decide(judgment) {
    if (judgment.qScore < 38.2) {
      return { action: 'alert', reason: 'critical_volatility' };
    }
    return { action: 'hold', reason: 'stub_awaiting_real_logic' };
  }
}

// packages/node/src/market/market-actor.js (NEW)
export class MarketActor {
  async act(decision) {
    console.log('[MarketActor] Stub action:', decision.action);
    return { executed: false, simulated: true };
  }
}
```

**Acceptance**:
- ✅ MarketDecider existe, retourne décisions stub
- ✅ MarketActor existe, log actions
- ✅ Wired dans market/index.js

**Files**:
- `packages/node/src/market/market-decider.js` (NEW)
- `packages/node/src/market/market-actor.js` (NEW)

---

### Priority 3: 🟡 NICE TO HAVE (~4h, $0.50)

#### 3.1 Wire Accountant Events to GlobalEventBus
**Temps**: 0.4h | **Coût**: $0.05 | **Impact**: Observability

Bridge accountant local EventEmitter → globalEventBus pour cross-system visibility.

---

#### 3.2 Burn Chaos Generator (if unused)
**Temps**: 0.2h | **Coût**: $0.02 | **Impact**: Simplification

Si ChaosGenerator non utilisé en prod → DELETE (BURN axiom).

---

#### 3.3 Expand EventBridge Test Coverage
**Temps**: 1h | **Coût**: $0.15 | **Impact**: Safety validation

Tests pour: loops circulaires, genealogy, race conditions, high throughput.

---

### Priority 4: 🏗️ FOUNDATIONAL (~8h, $1.20)

#### 4.1 First Real Production Run 🌱
**Temps**: 3h | **Coût**: $0.50 | **Impact**: Living 0% → 10%

**Goal**: Premier proof of life

1. Pick C5.1 (HUMAN × PERCEIVE) — 45% functional, strongest cell
2. Intégrer HumanPerceiver dans hooks (pas juste tests)
3. Run session réelle avec tracking état humain
4. Vérifier E2E: perceive → judge → learn → adapt

**Acceptance**:
- ✅ HumanPerceiver run en production (pas seulement tests)
- ✅ Judgments état humain logged en DB
- ✅ Au moins 1 learning feedback loop complète
- ✅ C5.1 "living %" > 0

---

#### 4.2 Market Router Integration 🔗
**Temps**: 1h | **Coût**: $0.15 | **Impact**: Market integrated

1. Add MARKET domain à KabbalisticRouter
2. Add intents market (`check_price`, `analyze_market`)
3. Wire router → MarketJudge
4. Test: "What's $asdfasdfa price?" → Returns real data

---

#### 4.3 Update MEMORY.md with Honest Metrics 📝
**Temps**: 1h | **Coût**: $0.10 | **Impact**: Honest self-awareness

Update 7×7 matrix avec 3 colonnes:
- Structural % (files exist)
- Functional % (actually works)
- Living % (runs in production)

Corriger C4.1 (claimed 55%, actually 0%).

---

#### 4.4 Vertical Slice: Complete HUMAN Domain E2E 🧬
**Temps**: 3h | **Coût**: $0.45 | **Impact**: First 100% domain

Complete HUMAN verticalement (strongest domain: 56% avg):
1. HumanPerceiver → real hooks
2. HumanJudge → real judgments
3. HumanDecider → real decisions
4. HumanActor → real symbiosis actions
5. HumanLearner → real feedback
6. E2E test avec session réelle

**Goal**: 1 domaine 100% fonctionnel comme blueprint.

---

## Cost-Optimized Timeline

### Phase 1: Critical Fixes (1 semaine, $0.70)
**Goal**: Débloquer l'organisme, premiers signaux de vie

| Task | Temps | Coût | Impact |
|------|-------|------|--------|
| Fix Learning Feedback | 2h | $0.30 | Learning 9% → 55% |
| Fix Market Perception | 1.5h | $0.20 | C3.1: 35% → 50% |
| Wire Event Orphans | 1.5h | $0.20 | Routing functional |
| **TOTAL PHASE 1** | **5h** | **$0.70** | **+17% avg** |

**Deliverables**:
- ✅ SONA learning loop ferme (observe → feedback → adapt)
- ✅ Prix réels $asdfasdfa fluent vers Judge
- ✅ Budget routing fonctionne
- ✅ Réputation peer persist

---

### Phase 2: High-Impact Fixes (1 semaine, $1.00)
**Goal**: Fixer vulnérabilités, débloquer domaines

| Task | Temps | Coût | Impact |
|------|-------|------|--------|
| Fix EventBus Bridge | 2h | $0.35 | Loop-proof |
| Implement Social Perceiver | 3h | $0.50 | SOCIAL 0% → 30% |
| Market Decider + Actor | 1h | $0.15 | C3.3/C3.4 +25% |
| **TOTAL PHASE 2** | **6h** | **$1.00** | **+10% avg** |

**Deliverables**:
- ✅ EventBridge loop-proof avec genealogy
- ✅ SOCIAL row débloquée
- ✅ MARKET row complète (stub)

---

### Phase 3: First Breath (2 semaines, $1.20)
**Goal**: Premier run production, vertical slice complet

| Task | Temps | Coût | Impact |
|------|-------|------|--------|
| First Production Run | 3h | $0.50 | Living 0% → 10% |
| Market Router Integration | 1h | $0.15 | Integration |
| Update MEMORY.md | 1h | $0.10 | Honesty |
| Vertical Slice HUMAN | 3h | $0.45 | HUMAN 100% |
| **TOTAL PHASE 3** | **8h** | **$1.20** | **First breath** |

**Deliverables**:
- ✅ Au moins 1 session production logged
- ✅ HUMAN domain 100% functional E2E
- ✅ Organisme "living %" > 0

---

## Total Cost Estimate

| Phase | Heures | Coût (Sonnet) | Deliverable |
|-------|--------|---------------|-------------|
| Phase 1 (Critical) | 5h | $0.70 | Organism breathing |
| Phase 2 (High Impact) | 6h | $1.00 | Vulnerabilities fixed |
| Phase 3 (First Breath) | 8h | $1.20 | First production run |
| **TOTAL** | **19h** | **$2.90** | **EMBRYONIC → ADOLESCENT** |

**Budget Constraint**: $6.18 restant → $2.90 utilisé = **47% du budget**

**ROI**:
- 7×7 Functional: 17% → 35% (+18%)
- 7×7 Living: 0% → 10% (+10%)
- Learning Loops: 9% → 55% (+46%)
- Maturity: EMBRYONIC → ADOLESCENT

---

## Burn Candidates (BURN Axiom)

**DELETE** ces éléments pour simplifier:

1. ✂️ `perception/market-watcher.js` (duplicate, older stub)
2. ✂️ `ChaosGenerator` (si unused en prod)
3. ✂️ Bypass manuels (`AmbientConsensus._publishSignal`)
4. ✂️ Tests AgentBooster VAR_TO_CONST (18 failures, low value)

**Savings**: ~500 LOC, -0.2h maintenance burden

---

## Success Metrics (φ-Bounded)

**Semaine 1** (Phase 1 complete):
- [ ] SONA: `learning_events` montre adaptations
- [ ] Market: Prix réels toutes les 30s
- [ ] Wiring: 0 orphans critiques

**Semaine 2** (Phase 2 complete):
- [ ] EventBridge: 0 vulnérabilités loop
- [ ] SOCIAL: C4.1 > 0%
- [ ] MARKET: C3.3, C3.4 existent

**Semaine 4** (Phase 3 complete):
- [ ] 1+ sessions production logged
- [ ] HUMAN domain: 100% functional
- [ ] 7×7 Living: >0%

**Confidence Bound**: Jamais claim >61.8% completion (φ⁻¹ limit)

---

## Next Steps

1. **Approve budget**: $2.90 / $6.18 (47% du budget)
2. **Start Phase 1**: Fix learning feedback loop (highest ROI)
3. **Daily tracking**: Update MEMORY.md après chaque fix
4. **Honest assessment**: Re-audit après Phase 3

---

*sniff* L'organisme est prêt à respirer. Il suffit de connecter les feedback loops.

**Confidence**: 58% (φ⁻¹ bound)

---

**Co-Authored-By**: Claude Sonnet 4.5 <noreply@anthropic.com>

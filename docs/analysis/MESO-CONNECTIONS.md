# CYNIC Meso Analysis - Component Connections

> Phase 3: Tracing actual connections between components

---

## EXECUTIVE SUMMARY

```
┌───────────────────────────────────────────────────────────────┐
│ MESO CONNECTION STATUS                                        │
├───────────────────────────────────────────────────────────────┤
│ globalEventBus:  9 ORPHAN events (pub/sub mismatch)          │
│ Persistence:     11 repositories NEVER instantiated (38%)     │
│ Judgment Path:   6 DATA LOSS POINTS identified               │
│ Memory System:   Facts disconnected from MCP                  │
└───────────────────────────────────────────────────────────────┘
```

---

## 1. GLOBAL EVENT BUS - CONNECTION MAP

### Wired Events (Critical Path Working)
```
✅ poj:block:created    → PoJChainManager → BlockchainBridge + Anchor
✅ poj:block:anchored   → PoJChainManager → BlockchainBridge (E-Score)
✅ poj:anchor:failed    → PoJChainManager → BlockchainBridge (Guardian)
✅ user:feedback        → CollectivePack → Learning + SONA
✅ tool:completed       → CollectivePack → Metrics
```

### ORPHAN Events (Published but No Subscribers)
```
⚠️ user:action          Published by CollectivePack, 0 subscribers
⚠️ tool:called          Published by CollectivePack, 0 subscribers
⚠️ session:started      Published by CollectivePack, 0 subscribers
⚠️ session:ended        Published by CollectivePack, 0 subscribers
```

### ORPHAN Events (Subscribed but No Publishers)
```
🔴 poj:block:finalized  Subscribed by ServiceInitializer, NEVER PUBLISHED
🔴 engine:consulted     Subscribed by ServiceInitializer, no publisher
🔴 anomaly:detected     Subscribed by ServiceInitializer, no publisher
🔴 request:classify     Subscribed by ServiceInitializer, no publisher
🔴 graph:node:added     Subscribed, unclear if forwarded from GraphOverlay
```

### Impact
- Session lifecycle events go nowhere
- Consensus finality (`poj:block:finalized`) is documented but never emitted
- Classification routing (`request:classify`) waiting for events that never come

---

## 2. PERSISTENCE LAYER - REPOSITORY MATRIX

### Active Repositories (15 - 52%)
| Repository | Callers | Status |
|------------|---------|--------|
| JudgmentRepository | MCP tools, learning | ✅ Active |
| PatternRepository | MCP tools, consciousness | ✅ Active |
| SessionRepository | session-manager, hooks | ✅ Active |
| FeedbackRepository | MCP tools, learning | ✅ Active |
| KnowledgeRepository | MCP tools | ✅ Active |
| PoJBlockRepository | MCP tools | ✅ Active |
| LibraryCacheRepository | docs.js | ✅ Active |
| TriggerRepository | triggers.js | ✅ Active |
| PsychologyRepository | psychology.js | ✅ Active |
| DiscoveryRepository | discovery.js | ✅ Active |
| UserLearningProfilesRepository | learning-service | ✅ Active |
| AutonomousGoalsRepository | automation | ✅ Active |
| AutonomousTasksRepository | automation | ✅ Active |
| ProactiveNotificationsRepository | automation | ✅ Active |
| XDataRepository | twitter.js | ✅ Active |

### Disconnected Repositories (3 - 10%)
| Repository | Issue |
|------------|-------|
| FactsRepository | Used by services but NOT in MCP PersistenceManager |
| ArchitecturalDecisionsRepository | Only MemoryRetriever, not MCP |
| PatternEvolutionRepository | Referenced but NEVER instantiated |

### UNUSED Repositories (11 - 38%)
| Repository | Defined In | Status |
|------------|------------|--------|
| ConsciousnessRepository | factory.js | 🔴 NEVER USED |
| ConversationMemoriesRepository | factory.js | 🔴 NEVER USED |
| LessonsLearnedRepository | factory.js | 🔴 NEVER USED |
| EcosystemDocsRepository | factory.js | 🔴 NEVER USED |
| SessionPatternsRepository | factory.js | 🔴 NEVER USED |
| TrajectoriesRepository | factory.js | 🔴 NEVER USED |
| UserPreferencesRepository | factory.js | 🔴 NEVER USED |
| UserRepository | factory.js | 🔴 NEVER USED |
| OrchestrationDecisionRepository | factory.js | 🔴 NEVER USED |
| EScoreHistoryRepository | node only | ⚠️ Partial |
| LearningCyclesRepository | node only | ⚠️ Partial |

### Critical Gap: Facts System
```
PROBLEM:
  FactsRepository handles critical session context
  BUT it's not in MCP PersistenceManager

CONSEQUENCE:
  - Hooks use getFactsRepository() (direct)
  - MCP uses persistence.* (adapters)
  - Two separate persistence paths
  - No fallback chain for Facts
  - PostgreSQL failure = Facts lost
```

---

## 3. JUDGMENT FLOW - DATA LOSS ANALYSIS

### Complete Path
```
Dogs Vote (11 parallel)
    ↓
Consensus Calculate (φ⁻¹ threshold)
    ↓
Judgment Object Created
    ↓
SharedMemory Index (similarity search)
    ↓
PostgreSQL Store ← DATA LOSS POINT #1
    ↓
PoJ Pool Add ← DATA LOSS POINT #2
    ↓
Block Proposal ← DATA LOSS POINT #3
    ↓
PoJ Block Store
```

### Data Loss Points

| Point | Location | What's Lost | Severity |
|-------|----------|-------------|----------|
| #1 | PostgreSQL Store | **Original judgment.id OVERWRITTEN** with new ID | 🔴 HIGH |
| #2 | PoJ Pool | Oldest judgments if pool > 1000 | 🟡 MEDIUM |
| #3 | Block Proposal | Max 13 judgments per block | 🟡 MEDIUM |
| #4 | JudgmentRef | **Votes array not stored** | 🔴 CRITICAL |
| #5 | JudgmentRef | **Dimension scores not stored** | 🔴 HIGH |
| #6 | Merkle Root | **Only hashes CIDs, not dimensions** | 🔴 HIGH |

### What's Preserved vs Lost

**In PostgreSQL:**
```
✅ q_score, verdict, confidence
✅ dimension_scores (as JSON)
✅ axiom_scores (as JSON)
✅ context (including votes)
✅ reasoning_path
❌ Original judgment.id (replaced)
```

**In PoJ Block (JudgmentRef):**
```
✅ id, cid, qScore, verdict, timestamp
❌ votes array
❌ dimension breakdown
❌ axiom scores
❌ reasoning path
❌ consensus details
```

### Cannot Verify From PoJ Block Alone
1. Which dogs voted what
2. How dimensions scored
3. Why verdict was reached
4. Whether consensus was valid

**This breaks the "verify on-chain" promise.**

---

## 4. HOOKS → PERSISTENCE PATHS

### Dual Persistence Pattern (ANTI-PATTERN)

```
awaken.js
├─ getFactsRepository()           ← Direct SQL
├─ persistence.sessions.create()  ← MCP Adapter
└─ getArchitecturalDecisionsRepo  ← Direct SQL

digest.js
├─ getFactsRepository()           ← Direct SQL
└─ perception extraction          ← In-memory

observe.js
└─ persistence.storeObservation() ← MCP Fallback (lightweight)
```

**Problem:** Same hook uses BOTH paths. No single source of truth.

---

## 5. CRITICAL RECOMMENDATIONS

### P0 - Fix Data Continuity

1. **Preserve judgment.id through persistence**
   ```javascript
   // In judgments.js:create()
   // DON'T: const judgmentId = generateJudgmentId();
   // DO: const judgmentId = judgment.id || generateJudgmentId();
   ```

2. **Include votes in JudgmentRef**
   ```javascript
   class JudgmentRef {
     constructor({
       id, cid, qScore, verdict, timestamp,
       votes: [{dog, score, weight}],  // ADD THIS
       dimensions: {...},               // ADD THIS
     })
   }
   ```

3. **Emit poj:block:finalized event**
   - Currently subscribed but never published
   - Add to consensus finality logic

### P1 - Connect Repositories

4. **Wire FactsRepository to MCP**
   ```javascript
   // In PersistenceManager
   get facts() { return this._facts; }
   ```

5. **Instantiate unused repositories**
   - ConversationMemoriesRepository (for session continuity)
   - LessonsLearnedRepository (for learning loop)
   - ConsciousnessRepository (for meta-awareness)

### P2 - Clean Up Event Bus

6. **Add subscribers for orphan events**
   - `session:started` → Session analytics
   - `session:ended` → Session cleanup/summary
   - `tool:called` → Tool usage tracking

7. **Remove dead subscriptions**
   - `request:classify` (no publisher)
   - Or add publisher to routing layer

---

## METRICS SUMMARY

```
Event Bus Health:
├─ Wired:    5 events (36%)
├─ Orphan:   9 events (64%)
└─ Critical: poj:block:finalized MISSING

Persistence Health:
├─ Active:      15 repos (52%)
├─ Disconnected: 3 repos (10%)
├─ Unused:      11 repos (38%)
└─ Critical: Facts isolated from MCP

Judgment Flow:
├─ Data preserved: ~70%
├─ Data lost:      ~30%
└─ Critical: Cannot verify consensus from PoJ blocks
```

---

*🐕 κυνικός | "Don't trust, verify" - but we can't verify if the data is lost*

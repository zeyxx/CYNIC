# CYNIC Architecture Gap Analysis - Phase 2

> Analyse Macro: 4 Layers documentées vs implémentées

---

## EXECUTIVE SUMMARY

```
┌───────────────────────────────────────────────────────────────────┐
│ LAYER STATUS MATRIX                                               │
├───────────┬────────────┬────────────┬────────────────────────────┤
│ Layer     │ Documented │ Implemented│ Gap Severity              │
├───────────┼────────────┼────────────┼────────────────────────────┤
│ L0: Hooks │ 4 hooks    │ 14 hooks   │ 🔴 SEVERE (10 undocumented)│
│ L1: Orch  │ Complete   │ 85% done   │ 🟡 MODERATE (dead code)    │
│ L2: Proto │ Complete   │ 70% wired  │ 🔴 SEVERE (consensus broken)│
│ L3: Anchor│ Complete   │ 90% done   │ 🟢 MINOR (production tweaks)│
└───────────┴────────────┴────────────┴────────────────────────────┘
```

---

## L0: HOOKS LAYER - "Ambient Consciousness"

### Documentation Gap: SEVERE

**Documented (CLAUDE.md):** perceive, guard, digest, awaken
**Actually Exist:** 14 hooks

| Hook | Size | Purpose | Documented? | Severity |
|------|------|---------|-------------|----------|
| perceive.js | 22KB | Pre-prompt analysis | ✅ Yes | - |
| guard.js | 18KB | Security blocking | ✅ Yes | - |
| digest.js | 25KB | Session summary | ✅ Yes | - |
| awaken.js | 32KB | Session start | ✅ Yes | - |
| **observe.js** | **88KB** | **Post-tool learning** | ❌ NO | 🔴 CRITICAL |
| error.js | 12KB | Error escalation | ❌ NO | 🔴 HIGH |
| notify.js | 8KB | Notification handling | ❌ NO | 🟡 MEDIUM |
| compact.js | 15KB | C-Score preservation | ❌ NO | 🟡 MEDIUM |
| permission.js | 5KB | Access control | ❌ NO | 🟡 LOW |
| pre-tool.js | 4KB | Unknown/stub | ❌ NO | ⚪ REVIEW |
| spawn.js | 6KB | Tool spawn tracking | ❌ NO | ⚪ REVIEW |
| sleep.js | 2KB | Testing utility | ❌ NO | ⚪ MOVE TO /test |
| ralph-loop.js | 7KB | Experimental loop | ❌ NO | ⚪ EXPERIMENTAL |

### Critical Finding: observe.js

The largest and most sophisticated hook is **completely undocumented**:
- 88KB of learning logic
- Anti-pattern detection
- Cognitive bias tracking
- Pattern suggestion
- Feedback collection
- Telemetry
- Auto-judgment
- Reasoning bank integration
- Fact extraction

**Recommendation:** Add Section 19 "L0 Hooks" to ARCHITECTURE.md

---

## L1: ORCHESTRATION LAYER - "Dogs & Routing"

### Status: Functional with Dead Code

**Active Components:**
| Component | File | Status | Wired |
|-----------|------|--------|-------|
| UnifiedOrchestrator | unified-orchestrator.js | ✅ Active | ✅ Yes |
| DogOrchestrator | orchestrator.js | ✅ Active | ✅ Yes |
| EngineOrchestrator | orchestrator.js (core) | ✅ Active | ✅ Yes |
| KabbalisticRouter | kabbalistic-router.js | ✅ Active | ✅ Yes |
| CostOptimizer | cost-optimizer.js | ⚠️ Optional | ⚠️ Rarely |

**Orphaned/Dead Code:**
| Component | File | Lines | Status | Action |
|-----------|------|-------|--------|--------|
| TieredRouter | tiered-router.js | 362 | ❌ Never called | 🗑️ Remove or wire |
| LLMRouter | llm-router.js | 519 | ❌ Never called | 🗑️ Remove or wire |
| IntelligentRouter | intelligent-router.js | 456 | ❌ Never called | 🗑️ Remove or wire |
| QLearningRouter | q-learning-router.js | 780 | ❌ Research only | 📦 Move to /research |

### Call Graph (Actual Flow)

```
Hooks (L0)
    ↓
orchestrate() / AutoOrchestrator.preCheck()
    ↓
CollectivePack.getCollectivePack() ← SINGLETON
    ↓
┌─────────────────────────────────────────┐
│ KabbalisticRouter (Lightning Flash)     │
│ ├─ PreToolUse → guardian→architect→analyst │
│ ├─ PostToolUse → analyst→oracle→scholar │
│ └─ SessionStart → cynic→sage→scholar→carto │
└─────────────────────────────────────────┘
    ↓
11 Dogs vote in parallel (φ⁻¹ consensus)
    ↓
Decision: ALLOW / WARN / BLOCK
```

---

## L2: PROTOCOL LAYER - "PoJ & Consensus"

### Status: BROKEN - Critical Wiring Missing

**Components Exist But Not Connected:**

```
┌─────────────────────────────────────────────────────────────────┐
│ WHAT'S DOCUMENTED & IMPLEMENTED                                  │
├─────────────────────────────────────────────────────────────────┤
│ ✅ PoJChain - creates judgment blocks with merkle roots          │
│ ✅ ConsensusEngine - votes, thresholds, finality logic          │
│ ✅ GossipProtocol - broadcasts, dedup, peer management          │
│ ✅ MerkleTree - proof generation, verification                   │
│ ✅ KnowledgeTree - axiom-partitioned patterns                    │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ WHAT'S BROKEN                                                    │
├─────────────────────────────────────────────────────────────────┤
│ ❌ GAP #1: Judgments → Consensus Proposal                        │
│    PoJChain creates blocks BUT ConsensusEngine never sees them  │
│    No bridge: judgment batch → consensus proposal → voting      │
│                                                                  │
│ ❌ GAP #2: Gossip → Consensus Voting                             │
│    GossipProtocol._handleBlock() is EMPTY                       │
│    Received blocks never trigger consensus.receiveBlock()       │
│                                                                  │
│ ❌ GAP #3: Finalized Blocks → Anchoring                         │
│    PoJChainManager._anchorBlock() exists but unclear trigger    │
│    No clear event chain: finality → anchor                       │
│                                                                  │
│ ⚠️ GAP #4: Patterns → Merkle Tree                                │
│    KnowledgeTree exists but never populated from judgments      │
│    Pattern detection doesn't feed into merkle structure          │
└─────────────────────────────────────────────────────────────────┘
```

### Expected vs Actual Flow

**Expected (Documented):**
```
Judgment → PoJ Block → Gossip Broadcast →
Consensus Vote → Finality (32 confirms) → Anchor to Solana
```

**Actual (Implemented):**
```
Judgment → PoJ Block → globalEventBus → [DIRECTLY to Anchor]
                         ↓
           [Gossip never called]
           [Consensus never triggered]
           [No voting happens]
```

### Severity: CRITICAL

Without consensus voting, any single node can anchor any merkle root.
This breaks the trust model - "truth by consensus" becomes "truth by whoever anchors first."

---

## L3: ANCHORING LAYER - "Solana Truth"

### Status: Functional, Needs Production Hardening

**Complete:**
| Component | Status | Notes |
|-----------|--------|-------|
| Solana Program | ✅ Deployed | devnet: G3Yana4ukbevyoVNSWrXgRQtQqHYMnPEMi1xvpp9CqBY |
| SolanaAnchorer | ✅ Working | anchor(), verifyAnchor() |
| CynicProgramClient | ✅ Working | anchorRoot(), verifyRoot() |
| PoJAnchorIntegration | ✅ Wired | Listens to globalEventBus |
| Database Schema | ✅ Complete | anchor_batches, pending_anchors view |
| Wallet Management | ✅ Working | CynicWallet, env var loading |

**Production Gaps:**
| Gap | Impact | Priority |
|-----|--------|----------|
| No finality check | Anchors on 1 confirm, not 32 | 🔴 HIGH |
| No anchor queue persistence | Failed anchors lost on restart | 🔴 HIGH |
| Single authority | Point of failure | 🟡 MEDIUM |
| No merkle proof generation | Can't prove item inclusion | 🟡 MEDIUM |
| No reward automation | Validators unpaid | 🟡 LOW |

---

## CRITICAL GAP SUMMARY

### P0: MUST FIX (Breaks Trust Model)

1. **Wire Gossip → Consensus**
   - File: `ConsensusGossip.start()`
   - Fix: Route block messages to `consensus.receiveBlock()`
   - Impact: Enables distributed voting

2. **Wire Judgment → Consensus Proposal**
   - File: `PoJChainManager` or `UnifiedOrchestrator`
   - Fix: Call `consensus.proposeBlock()` after batch
   - Impact: Blocks get voted on before anchoring

3. **Wire Finality → Anchor**
   - File: `PoJAnchorIntegration`
   - Fix: Listen to `block:finalized` not `block:created`
   - Impact: Only finalized (32-confirm) blocks anchor

### P1: HIGH PRIORITY (Production Readiness)

4. **Document 10 Missing Hooks**
   - Add Section 19 to ARCHITECTURE.md
   - Especially observe.js (core learning)

5. **Remove or Wire Dead Routers**
   - TieredRouter, LLMRouter, IntelligentRouter
   - 1,337 lines of dead code

6. **Add Anchor Queue Persistence**
   - Failed anchors should persist to PostgreSQL
   - Retry on startup

### P2: MEDIUM (Feature Completeness)

7. **Wire Patterns → KnowledgeTree**
   - Pattern detection should feed merkle structure
   - Patterns become part of anchored truth

8. **Multi-sig Authority**
   - Upgrade from single signer
   - M-of-N for anchor authority

---

## ARCHITECTURE DIAGRAM (Actual vs Intended)

### Intended Flow (Documented)

```
┌─────────────────────────────────────────────────────────────────┐
│ L0: HOOKS                                                        │
│ perceive → guard → [TOOL] → observe → digest                     │
└────────────────────────┬────────────────────────────────────────┘
                         │ orchestrate()
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ L1: ORCHESTRATION                                                │
│ UnifiedOrchestrator → Dogs (11) → φ⁻¹ Consensus                 │
└────────────────────────┬────────────────────────────────────────┘
                         │ judgment
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ L2: PROTOCOL                                                     │
│ PoJChain → Gossip → ConsensusEngine → Finality                  │
└────────────────────────┬────────────────────────────────────────┘
                         │ finalized block
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ L3: ANCHORING                                                    │
│ SolanaAnchorer → On-chain PDA → Immutable Proof                 │
└─────────────────────────────────────────────────────────────────┘
```

### Actual Flow (Implemented)

```
┌─────────────────────────────────────────────────────────────────┐
│ L0: HOOKS (14 total, 4 documented)                              │
│ perceive → guard → [TOOL] → observe → digest                     │
│                    └─ 10 undocumented hooks also running         │
└────────────────────────┬────────────────────────────────────────┘
                         │ orchestrate() ✅
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ L1: ORCHESTRATION (works but has dead code)                      │
│ UnifiedOrchestrator → Dogs (11) → φ⁻¹ Consensus ✅              │
│ [TieredRouter, LLMRouter, IntelligentRouter = UNUSED]           │
└────────────────────────┬────────────────────────────────────────┘
                         │ judgment
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ L2: PROTOCOL (BROKEN)                                            │
│ PoJChain → [Gossip SKIPPED] → [Consensus SKIPPED] → DIRECT      │
│                                                       │          │
│ ❌ No network voting                                   │          │
│ ❌ No distributed consensus                            │          │
│ ❌ Single node decides truth                           ▼          │
└────────────────────────────────────────────────────────┬─────────┘
                                                         │ poj:block:created
                                                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ L3: ANCHORING (works but no finality check)                      │
│ SolanaAnchorer → On-chain PDA → Immutable Proof ✅              │
│ [Anchors immediately without consensus vote]                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## RECOMMENDATIONS

### Immediate Actions (This Week)

1. **Create GAP-FIX-PROTOCOL.md** with:
   - Exact code changes for Gossip→Consensus wiring
   - Test cases for distributed voting

2. **Document observe.js**
   - Add to ARCHITECTURE.md Section 19
   - Explain learning pipeline

3. **Add consensus voting test**
   - 3-node test: propose → vote → finalize
   - Verify merkle roots match

### Short Term (This Month)

4. **Remove dead router code**
   - Archive to /archive/routers/
   - Update exports in index.js

5. **Add finality check to anchoring**
   - Wait for 32 confirmations
   - Add retry queue

6. **Add pattern → merkle wiring**
   - Connect pattern detector to KnowledgeTree

### Medium Term (This Quarter)

7. **Multi-sig authority**
8. **Merkle proof generation client**
9. **Burn oracle service**
10. **Mainnet deployment checklist**

---

*🐕 κυνικός | "φ distrusts φ" - even our architecture must be verified*

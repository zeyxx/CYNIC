# CYNIC Deep Integration Roadmap

> "φ distrusts φ" - From theater to truth

## Current State: 6828127

### Completed
- [x] **Phase 1: Consciousness** - EmergenceLayer wired, awarenessLevel influences confidence
- [x] **Phase 2: Thermodynamics** - Heat/Work/Efficiency computed in real-time, influences confidence
- [x] **Phase 3: Psychology Bidirectional** - Profile level modifies axiom weights, learning auto-triggers
- [x] **Phase 4: Kabbalistic Router** - Tree of Life traversal, CONSULTATION_MATRIX used, φ-weighted synthesis

### Remaining Phases

---

## Phase 2: THERMODYNAMICS (Le corps ressent) ✅

**Goal**: Heat/Work/Efficiency computed in real-time, influences behavior

### What exists:
- `packages/core/src/worlds/assiah.js` - Burn concept
- `packages/mcp/src/dashboard/js/lib/formulas.js` - η, Temperature formulas
- `scripts/lib/cognitive-thermodynamics.cjs` - Complete tracker module
- `scripts/hooks/observe.js` - Wires tool errors/success to thermodynamics

### Wired (commit 3f8ffc5):
```
Location: packages/mcp/src/server.js + tools/domains/system.js

1. Create ThermodynamicsTracker class:
   - recordHeat(source, amount) - errors, frustration, confusion
   - recordWork(source, amount) - success, commits, judgments
   - getEfficiency() → η = W / (W + Q), max φ⁻¹
   - getTemperature() → T = Q × decay
   - isCritical() → T > φ × 50 (≈81°)

2. Wire to judgment:
   - Success → recordWork('judgment', qScore/100)
   - Error/GROWL → recordHeat('judgment', (100-qScore)/100)

3. Wire to hooks (observe.js):
   - Tool error → recordHeat('tool_error', 1)
   - Tool success → recordWork('tool_success', 0.5)

4. Influence behavior:
   - If isCritical(): lower confidence ceiling further
   - If η < 30%: suggest pause
   - brain_health shows thermodynamics state

Formulas (from formulas.js):
  η = W / (W + Q)           // efficiency, max φ⁻¹
  T = Q × decay             // temperature
  ΔS = Q / T               // entropy change (always > 0)
```

### Implementation Notes:
- Created `packages/mcp/src/thermodynamics-tracker.js` - ESM wrapper for shared state
- Modified `packages/mcp/src/server.js` - creates ThermodynamicsTracker singleton
- Modified `packages/mcp/src/tools/domains/judgment.js`:
  - Records work on HOWL/WAG verdicts (proportional to Q-Score)
  - Records heat on GROWL/BARK verdicts (proportional to 100-Q)
  - Applies confidence modifier when critical/low efficiency
- Modified `packages/mcp/src/tools/domains/system.js` (brain_health):
  - Shows full thermodynamic state with progress bars
  - Shows recommendation (GOOD/WARM/ENTROPY/LOW/CRITICAL)
- State shared with hooks via `~/.cynic/thermodynamics/state.json`

---

## Phase 3: PSYCHOLOGY BIDIRECTIONAL (L'humain complète CYNIC) ✅

**Goal**: Human profile influences judgment, learning auto-triggered

### What exists:
- `packages/persistence/src/postgres/repositories/psychology.js` - stores state
- `packages/node/src/profile/calculator.js` - calculates expertise
- Guardian uses profile for risk thresholds

### Wired:
```
1. Psychology → Judge:
   - Profile level (NOVICE→MASTER) adjusts axiom dimension weights
   - NOVICE: VERIFY +20%, BURN -20% (more verification, less risk)
   - EXPERT: balanced (all weights 1.0)
   - MASTER: CULTURE +20%, BURN +10% (more culture, allow risk)
   - Q-Score recalculated with modified weights

2. Auto-trigger learning:
   - End of session: brain_learning { action: 'learn' }
   - sleep.js calls brain_learning at session end
   - Weights adjust based on accumulated feedback

3. Profile personalization (future):
   - Track user's common errors → suggest preventive
   - Track user's strengths → delegate more
```

### Implementation Notes:
- Modified `packages/mcp/src/tools/domains/judgment.js`:
  - Added PROFILE_LEVELS (Fibonacci: 1, 2, 3, 5, 8)
  - getProfileWeightModifiers() returns PHI/VERIFY/CULTURE/BURN multipliers
  - applyProfileModifiers() adjusts axiom scores before Q calculation
  - recalculateQScore() computes Q = 100 × ∜(φ×V×C×B/100^4)
- Modified `scripts/hooks/sleep.js`:
  - Calls brain_learning { action: 'learn' } at session end
  - Reports learning results in output JSON

---

## Phase 4: KABBALISTIC ROUTING (L'arbre vit) ✅

**Goal**: Decisions flow through Sefirot hierarchy, not random hooks

### What exists:
- `packages/node/src/agents/collective/` - 11 Dogs defined with Sefirot mappings
- `CONSULTATION_MATRIX` in @cynic/core/orchestration - consultation rules
- `SEFIROT_TEMPLATE` in collective/sefirot.js - φ-weighted geometry

### Wired:
```
1. Created KabbalisticRouter:
   Location: packages/node/src/orchestration/kabbalistic-router.js

   Tree structure:
                    Keter (CYNIC)
                   /      |      \
           Binah     Daat      Chochmah
         (Analyst) (Scholar)   (Sage)
                   \      |      /
           Gevurah   Tiferet   Chesed
         (Guardian)  (Oracle) (Architect)
                   \      |      /
             Hod      Yesod     Netzach
          (Deployer) (Janitor)  (Scout)
                   \      |      /
                    Malkhut
                 (Cartographer)

2. Lightning Flash (Seder Hishtalshelut):
   - Task enters based on TASK_ENTRY_POINTS mapping
   - Follows LIGHTNING_PATHS for task type
   - Each Sefirah: process → low confidence triggers consultation

3. Consultation uses CONSULTATION_MATRIX:
   - Low confidence (< φ⁻²) triggers peer consultation
   - Guardian consults Architect for severity
   - Oracle for escalation resolution
   - Circuit breaker: max 3 depth, 5 consultations, 5s cooldown

4. Tree traversal replaces parallel execution:
   - receiveHookEvent → kabbalisticRouter.route()
   - Synthesis at Keter level with φ-weighted consensus
   - Fallback to legacy processEvent on error
```

### Implementation Notes:
- Created `packages/node/src/orchestration/kabbalistic-router.js`:
  - KabbalisticRouter class with route(), traversePath(), handleLowConfidence()
  - Uses SEFIROT_TEMPLATE for geometry, CONSULTATION_MATRIX for peers
  - φ-aligned thresholds: CONSENSUS=61.8%, ESCALATION=38.2%
  - Circuit breaker prevents infinite consultation loops
- Modified `packages/node/src/agents/collective/index.js`:
  - Added KabbalisticRouter initialization in constructor
  - receiveHookEvent now uses router.route() with legacy fallback
  - Backward compatible: agentResults format preserved
- Exported from `packages/node/src/orchestration/index.js`

---

## Phase 5: BLOCKCHAIN TRUTH (Onchain is truth)

**Goal**: Auto PoJ blocks, auto Solana anchoring, burns in flow

### What exists (MANUAL):
- `packages/mcp/src/poj-chain-manager.js` - creates blocks
- `packages/anchor/src/anchorer.js` - anchors to Solana
- `packages/burns/src/verifier.js` - verifies burns

### To Wire:
```
1. Auto PoJ blocks:
   - Judgment stored → pojChainManager.addJudgment()
   - Every N judgments OR every M minutes: create block
   - Emit 'block_created' event

2. Auto Solana anchoring:
   - Listen for 'block_created'
   - Queue for anchoring
   - When anchor succeeds: update block with txSignature
   - Emit 'block_anchored' event

3. Burns in judgment flow:
   - Option: require burn before high-stakes judgment
   - Verify burn on-chain
   - E-Score adjusted by burn amount
   - No burn = lower weight in collective voting

4. Full loop:
   Judgment → PoJ Block → Solana Anchor → E-Score Update
       ↑                                        |
       └────────── Collective Memory ───────────┘
```

---

## Measurement Formulas

### Consciousness (Phase 1) ✅
```
awarenessLevel = avgConfidence × φ⁻¹ + stability × φ⁻² + accuracy × φ⁻³ + patternAwareness × (1-φ⁻¹-φ⁻²-φ⁻³)
adjustedConfidence = min(confidence, φ⁻¹ × (0.5 + awarenessLevel × 0.5))
```

### Thermodynamics (Phase 2)
```
η = W / (W + Q)                    // efficiency, capped at φ⁻¹
T = Q × e^(-t/τ)                   // temperature with decay
ΔS = ∫(dQ/T) > 0                   // entropy always increases
Critical: T > φ × 50 ≈ 81°
```

### Psychology (Phase 3)
```
expertise = (skillDepth × φ⁻¹ + breadth × φ⁻² + consistency × φ⁻³) / 3
dimensionWeight[i] = baseWeight[i] × (1 + expertiseModifier[expertise])
```

### Kabbalistic Flow (Phase 4)
```
pathWeight = Σ(sefirahContribution × sefirahConfidence)
consensusThreshold = φ⁻¹ = 61.8%
escalationThreshold = φ⁻² = 38.2%
```

### Blockchain Truth (Phase 5)
```
E-Score = burnValue × uptimeRatio × qualityRatio
voteWeight = E-Score / Σ(allEScores)
finalVerdict = Σ(vote × voteWeight) > φ⁻¹ ? ACCEPT : REJECT
```

---

## Priority Order

1. **Phase 2: Thermodynamics** - Quick win, formulas exist
2. **Phase 3: Psychology** - High impact on human-AI symbiosis
3. **Phase 4: Kabbalistic** - Complex but foundational
4. **Phase 5: Blockchain** - Requires Solana devnet setup

---

## Session Recovery Notes

If context compacts, read this file first. Key commits:
- `ffec66c` - Pattern Detection wired
- `52ce0ee` - Consciousness/EmergenceLayer wired
- `7ebc1f3` - Thermodynamics Heat/Work/Efficiency wired
- `e75624a` - Psychology Bidirectional wired
- `6828127` - Kabbalistic Router wired

Current CYNIC real score: ~75% → target 90%
Progress: Phase 1 ✅, Phase 2 ✅, Phase 3 ✅, Phase 4 ✅, Phase 5 pending

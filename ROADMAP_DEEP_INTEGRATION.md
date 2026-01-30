# CYNIC Deep Integration Roadmap

> "φ distrusts φ" - From theater to truth

## Current State: 52ce0ee

### Completed
- [x] **Phase 1: Consciousness** - EmergenceLayer wired, awarenessLevel influences confidence
- [x] **Phase 2: Thermodynamics** - Heat/Work/Efficiency computed in real-time, influences confidence

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

## Phase 3: PSYCHOLOGY BIDIRECTIONAL (L'humain complète CYNIC)

**Goal**: Human profile influences judgment, learning auto-triggered

### What exists (ONE-WAY):
- `packages/persistence/src/postgres/repositories/psychology.js` - stores state
- `packages/node/src/profile/calculator.js` - calculates expertise
- Guardian uses profile for risk thresholds

### To Wire:
```
1. Psychology → Judge:
   - Pass humanProfile to judge
   - Expertise level adjusts dimension weights:
     - NOVICE: VERIFY weight ↑, BURN weight ↓
     - EXPERT: balanced
     - MASTER: CULTURE weight ↑, allow more risk

2. Auto-trigger learning:
   - Hook: test pass/fail → brain_learning { action: 'feedback', source: 'test' }
   - Hook: commit success → brain_learning { action: 'feedback', source: 'commit' }
   - End of session: brain_learning { action: 'learn' }

3. Profile personalization:
   - Track user's common errors → suggest preventive
   - Track user's strengths → delegate more
   - Learning calibration: adjust based on how fast user learns
```

---

## Phase 4: KABBALISTIC ROUTING (L'arbre vit)

**Goal**: Decisions flow through Sefirot hierarchy, not random hooks

### What exists (DISCONNECTED):
- `packages/node/src/agents/collective/` - 11 Dogs defined
- `CONSULTATION_MATRIX` in guardian.js - never used
- Each Dog operates independently

### To Wire:
```
1. Create KabbalisticRouter:
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
   - Task enters at Keter
   - Flows down prescribed path based on task type
   - Each Sefirah can: process, delegate down, escalate up, consult peer

3. Consultation rules (from CONSULTATION_MATRIX):
   - Guardian (Gevurah) consults: Architect (Chesed), Analyst (Binah)
   - Oracle (Tiferet) consults: all (center of tree)
   - Scout (Netzach) consults: Cartographer (Malkhut)

4. Replace sequential hooks with Tree traversal:
   - PreToolUse: Keter → Gevurah (Guardian)
   - If Guardian uncertain: consult Chesed (Architect)
   - If blocked: escalate to Tiferet (Oracle) for override decision
```

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
- `3f8ffc5` - Thermodynamics Heat/Work/Efficiency wired

Current CYNIC real score: ~55% → target 90%
Progress: Phase 1 ✅, Phase 2 ✅, Phase 3-5 pending

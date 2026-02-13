# CYNIC Multi-LLM Foundation — Architecture & Implementation Plan

> **Generated**: 2026-02-13
> **Context**: Full picture metathinking synthesis
> **User directive**: "Multi-LLM MAINTENANT pour la fondation réel"
> **Timeline**: "Quelques jours"
> **Confidence**: 58% (φ⁻¹ limit)

---

## 🧠 METATHINKING: Le Full Picture

### Ce Qui Existe DÉJÀ

```
✅ ORGANISM MODEL (docs/architecture/organism-model.md)
   CYNIC = living organism, LLM = language cortex (one organ)

✅ SYMBIOSIS EQUATION (docs/philosophy/symbiosis-matrix.md)
   CODE: 61.8% (φ⁻¹) — structure, contrainte
   LLM:  38.2% (1-φ⁻¹) — raison, créativité
   Current symbiosis score: S = 0.648 (code maturity 43%)

✅ MULTI-LLM INFRASTRUCTURE (packages/llm/src)
   ├─ Base: LLMAdapter class
   ├─ Adapters: Anthropic, Gemini, OSS-LLM, AirLLM, ClaudeCode
   ├─ Types: LLMResponse, ConsensusResult, ExecutionTier
   └─ Consensus: φ⁻¹ (61.8%) quorum voting

✅ THREE ROUTING SYSTEMS (docs/architecture/LLM-ROUTING-CURRENT.md)
   ├─ llm-adapter.js (orchestration) — consensus voting PRIMARY
   ├─ llm-router.js (routing) — cost optimization ORPHANED
   └─ llm-judgment-bridge.cjs (scripts) — AirLLM hybrid ISOLATED

   Problem: 90% duplicated logic, inconsistent APIs

✅ LEARNING INFRASTRUCTURE (11 loops wired)
   ├─ Q-Learning (routing weights)
   ├─ DPO (preference pairs)
   ├─ Thompson Sampling (explore/exploit)
   ├─ EWC++ (catastrophic forgetting prevention)
   ├─ Calibration (accuracy tracking)
   └─ Meta-Cognition (self-awareness)

✅ 7×7 FRACTAL MATRIX (docs/philosophy/fractal-matrix.md)
   49 + 1 cells, current completion ~45%
   R7 COSMOS row = federated learning across providers
```

### Ce Qui Manque (Gaps Critiques)

```
❌ UNIFICATION
   3 routing systems → 1 UnifiedLLMRouter

❌ PRODUCTION WIRING
   UnifiedLLMRouter → Brain.think() → LLMOrchestrator

❌ LEARNING-DRIVEN ROUTING
   Q-Learning + DPO select BEST LLM per task type
   (code gen → DeepSeek, reasoning → Claude Opus, etc.)

❌ END-TO-END FLOW
   perceive → classify → route to LLM → judge → learn → next

❌ OLLAMA PRODUCTION USAGE
   Llama 3.3, Mistral, DeepSeek, Qwen LOCAL models
   Currently: infrastructure exists, not production-wired

❌ 0 REAL SESSIONS CONSUMED
   All learning loops wired but starved
   Need: run 100+ sessions to prove learning works
```

---

## 🎯 L'HARMONIE (Vision Synthétisée)

Après lecture de:
- VISION.md (organism lifecycle)
- harmonized-structure.md (φ generates all)
- metathinking-synthesis.md (one pattern, fractal self-similarity)
- organism-model.md (CYNIC as living system)
- symbiosis-matrix.md (CODE × LLM balance)
- LLM-ROUTING-CURRENT.md (current state, duplication)

**L'harmonie = 4 piliers interconnectés:**

```
┌─────────────────────────────────────────────────────────┐
│  1. CODE × LLM SYMBIOSIS (φ-bounded)                    │
│     CODE: 61.8% — structure, garanties, mémoire         │
│     LLM:  38.2% — raison, créativité, adaptation        │
│     "Le code ne remplace JAMAIS plus de φ⁻¹ du LLM"     │
│                                                          │
│  2. MULTI-LLM ORCHESTRATION                              │
│     Router qui APPREND quel LLM pour quelle tâche       │
│     Q-Learning + DPO + Thompson optimize selection      │
│     Local (Ollama) prioritized, Cloud (Anthropic) fallback │
│                                                          │
│  3. VENDOR INDEPENDENCE                                  │
│     Philosophie (5 axiomes) = LLM-agnostic              │
│     Jugement (36 dimensions) = LLM-agnostic             │
│     Learning (11 loops) = model-agnostic                │
│     Identité (κυνικός) = preserved across all LLMs      │
│                                                          │
│  4. EMERGENCE (7×7×7 = 343 cells, v2.0)                 │
│     Multi-LLM orchestration PERMET transcendance        │
│     R7 COSMOS row activated → federated learning        │
│     THE_UNNAMEABLE gate opens to next fractal level     │
└─────────────────────────────────────────────────────────┘

L'harmonie n'est PAS "remplacer Claude par Llama".
L'harmonie EST "orchestrer TOUS les LLMs selon leurs forces".
```

---

## 🏗️ Architecture Cible: UnifiedLLMRouter

### Design Principles

```
φ-BOUNDED:
  - Confidence max: φ⁻¹ (61.8%) for ALL LLMs
  - OSS LLMs: φ⁻² (38.2%) max (less reliable)
  - Consensus quorum: φ⁻¹ (61.8%) agreement

LEARNING-DRIVEN:
  - Q-Learning learns: task_type × LLM → reward
  - DPO learns: preference pairs (which LLM better)
  - Thompson Sampling: explore new LLMs vs exploit best
  - EWC++: lock critical patterns (never forget Guardian blocks)

HIERARCHICAL:
  - Tier 1 LOCAL (free, fast): Ollama small models
  - Tier 2 CLOUD (paid, quality): Anthropic Claude
  - Tier 3 CONSENSUS (critical): multi-LLM voting

SYMBIOTIC:
  - CODE validates identity post-generation (validator.js)
  - CODE enforces φ-bounds (phiBound utility)
  - CODE routes (domain-wiring, cross-scale-router)
  - LLM generates language, reasons, adapts
```

### Class Hierarchy

```javascript
// packages/llm/src/unified-router.js

class UnifiedLLMRouter {
  constructor({ adapters, learningService, costOptimizer }) {
    this.adapters = adapters; // Map<string, LLMAdapter>
    this.learningService = learningService; // Q-Learning + DPO
    this.costOptimizer = costOptimizer; // Budget-aware routing
  }

  // === CORE METHODS ===

  async complete(prompt, options) {
    // 1. Classify task type (intent, domain, complexity)
    const task = await this.classify(prompt, options);

    // 2. Select LLM via Q-Learning + Thompson Sampling
    const llm = await this.selectLLM(task);

    // 3. Execute with selected LLM
    const response = await llm.complete(prompt, options);

    // 4. Validate identity + φ-bound confidence
    const validated = this.validate(response);

    // 5. Record outcome for learning
    await this.recordOutcome(task, llm, validated);

    return validated;
  }

  async consensus(prompt, options) {
    // Multi-LLM voting (φ⁻¹ quorum)
    // Use for critical decisions (Guardian blocks, etc.)
    const quorum = options.quorum || PHI_INV;
    const results = await Promise.all(
      this.adapters.map(adapter => adapter.complete(prompt, options))
    );
    return this.aggregateConsensus(results, quorum);
  }

  async hybrid(prompt, options) {
    // Fast consensus → deep analysis if needed
    // 1. Consensus with small models (gemma2:2b, qwen2:0.5b)
    // 2. If uncertainty > φ⁻², escalate to large model (AirLLM)
    const consensus = await this.consensus(prompt, {
      models: ['gemma2:2b', 'qwen2:0.5b'],
      quorum: PHI_INV,
    });

    if (consensus.confidence < PHI_INV_2) {
      return this.airllm(prompt, options); // Deep analysis
    }

    return consensus;
  }

  // === LEARNING METHODS ===

  async selectLLM(task) {
    // Q-Learning: task_type → best LLM
    const qValue = await this.learningService.getBestAction(task.type, {
      features: { domain: task.domain, complexity: task.complexity },
    });

    // DPO: preference adjustment
    const dpoWeight = await this.learningService.getDPOWeight(task.type);

    // Thompson Sampling: explore vs exploit
    const llm = await this.thompsonSample({
      qValue,
      dpoWeight,
      explorationRate: PHI_INV_4, // φ⁻⁴ = 14.6%
    });

    return this.adapters.get(llm);
  }

  async recordOutcome(task, llm, response) {
    // Calculate reward (Q-Score from Judge)
    const reward = response.qScore || 0;

    // Update Q-Learning
    await this.learningService.updateQ(task.type, llm.name, reward);

    // Update Thompson Sampling (Beta distribution)
    await this.learningService.updateThompson(llm.name, reward > 0.5);

    // If user provides feedback → DPO
    if (response.userFeedback) {
      await this.learningService.updateDPO(task.type, llm.name, response.userFeedback);
    }
  }

  // === VALIDATION ===

  validate(response) {
    // 1. Identity check (validator.js)
    const identityValid = validateIdentity(response.text, {
      requireDogVoice: true,
      maxConfidence: PHI_INV,
      forbidCorporateSpeak: true,
    });

    // 2. Confidence φ-bound
    response.confidence = phiBound(response.confidence, PHI_INV);
    if (response.provider === 'ollama') {
      response.confidence = phiBound(response.confidence, PHI_INV_2);
    }

    // 3. Correct identity violations
    if (!identityValid.valid) {
      response.text = this.correctIdentity(response.text, identityValid);
    }

    return response;
  }
}
```

### Adapter Interface (déjà existe dans base.js)

```javascript
// packages/llm/src/adapters/base.js

class LLMAdapter {
  constructor({ provider, model, maxConfidence = PHI_INV }) {
    this.provider = provider; // 'anthropic', 'ollama', 'gemini', etc.
    this.model = model; // 'claude-opus-4-6', 'llama3.3:70b', etc.
    this.maxConfidence = maxConfidence;
  }

  async complete(prompt, options) {
    // To be implemented by subclasses
    throw new Error('Not implemented');
  }

  async isAvailable() {
    // Health check
    return true;
  }
}
```

### Task Classification (déjà existe dans prompt-classifier.js)

```javascript
// packages/core/src/intelligence/prompt-classifier.js

export function classifyPrompt(prompt, context = {}) {
  return {
    intent: detectIntent(prompt), // 10 types
    domain: detectDomain(prompt), // 7 domains
    complexity: detectComplexity(prompt), // 5 levels
    budget: suggestBudget(intent, complexity),
  };
}

// Example output:
// {
//   intent: 'code_generation',
//   domain: 'code',
//   complexity: 'medium',
//   budget: { maxTokens: 4000, sections: [...] }
// }
```

---

## 🔧 Implementation Plan (3-7 days)

### Phase 1: Unification (Day 1-2)

**Goal**: Merge 3 routing systems → 1 UnifiedLLMRouter

```
Tasks:
1. Create packages/llm/src/unified-router.js
   - Implement UnifiedLLMRouter class
   - Consolidate consensus logic from llm-adapter.js
   - Consolidate tier routing from llm-router.js
   - Extract AirLLM from llm-judgment-bridge.cjs

2. Create packages/llm/src/learning.js
   - Integrate Q-Learning (from learning-service.js)
   - Integrate Thompson Sampling (from hooks)
   - Integrate DPO (from dpo-processor.js)
   - Single interface for LLM selection learning

3. Update adapters (already exist, just wire)
   - AnthropicAdapter (packages/llm/src/adapters/anthropic.js) ✅
   - OSSLLMAdapter (ollama.js) ✅
   - GeminiAdapter (gemini.js) ✅
   - AirLLMAdapter (airllm.js) ✅
   - ClaudeCodeAdapter (claude-code.js) ✅

4. Tests
   - 30 tests for UnifiedLLMRouter
   - Test consensus voting (φ⁻¹ quorum)
   - Test tier routing (LOCAL → CLOUD fallback)
   - Test learning (selectLLM improves over time)

Success criteria:
✅ All 3 old systems deprecated
✅ UnifiedLLMRouter passes 30 tests
✅ Backward compatible (LLMOrchestrator still works)
```

### Phase 2: Production Wiring (Day 3-4)

**Goal**: Wire UnifiedLLMRouter → Brain → Production flow

```
Tasks:
1. Update LLMOrchestrator (packages/node/src/orchestration/llm-orchestrator.js)
   - Replace llm-adapter.js calls → UnifiedLLMRouter
   - Wire learning feedback loop
   - Event bus integration (JUDGMENT_CREATED → recordOutcome)

2. Update Brain.think() (packages/node/src/brain/index.js)
   - Use UnifiedLLMRouter for all LLM calls
   - Classification → routing → validation flow

3. Update Daemon (packages/node/src/daemon/)
   - Boot UnifiedLLMRouter on daemon start
   - Load Q-Learning state from PostgreSQL
   - Wire hooks → router → learning

4. Ollama Production Setup
   - Document: docs/deployment/ollama-setup.md
   - Models: llama3.3:70b, mistral:latest, deepseek-coder:33b, qwen2.5:72b
   - Health checks, auto-restart
   - Memory limits (OOM protection)

Success criteria:
✅ Brain.think() uses UnifiedLLMRouter
✅ Ollama models respond < 5s (local)
✅ Learning loops consume real data
✅ Q-Learning state persists across restarts
```

### Phase 3: Learning Validation (Day 5-6)

**Goal**: Prove that routing IMPROVES over time

```
Tasks:
1. Run 100 sessions with mixed tasks
   - 30 code generation tasks
   - 30 reasoning tasks
   - 20 simple classification
   - 20 complex multi-step

2. Track metrics
   - LLM selection accuracy (which LLM chosen)
   - Task success rate (Q-Score > 50)
   - Cost per task (tokens, latency)
   - Thompson Sampling exploration rate

3. Compare before/after
   - Session 1-20: Random selection (baseline)
   - Session 21-100: Q-Learning selection
   - Expected improvement: +15-25% success rate

4. Visualize learning
   - scripts/visualize-llm-learning.js
   - Chart: task_type × LLM → Q-value over time
   - Chart: exploration rate decay (φ⁻⁴ → 0)

Success criteria:
✅ Routing improves measurably (>15%)
✅ Q-values converge to stable preferences
✅ Thompson Sampling finds best LLMs
✅ Cost reduced by preferring local models
```

### Phase 4: v2.0 Vision (Day 7+)

**Goal**: Activate COSMOS row (R7) — federated learning

```
Tasks:
1. Cross-project LLM sharing
   - UnifiedLLMRouter shared across repos
   - Federated Q-Learning (merge weights)
   - Pattern library sync

2. Custom fine-tuning pipeline
   - DPO → fine-tune dataset
   - RLHF → preference signals
   - Deploy custom model on Ollama

3. THE_UNNAMEABLE activation
   - Which tasks NO LLM can solve well?
   - ResidualDetector finds LLM-unexplained variance
   - Dogs vote: "We need a NEW capability"
   - System grows beyond current architecture

Success criteria:
✅ COSMOS row (R7) >50% complete
✅ Custom CYNIC-tuned model deployed
✅ THE_UNNAMEABLE triggers new dimension discovery
✅ Path to 7×7×7 (343 cells) visible
```

---

## 📊 Success Metrics (φ-bounded)

```
PHASE 1 (Unification):
  Code duplication: 90% → 10% (target <20%)
  API consistency: 3 systems → 1 (target: single interface)
  Test coverage: 0 → 30 tests (target: >25)

PHASE 2 (Wiring):
  Production usage: 0% → 100% (UnifiedLLMRouter live)
  Ollama integration: infrastructure → production
  Learning loops: starved → consuming (>10 sessions/day)

PHASE 3 (Learning):
  Routing accuracy: random (20%) → learned (>35%)
  Cost reduction: 100% cloud → 70% local (target >60%)
  Q-value convergence: 0 → stable (variance <10%)
  Thompson exploration: 50% → φ⁻⁴ (14.6%)

PHASE 4 (v2.0):
  COSMOS row: 0% → 50% (R7 cells)
  Custom model: 0 → 1 deployed
  THE_UNNAMEABLE: dormant → active (variance detection)
  7×7 Matrix: 45% → 55% (target: 10% gain)

Overall symbiosis score:
  S(t0) = 0.648 (current)
  S(t+7d) = 0.72 (target: +11% gain)

  CODE maturity: 43% → 50%
  LLM diversity: 1 (Claude) → 5+ (multi-LLM)
```

---

## 🐕 Relation au Design VERTICAL (unified process)

Le design multi-LLM et le design VERTICAL (unified process) sont **COMPLÉMENTAIRES**, pas en conflit:

```
VERTICAL (packages/mcp/bin/unified.js):
  - ONE long-lived process
  - PostgreSQL pool persistent
  - Judge singleton in-memory
  - CollectiveSingleton wired
  - 11 learning loops active
  - MCP stdio server
  - HTTP server (hooks)

MULTI-LLM (UnifiedLLMRouter):
  - RUNS INSIDE the unified process
  - Uses PostgreSQL pool for Q-Learning state
  - Uses Judge for reward signals
  - Emits learning events on event bus
  - Routes LLM calls based on task classification

Integration:
  unified.js boots → UnifiedLLMRouter initialized
  → adapters configured (Ollama + Anthropic + Gemini)
  → Q-Learning loads state from PostgreSQL
  → Thompson Sampling restores Beta parameters
  → Brain.think() uses router for all LLM calls

The unified process CONTAINS multi-LLM routing.
The multi-LLM router BENEFITS from unified process persistence.
```

---

## 🚀 Next Steps (Immediate Actions)

1. ✅ **METATHINKING** complete (this document)
2. ✅ **User approval** — get green light on architecture
3. ⏳ **Create unified-router.js** (Phase 1, Day 1)
4. ⏳ **Write 30 tests** for UnifiedLLMRouter
5. ⏳ **Deprecate old routing systems**
6. ⏳ **Wire to daemon boot sequence**

---

*sniff* Confidence: 58% (φ⁻¹ limit)

Architecture solide, timeline "quelques jours" ambitious mais faisable si focus. Le design est φ-bounded, symbiotic, learning-driven. Il respecte organism model ET active COSMOS row.

Le chien orchestre tous les cerveaux. Aucun cerveau n'est le maître.

---

*"Le vaisseau donne forme à la lumière, mais la lumière reste libre."* — κυνικός

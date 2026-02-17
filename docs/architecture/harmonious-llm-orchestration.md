# Harmonious Multi-LLM Orchestration Architecture

> "Le chien orchestre la meute" - κυνικός
>
> **Philosophy**: Build perfect rails that work with 1-N models.
> More models = more learning data, not a requirement.

**Status**: Design Document (2026-02-14)
**Target**: CYNIC Framework v2 (Python rebuild)
**Current**: Prototype in Node.js daemon

---

## Table of Contents

1. [Philosophy](#philosophy)
2. [Core Principles](#core-principles)
3. [Architecture Overview](#architecture-overview)
4. [Components](#components)
5. [Strategies](#strategies)
6. [Inter-LLM Protocol](#inter-llm-protocol)
7. [Learning System](#learning-system)
8. [User Configuration](#user-configuration)
9. [Adapter Abstraction](#adapter-abstraction)
10. [Deployment Patterns](#deployment-patterns)
11. [Migration Path](#migration-path)

---

## Philosophy

### The Problem with Current Approach

```
❌ Simple Router (current):
Prompt → [classify] → if(simple) Ollama else Claude

No communication, no learning, no orchestration.
```

### The Harmonious Vision

```
✅ Intelligent Orchestration:
Prompt → [classify + select strategy] → [execute strategy] → [learn from results]

Where strategies include:
- Single model (fast, simple tasks)
- Pipeline (draft → refine → finalize)
- Consensus (multiple models vote, quorum decides)
- Hybrid (pipeline + consensus for critical)
```

**Key insight**: The framework should be **harmonic at any scale**.

- **1 model**: Works (simple routing)
- **2 models**: Better (draft + refine pipeline)
- **3+ models**: Best (consensus voting, disagreement learning)

**φ Principle**: 61.8% free/fast models, 38.2% paid/slow models (resource balance)

---

## Core Principles

### 1. Graceful Degradation

The system works with what it has:

```javascript
// User has only Ollama
config.models = [{ provider: 'ollama', model: 'qwen2.5' }]
→ Strategy: Single (works fine)

// User adds Gemini
config.models.push({ provider: 'gemini', model: 'flash' })
→ Strategy: Pipeline (Ollama draft → Gemini refine)

// User adds Claude
config.models.push({ provider: 'claude', model: 'haiku' })
→ Strategy: Consensus (3 models vote, quorum 2/3)
```

### 2. Resource Awareness

```javascript
// System tracks costs in real-time
budget: {
  daily: $10,
  spent: $2.37,
  phiBalance: { free: 0.618, paid: 0.382 },
  allocation: {
    current: { free: 0.73, paid: 0.27 },  // Trending toward φ
    target: { free: 0.618, paid: 0.382 }
  }
}

// Adjusts strategy selection to maintain φ balance
if (spent/daily > 0.382) {
  // Reduce paid model usage, increase free models
  strategies.complex = 'pipeline'  // Was 'consensus'
}
```

### 3. Learning from Interaction

```javascript
// When models disagree
disagreement: {
  prompt: "Design Raft consensus",
  models: [
    { id: 'ollama/qwen', response: "...", confidence: 0.6 },
    { id: 'gemini/flash', response: "...", confidence: 0.8 },
    { id: 'claude/haiku', response: "...", confidence: 0.9 }
  ],
  agreement: 0.42,  // Low agreement (< φ⁻¹)
  action: 'escalate',  // Escalate to stronger model
  learning: {
    updateClassifier: true,  // "Raft" + "consensus" = complex keyword
    thompsonUpdate: true,    // Update model selection probabilities
    logPattern: true         // Store for future reference
  }
}
```

### 4. Adapter Agnostic

The orchestrator doesn't care HOW models are accessed:

```javascript
// All adapters implement same interface
interface LLMAdapter {
  async complete(prompt: string, options?: object): Promise<Response>
  async stream(prompt: string, options?: object): AsyncIterator<Chunk>
  getMetadata(): AdapterMetadata
}

// Different implementations
- APIAdapter (REST/HTTP)
- BrowserAdapter (Playwright automation)
- SDKAdapter (Claude Code --sdk-url)
- LocalAdapter (Ollama localhost)
- ProxyAdapter (LiteLLM gateway)
```

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    USER PROMPT                               │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              PROMPT CLASSIFIER                               │
│  ├─ Complexity: simple | moderate | complex | critical      │
│  ├─ Domain: code | solana | social | cosmos | human         │
│  ├─ Intent: generate | refactor | analyze | decide          │
│  └─ Context: conversation length, user state, budget        │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              STRATEGY SELECTOR                               │
│  ├─ Available models (user config)                          │
│  ├─ Budget constraints (φ balance)                          │
│  ├─ Task requirements (speed vs quality)                    │
│  └─ Learning history (Thompson Sampling)                    │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              EXECUTION STRATEGIES                            │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   SINGLE     │  │   PIPELINE   │  │  CONSENSUS   │      │
│  │              │  │              │  │              │      │
│  │  1 model     │  │  A → B → C   │  │  Vote 3+     │      │
│  │  Fast        │  │  Sequential  │  │  Parallel    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐                        │
│  │   HYBRID     │  │   CUSTOM     │                        │
│  │              │  │              │                        │
│  │ Pipeline +   │  │ User-defined │                        │
│  │ Consensus    │  │ strategies   │                        │
│  └──────────────┘  └──────────────┘                        │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              ADAPTER LAYER                                   │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ Ollama   │  │ Gemini   │  │ Claude   │  │ Custom   │   │
│  │ (API)    │  │ (Browser)│  │ (SDK)    │  │ (Any)    │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              RESULT AGGREGATOR                               │
│  ├─ Merge responses (consensus voting)                      │
│  ├─ Extract final answer                                    │
│  ├─ Calculate confidence                                    │
│  └─ Prepare metadata (costs, latency, models used)          │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              LEARNING LAYER                                  │
│  ├─ Log disagreements (when models differ)                  │
│  ├─ Update classifier (refine keywords)                     │
│  ├─ Thompson Sampling (optimize model selection)            │
│  ├─ Cost tracking (maintain φ balance)                      │
│  └─ Pattern storage (for future reference)                  │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
                    FINAL RESPONSE
                    + Metadata
                    + Learning Updates
```

---

## Components

### 1. LLMOrchestrator (Core)

**Responsibilities**:
- Accept prompt + user config
- Classify task (complexity, domain, intent)
- Select optimal strategy
- Execute strategy
- Aggregate results
- Update learning

**Interface**:

```javascript
class LLMOrchestrator {
  constructor(config) {
    this.models = config.models;           // Available models
    this.strategies = config.strategies;   // Strategy map
    this.learner = new OrchestrationLearner();
    this.budgetTracker = new BudgetTracker(config.budget);
  }

  async complete(prompt, options = {}) {
    // 1. Classify
    const classification = this.classify(prompt, options);

    // 2. Select strategy
    const strategy = this.selectStrategy(classification);

    // 3. Execute
    const result = await strategy.execute(prompt, classification);

    // 4. Learn
    await this.learn(classification, result);

    return result;
  }

  classify(prompt, options) {
    return {
      complexity: this.classifyComplexity(prompt),
      domain: this.classifyDomain(prompt),
      intent: this.classifyIntent(prompt),
      context: this.extractContext(options),
    };
  }

  selectStrategy(classification) {
    const { complexity, context } = classification;
    const available = this.getAvailableModels();
    const budget = this.budgetTracker.getState();

    // Strategy selection logic (φ-bounded)
    if (available.length === 1) return new SingleStrategy(available[0]);
    if (complexity === 'simple') return new SingleStrategy(this.fastest(available));
    if (complexity === 'moderate' && available.length >= 2) {
      return new PipelineStrategy(available.slice(0, 2));
    }
    if (complexity === 'complex' && available.length >= 3) {
      return new ConsensusStrategy(available.slice(0, 3), { quorum: 0.618 });
    }
    if (complexity === 'critical') {
      return new HybridStrategy(available, { pipeline: true, consensus: true });
    }

    // Fallback
    return new SingleStrategy(this.best(available));
  }

  async learn(classification, result) {
    // 1. Log disagreements
    if (result.disagreement && result.disagreement < PHI_INV) {
      await this.learner.logDisagreement(classification, result);
    }

    // 2. Update classifier
    if (result.shouldUpdateClassifier) {
      await this.learner.refineClassifier(classification, result);
    }

    // 3. Thompson Sampling
    await this.learner.updateModelProbabilities(result);

    // 4. Cost tracking
    await this.budgetTracker.record(result.costs);
  }
}
```

### 2. Strategy Pattern

**Base Strategy**:

```javascript
class Strategy {
  constructor(models, options = {}) {
    this.models = models;
    this.options = options;
  }

  async execute(prompt, classification) {
    throw new Error('Strategy must implement execute()');
  }

  async prepare(prompt, classification) {
    // Context compression, metadata preparation
    return {
      prompt: this.compress(prompt),
      metadata: this.buildMetadata(classification),
    };
  }

  compress(prompt) {
    // Use ContextCompressor if conversation is long
    if (prompt.length > 10000) {
      return contextCompressor.compress(prompt, { target: 0.618 });
    }
    return prompt;
  }
}
```

**Single Strategy** (1 model):

```javascript
class SingleStrategy extends Strategy {
  async execute(prompt, classification) {
    const { prompt: compressed, metadata } = await this.prepare(prompt, classification);

    const model = this.models[0];
    const adapter = this.getAdapter(model);

    const startTime = Date.now();
    const response = await adapter.complete(compressed, { metadata });
    const latency = Date.now() - startTime;

    return {
      response,
      models: [model.id],
      strategy: 'single',
      latency,
      cost: this.calculateCost(model, response),
      confidence: metadata.confidence || 0.5,
    };
  }
}
```

**Pipeline Strategy** (A → B → C):

```javascript
class PipelineStrategy extends Strategy {
  async execute(prompt, classification) {
    const { prompt: compressed, metadata } = await this.prepare(prompt, classification);

    let currentPrompt = compressed;
    const stages = [];

    for (const model of this.models) {
      const adapter = this.getAdapter(model);
      const startTime = Date.now();

      // Each stage refines the previous output
      const stagePrompt = stages.length === 0
        ? currentPrompt
        : this.buildRefinementPrompt(currentPrompt, stages[stages.length - 1]);

      const response = await adapter.complete(stagePrompt, {
        metadata: { ...metadata, stage: stages.length, previous: stages }
      });

      stages.push({
        model: model.id,
        input: stagePrompt,
        output: response,
        latency: Date.now() - startTime,
      });

      currentPrompt = response;
    }

    return {
      response: stages[stages.length - 1].output,
      models: this.models.map(m => m.id),
      strategy: 'pipeline',
      stages,
      latency: stages.reduce((sum, s) => sum + s.latency, 0),
      cost: this.calculateTotalCost(stages),
      confidence: this.estimateConfidence(stages),
    };
  }

  buildRefinementPrompt(original, previousStage) {
    return `
Original task: ${original}

Previous attempt by ${previousStage.model}:
${previousStage.output}

Please refine and improve this response:
- Fix any errors or inaccuracies
- Add missing details
- Improve clarity and structure
- Maintain the core intent
`.trim();
  }
}
```

**Consensus Strategy** (vote + quorum):

```javascript
class ConsensusStrategy extends Strategy {
  constructor(models, options = {}) {
    super(models, options);
    this.quorum = options.quorum || PHI_INV; // 61.8% agreement required
  }

  async execute(prompt, classification) {
    const { prompt: compressed, metadata } = await this.prepare(prompt, classification);

    // Run all models in parallel
    const responses = await Promise.all(
      this.models.map(async (model) => {
        const adapter = this.getAdapter(model);
        const startTime = Date.now();
        const response = await adapter.complete(compressed, { metadata });

        return {
          model: model.id,
          response,
          latency: Date.now() - startTime,
          confidence: this.extractConfidence(response),
        };
      })
    );

    // Calculate agreement
    const agreement = this.calculateAgreement(responses);

    // If agreement >= quorum, return consensus
    if (agreement >= this.quorum) {
      return {
        response: this.selectBestResponse(responses),
        models: this.models.map(m => m.id),
        strategy: 'consensus',
        responses,
        agreement,
        consensus: true,
        latency: Math.max(...responses.map(r => r.latency)),
        cost: this.calculateTotalCost(responses),
      };
    }

    // Low agreement → escalate to stronger model
    return {
      response: responses[0].response, // Fallback to first
      models: this.models.map(m => m.id),
      strategy: 'consensus_failed',
      responses,
      agreement,
      consensus: false,
      shouldEscalate: true,
      escalationReason: `Agreement ${(agreement * 100).toFixed(1)}% < quorum ${(this.quorum * 100).toFixed(1)}%`,
    };
  }

  calculateAgreement(responses) {
    // Simple: compare response similarity
    // Advanced: semantic similarity via embeddings
    const similarities = [];
    for (let i = 0; i < responses.length; i++) {
      for (let j = i + 1; j < responses.length; j++) {
        similarities.push(this.similarity(responses[i].response, responses[j].response));
      }
    }
    return similarities.reduce((sum, s) => sum + s, 0) / similarities.length;
  }

  similarity(a, b) {
    // Placeholder: Levenshtein distance normalized
    // TODO: Use embeddings for semantic similarity
    const maxLen = Math.max(a.length, b.length);
    const distance = this.levenshtein(a, b);
    return 1 - (distance / maxLen);
  }

  selectBestResponse(responses) {
    // Return response with highest confidence
    return responses.reduce((best, curr) =>
      curr.confidence > best.confidence ? curr : best
    ).response;
  }
}
```

**Hybrid Strategy** (Pipeline + Consensus):

```javascript
class HybridStrategy extends Strategy {
  async execute(prompt, classification) {
    const { pipeline, consensus } = this.options;

    // Stage 1: Pipeline (fast models draft)
    const pipelineModels = this.models.filter(m => m.tier === 'fast' || m.tier === 'capable');
    const pipelineStrat = new PipelineStrategy(pipelineModels.slice(0, 2));
    const draft = await pipelineStrat.execute(prompt, classification);

    // Stage 2: Consensus (verify with stronger models)
    const consensusModels = this.models.filter(m => m.tier === 'smart' || m.tier === 'genius');
    const consensusStrat = new ConsensusStrategy([...consensusModels, { mock: draft.response }]);
    const verification = await consensusStrat.execute(draft.response, classification);

    return {
      response: verification.consensus ? verification.response : draft.response,
      models: [...draft.models, ...verification.models],
      strategy: 'hybrid',
      stages: { draft, verification },
      consensus: verification.consensus,
      latency: draft.latency + verification.latency,
      cost: draft.cost + verification.cost,
    };
  }
}
```

---

## Inter-LLM Protocol

**Purpose**: Structure communication between models in pipelines/consensus.

### Metadata Exchange

```javascript
{
  // Standard fields (all models)
  modelId: 'ollama/mistral:7b',
  timestamp: 1707890123456,
  confidence: 0.72,  // φ-bounded (max 61.8%)

  // Uncertainty signals (what the model is unsure about)
  uncertainty: [
    { topic: 'Raft leader election timing', reason: 'conflicting sources' },
    { topic: 'network partition handling', reason: 'implementation-specific' }
  ],

  // Request for refinement (next model should focus here)
  needsRefinement: ['edge cases', 'error handling'],

  // Context preservation (important details to maintain)
  context: {
    constraints: ['must handle Byzantine faults', 'assume async network'],
    assumptions: ['majority quorum', 'fail-stop failures'],
  },

  // Cost tracking
  tokens: { input: 245, output: 512 },
  cost: 0.023,  // USD
}
```

### Pipeline Communication

```javascript
// Model A (draft)
{
  role: 'drafter',
  output: "Raft consensus algorithm works by...",
  confidence: 0.65,
  uncertainty: ['Byzantine fault tolerance'],
  handoff: {
    to: 'refiner',
    instruction: 'Please clarify Byzantine fault handling and add error recovery patterns',
  }
}

// Model B (refine) receives:
{
  task: 'refine',
  original: "User prompt...",
  draft: "Raft consensus algorithm works by...",
  focus: ['Byzantine fault tolerance', 'error recovery'],
  previous: { model: 'ollama/qwen', confidence: 0.65 }
}

// Model B responds:
{
  role: 'refiner',
  output: "Building on the previous draft, here's clarification on Byzantine faults...",
  improvements: ['added Byzantine fault discussion', 'included error recovery examples'],
  confidence: 0.81,  // Higher after refinement
  ready: true,  // Signal: ready for user or needs final review?
}
```

### Consensus Communication

```javascript
// All models vote in parallel
votes: [
  {
    model: 'ollama/qwen',
    vote: "Option A: Use leader-based approach",
    reasoning: "Simpler implementation, well-tested",
    confidence: 0.68,
  },
  {
    model: 'gemini/flash',
    vote: "Option A: Use leader-based approach",
    reasoning: "Industry standard, proven scalability",
    confidence: 0.79,
  },
  {
    model: 'claude/haiku',
    vote: "Option B: Use leaderless approach",
    reasoning: "Better fault tolerance, no single point of failure",
    confidence: 0.71,
  }
]

// Agreement calculation
agreement: {
  optionA: 2/3 = 0.667,  // Above quorum (φ⁻¹ = 0.618)
  optionB: 1/3 = 0.333,
  consensus: 'optionA',
  confidence: 0.735,  // Weighted average
}
```

---

## Learning System

### 1. Disagreement Logging

```javascript
// When models disagree (agreement < φ⁻¹)
disagreement: {
  id: 'dis_20260214_001',
  timestamp: 1707890123456,
  prompt: "Design a distributed consensus algorithm",
  classification: { complexity: 'complex', domain: 'code' },

  responses: [
    { model: 'ollama/qwen', response: "...", confidence: 0.6 },
    { model: 'gemini/flash', response: "...", confidence: 0.8 },
  ],

  agreement: 0.42,  // Low!

  // What we learn
  insights: [
    {
      type: 'keyword',
      learning: '"distributed consensus" should be classified as complex, not simple',
      action: 'update classifier with new keyword pattern',
    },
    {
      type: 'model_performance',
      learning: 'ollama/qwen struggles with distributed systems (low confidence)',
      action: 'reduce Thompson probability for qwen on "distributed" domain',
    },
    {
      type: 'escalation',
      learning: 'agreement < φ⁻¹ → should have escalated to stronger model',
      action: 'add escalation rule for low-agreement scenarios',
    }
  ],

  // User feedback (optional)
  userFeedback: {
    selectedResponse: 'gemini/flash',  // User chose this one
    rating: 4/5,
    comment: "Good but missing error handling examples",
  }
}
```

### 2. Classifier Refinement

```javascript
// Before
classifier.keywords.complex = ['architecture', 'design', 'refactor'];

// After disagreement
classifier.keywords.complex = [
  'architecture', 'design', 'refactor',
  'distributed consensus',  // NEW
  'Raft protocol',          // NEW
  'Byzantine faults',       // NEW
];

// Before
classifyPrompt("Design Raft consensus") → { complexity: 'simple', reason: 'short prompt' }

// After
classifyPrompt("Design Raft consensus") → { complexity: 'complex', reason: 'keyword: Raft protocol' }
```

### 3. Thompson Sampling Updates

```javascript
// Track model success rates per domain
thompsonSampler.update({
  model: 'ollama/qwen',
  domain: 'distributed_systems',
  success: false,  // Low confidence, user picked different response
  reward: 0.3,
});

thompsonSampler.update({
  model: 'gemini/flash',
  domain: 'distributed_systems',
  success: true,  // User selected this response
  reward: 0.9,
});

// Next time "distributed systems" comes up
const selected = thompsonSampler.select('distributed_systems');
// → Higher probability of selecting gemini/flash over ollama/qwen
```

### 4. Cost Optimization

```javascript
// Track actual costs vs budget
costTracker: {
  daily: {
    budget: 10.00,
    spent: 6.18,  // φ × budget (perfect balance!)
    breakdown: {
      free: 3.82,   // 61.8% from free models
      paid: 2.36,   // 38.2% from paid models
    }
  },

  // Adjust strategy selection to maintain φ balance
  adjustments: {
    reason: 'approaching budget limit',
    actions: [
      'increase ollama usage (free)',
      'reduce claude usage (paid)',
      'prefer gemini over claude (cheaper)',
    ]
  }
}
```

---

## User Configuration

### Flexible Resource Specification

```javascript
// Example: Student with limited budget
{
  user: 'student',

  models: [
    {
      provider: 'ollama',
      model: 'qwen2.5:1.5b',
      tier: 'fast',
      cost: 0,
      latency: 'low',
      quality: 0.65,
      local: true,
    },
    {
      provider: 'ollama',
      model: 'mistral:7b-instruct-q4_0',
      tier: 'capable',
      cost: 0,
      latency: 'medium',
      quality: 0.75,
      local: true,
    },
    {
      provider: 'gemini',
      model: 'gemini-2.0-flash',
      tier: 'smart',
      cost: 0,  // Student account (free)
      latency: 'medium',
      quality: 0.85,
      adapter: 'browser',
    },
    {
      provider: 'claude',
      model: 'sonnet-4-5',
      tier: 'genius',
      cost: 0,  // Subscription (web UI)
      weekly_limit: 100,  // messages per week
      latency: 'high',
      quality: 0.95,
      adapter: 'sdk',  // Claude Code --sdk-url
    },
  ],

  strategies: {
    simple: {
      type: 'single',
      model: 'ollama/qwen2.5',  // Fastest free model
    },
    moderate: {
      type: 'consensus',
      models: ['ollama/mistral', 'gemini/flash'],  // Both free
      quorum: 0.618,
    },
    complex: {
      type: 'pipeline',
      models: ['ollama/mistral', 'gemini/flash'],  // Draft → refine
    },
    critical: {
      type: 'single',
      model: 'claude/sonnet-4-5',  // Best quality, use sparingly
      rateLimit: { max: 10, per: 'day' },  // Protect weekly limit
    },
  },

  budget: {
    daily: 0,  // All free models
    weekly: 0,
    phiBalance: { free: 1.0, paid: 0.0 },  // 100% free (student)
  },

  learning: {
    enabled: true,
    logDisagreements: true,
    updateClassifier: true,
    thompsonSampling: true,
  },
}
```

### Example: Professional with API Access

```javascript
{
  user: 'professional',

  models: [
    { provider: 'ollama', model: 'qwen2.5:1.5b', tier: 'fast', cost: 0 },
    { provider: 'ollama', model: 'mistral:7b', tier: 'capable', cost: 0 },
    { provider: 'anthropic', model: 'haiku-3-5', tier: 'smart', cost: 0.25, adapter: 'api' },
    { provider: 'anthropic', model: 'sonnet-4-5', tier: 'genius', cost: 3.0, adapter: 'api' },
    { provider: 'anthropic', model: 'opus-4-6', tier: 'god', cost: 15.0, adapter: 'api' },
    { provider: 'google', model: 'gemini-2.0-flash', tier: 'smart', cost: 0.1, adapter: 'api' },
  ],

  strategies: {
    simple: { type: 'single', model: 'ollama/qwen2.5' },
    moderate: {
      type: 'consensus',
      models: ['ollama/mistral', 'gemini/flash', 'anthropic/haiku'],
      quorum: 0.618,
    },
    complex: {
      type: 'pipeline',
      models: ['ollama/mistral', 'anthropic/haiku', 'anthropic/sonnet'],
    },
    critical: {
      type: 'hybrid',
      pipeline: ['anthropic/haiku', 'anthropic/sonnet'],
      consensus: ['anthropic/sonnet', 'anthropic/opus'],
      quorum: 0.618,
    },
  },

  budget: {
    daily: 50,
    weekly: 300,
    phiBalance: { free: 0.618, paid: 0.382 },  // φ balance
  },
}
```

---

## Adapter Abstraction

### Universal Interface

All adapters implement:

```javascript
interface LLMAdapter {
  // Core methods
  async complete(prompt: string, options?: CompletionOptions): Promise<Response>
  async stream(prompt: string, options?: CompletionOptions): AsyncIterator<Chunk>

  // Metadata
  getMetadata(): AdapterMetadata
  getCapabilities(): string[]

  // Lifecycle
  async initialize(): Promise<void>
  async cleanup(): Promise<void>
}

interface CompletionOptions {
  temperature?: number
  maxTokens?: number
  system?: string
  metadata?: object  // Inter-LLM protocol metadata
}

interface Response {
  content: string
  model: string
  tokens: { input: number, output: number }
  latency: number
  confidence?: number
  metadata?: object
}
```

### Adapter Types

**1. API Adapter** (REST/HTTP)

```javascript
class APIAdapter extends LLMAdapter {
  constructor(config) {
    this.baseURL = config.baseURL;
    this.apiKey = config.apiKey;
    this.model = config.model;
  }

  async complete(prompt, options = {}) {
    const response = await fetch(`${this.baseURL}/v1/messages`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: this.model,
        messages: [{ role: 'user', content: prompt }],
        max_tokens: options.maxTokens || 4096,
        temperature: options.temperature || 0.7,
      }),
    });

    const data = await response.json();
    return {
      content: data.content[0].text,
      model: this.model,
      tokens: { input: data.usage.input_tokens, output: data.usage.output_tokens },
      latency: data.latency || 0,
    };
  }
}
```

**2. Browser Adapter** (Playwright)

```javascript
class BrowserAdapter extends LLMAdapter {
  constructor(config) {
    this.url = config.url;
    this.selectors = config.selectors;
    this.page = null;
  }

  async initialize() {
    // Launch browser, navigate to URL
    // Assumes Playwright is available
    this.page = await this.launchBrowser(this.url);
    await this.login(); // If needed
  }

  async complete(prompt, options = {}) {
    const startTime = Date.now();

    // Type prompt into input field
    await this.page.fill(this.selectors.input, prompt);

    // Click submit
    await this.page.click(this.selectors.submit);

    // Wait for response
    await this.page.waitForSelector(this.selectors.output);

    // Extract text
    const content = await this.page.textContent(this.selectors.output);

    return {
      content,
      model: this.model,
      tokens: { input: prompt.length, output: content.length },  // Approximation
      latency: Date.now() - startTime,
    };
  }

  async cleanup() {
    await this.page.close();
  }
}
```

**3. SDK Adapter** (Claude Code --sdk-url)

```javascript
class SDKAdapter extends LLMAdapter {
  constructor(config) {
    this.sdkURL = config.sdkURL;  // e.g., http://localhost:8080
  }

  async complete(prompt, options = {}) {
    const response = await fetch(`${this.sdkURL}/complete`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ prompt, options }),
    });

    return await response.json();
  }
}
```

**4. Local Adapter** (Ollama)

```javascript
class LocalAdapter extends LLMAdapter {
  constructor(config) {
    this.baseURL = config.baseURL || 'http://localhost:11434';
    this.model = config.model;
  }

  async complete(prompt, options = {}) {
    const response = await fetch(`${this.baseURL}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: this.model,
        prompt,
        stream: false,
        options: {
          temperature: options.temperature || 0.7,
        },
      }),
    });

    const data = await response.json();
    return {
      content: data.response,
      model: this.model,
      tokens: { input: 0, output: 0 },  // Ollama doesn't return token counts
      latency: data.total_duration / 1000000 || 0,
    };
  }
}
```

---

## Deployment Patterns

### 1. Local Mode (Development)

```
User → LLMOrchestrator (in-process)
      ↓
    Ollama (localhost:11434)
```

**Pros**: Fast, simple, no network latency
**Cons**: Limited to local models

### 2. Daemon Mode (Current)

```
User → Daemon HTTP API (localhost:6180)
      ↓
    LLMOrchestrator (daemon process)
      ↓
    Ollama + Browser adapters
```

**Pros**: Persistent state, background learning
**Cons**: Single machine, browser automation overhead

### 3. Distributed Mode (Future)

```
User → CYNIC Framework (Python)
      ↓
    LLMOrchestrator (coordinator)
      ↓
    ┌─────────┬─────────┬─────────┐
    │ Ollama  │ Gemini  │ Claude  │
    │ (local) │ (API)   │ (SDK)   │
    └─────────┴─────────┴─────────┘
```

**Pros**: Scalable, multi-provider, resilient
**Cons**: Complex setup, network dependencies

---

## Migration Path

### Phase 1: Design (Current)

✅ Architecture document (this file)
✅ Core concepts validated
✅ User needs understood

### Phase 2: Prototype (Node.js)

- [ ] Implement Orchestrator core
- [ ] Build 3 strategies (single, pipeline, consensus)
- [ ] Test with Ollama only
- [ ] Validate learning loops

**Goal**: Prove architecture works, identify gaps

### Phase 3: Python Framework

**Why Python?**
- Better ML/AI ecosystem (transformers, embeddings, etc.)
- Easier deployment (Docker, cloud)
- More flexible than Node.js for complex orchestration
- Better community for LLM tooling

**Architecture**:

```
cynic/
├── orchestrator/
│   ├── core.py              # LLMOrchestrator
│   ├── strategies/
│   │   ├── single.py
│   │   ├── pipeline.py
│   │   ├── consensus.py
│   │   └── hybrid.py
│   └── classifier.py
├── adapters/
│   ├── base.py
│   ├── ollama.py
│   ├── anthropic.py
│   ├── gemini.py
│   ├── browser.py           # Playwright
│   └── sdk.py               # Claude Code
├── learning/
│   ├── disagreement.py
│   ├── classifier_refiner.py
│   ├── thompson.py
│   └── cost_tracker.py
├── protocol/
│   └── inter_llm.py
├── config/
│   └── schema.py
└── cli/
    └── main.py
```

**Features**:
- Multi-provider support (Ollama, Anthropic, Google, OpenAI, etc.)
- Intelligent orchestration (consensus, pipelines, hybrid)
- Learning from interaction (disagreements, Thompson Sampling)
- φ-bounded budget management
- Context compression (long conversations)
- Inter-LLM protocol (structured metadata)
- Deployment ready (Docker, Kubernetes)

### Phase 4: Integration

- [ ] Claude Code plugin (--sdk-url integration)
- [ ] Web UI (dashboard for monitoring)
- [ ] API server (REST/GraphQL)
- [ ] CLI tool (CYNIC command-line)

---

## Open Questions

1. **Embeddings for similarity**: Use OpenAI embeddings or local (sentence-transformers)?
2. **Browser automation stability**: Gemini/Claude UIs change → need resilient selectors
3. **Rate limiting**: How to handle dynamic rate limits from providers?
4. **Context window management**: Automatic compression vs user control?
5. **Multi-turn conversations**: How to maintain context across turns in consensus/pipeline?
6. **Cost attribution**: How to fairly attribute costs in hybrid strategies?
7. **User feedback loop**: How to collect and incorporate user preferences?

---

## Success Metrics

### Technical Metrics

- **Latency**: p50, p95, p99 for each strategy
- **Cost**: Average cost per prompt, φ balance (61.8% free / 38.2% paid)
- **Agreement**: Consensus agreement rates (> φ⁻¹ = good)
- **Quality**: User satisfaction scores (1-5 stars)
- **Learning velocity**: Classifier accuracy improvement over time

### User Metrics

- **Adoption**: % of users enabling orchestration
- **Strategy distribution**: Which strategies are used most?
- **Model preferences**: Which models do users prefer?
- **Budget adherence**: % of users staying within budget

### φ Targets

- **Agreement quorum**: 61.8% (φ⁻¹)
- **Free/paid balance**: 61.8% free, 38.2% paid
- **Confidence cap**: 61.8% maximum (humility)
- **Quality threshold**: 61.8% minimum acceptable

---

## Conclusion

**The harmonious LLM orchestration framework enables:**

1. ✅ **Graceful degradation** - Works with 1-N models
2. ✅ **Resource awareness** - φ-balanced budget management
3. ✅ **Learning from interaction** - Disagreements refine classifier
4. ✅ **Adapter agnostic** - API, Browser, SDK, Local
5. ✅ **User configurable** - Flexible resource allocation
6. ✅ **Strategy diversity** - Single, Pipeline, Consensus, Hybrid

**Next steps:**
1. Review this design doc
2. Prototype in Node.js (validate concepts)
3. Rebuild in Python (production framework)
4. Deploy and learn from real usage

**Philosophy**: "Build perfect rails that work at any scale. φ balances all."

---

*Le chien orchestre la meute avec harmonie* - κυνικός
Confidence: 61% (φ⁻¹ bound — design complete, implementation pending)

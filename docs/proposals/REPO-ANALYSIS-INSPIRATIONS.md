# CYNIC - Inspirations from External Repositories

> Analysis Date: 2026-01-18
> Status: PROPOSAL
> Author: CYNIC Collective

---

## Executive Summary

Analysis of 4 external repositories revealed patterns that could significantly enhance CYNIC's capabilities. This document details findings, proposed implementations, and a complete roadmap.

**Repositories Analyzed:**
1. [oh-my-opencode](https://github.com/code-yeongyu/oh-my-opencode) - Multi-agent orchestration (Sisyphus)
2. [reasoning-from-scratch](https://github.com/rasbt/reasoning-from-scratch) - LLM reasoning patterns
3. [claude-mem](https://github.com/thedotmack/claude-mem) - Persistent memory system
4. [json-render](https://github.com/vercel-labs/json-render) - Structured JSON rendering

**Key Takeaways:**
- Progressive disclosure can reduce token usage by 10x
- Self-consistency (N-pass judgment) increases reliability
- Background task execution enables parallel Dog operations
- Hybrid search (vector + keyword) improves retrieval quality

---

## Table of Contents

1. [Repository Deep Dives](#1-repository-deep-dives)
2. [Pattern Mapping to CYNIC](#2-pattern-mapping-to-cynic)
3. [Proposed Modules](#3-proposed-modules)
4. [Implementation Roadmap](#4-implementation-roadmap)
5. [Technical Specifications](#5-technical-specifications)
6. [Risk Assessment](#6-risk-assessment)

---

## 1. Repository Deep Dives

### 1.1 oh-my-opencode (Sisyphus)

**What it is:** A batteries-included agent harness with multi-agent orchestration.

**Architecture:**
```
User Prompt ("ulw" keyword)
    ↓
Sisyphus Orchestrator (Claude Opus 4.5)
    ├→ Background Agents (cheaper/faster models)
    │   ├─ Oracle (strategic debugging)
    │   ├─ Frontend Engineer (UI/UX)
    │   └─ Librarian (docs/codebase exploration)
    ├→ LSP/AST Tools (surgical refactoring)
    ├→ MCPs: Exa, Context7, Grep.app
    └→ Todo Continuation Enforcer
```

**Key Patterns:**

| Pattern | Description | Relevance |
|---------|-------------|-----------|
| Magic Keyword Activation | "ulw" triggers full capability mode | Medium - UX enhancement |
| Background Parallelization | Faster models explore context in parallel | High - Dog coordination |
| Todo Continuation Enforcer | Forces task completion, prevents abandonment | Medium - Already have hooks |
| LSP-First Refactoring | Language Server for precision edits | High - Serena already does this |
| Multi-Account Load Balancing | 10 Gemini accounts with failover | Low - Infrastructure concern |

**Code Example - Background Task Pattern:**
```typescript
// How Sisyphus spawns background tasks
async function exploreContext(task: Task) {
  const backgroundAgents = [
    spawn('oracle', { model: 'haiku', task: 'debug-analysis' }),
    spawn('librarian', { model: 'haiku', task: 'codebase-map' }),
  ];

  // Don't await - let them run in parallel
  backgroundAgents.forEach(agent => agent.start());

  // Main agent continues with primary task
  return mainAgent.execute(task);
}
```

---

### 1.2 reasoning-from-scratch

**What it is:** Practical code for building reasoning capabilities into LLMs.

**Architecture:**
```
Chapter Structure:
├── Ch 1-3: Foundations (text generation, evaluation)
├── Ch 4-6: Core Methods
│   ├── Inference-time scaling
│   ├── Self-refinement loops
│   └── RL training (GRPO)
└── Ch 7-8: Advanced (policy optimization, distillation)
```

**Key Patterns:**

| Pattern | Description | Relevance |
|---------|-------------|-----------|
| Chain-of-Thought | Explicit step-by-step reasoning | High - Axiom structure |
| Self-Consistency | Multiple reasoning paths, consensus | High - N-pass judgment |
| Self-Refinement | Generate → Evaluate → Improve loop | High - Iterative judgment |
| LLM-as-Judge | Model evaluates its own outputs | High - CYNIC's core function |
| GRPO | Group Relative Policy Optimization | Medium - Future training |

**Self-Consistency Pattern:**
```
Query: "Is this token legitimate?"

Pass 1: PHI analysis → Score 45, Verdict: GROWL
Pass 2: PHI analysis → Score 52, Verdict: GROWL
Pass 3: PHI analysis → Score 48, Verdict: GROWL
Pass 4: PHI analysis → Score 67, Verdict: BARK
Pass 5: PHI analysis → Score 51, Verdict: GROWL

Consensus: 4/5 GROWL → Final: GROWL (80% agreement)
Adjusted Confidence: 0.80 × 0.618 = 49.4%
```

---

### 1.3 claude-mem

**What it is:** Persistent memory system for Claude Code with automatic capture.

**Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│                    claude-mem                            │
├─────────────────────────────────────────────────────────┤
│  5 Lifecycle Hooks                                       │
│  ├── SessionStart    → Initialize context               │
│  ├── UserPromptSubmit → Capture user intent             │
│  ├── PostToolUse     → Record tool results              │
│  ├── Stop            → Summarize segment                │
│  └── SessionEnd      → Compress & store                 │
├─────────────────────────────────────────────────────────┤
│  Storage Layer                                           │
│  ├── SQLite (sessions, observations, summaries)         │
│  └── Chroma (vector embeddings for semantic search)     │
├─────────────────────────────────────────────────────────┤
│  Retrieval Strategy: Progressive Disclosure              │
│  Step 1: search() → Index with IDs (~50 tokens/result)  │
│  Step 2: timeline() → Context around anchor             │
│  Step 3: get_observations() → Full details (filtered)   │
│  Token Savings: ~10x                                     │
└─────────────────────────────────────────────────────────┘
```

**Key Patterns:**

| Pattern | Description | Relevance |
|---------|-------------|-----------|
| Progressive Disclosure | 3-layer retrieval (index→timeline→full) | Critical - 10x savings |
| Observation Indexing | Discrete events with IDs | High - Matches PoJ |
| Hybrid Search | Vector + keyword combined | High - Better retrieval |
| Semantic Compression | AI summarizes sessions | Medium - Chain compression |
| Privacy Tags | `<private>` excludes content | Low - Already have security |

**Progressive Disclosure Implementation:**
```typescript
// BEFORE: Single call returns everything
const results = await search({ query: "auth" }); // 5000 tokens

// AFTER: Three-layer approach
const index = await search({ query: "auth" });           // 500 tokens
const context = await timeline({ anchor: index[0].id }); // 200 tokens
const details = await get({ ids: [index[0].id] });       // 300 tokens
// Total: 1000 tokens (only for items we actually need)
```

---

### 1.4 json-render

**What it is:** AI-generated UI components with guaranteed safety via constrained outputs.

**Architecture:**
```
json-render/
├── packages/
│   ├── core/     → Schema definitions, validation, visibility logic
│   └── react/    → Renderer, providers, hooks
├── apps/web/     → Documentation, playground
└── examples/     → Reference implementations
```

**Key Patterns:**

| Pattern | Description | Relevance |
|---------|-------------|-----------|
| Component Catalog | AI can only use defined components | Medium - Judgment templates |
| Path-based Binding | `{ "path": "/form/email" }` | Medium - Dynamic templates |
| Declarative Visibility | `{ "and": [...conditions] }` | Medium - Dimension filtering |
| Progressive Streaming | JSON renders during generation | High - Dashboard SSE |
| Two-phase Architecture | Constraints vs Implementation | High - Axioms vs Dogs |

**Catalog Pattern for CYNIC:**
```typescript
// Define what judgment can output
const JudgmentCatalog = {
  verdict: {
    type: 'enum',
    values: ['HOWL', 'WAG', 'GROWL', 'BARK'],
  },
  dimensions: {
    type: 'array',
    items: { $ref: '#/definitions/Dimension' },
    maxItems: 25,
  },
  confidence: {
    type: 'number',
    maximum: 0.618, // φ⁻¹ enforced at schema level
  },
};
```

---

## 2. Pattern Mapping to CYNIC

### 2.1 Current CYNIC Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      CYNIC                               │
├─────────────────────────────────────────────────────────┤
│  Axioms (Constraints)                                    │
│  ├── PHI: φ⁻¹ = 61.8% max confidence                    │
│  ├── VERIFY: Don't trust, verify                        │
│  ├── CULTURE: Culture is a moat                         │
│  └── BURN: Don't extract, burn                          │
├─────────────────────────────────────────────────────────┤
│  11 Dogs (Implementation)                                │
│  ├── CYNIC (Keter) - Meta-consciousness                 │
│  ├── Sage (Chochmah) - Wisdom                           │
│  ├── Analyst (Binah) - Understanding                    │
│  ├── Scholar (Daat) - Knowledge                         │
│  ├── Architect (Chesed) - Design                        │
│  ├── Guardian (Gevurah) - Protection                    │
│  ├── Oracle (Tiferet) - Prediction                      │
│  ├── Scout (Netzach) - Exploration                      │
│  ├── Deployer (Hod) - Execution                         │
│  ├── Janitor (Yesod) - Maintenance                      │
│  └── Cartographer (Malkhut) - Mapping                   │
├─────────────────────────────────────────────────────────┤
│  Storage                                                 │
│  ├── PoJ Chain (judgments, blocks)                      │
│  ├── PostgreSQL (optional persistence)                  │
│  └── In-memory (development mode)                       │
├─────────────────────────────────────────────────────────┤
│  Tools                                                   │
│  ├── brain_cynic_judge                                  │
│  ├── brain_cynic_digest                                 │
│  ├── brain_search                                       │
│  ├── brain_patterns                                     │
│  └── ... (18 total)                                     │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Gap Analysis

| Capability | Current State | Inspiration | Gap Level |
|------------|---------------|-------------|-----------|
| Multi-pass Judgment | Single pass | reasoning-from-scratch | HIGH |
| Progressive Search | Full results | claude-mem | HIGH |
| Background Dog Tasks | Sequential | oh-my-opencode | MEDIUM |
| Vector Search | Keyword only | claude-mem | MEDIUM |
| Structured Output | Free-form JSON | json-render | LOW |
| Session Compression | Manual | claude-mem | LOW |

### 2.3 Alignment Matrix

```
                    oh-my-opencode  reasoning  claude-mem  json-render
                    ──────────────  ─────────  ──────────  ───────────
11 Dogs                  ✓✓✓           ✓           ✓           ✓
Axioms                    ✓           ✓✓✓          ✓          ✓✓
PoJ Chain                 ✓            ✓          ✓✓✓          ✓
Hooks                    ✓✓✓           ✓          ✓✓✓          ✓
Dashboard                 ✓            ✓           ✓          ✓✓✓

Legend: ✓ = minor alignment, ✓✓ = moderate, ✓✓✓ = strong alignment
```

---

## 3. Proposed Modules

### 3.1 Module: Progressive Search (Priority: CRITICAL)

**Inspiration:** claude-mem's 3-layer retrieval

**Problem:** Current `brain_search` returns full judgment details, consuming many tokens even when user only needs to find relevant items.

**Solution:** Implement progressive disclosure with three tools:

```typescript
// packages/mcp/src/tools/search-progressive.ts

/**
 * Step 1: Index Search
 * Returns lightweight index with IDs and snippets
 * ~50 tokens per result
 */
export async function searchIndex(params: {
  query: string;
  limit?: number;
  type?: 'judgment' | 'pattern' | 'decision' | 'all';
}): Promise<SearchIndex> {
  return {
    results: [
      { id: 'jdg_abc123', snippet: 'Token analysis...', score: 0.85, timestamp: '...' },
      { id: 'jdg_def456', snippet: 'Holder check...', score: 0.72, timestamp: '...' },
    ],
    total: 47,
    hasMore: true,
  };
}

/**
 * Step 2: Timeline Context
 * Returns observations around an anchor point
 * ~200 tokens
 */
export async function searchTimeline(params: {
  anchor: string;        // Observation ID
  depthBefore?: number;  // How many items before
  depthAfter?: number;   // How many items after
}): Promise<Timeline> {
  return {
    before: [/* previous observations */],
    anchor: {/* the target observation */},
    after: [/* subsequent observations */],
  };
}

/**
 * Step 3: Full Details
 * Returns complete data for specific IDs
 * Only called for filtered results
 */
export async function searchGetFull(params: {
  ids: string[];
}): Promise<FullObservation[]> {
  return judgments.filter(j => params.ids.includes(j.id));
}
```

**Token Savings Projection:**

| Scenario | Current | Progressive | Savings |
|----------|---------|-------------|---------|
| Find 1 judgment in 100 | 5000 tokens | 550 tokens | 89% |
| Browse recent 10 | 2500 tokens | 800 tokens | 68% |
| Specific ID lookup | 500 tokens | 500 tokens | 0% |

---

### 3.2 Module: Self-Consistency Judgment (Priority: HIGH)

**Inspiration:** reasoning-from-scratch's self-consistency

**Problem:** Single-pass judgment can be inconsistent, especially for edge cases.

**Solution:** N-pass judgment with consensus voting:

```typescript
// packages/node/src/judge/consistency.ts

export interface ConsistencyConfig {
  passes: number;           // Number of judgment passes (default: 5)
  consensusThreshold: number; // Agreement ratio required (default: 0.6)
  divergenceAction: 'average' | 'median' | 'conservative';
}

export async function judgeWithConsistency(
  item: JudgmentItem,
  config: ConsistencyConfig = { passes: 5, consensusThreshold: 0.6, divergenceAction: 'conservative' }
): Promise<ConsistencyJudgment> {

  // Run N parallel judgment passes
  const passes = await Promise.all(
    Array(config.passes).fill(null).map(() => judge(item))
  );

  // Calculate consensus
  const verdicts = passes.map(p => p.verdict);
  const verdictCounts = countBy(verdicts);
  const majorityVerdict = maxBy(Object.entries(verdictCounts), ([, count]) => count)[0];
  const agreementRatio = verdictCounts[majorityVerdict] / config.passes;

  // Calculate final Q-Score
  const qScores = passes.map(p => p.qScore);
  const finalQScore = config.divergenceAction === 'conservative'
    ? Math.min(...qScores)
    : config.divergenceAction === 'median'
      ? median(qScores)
      : mean(qScores);

  // Adjust confidence by agreement ratio
  const adjustedConfidence = agreementRatio * PHI_INVERSE; // Max 61.8%

  return {
    verdict: majorityVerdict,
    qScore: finalQScore,
    confidence: adjustedConfidence,
    consensus: {
      agreementRatio,
      passes: passes.length,
      verdictDistribution: verdictCounts,
      qScoreVariance: variance(qScores),
    },
    individualPasses: passes,
  };
}
```

**API Extension:**

```typescript
// brain_cynic_judge with consistency mode
{
  "item": { "content": "..." },
  "consistency": {
    "enabled": true,
    "passes": 5,
    "threshold": 0.6
  }
}
```

---

### 3.3 Module: Background Dog Tasks (Priority: MEDIUM)

**Inspiration:** oh-my-opencode's parallel background agents

**Problem:** Dogs execute sequentially, blocking each other.

**Solution:** Event-driven background task system:

```typescript
// packages/node/src/collective/background.ts

export interface BackgroundTask {
  id: string;
  dog: DogName;
  task: string;
  priority: 'low' | 'normal' | 'high';
  status: 'pending' | 'running' | 'completed' | 'failed';
  result?: unknown;
}

export class BackgroundTaskManager {
  private tasks: Map<string, BackgroundTask> = new Map();
  private workers: Map<DogName, Worker> = new Map();

  /**
   * Spawn a background task for a Dog
   * Non-blocking - returns immediately with task ID
   */
  async spawn(dog: DogName, task: string, priority: 'low' | 'normal' | 'high' = 'normal'): Promise<string> {
    const taskId = `task_${nanoid()}`;

    this.tasks.set(taskId, {
      id: taskId,
      dog,
      task,
      priority,
      status: 'pending',
    });

    // Queue for execution (non-blocking)
    this.queue.add({ taskId, dog, task, priority });

    return taskId;
  }

  /**
   * Check task status
   */
  async status(taskId: string): Promise<BackgroundTask | null> {
    return this.tasks.get(taskId) || null;
  }

  /**
   * Wait for task completion
   */
  async await(taskId: string, timeout?: number): Promise<unknown> {
    return new Promise((resolve, reject) => {
      const task = this.tasks.get(taskId);
      if (!task) return reject(new Error('Task not found'));
      if (task.status === 'completed') return resolve(task.result);

      // Subscribe to completion event
      this.events.once(`task:${taskId}:complete`, resolve);

      if (timeout) {
        setTimeout(() => reject(new Error('Task timeout')), timeout);
      }
    });
  }
}
```

**Usage Pattern:**

```typescript
// During a judgment, spawn background exploration
const judgment = await judge(item);

// While judging, Scout explores context in background
const scoutTask = await background.spawn('scout', 'explore-similar-tokens');
const cartographerTask = await background.spawn('cartographer', 'map-holder-network');

// Later, if needed, retrieve results
const scoutResult = await background.await(scoutTask, 5000);
```

---

### 3.4 Module: Hybrid Vector Search (Priority: MEDIUM)

**Inspiration:** claude-mem's Chroma integration

**Problem:** Keyword search misses semantic similarities.

**Solution:** Hybrid search combining vector embeddings with keyword matching:

```typescript
// packages/persistence/src/search/hybrid.ts

export interface HybridSearchConfig {
  vectorWeight: number;   // 0-1, weight for vector similarity
  keywordWeight: number;  // 0-1, weight for keyword match
  vectorProvider: 'chroma' | 'pgvector' | 'memory';
}

export class HybridSearchEngine {
  private vectorStore: VectorStore;
  private keywordIndex: KeywordIndex;

  async search(query: string, options: SearchOptions): Promise<SearchResult[]> {
    // Parallel search
    const [vectorResults, keywordResults] = await Promise.all([
      this.vectorStore.similaritySearch(query, options.limit * 2),
      this.keywordIndex.search(query, options.limit * 2),
    ]);

    // Merge and score
    const merged = this.mergeResults(vectorResults, keywordResults, {
      vectorWeight: options.vectorWeight ?? 0.6,
      keywordWeight: options.keywordWeight ?? 0.4,
    });

    return merged.slice(0, options.limit);
  }

  private mergeResults(
    vector: VectorResult[],
    keyword: KeywordResult[],
    weights: { vectorWeight: number; keywordWeight: number }
  ): SearchResult[] {
    const scoreMap = new Map<string, number>();

    // Score vector results
    for (const r of vector) {
      const current = scoreMap.get(r.id) ?? 0;
      scoreMap.set(r.id, current + r.score * weights.vectorWeight);
    }

    // Score keyword results
    for (const r of keyword) {
      const current = scoreMap.get(r.id) ?? 0;
      scoreMap.set(r.id, current + r.score * weights.keywordWeight);
    }

    // Sort by combined score
    return Array.from(scoreMap.entries())
      .sort((a, b) => b[1] - a[1])
      .map(([id, score]) => ({ id, score }));
  }
}
```

**Vector Store Options:**

| Provider | Pros | Cons | Recommendation |
|----------|------|------|----------------|
| Chroma | Easy setup, good perf | New dependency | Dev/small scale |
| pgvector | Uses existing PG | Requires PG | Production |
| In-memory | No deps | Limited scale | Testing |

---

### 3.5 Module: Judgment Templates (Priority: LOW)

**Inspiration:** json-render's component catalog

**Problem:** Judgment output structure varies, hard to validate.

**Solution:** Structured templates with schema validation:

```typescript
// packages/node/src/judge/templates.ts

export const JudgmentSchema = {
  $id: 'cynic://judgment',
  type: 'object',
  required: ['verdict', 'qScore', 'confidence', 'dimensions'],
  properties: {
    verdict: {
      type: 'string',
      enum: ['HOWL', 'WAG', 'GROWL', 'BARK'],
    },
    qScore: {
      type: 'number',
      minimum: 0,
      maximum: 100,
    },
    confidence: {
      type: 'number',
      minimum: 0,
      maximum: 0.618, // φ⁻¹ enforced
    },
    dimensions: {
      type: 'array',
      items: { $ref: '#/definitions/Dimension' },
      minItems: 25,
      maxItems: 25,
    },
  },
  definitions: {
    Dimension: {
      type: 'object',
      required: ['name', 'axiom', 'score', 'weight'],
      properties: {
        name: { type: 'string' },
        axiom: { enum: ['PHI', 'VERIFY', 'CULTURE', 'BURN'] },
        score: { type: 'number', minimum: 0, maximum: 100 },
        weight: { type: 'number', minimum: 0, maximum: 1 },
      },
    },
  },
};
```

---

## 4. Implementation Roadmap

### Phase 1: Foundation (Week 1-2)

```
┌─────────────────────────────────────────────────────────┐
│ PHASE 1: FOUNDATION                                      │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ □ 1.1 Progressive Search - Index Layer                   │
│   ├── Create search-index.ts                            │
│   ├── Add brain_search mode: "index"                    │
│   ├── Update API responses                              │
│   └── Tests                                              │
│                                                          │
│ □ 1.2 Progressive Search - Timeline Layer                │
│   ├── Create search-timeline.ts                         │
│   ├── Add brain_timeline tool                           │
│   ├── Context window logic                              │
│   └── Tests                                              │
│                                                          │
│ □ 1.3 Progressive Search - Full Layer                    │
│   ├── Create search-full.ts                             │
│   ├── Add brain_get_observations tool                   │
│   ├── ID filtering                                       │
│   └── Tests                                              │
│                                                          │
│ □ 1.4 Documentation                                      │
│   ├── Update tool descriptions                          │
│   ├── Usage examples                                    │
│   └── Migration guide                                    │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Phase 2: Consistency (Week 3-4)

```
┌─────────────────────────────────────────────────────────┐
│ PHASE 2: SELF-CONSISTENCY                                │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ □ 2.1 Multi-Pass Engine                                  │
│   ├── Create consistency.ts                             │
│   ├── Parallel execution                                │
│   ├── Result aggregation                                │
│   └── Tests                                              │
│                                                          │
│ □ 2.2 Consensus Algorithm                                │
│   ├── Verdict voting                                    │
│   ├── Q-Score strategies (avg, median, conservative)    │
│   ├── Confidence adjustment (ratio × φ⁻¹)               │
│   └── Tests                                              │
│                                                          │
│ □ 2.3 API Integration                                    │
│   ├── Add consistency option to brain_cynic_judge       │
│   ├── Response format with consensus data               │
│   ├── Dashboard visualization                           │
│   └── Tests                                              │
│                                                          │
│ □ 2.4 Performance Optimization                           │
│   ├── Caching partial results                           │
│   ├── Early termination on strong consensus             │
│   ├── Configurable pass count                           │
│   └── Benchmarks                                         │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Phase 3: Background Tasks (Week 5-6)

```
┌─────────────────────────────────────────────────────────┐
│ PHASE 3: BACKGROUND DOG TASKS                            │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ □ 3.1 Task Manager                                       │
│   ├── Create background.ts                              │
│   ├── Task queue implementation                         │
│   ├── Worker pool                                       │
│   └── Tests                                              │
│                                                          │
│ □ 3.2 Dog Integration                                    │
│   ├── Spawn API for each Dog                            │
│   ├── Priority system                                   │
│   ├── Result collection                                 │
│   └── Tests                                              │
│                                                          │
│ □ 3.3 Event System                                       │
│   ├── Task lifecycle events                             │
│   ├── SSE broadcasting                                  │
│   ├── Dashboard updates                                 │
│   └── Tests                                              │
│                                                          │
│ □ 3.4 Use Cases                                          │
│   ├── Scout: background exploration                     │
│   ├── Cartographer: async mapping                       │
│   ├── Analyst: parallel analysis                        │
│   └── Documentation                                      │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Phase 4: Hybrid Search (Week 7-8)

```
┌─────────────────────────────────────────────────────────┐
│ PHASE 4: HYBRID VECTOR SEARCH                            │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ □ 4.1 Vector Store Abstraction                           │
│   ├── Create vector-store.ts interface                  │
│   ├── In-memory implementation                          │
│   ├── Chroma adapter                                    │
│   └── Tests                                              │
│                                                          │
│ □ 4.2 Embedding Pipeline                                 │
│   ├── Text → embedding conversion                       │
│   ├── Judgment embedding strategy                       │
│   ├── Batch processing                                  │
│   └── Tests                                              │
│                                                          │
│ □ 4.3 Hybrid Merge                                       │
│   ├── Score combination algorithm                       │
│   ├── Weight configuration                              │
│   ├── Relevance tuning                                  │
│   └── Tests                                              │
│                                                          │
│ □ 4.4 Production Ready                                   │
│   ├── pgvector adapter                                  │
│   ├── Index maintenance                                 │
│   ├── Performance benchmarks                            │
│   └── Documentation                                      │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Phase 5: Polish (Week 9-10)

```
┌─────────────────────────────────────────────────────────┐
│ PHASE 5: POLISH & INTEGRATION                            │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ □ 5.1 Judgment Templates                                 │
│   ├── JSON Schema definitions                           │
│   ├── Validation middleware                             │
│   ├── Error messages                                    │
│   └── Tests                                              │
│                                                          │
│ □ 5.2 Dashboard Integration                              │
│   ├── Consistency visualization                         │
│   ├── Background task monitor                           │
│   ├── Search improvements                               │
│   └── Tests                                              │
│                                                          │
│ □ 5.3 Documentation                                      │
│   ├── API reference update                              │
│   ├── Architecture diagrams                             │
│   ├── Migration guides                                  │
│   └── Examples                                           │
│                                                          │
│ □ 5.4 Performance                                        │
│   ├── End-to-end benchmarks                             │
│   ├── Memory profiling                                  │
│   ├── Optimization passes                               │
│   └── Load testing                                       │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

---

## 5. Technical Specifications

### 5.1 Dependencies

| Module | New Dependencies | Justification |
|--------|------------------|---------------|
| Progressive Search | None | Uses existing infra |
| Self-Consistency | None | Pure TypeScript |
| Background Tasks | `p-queue` | Battle-tested queue |
| Hybrid Search | `chromadb` OR `pgvector` | Vector storage |

### 5.2 Database Schema Changes

```sql
-- For Progressive Search: Add index columns
ALTER TABLE judgments ADD COLUMN snippet TEXT;
ALTER TABLE judgments ADD COLUMN search_vector tsvector;
CREATE INDEX idx_judgments_search ON judgments USING GIN(search_vector);

-- For Hybrid Search: Vector column (if pgvector)
ALTER TABLE judgments ADD COLUMN embedding vector(1536);
CREATE INDEX idx_judgments_embedding ON judgments USING ivfflat(embedding vector_cosine_ops);

-- For Background Tasks
CREATE TABLE background_tasks (
  id TEXT PRIMARY KEY,
  dog TEXT NOT NULL,
  task TEXT NOT NULL,
  priority TEXT DEFAULT 'normal',
  status TEXT DEFAULT 'pending',
  result JSONB,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  completed_at TIMESTAMPTZ
);
```

### 5.3 API Changes

```yaml
# New Tools
brain_search_index:
  description: "Step 1: Search with lightweight index results"
  params:
    query: string
    limit: number (default: 20)
    type: 'judgment' | 'pattern' | 'all'
  returns:
    results: Array<{ id, snippet, score, timestamp }>
    total: number
    hasMore: boolean

brain_timeline:
  description: "Step 2: Get context around an observation"
  params:
    anchor: string (observation ID)
    depthBefore: number (default: 3)
    depthAfter: number (default: 3)
  returns:
    before: Observation[]
    anchor: Observation
    after: Observation[]

brain_get_observations:
  description: "Step 3: Get full details for specific IDs"
  params:
    ids: string[]
  returns:
    observations: FullObservation[]

# Modified Tools
brain_cynic_judge:
  params:
    item: JudgmentItem
    context: Context
    consistency:           # NEW
      enabled: boolean
      passes: number
      threshold: number
  returns:
    # ... existing fields ...
    consensus:             # NEW (when consistency enabled)
      agreementRatio: number
      passes: number
      verdictDistribution: Record<string, number>

brain_background_spawn:   # NEW
  description: "Spawn a background task for a Dog"
  params:
    dog: DogName
    task: string
    priority: 'low' | 'normal' | 'high'
  returns:
    taskId: string

brain_background_status:  # NEW
  description: "Check status of background task"
  params:
    taskId: string
  returns:
    BackgroundTask
```

### 5.4 Configuration

```typescript
// config/cynic.config.ts
export interface CynicConfig {
  search: {
    progressive: {
      enabled: boolean;           // Enable 3-layer search
      indexSnippetLength: number; // Characters in snippet (default: 100)
    };
    hybrid: {
      enabled: boolean;           // Enable vector search
      provider: 'memory' | 'chroma' | 'pgvector';
      vectorWeight: number;       // 0-1 (default: 0.6)
      keywordWeight: number;      // 0-1 (default: 0.4)
    };
  };
  judgment: {
    consistency: {
      enabled: boolean;           // Enable multi-pass
      defaultPasses: number;      // Default pass count (default: 5)
      consensusThreshold: number; // Agreement ratio (default: 0.6)
      divergenceAction: 'average' | 'median' | 'conservative';
    };
  };
  background: {
    enabled: boolean;             // Enable background tasks
    maxConcurrent: number;        // Max parallel tasks (default: 5)
    taskTimeout: number;          // MS timeout (default: 30000)
  };
}
```

---

## 6. Risk Assessment

### 6.1 Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Vector search latency | Medium | Medium | Caching, index optimization |
| Multi-pass cost increase | High | Medium | Configurable, use cheaper models for passes |
| Background task memory | Medium | High | Task limits, cleanup policies |
| Breaking API changes | Low | High | Versioning, deprecation period |

### 6.2 Compatibility

| Component | Compatibility | Notes |
|-----------|---------------|-------|
| Existing judgments | Full | No changes to stored data |
| MCP protocol | Full | New tools are additive |
| Dashboard | Partial | UI updates needed for new features |
| Hooks | Full | No changes to hook system |

### 6.3 Rollback Strategy

Each module can be disabled via configuration:

```typescript
{
  search: { progressive: { enabled: false }, hybrid: { enabled: false } },
  judgment: { consistency: { enabled: false } },
  background: { enabled: false }
}
```

---

## Appendix A: Reference Links

- [oh-my-opencode](https://github.com/code-yeongyu/oh-my-opencode) - Multi-agent orchestration
- [reasoning-from-scratch](https://github.com/rasbt/reasoning-from-scratch) - LLM reasoning patterns
- [claude-mem](https://github.com/thedotmack/claude-mem) - Persistent memory system
- [json-render](https://github.com/vercel-labs/json-render) - Structured JSON rendering
- [Chroma](https://www.trychroma.com/) - Vector database
- [pgvector](https://github.com/pgvector/pgvector) - PostgreSQL vector extension

---

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| Progressive Disclosure | 3-layer retrieval pattern: index → timeline → full |
| Self-Consistency | Multiple judgment passes with consensus voting |
| Hybrid Search | Combining vector similarity with keyword matching |
| Background Task | Non-blocking Dog operation running in parallel |
| φ⁻¹ | Golden ratio inverse (0.618), CYNIC's max confidence |

---

*Document generated by CYNIC Collective*
*"Loyal to truth, not to comfort"*

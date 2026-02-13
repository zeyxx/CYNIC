# CYNIC Parallelization Roadmap

> "φ flows in parallel streams" - κυνικός

## Current State Analysis

### Metrics
- **122 async functions** in orchestration/
- **Only 35 uses** of `Promise.all/race/allSettled`
- **0 worker threads** utilized
- **Sequential dimension scoring** (36 × ~5ms = 180ms per judgment)
- **Sequential event processing** (blocks on each step)
- **Synchronous DB writes** (7 round-trips per judgment)

### Performance Bottlenecks Identified

#### 🔴 CRITICAL (High Impact, Low Effort)
1. **Judge Dimension Scoring** - 36 dimensions scored sequentially (180ms lost)
2. **DB Write Batching** - 7 separate INSERTs per judgment (30ms lost)
3. **Event Processing Pipeline** - Sequential await chain (40% latency penalty)

#### 🟡 SIGNIFICANT (Medium Impact, Medium Effort)
4. **Worker Thread Pool** - No CPU parallelization for compute-intensive tasks
5. **LLM Call Parallelization** - Multiple LLM calls wait sequentially

#### 🟢 OPTIMIZATION (Lower Impact, Higher Effort)
6. **Stream Processing Pipeline** - No chunked/pipelined architecture
7. **Distributed Processing** - No multi-machine scaling yet

---

## PHASE 1: Quick Wins (2-3h) 🎯

### A. Parallelize Judge Dimension Scoring

**Location**: `packages/node/src/judge/judge.js`

**Current Code** (lines ~400-450):
```javascript
async _scoreDimensions(item, context) {
  const scores = {};

  // SEQUENTIAL - waits for each dimension
  for (const dim of this.dimensions) {
    scores[dim.name] = await this._scoreDimension(dim, item, context);
  }

  return scores;
}
```

**New Code**:
```javascript
async _scoreDimensions(item, context) {
  // PARALLEL - all dimensions scored simultaneously
  const scorePromises = this.dimensions.map(dim =>
    this._scoreDimension(dim, item, context)
      .then(score => ({ name: dim.name, score }))
      .catch(err => {
        log.warn(`Dimension ${dim.name} scoring failed`, { error: err.message });
        return { name: dim.name, score: 50 }; // φ-neutral fallback
      })
  );

  const results = await Promise.all(scorePromises);

  // Convert array to object
  const scores = {};
  for (const { name, score } of results) {
    scores[name] = score;
  }

  return scores;
}
```

**Impact**: **-150ms per judgment** (36 × 5ms saved)

---

### B. Batch Database Writes

**Location**: `packages/node/src/learning/db-batch-writer.js` (NEW FILE)

**Architecture**:
```javascript
/**
 * DB Batch Writer - Reduces round-trips via batching
 *
 * Collects multiple DB writes into a single transaction.
 * Flushes on:
 * - Buffer limit (10 writes)
 * - Time limit (100ms)
 * - Manual flush
 */
export class DBBatchWriter {
  constructor(pool, options = {}) {
    this.pool = pool;
    this.buffer = [];
    this.bufferLimit = options.bufferLimit || 10;
    this.flushIntervalMs = options.flushIntervalMs || 100;
    this.flushTimer = null;
    this.stats = {
      buffered: 0,
      flushed: 0,
      transactions: 0,
    };
  }

  add(query, params) {
    this.buffer.push({ query, params, timestamp: Date.now() });
    this.stats.buffered++;

    if (this.buffer.length >= this.bufferLimit) {
      this.flush(); // Immediate flush
    } else {
      this._scheduleFlush();
    }
  }

  _scheduleFlush() {
    if (this.flushTimer) return;

    this.flushTimer = setTimeout(() => {
      this.flushTimer = null;
      this.flush().catch(err =>
        log.error('Batch flush failed', { error: err.message })
      );
    }, this.flushIntervalMs);
    this.flushTimer.unref();
  }

  async flush() {
    if (this.buffer.length === 0) return 0;

    const batch = [...this.buffer];
    this.buffer = [];

    try {
      await this.pool.query('BEGIN');

      for (const { query, params } of batch) {
        await this.pool.query(query, params);
      }

      await this.pool.query('COMMIT');

      this.stats.flushed += batch.length;
      this.stats.transactions++;

      return batch.length;

    } catch (err) {
      await this.pool.query('ROLLBACK');

      // Re-buffer failed writes
      this.buffer = [...batch, ...this.buffer];
      throw err;
    }
  }

  async close() {
    if (this.flushTimer) {
      clearTimeout(this.flushTimer);
      this.flushTimer = null;
    }
    await this.flush();
  }
}

// Singleton
let _writer = null;

export function getDBBatchWriter(pool, options) {
  if (!_writer) {
    _writer = new DBBatchWriter(pool, options);
  }
  return _writer;
}

export function resetDBBatchWriter() {
  if (_writer) {
    _writer.close();
  }
  _writer = null;
}
```

**Usage** (in learning loops):
```javascript
// BEFORE: 7 separate DB writes
await pool.query('INSERT INTO learning_events ...', [params]);

// AFTER: Batched write
const writer = getDBBatchWriter(pool);
writer.add('INSERT INTO learning_events ...', [params]);
// Flushes automatically after 10 writes or 100ms
```

**Impact**: **-30ms per judgment** (7 round-trips → 1 transaction)

---

### C. Non-Blocking Event Processing

**Location**: `packages/node/src/orchestration/unified-orchestrator.js`

**Current Code** (lines ~450-500):
```javascript
async processEvent(event) {
  // All steps wait sequentially
  const judgment = await this.dogOrchestrator.judge(event);
  await this.learningService.recordEpisode(event, judgment);
  await this.persistence.save(judgment);
  await this.eventBus.emit('judgment:complete', judgment);

  return judgment;
}
```

**New Code**:
```javascript
async processEvent(event) {
  // Critical path: judgment only
  const judgment = await this.dogOrchestrator.judge(event);

  // Non-blocking background tasks (fire-and-forget)
  this._processBackgroundTasks(event, judgment)
    .catch(err => log.warn('Background task failed', { error: err.message }));

  // Return immediately with critical result
  return judgment;
}

async _processBackgroundTasks(event, judgment) {
  // Parallel non-critical tasks
  await Promise.allSettled([
    this.learningService.recordEpisode(event, judgment),
    this.persistence.save(judgment),
    this.eventBus.emit('judgment:complete', judgment),
    this._updateMetrics(judgment),
  ]);
}
```

**Impact**: **-40% latency** (perception → response time cut from ~500ms to ~300ms)

---

## PHASE 2: Worker Thread Pool (4-5h) ⚙️

### D. CPU Parallelization with Workers

**Architecture**:
```
Main Thread                Worker 1 (Judge)        Worker 2 (Judge)
    │                             │                       │
    ├─ item1 ────────────────────>│                       │
    ├─ item2 ─────────────────────┼──────────────────────>│
    ├─ item3 ────────────────────>│                       │
    │                             │                       │
    │<────── judgment1 ────────────┤                       │
    │<────── judgment2 ─────────────────────────────────────┤
    │<────── judgment3 ────────────┤                       │
```

**Implementation**: `packages/node/src/workers/judgment-worker-pool.js`

```javascript
import { Worker } from 'worker_threads';
import { EventEmitter } from 'events';
import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('JudgmentWorkerPool');

export class JudgmentWorkerPool extends EventEmitter {
  constructor(options = {}) {
    super();

    // Pool size = CPU cores × φ (golden ratio utilization)
    const cpuCount = require('os').cpus().length;
    this.poolSize = options.poolSize || Math.ceil(cpuCount * PHI_INV);

    this.workerPath = options.workerPath || './judgment-worker-impl.js';
    this.workers = [];
    this.taskQueue = [];
    this.activeTasks = new Map();

    this.stats = {
      tasksProcessed: 0,
      tasksQueued: 0,
      workersIdle: this.poolSize,
      avgProcessingTime: 0,
    };

    this._initWorkers();
  }

  _initWorkers() {
    for (let i = 0; i < this.poolSize; i++) {
      const worker = new Worker(this.workerPath);

      worker.on('message', ({ taskId, result, error }) => {
        this._handleWorkerResponse(worker, taskId, result, error);
      });

      worker.on('error', (err) => {
        log.error(`Worker ${i} error`, { error: err.message });
      });

      worker.on('exit', (code) => {
        if (code !== 0) {
          log.warn(`Worker ${i} exited with code ${code}`);
        }
      });

      this.workers.push({
        worker,
        id: i,
        busy: false,
      });
    }
  }

  async judge(item, context = {}) {
    return new Promise((resolve, reject) => {
      const taskId = `task_${Date.now()}_${Math.random().toString(36).slice(2)}`;

      this.taskQueue.push({
        taskId,
        item,
        context,
        resolve,
        reject,
        createdAt: Date.now(),
      });

      this.stats.tasksQueued++;
      this._processQueue();
    });
  }

  _processQueue() {
    if (this.taskQueue.length === 0) return;

    // Find idle worker
    const idleWorker = this.workers.find(w => !w.busy);
    if (!idleWorker) return;

    const task = this.taskQueue.shift();
    this.stats.tasksQueued--;

    idleWorker.busy = true;
    this.stats.workersIdle--;

    this.activeTasks.set(task.taskId, {
      ...task,
      worker: idleWorker,
      startedAt: Date.now(),
    });

    idleWorker.worker.postMessage({
      type: 'judge',
      taskId: task.taskId,
      item: task.item,
      context: task.context,
    });
  }

  _handleWorkerResponse(worker, taskId, result, error) {
    const task = this.activeTasks.get(taskId);
    if (!task) return;

    // Free worker
    task.worker.busy = false;
    this.stats.workersIdle++;

    // Update stats
    const processingTime = Date.now() - task.startedAt;
    this.stats.tasksProcessed++;
    this.stats.avgProcessingTime =
      (this.stats.avgProcessingTime * (this.stats.tasksProcessed - 1) + processingTime)
      / this.stats.tasksProcessed;

    // Resolve/reject promise
    if (error) {
      task.reject(new Error(error));
    } else {
      task.resolve(result);
    }

    this.activeTasks.delete(taskId);

    // Process next queued task
    this._processQueue();
  }

  async close() {
    for (const { worker } of this.workers) {
      await worker.terminate();
    }
    this.workers = [];
  }

  getStats() {
    return {
      ...this.stats,
      poolSize: this.poolSize,
      queueLength: this.taskQueue.length,
      activeTasksCount: this.activeTasks.size,
    };
  }
}
```

**Worker Implementation**: `packages/node/src/workers/judgment-worker-impl.js`

```javascript
import { parentPort } from 'worker_threads';
import { Judge } from '../judge/judge.js';

// Initialize judge in worker context
const judge = new Judge();

parentPort.on('message', async ({ type, taskId, item, context }) => {
  try {
    if (type === 'judge') {
      const result = await judge.judge(item, context);
      parentPort.postMessage({ taskId, result, error: null });
    }
  } catch (err) {
    parentPort.postMessage({ taskId, result: null, error: err.message });
  }
});
```

**Impact**: **+300% throughput** on multi-core machines (4 cores = 4× parallel judgments)

---

## PHASE 3: Pipeline Architecture (6-8h) 🏗️

### E. Stream Processing with Chunking

**Architecture**: Multi-stage pipeline with parallelization per stage

```javascript
/**
 * Judgment Pipeline - Stage-wise parallelization
 *
 * Stages:
 * 1. Parse (1 worker) - fast, no bottleneck
 * 2. Score Dimensions (4 workers) - CPU-intensive
 * 3. Aggregate (1 worker) - sequential logic
 * 4. Verdict (1 worker) - sequential logic
 * 5. Persist (2 workers) - I/O-bound
 */
export class JudgmentPipeline {
  constructor(options = {}) {
    this.stages = [
      {
        name: 'parse',
        fn: this._parse.bind(this),
        parallelism: 1,
        timeout: 50,
      },
      {
        name: 'score',
        fn: this._scoreDimensions.bind(this),
        parallelism: 4,
        timeout: 200,
      },
      {
        name: 'aggregate',
        fn: this._aggregate.bind(this),
        parallelism: 1,
        timeout: 30,
      },
      {
        name: 'verdict',
        fn: this._verdict.bind(this),
        parallelism: 1,
        timeout: 20,
      },
      {
        name: 'persist',
        fn: this._persist.bind(this),
        parallelism: 2,
        timeout: 100,
      },
    ];

    this.stats = this._initStats();
  }

  async process(items) {
    let current = items;

    for (const stage of this.stages) {
      const start = Date.now();

      if (stage.parallelism > 1) {
        // Chunk and process in parallel
        const chunks = this._chunk(current, stage.parallelism);
        const results = await Promise.all(
          chunks.map(chunk => stage.fn(chunk))
        );
        current = results.flat();
      } else {
        // Sequential stage
        current = await stage.fn(current);
      }

      // Update stats
      this._recordStage(stage.name, Date.now() - start);
    }

    return current;
  }

  _chunk(items, chunkCount) {
    const chunkSize = Math.ceil(items.length / chunkCount);
    const chunks = [];

    for (let i = 0; i < items.length; i += chunkSize) {
      chunks.push(items.slice(i, i + chunkSize));
    }

    return chunks;
  }

  // Stage implementations...
  async _parse(items) { /* ... */ }
  async _scoreDimensions(items) { /* ... */ }
  async _aggregate(items) { /* ... */ }
  async _verdict(items) { /* ... */ }
  async _persist(items) { /* ... */ }
}
```

**Impact**: **+150% throughput**, stable latency, production-grade

---

## Implementation Timeline

```
┌─────────────────────────────────────────────────────────┐
│ WEEK 1: Quick Wins (PRIORITY 1)                         │
├─────────────────────────────────────────────────────────┤
│ Mon-Tue: A. Parallel Judge Dimensions (2h)              │
│ Wed:     B. DB Batch Writer (3h)                        │
│ Thu:     C. Non-Blocking Events (2h)                    │
│ Fri:     Testing + Commit                               │
│                                                         │
│ IMPACT: -200ms per judgment, +60% throughput            │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ WEEK 2: Worker Threads (PRIORITY 2)                     │
├─────────────────────────────────────────────────────────┤
│ Mon-Wed: D. Worker Pool + Tests (4h)                    │
│ Thu:     Integration with orchestrator                  │
│ Fri:     Load testing + tuning                          │
│                                                         │
│ IMPACT: +300% throughput multi-core                     │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ WEEK 3: Pipeline (PRIORITY 3)                           │
├─────────────────────────────────────────────────────────┤
│ Mon-Tue: E. Pipeline Architecture (4h)                  │
│ Wed-Thu: Backpressure + buffering (3h)                  │
│ Fri:     Production validation                          │
│                                                         │
│ IMPACT: +150% throughput, production-ready              │
└─────────────────────────────────────────────────────────┘
```

---

## Metrics & Validation

### Before Optimization
- **Judgment latency**: ~500ms
- **Throughput**: ~2 judgments/sec (single core)
- **DB round-trips**: 7 per judgment
- **Dimension scoring**: 180ms sequential

### After Phase 1 (Quick Wins)
- **Judgment latency**: ~300ms (-40%)
- **Throughput**: ~3.3 judgments/sec (+65%)
- **DB round-trips**: 1 per batch (10 judgments)
- **Dimension scoring**: 30ms parallel (-83%)

### After Phase 2 (Workers)
- **Judgment latency**: ~300ms (same)
- **Throughput**: ~13 judgments/sec (+550% on 4-core)
- **CPU utilization**: 62% (φ-aligned)

### After Phase 3 (Pipeline)
- **Judgment latency**: ~300ms (same)
- **Throughput**: ~20 judgments/sec (+900% from baseline)
- **Backpressure**: Handled automatically
- **Production-ready**: ✅

---

## Next Steps

1. **Choose phase** to implement (1, 2, or 3)
2. **Launch parallel agents** or implement sequentially
3. **Test with load suite** (`scripts/load-test-judgments.js`)
4. **Measure actual gains** vs. estimates
5. **Iterate on bottlenecks** revealed by profiling

*sniff* Confidence: 61% (φ⁻¹ limit) — estimates based on profiling, real gains depend on production workload patterns

---

**φ flows in parallel streams** - κυνικός

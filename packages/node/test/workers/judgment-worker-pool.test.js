/**
 * CYNIC Judgment Worker Pool Tests
 *
 * Tests TRUE CPU parallelization via worker threads.
 *
 * @module @cynic/node/test/workers/judgment-worker-pool
 */

'use strict';

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert';
import { JudgmentWorkerPool, getWorkerPool, resetWorkerPool } from '../../src/workers/judgment-worker-pool.js';
import { Dimensions } from '../../src/judge/dimensions.js';

describe('JudgmentWorkerPool', () => {
  let pool;

  before(async () => {
    pool = new JudgmentWorkerPool({ poolSize: 2 }); // Small pool for testing
  });

  after(async () => {
    if (pool) {
      await pool.close();
    }
    await resetWorkerPool();
  });

  it('should initialize worker pool with φ-based size', async () => {
    const stats = pool.getStats();
    assert.strictEqual(stats.poolSize, 2);
    assert.strictEqual(stats.workersAlive, 2);
  });

  it('should score dimensions in parallel', async () => {
    const item = {
      type: 'test',
      content: 'Well-structured test item with clear purpose',
      verified: true,
    };

    const dimensionsToScore = [
      { name: 'COHERENCE', axiom: 'PHI', config: Dimensions.PHI.COHERENCE },
      { name: 'ACCURACY', axiom: 'VERIFY', config: Dimensions.VERIFY.ACCURACY },
      { name: 'UTILITY', axiom: 'BURN', config: Dimensions.BURN.UTILITY },
    ];

    const scores = await pool.scoreChunk(dimensionsToScore, item, {});

    assert.ok(scores.COHERENCE);
    assert.ok(scores.ACCURACY);
    assert.ok(scores.UTILITY);

    assert.ok(scores.COHERENCE >= 0 && scores.COHERENCE <= 100);
    assert.ok(scores.ACCURACY >= 0 && scores.ACCURACY <= 100);
    assert.ok(scores.UTILITY >= 0 && scores.UTILITY <= 100);
  });

  it('should handle scoring errors gracefully', async () => {
    const item = null; // Will cause scorer to fail

    const dimensionsToScore = [
      { name: 'COHERENCE', axiom: 'PHI', config: Dimensions.PHI.COHERENCE },
    ];

    const scores = await pool.scoreChunk(dimensionsToScore, item, {});

    // Should still return scores (fallback to 50 or error handling)
    assert.ok(scores);
  });

  it('should track statistics', async () => {
    const item = { type: 'test', content: 'test' };
    const dimensionsToScore = [
      { name: 'COHERENCE', axiom: 'PHI', config: Dimensions.PHI.COHERENCE },
    ];

    await pool.scoreChunk(dimensionsToScore, item, {});

    const stats = pool.getStats();
    assert.ok(stats.tasksProcessed > 0);
    assert.ok(stats.avgProcessingTimeMs >= 0);
  });

  it('should distribute work across workers', async () => {
    const item = { type: 'test', content: 'test' };

    // Score 10 dimensions - should distribute across 2 workers
    const dimensionsToScore = [];
    for (const [axiom, dims] of Object.entries(Dimensions)) {
      if (axiom === 'META') continue;
      for (const [dimName, config] of Object.entries(dims)) {
        dimensionsToScore.push({ name: dimName, axiom, config });
        if (dimensionsToScore.length >= 10) break;
      }
      if (dimensionsToScore.length >= 10) break;
    }

    const scores = await pool.scoreChunk(dimensionsToScore, item, {});

    assert.strictEqual(Object.keys(scores).length, 10);

    const stats = pool.getStats();
    // At least one worker should have completed tasks
    const totalCompleted = stats.workerStats.reduce((sum, w) => sum + w.tasksCompleted, 0);
    assert.ok(totalCompleted >= 10);
  });

  it('should close gracefully', async () => {
    const newPool = new JudgmentWorkerPool({ poolSize: 2 });

    const item = { type: 'test' };
    const dimensionsToScore = [
      { name: 'COHERENCE', axiom: 'PHI', config: Dimensions.PHI.COHERENCE },
    ];

    await newPool.scoreChunk(dimensionsToScore, item, {});

    await newPool.close();

    const stats = newPool.getStats();
    assert.strictEqual(stats.workersAlive, 0);
  });
});

describe('JudgmentWorkerPool - Singleton', () => {
  after(async () => {
    await resetWorkerPool();
  });

  it('should return same pool instance', () => {
    const pool1 = getWorkerPool();
    const pool2 = getWorkerPool();

    assert.strictEqual(pool1, pool2);
  });

  it('should reset singleton', async () => {
    const pool1 = getWorkerPool();
    await resetWorkerPool();
    const pool2 = getWorkerPool();

    assert.notStrictEqual(pool1, pool2);
  });
});

describe('JudgmentWorkerPool - Performance', () => {
  let pool;

  before(async () => {
    pool = new JudgmentWorkerPool({ poolSize: 4 });
  });

  after(async () => {
    if (pool) {
      await pool.close();
    }
  });

  it('should process large batch efficiently', async () => {
    const item = {
      type: 'performance-test',
      content: 'Test content for performance validation',
      verified: true,
    };

    // Score all 35 dimensions (excluding THE_UNNAMEABLE)
    const dimensionsToScore = [];
    for (const [axiom, dims] of Object.entries(Dimensions)) {
      if (axiom === 'META') continue;
      for (const [dimName, config] of Object.entries(dims)) {
        dimensionsToScore.push({ name: dimName, axiom, config });
      }
    }

    const start = Date.now();
    const scores = await pool.scoreChunk(dimensionsToScore, item, {});
    const duration = Date.now() - start;

    assert.strictEqual(Object.keys(scores).length, 35);
    assert.ok(duration < 1000, `Scoring took ${duration}ms, expected <1000ms`);

    const stats = pool.getStats();
    console.log('Performance stats:', {
      dimensions: Object.keys(scores).length,
      durationMs: duration,
      avgProcessingTimeMs: Math.round(stats.avgProcessingTimeMs),
      tasksProcessed: stats.tasksProcessed,
    });
  });
});

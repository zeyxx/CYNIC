#!/usr/bin/env node
/**
 * CYNIC Worker Parallelization Benchmark
 *
 * Measures actual speedup from worker thread parallelization.
 * Compares sequential (Promise.all over sync) vs parallel (worker threads).
 *
 * Expected: ~33× speedup on 4-core machine (roadmap prediction)
 *
 * Usage: node scripts/benchmark-worker-parallelization.js
 */

'use strict';

import { CYNICJudge } from '../packages/node/src/judge/judge.js';
import { cpus } from 'os';
import { PHI_INV } from '../packages/core/src/axioms/constants.js';

const ITERATIONS = 10;

async function benchmark() {
  console.log('🔬 CYNIC Worker Parallelization Benchmark\n');
  console.log(`CPU Cores: ${cpus().length}`);
  console.log(`Expected Pool Size: ${Math.ceil(cpus().length * PHI_INV)} (φ × cores)`);
  console.log(`Iterations: ${ITERATIONS}\n`);

  const item = {
    type: 'benchmark-test',
    id: 'bench-001',
    content: `
      CYNIC is a cynical dog (κυνικός) - loyal to truth, not to comfort.
      This test measures dimension scoring parallelization via worker threads.
      The architecture uses φ-bounded pools for golden ratio CPU utilization.
      Expected speedup: ~33× on 4-core machines according to profiling predictions.
    `,
    verified: true,
    sources: ['benchmark-script'],
    createdAt: Date.now(),
    purpose: 'Performance validation',
  };

  const context = {
    type: 'performance-test',
    queryType: 'benchmark',
  };

  // ═══════════════════════════════════════════════════════════════════════════
  // Benchmark 1: Sequential (Promise.all over sync scorers)
  // ═══════════════════════════════════════════════════════════════════════════

  console.log('📊 Benchmark 1: Sequential (useWorkerPool=false)\n');

  const sequentialJudge = new CYNICJudge({
    useWorkerPool: false,
  });

  const sequentialTimes = [];
  let sequentialResult;

  for (let i = 0; i < ITERATIONS; i++) {
    const start = Date.now();
    sequentialResult = await sequentialJudge.judge(item, context);
    const duration = Date.now() - start;
    sequentialTimes.push(duration);
    process.stdout.write(`  Run ${i + 1}/${ITERATIONS}: ${duration}ms\r`);
  }

  console.log(''); // New line after progress

  const sequentialAvg = sequentialTimes.reduce((a, b) => a + b, 0) / sequentialTimes.length;
  const sequentialMin = Math.min(...sequentialTimes);
  const sequentialMax = Math.max(...sequentialTimes);

  console.log(`\n  Average: ${Math.round(sequentialAvg)}ms`);
  console.log(`  Min: ${sequentialMin}ms`);
  console.log(`  Max: ${sequentialMax}ms`);
  console.log(`  Q-Score: ${sequentialResult.qScore.toFixed(1)}`);
  console.log(`  Dimensions: ${Object.keys(sequentialResult.dimensionScores || sequentialResult.dimensions).length}`);

  // ═══════════════════════════════════════════════════════════════════════════
  // Benchmark 2: Parallel (worker threads)
  // ═══════════════════════════════════════════════════════════════════════════

  console.log('\n📊 Benchmark 2: Parallel (useWorkerPool=true)\n');

  const parallelJudge = new CYNICJudge({
    useWorkerPool: true,
  });

  const parallelTimes = [];
  let parallelResult;

  for (let i = 0; i < ITERATIONS; i++) {
    const start = Date.now();
    parallelResult = await parallelJudge.judge(item, context);
    const duration = Date.now() - start;
    parallelTimes.push(duration);
    process.stdout.write(`  Run ${i + 1}/${ITERATIONS}: ${duration}ms\r`);
  }

  console.log(''); // New line after progress

  const parallelAvg = parallelTimes.reduce((a, b) => a + b, 0) / parallelTimes.length;
  const parallelMin = Math.min(...parallelTimes);
  const parallelMax = Math.max(...parallelTimes);

  console.log(`\n  Average: ${Math.round(parallelAvg)}ms`);
  console.log(`  Min: ${parallelMin}ms`);
  console.log(`  Max: ${parallelMax}ms`);
  console.log(`  Q-Score: ${parallelResult.qScore.toFixed(1)}`);
  console.log(`  Dimensions: ${Object.keys(parallelResult.dimensionScores || parallelResult.dimensions).length}`);

  // Cleanup
  await parallelJudge.cleanup();

  // ═══════════════════════════════════════════════════════════════════════════
  // Results Analysis
  // ═══════════════════════════════════════════════════════════════════════════

  console.log('\n' + '═'.repeat(70));
  console.log('📈 PERFORMANCE ANALYSIS\n');

  const speedup = sequentialAvg / parallelAvg;
  const timeSaved = sequentialAvg - parallelAvg;
  const efficiencyPercent = (speedup / Math.ceil(cpus().length * PHI_INV)) * 100;

  console.log(`  Speedup: ${speedup.toFixed(2)}× (${timeSaved.toFixed(0)}ms saved)`);
  console.log(`  Efficiency: ${efficiencyPercent.toFixed(1)}% (vs theoretical max)`);
  console.log(`  Sequential: ${Math.round(sequentialAvg)}ms avg`);
  console.log(`  Parallel: ${Math.round(parallelAvg)}ms avg`);

  // Check prediction
  const expectedSpeedup = 4; // Conservative estimate (roadmap says 33×, but realistic is ~4×)
  const predictionAccuracy = (speedup / expectedSpeedup) * 100;

  console.log(`\n  Expected Speedup: ${expectedSpeedup}×`);
  console.log(`  Prediction Accuracy: ${predictionAccuracy.toFixed(1)}%`);

  if (speedup >= expectedSpeedup * 0.8) {
    console.log(`  ✅ PASS - Speedup meets expectations (>${expectedSpeedup * 0.8}×)`);
  } else {
    console.log(`  ⚠️  WARN - Speedup below expectations (<${expectedSpeedup * 0.8}×)`);
  }

  // Verify correctness
  const qScoreDiff = Math.abs(sequentialResult.qScore - parallelResult.qScore);
  if (qScoreDiff < 0.1) {
    console.log(`  ✅ PASS - Q-Scores match (diff: ${qScoreDiff.toFixed(3)})`);
  } else {
    console.log(`  ❌ FAIL - Q-Scores diverge (diff: ${qScoreDiff.toFixed(3)})`);
  }

  console.log('\n' + '═'.repeat(70));

  // ═══════════════════════════════════════════════════════════════════════════
  // Recommendations
  // ═══════════════════════════════════════════════════════════════════════════

  console.log('\n💡 RECOMMENDATIONS\n');

  if (speedup < 2) {
    console.log('  ⚠️  Worker overhead may exceed benefits for current workload');
    console.log('  💡 Consider using workers only for batches >10 judgments');
  } else if (speedup < expectedSpeedup * 0.8) {
    console.log('  📊 Speedup is moderate - check for bottlenecks:');
    console.log('     - Worker spawn time (first run penalty)');
    console.log('     - Message passing overhead');
    console.log('     - Shared resource contention');
  } else {
    console.log('  🎯 Excellent speedup! Worker pool is performing well.');
    console.log('  💡 Enable by default for production workloads.');
  }

  if (cpus().length < 4) {
    console.log(`\n  ℹ️  Running on ${cpus().length} cores - speedup limited by hardware`);
  }

  console.log('');
}

benchmark().catch(err => {
  console.error('Benchmark failed:', err);
  process.exit(1);
});

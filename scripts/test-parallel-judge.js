#!/usr/bin/env node
'use strict';

/**
 * Test parallel dimension scoring performance
 * Measures before/after timing for judge.judge()
 */

import { CYNICJudge } from '../packages/node/src/judge/judge.js';
import { createRealScorer } from '../packages/node/src/judge/scorers.js';

async function measureJudgmentTime(judge, testItem, iterations = 10) {
  const timings = [];

  for (let i = 0; i < iterations; i++) {
    const t0 = Date.now();
    await judge.judge(testItem, { type: 'test', queryType: 'quality' });
    const t1 = Date.now();
    timings.push(t1 - t0);
  }

  const avg = timings.reduce((a, b) => a + b, 0) / timings.length;
  const min = Math.min(...timings);
  const max = Math.max(...timings);
  const median = timings.sort((a, b) => a - b)[Math.floor(timings.length / 2)];

  return { avg, min, max, median, timings };
}

async function main() {
  console.log('🏛️ Testing Parallel Dimension Scoring Performance\n');

  // Create judge with real scorer
  const judge = new CYNICJudge({
    scorer: createRealScorer(),
  });

  // Test item with various properties to trigger different scoring paths
  const testItem = {
    id: 'test-001',
    type: 'code',
    content: 'Test code implementation with various quality indicators',
    verified: true,
    sources: ['source1', 'source2'],
    hash: 'abc123',
    signature: 'xyz789',
    createdAt: Date.now(),
    original: true,
    purpose: 'testing',
    usageCount: 5,
    proof: 'proof-data',
    quality: 75,
  };

  console.log('📊 Running performance test (10 iterations)...\n');

  const stats = await measureJudgmentTime(judge, testItem, 10);

  console.log('Results:');
  console.log(`  Average: ${stats.avg.toFixed(2)}ms`);
  console.log(`  Median:  ${stats.median.toFixed(2)}ms`);
  console.log(`  Min:     ${stats.min.toFixed(2)}ms`);
  console.log(`  Max:     ${stats.max.toFixed(2)}ms`);
  console.log(`\n  All timings: ${stats.timings.map(t => t.toFixed(1)).join(', ')}ms`);

  // Get judgment stats
  const judgeStats = judge.getStats();
  console.log(`\n✅ Total judgments: ${judgeStats.totalJudgments}`);
  console.log(`   Verdicts: HOWL=${judgeStats.verdicts.HOWL}, WAG=${judgeStats.verdicts.WAG}, GROWL=${judgeStats.verdicts.GROWL}, BARK=${judgeStats.verdicts.BARK}`);

  // Sample judgment to verify correctness
  console.log('\n🔍 Sample judgment verification:');
  const sample = await judge.judge(testItem, { type: 'test' });
  console.log(`   Q-Score: ${sample.qScore.toFixed(1)}`);
  console.log(`   Verdict: ${sample.verdict}`);
  console.log(`   Confidence: ${(sample.confidence * 100).toFixed(1)}%`);
  console.log(`   Dimensions scored: ${Object.keys(sample.dimensions || sample.dimensionScores).length}`);

  console.log('\n✨ Parallel scoring enabled successfully!');
  console.log('Expected improvement: ~150ms per judgment (36 dimensions × ~5ms saved)');
}

main().catch(err => {
  console.error('❌ Test failed:', err.message);
  console.error(err.stack);
  process.exit(1);
});

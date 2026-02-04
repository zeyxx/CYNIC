#!/usr/bin/env node
/**
 * Run All Benchmarks
 *
 * Master script to run the complete CYNIC benchmarking pipeline.
 *
 * Usage:
 *   node run-all.js           # Run all benchmarks
 *   node run-all.js --l1      # L1 only
 *   node run-all.js --l2      # L2 only
 *   node run-all.js --dogs    # Per-dog benchmark
 *   node run-all.js --l3      # L3 learning benchmark
 *   node run-all.js --quick   # Quick mode (fewer samples)
 *
 * @module @cynic/benchmarks/run-all
 */

'use strict';

import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { mkdirSync, existsSync, writeFileSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));

async function main() {
  const args = process.argv.slice(2);

  const runL1 = args.includes('--l1') || args.includes('--all') || args.length === 0;
  const runL2 = args.includes('--l2') || args.includes('--all') || args.length === 0;
  const runDogs = args.includes('--dogs');
  const runL3 = args.includes('--l3');
  const quickMode = args.includes('--quick');

  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('  CYNIC BENCHMARK SUITE');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');
  console.log(`  Mode: ${quickMode ? 'Quick' : 'Full'}`);
  console.log(`  L1: ${runL1 ? '✓' : '✗'}  L2: ${runL2 ? '✓' : '✗'}  Dogs: ${runDogs ? '✓' : '✗'}  L3: ${runL3 ? '✓' : '✗'}`);
  console.log('');

  const results = {
    timestamp: new Date().toISOString(),
    mode: quickMode ? 'quick' : 'full',
    benchmarks: {},
  };

  // Ensure results directory
  const resultsDir = join(__dirname, 'results');
  if (!existsSync(resultsDir)) {
    mkdirSync(resultsDir, { recursive: true });
  }

  // Run L1 vs L2 Pipeline
  if (runL1 || runL2) {
    console.log('───────────────────────────────────────────────────────────────');
    console.log('  Running L1 vs L2 Pipeline...');
    console.log('───────────────────────────────────────────────────────────────');

    try {
      const { LLMBenchmarkPipeline } = await import('./llm-benchmark-pipeline.js');
      const pipeline = new LLMBenchmarkPipeline();
      await pipeline.initialize();

      if (runL1) {
        const l1Results = await pipeline.runL1Benchmark();
        results.benchmarks.l1 = l1Results.metrics;
      }

      if (runL2) {
        const l2Results = await pipeline.runL2Benchmark();
        results.benchmarks.l2 = l2Results.metrics;
      }

      if (runL1 && runL2) {
        const comparison = pipeline.compare();
        results.benchmarks.comparison = comparison;
      }

      pipeline.saveResults();
    } catch (e) {
      console.error(`  Pipeline error: ${e.message}`);
      results.benchmarks.pipeline_error = e.message;
    }
  }

  // Run Per-Dog Benchmark
  if (runDogs) {
    console.log('\n───────────────────────────────────────────────────────────────');
    console.log('  Running Per-Dog Benchmark...');
    console.log('───────────────────────────────────────────────────────────────');

    try {
      const { DogBenchmarkRunner } = await import('./dog-benchmark.js');
      const runner = new DogBenchmarkRunner();
      await runner.initialize();
      const summary = await runner.runAll();
      runner.saveResults();
      results.benchmarks.dogs = summary;
    } catch (e) {
      console.error(`  Dog benchmark error: ${e.message}`);
      results.benchmarks.dogs_error = e.message;
    }
  }

  // Run L3 Learning Benchmark
  if (runL3) {
    console.log('\n───────────────────────────────────────────────────────────────');
    console.log('  Running L3 Learning Benchmark...');
    console.log('───────────────────────────────────────────────────────────────');

    try {
      const { L3LearningBenchmark } = await import('./l3-learning-benchmark.js');
      const benchmark = new L3LearningBenchmark();
      const summary = await benchmark.run();
      benchmark.saveResults();
      await benchmark.cleanup();
      results.benchmarks.l3 = summary;
    } catch (e) {
      console.error(`  L3 benchmark error: ${e.message}`);
      results.benchmarks.l3_error = e.message;
    }
  }

  // Final Summary
  console.log('\n═══════════════════════════════════════════════════════════════');
  console.log('  FINAL SUMMARY');
  console.log('═══════════════════════════════════════════════════════════════\n');

  if (results.benchmarks.l1 && results.benchmarks.l2) {
    console.log('  L1 vs L2:');
    console.log(`    L1 F1: ${(results.benchmarks.l1.f1 * 100).toFixed(1)}%`);
    console.log(`    L2 F1: ${(results.benchmarks.l2.f1 * 100).toFixed(1)}%`);

    if (results.benchmarks.comparison) {
      const delta = results.benchmarks.comparison.f1Diff;
      console.log(`    Δ F1:  ${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`);
    }
  }

  if (results.benchmarks.dogs) {
    console.log('\n  Per-Dog Performance:');
    for (const [dog, metrics] of Object.entries(results.benchmarks.dogs)) {
      console.log(`    ${dog}: F1=${(metrics.f1 * 100).toFixed(1)}%`);
    }
  }

  if (results.benchmarks.l3) {
    console.log('\n  L3 Learning:');
    console.log(`    Patterns learned: ${results.benchmarks.l3.totalPatternsLearned}`);
    console.log(`    F1 improvement: ${(results.benchmarks.l3.f1Improvement * 100).toFixed(1)}%`);
  }

  // Save combined results
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const outputPath = join(resultsDir, `combined-${timestamp}.json`);
  writeFileSync(outputPath, JSON.stringify(results, null, 2));

  console.log(`\n  Combined results: ${outputPath}`);
  console.log('\n═══════════════════════════════════════════════════════════════\n');
}

main().catch(e => {
  console.error('Fatal error:', e.message);
  process.exit(1);
});

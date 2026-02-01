/**
 * cynic benchmark - Performance Testing
 *
 * Runs performance benchmarks for CYNIC components.
 *
 * Usage:
 *   cynic benchmark              Run all benchmarks
 *   cynic benchmark --suite <n>  Run specific benchmark suite
 *   cynic benchmark --quick      Quick smoke test
 *   cynic benchmark --json       Output results as JSON
 *
 * @module @cynic/node/cli/commands/benchmark
 */

'use strict';

import chalk from 'chalk';
import { performance } from 'perf_hooks';

const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;

/**
 * Benchmark result type
 */
class BenchmarkResult {
  constructor(name, iterations, totalMs, minMs, maxMs) {
    this.name = name;
    this.iterations = iterations;
    this.totalMs = totalMs;
    this.avgMs = totalMs / iterations;
    this.minMs = minMs;
    this.maxMs = maxMs;
    this.opsPerSec = Math.round(1000 / this.avgMs * iterations);
  }
}

/**
 * Run a single benchmark
 */
async function runBenchmark(name, fn, options = {}) {
  const { iterations = 100, warmup = 10 } = options;

  // Warmup
  for (let i = 0; i < warmup; i++) {
    await fn();
  }

  // Run benchmark
  const times = [];
  const start = performance.now();

  for (let i = 0; i < iterations; i++) {
    const iterStart = performance.now();
    await fn();
    times.push(performance.now() - iterStart);
  }

  const totalMs = performance.now() - start;
  const minMs = Math.min(...times);
  const maxMs = Math.max(...times);

  return new BenchmarkResult(name, iterations, totalMs, minMs, maxMs);
}

/**
 * Format benchmark result for display
 */
function formatResult(result, verbose = false) {
  const avgStr = result.avgMs < 1
    ? `${(result.avgMs * 1000).toFixed(2)} µs`
    : `${result.avgMs.toFixed(2)} ms`;

  const opsStr = result.opsPerSec > 1000
    ? `${(result.opsPerSec / 1000).toFixed(1)}k`
    : result.opsPerSec.toString();

  const line = `  ${result.name.padEnd(30)} ${avgStr.padStart(12)} ${(opsStr + ' ops/s').padStart(12)}`;

  if (verbose) {
    return line + chalk.gray(` (min: ${result.minMs.toFixed(2)}ms, max: ${result.maxMs.toFixed(2)}ms)`);
  }

  return line;
}

/**
 * Judge benchmark suite
 */
async function benchmarkJudge(quick = false) {
  const results = [];
  const iterations = quick ? 10 : 100;

  try {
    const { CYNICJudge } = await import('@cynic/node');
    const judge = new CYNICJudge();

    // Simple judgment
    results.push(await runBenchmark('Judge: Simple string', async () => {
      await judge.judge({ type: 'code', content: 'console.log("hello")' });
    }, { iterations }));

    // Complex judgment
    const complexItem = {
      type: 'code',
      content: 'function fibonacci(n) { return n <= 1 ? n : fibonacci(n-1) + fibonacci(n-2); }',
      context: { language: 'javascript', purpose: 'algorithm' },
    };

    results.push(await runBenchmark('Judge: Complex with context', async () => {
      await judge.judge(complexItem);
    }, { iterations }));

  } catch (error) {
    console.log(chalk.yellow(`  ⚠ Judge benchmarks skipped: ${error.message}`));
  }

  return results;
}

/**
 * Pattern detection benchmark suite
 */
async function benchmarkPatterns(quick = false) {
  const results = [];
  const iterations = quick ? 10 : 50;

  try {
    const { PatternDetector } = await import('@cynic/node');
    const detector = new PatternDetector();

    // Feed some data first
    for (let i = 0; i < 20; i++) {
      detector.observe({ type: 'test', value: Math.random(), timestamp: Date.now() });
    }

    // Observe new event
    results.push(await runBenchmark('Pattern: Observe event', async () => {
      detector.observe({ type: 'benchmark', value: Math.random() });
    }, { iterations }));

    // Detect patterns
    results.push(await runBenchmark('Pattern: Detect all', async () => {
      detector.detect();
    }, { iterations }));

  } catch (error) {
    console.log(chalk.yellow(`  ⚠ Pattern benchmarks skipped: ${error.message}`));
  }

  return results;
}

/**
 * Router benchmark suite
 */
async function benchmarkRouter(quick = false) {
  const results = [];
  const iterations = quick ? 10 : 100;

  try {
    const { KabbalisticRouter, QLearningRouter } = await import('@cynic/node');

    // Kabbalistic Router
    const kRouter = new KabbalisticRouter();
    results.push(await runBenchmark('Router: Kabbalistic route', async () => {
      kRouter.route({ type: 'code', content: 'test' });
    }, { iterations }));

    // Q-Learning Router
    const qRouter = new QLearningRouter();
    results.push(await runBenchmark('Router: Q-Learning route', async () => {
      qRouter.selectAction({ complexity: 0.5, hasCode: true });
    }, { iterations }));

  } catch (error) {
    console.log(chalk.yellow(`  ⚠ Router benchmarks skipped: ${error.message}`));
  }

  return results;
}

/**
 * Vector/Embedding benchmark suite
 */
async function benchmarkVector(quick = false) {
  const results = [];
  const iterations = quick ? 5 : 20;

  try {
    const { UnifiedEmbedder } = await import('@cynic/core');
    const embedder = new UnifiedEmbedder();

    // Short text embedding
    results.push(await runBenchmark('Vector: Short text embed', async () => {
      embedder.embed('Hello world');
    }, { iterations }));

    // Long text embedding
    const longText = 'The quick brown fox jumps over the lazy dog. '.repeat(50);
    results.push(await runBenchmark('Vector: Long text embed', async () => {
      embedder.embed(longText);
    }, { iterations }));

    // Similarity calculation
    const vec1 = embedder.embed('Hello world');
    const vec2 = embedder.embed('Hi there');
    results.push(await runBenchmark('Vector: Cosine similarity', async () => {
      embedder.similarity(vec1, vec2);
    }, { iterations: iterations * 10 }));

  } catch (error) {
    console.log(chalk.yellow(`  ⚠ Vector benchmarks skipped: ${error.message}`));
  }

  return results;
}

/**
 * Core operations benchmark
 */
async function benchmarkCore(quick = false) {
  const results = [];
  const iterations = quick ? 100 : 1000;

  try {
    const { PHI, PHI_INV, fibonacci } = await import('@cynic/core');

    // Fibonacci calculation
    results.push(await runBenchmark('Core: Fibonacci(20)', async () => {
      fibonacci(20);
    }, { iterations }));

    // Hash calculation
    const { createHash } = await import('crypto');
    results.push(await runBenchmark('Core: SHA256 hash', async () => {
      createHash('sha256').update('test input').digest('hex');
    }, { iterations }));

  } catch (error) {
    console.log(chalk.yellow(`  ⚠ Core benchmarks skipped: ${error.message}`));
  }

  return results;
}

/**
 * Benchmark command handler
 */
export async function benchmarkCommand(options) {
  const { suite, quick = false, json = false, verbose = false } = options;

  if (!json) {
    console.log(chalk.yellow('\n╔═════════════════════════════════════════╗'));
    console.log(chalk.yellow('║') + chalk.bold.cyan('  CYNIC Performance Benchmarks          ') + chalk.yellow('║'));
    console.log(chalk.yellow('╚═════════════════════════════════════════╝\n'));

    console.log(chalk.gray(`  Mode: ${quick ? 'Quick' : 'Full'}`));
    console.log(chalk.gray(`  Suite: ${suite || 'All'}\n`));
  }

  const allResults = [];
  const suites = {
    core: { name: 'Core Operations', fn: benchmarkCore },
    judge: { name: 'Judgment System', fn: benchmarkJudge },
    patterns: { name: 'Pattern Detection', fn: benchmarkPatterns },
    router: { name: 'Routing', fn: benchmarkRouter },
    vector: { name: 'Vector/Embeddings', fn: benchmarkVector },
  };

  const suitesToRun = suite ? { [suite]: suites[suite] } : suites;

  for (const [key, suiteConfig] of Object.entries(suitesToRun)) {
    if (!suiteConfig) {
      console.log(chalk.red(`  Unknown suite: ${key}`));
      continue;
    }

    if (!json) {
      console.log(chalk.bold(`  ── ${suiteConfig.name} ──`));
    }

    const startTime = performance.now();
    const results = await suiteConfig.fn(quick);
    const suiteTime = performance.now() - startTime;

    if (!json) {
      for (const result of results) {
        console.log(formatResult(result, verbose));
      }
      console.log(chalk.gray(`  Suite completed in ${(suiteTime / 1000).toFixed(2)}s\n`));
    }

    allResults.push(...results.map(r => ({ suite: key, ...r })));
  }

  // Summary
  if (json) {
    console.log(JSON.stringify({
      timestamp: new Date().toISOString(),
      mode: quick ? 'quick' : 'full',
      results: allResults,
    }, null, 2));
  } else {
    console.log(chalk.yellow('═════════════════════════════════════════'));

    const totalBenchmarks = allResults.length;
    const avgOps = totalBenchmarks > 0
      ? Math.round(allResults.reduce((sum, r) => sum + r.opsPerSec, 0) / totalBenchmarks)
      : 0;

    console.log(`\n  Total benchmarks: ${totalBenchmarks}`);
    console.log(`  Average throughput: ${avgOps.toLocaleString()} ops/s`);
    console.log(chalk.green('\n  *tail wag* Benchmarks complete!'));
    console.log(chalk.cyan(`  φ⁻¹ = ${(PHI_INV * 100).toFixed(1)}% max confidence\n`));
  }
}

export default { benchmarkCommand };

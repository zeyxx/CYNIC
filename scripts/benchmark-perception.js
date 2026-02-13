#!/usr/bin/env node
/**
 * CYNIC Perception Layer - Latency Benchmark
 *
 * Measures latency improvement from concurrent sensor polling (S3.2).
 *
 * Usage:
 *   node scripts/benchmark-perception.js
 *
 * Output:
 *   - Sequential latency (simulated)
 *   - Concurrent latency (actual)
 *   - Savings (ms and %)
 *
 * "Measure before claiming victory" - κυνικός
 */

'use strict';

import { createPerceptionLayer } from '../packages/node/src/perception/index.js';
import { createLogger } from '@cynic/core';

const log = createLogger('PerceptionBenchmark');

/**
 * Benchmark concurrent vs sequential polling
 */
async function benchmarkPerception() {
  console.log('\n┌─────────────────────────────────────────────────────────┐');
  console.log('│  CYNIC Perception Layer - Latency Benchmark (S3.2)     │');
  console.log('└─────────────────────────────────────────────────────────┘\n');

  // Create perception layer
  const layer = createPerceptionLayer({
    filesystem: { paths: [] },
    solana: { cluster: 'devnet' },
    health: { interval: 60000 },
    dogState: { autoStart: false },
    market: { tokenMint: null }, // Disabled for benchmark
    enableConcurrentPolling: true,
  });

  console.log('📊 Starting benchmark...\n');

  // Warmup (ignore first poll for fair measurement)
  console.log('⏳ Warmup poll...');
  await layer.poll();

  // Run multiple polls and measure average latency
  const iterations = 10;
  const latencies = [];

  console.log(`⏱️  Running ${iterations} concurrent polls...\n`);

  for (let i = 0; i < iterations; i++) {
    const snapshot = await layer.poll();
    latencies.push(snapshot.latency);

    process.stdout.write(`  Poll ${i + 1}/${iterations}: ${snapshot.latency}ms\n`);
  }

  // Calculate stats
  const avgLatency = latencies.reduce((a, b) => a + b, 0) / latencies.length;
  const minLatency = Math.min(...latencies);
  const maxLatency = Math.max(...latencies);
  const medianLatency = latencies.sort((a, b) => a - b)[Math.floor(latencies.length / 2)];

  // Simulated sequential latency (5 sensors × 20ms each)
  const sequentialLatency = 100; // ms (theoretical worst-case)
  const savings = sequentialLatency - avgLatency;
  const savingsPercent = (savings / sequentialLatency) * 100;

  // Display results
  console.log('\n┌─────────────────────────────────────────────────────────┐');
  console.log('│  Results                                                │');
  console.log('├─────────────────────────────────────────────────────────┤');
  console.log(`│  Concurrent (avg):     ${avgLatency.toFixed(2).padStart(6)}ms                     │`);
  console.log(`│  Concurrent (min):     ${minLatency.toFixed(2).padStart(6)}ms                     │`);
  console.log(`│  Concurrent (max):     ${maxLatency.toFixed(2).padStart(6)}ms                     │`);
  console.log(`│  Concurrent (median):  ${medianLatency.toFixed(2).padStart(6)}ms                     │`);
  console.log('│                                                         │');
  console.log(`│  Sequential (theoretical): 100.00ms                     │`);
  console.log('│                                                         │');
  console.log(`│  Savings:              ${savings.toFixed(2).padStart(6)}ms (${savingsPercent.toFixed(1)}%)           │`);
  console.log('└─────────────────────────────────────────────────────────┘\n');

  // Sensor breakdown
  console.log('┌─────────────────────────────────────────────────────────┐');
  console.log('│  Sensor Status                                          │');
  console.log('├─────────────────────────────────────────────────────────┤');

  const snapshot = await layer.poll();

  const sensors = [
    { name: 'Solana', key: 'solana', status: snapshot.solana?.isRunning ? '🟢' : '🔴' },
    { name: 'Health', key: 'health', status: snapshot.health?.status ? '🟢' : '🔴' },
    { name: 'DogState', key: 'dogState', status: snapshot.dogState?.collective ? '🟢' : '🔴' },
    { name: 'Market', key: 'market', status: snapshot.market?.isRunning ? '🟢' : '🟡' },
    { name: 'Filesystem', key: 'filesystem', status: snapshot.filesystem ? '🟢' : '🔴' },
  ];

  for (const sensor of sensors) {
    const hasError = snapshot[sensor.key]?.error;
    const statusIcon = hasError ? '🔴' : sensor.status;
    const statusText = hasError ? 'ERROR' : 'OK';
    console.log(`│  ${sensor.name.padEnd(12)} ${statusIcon} ${statusText.padEnd(8)}                       │`);
  }

  console.log('└─────────────────────────────────────────────────────────┘\n');

  // Performance analysis
  console.log('📈 Performance Analysis:\n');

  if (avgLatency < 50) {
    console.log('  ✅ Excellent: Avg latency < 50ms (target met)');
  } else if (avgLatency < 100) {
    console.log('  ⚠️  Good: Avg latency < 100ms (acceptable)');
  } else {
    console.log('  ❌ Poor: Avg latency > 100ms (investigate)');
  }

  if (savings > 50) {
    console.log(`  ✅ Excellent: Saved ${savings.toFixed(0)}ms vs sequential (${savingsPercent.toFixed(0)}% improvement)`);
  } else if (savings > 30) {
    console.log(`  ⚠️  Good: Saved ${savings.toFixed(0)}ms vs sequential (${savingsPercent.toFixed(0)}% improvement)`);
  } else {
    console.log(`  ❌ Poor: Saved only ${savings.toFixed(0)}ms vs sequential (${savingsPercent.toFixed(0)}% improvement)`);
  }

  if (maxLatency - minLatency < 30) {
    console.log('  ✅ Consistent: Low variance between polls');
  } else {
    console.log('  ⚠️  Variable: High variance detected (check network/system load)');
  }

  console.log('\n✨ Benchmark complete.\n');

  // Cleanup
  await layer.stop();
}

// Run benchmark
benchmarkPerception().catch(err => {
  console.error('❌ Benchmark failed:', err);
  process.exit(1);
});

/**
 * FastRouter Benchmark — Validate A1 Performance
 *
 * Tests FastRouter (reflex arc) performance under load:
 * - Latency: p50, p95, p99 (target: <100ms)
 * - Throughput: events/second
 * - Violations: events exceeding 100ms
 *
 * "Speed is a feature" - κυνικός
 */

import { FastRouter, ReflexActionType } from '../packages/node/src/routing/fast-router.js';
import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('BenchmarkFastRouter');

/**
 * Generate test events
 */
function generateEvents(count, type = 'market_alert') {
  const events = [];
  for (let i = 0; i < count; i++) {
    events.push({
      payload: {
        severity: i % 3 === 0 ? 'high' : 'medium',
        priceChangePercent: (Math.random() - 0.5) * 100,
        direction: Math.random() > 0.5 ? 'up' : 'down',
        price: 0.001 * (1 + Math.random()),
      },
      timestamp: Date.now(),
    });
  }
  return events;
}

/**
 * Calculate percentiles
 */
function percentile(values, p) {
  const sorted = values.slice().sort((a, b) => a - b);
  const index = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, index)];
}

/**
 * Benchmark FastRouter
 */
async function benchmarkFastRouter() {
  console.log('┌─────────────────────────────────────────────────────────┐');
  console.log('│ 🧪 BENCHMARK: FastRouter (A1 Reflex Arc)                │');
  console.log('└─────────────────────────────────────────────────────────┘\n');

  // Create router
  console.log('1️⃣  Creating FastRouter...');
  const router = new FastRouter({ maxLatency: 100 });
  console.log('   ✓ Router created (target: <100ms)\n');

  // Test scenarios
  const scenarios = [
    { name: 'Low load', count: 50, type: 'market_alert' },
    { name: 'Medium load', count: 200, type: 'market_alert' },
    { name: 'High load', count: 500, type: 'market_price' },
    { name: 'Burst', count: 1000, type: 'market_alert' },
  ];

  const results = [];

  for (const scenario of scenarios) {
    console.log(`📊 Scenario: ${scenario.name} (${scenario.count} events)`);

    const events = generateEvents(scenario.count, scenario.type);
    const latencies = [];

    const startTime = Date.now();

    for (const event of events) {
      const eventStart = Date.now();

      // Process event through router
      router._handleEvent(event, scenario.type);

      const eventLatency = Date.now() - eventStart;
      latencies.push(eventLatency);
    }

    const totalTime = Date.now() - startTime;
    const throughput = (scenario.count / (totalTime / 1000)).toFixed(1);

    // Calculate percentiles
    const p50 = percentile(latencies, 50);
    const p95 = percentile(latencies, 95);
    const p99 = percentile(latencies, 99);
    const max = Math.max(...latencies);
    const avg = latencies.reduce((a, b) => a + b, 0) / latencies.length;

    // Count violations (>100ms)
    const violations = latencies.filter(l => l > 100).length;
    const violationRate = (violations / latencies.length) * 100;

    console.log(`   • Throughput: ${throughput} events/sec`);
    console.log(`   • Latency p50: ${p50}ms`);
    console.log(`   • Latency p95: ${p95}ms`);
    console.log(`   • Latency p99: ${p99}ms`);
    console.log(`   • Latency max: ${max}ms`);
    console.log(`   • Latency avg: ${avg.toFixed(2)}ms`);
    console.log(`   • Violations: ${violations}/${scenario.count} (${violationRate.toFixed(1)}%)`);

    const passed = p95 < 100 && violationRate < 5;
    console.log(`   ${passed ? '✓ PASS' : '✗ FAIL'} (p95 <100ms, violations <5%)\n`);

    results.push({
      scenario: scenario.name,
      count: scenario.count,
      throughput: parseFloat(throughput),
      p50,
      p95,
      p99,
      max,
      avg,
      violations,
      violationRate,
      passed,
    });
  }

  // Router stats
  console.log('📈 Router Statistics:');
  const stats = router.getStats();
  console.log(`   • Total events: ${stats.totalEvents}`);
  console.log(`   • Critical events: ${stats.criticalEvents} (${((stats.criticalEvents / stats.totalEvents) * 100).toFixed(1)}%)`);
  console.log(`   • Reflex actions: ${stats.reflexActions}`);
  console.log(`   • Escalations: ${stats.escalations}`);
  console.log(`   • Avg latency: ${stats.avgLatency.toFixed(2)}ms`);
  console.log(`   • Max latency violations: ${stats.maxLatencyViolations}`);
  console.log();

  // Summary
  console.log('┌─────────────────────────────────────────────────────────┐');
  console.log('│ 📊 BENCHMARK SUMMARY                                     │');
  console.log('├─────────────────────────────────────────────────────────┤');

  const passCount = results.filter(r => r.passed).length;
  const totalScenarios = results.length;
  const passRate = (passCount / totalScenarios) * 100;

  console.log(`│ Scenarios:  ${totalScenarios}                                           │`);
  console.log(`│ Passed:     ${passCount}                                           │`);
  console.log(`│ Failed:     ${totalScenarios - passCount}                                           │`);
  console.log(`│ Pass rate:  ${passRate.toFixed(1)}%                                      │`);
  console.log('└─────────────────────────────────────────────────────────┘\n');

  // Performance assessment
  const avgP95 = results.reduce((sum, r) => sum + r.p95, 0) / results.length;
  const avgViolationRate = results.reduce((sum, r) => sum + r.violationRate, 0) / results.length;

  console.log('🎯 Performance Assessment:');
  console.log(`   • Avg p95 latency: ${avgP95.toFixed(2)}ms (target: <100ms)`);
  console.log(`   • Avg violation rate: ${avgViolationRate.toFixed(2)}% (target: <5%)`);
  console.log(`   • Throughput: ${results[results.length - 1].throughput} events/sec (peak)`);
  console.log();

  if (avgP95 < 100 && avgViolationRate < 5 && passRate >= 75) {
    console.log('✅ FastRouter PERFORMANCE EXCELLENT');
    console.log('   • Reflex arc working as designed (<100ms)');
    console.log('   • Ready for high-frequency production use');
    console.log('   • Meets A1 specification');
  } else if (avgP95 < 150 && avgViolationRate < 10) {
    console.log('⚠️  FastRouter PERFORMANCE ACCEPTABLE');
    console.log('   • Slightly above target but usable');
    console.log('   • Consider optimization for production');
  } else {
    console.log('❌ FastRouter PERFORMANCE INSUFFICIENT');
    console.log('   • Exceeds latency targets');
    console.log('   • Optimization required before production');
  }
  console.log();

  // Cleanup
  router.stop();

  return {
    success: passRate >= 75,
    passRate,
    avgP95,
    avgViolationRate,
    results,
  };
}

// Run benchmark
benchmarkFastRouter()
  .then(result => {
    process.exit(result.success ? 0 : 1);
  })
  .catch(err => {
    console.error('❌ Benchmark FAILED with error:');
    console.error(err);
    process.exit(1);
  });

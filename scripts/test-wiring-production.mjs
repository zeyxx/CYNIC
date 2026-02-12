#!/usr/bin/env node
/**
 * Production Wiring Tests
 *
 * Validates watchdog, budget control, and event wiring under realistic conditions.
 * Run from daemon context or with singletons loaded.
 *
 * Usage: node scripts/test-wiring-production.mjs
 */

import { globalEventBus } from '@cynic/core';
import { EventEmitter } from 'events';

const results = {
  passed: 0,
  failed: 0,
  tests: [],
};

/**
 * Test harness
 */
async function runTest(name, testFn) {
  console.log(`\n🧪 Running: ${name}`);
  try {
    await testFn();
    console.log(`✅ PASS: ${name}`);
    results.passed++;
    results.tests.push({ name, status: 'pass' });
  } catch (err) {
    console.error(`❌ FAIL: ${name}`);
    console.error(`   Error: ${err.message}`);
    results.failed++;
    results.tests.push({ name, status: 'fail', error: err.message });
  }
}

/**
 * Helper: Wait for event with timeout
 */
function waitForEvent(emitter, eventName, timeoutMs = 5000) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      emitter.removeListener(eventName, handler);
      reject(new Error(`Timeout waiting for event: ${eventName}`));
    }, timeoutMs);

    const handler = (data) => {
      clearTimeout(timer);
      resolve(data);
    };

    emitter.once(eventName, handler);
  });
}

// =============================================================================
// TEST 1: Model Recommendation Event Flow
// =============================================================================

await runTest('Model recommendation flows to router', async () => {
  let received = false;
  const listener = () => { received = true; };

  globalEventBus.on('model:recommendation', listener);

  globalEventBus.emit('model:recommendation', {
    model: 'haiku',
    reason: 'test',
    budgetLevel: 'moderate',
  });

  await new Promise(r => setTimeout(r, 100)); // Let event propagate

  globalEventBus.removeListener('model:recommendation', listener);

  if (!received) {
    throw new Error('Event not received by listener');
  }
});

// =============================================================================
// TEST 2: Error Clustering Detection
// =============================================================================

await runTest('Error clustering detection', async () => {
  const errorMsg = 'Test error: connection timeout';

  // Emit same error 3 times
  for (let i = 0; i < 3; i++) {
    globalEventBus.emit('ERROR_OCCURRED', {
      error: new Error(errorMsg),
      context: 'test',
      tool: 'Bash',
      command: 'curl',
    });
  }

  // EmergenceDetector should emit pattern:detected
  // Note: This requires EmergenceDetector singleton to be running
  console.log('   ℹ️  EmergenceDetector should detect error clustering pattern');
  console.log('   ℹ️  Check logs for "Error clustering detected"');
});

// =============================================================================
// TEST 3: Learning Stagnation Detection
// =============================================================================

await runTest('Learning stagnation detection', async () => {
  // Emit 5 non-converged cycles
  for (let i = 0; i < 5; i++) {
    globalEventBus.emit('learning:cycle:complete', {
      module: 'test-module',
      converged: false,
      maturity: 0.3,
    });
  }

  console.log('   ℹ️  EmergenceDetector should detect learning stagnation');
  console.log('   ℹ️  Check logs for "Learning stagnation detected"');
});

// =============================================================================
// TEST 4: Subagent Spawn Storm Detection
// =============================================================================

await runTest('Subagent spawn storm detection', async () => {
  // Emit 5 subagent starts rapidly
  for (let i = 0; i < 5; i++) {
    globalEventBus.emit('SUBAGENT_STARTED', {
      subagentId: `test-${i}`,
      taskType: 'exploration',
      parentId: 'test-parent',
    });
  }

  console.log('   ℹ️  UnifiedOrchestrator should detect spawn storm');
  console.log('   ℹ️  Check logs for "Subagent spawn storm detected"');
});

// =============================================================================
// TEST 5: Health Degradation Event
// =============================================================================

await runTest('Health degradation event flow', async () => {
  globalEventBus.emit('daemon:health:degraded', {
    level: 'warning',
    issues: [{ subsystem: 'test', level: 'warning', message: 'Test warning' }],
    timestamp: Date.now(),
  });

  console.log('   ℹ️  KabbalisticRouter should prefer LIGHT tier');
  console.log('   ℹ️  Check logs for "Health WARNING"');
});

// =============================================================================
// TEST 6: Memory Pressure Event
// =============================================================================

await runTest('Memory pressure event flow', async () => {
  globalEventBus.emit('daemon:memory:pressure', {
    heapRatio: 0.85,
    heapUsedMB: 800,
    timestamp: Date.now(),
  });

  console.log('   ℹ️  ContextCompressor should clear caches');
  console.log('   ℹ️  ModelIntelligence should force Haiku');
  console.log('   ℹ️  Check logs for "Memory pressure detected"');
});

// =============================================================================
// TEST 7: Velocity Alarm Event
// =============================================================================

await runTest('Velocity alarm event flow', async () => {
  globalEventBus.emit('velocity:alarm', {
    velocity: 0.75,
    trend: 'accelerating',
    tokensPerMinute: 5000,
    action: 'throttle_and_downgrade',
  });

  console.log('   ℹ️  CostLedger should record alarm');
  console.log('   ℹ️  EmergenceDetector should track spike');
  console.log('   ℹ️  Check logs for "Velocity alarm recorded"');
});

// =============================================================================
// RESULTS
// =============================================================================

console.log('\n' + '='.repeat(60));
console.log('📊 Test Results');
console.log('='.repeat(60));
console.log(`✅ Passed: ${results.passed}`);
console.log(`❌ Failed: ${results.failed}`);
console.log(`📈 Success Rate: ${((results.passed / (results.passed + results.failed)) * 100).toFixed(1)}%`);

if (results.failed > 0) {
  console.log('\nFailed tests:');
  results.tests.filter(t => t.status === 'fail').forEach(t => {
    console.log(`  - ${t.name}: ${t.error}`);
  });
}

console.log('\n💡 Manual Verification Checklist:');
console.log('  1. Check daemon logs for event handling');
console.log('  2. Verify ContextCompressor cache clearing');
console.log('  3. Verify ModelIntelligence model selection');
console.log('  4. Check EmergenceDetector pattern detection');
console.log('  5. Check UnifiedOrchestrator subagent tracking');
console.log('  6. Verify KabbalisticRouter tier forcing');
console.log('  7. Check CostLedger velocity alarm history');

console.log('\n*sniff* Tests complete. Check logs for detailed behavior.');

process.exit(results.failed > 0 ? 1 : 0);

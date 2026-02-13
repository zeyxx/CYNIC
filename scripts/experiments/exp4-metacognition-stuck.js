#!/usr/bin/env node
/**
 * Experiment 4: Meta-Cognition Stuck Detection Validation
 *
 * **Goal**: Validate stuck state detection prevents thrashing
 * **Method**: Simulate repetitive failed actions, measure recovery time
 * **Success**: Stuck detected within 5 failures, strategy switches automatically
 *
 * From: docs/architecture/learning-validation.md § 6.3
 *
 * Usage: node scripts/experiments/exp4-metacognition-stuck.js
 */

'use strict';

import { MetaCognition } from '../../packages/node/src/learning/meta-cognition.js';
import { globalEventBus, EventType } from '@cynic/core';

console.log('🐕 Experiment 4: Meta-Cognition Stuck Detection');
console.log('===============================================\n');

/**
 * Run Experiment 4
 */
async function runExperiment() {
  console.log('Step 1: Initializing MetaCognition service...\n');

  const service = new MetaCognition({
    serviceId: 'exp4-metacognition',
    config: {
      stuckThreshold: 5, // 5 consecutive failures = stuck
      successRateWindow: 10,
      strategyNames: ['default', 'fallback', 'conservative'],
    },
  });

  console.log('✓ MetaCognition service initialized\n');

  console.log('Step 2: Simulating repetitive failed actions...\n');

  let stuckDetectedAtFailure = null;
  let initialStrategy = service.currentStrategy;

  console.log(`   Initial strategy: ${initialStrategy}`);
  console.log('   Simulating 10 consecutive failures...\n');

  // Simulate 10 consecutive failures
  for (let i = 1; i <= 10; i++) {
    service.recordAction({
      action: 'test_action',
      success: false,
      confidence: 0.3,
      duration: 100,
    });

    const state = service.getState().state;
    const strategy = service.currentStrategy;

    // Check if stuck state reached
    if ((state === 'stuck' || state === 'thrashing') && !stuckDetectedAtFailure) {
      stuckDetectedAtFailure = i;
    }

    console.log(`   Failure ${i}/10 - State: ${state}, Strategy: ${strategy}`);

    // Small delay
    await new Promise(r => setTimeout(r, 10));
  }

  console.log();

  console.log();
  console.log('Step 3: Checking stuck detection...\n');

  const finalState = service.getState().state;
  const finalStrategy = service.currentStrategy;
  const strategyChanged = finalStrategy !== initialStrategy;

  if (stuckDetectedAtFailure) {
    console.log(`✓ Stuck detected at failure #${stuckDetectedAtFailure}`);
    console.log(`  Threshold: 5 failures`);
    console.log(`  Status: ${stuckDetectedAtFailure <= 5 ? '✓ PASS (within threshold)' : '✗ FAIL (too slow)'}\n`);
  } else {
    console.log(`✗ Stuck NOT detected after 10 failures\n`);
  }

  console.log('Step 4: Checking strategy switching...\n');

  console.log(`  Initial strategy: ${initialStrategy}`);
  console.log(`  Final strategy: ${finalStrategy}`);
  console.log(`  Changed: ${strategyChanged ? 'YES ✓' : 'NO ✗'}\n`);

  console.log('Step 5: Testing recovery...\n');

  // Record some successes to verify recovery
  for (let i = 1; i <= 3; i++) {
    service.recordAction({
      action: 'test_action',
      success: true,
      confidence: 0.8,
      duration: 50,
    });

    console.log(`   Success ${i}/3`);
    await new Promise(r => setTimeout(r, 10));
  }

  const stats = service.getStats();

  console.log(`\n   Success rate: ${(stats.successRate * 100).toFixed(1)}%`);
  console.log(`   Total actions: ${stats.totalActions}`);
  console.log(`   Current state: ${stats.currentState || 'unknown'}\n`);

  // Final validation
  console.log('═'.repeat(60));
  console.log('\nExperiment 4 Results:\n');

  const tests = {
    'Stuck detected': !!stuckDetectedAtFailure,
    'Detected within 5 failures': stuckDetectedAtFailure && stuckDetectedAtFailure <= 5,
    'Strategy switching works': strategyChanged,
    'Final state correct': finalState === 'stuck' || finalState === 'thrashing' || finalState === 'recovering',
  };

  let passCount = 0;
  const totalTests = Object.keys(tests).length;

  for (const [test, pass] of Object.entries(tests)) {
    if (pass) passCount++;
    console.log(`  ${pass ? '✓' : '✗'} ${test}`);
  }

  console.log('\n' + '═'.repeat(60));

  if (tests['Stuck detected'] && tests['Detected within 5 failures'] && tests['Strategy switching works']) {
    console.log('\n🎉 Experiment 4: SUCCESS ✓');
    console.log('\nMeta-Cognition validation passed:');
    console.log(`  - Stuck detected at failure #${stuckDetectedAtFailure} (threshold: 5)`);
    console.log(`  - Strategy switching: ${initialStrategy} → ${finalStrategy}`);
    console.log(`  - Final state: ${finalState}`);
    console.log('\nMeta-Cognition prevents thrashing via early stuck detection.');
    console.log('Task #15: COMPLETE ✓');
    process.exit(0);
  } else {
    console.log('\n⚠️ Experiment 4: PARTIAL');
    console.log(`\nIssues detected:`);
    if (!tests['Stuck detected']) {
      console.log(`  - Stuck state not detected after 10 failures`);
    }
    if (!tests['Detected within 5 failures']) {
      console.log(`  - Stuck detected too late (failure #${stuckDetectedAtFailure}, threshold: 5)`);
    }
    if (!tests['Strategy switching works']) {
      console.log(`  - Strategy switching did not occur`);
    }
    process.exit(1);
  }
}

// Run experiment
try {
  await runExperiment();
} catch (error) {
  console.error('\n✗ Experiment failed:', error.message);
  console.error(error.stack);
  process.exit(1);
}

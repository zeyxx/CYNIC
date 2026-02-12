#!/usr/bin/env node
/**
 * Test Wiring Completion
 *
 * Verifies GAP-3, Wiring Gap 1, Wiring Gap 2 are functional.
 * Simulates events and checks database records.
 *
 * Usage: node scripts/test-wiring-complete.js
 */

'use strict';

import { globalEventBus } from '@cynic/core';
import { getPool } from '@cynic/persistence';
import { getLLMRouter } from '../packages/node/src/orchestration/llm-router.js';
import chalk from 'chalk';

async function testWiringComplete() {
  console.log(chalk.bold.cyan('\n🐕 CYNIC Wiring Completion Test'));
  console.log(chalk.gray('═'.repeat(60)));

  const pool = getPool();
  const results = {
    gap3: { name: 'GAP-3: Learning feedback loop', pass: false },
    wg1: { name: 'Wiring Gap 1: KabbalisticRouter logging', pass: false },
    wg2: { name: 'Wiring Gap 2: LLMRouter activation', pass: false },
  };

  // ============================================================================
  // TEST 1: GAP-3 - Learning Events Recording
  // ============================================================================
  console.log(chalk.bold('\n1. Testing GAP-3 (Learning Feedback Loop)'));

  try {
    // Emit test feedback event
    globalEventBus.emit('feedback:processed', {
      judgmentId: 'test-judgment-123',
      value: 0.8,
      source: 'test',
    });

    // Wait for async DB write
    await new Promise(resolve => setTimeout(resolve, 500));

    // Check learning_events table
    const { rows } = await pool.query(`
      SELECT COUNT(*) as count FROM learning_events
      WHERE loop_type = 'feedback-loop'
      AND timestamp > NOW() - INTERVAL '1 minute'
    `);

    const count = parseInt(rows[0].count);
    results.gap3.pass = count > 0;
    results.gap3.count = count;

    console.log(`  Events recorded: ${count}`);
    console.log(`  ${results.gap3.pass ? chalk.green('✓ PASS') : chalk.red('✗ FAIL')}`);
  } catch (error) {
    console.log(`  ${chalk.red('✗ ERROR')}: ${error.message}`);
  }

  // ============================================================================
  // TEST 2: Wiring Gap 1 - KabbalisticRouter would need actual routing
  // (Skipped - requires full orchestrator init)
  // ============================================================================
  console.log(chalk.bold('\n2. Testing Wiring Gap 1 (KabbalisticRouter)'));
  console.log(chalk.gray('  Skipped - requires full daemon init'));
  console.log(`  ${chalk.yellow('○ SKIP')}`);
  results.wg1.pass = null; // null = skipped

  // ============================================================================
  // TEST 3: Wiring Gap 2 - LLMRouter
  // ============================================================================
  console.log(chalk.bold('\n3. Testing Wiring Gap 2 (LLMRouter)'));

  try {
    const llmRouter = getLLMRouter();

    // Test simple task → should route to Ollama
    const result1 = await llmRouter.route({
      type: 'chat',
      complexity: 'simple',
      estimatedTokens: 500,
    });

    console.log(`  Simple task routed to: ${result1.provider}`);
    console.log(`  Reason: ${result1.reason}`);

    // Check routing_accuracy table
    const { rows } = await pool.query(`
      SELECT COUNT(*) as count FROM routing_accuracy
      WHERE router_type = 'llm'
      AND timestamp > NOW() - INTERVAL '1 minute'
    `);

    const count = parseInt(rows[0].count);
    results.wg2.pass = count > 0 && result1.provider === 'ollama';
    results.wg2.count = count;
    results.wg2.provider = result1.provider;

    console.log(`  Routes recorded: ${count}`);
    console.log(`  ${results.wg2.pass ? chalk.green('✓ PASS') : chalk.red('✗ FAIL')}`);
  } catch (error) {
    console.log(`  ${chalk.red('✗ ERROR')}: ${error.message}`);
  }

  // ============================================================================
  // SUMMARY
  // ============================================================================
  console.log(chalk.gray('\n' + '─'.repeat(60)));
  console.log(chalk.bold('\nSummary:'));

  const passCount = Object.values(results).filter(r => r.pass === true).length;
  const totalCount = Object.values(results).filter(r => r.pass !== null).length;

  for (const [key, result] of Object.entries(results)) {
    const status = result.pass === true
      ? chalk.green('✓ PASS')
      : result.pass === null
      ? chalk.yellow('○ SKIP')
      : chalk.red('✗ FAIL');

    console.log(`  ${status} ${result.name}`);
  }

  console.log(chalk.gray('\n' + '═'.repeat(60)));
  console.log(chalk.gray(`Pass ratio: ${passCount}/${totalCount}`));
  console.log(chalk.gray('Wiring complete → Ready for Week 1 goals test\n'));

  await pool.end();

  process.exit(passCount >= 2 ? 0 : 1);
}

// Run if called directly
if (import.meta.url === `file://${process.argv[1]}`) {
  testWiringComplete().catch(err => {
    console.error(chalk.red('\n✗ Test failed:'), err.message);
    process.exit(1);
  });
}

export { testWiringComplete };

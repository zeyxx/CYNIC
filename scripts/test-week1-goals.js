#!/usr/bin/env node
/**
 * Week 1 Goals Test Script
 *
 * Tests all 5 Week 1 goals to verify organism is waking up.
 * Run after wiring GAP-3, Wiring Gap 1, Wiring Gap 2.
 *
 * Usage: node scripts/test-week1-goals.js
 */

'use strict';

import { getPool } from '@cynic/persistence';
import chalk from 'chalk';
import { PHI_INV } from '@cynic/core';

const GOALS = {
  'G1.1': { name: 'Watchers polling', target: 3, query: `
    SELECT COUNT(DISTINCT watcher_name) as count
    FROM watcher_heartbeats
    WHERE timestamp > NOW() - INTERVAL '1 hour'
    AND status = 'active'
  ` },
  'G1.2': { name: 'Learning loops consuming data', target: 5, query: `
    SELECT get_active_learning_loops(CURRENT_DATE) as count
  ` },
  'G1.3': { name: 'Q-weight updates/day', target: 10, query: `
    SELECT get_qweight_updates_today() as count
  ` },
  'G1.4': { name: 'KabbalisticRouter calls', target: 20, query: `
    SELECT COUNT(*) as count
    FROM routing_accuracy
    WHERE router_type = 'kabbalistic'
    AND timestamp > NOW() - INTERVAL '24 hours'
  ` },
  'G1.5': { name: 'LLMRouter non-Anthropic routes', target: 10, query: `
    SELECT COUNT(*) as count
    FROM routing_accuracy
    WHERE router_type = 'llm'
    AND timestamp > NOW() - INTERVAL '24 hours'
    AND metadata->>'provider' != 'anthropic'
  ` },
};

async function testWeek1Goals() {
  console.log(chalk.bold.cyan('\n🐕 CYNIC Week 1 Goals Test'));
  console.log(chalk.gray('═'.repeat(60)));

  const pool = getPool();
  const results = {};
  let passCount = 0;

  for (const [goalId, goal] of Object.entries(GOALS)) {
    try {
      const { rows } = await pool.query(goal.query);
      const actual = parseInt(rows[0].count || 0);
      const pass = actual >= goal.target;

      results[goalId] = { actual, pass };

      if (pass) passCount++;

      const status = pass ? chalk.green('✓ PASS') : chalk.red('✗ FAIL');
      const actualStr = actual.toString().padStart(3);
      const targetStr = goal.target.toString().padStart(3);

      console.log(`  ${goalId}: ${goal.name}`);
      console.log(`        ${status}  ${actualStr}/${targetStr}`);
    } catch (error) {
      console.log(`  ${goalId}: ${goal.name}`);
      console.log(`        ${chalk.red('✗ ERROR')} ${error.message}`);
      results[goalId] = { actual: 0, pass: false, error: error.message };
    }
  }

  console.log(chalk.gray('\n' + '─'.repeat(60)));
  console.log(chalk.bold('\nSummary:'));
  console.log(`  Pass ratio: ${formatPercent(passCount / 5)}`);
  console.log(`  Week status: ${passCount >= 4 ? chalk.green('✓ COMPLETE') : chalk.yellow('○ IN PROGRESS')}`);

  console.log(chalk.gray('\n' + '═'.repeat(60)));
  console.log(chalk.gray(`φ-bounded confidence: max ${(PHI_INV * 100).toFixed(1)}%`));
  console.log(chalk.gray('Week 1 success: 4/5 goals PASS\n'));

  await pool.end();

  // Exit code: 0 if week complete, 1 otherwise
  process.exit(passCount >= 4 ? 0 : 1);
}

function formatPercent(value) {
  const pct = (value * 100).toFixed(1);
  if (value >= 0.8) {
    return chalk.green(`${pct}%`);
  } else if (value >= 0.6) {
    return chalk.yellow(`${pct}%`);
  } else {
    return chalk.red(`${pct}%`);
  }
}

// Run if called directly
if (import.meta.url === `file://${process.argv[1]}`) {
  testWeek1Goals().catch(err => {
    console.error(chalk.red('\n✗ Test failed:'), err.message);
    process.exit(1);
  });
}

export { testWeek1Goals };

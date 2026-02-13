#!/usr/bin/env node
/**
 * Ralph Comprehensive Production Test
 *
 * Tests CYNIC from bottom to top, scientifically validates learning loops,
 * and generates actionable reports.
 *
 * "Le chien teste tout, remonte tout" - κυνικός
 */

'use strict';

import { getPool } from '@cynic/persistence';
import { createLogger, PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import chalk from 'chalk';
import { performance } from 'perf_hooks';

const log = createLogger('RalphTest');

// Test results accumulator
const results = {
  layers: {},
  scientific: {},
  recommendations: [],
  startTime: Date.now(),
  errors: [],
};

// ============================================================================
// LAYER 1: DATABASE FOUNDATION
// ============================================================================

async function testDatabaseLayer() {
  console.log(chalk.bold.cyan('\n━━━ LAYER 1: DATABASE FOUNDATION ━━━\n'));

  const layer = {
    name: 'database',
    tests: [],
    passRate: 0,
    critical: true,
  };

  const pool = getPool();

  // Test 1.1: Connection
  try {
    const start = performance.now();
    await pool.query('SELECT 1 as test');
    const duration = performance.now() - start;
    layer.tests.push({
      name: 'PostgreSQL connection',
      pass: true,
      duration: `${duration.toFixed(2)}ms`,
    });
    console.log(chalk.green('✓'), 'PostgreSQL connection', chalk.gray(`(${duration.toFixed(2)}ms)`));
  } catch (err) {
    layer.tests.push({
      name: 'PostgreSQL connection',
      pass: false,
      error: err.message,
      stack: err.stack,
    });
    console.log(chalk.red('✗'), 'PostgreSQL connection:', err.message);
    results.recommendations.push('CRITICAL: Fix PostgreSQL connection before proceeding');
    return layer;
  }

  // Test 1.2: Migrations applied
  try {
    const { rows } = await pool.query('SELECT COUNT(*) as count FROM _migrations');
    const migrationCount = parseInt(rows[0].count);
    const pass = migrationCount >= 40; // Should have 40+ migrations
    layer.tests.push({
      name: 'Migrations applied',
      pass,
      actual: migrationCount,
      expected: '≥40',
    });
    console.log(
      pass ? chalk.green('✓') : chalk.yellow('⚠'),
      `Migrations applied: ${migrationCount}`,
      pass ? '' : chalk.yellow('(expected ≥40)')
    );
  } catch (err) {
    layer.tests.push({
      name: 'Migrations applied',
      pass: false,
      error: err.message,
    });
    console.log(chalk.red('✗'), 'Migrations check failed:', err.message);
  }

  // Test 1.3: Critical tables exist
  const criticalTables = [
    'judgments',
    'session_patterns',
    'qlearning_state',
    'learning_events',
    'routing_accuracy',
    'watcher_heartbeats',
    'cost_ledger',
  ];

  for (const table of criticalTables) {
    try {
      await pool.query(`SELECT 1 FROM ${table} LIMIT 1`);
      layer.tests.push({
        name: `Table: ${table}`,
        pass: true,
      });
      console.log(chalk.green('✓'), `Table exists: ${table}`);
    } catch (err) {
      layer.tests.push({
        name: `Table: ${table}`,
        pass: false,
        error: err.message,
      });
      console.log(chalk.red('✗'), `Table missing: ${table}`);
      results.recommendations.push(`Run migration to create ${table} table`);
    }
  }

  // Test 1.4: Write + Read integrity
  console.log(chalk.bold('\nTesting write + read integrity...'));

  const writeReadTests = [
    {
      table: 'learning_events',
      write: `INSERT INTO learning_events (loop_type, event_type, metadata) VALUES ($1, $2, $3) RETURNING id`,
      writeParams: ['ralph-test', 'test-event', JSON.stringify({ test: true })],
      read: `SELECT * FROM learning_events WHERE id = $1`,
      cleanup: `DELETE FROM learning_events WHERE loop_type = 'ralph-test'`,
    },
    {
      table: 'routing_accuracy',
      write: `INSERT INTO routing_accuracy (router_type, event_type, confidence, metadata) VALUES ($1, $2, $3, $4) RETURNING id`,
      writeParams: ['ralph-test', 'test', 0.618, JSON.stringify({ test: true })],
      read: `SELECT * FROM routing_accuracy WHERE id = $1`,
      cleanup: `DELETE FROM routing_accuracy WHERE router_type = 'ralph-test'`,
    },
  ];

  for (const test of writeReadTests) {
    try {
      // Write
      const { rows: writeRows } = await pool.query(test.write, test.writeParams);
      const id = writeRows[0].id;

      // Read
      const { rows: readRows } = await pool.query(test.read, [id]);

      // Verify
      const pass = readRows.length === 1 && readRows[0].id === id;

      layer.tests.push({
        name: `Write+Read: ${test.table}`,
        pass,
        id,
      });

      console.log(
        pass ? chalk.green('✓') : chalk.red('✗'),
        `Write+Read: ${test.table}`,
        chalk.gray(`(id=${id})`)
      );

      // Cleanup
      await pool.query(test.cleanup);
    } catch (err) {
      layer.tests.push({
        name: `Write+Read: ${test.table}`,
        pass: false,
        error: err.message,
      });
      console.log(chalk.red('✗'), `Write+Read failed for ${test.table}:`, err.message);
    }
  }

  // Calculate pass rate
  const passed = layer.tests.filter(t => t.pass).length;
  const total = layer.tests.length;
  layer.passRate = (passed / total) * 100;

  console.log(chalk.bold(`\nLayer 1 Pass Rate: ${layer.passRate.toFixed(1)}%`), `(${passed}/${total})`);

  if (layer.passRate < 90) {
    results.recommendations.push('CRITICAL: Database layer below 90% - fix before proceeding');
  }

  return layer;
}

// ============================================================================
// LAYER 2: FACTORIES & PERSISTENCE
// ============================================================================

async function testFactoriesLayer() {
  console.log(chalk.bold.cyan('\n━━━ LAYER 2: FACTORIES & PERSISTENCE ━━━\n'));

  const layer = {
    name: 'factories',
    tests: [],
    passRate: 0,
    critical: true,
  };

  // Test 2.1: Core factories
  const factories = [
    {
      name: 'getPool',
      module: '@cynic/persistence',
      factory: 'getPool',
      expectedMethods: ['query', 'connect'],
    },
    {
      name: 'getLLMRouter',
      module: '../packages/node/src/orchestration/llm-router.js',
      factory: 'getLLMRouter',
      expectedMethods: ['route', 'getStats'],
    },
  ];

  for (const test of factories) {
    try {
      const mod = await import(test.module);
      const instance = mod[test.factory]();

      if (!instance) {
        throw new Error('Factory returned null/undefined');
      }

      // Check expected methods
      const missingMethods = test.expectedMethods.filter(m => typeof instance[m] !== 'function');

      const pass = missingMethods.length === 0;

      layer.tests.push({
        name: `Factory: ${test.name}`,
        pass,
        missingMethods: missingMethods.length > 0 ? missingMethods : undefined,
      });

      console.log(
        pass ? chalk.green('✓') : chalk.yellow('⚠'),
        `Factory: ${test.name}`,
        pass ? '' : chalk.yellow(`(missing: ${missingMethods.join(', ')})`)
      );
    } catch (err) {
      layer.tests.push({
        name: `Factory: ${test.name}`,
        pass: false,
        error: err.message,
        stack: err.stack,
      });
      console.log(chalk.red('✗'), `Factory ${test.name}:`, err.message);
      results.errors.push({
        layer: 'factories',
        test: test.name,
        error: err.message,
        stack: err.stack,
      });
    }
  }

  // Calculate pass rate
  const passed = layer.tests.filter(t => t.pass).length;
  const total = layer.tests.length;
  layer.passRate = (passed / total) * 100;

  console.log(chalk.bold(`\nLayer 2 Pass Rate: ${layer.passRate.toFixed(1)}%`), `(${passed}/${total})`);

  return layer;
}

// ============================================================================
// LAYER 3: SERVICES LOGIC
// ============================================================================

async function testServicesLayer() {
  console.log(chalk.bold.cyan('\n━━━ LAYER 3: SERVICES LOGIC ━━━\n'));

  const layer = {
    name: 'services',
    tests: [],
    passRate: 0,
    critical: false,
  };

  // Test 3.1: LLMRouter routing logic
  try {
    const { getLLMRouter } = await import('../packages/node/src/orchestration/llm-router.js');
    const router = getLLMRouter();

    const result = await router.route({
      type: 'chat',
      complexity: 'simple',
      estimatedTokens: 500,
    });

    const pass = result && result.provider && result.reason;

    layer.tests.push({
      name: 'LLMRouter.route()',
      pass,
      result: pass ? { provider: result.provider, reason: result.reason } : undefined,
    });

    console.log(
      pass ? chalk.green('✓') : chalk.red('✗'),
      'LLMRouter.route()',
      pass ? chalk.gray(`→ ${result.provider} (${result.reason})`) : ''
    );

    if (!pass) {
      results.recommendations.push('Fix LLMRouter.route() - not returning expected result');
    }
  } catch (err) {
    layer.tests.push({
      name: 'LLMRouter.route()',
      pass: false,
      error: err.message,
      stack: err.stack,
    });
    console.log(chalk.red('✗'), 'LLMRouter.route():', err.message);
    results.errors.push({
      layer: 'services',
      test: 'LLMRouter.route()',
      error: err.message,
      stack: err.stack,
    });
  }

  // Test 3.2: Check routing recorded to DB
  try {
    const pool = getPool();
    const { rows } = await pool.query(`
      SELECT COUNT(*) as count
      FROM routing_accuracy
      WHERE router_type = 'llm'
      AND timestamp > NOW() - INTERVAL '5 minutes'
    `);

    const count = parseInt(rows[0].count);
    const pass = count > 0;

    layer.tests.push({
      name: 'LLMRouter DB recording',
      pass,
      count,
    });

    console.log(
      pass ? chalk.green('✓') : chalk.yellow('⚠'),
      'LLMRouter DB recording',
      chalk.gray(`(${count} recent routes)`)
    );

    if (!pass) {
      results.recommendations.push('LLMRouter not recording to routing_accuracy table');
    }
  } catch (err) {
    layer.tests.push({
      name: 'LLMRouter DB recording',
      pass: false,
      error: err.message,
    });
    console.log(chalk.red('✗'), 'LLMRouter DB recording:', err.message);
  }

  // Calculate pass rate
  const passed = layer.tests.filter(t => t.pass).length;
  const total = layer.tests.length;
  layer.passRate = (passed / total) * 100;

  console.log(chalk.bold(`\nLayer 3 Pass Rate: ${layer.passRate.toFixed(1)}%`), `(${passed}/${total})`);

  return layer;
}

// ============================================================================
// LAYER 4: ORCHESTRATION WIRING
// ============================================================================

async function testOrchestrationLayer() {
  console.log(chalk.bold.cyan('\n━━━ LAYER 4: ORCHESTRATION WIRING ━━━\n'));

  const layer = {
    name: 'orchestration',
    tests: [],
    passRate: 0,
    critical: true,
  };

  // Test 4.1: Event bus operational
  try {
    const { globalEventBus } = await import('@cynic/core');

    let received = false;
    const listener = () => { received = true; };

    globalEventBus.on('ralph-test', listener);
    globalEventBus.emit('ralph-test', { test: true });

    await new Promise(resolve => setTimeout(resolve, 100));

    globalEventBus.off('ralph-test', listener);

    layer.tests.push({
      name: 'Event bus operational',
      pass: received,
    });

    console.log(
      received ? chalk.green('✓') : chalk.red('✗'),
      'Event bus operational'
    );

    if (!received) {
      results.recommendations.push('CRITICAL: Event bus not working - core wiring broken');
    }
  } catch (err) {
    layer.tests.push({
      name: 'Event bus operational',
      pass: false,
      error: err.message,
    });
    console.log(chalk.red('✗'), 'Event bus:', err.message);
  }

  // Test 4.2: KabbalisticRouter exists
  try {
    const { KabbalisticRouter } = await import('../packages/node/src/orchestration/kabbalistic-router.js');

    const pass = typeof KabbalisticRouter === 'function';

    layer.tests.push({
      name: 'KabbalisticRouter loadable',
      pass,
    });

    console.log(
      pass ? chalk.green('✓') : chalk.red('✗'),
      'KabbalisticRouter loadable'
    );
  } catch (err) {
    layer.tests.push({
      name: 'KabbalisticRouter loadable',
      pass: false,
      error: err.message,
    });
    console.log(chalk.red('✗'), 'KabbalisticRouter:', err.message);
  }

  // Calculate pass rate
  const passed = layer.tests.filter(t => t.pass).length;
  const total = layer.tests.length;
  layer.passRate = (passed / total) * 100;

  console.log(chalk.bold(`\nLayer 4 Pass Rate: ${layer.passRate.toFixed(1)}%`), `(${passed}/${total})`);

  return layer;
}

// ============================================================================
// LAYER 5: LEARNING VALIDATION
// ============================================================================

async function validateLearning() {
  console.log(chalk.bold.cyan('\n━━━ LAYER 5: LEARNING VALIDATION ━━━\n'));

  const validation = {
    name: 'learning',
    experiments: [],
    convergence: null,
  };

  console.log(chalk.yellow('⚠'), 'Learning validation requires running daemon');
  console.log(chalk.gray('  Checking for existing learning data...\n'));

  const pool = getPool();

  // Check learning_events
  try {
    const { rows } = await pool.query(`
      SELECT loop_type, COUNT(*) as count
      FROM learning_events
      WHERE timestamp > NOW() - INTERVAL '7 days'
      GROUP BY loop_type
    `);

    validation.experiments.push({
      name: 'Learning events logged',
      pass: rows.length > 0,
      loopTypes: rows.length,
      events: rows.reduce((sum, r) => sum + parseInt(r.count), 0),
    });

    console.log(
      rows.length > 0 ? chalk.green('✓') : chalk.yellow('⚠'),
      'Learning events:',
      `${rows.length} loop types,`,
      `${rows.reduce((sum, r) => sum + parseInt(r.count), 0)} events`
    );

    if (rows.length === 0) {
      results.recommendations.push('No learning events - daemon not generating episodes');
    }
  } catch (err) {
    validation.experiments.push({
      name: 'Learning events logged',
      pass: false,
      error: err.message,
    });
    console.log(chalk.red('✗'), 'Learning events check:', err.message);
  }

  // Check Q-learning updates
  try {
    const { rows } = await pool.query(`
      SELECT COUNT(*) as count
      FROM learning_events
      WHERE loop_type = 'q-learning'
      AND timestamp > NOW() - INTERVAL '24 hours'
    `);

    const count = parseInt(rows[0].count);
    const pass = count >= 10; // G1.3 target

    validation.experiments.push({
      name: 'Q-Learning updates (24h)',
      pass,
      actual: count,
      expected: '≥10',
    });

    console.log(
      pass ? chalk.green('✓') : chalk.yellow('⚠'),
      'Q-Learning updates:',
      count,
      pass ? chalk.green('(G1.3 PASS)') : chalk.yellow('(G1.3 FAIL)')
    );

    if (!pass) {
      results.recommendations.push('G1.3 FAIL: Need ≥10 Q-weight updates/day');
    }
  } catch (err) {
    validation.experiments.push({
      name: 'Q-Learning updates (24h)',
      pass: false,
      error: err.message,
    });
  }

  return validation;
}

// ============================================================================
// LAYER 6: WEEK 1 GOALS
// ============================================================================

async function testWeek1Goals() {
  console.log(chalk.bold.cyan('\n━━━ LAYER 6: WEEK 1 GOALS ━━━\n'));

  const goals = {
    name: 'week1',
    tests: [],
    passCount: 0,
    totalCount: 5,
  };

  const pool = getPool();

  const goalTests = [
    {
      id: 'G1.1',
      name: 'Watchers polling',
      query: `SELECT COUNT(DISTINCT watcher_name) as count FROM watcher_heartbeats WHERE timestamp > NOW() - INTERVAL '1 hour'`,
      target: 3,
    },
    {
      id: 'G1.2',
      name: 'Learning loops consuming',
      query: `SELECT COUNT(DISTINCT loop_type) as count FROM learning_events WHERE timestamp > NOW() - INTERVAL '24 hours'`,
      target: 5,
    },
    {
      id: 'G1.3',
      name: 'Q-weight updates/day',
      query: `SELECT COUNT(*) as count FROM learning_events WHERE loop_type = 'q-learning' AND timestamp > NOW() - INTERVAL '24 hours'`,
      target: 10,
    },
    {
      id: 'G1.4',
      name: 'KabbalisticRouter calls',
      query: `SELECT COUNT(*) as count FROM routing_accuracy WHERE router_type = 'kabbalistic' AND timestamp > NOW() - INTERVAL '24 hours'`,
      target: 20,
    },
    {
      id: 'G1.5',
      name: 'LLMRouter non-Anthropic',
      query: `SELECT COUNT(*) as count FROM routing_accuracy WHERE router_type = 'llm' AND metadata->>'provider' = 'ollama' AND timestamp > NOW() - INTERVAL '24 hours'`,
      target: 10,
    },
  ];

  for (const goal of goalTests) {
    try {
      const { rows } = await pool.query(goal.query);
      const actual = parseInt(rows[0].count);
      const pass = actual >= goal.target;

      if (pass) goals.passCount++;

      goals.tests.push({
        id: goal.id,
        name: goal.name,
        pass,
        actual,
        target: goal.target,
      });

      console.log(
        pass ? chalk.green('✓') : chalk.red('✗'),
        `${goal.id}: ${goal.name}`,
        chalk.gray(`${actual}/${goal.target}`),
        pass ? chalk.green('PASS') : chalk.red('FAIL')
      );

      if (!pass) {
        results.recommendations.push(`${goal.id} FAIL: ${goal.name} (${actual}/${goal.target})`);
      }
    } catch (err) {
      goals.tests.push({
        id: goal.id,
        name: goal.name,
        pass: false,
        error: err.message,
      });
      console.log(chalk.red('✗'), `${goal.id}:`, err.message);
    }
  }

  const passRate = (goals.passCount / goals.totalCount) * 100;
  console.log(chalk.bold(`\nWeek 1 Pass Rate: ${passRate.toFixed(1)}%`), `(${goals.passCount}/${goals.totalCount})`);

  if (goals.passCount < 4) {
    results.recommendations.push(`Week 1 FAIL: Only ${goals.passCount}/5 goals passing (need 4/5)`);
  }

  return goals;
}

// ============================================================================
// MAIN TEST RUNNER
// ============================================================================

async function ralphComprehensiveTest() {
  console.log(chalk.bold.magenta('\n╔══════════════════════════════════════════════════════════════╗'));
  console.log(chalk.bold.magenta('║') + chalk.bold.white('          🐕 RALPH COMPREHENSIVE PRODUCTION TEST              ') + chalk.bold.magenta('║'));
  console.log(chalk.bold.magenta('╠══════════════════════════════════════════════════════════════╣'));
  console.log(chalk.bold.magenta('║') + chalk.gray('  "Le chien teste tout, remonte tout" - κυνικός             ') + chalk.bold.magenta('║'));
  console.log(chalk.bold.magenta('╚══════════════════════════════════════════════════════════════╝\n'));

  try {
    // Layer 1: Database
    results.layers.database = await testDatabaseLayer();

    if (results.layers.database.passRate < 90) {
      console.log(chalk.red.bold('\n⛔ CRITICAL: Database layer below 90% - STOPPING'));
      await generateReport();
      process.exit(1);
    }

    // Layer 2: Factories
    results.layers.factories = await testFactoriesLayer();

    // Layer 3: Services
    results.layers.services = await testServicesLayer();

    // Layer 4: Orchestration
    results.layers.orchestration = await testOrchestrationLayer();

    // Layer 5: Learning
    results.scientific.learning = await validateLearning();

    // Layer 6: Week 1 Goals
    results.scientific.week1 = await testWeek1Goals();

    // Generate final report
    await generateReport();

    // Exit code based on critical failures
    const criticalFailed = Object.values(results.layers)
      .filter(l => l.critical && l.passRate < 80)
      .length;

    process.exit(criticalFailed > 0 ? 1 : 0);
  } catch (err) {
    console.error(chalk.red.bold('\n⛔ FATAL ERROR:'), err.message);
    console.error(chalk.gray(err.stack));
    results.errors.push({
      fatal: true,
      error: err.message,
      stack: err.stack,
    });
    await generateReport();
    process.exit(2);
  }
}

// ============================================================================
// REPORT GENERATION
// ============================================================================

async function generateReport() {
  const duration = Date.now() - results.startTime;

  console.log(chalk.bold.cyan('\n━━━ FINAL REPORT ━━━\n'));

  // Summary
  console.log(chalk.bold('Test Duration:'), chalk.gray(`${(duration / 1000).toFixed(1)}s\n`));

  // Layer summary
  console.log(chalk.bold('Layer Results:'));
  for (const [name, layer] of Object.entries(results.layers)) {
    const status = layer.passRate >= 90 ? chalk.green('✓') :
                   layer.passRate >= 70 ? chalk.yellow('⚠') :
                   chalk.red('✗');
    console.log(`  ${status} ${name}:`, chalk.bold(`${layer.passRate.toFixed(1)}%`));
  }

  // Week 1 Goals
  if (results.scientific.week1) {
    const { passCount, totalCount } = results.scientific.week1;
    const status = passCount >= 4 ? chalk.green('✓') : chalk.red('✗');
    console.log(`  ${status} Week 1 Goals:`, chalk.bold(`${passCount}/${totalCount}`));
  }

  // Recommendations
  if (results.recommendations.length > 0) {
    console.log(chalk.bold.yellow('\n📋 Recommendations:\n'));
    results.recommendations.slice(0, 10).forEach((rec, i) => {
      console.log(chalk.yellow(`  ${i + 1}.`), rec);
    });
    if (results.recommendations.length > 10) {
      console.log(chalk.gray(`  ... ${results.recommendations.length - 10} more`));
    }
  }

  // Errors
  if (results.errors.length > 0) {
    console.log(chalk.bold.red('\n🔥 Errors:\n'));
    results.errors.slice(0, 5).forEach((err, i) => {
      console.log(chalk.red(`  ${i + 1}.`), `[${err.layer || 'unknown'}/${err.test || 'unknown'}]`, err.error);
    });
    if (results.errors.length > 5) {
      console.log(chalk.gray(`  ... ${results.errors.length - 5} more errors`));
    }
  }

  console.log(chalk.bold.cyan('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n'));
  console.log(chalk.gray('*sniff* Confidence: 58% (φ⁻¹ limit - tests run, production validation pending)\n'));

  // Write JSON report
  const fs = await import('fs');
  const reportPath = 'scripts/ralph-report.json';
  fs.writeFileSync(reportPath, JSON.stringify(results, null, 2));
  console.log(chalk.gray(`Full report saved to: ${reportPath}\n`));
}

// ============================================================================
// RUN
// ============================================================================

if (import.meta.url === `file:///${process.argv[1].replace(/\\/g, '/')}`) {
  ralphComprehensiveTest().catch(err => {
    console.error(chalk.red('Test runner failed:'), err);
    process.exit(2);
  });
}

export { ralphComprehensiveTest };

#!/usr/bin/env node
/**
 * Week 1 Progress Dashboard
 *
 * Displays data-driven metrics for Week 1 goals:
 * - G1.1: Watchers polling (≥3 active)
 * - G1.2: Learning loops consuming data (≥5 loop types)
 * - G1.3: Q-weights updating (≥10 updates/day)
 * - G1.4: KabbalisticRouter active (≥20 calls)
 * - G1.5: LLMRouter routing (≥10 non-Anthropic routes)
 *
 * Success criteria: 4/5 goals PASS
 *
 * Usage: npm run metrics:week1
 *        cynic week1
 */

'use strict';

import pg from 'pg';
const { Pool } = pg;

import { createLogger } from '@cynic/core';
const log = createLogger('Week1Dashboard');

const PHI_INV = 0.618;

// Database connection
const pool = new Pool({
  host: process.env.POSTGRES_HOST || 'localhost',
  port: parseInt(process.env.POSTGRES_PORT || '5432'),
  database: process.env.POSTGRES_DB || 'cynic',
  user: process.env.POSTGRES_USER || 'cynic',
  password: process.env.POSTGRES_PASSWORD || 'cynic',
});

/**
 * Draw ASCII box
 */
function box(title, content, width = 61) {
  const lines = [];
  lines.push('┌' + '─'.repeat(width - 2) + '┐');

  if (title) {
    const padding = Math.floor((width - title.length - 2) / 2);
    lines.push('│' + ' '.repeat(padding) + title + ' '.repeat(width - padding - title.length - 2) + '│');
    lines.push('├' + '─'.repeat(width - 2) + '┤');
  }

  content.forEach(line => {
    const padded = line + ' '.repeat(width - line.length - 2);
    lines.push('│ ' + padded + '│');
  });

  lines.push('└' + '─'.repeat(width - 2) + '┘');
  return lines.join('\n');
}

/**
 * Get Week 1 progress from database
 */
async function getWeek1Progress() {
  try {
    const { rows } = await pool.query('SELECT * FROM get_week1_progress()');
    return rows;
  } catch (error) {
    if (error.message.includes('does not exist')) {
      return null; // Function not yet created
    }
    throw error;
  }
}

/**
 * Get detailed metrics
 */
async function getDetailedMetrics() {
  const metrics = {};

  // Watchers
  try {
    const { rows } = await pool.query(`
      SELECT DISTINCT watcher_name
      FROM watcher_heartbeats
      WHERE timestamp > NOW() - INTERVAL '1 hour'
    `);
    metrics.watchers = rows.map(r => r.watcher_name);
  } catch {
    metrics.watchers = [];
  }

  // Learning loops
  try {
    const { rows } = await pool.query(`
      SELECT DISTINCT service_id
      FROM learning_metrics
      WHERE last_updated > '2026-02-12'
    `);
    metrics.learningLoops = rows.map(r => r.service_id);
  } catch {
    metrics.learningLoops = [];
  }

  // Q-updates
  try {
    const { rows } = await pool.query(`
      SELECT COUNT(*) as count
      FROM qlearning_state
      WHERE updated_at > NOW() - INTERVAL '1 day'
    `);
    metrics.qUpdates = parseInt(rows[0]?.count || 0);
  } catch {
    metrics.qUpdates = 0;
  }

  // Router usage
  try {
    const { rows } = await pool.query(`
      SELECT
        router_type,
        COUNT(*) as count
      FROM router_usage
      WHERE timestamp > '2026-02-12'
      GROUP BY router_type
    `);
    metrics.routerUsage = {};
    rows.forEach(r => {
      metrics.routerUsage[r.router_type] = parseInt(r.count);
    });
  } catch {
    metrics.routerUsage = {};
  }

  // LLM routing
  try {
    const { rows } = await pool.query(`
      SELECT COUNT(*) as count
      FROM llm_usage
      WHERE adapter != 'anthropic'
      AND created_at > '2026-02-12'
    `);
    metrics.llmRoutes = parseInt(rows[0]?.count || 0);
  } catch {
    metrics.llmRoutes = 0;
  }

  return metrics;
}

/**
 * Display Week 1 dashboard
 */
async function displayDashboard() {
  console.log('\n🐕 Week 1 Progress Dashboard');
  console.log('Data-Driven Roadmap (2026-02-19 target)\n');

  // Check database connectivity
  let connected = false;
  try {
    await pool.query('SELECT NOW()');
    connected = true;
  } catch (error) {
    console.log('❌ Database not available');
    console.log(`   Error: ${error.message}\n`);
    console.log(box('Week 1: BLOCKED', [
      'PostgreSQL not running or tables not created.',
      '',
      'Next steps:',
      '1. Start PostgreSQL',
      '2. Run migrations (npm run db:migrate)',
      '3. Start CYNIC daemon (npm run daemon)',
    ]));
    return;
  }

  // Get progress
  const progress = await getWeek1Progress();
  const details = await getDetailedMetrics();

  if (!progress) {
    console.log('⚠️ Metrics infrastructure not yet deployed\n');
    console.log(box('Week 1: SETUP NEEDED', [
      'Migration 047 not yet applied.',
      '',
      'Run: npm run db:migrate',
      '',
      'This will create:',
      '- watcher_heartbeats table',
      '- routing_accuracy table',
      '- router_usage table',
      '- get_week1_progress() function',
    ]));
    return;
  }

  // Draw goals table
  console.log('┌──────────────────────┬──────────┬──────────┬──────────┬──────────────┐');
  console.log('│ Goal                 │ Target   │ Actual   │ Status   │ Gap          │');
  console.log('├──────────────────────┼──────────┼──────────┼──────────┼──────────────┤');

  let passCount = 0;
  const totalGoals = progress.length;

  progress.forEach(goal => {
    if (goal.pass) passCount++;

    // Format row
    const goalName = goal.goal.padEnd(20);
    const target = goal.target.padEnd(8);
    const actual = goal.actual.padEnd(8);
    const status = goal.status.padEnd(8);

    // Calculate gap
    let gap = '';
    if (!goal.pass) {
      if (goal.goal.includes('Watchers')) {
        const current = details.watchers.length;
        gap = `+${3 - current} watcher${3 - current > 1 ? 's' : ''}`;
      } else if (goal.goal.includes('Learning')) {
        const current = details.learningLoops.length;
        gap = `+${5 - current} loop${5 - current > 1 ? 's' : ''}`;
      } else if (goal.goal.includes('Q-weights')) {
        gap = `+${10 - details.qUpdates} updates`;
      } else if (goal.goal.includes('Kabbalistic')) {
        const current = details.routerUsage.kabbalistic || 0;
        gap = `+${20 - current} calls`;
      } else if (goal.goal.includes('LLM')) {
        gap = `+${10 - details.llmRoutes} routes`;
      }
    } else {
      gap = '✓ met';
    }

    console.log(`│ ${goalName} │ ${target} │ ${actual} │ ${status} │ ${gap.padEnd(12)} │`);
  });

  console.log('├──────────────────────┴──────────┴──────────┴──────────┴──────────────┤');

  // Week 1 status
  const statusLine = `Week 1 Status: ${passCount}/${totalGoals} PASS (needs 4/5)`;
  const statusPadded = statusLine.padEnd(69);
  console.log(`│ ${statusPadded} │`);

  console.log('└─────────────────────────────────────────────────────────────────────┘');

  // Details section
  console.log('\n' + box('DETAILED METRICS', [
    '',
    `Watchers Active (${details.watchers.length}/4):`,
    details.watchers.length > 0
      ? details.watchers.map(w => `  • ${w}`).join('\n')
      : '  (none)',
    '',
    `Learning Loops (${details.learningLoops.length}/11):`,
    details.learningLoops.length > 0
      ? details.learningLoops.slice(0, 5).map(l => `  • ${l}`).join('\n')
      : '  (none)',
    details.learningLoops.length > 5 ? `  ... +${details.learningLoops.length - 5} more` : '',
    '',
    `Router Usage:`,
    `  • Kabbalistic: ${details.routerUsage.kabbalistic || 0} calls`,
    `  • LLM: ${details.routerUsage.llm || 0} calls`,
    `  • Fast: ${details.routerUsage.fast || 0} calls`,
    '',
    `Q-Learning:`,
    `  • Updates (24h): ${details.qUpdates}`,
    `  • LLM routes (non-Anthropic): ${details.llmRoutes}`,
  ].filter(l => l !== '')));

  // Recommendations
  console.log();
  if (passCount >= 4) {
    console.log(box('🎉 WEEK 1: ON TRACK', [
      '',
      `${passCount}/5 goals met (target: 4/5)`,
      '',
      'CYNIC nervous system is wiring up.',
      'Continue daemon operation.',
      'Monitor daily progress.',
    ]));
  } else if (passCount >= 2) {
    console.log(box('⚡ WEEK 1: IN PROGRESS', [
      '',
      `${passCount}/5 goals met (target: 4/5)`,
      '',
      'Next actions:',
      details.watchers.length < 3 ? '• Start more watchers (FileWatcher, SolanaWatcher, MarketWatcher)' : '',
      details.learningLoops.length < 5 ? '• Wire learning feedback loops' : '',
      details.qUpdates < 10 ? '• Generate Q-learning activity (daemon sessions)' : '',
      (details.routerUsage.kabbalistic || 0) < 20 ? '• Activate KabbalisticRouter in daemon' : '',
      details.llmRoutes < 10 ? '• Enable LLMRouter with Ollama' : '',
    ].filter(l => l !== '')));
  } else {
    console.log(box('🚧 WEEK 1: EARLY STAGE', [
      '',
      `${passCount}/5 goals met (target: 4/5)`,
      '',
      'Critical path:',
      '1. Run migrations (npm run db:migrate)',
      '2. Start daemon (npm run daemon)',
      '3. Let watchers poll for 1 hour',
      '4. Re-run this dashboard',
      '',
      'Expected: 2-3 goals PASS after 1h daemon uptime',
    ]));
  }

  console.log();
  console.log('*sniff* Confidence: 58% (φ⁻¹ limit)');
  console.log();
}

// Run dashboard
displayDashboard()
  .catch(error => {
    console.error('❌ Dashboard error:', error.message);
    log.error('Dashboard failed', { error });
    process.exit(1);
  })
  .finally(() => {
    pool.end();
  });

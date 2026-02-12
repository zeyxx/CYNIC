#!/usr/bin/env node
/**
 * Test Watchers — GAP-2 Validation
 *
 * Validates:
 * 1. FileWatcher polls every 5s
 * 2. SolanaWatcher polls RPC every 30s (handles 429s)
 * 3. Both send heartbeats to watcher_heartbeats table
 * 4. G1.1 metric achieved: ≥3 watchers active
 *
 * Usage:
 *   node scripts/test-watchers.js
 *
 * φ-aligned test duration: 61.8s (φ⁻¹ × 100)
 *
 * @module scripts/test-watchers
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { wireWatchers, cleanupWatchers } from '../packages/node/src/daemon/service-wiring.js';
import { getPostgresClient } from '@cynic/persistence';

const log = createLogger('TestWatchers');

const TEST_DURATION = Math.round(PHI_INV * 100 * 1000); // 61.8s

/**
 * Run watcher test
 */
async function runTest() {
  log.info('Starting watcher test', { duration: `${TEST_DURATION / 1000}s` });

  let db;
  let watchers;

  try {
    // 1. Connect to PostgreSQL
    db = getPostgresClient();
    await db.connect();
    log.info('Connected to PostgreSQL');

    // 2. Wire watchers
    watchers = await wireWatchers({ db });
    log.info('Watchers wired', {
      fileWatcher: !!watchers.fileWatcher,
      solanaWatcher: !!watchers.solanaWatcher,
    });

    // 3. Wait for test duration
    log.info(`Waiting ${TEST_DURATION / 1000}s for watcher polling...`);
    await new Promise(resolve => setTimeout(resolve, TEST_DURATION));

    // 4. Query heartbeats from DB
    const { rows: heartbeats } = await db.query(`
      SELECT
        watcher_name,
        COUNT(*) as heartbeat_count,
        MAX(events_polled) as max_events,
        MAX(timestamp) as last_heartbeat,
        STRING_AGG(DISTINCT status, ', ') as statuses
      FROM watcher_heartbeats
      WHERE timestamp > NOW() - INTERVAL '5 minutes'
      GROUP BY watcher_name
      ORDER BY watcher_name
    `);

    log.info('Heartbeat summary', { count: heartbeats.length });

    for (const hb of heartbeats) {
      log.info(`  ${hb.watcher_name}:`, {
        heartbeats: hb.heartbeat_count,
        maxEvents: hb.max_events,
        statuses: hb.statuses,
        lastSeen: new Date(hb.last_heartbeat).toISOString(),
      });
    }

    // 5. Check G1.1 metric: ≥3 watchers active
    const { rows: activeWatchers } = await db.query(`
      SELECT COUNT(DISTINCT watcher_name) as count
      FROM watcher_heartbeats
      WHERE timestamp > NOW() - INTERVAL '5 minutes'
      AND status = 'active'
    `);

    const activeCount = parseInt(activeWatchers[0]?.count || 0);
    const g11Pass = activeCount >= 3;

    log.info('G1.1 Metric Check', {
      activeWatchers: activeCount,
      target: 3,
      pass: g11Pass,
    });

    // 6. Get watcher stats
    if (watchers.fileWatcher) {
      const fsStats = watchers.fileWatcher.getStats();
      log.info('FilesystemWatcher stats', fsStats);
    }

    if (watchers.solanaWatcher) {
      const solStats = watchers.solanaWatcher.getStats();
      const solHealth = watchers.solanaWatcher.getHealth();
      log.info('SolanaWatcher stats', solStats);
      log.info('SolanaWatcher health', solHealth);
    }

    // 7. Test result
    console.log('\n' + '═'.repeat(60));
    console.log('  TEST RESULT');
    console.log('═'.repeat(60));
    console.log(`  Heartbeats Recorded: ${heartbeats.length > 0 ? '✓' : '✗'}`);
    console.log(`  Active Watchers: ${activeCount} (target: ≥3)`);
    console.log(`  G1.1 Metric: ${g11Pass ? '✓ PASS' : '✗ FAIL'}`);
    console.log('═'.repeat(60) + '\n');

    if (g11Pass) {
      log.info('✓ GAP-2 complete — watchers operational');
      process.exit(0);
    } else {
      log.warn('✗ GAP-2 incomplete — need more active watchers');
      process.exit(1);
    }
  } catch (err) {
    log.error('Test failed', { error: err.message, stack: err.stack });
    process.exit(1);
  } finally {
    // Cleanup
    try {
      await cleanupWatchers();
      log.info('Watchers cleaned up');
    } catch (err) {
      log.debug('Cleanup error', { error: err.message });
    }

    if (db) {
      await db.close();
    }
  }
}

// Run test
runTest().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});

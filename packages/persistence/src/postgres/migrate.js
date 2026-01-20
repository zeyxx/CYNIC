#!/usr/bin/env node
/**
 * Database Migration Runner
 *
 * Run: npm run migrate
 *
 * @module @cynic/persistence/postgres/migrate
 */

'use strict';

import 'dotenv/config';
import { readFileSync, readdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { PostgresClient } from './client.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const MIGRATIONS_DIR = join(__dirname, 'migrations');

/**
 * Repair partially applied migrations by marking them as done
 * This handles cases where migrations were partially applied but not tracked
 */
async function repairPartialMigrations(db, log) {
  // Map of migration name -> table that should exist if it was applied
  const migrationChecks = {
    '003_ecosystem_docs': 'ecosystem_docs',
    '004_solana_anchoring': 'anchor_batches',
    '005_learning': 'learning_state',
    '006_triggers': 'triggers_registry',
    '007_discovery': 'discovered_mcp_servers',
  };

  for (const [migrationName, tableName] of Object.entries(migrationChecks)) {
    try {
      // Check if table exists
      const { rows } = await db.query(`
        SELECT EXISTS (
          SELECT FROM information_schema.tables
          WHERE table_schema = 'public' AND table_name = $1
        ) as exists
      `, [tableName]);

      if (rows[0]?.exists) {
        // Table exists, mark migration as applied if not already
        await db.query(
          'INSERT INTO _migrations (name) VALUES ($1) ON CONFLICT (name) DO NOTHING',
          [migrationName]
        );
        log(`🔧 Repaired: ${migrationName} (table ${tableName} exists)`);
      }
    } catch (err) {
      // Ignore errors during repair
    }
  }
}

/**
 * Run database migrations
 * @param {Object} options - Migration options
 * @param {boolean} options.silent - Suppress console output (default: false)
 * @param {boolean} options.exitOnError - Exit process on error (default: true for CLI, false for programmatic)
 * @returns {Promise<{applied: number, skipped: number, total: number}>} Migration results
 */
export async function migrate(options = {}) {
  const { silent = false, exitOnError = false } = options;
  const log = silent ? () => {} : console.log.bind(console);
  const logError = silent ? () => {} : console.error.bind(console);

  log('🐕 CYNIC Database Migration');
  log('============================\n');

  const db = new PostgresClient();
  const result = { applied: 0, skipped: 0, total: 0 };

  try {
    await db.connect();

    // Ensure migrations table exists
    await db.query(`
      CREATE TABLE IF NOT EXISTS _migrations (
        id SERIAL PRIMARY KEY,
        name VARCHAR(100) UNIQUE NOT NULL,
        applied_at TIMESTAMPTZ DEFAULT NOW()
      )
    `);

    // Repair any partially applied migrations first
    await repairPartialMigrations(db, log);

    // Get applied migrations (after repair)
    const { rows: applied } = await db.query('SELECT name FROM _migrations');
    const appliedSet = new Set(applied.map(r => r.name));

    // Get migration files
    const files = readdirSync(MIGRATIONS_DIR)
      .filter(f => f.endsWith('.sql'))
      .sort();

    result.total = files.length;
    log(`Found ${files.length} migration(s)\n`);

    for (const file of files) {
      const name = file.replace('.sql', '');

      if (appliedSet.has(name)) {
        log(`⏭️  ${name} (already applied)`);
        result.skipped++;
        continue;
      }

      log(`▶️  Applying ${name}...`);

      const sql = readFileSync(join(MIGRATIONS_DIR, file), 'utf8');

      await db.transaction(async (client) => {
        await client.query(sql);
        await client.query(
          'INSERT INTO _migrations (name) VALUES ($1) ON CONFLICT DO NOTHING',
          [name]
        );
      });

      log(`✅ ${name} applied`);
      result.applied++;
    }

    log(`\n============================`);
    log(`✅ Applied: ${result.applied}`);
    log(`⏭️  Skipped: ${result.skipped}`);
    log('🐕 Migration complete\n');

    return result;

  } catch (error) {
    logError('❌ Migration failed:', error.message);
    if (exitOnError) {
      process.exit(1);
    }
    throw error;
  } finally {
    await db.close();
  }
}

// Run if called directly (CLI mode)
const isMain = process.argv[1]?.includes('migrate');
if (isMain) {
  migrate({ exitOnError: true }).catch(err => {
    console.error(err);
    process.exit(1);
  });
}

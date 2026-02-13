#!/usr/bin/env node
import { getPool } from '@cynic/persistence';
import fs from 'fs';
import path from 'path';

async function applyMigration() {
  try {
    const pool = getPool();
    const migrationPath = path.join(process.cwd(), 'packages/persistence/src/postgres/migrations/046_cost_ledger.sql');
    const sql = fs.readFileSync(migrationPath, 'utf8');

    console.log('Applying migration 046_cost_ledger...');
    await pool.query(sql);
    console.log('✅ Migration applied successfully');

    // Verify the tables exist
    const result = await pool.query(`
      SELECT table_name
      FROM information_schema.tables
      WHERE table_schema = 'public'
      AND table_name IN ('cost_ledger', 'budget_state', 'budget_alerts')
      ORDER BY table_name
    `);

    console.log('\n✅ Tables created:');
    result.rows.forEach(row => console.log(`  - ${row.table_name}`));

    process.exit(0);
  } catch (err) {
    console.error('❌ Error applying migration:', err.message);
    console.error(err.stack);
    process.exit(1);
  }
}

applyMigration();

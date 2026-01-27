#!/usr/bin/env node
/**
 * Apply a single migration to PostgreSQL
 *
 * Usage: DATABASE_URL="..." node scripts/apply-migration.cjs <migration-file>
 *
 * Example:
 *   DATABASE_URL="postgresql://user:pass@host/db" node scripts/apply-migration.cjs 013_fix_username_unique.sql
 */

const { Client } = require('pg');
const fs = require('fs');
const path = require('path');

async function applyMigration() {
  const migrationName = process.argv[2];

  if (!migrationName) {
    console.error('Usage: DATABASE_URL="..." node scripts/apply-migration.cjs <migration-file>');
    console.error('Example: DATABASE_URL="postgresql://..." node scripts/apply-migration.cjs 013_fix_username_unique.sql');
    process.exit(1);
  }

  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl) {
    console.error('DATABASE_URL environment variable is required');
    process.exit(1);
  }

  // Find migration file
  const migrationsDir = path.join(__dirname, '..', 'packages', 'persistence', 'src', 'postgres', 'migrations');
  const migrationPath = path.join(migrationsDir, migrationName);

  if (!fs.existsSync(migrationPath)) {
    console.error(`Migration file not found: ${migrationPath}`);
    process.exit(1);
  }

  const sql = fs.readFileSync(migrationPath, 'utf-8');

  console.log(`Applying migration: ${migrationName}`);
  console.log(`Database: ${databaseUrl.replace(/:[^:@]+@/, ':***@')}`);

  const client = new Client({
    connectionString: databaseUrl,
    ssl: { rejectUnauthorized: false }
  });

  try {
    await client.connect();
    console.log('Connected to PostgreSQL');

    await client.query(sql);
    console.log('Migration applied successfully!');

  } catch (error) {
    console.error('Migration failed:', error.message);
    process.exit(1);
  } finally {
    await client.end();
  }
}

applyMigration();

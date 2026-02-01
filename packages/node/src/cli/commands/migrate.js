/**
 * cynic migrate - Database Migrations
 *
 * Run database migrations for CYNIC persistence layer.
 *
 * Usage:
 *   cynic migrate           Run pending migrations
 *   cynic migrate --status  Show migration status only
 *   cynic migrate --reset   Reset all migrations (dangerous!)
 *
 * @module @cynic/node/cli/commands/migrate
 */

'use strict';

import chalk from 'chalk';

const PHI_INV = 0.618033988749895;

/**
 * Check migration status
 */
async function checkMigrationStatus(db) {
  try {
    const { rows } = await db.query(`
      SELECT name, applied_at
      FROM _migrations
      ORDER BY applied_at ASC
    `);
    return rows;
  } catch (error) {
    // Table might not exist yet
    return [];
  }
}

/**
 * Get available migration files
 */
async function getAvailableMigrations() {
  try {
    const { readdirSync } = await import('fs');
    const { join, dirname } = await import('path');
    const { fileURLToPath } = await import('url');

    // Try to find migrations directory
    const possiblePaths = [
      join(process.cwd(), 'packages/persistence/src/postgres/migrations'),
      join(process.cwd(), 'node_modules/@cynic/persistence/src/postgres/migrations'),
    ];

    for (const path of possiblePaths) {
      try {
        const files = readdirSync(path).filter(f => f.endsWith('.sql')).sort();
        return { path, files };
      } catch {
        continue;
      }
    }

    return { path: null, files: [] };
  } catch (error) {
    return { path: null, files: [] };
  }
}

/**
 * Display migration status
 */
async function showStatus(verbose) {
  console.log(chalk.gray('  Checking migration status...\n'));

  let db;
  try {
    const { PostgresClient } = await import('@cynic/persistence');
    db = new PostgresClient();
    await db.connect();
  } catch (error) {
    console.log(chalk.red('  ✗ Cannot connect to database'));
    console.log(chalk.gray(`    ${error.message}`));
    console.log(chalk.gray('\n  Set CYNIC_DB_HOST or DATABASE_URL'));
    return false;
  }

  try {
    const applied = await checkMigrationStatus(db);
    const { path, files } = await getAvailableMigrations();

    const appliedSet = new Set(applied.map(m => m.name));
    const pending = files.filter(f => !appliedSet.has(f.replace('.sql', '')));

    console.log(chalk.bold('  Applied migrations:'));
    if (applied.length === 0) {
      console.log(chalk.gray('    (none)'));
    } else {
      for (const migration of applied) {
        const date = new Date(migration.applied_at).toLocaleDateString();
        console.log(chalk.green(`    ✓ ${migration.name}`) + chalk.gray(` (${date})`));
      }
    }

    console.log();
    console.log(chalk.bold('  Pending migrations:'));
    if (pending.length === 0) {
      console.log(chalk.gray('    (none)'));
    } else {
      for (const file of pending) {
        console.log(chalk.yellow(`    ○ ${file.replace('.sql', '')}`));
      }
    }

    console.log();
    console.log(chalk.gray(`  Total: ${applied.length} applied, ${pending.length} pending`));

    if (path && verbose) {
      console.log(chalk.gray(`  Path: ${path}`));
    }

    return pending.length === 0;
  } finally {
    await db.close();
  }
}

/**
 * Run migrations using @cynic/persistence
 */
async function runMigrations(silent = false) {
  try {
    const { migrate } = await import('@cynic/persistence');
    const result = await migrate({ silent, exitOnError: false });
    return result;
  } catch (error) {
    // Try direct import if package import fails
    try {
      const { migrate } = await import('../../../../../../persistence/src/postgres/migrate.js');
      const result = await migrate({ silent, exitOnError: false });
      return result;
    } catch {
      throw error;
    }
  }
}

/**
 * Reset all migrations (dangerous!)
 */
async function resetMigrations() {
  console.log(chalk.red('\n  ⚠️  WARNING: This will drop all CYNIC tables!'));
  console.log(chalk.gray('  This action cannot be undone.\n'));

  // In non-interactive mode, just warn and exit
  console.log(chalk.yellow('  To confirm, run with --yes flag'));
  return false;
}

/**
 * Migrate command handler
 */
export async function migrateCommand(options) {
  const { status = false, reset = false, yes = false, verbose = false } = options;

  console.log(chalk.yellow('\n╔═════════════════════════════════════════╗'));
  console.log(chalk.yellow('║') + chalk.bold.cyan('  CYNIC Database Migrations             ') + chalk.yellow('║'));
  console.log(chalk.yellow('╚═════════════════════════════════════════╝\n'));

  // Status-only mode
  if (status) {
    const upToDate = await showStatus(verbose);
    if (upToDate) {
      console.log(chalk.green('\n  *tail wag* Database is up to date!\n'));
    } else {
      console.log(chalk.yellow('\n  *sniff* Pending migrations found'));
      console.log(chalk.gray('  Run: cynic migrate\n'));
    }
    return;
  }

  // Reset mode (dangerous)
  if (reset) {
    if (!yes) {
      await resetMigrations();
      process.exit(1);
    }
    console.log(chalk.red('  Reset not yet implemented for safety\n'));
    process.exit(1);
  }

  // Run migrations
  console.log(chalk.gray('  *sniff* Running migrations...\n'));

  try {
    const result = await runMigrations(false);

    console.log(chalk.yellow('\n═════════════════════════════════════════'));

    if (result.applied > 0) {
      console.log(chalk.green(`\n  *tail wag* Applied ${result.applied} migration(s)`));
    } else {
      console.log(chalk.green('\n  *tail wag* Database already up to date'));
    }

    console.log(chalk.cyan(`  φ⁻¹ = ${(PHI_INV * 100).toFixed(1)}% max confidence\n`));

  } catch (error) {
    console.log(chalk.red('\n  *GROWL* Migration failed!'));
    console.log(chalk.gray(`  ${error.message}`));

    if (verbose) {
      console.log(chalk.gray(error.stack));
    }

    process.exit(1);
  }
}

export default { migrateCommand };

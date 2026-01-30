/**
 * SQLite Persistence Layer - Offline/Edge Fallback
 *
 * Provides SQLite-compatible storage when PostgreSQL is unavailable.
 * Inspired by Claude Flow's fallback storage architecture.
 *
 * Use cases:
 * - Local development without PostgreSQL
 * - Edge deployment (limited resources)
 * - Offline operation (no network)
 * - Testing without external dependencies
 *
 * "Small kennel, same loyalty" - κυνικός
 *
 * @module @cynic/persistence/sqlite
 */

'use strict';

export * from './client.js';
export * from './schema.js';
export * from './repositories/index.js';

import { SQLiteClient, getSQLiteClient, createSQLiteClient } from './client.js';
import { initializeSchema, isSchemaInitialized, getSchemaVersion } from './schema.js';
import { SQLiteJudgmentRepository } from './repositories/judgments.js';
import { SQLitePatternRepository } from './repositories/patterns.js';
import { SQLiteUserRepository } from './repositories/users.js';

/**
 * SQLite Repository Factory
 *
 * Creates SQLite-compatible repositories.
 */
export class SQLiteRepositoryFactory {
  #db;

  /**
   * @param {SQLiteClient} db - SQLite client
   */
  constructor(db) {
    this.#db = db;
  }

  /**
   * Create a repository by name
   */
  create(name) {
    switch (name) {
      case 'judgments':
        return new SQLiteJudgmentRepository(this.#db);
      case 'patterns':
        return new SQLitePatternRepository(this.#db);
      case 'users':
        return new SQLiteUserRepository(this.#db);
      default:
        throw new Error(`Unknown SQLite repository: ${name}. Available: judgments, patterns, users`);
    }
  }

  /**
   * Create all available repositories
   */
  createAll() {
    return {
      judgments: this.create('judgments'),
      patterns: this.create('patterns'),
      users: this.create('users'),
    };
  }

  /**
   * Get available repository names
   */
  getAvailableNames() {
    return ['judgments', 'patterns', 'users'];
  }
}

/**
 * Initialize SQLite storage
 *
 * @param {Object} [options] - Client options
 * @returns {Promise<Object>} Initialized client and factory
 */
export async function initializeSQLite(options = {}) {
  const client = createSQLiteClient(options);
  await client.connect();

  // Initialize schema if needed
  const initialized = await isSchemaInitialized(client);
  if (!initialized) {
    await initializeSchema(client);
  }

  const factory = new SQLiteRepositoryFactory(client);

  return {
    client,
    factory,
    repositories: factory.createAll(),
  };
}

/**
 * Check if SQLite is available
 * @returns {Promise<boolean>}
 */
export async function isSQLiteAvailable() {
  try {
    await import('better-sqlite3');
    return true;
  } catch {
    return false;
  }
}

export default {
  SQLiteClient,
  SQLiteRepositoryFactory,
  getSQLiteClient,
  createSQLiteClient,
  initializeSQLite,
  initializeSchema,
  isSchemaInitialized,
  getSchemaVersion,
  isSQLiteAvailable,
  // Repositories
  SQLiteJudgmentRepository,
  SQLitePatternRepository,
  SQLiteUserRepository,
};

/**
 * Fallback Repository Factory - Automatic Backend Selection
 *
 * Provides seamless fallback from PostgreSQL to SQLite when:
 * - PostgreSQL is unavailable (offline, connection error)
 * - Running in edge/local environment
 * - Explicitly configured for SQLite
 *
 * Inspired by Claude Flow's resilient storage architecture.
 *
 * "When the big dog sleeps, the small dog guards" - κυνικός
 *
 * @module @cynic/persistence/fallback-factory
 */

'use strict';

import { EventEmitter } from 'events';
import { RepositoryFactory } from './factory.js';
import { getPool } from './postgres/client.js';

/**
 * φ-aligned constants
 */
const PHI_INV = 0.618;
const FIBONACCI = {
  F5: 5,    // Retry attempts
  F8: 21,   // Retry delay base (seconds)
};

/**
 * Backend types
 */
export const BackendType = {
  POSTGRES: 'postgres',
  SQLITE: 'sqlite',
  MEMORY: 'memory',
};

/**
 * Fallback Repository Factory
 *
 * Automatically selects the best available storage backend.
 */
export class FallbackRepositoryFactory extends EventEmitter {
  #postgresFactory = null;
  #sqliteFactory = null;
  #activeBackend = null;
  #preferredBackend = null;
  #options = {};

  /**
   * @param {Object} [options] - Factory options
   * @param {string} [options.preferred='postgres'] - Preferred backend
   * @param {boolean} [options.autoFallback=true] - Auto fallback on failure
   * @param {Object} [options.postgres] - PostgreSQL options
   * @param {Object} [options.sqlite] - SQLite options
   */
  constructor(options = {}) {
    super();

    this.#options = {
      preferred: BackendType.POSTGRES,
      autoFallback: true,
      postgres: {},
      sqlite: { memory: false },
      ...options,
    };

    this.#preferredBackend = this.#options.preferred;
  }

  /**
   * Initialize the factory
   * Attempts to connect to preferred backend, falls back if needed
   *
   * @returns {Promise<string>} Active backend type
   */
  async initialize() {
    // Try preferred backend first
    if (this.#preferredBackend === BackendType.POSTGRES) {
      const postgresOk = await this.#tryPostgres();
      if (postgresOk) {
        return this.#activeBackend;
      }
    }

    // Try SQLite fallback
    if (this.#options.autoFallback || this.#preferredBackend === BackendType.SQLITE) {
      const sqliteOk = await this.#trySQLite();
      if (sqliteOk) {
        return this.#activeBackend;
      }
    }

    // Last resort: in-memory SQLite
    if (this.#options.autoFallback) {
      const memoryOk = await this.#tryMemory();
      if (memoryOk) {
        return this.#activeBackend;
      }
    }

    throw new Error('No storage backend available');
  }

  /**
   * Try to connect to PostgreSQL
   * @private
   */
  async #tryPostgres() {
    try {
      const pool = getPool();
      await pool.query('SELECT 1');

      this.#postgresFactory = new RepositoryFactory(pool);
      this.#activeBackend = BackendType.POSTGRES;

      this.emit('backend:connected', {
        type: BackendType.POSTGRES,
        message: 'PostgreSQL connected',
      });

      return true;
    } catch (err) {
      this.emit('backend:failed', {
        type: BackendType.POSTGRES,
        error: err.message,
      });
      return false;
    }
  }

  /**
   * Try to connect to SQLite
   * @private
   */
  async #trySQLite() {
    try {
      // Dynamic import to avoid loading if not needed
      const { initializeSQLite, isSQLiteAvailable } = await import('./sqlite/index.js');

      if (!await isSQLiteAvailable()) {
        this.emit('backend:unavailable', {
          type: BackendType.SQLITE,
          message: 'better-sqlite3 not installed',
        });
        return false;
      }

      const { client, factory } = await initializeSQLite(this.#options.sqlite);

      this.#sqliteFactory = factory;
      this.#activeBackend = BackendType.SQLITE;

      this.emit('backend:connected', {
        type: BackendType.SQLITE,
        message: 'SQLite connected (fallback)',
        path: client.getStats().path,
      });

      return true;
    } catch (err) {
      this.emit('backend:failed', {
        type: BackendType.SQLITE,
        error: err.message,
      });
      return false;
    }
  }

  /**
   * Try in-memory SQLite (last resort)
   * @private
   */
  async #tryMemory() {
    try {
      const { initializeSQLite, isSQLiteAvailable } = await import('./sqlite/index.js');

      if (!await isSQLiteAvailable()) {
        return false;
      }

      const { factory } = await initializeSQLite({ memory: true });

      this.#sqliteFactory = factory;
      this.#activeBackend = BackendType.MEMORY;

      this.emit('backend:connected', {
        type: BackendType.MEMORY,
        message: 'In-memory SQLite (ephemeral)',
      });

      return true;
    } catch (err) {
      this.emit('backend:failed', {
        type: BackendType.MEMORY,
        error: err.message,
      });
      return false;
    }
  }

  /**
   * Create a repository by name
   *
   * @param {string} name - Repository name
   * @returns {Object} Repository instance
   */
  create(name) {
    if (!this.#activeBackend) {
      throw new Error('Factory not initialized. Call initialize() first.');
    }

    if (this.#activeBackend === BackendType.POSTGRES) {
      return this.#postgresFactory.create(name);
    }

    // SQLite or Memory
    return this.#sqliteFactory.create(name);
  }

  /**
   * Create all available repositories
   *
   * @returns {Object} All repositories
   */
  createAll() {
    if (!this.#activeBackend) {
      throw new Error('Factory not initialized. Call initialize() first.');
    }

    if (this.#activeBackend === BackendType.POSTGRES) {
      return this.#postgresFactory.createAll();
    }

    return this.#sqliteFactory.createAll();
  }

  /**
   * Get active backend type
   * @returns {string|null} Active backend
   */
  getActiveBackend() {
    return this.#activeBackend;
  }

  /**
   * Check if using fallback
   * @returns {boolean}
   */
  isUsingFallback() {
    return this.#activeBackend !== this.#preferredBackend;
  }

  /**
   * Get available repository names
   * @returns {string[]}
   */
  getAvailableNames() {
    if (this.#activeBackend === BackendType.POSTGRES) {
      return this.#postgresFactory.getAvailableNames();
    }
    return this.#sqliteFactory?.getAvailableNames() || [];
  }

  /**
   * Get factory status
   * @returns {Object}
   */
  getStatus() {
    return {
      activeBackend: this.#activeBackend,
      preferredBackend: this.#preferredBackend,
      isUsingFallback: this.isUsingFallback(),
      availableRepositories: this.getAvailableNames(),
    };
  }
}

/**
 * Create and initialize a fallback factory
 *
 * @param {Object} [options] - Factory options
 * @returns {Promise<FallbackRepositoryFactory>}
 */
export async function createFallbackFactory(options = {}) {
  const factory = new FallbackRepositoryFactory(options);
  await factory.initialize();
  return factory;
}

export default {
  FallbackRepositoryFactory,
  BackendType,
  createFallbackFactory,
};

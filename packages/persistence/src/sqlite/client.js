/**
 * SQLite Client - Fallback Storage for Offline/Edge Deployment
 *
 * Provides PostgreSQL-compatible interface using SQLite for scenarios
 * where PostgreSQL is unavailable (offline, edge, local dev).
 *
 * Inspired by Claude Flow's fallback storage architecture.
 *
 * "When the big kennel is closed, the small kennel opens" - κυνικός
 *
 * @module @cynic/persistence/sqlite/client
 */

'use strict';

import { EventEmitter } from 'events';
import { existsSync, mkdirSync } from 'fs';
import { dirname, join } from 'path';
import { homedir } from 'os';

// Dynamic import for better-sqlite3 (optional dependency)
let Database = null;

/**
 * φ-aligned constants
 */
const PHI_INV = 0.618;
const FIBONACCI = {
  F8: 21,   // Connection timeout seconds
  F10: 55,  // Max memory pages
  F11: 89,  // Cache size KB
};

/**
 * Default SQLite options
 */
const DEFAULT_OPTIONS = {
  // Database file path (default: ~/.cynic/cynic.db)
  path: null,
  // Memory-only mode (no file persistence)
  memory: false,
  // Read-only mode
  readonly: false,
  // WAL mode for better concurrency
  wal: true,
  // Foreign keys enforcement
  foreignKeys: true,
  // Cache size in KB (φ-aligned)
  cacheSize: FIBONACCI.F11 * 1024, // ~89MB
  // Timeout in ms
  timeout: FIBONACCI.F8 * 1000, // 21 seconds
};

/**
 * Load SQLite library dynamically
 * @returns {Promise<Object>} better-sqlite3 Database class
 */
async function loadSQLite() {
  if (Database) return Database;

  try {
    const module = await import('better-sqlite3');
    Database = module.default;
    return Database;
  } catch (err) {
    throw new Error(
      'SQLite fallback requires better-sqlite3. Install with: npm install better-sqlite3\n' +
      `Original error: ${err.message}`
    );
  }
}

/**
 * Get default database path
 * @returns {string} Default path
 */
function getDefaultPath() {
  const cynicDir = join(homedir(), '.cynic');
  if (!existsSync(cynicDir)) {
    mkdirSync(cynicDir, { recursive: true });
  }
  return join(cynicDir, 'cynic.db');
}

/**
 * SQLite Client
 *
 * Provides PostgreSQL-compatible query interface using SQLite.
 */
export class SQLiteClient extends EventEmitter {
  /**
   * @param {Object} [options] - Client options
   */
  constructor(options = {}) {
    super();

    this.options = { ...DEFAULT_OPTIONS, ...options };
    this.db = null;
    this.connected = false;
    this.stats = {
      queries: 0,
      inserts: 0,
      updates: 0,
      deletes: 0,
      errors: 0,
    };
  }

  /**
   * Connect to SQLite database
   * @returns {Promise<void>}
   */
  async connect() {
    if (this.connected) return;

    const SqliteDatabase = await loadSQLite();

    const dbPath = this.options.memory
      ? ':memory:'
      : (this.options.path || getDefaultPath());

    // Ensure directory exists
    if (!this.options.memory && dbPath !== ':memory:') {
      const dir = dirname(dbPath);
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true });
      }
    }

    try {
      this.db = new SqliteDatabase(dbPath, {
        readonly: this.options.readonly,
        timeout: this.options.timeout,
      });

      // Configure SQLite for better performance
      if (this.options.wal && !this.options.readonly) {
        this.db.pragma('journal_mode = WAL');
      }
      if (this.options.foreignKeys) {
        this.db.pragma('foreign_keys = ON');
      }
      this.db.pragma(`cache_size = -${this.options.cacheSize}`);
      this.db.pragma('synchronous = NORMAL');

      this.connected = true;
      this.emit('connect', { path: dbPath });
    } catch (err) {
      this.emit('error', err);
      throw err;
    }
  }

  /**
   * Close database connection
   */
  close() {
    if (this.db) {
      this.db.close();
      this.db = null;
      this.connected = false;
      this.emit('close');
    }
  }

  /**
   * Execute a query (PostgreSQL-compatible interface)
   *
   * Translates PostgreSQL parameterized queries ($1, $2) to SQLite (?, ?).
   *
   * @param {string} sql - SQL query
   * @param {Array} [params=[]] - Query parameters
   * @returns {Promise<Object>} Query result with rows and rowCount
   */
  async query(sql, params = []) {
    if (!this.connected) {
      await this.connect();
    }

    try {
      // Translate PostgreSQL $N placeholders to SQLite ?
      const sqliteSql = this._translateSQL(sql);

      this.stats.queries++;

      // Determine query type
      const trimmed = sqliteSql.trim().toUpperCase();

      if (trimmed.startsWith('SELECT') || trimmed.startsWith('WITH')) {
        const stmt = this.db.prepare(sqliteSql);
        const rows = stmt.all(...params);
        return { rows, rowCount: rows.length };
      }

      if (trimmed.startsWith('INSERT')) {
        this.stats.inserts++;
        const stmt = this.db.prepare(sqliteSql);
        const result = stmt.run(...params);

        // Handle RETURNING clause (simulated)
        if (sqliteSql.toUpperCase().includes('RETURNING')) {
          const tableName = this._extractTableName(sqliteSql);
          const lastRow = this.db.prepare(
            `SELECT * FROM ${tableName} WHERE rowid = ?`
          ).get(result.lastInsertRowid);
          return { rows: lastRow ? [lastRow] : [], rowCount: 1 };
        }

        return { rows: [], rowCount: result.changes };
      }

      if (trimmed.startsWith('UPDATE')) {
        this.stats.updates++;
        const stmt = this.db.prepare(sqliteSql);
        const result = stmt.run(...params);

        // Handle RETURNING clause
        if (sqliteSql.toUpperCase().includes('RETURNING')) {
          return { rows: [], rowCount: result.changes };
        }

        return { rows: [], rowCount: result.changes };
      }

      if (trimmed.startsWith('DELETE')) {
        this.stats.deletes++;
        const stmt = this.db.prepare(sqliteSql);
        const result = stmt.run(...params);
        return { rows: [], rowCount: result.changes };
      }

      // DDL or other statements
      this.db.exec(sqliteSql);
      return { rows: [], rowCount: 0 };
    } catch (err) {
      this.stats.errors++;
      this.emit('error', err);
      throw err;
    }
  }

  /**
   * Translate PostgreSQL SQL to SQLite
   * @private
   */
  _translateSQL(sql) {
    let result = sql;

    // Replace $1, $2, etc. with ?
    result = result.replace(/\$(\d+)/g, '?');

    // Replace PostgreSQL-specific syntax
    result = result.replace(/::text/gi, '');
    result = result.replace(/::integer/gi, '');
    result = result.replace(/::float/gi, '');
    result = result.replace(/::jsonb?/gi, '');
    result = result.replace(/::uuid/gi, '');
    result = result.replace(/::timestamptz?/gi, '');
    result = result.replace(/::varchar\(\d+\)/gi, '');
    result = result.replace(/::decimal\(\d+,\d+\)/gi, '');

    // Replace ILIKE with LIKE (case-insensitive handled differently)
    result = result.replace(/\bILIKE\b/gi, 'LIKE');

    // Replace NOW() with datetime('now')
    result = result.replace(/\bNOW\(\)/gi, "datetime('now')");

    // Replace CURRENT_TIMESTAMP
    result = result.replace(/\bCURRENT_TIMESTAMP\b/gi, "datetime('now')");

    // Replace gen_random_uuid() with hex(randomblob(16))
    result = result.replace(/gen_random_uuid\(\)/gi, "lower(hex(randomblob(16)))");

    // Handle RETURNING * (SQLite doesn't support it directly)
    // We'll handle this in the query method

    // Replace SERIAL/BIGSERIAL with INTEGER PRIMARY KEY
    result = result.replace(/\bSERIAL\b/gi, 'INTEGER');
    result = result.replace(/\bBIGSERIAL\b/gi, 'INTEGER');

    // Replace BOOLEAN with INTEGER (SQLite uses 0/1)
    result = result.replace(/\bBOOLEAN\b/gi, 'INTEGER');

    // Replace JSONB with TEXT (we'll JSON.stringify/parse)
    result = result.replace(/\bJSONB\b/gi, 'TEXT');
    result = result.replace(/\bJSON\b/gi, 'TEXT');

    // Replace UUID with TEXT
    result = result.replace(/\bUUID\b/gi, 'TEXT');

    // Replace TIMESTAMPTZ with TEXT
    result = result.replace(/\bTIMESTAMPTZ\b/gi, 'TEXT');
    result = result.replace(/\bTIMESTAMP\b/gi, 'TEXT');

    // Replace TEXT[] with TEXT (we'll JSON.stringify arrays)
    result = result.replace(/\bTEXT\[\]/gi, 'TEXT');

    // Replace DECIMAL with REAL
    result = result.replace(/\bDECIMAL\(\d+,\d+\)/gi, 'REAL');

    // Replace vector(N) with TEXT (we'll store as JSON)
    result = result.replace(/\bvector\(\d+\)/gi, 'TEXT');

    // Replace IF NOT EXISTS for columns (not supported in SQLite ALTER)
    // This is handled differently - we skip these

    return result;
  }

  /**
   * Extract table name from SQL
   * @private
   */
  _extractTableName(sql) {
    const match = sql.match(/(?:INSERT INTO|UPDATE|FROM)\s+["']?(\w+)["']?/i);
    return match ? match[1] : 'unknown';
  }

  /**
   * Execute raw SQL (for migrations/setup)
   * @param {string} sql - SQL to execute
   */
  exec(sql) {
    if (!this.connected) {
      throw new Error('Not connected');
    }
    this.db.exec(sql);
  }

  /**
   * Begin a transaction
   * @returns {Object} Transaction object
   */
  transaction(fn) {
    if (!this.connected) {
      throw new Error('Not connected');
    }
    return this.db.transaction(fn);
  }

  /**
   * Get client statistics
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      connected: this.connected,
      path: this.options.memory ? ':memory:' : (this.options.path || getDefaultPath()),
    };
  }
}

/**
 * Singleton client instance
 */
let defaultClient = null;

/**
 * Get default SQLite client
 * @param {Object} [options] - Client options
 * @returns {SQLiteClient} Client instance
 */
export function getSQLiteClient(options = {}) {
  if (!defaultClient) {
    defaultClient = new SQLiteClient(options);
  }
  return defaultClient;
}

/**
 * Create a new SQLite client
 * @param {Object} [options] - Client options
 * @returns {SQLiteClient} New client instance
 */
export function createSQLiteClient(options = {}) {
  return new SQLiteClient(options);
}

export default {
  SQLiteClient,
  getSQLiteClient,
  createSQLiteClient,
};

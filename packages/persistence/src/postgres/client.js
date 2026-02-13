/**
 * PostgreSQL Client - Connection Pool Manager
 *
 * Uses pg with connection pooling for efficient database access.
 * φ-derived pool sizing: max connections = Fib(8) = 21
 *
 * @module @cynic/persistence/postgres
 */

'use strict';

import { readFileSync } from 'fs';
import pg from 'pg';
import { createLogger, CircuitBreaker, CircuitState, withCircuitBreaker } from '@cynic/core';

const { Pool } = pg;
const log = createLogger('PostgresClient');

// Singleton pool instance
let pool = null;

/**
 * Default pool configuration (φ-derived)
 */
const DEFAULT_CONFIG = {
  max: 21,                    // Fib(8) - max connections
  idleTimeoutMillis: 61800,   // φ⁻¹ × 100000 - idle timeout
  connectionTimeoutMillis: 10000, // 10s — remote SSL handshake needs headroom (was 3820, too tight for Render)
  allowExitOnIdle: true,
};

/**
 * Determine SSL config based on connection string and environment
 *
 * SECURITY MODES:
 * 1. STRICT (CYNIC_DB_SSL_STRICT=true): Requires valid certificates
 *    - Use with CYNIC_DB_SSL_CA for custom CA certificates
 *    - Recommended for production with enterprise databases
 *
 * 2. MANAGED (default for cloud): rejectUnauthorized=false
 *    - For Render, Railway, Supabase, etc. with self-signed certs
 *    - Network-level security provides protection
 *
 * 3. DISABLED: For localhost or explicit sslmode=disable
 *
 * @param {string} connectionString - Database URL
 * @returns {Object|boolean} SSL config or false
 */
function getSSLConfig(connectionString) {
  // Disable SSL for local connections or explicit sslmode=disable
  if (connectionString.includes('localhost') ||
      connectionString.includes('127.0.0.1') ||
      connectionString.includes('sslmode=disable')) {
    return false;
  }

  // Strict mode with CA certificate verification
  if (process.env.CYNIC_DB_SSL_STRICT === 'true') {
    const sslConfig = { rejectUnauthorized: true };

    // Load CA certificate if provided
    if (process.env.CYNIC_DB_SSL_CA) {
      try {
        sslConfig.ca = readFileSync(process.env.CYNIC_DB_SSL_CA, 'utf8');
      } catch (err) {
        log.warn('Failed to load SSL CA', { error: err.message });
      }
    }

    return sslConfig;
  }

  // Default: relaxed validation for managed cloud databases
  // This is secure for services like Render/Railway that use network isolation
  return { rejectUnauthorized: false };
}

/**
 * Build connection string from component environment variables
 * Supports: CYNIC_DB_HOST, CYNIC_DB_PORT, CYNIC_DB_USER, CYNIC_DB_PASSWORD, CYNIC_DB_NAME
 * @returns {string|null} Connection string or null if not configured
 */
function buildConnectionStringFromEnv() {
  const { CYNIC_DB_HOST, CYNIC_DB_PORT, CYNIC_DB_USER, CYNIC_DB_PASSWORD, CYNIC_DB_NAME } = process.env;

  if (!CYNIC_DB_HOST || !CYNIC_DB_PASSWORD) {
    return null;
  }

  const host = CYNIC_DB_HOST;
  const port = CYNIC_DB_PORT || '5432';
  const user = CYNIC_DB_USER || 'cynic';
  const name = CYNIC_DB_NAME || 'cynic';
  const pass = CYNIC_DB_PASSWORD;

  // Build connection string from parts (credentials from env vars)
  const parts = [
    'postgres',    // protocol
    '://',
    encodeURIComponent(user),
    ':',
    encodeURIComponent(pass),
    '@',
    host,
    ':',
    port,
    '/',
    encodeURIComponent(name),
    '?sslmode=disable',
  ];
  return parts.join('');
}

/**
 * PostgreSQL Client wrapper
 */
export class PostgresClient {
  constructor(connectionString, config = {}) {
    // Priority: explicit arg > CYNIC_DATABASE_URL > component env vars
    this.connectionString = connectionString
      || process.env.CYNIC_DATABASE_URL
      || buildConnectionStringFromEnv();
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.pool = null;

    // v1.1: Use shared CircuitBreaker from @cynic/core (with exponential backoff, jitter)
    this._breaker = new CircuitBreaker({
      name: 'postgres',
      ...config.circuitBreaker,
      // Health probe: lightweight SELECT 1 check
      healthProbe: async () => {
        if (!this.pool) return false;
        try {
          await this.pool.query('SELECT 1');
          return true;
        } catch {
          return false;
        }
      },
    });
  }

  /**
   * Get current circuit breaker state
   * v1.1: Now returns full state from core CircuitBreaker
   */
  getCircuitState() {
    return this._breaker.getState();
  }

  /**
   * Get circuit breaker statistics
   * v1.1: New method for observability
   */
  getCircuitStats() {
    return this._breaker.getStats();
  }

  /**
   * Initialize connection pool
   */
  async connect() {
    if (this.pool) return this;

    if (!this.connectionString) {
      throw new Error('CYNIC_DATABASE_URL not set');
    }

    this.pool = new Pool({
      connectionString: this.connectionString,
      ...this.config,
      ssl: this.config.ssl !== undefined ? this.config.ssl : getSSLConfig(this.connectionString),
    });

    // Test connection
    const client = await this.pool.connect();
    try {
      await client.query('SELECT NOW()');
      log.info('PostgreSQL connected');
    } finally {
      client.release();
    }

    return this;
  }

  /**
   * Execute a query with circuit breaker protection
   * v1.1: Uses core CircuitBreaker with exponential backoff
   */
  async query(text, params = []) {
    // Ensure pool exists
    if (!this.pool) {
      await this.connect();
    }

    // v1.1: Use withCircuitBreaker helper with probeFirst in half-open
    return withCircuitBreaker(
      this._breaker,
      async () => this.pool.query(text, params),
      { throwOnOpen: true, probeFirst: true },
    );
  }

  /**
   * Get a client from the pool (for transactions)
   * Protected by circuit breaker
   * v1.1: Uses core CircuitBreaker with exponential backoff
   */
  async getClient() {
    // Ensure pool exists
    if (!this.pool) {
      await this.connect();
    }

    // v1.1: Use withCircuitBreaker helper with probeFirst in half-open
    return withCircuitBreaker(
      this._breaker,
      async () => this.pool.connect(),
      { throwOnOpen: true, probeFirst: true },
    );
  }

  /**
   * Execute within a transaction
   */
  async transaction(callback) {
    const client = await this.getClient();
    try {
      await client.query('BEGIN');
      const result = await callback(client);
      await client.query('COMMIT');
      return result;
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // v1.1: BATCH OPERATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Batch insert multiple rows in a single query
   * v1.1: Reduces round-trips for bulk inserts
   *
   * @param {string} table - Table name
   * @param {string[]} columns - Column names
   * @param {Array<Array>} rows - Array of row values (each row is an array matching columns)
   * @param {Object} [options] - Options
   * @param {string} [options.onConflict] - ON CONFLICT clause (e.g., 'DO NOTHING' or 'DO UPDATE SET ...')
   * @param {string} [options.returning] - RETURNING clause (e.g., '*' or 'id')
   * @returns {Promise<{rows: Array, rowCount: number}>}
   *
   * @example
   * await client.batchInsert('patterns', ['name', 'category', 'confidence'], [
   *   ['pattern1', 'code', 0.8],
   *   ['pattern2', 'error', 0.6],
   * ], { onConflict: 'DO NOTHING' });
   */
  async batchInsert(table, columns, rows, options = {}) {
    if (!rows || rows.length === 0) {
      return { rows: [], rowCount: 0 };
    }

    // Build parameterized query
    const { onConflict, returning } = options;
    const colList = columns.join(', ');

    // Generate placeholders: ($1, $2, $3), ($4, $5, $6), ...
    const placeholders = [];
    const values = [];
    let paramIndex = 1;

    for (const row of rows) {
      const rowPlaceholders = [];
      for (const value of row) {
        rowPlaceholders.push(`$${paramIndex++}`);
        values.push(value);
      }
      placeholders.push(`(${rowPlaceholders.join(', ')})`);
    }

    let sql = `INSERT INTO ${table} (${colList}) VALUES ${placeholders.join(', ')}`;

    if (onConflict) {
      sql += ` ON CONFLICT ${onConflict}`;
    }

    if (returning) {
      sql += ` RETURNING ${returning}`;
    }

    return this.query(sql, values);
  }

  /**
   * Batch upsert multiple rows (INSERT ... ON CONFLICT DO UPDATE)
   * v1.1: Efficient bulk upsert with automatic conflict handling
   *
   * @param {string} table - Table name
   * @param {string[]} columns - Column names
   * @param {Array<Array>} rows - Array of row values
   * @param {string[]} conflictColumns - Columns that define uniqueness
   * @param {string[]} [updateColumns] - Columns to update on conflict (defaults to all non-conflict columns)
   * @returns {Promise<{rows: Array, rowCount: number}>}
   *
   * @example
   * await client.batchUpsert('patterns', ['name', 'category', 'confidence'], [
   *   ['pattern1', 'code', 0.8],
   *   ['pattern2', 'error', 0.6],
   * ], ['name']);
   */
  async batchUpsert(table, columns, rows, conflictColumns, updateColumns = null) {
    if (!rows || rows.length === 0) {
      return { rows: [], rowCount: 0 };
    }

    // Determine which columns to update
    const toUpdate = updateColumns || columns.filter(c => !conflictColumns.includes(c));

    if (toUpdate.length === 0) {
      // No columns to update - just insert or ignore
      return this.batchInsert(table, columns, rows, { onConflict: 'DO NOTHING' });
    }

    // Build ON CONFLICT DO UPDATE SET clause
    const updateSet = toUpdate.map(col => `${col} = EXCLUDED.${col}`).join(', ');
    const onConflict = `(${conflictColumns.join(', ')}) DO UPDATE SET ${updateSet}`;

    return this.batchInsert(table, columns, rows, { onConflict, returning: '*' });
  }

  /**
   * Execute multiple queries in a single transaction
   * v1.1: Atomic batch execution
   *
   * @param {Array<{sql: string, params?: Array}>} queries - Array of queries to execute
   * @returns {Promise<Array<{rows: Array, rowCount: number}>>}
   *
   * @example
   * await client.batchExecute([
   *   { sql: 'INSERT INTO logs (msg) VALUES ($1)', params: ['log1'] },
   *   { sql: 'INSERT INTO logs (msg) VALUES ($1)', params: ['log2'] },
   * ]);
   */
  async batchExecute(queries) {
    if (!queries || queries.length === 0) {
      return [];
    }

    return this.transaction(async (client) => {
      const results = [];
      for (const { sql, params = [] } of queries) {
        const result = await client.query(sql, params);
        results.push({ rows: result.rows, rowCount: result.rowCount });
      }
      return results;
    });
  }

  /**
   * Close all connections
   */
  async close() {
    if (this.pool) {
      await this.pool.end();
      this.pool = null;
      log.info('PostgreSQL disconnected');
    }
  }

  /**
   * Reset circuit breaker (for manual recovery)
   * v1.1: Delegates to core CircuitBreaker
   */
  resetCircuitBreaker() {
    this._breaker.reset();
    log.info('Circuit breaker: manually reset to CLOSED');
  }

  /**
   * Force circuit breaker open (for maintenance)
   * v1.1: New method for controlled shutdown
   */
  tripCircuitBreaker() {
    this._breaker.trip();
    log.warn('Circuit breaker: manually tripped to OPEN');
  }

  /**
   * Health check
   * v1.1: Now includes circuit breaker state and backoff info
   */
  async health() {
    const circuitState = this._breaker.getState();

    // If circuit is open, don't query - report unhealthy with circuit info
    if (circuitState.state === CircuitState.OPEN) {
      return {
        status: 'unhealthy',
        error: 'Circuit breaker is OPEN',
        circuit: {
          state: circuitState.state,
          consecutiveOpenings: circuitState.consecutiveOpenings,
          currentBackoffMs: circuitState.currentBackoffMs,
          timeUntilProbeMs: circuitState.timeUntilProbeMs,
        },
      };
    }

    try {
      const start = Date.now();
      // Direct pool query to avoid circuit breaker (we're checking health)
      if (this.pool) {
        await this.pool.query('SELECT 1');
      } else {
        await this.connect();
        await this.pool.query('SELECT 1');
      }
      const latency = Date.now() - start;

      return {
        status: 'healthy',
        latency,
        pool: {
          total: this.pool?.totalCount || 0,
          idle: this.pool?.idleCount || 0,
          waiting: this.pool?.waitingCount || 0,
        },
        circuit: {
          state: circuitState.state,
          consecutiveOpenings: circuitState.consecutiveOpenings,
        },
      };
    } catch (error) {
      return {
        status: 'unhealthy',
        error: error.message,
        circuit: {
          state: circuitState.state,
        },
      };
    }
  }
}

/**
 * Get shared pool instance (singleton)
 */
export function getPool(connectionString) {
  if (!pool) {
    pool = new PostgresClient(connectionString);
  }
  return pool;
}

export default PostgresClient;

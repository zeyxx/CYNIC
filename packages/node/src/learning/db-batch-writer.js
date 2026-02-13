/**
 * DB Batch Writer - Reduces database round-trips via batching
 *
 * Collects multiple DB writes into a single transaction.
 * Flushes on:
 * - Buffer limit (10 writes) — Fibonacci F(5)
 * - Time limit (100ms) — φ⁻¹ × 161.8ms
 * - Manual flush
 *
 * "Seven round-trips become one" - κυνικός
 *
 * @module @cynic/node/learning/db-batch-writer
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('DBBatchWriter');

// φ-aligned configuration
const DEFAULT_CONFIG = {
  bufferLimit: 10,        // F(5) = 10 max writes per batch
  flushIntervalMs: 100,   // φ⁻¹ × 161.8 ≈ 100ms
  maxRetries: 3,          // F(4) = 3 retries on failure
  retryDelayMs: 100,      // 100ms between retries
};

/**
 * DB Batch Writer - Reduces round-trips via batching
 *
 * Collects multiple DB writes into a single transaction.
 */
export class DBBatchWriter {
  /**
   * @param {Object} pool - PostgreSQL pool with query() method
   * @param {Object} [options] - Configuration options
   * @param {number} [options.bufferLimit] - Max writes per batch (default: 10)
   * @param {number} [options.flushIntervalMs] - Max time before flush (default: 100ms)
   * @param {number} [options.maxRetries] - Max retry attempts (default: 3)
   */
  constructor(pool, options = {}) {
    this.pool = pool;
    this.config = { ...DEFAULT_CONFIG, ...options };

    // Write buffer
    this.buffer = [];
    this.flushTimer = null;

    // Statistics
    this.stats = {
      buffered: 0,
      flushed: 0,
      transactions: 0,
      errors: 0,
      retries: 0,
    };

    // State
    this._flushing = false;
    this._closed = false;
  }

  /**
   * Add a query to the batch
   *
   * @param {string} query - SQL query with $1, $2... placeholders
   * @param {Array} params - Query parameters
   */
  add(query, params = []) {
    if (this._closed) {
      throw new Error('DBBatchWriter is closed');
    }

    this.buffer.push({
      query,
      params,
      timestamp: Date.now(),
    });
    this.stats.buffered++;

    // Immediate flush if buffer full
    if (this.buffer.length >= this.config.bufferLimit) {
      this.flush().catch(err =>
        log.error('Batch flush failed', { error: err.message })
      );
    } else {
      // Schedule flush
      this._scheduleFlush();
    }
  }

  /**
   * Schedule a debounced flush
   * @private
   */
  _scheduleFlush() {
    if (this.flushTimer) return;

    this.flushTimer = setTimeout(() => {
      this.flushTimer = null;
      this.flush().catch(err =>
        log.error('Scheduled flush failed', { error: err.message })
      );
    }, this.config.flushIntervalMs);

    // Allow process to exit even with pending timer
    if (this.flushTimer.unref) {
      this.flushTimer.unref();
    }
  }

  /**
   * Flush all buffered writes in a single transaction
   *
   * @returns {Promise<number>} Number of writes flushed
   */
  async flush() {
    if (this._flushing || this.buffer.length === 0) {
      return 0;
    }

    this._flushing = true;

    // Clear timer
    if (this.flushTimer) {
      clearTimeout(this.flushTimer);
      this.flushTimer = null;
    }

    const batch = [...this.buffer];
    this.buffer = [];

    try {
      await this._executeBatch(batch);

      this.stats.flushed += batch.length;
      this.stats.transactions++;

      log.debug('Batch flushed', { writes: batch.length });

      return batch.length;

    } catch (err) {
      // Put failed writes back in buffer (at front)
      this.buffer = [...batch, ...this.buffer];
      this.stats.errors++;

      log.error('Batch execution failed', {
        error: err.message,
        writes: batch.length,
      });

      throw err;
    } finally {
      this._flushing = false;
    }
  }

  /**
   * Execute batch with transaction and retry logic
   * @private
   * @param {Array} batch - Array of {query, params} objects
   */
  async _executeBatch(batch) {
    let lastError = null;

    for (let attempt = 1; attempt <= this.config.maxRetries; attempt++) {
      try {
        // Execute all writes in a single transaction
        await this.pool.query('BEGIN');

        for (const { query, params } of batch) {
          await this.pool.query(query, params);
        }

        await this.pool.query('COMMIT');

        // Success - exit retry loop
        return;

      } catch (err) {
        lastError = err;
        this.stats.retries++;

        // Always rollback on error
        try {
          await this.pool.query('ROLLBACK');
        } catch (rollbackErr) {
          log.warn('Rollback failed', { error: rollbackErr.message });
        }

        // Retry if not last attempt
        if (attempt < this.config.maxRetries) {
          log.warn('Batch execution failed, retrying', {
            attempt,
            maxRetries: this.config.maxRetries,
            error: err.message,
          });

          // Exponential backoff
          await this._sleep(this.config.retryDelayMs * attempt);
        }
      }
    }

    // All retries exhausted
    throw lastError;
  }

  /**
   * Sleep helper
   * @private
   */
  _sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * Close writer and flush remaining writes
   */
  async close() {
    if (this._closed) return;

    this._closed = true;

    if (this.flushTimer) {
      clearTimeout(this.flushTimer);
      this.flushTimer = null;
    }

    // Final flush
    await this.flush();

    log.info('DBBatchWriter closed', this.stats);
  }

  /**
   * Get statistics
   *
   * @returns {Object} Statistics
   */
  getStats() {
    return {
      ...this.stats,
      bufferSize: this.buffer.length,
      isFlushing: this._flushing,
      isClosed: this._closed,
      // φ-bounded efficiency score
      efficiency: this.stats.buffered > 0
        ? Math.min(PHI_INV, this.stats.flushed / this.stats.buffered)
        : 0,
    };
  }

  /**
   * Get health assessment
   *
   * @returns {Object} Health status
   */
  getHealth() {
    const errorRate = this.stats.transactions > 0
      ? this.stats.errors / this.stats.transactions
      : 0;

    const avgBatchSize = this.stats.transactions > 0
      ? this.stats.flushed / this.stats.transactions
      : 0;

    return {
      status: errorRate < 0.1 ? 'healthy' : errorRate < 0.3 ? 'degraded' : 'unhealthy',
      errorRate,
      avgBatchSize: Math.round(avgBatchSize * 10) / 10,
      bufferSize: this.buffer.length,
      efficiency: this.getStats().efficiency,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create DBBatchWriter singleton
 *
 * @param {Object} pool - PostgreSQL pool
 * @param {Object} [options] - Configuration options
 * @returns {DBBatchWriter} Singleton instance
 */
export function getDBBatchWriter(pool, options = {}) {
  if (!_instance) {
    if (!pool) {
      throw new Error('DBBatchWriter requires a pool instance');
    }
    _instance = new DBBatchWriter(pool, options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export async function resetDBBatchWriter() {
  if (_instance) {
    await _instance.close();
  }
  _instance = null;
}

export default {
  DBBatchWriter,
  getDBBatchWriter,
  resetDBBatchWriter,
};

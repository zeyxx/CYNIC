/**
 * BatchQueue - Accumulates operations for batched execution
 *
 * v1.1: Reduces database round-trips by batching writes
 *
 * φ-derived thresholds:
 * - Default batch size: 13 (Fibonacci)
 * - Default flush interval: 5000ms (5s)
 * - Max queue size: 89 (Fibonacci)
 *
 * "Patience, then action" - κυνικός
 *
 * @module @cynic/persistence/batch-queue
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('BatchQueue');

/**
 * Default batch queue configuration
 */
export const DEFAULT_BATCH_CONFIG = Object.freeze({
  batchSize: 13,           // Flush when queue reaches this size (Fibonacci)
  flushIntervalMs: 5000,   // Flush every 5 seconds
  maxQueueSize: 89,        // Max items before forced flush (Fibonacci)
});

/**
 * BatchQueue - Accumulates items and flushes them in batches
 *
 * @example
 * const queue = new BatchQueue({
 *   name: 'patterns',
 *   flushFn: async (items) => {
 *     await db.batchInsert('patterns', ['name', 'value'], items.map(i => [i.name, i.value]));
 *   },
 * });
 *
 * queue.add({ name: 'pattern1', value: 'foo' });
 * queue.add({ name: 'pattern2', value: 'bar' });
 * // Items will be flushed automatically when batch size or interval is reached
 */
export class BatchQueue {
  /**
   * Create a batch queue
   *
   * @param {Object} options - Configuration
   * @param {string} [options.name='default'] - Queue name for logging
   * @param {Function} options.flushFn - Async function to execute with batched items
   * @param {number} [options.batchSize=13] - Items before auto-flush
   * @param {number} [options.flushIntervalMs=5000] - Interval for time-based flush
   * @param {number} [options.maxQueueSize=89] - Max items before forced flush
   * @param {Function} [options.onError] - Error handler callback
   */
  constructor(options = {}) {
    if (!options.flushFn) {
      throw new Error('BatchQueue requires a flushFn');
    }

    this._name = options.name || 'default';
    this._flushFn = options.flushFn;
    this._config = {
      ...DEFAULT_BATCH_CONFIG,
      batchSize: options.batchSize ?? DEFAULT_BATCH_CONFIG.batchSize,
      flushIntervalMs: options.flushIntervalMs ?? DEFAULT_BATCH_CONFIG.flushIntervalMs,
      maxQueueSize: options.maxQueueSize ?? DEFAULT_BATCH_CONFIG.maxQueueSize,
    };
    this._onError = options.onError || ((err) => log.error('Batch flush error', { name: this._name, error: err.message }));

    // State
    this._queue = [];
    this._flushing = false;
    this._flushTimer = null;
    this._closed = false;

    // Stats
    this._stats = {
      totalAdded: 0,
      totalFlushed: 0,
      flushCount: 0,
      errors: 0,
      lastFlush: null,
    };

    // Start periodic flush timer
    this._startFlushTimer();

    log.debug('BatchQueue created', { name: this._name, config: this._config });
  }

  /**
   * Add an item to the queue
   * May trigger immediate flush if thresholds reached
   *
   * @param {*} item - Item to queue
   * @returns {Promise<void>}
   */
  async add(item) {
    if (this._closed) {
      throw new Error(`BatchQueue "${this._name}" is closed`);
    }

    this._queue.push(item);
    this._stats.totalAdded++;

    // Check if we should flush
    if (this._queue.length >= this._config.maxQueueSize) {
      // Force immediate flush - max queue size reached
      await this._flush();
    } else if (this._queue.length >= this._config.batchSize && !this._flushing) {
      // Trigger batch flush (non-blocking)
      this._flush().catch(this._onError);
    }
  }

  /**
   * Add multiple items at once
   *
   * @param {Array} items - Items to queue
   * @returns {Promise<void>}
   */
  async addMany(items) {
    if (!items || items.length === 0) return;

    for (const item of items) {
      await this.add(item);
    }
  }

  /**
   * Force flush all queued items immediately
   *
   * @returns {Promise<number>} Number of items flushed
   */
  async flush() {
    return this._flush();
  }

  /**
   * Internal flush implementation
   * @private
   */
  async _flush() {
    if (this._flushing || this._queue.length === 0) {
      return 0;
    }

    this._flushing = true;
    const items = this._queue.splice(0); // Take all items

    try {
      await this._flushFn(items);
      this._stats.totalFlushed += items.length;
      this._stats.flushCount++;
      this._stats.lastFlush = Date.now();

      log.debug('Batch flushed', {
        name: this._name,
        count: items.length,
        remaining: this._queue.length,
      });

      return items.length;
    } catch (err) {
      // Put items back on failure (at front of queue)
      this._queue.unshift(...items);
      this._stats.errors++;
      this._onError(err);
      return 0;
    } finally {
      this._flushing = false;
    }
  }

  /**
   * Start periodic flush timer
   * @private
   */
  _startFlushTimer() {
    if (this._flushTimer) return;

    this._flushTimer = setInterval(async () => {
      if (this._queue.length > 0 && !this._flushing) {
        await this._flush().catch(this._onError);
      }
    }, this._config.flushIntervalMs);

    // Don't prevent process exit
    if (this._flushTimer.unref) {
      this._flushTimer.unref();
    }
  }

  /**
   * Stop the flush timer
   * @private
   */
  _stopFlushTimer() {
    if (this._flushTimer) {
      clearInterval(this._flushTimer);
      this._flushTimer = null;
    }
  }

  /**
   * Close the queue, flushing remaining items
   *
   * @returns {Promise<void>}
   */
  async close() {
    this._closed = true;
    this._stopFlushTimer();

    // Final flush
    if (this._queue.length > 0) {
      await this._flush();
    }

    log.debug('BatchQueue closed', { name: this._name, stats: this._stats });
  }

  /**
   * Get queue statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this._stats,
      queueLength: this._queue.length,
      flushing: this._flushing,
      closed: this._closed,
    };
  }

  /**
   * Get current queue length
   * @returns {number}
   */
  get length() {
    return this._queue.length;
  }

  /**
   * Check if queue is empty
   * @returns {boolean}
   */
  get isEmpty() {
    return this._queue.length === 0;
  }

  /**
   * Check if queue is closed
   * @returns {boolean}
   */
  get isClosed() {
    return this._closed;
  }
}

/**
 * Create a batch queue for a specific table
 * Convenience factory for common table insert patterns
 *
 * @param {Object} options - Configuration
 * @param {PostgresClient} options.client - Database client
 * @param {string} options.table - Table name
 * @param {string[]} options.columns - Column names
 * @param {Function} options.rowMapper - Function to map item to row array
 * @param {string[]} [options.conflictColumns] - Columns for upsert conflict
 * @param {Object} [options.queueOptions] - BatchQueue options
 * @returns {BatchQueue}
 *
 * @example
 * const patternQueue = createTableBatchQueue({
 *   client: postgres,
 *   table: 'patterns',
 *   columns: ['name', 'category', 'confidence'],
 *   rowMapper: (p) => [p.name, p.category, p.confidence],
 *   conflictColumns: ['name'],
 * });
 */
export function createTableBatchQueue(options) {
  const { client, table, columns, rowMapper, conflictColumns, queueOptions = {} } = options;

  return new BatchQueue({
    name: table,
    ...queueOptions,
    flushFn: async (items) => {
      const rows = items.map(rowMapper);

      if (conflictColumns && conflictColumns.length > 0) {
        await client.batchUpsert(table, columns, rows, conflictColumns);
      } else {
        await client.batchInsert(table, columns, rows);
      }
    },
  });
}

export default BatchQueue;

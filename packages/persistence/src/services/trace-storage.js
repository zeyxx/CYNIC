/**
 * Trace Storage Service
 *
 * Buffer-based batch insert for distributed tracing spans.
 * Flushes every 8s (Fibonacci F6) or at 1000 spans.
 *
 * "φ stocke ce qui compte" - κυνικός
 *
 * @module @cynic/persistence/services/trace-storage
 */

'use strict';

import { EventEmitter } from 'events';

const FLUSH_INTERVAL_MS = 8000;  // F6 = 8s
const MAX_BUFFER_SIZE = 1000;

/**
 * TraceStorage persists spans to PostgreSQL with buffered batch inserts.
 */
export class TraceStorage extends EventEmitter {
  /**
   * @param {Object} options
   * @param {Object} options.pool - PostgreSQL connection pool (pg.Pool)
   * @param {number} [options.flushIntervalMs=8000]
   * @param {number} [options.maxBufferSize=1000]
   */
  constructor(options = {}) {
    super();
    this._pool = options.pool || null;
    this._flushIntervalMs = options.flushIntervalMs || FLUSH_INTERVAL_MS;
    this._maxBufferSize = options.maxBufferSize || MAX_BUFFER_SIZE;
    this._buffer = [];
    this._flushTimer = null;
    this._stats = {
      spansStored: 0,
      tracesCreated: 0,
      flushCount: 0,
      errors: 0,
    };

    if (this._pool) {
      this._startFlushTimer();
    }
  }

  /**
   * Buffer a span for batch insert.
   * Triggers immediate flush if buffer exceeds max size.
   *
   * @param {import('@cynic/core').Span} span
   */
  storeSpan(span) {
    if (!span || !span.context) return;

    this._buffer.push(span);

    if (this._buffer.length >= this._maxBufferSize) {
      this.flush().catch(() => {});
    }
  }

  /**
   * Flush all buffered spans to PostgreSQL.
   * Creates trace records on first span per traceId.
   *
   * @returns {Promise<number>} Number of spans flushed
   */
  async flush() {
    if (this._buffer.length === 0) return 0;

    const spans = this._buffer.splice(0);
    this._stats.flushCount++;

    if (!this._pool) {
      // No database - discard
      return 0;
    }

    const client = await this._pool.connect();
    try {
      await client.query('BEGIN');

      // Group spans by traceId to create trace records
      const traceIds = new Set();
      for (const span of spans) {
        const traceId = span.context.traceId;
        if (!traceIds.has(traceId)) {
          traceIds.add(traceId);

          // Upsert trace (first span creates it, later spans may update end_time)
          await client.query(
            `INSERT INTO traces (trace_id, root_span_id, name, start_time, end_time, duration, status, service, attributes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (trace_id) DO UPDATE SET
               end_time = GREATEST(traces.end_time, EXCLUDED.end_time),
               duration = GREATEST(traces.end_time, EXCLUDED.end_time) - traces.start_time,
               status = CASE WHEN EXCLUDED.status = 'error' THEN 'error' ELSE traces.status END`,
            [
              traceId,
              span.context.parentSpanId ? null : span.context.spanId,
              span.name,
              span.startTime,
              span.endTime,
              span.duration,
              span.status,
              span.attributes?.['service.name'] || 'cynic',
              JSON.stringify(span.attributes || {}),
            ]
          );
          this._stats.tracesCreated++;
        }

        // Insert span
        await client.query(
          `INSERT INTO spans (span_id, trace_id, parent_span_id, name, start_time, end_time, duration, status, error, attributes, events)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           ON CONFLICT (span_id) DO NOTHING`,
          [
            span.context.spanId,
            traceId,
            span.context.parentSpanId,
            span.name,
            span.startTime,
            span.endTime,
            span.duration,
            span.status,
            span.error,
            JSON.stringify(span.attributes || {}),
            JSON.stringify(span.events || []),
          ]
        );
        this._stats.spansStored++;
      }

      await client.query('COMMIT');
      this.emit('flush', { count: spans.length });
      return spans.length;
    } catch (error) {
      await client.query('ROLLBACK').catch(() => {});
      this._stats.errors++;
      this.emit('error', error);
      return 0;
    } finally {
      client.release();
    }
  }

  /**
   * Query traces with optional filters.
   *
   * @param {Object} [filters]
   * @param {string} [filters.name] - Filter by trace name (LIKE)
   * @param {string} [filters.status] - Filter by status
   * @param {number} [filters.minDuration] - Min duration in ms
   * @param {number} [filters.since] - Start time lower bound (epoch ms)
   * @param {number} [filters.limit=50] - Max results
   * @returns {Promise<Object[]>}
   */
  async queryTraces(filters = {}) {
    if (!this._pool) return [];

    const conditions = [];
    const params = [];
    let idx = 1;

    if (filters.name) {
      conditions.push(`name LIKE $${idx++}`);
      params.push(`%${filters.name}%`);
    }
    if (filters.status) {
      conditions.push(`status = $${idx++}`);
      params.push(filters.status);
    }
    if (filters.minDuration) {
      conditions.push(`duration >= $${idx++}`);
      params.push(filters.minDuration);
    }
    if (filters.since) {
      conditions.push(`start_time >= $${idx++}`);
      params.push(filters.since);
    }

    const where = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
    const limit = filters.limit || 50;

    const { rows } = await this._pool.query(
      `SELECT * FROM traces ${where} ORDER BY start_time DESC LIMIT $${idx}`,
      [...params, limit]
    );

    return rows;
  }

  /**
   * Get a full trace with all its spans as a tree.
   *
   * @param {string} traceId
   * @returns {Promise<Object|null>}
   */
  async getTrace(traceId) {
    if (!this._pool) return null;

    const { rows: [trace] } = await this._pool.query(
      'SELECT * FROM traces WHERE trace_id = $1',
      [traceId]
    );

    if (!trace) return null;

    const { rows: spans } = await this._pool.query(
      'SELECT * FROM spans WHERE trace_id = $1 ORDER BY start_time ASC',
      [traceId]
    );

    return {
      ...trace,
      spans,
      tree: this._buildTraceTree(spans),
    };
  }

  /**
   * Reconstruct parent-child hierarchy from flat spans.
   *
   * @param {Object[]} spans
   * @returns {Object[]} Root spans with nested children
   */
  _buildTraceTree(spans) {
    const byId = new Map();
    const roots = [];

    for (const span of spans) {
      byId.set(span.span_id, { ...span, children: [] });
    }

    for (const span of byId.values()) {
      if (span.parent_span_id && byId.has(span.parent_span_id)) {
        byId.get(span.parent_span_id).children.push(span);
      } else {
        roots.push(span);
      }
    }

    return roots;
  }

  /** Start the periodic flush timer */
  _startFlushTimer() {
    if (this._flushTimer) return;
    this._flushTimer = setInterval(() => {
      this.flush().catch(() => {});
    }, this._flushIntervalMs);
    if (this._flushTimer.unref) this._flushTimer.unref();
  }

  /** Stop and clean up */
  async stop() {
    if (this._flushTimer) {
      clearInterval(this._flushTimer);
      this._flushTimer = null;
    }
    // Final flush
    await this.flush().catch(() => {});
  }

  /** @returns {Object} Storage statistics */
  get stats() {
    return {
      ...this._stats,
      bufferSize: this._buffer.length,
    };
  }
}

/**
 * Factory function.
 * @param {Object} options
 * @returns {TraceStorage}
 */
export function createTraceStorage(options) {
  return new TraceStorage(options);
}

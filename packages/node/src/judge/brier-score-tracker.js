/**
 * Brier Score Tracker - Measures prediction sharpness (decisiveness)
 *
 * Complements ECE (calibration) with sharpness measurement.
 * "φ questions even its own sharpness" - κυνικός
 *
 * Brier Score = (1/N) · Σ (p_i - o_i)²
 * where:
 *   p_i = predicted probability
 *   o_i = actual outcome (0 or 1)
 *   N = number of predictions
 *
 * Interpretation:
 * - Perfect: Brier = 0
 * - Baseline (always predict 0.5): Brier = 0.25
 * - Good: Brier < 0.10
 * - Acceptable: Brier < 0.20
 *
 * @module @cynic/node/judge/brier-score-tracker
 */

'use strict';

import { getPool } from '@cynic/persistence';
import { globalEventBus, EventType } from '@cynic/core';

// Simple logger
const log = {
  debug: (mod, msg, data) => process.env.CYNIC_DEBUG && console.debug(`[${mod}]`, msg, data || ''),
  info: (mod, msg, data) => console.log(`[${mod}]`, msg, data || ''),
  warn: (mod, msg, data) => console.warn(`[${mod}]`, msg, data || ''),
  error: (mod, msg, data) => console.error(`[${mod}]`, msg, data || ''),
};

// ═══════════════════════════════════════════════════════════════════════════
// THRESHOLDS
// ═══════════════════════════════════════════════════════════════════════════

const BASELINE_BRIER = 0.25; // Always predicting 0.5
const GOOD_BRIER = 0.10;
const ACCEPTABLE_BRIER = 0.20;

/**
 * Brier Score Tracker
 *
 * Tracks sharpness (decisiveness) of CYNIC's predictions.
 * Separate from calibration - see learning-validation.md §2.4.
 */
export class BrierScoreTracker {
  constructor(options = {}) {
    this.pool = options.pool || getPool();
    this.serviceId = options.serviceId || 'default';

    // In-memory buffer for batching
    this._buffer = [];
    this._bufferLimit = options.bufferLimit || 50;
    this._flushDebounce = null;
    this._flushIntervalMs = options.flushIntervalMs || 60000; // 1 min

    // Stats
    this.stats = {
      predictions: 0,
      currentBrier: null,
      lastCheck: null,
    };
  }

  /**
   * Record a prediction and its outcome
   *
   * @param {number} predicted - Predicted probability (0-1)
   * @param {boolean} actual - Actual outcome (true/false)
   * @param {Object} [metadata={}] - Optional metadata
   */
  record(predicted, actual, metadata = {}) {
    // Validate inputs
    if (typeof predicted !== 'number' || predicted < 0 || predicted > 1) {
      log.warn('BrierScoreTracker', 'Invalid predicted probability', { predicted });
      return;
    }

    if (typeof actual !== 'boolean') {
      log.warn('BrierScoreTracker', 'Invalid actual outcome', { actual });
      return;
    }

    // Add to buffer
    this._buffer.push({
      predicted,
      actual: actual ? 1 : 0,
      timestamp: new Date(),
      metadata,
    });

    this.stats.predictions++;

    // Flush if buffer full
    if (this._buffer.length >= this._bufferLimit) {
      this._flush();
    } else {
      this._scheduleFlush();
    }
  }

  /**
   * Get current Brier Score
   *
   * @param {number} [days=30] - Number of days to include
   * @returns {Promise<Object>} Brier score breakdown
   */
  async getBrierScore(days = 30) {
    try {
      const { rows } = await this.pool.query(`
        SELECT
          COUNT(*) as total,
          AVG(POW(predicted - actual, 2)) as brier_score,
          STDDEV(POW(predicted - actual, 2)) as brier_std
        FROM brier_predictions
        WHERE service_id = $1
        AND timestamp > NOW() - INTERVAL '${days} days'
      `, [this.serviceId]);

      if (!rows[0] || rows[0].total === 0) {
        return {
          brierScore: null,
          total: 0,
          status: 'insufficient_data',
        };
      }

      const brierScore = parseFloat(rows[0].brier_score);
      const brierStd = parseFloat(rows[0].brier_std || 0);

      // Determine status
      let status = 'poor';
      if (brierScore < GOOD_BRIER) {
        status = 'good';
      } else if (brierScore < ACCEPTABLE_BRIER) {
        status = 'acceptable';
      }

      this.stats.currentBrier = brierScore;
      this.stats.lastCheck = new Date();

      return {
        brierScore,
        brierStd,
        total: parseInt(rows[0].total),
        status,
        thresholds: {
          baseline: BASELINE_BRIER,
          acceptable: ACCEPTABLE_BRIER,
          good: GOOD_BRIER,
        },
        comparison: {
          vsBaseline: brierScore / BASELINE_BRIER,
          vsGood: brierScore / GOOD_BRIER,
        },
      };
    } catch (error) {
      log.error('BrierScoreTracker', 'Failed to compute Brier score', {
        error: error.message,
      });
      throw error;
    }
  }

  /**
   * Get Brier Score trend over time
   *
   * @param {number} [days=30] - Number of days
   * @param {number} [buckets=7] - Number of time buckets
   * @returns {Promise<Array>} Time series of Brier scores
   */
  async getBrierTrend(days = 30, buckets = 7) {
    try {
      const { rows } = await this.pool.query(`
        SELECT
          DATE_TRUNC('day', timestamp) as day,
          AVG(POW(predicted - actual, 2)) as brier_score,
          COUNT(*) as count
        FROM brier_predictions
        WHERE service_id = $1
        AND timestamp > NOW() - INTERVAL '${days} days'
        GROUP BY DATE_TRUNC('day', timestamp)
        ORDER BY day ASC
      `, [this.serviceId]);

      return rows.map(row => ({
        day: row.day,
        brierScore: parseFloat(row.brier_score),
        count: parseInt(row.count),
      }));
    } catch (error) {
      log.error('BrierScoreTracker', 'Failed to compute Brier trend', {
        error: error.message,
      });
      throw error;
    }
  }

  // =============================================================================
  // PRIVATE METHODS
  // =============================================================================

  /**
   * Schedule flush with debounce
   * @private
   */
  _scheduleFlush() {
    if (this._flushDebounce) {
      clearTimeout(this._flushDebounce);
    }

    this._flushDebounce = setTimeout(() => {
      this._flush();
    }, this._flushIntervalMs);
  }

  /**
   * Flush buffer to database
   * @private
   */
  async _flush() {
    if (this._buffer.length === 0) return;

    const batch = this._buffer.splice(0, this._buffer.length);

    try {
      // Insert all predictions
      const values = batch.map((p, i) => {
        const offset = i * 4;
        return `($${offset + 1}, $${offset + 2}, $${offset + 3}, $${offset + 4})`;
      }).join(', ');

      const params = batch.flatMap(p => [
        this.serviceId,
        p.predicted,
        p.actual,
        JSON.stringify(p.metadata || {}),
      ]);

      await this.pool.query(`
        INSERT INTO brier_predictions (service_id, predicted, actual, metadata)
        VALUES ${values}
      `, params);

      log.debug('BrierScoreTracker', 'Flushed predictions', {
        count: batch.length,
      });

      // Emit event
      globalEventBus.emit(EventType.BRIER_UPDATED, {
        serviceId: this.serviceId,
        count: batch.length,
        timestamp: new Date(),
      });
    } catch (error) {
      log.error('BrierScoreTracker', 'Failed to flush predictions', {
        error: error.message,
        count: batch.length,
      });

      // Re-add to buffer on failure
      this._buffer.unshift(...batch);
    }
  }

  /**
   * Shutdown tracker (flush remaining)
   */
  async shutdown() {
    if (this._flushDebounce) {
      clearTimeout(this._flushDebounce);
    }

    await this._flush();
  }
}

// Singleton instance
let _instance = null;

/**
 * Get singleton instance
 */
export function getBrierScoreTracker() {
  if (!_instance) {
    _instance = new BrierScoreTracker();
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetBrierScoreTracker() {
  if (_instance) {
    _instance.shutdown();
  }
  _instance = null;
}

export default BrierScoreTracker;

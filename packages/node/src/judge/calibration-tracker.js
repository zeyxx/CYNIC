/**
 * Calibration Tracker - Monitors prediction accuracy and calibration drift
 *
 * "φ distrusts φ" - CYNIC monitors its own confidence calibration
 *
 * Tracks:
 * - Predicted confidence vs actual outcomes
 * - Calibration curve (expected vs observed accuracy per confidence bucket)
 * - Drift detection with alerts
 *
 * A well-calibrated model should have:
 * - 60% confidence predictions correct ~60% of the time
 * - Linear calibration curve (diagonal)
 *
 * @module @cynic/node/judge/calibration-tracker
 */

'use strict';

import { getPool } from '@cynic/persistence';
import { globalEventBus, EventType } from '@cynic/core';

// Simple logger (no external dependency)
const log = {
  debug: (mod, msg, data) => process.env.CYNIC_DEBUG && console.debug(`[${mod}]`, msg, data || ''),
  info: (mod, msg, data) => console.log(`[${mod}]`, msg, data || ''),
  warn: (mod, msg, data) => console.warn(`[${mod}]`, msg, data || ''),
  error: (mod, msg, data) => console.error(`[${mod}]`, msg, data || ''),
};

// ═══════════════════════════════════════════════════════════════════════════
// φ CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

const PHI_INV = 0.618033988749895;  // φ⁻¹ - max confidence
const PHI_INV_2 = 0.381966011250105; // φ⁻² - drift alert threshold

// Calibration buckets: 0-10%, 10-20%, ..., 90-100%
const NUM_BUCKETS = 10;
const BUCKET_SIZE = 10;

/**
 * Calibration Tracker
 *
 * Monitors CYNIC's prediction calibration and detects drift.
 */
export class CalibrationTracker {
  constructor(options = {}) {
    this.pool = options.pool || getPool();
    this.serviceId = options.serviceId || 'default';

    // Drift detection config
    // Industry standard: 10% (0.10) - lowered from φ⁻² (38.2%) per learning-validation.md
    this.driftThreshold = options.driftThreshold || 0.10; // 10% ECE threshold
    this.minSamplesForAlert = options.minSamplesForAlert || 10;
    this.alertCooldownMs = options.alertCooldownMs || 30 * 60 * 1000; // 30 min

    // In-memory buffer for batching
    this._buffer = [];
    this._bufferLimit = options.bufferLimit || 50;
    this._flushDebounce = null;
    this._flushIntervalMs = options.flushIntervalMs || 60000; // 1 min

    // Alert state
    this._lastAlertAt = 0;
    this._driftAlerts = [];

    // Stats
    this.stats = {
      predictions: 0,
      correct: 0,
      lastCalibrationCheck: null,
      currentECE: null, // Expected Calibration Error
      driftAlerts: 0,
    };
  }

  /**
   * Record a prediction and its outcome
   *
   * @param {Object} prediction - Prediction data
   * @param {string} prediction.predicted - Predicted outcome
   * @param {string} prediction.actual - Actual outcome
   * @param {number} prediction.confidence - Confidence (0-1)
   * @param {string} [prediction.contextType] - Context type
   * @param {string} [prediction.predictionId] - Optional ID for tracking
   */
  record(prediction) {
    const {
      predicted,
      actual,
      confidence,
      contextType = 'general',
      predictionId = null,
    } = prediction;

    if (confidence === undefined || confidence === null) {
      log.warn('CalibrationTracker', 'Missing confidence in prediction');
      return;
    }

    // Clamp confidence to [0, φ⁻¹]
    const clampedConfidence = Math.min(PHI_INV, Math.max(0, confidence));

    // Calculate bucket (0-9)
    const bucket = Math.min(NUM_BUCKETS - 1, Math.floor(clampedConfidence * 10));

    // Add to buffer
    this._buffer.push({
      predicted_outcome: predicted,
      actual_outcome: actual,
      predicted_confidence: clampedConfidence,
      confidence_bucket: bucket,
      context_type: contextType,
      prediction_id: predictionId,
      created_at: new Date().toISOString(),
    });

    // Update local stats
    this.stats.predictions++;
    if (predicted === actual) {
      this.stats.correct++;
    }

    // Flush if buffer full
    if (this._buffer.length >= this._bufferLimit) {
      this.flush();
    } else {
      // Debounce flush
      this._scheduleFlush();
    }
  }

  /**
   * Schedule a debounced flush
   * @private
   */
  _scheduleFlush() {
    if (this._flushDebounce) return;

    this._flushDebounce = setTimeout(() => {
      this._flushDebounce = null;
      this.flush().catch(err => {
        log.error('CalibrationTracker', 'Scheduled flush failed', { error: err.message });
      });
    }, this._flushIntervalMs);
    this._flushDebounce.unref();
  }

  /**
   * Flush buffer to database
   *
   * @returns {Promise<number>} Number of records flushed
   */
  async flush() {
    if (this._buffer.length === 0) {
      return 0;
    }

    const toFlush = [...this._buffer];
    this._buffer = [];

    try {
      // Batch insert (includes prediction_id for feedback matching)
      const values = toFlush.map((p, i) => {
        const base = i * 7;
        return `($${base + 1}, $${base + 2}, $${base + 3}, $${base + 4}, $${base + 5}, $${base + 6}, $${base + 7})`;
      }).join(', ');

      const params = toFlush.flatMap(p => [
        this.serviceId,
        p.predicted_outcome,
        p.actual_outcome,
        p.predicted_confidence,
        p.confidence_bucket,
        p.context_type,
        p.prediction_id,
      ]);

      await this.pool.query(`
        INSERT INTO calibration_tracking
        (service_id, predicted_outcome, actual_outcome, predicted_confidence, confidence_bucket, context_type, prediction_id)
        VALUES ${values}
      `, params);

      log.debug('CalibrationTracker', `Flushed ${toFlush.length} predictions`);

      // Check drift after flush (throttled by alertCooldownMs)
      this._checkDriftAfterFlush();

      return toFlush.length;

    } catch (err) {
      // Put records back in buffer on failure
      this._buffer = [...toFlush, ...this._buffer].slice(0, this._bufferLimit * 2);
      log.error('CalibrationTracker', 'Flush failed', { error: err.message });
      throw err;
    }
  }

  /**
   * Update actual outcome for a prediction (closes the feedback loop)
   *
   * @param {string} predictionId - Judgment ID that was used as prediction_id
   * @param {string} actual - Actual outcome ('HOWL', 'WAG', 'GROWL', 'BARK', or user verdict)
   * @returns {Promise<boolean>} True if updated
   */
  async updateActual(predictionId, actual) {
    if (!predictionId || !actual) return false;
    try {
      const { rowCount } = await this.pool.query(`
        UPDATE calibration_tracking
        SET actual_outcome = $1
        WHERE prediction_id = $2 AND actual_outcome IS NULL
      `, [actual, predictionId]);
      return rowCount > 0;
    } catch (err) {
      log.debug('CalibrationTracker', 'updateActual failed', { error: err.message });
      return false;
    }
  }

  /**
   * Check for drift after flush and emit event if detected
   * Throttled by _shouldAlert() cooldown
   * @private
   */
  async _checkDriftAfterFlush() {
    if (!this._shouldAlert()) return;
    try {
      const { summary } = await this.getCalibrationCurve(7);
      if (summary.driftDetected) {
        globalEventBus.emit(EventType.CALIBRATION_DRIFT_DETECTED, {
          payload: {
            ece: summary.ece,
            threshold: summary.threshold,
            totalSamples: summary.totalSamples,
          },
        });
      }
    } catch (err) {
      log.debug('CalibrationTracker', 'Drift check failed (non-blocking)', { error: err.message });
    }
  }

  /**
   * Get calibration curve data
   *
   * @param {number} [days=7] - Number of days to analyze
   * @returns {Promise<Object>} Calibration data
   */
  async getCalibrationCurve(days = 7) {
    // Ensure buffer is flushed
    await this.flush();

    const { rows } = await this.pool.query(`
      SELECT * FROM get_calibration_curve($1, $2)
    `, [this.serviceId, days]);

    // Calculate Expected Calibration Error (ECE)
    let ece = 0;
    let totalSamples = 0;

    for (const row of rows) {
      const sampleCount = parseInt(row.sample_count) || 0;
      const calibrationError = parseFloat(row.calibration_error) || 0;

      ece += sampleCount * calibrationError;
      totalSamples += sampleCount;
    }

    ece = totalSamples > 0 ? ece / totalSamples : 0;

    // Check for drift
    const driftDetected = ece > this.driftThreshold;
    if (driftDetected && this._shouldAlert()) {
      this._recordDriftAlert(ece);
    }

    // Update stats
    this.stats.lastCalibrationCheck = new Date().toISOString();
    this.stats.currentECE = ece;

    return {
      curve: rows.map(r => ({
        bucket: r.bucket,
        predictedRate: parseFloat(r.predicted_rate),
        actualRate: parseFloat(r.actual_rate) || 0,
        sampleCount: parseInt(r.sample_count),
        calibrationError: parseFloat(r.calibration_error) || 0,
      })),
      summary: {
        ece,
        totalSamples,
        driftDetected,
        threshold: this.driftThreshold,
        isWellCalibrated: ece <= this.driftThreshold,
      },
    };
  }

  /**
   * Check if we should send a drift alert (respects cooldown)
   * @private
   */
  _shouldAlert() {
    const now = Date.now();
    return now - this._lastAlertAt > this.alertCooldownMs;
  }

  /**
   * Record a drift alert
   * @private
   */
  _recordDriftAlert(ece) {
    this._lastAlertAt = Date.now();
    this.stats.driftAlerts++;

    const alert = {
      timestamp: new Date().toISOString(),
      ece,
      threshold: this.driftThreshold,
      message: `Calibration drift detected: ECE=${(ece * 100).toFixed(1)}% > ${(this.driftThreshold * 100).toFixed(1)}%`,
    };

    this._driftAlerts.push(alert);
    if (this._driftAlerts.length > 10) {
      this._driftAlerts.shift();
    }

    log.warn('CalibrationTracker', alert.message, { ece, threshold: this.driftThreshold });
  }

  /**
   * Get recent drift alerts
   *
   * @returns {Object[]} Recent alerts
   */
  getDriftAlerts() {
    return [...this._driftAlerts];
  }

  /**
   * Get tracker statistics
   *
   * @returns {Object} Stats
   */
  getStats() {
    const accuracy = this.stats.predictions > 0 ?
      this.stats.correct / this.stats.predictions : 0;

    return {
      ...this.stats,
      accuracy,
      bufferSize: this._buffer.length,
      recentAlerts: this._driftAlerts.length,
    };
  }

  /**
   * Get overall accuracy by context type
   *
   * @param {number} [days=7] - Number of days
   * @returns {Promise<Object[]>} Accuracy by context
   */
  async getAccuracyByContext(days = 7) {
    const { rows } = await this.pool.query(`
      SELECT
        context_type,
        COUNT(*) as total,
        SUM(CASE WHEN predicted_outcome = actual_outcome THEN 1 ELSE 0 END) as correct,
        AVG(predicted_confidence) as avg_confidence
      FROM calibration_tracking
      WHERE service_id = $1
        AND created_at > NOW() - ($2 || ' days')::INTERVAL
      GROUP BY context_type
      ORDER BY total DESC
    `, [this.serviceId, days]);

    return rows.map(r => ({
      contextType: r.context_type,
      total: parseInt(r.total),
      correct: parseInt(r.correct),
      accuracy: parseInt(r.total) > 0 ? parseInt(r.correct) / parseInt(r.total) : 0,
      avgConfidence: parseFloat(r.avg_confidence) || 0,
    }));
  }

  /**
   * Clean up old calibration data
   *
   * @param {number} [days=30] - Keep data from last N days
   * @returns {Promise<number>} Deleted count
   */
  async cleanup(days = 30) {
    const { rows } = await this.pool.query(`
      SELECT cleanup_calibration_data($1, $2) as deleted
    `, [this.serviceId, days]);

    const deleted = parseInt(rows[0]?.deleted || 0);
    log.info('CalibrationTracker', `Cleaned up ${deleted} old records`);

    return deleted;
  }

  /**
   * Close tracker (flush buffer)
   */
  async close() {
    if (this._flushDebounce) {
      clearTimeout(this._flushDebounce);
      this._flushDebounce = null;
    }
    await this.flush();
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create the CalibrationTracker singleton
 *
 * @param {Object} options - Tracker options
 * @returns {CalibrationTracker} Singleton instance
 */
export function getCalibrationTracker(options = {}) {
  if (!_instance) {
    if (!options.pool) {
      try { options.pool = getPool(); } catch { return null; }
    }
    _instance = new CalibrationTracker(options);
  }
  return _instance;
}

/**
 * Reset the singleton (for testing)
 */
export function resetCalibrationTracker() {
  if (_instance) {
    _instance.close();
  }
  _instance = null;
}

export default {
  CalibrationTracker,
  getCalibrationTracker,
  resetCalibrationTracker,
};

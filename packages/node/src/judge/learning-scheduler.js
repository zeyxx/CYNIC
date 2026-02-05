/**
 * Learning Scheduler - Daily automated learning cycles
 *
 * "φ learns while you sleep" - Scheduled DPO optimization
 *
 * Runs learning cycles on a configurable schedule:
 * - Daily DPO optimization
 * - Calibration checks
 * - Residual governance reviews
 *
 * @module @cynic/node/judge/learning-scheduler
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { EventEmitter } from 'events';

// Simple logger
const log = {
  debug: (mod, msg, data) => process.env.CYNIC_DEBUG && console.debug(`[${mod}]`, msg, data || ''),
  info: (mod, msg, data) => console.log(`[${mod}]`, msg, data || ''),
  warn: (mod, msg, data) => console.warn(`[${mod}]`, msg, data || ''),
  error: (mod, msg, data) => console.error(`[${mod}]`, msg, data || ''),
};

// ═══════════════════════════════════════════════════════════════════════════
// φ-ALIGNED SCHEDULE DEFAULTS
// ═══════════════════════════════════════════════════════════════════════════

const DEFAULT_SCHEDULE = {
  // DPO optimization - run at 3 AM daily (low activity period)
  dpoHour: 3,
  dpoMinute: 0,

  // Calibration check - run every 6 hours
  calibrationIntervalHours: 6,

  // Governance review - run at 4 AM daily
  governanceHour: 4,
  governanceMinute: 0,

  // Minimum interval between runs (prevent rapid retries)
  minIntervalMs: 60 * 60 * 1000, // 1 hour

  // Enable/disable individual schedules
  enableDPO: true,
  enableCalibration: true,
  enableGovernance: true,
};

/**
 * Learning Scheduler
 *
 * Manages scheduled learning tasks.
 */
export class LearningScheduler extends EventEmitter {
  constructor(options = {}) {
    super();

    // Components
    this.learningManager = options.learningManager || null;
    this.dpoOptimizer = options.dpoOptimizer || null;
    this.calibrationTracker = options.calibrationTracker || null;
    this.residualGovernance = options.residualGovernance || null;

    // Schedule configuration
    this.schedule = { ...DEFAULT_SCHEDULE, ...options.schedule };

    // Timers
    this._timers = {
      dpo: null,
      calibration: null,
      governance: null,
    };

    // Last run timestamps
    this._lastRun = {
      dpo: null,
      calibration: null,
      governance: null,
    };

    // Stats
    this.stats = {
      dpoRuns: 0,
      calibrationRuns: 0,
      governanceRuns: 0,
      errors: 0,
      lastDPOResult: null,
      lastCalibrationResult: null,
      lastGovernanceResult: null,
    };

    this._running = false;
  }

  /**
   * Set dependencies (for late binding)
   */
  setDependencies({ learningManager, dpoOptimizer, calibrationTracker, residualGovernance }) {
    if (learningManager) this.learningManager = learningManager;
    if (dpoOptimizer) this.dpoOptimizer = dpoOptimizer;
    if (calibrationTracker) this.calibrationTracker = calibrationTracker;
    if (residualGovernance) this.residualGovernance = residualGovernance;
  }

  /**
   * Start the scheduler
   */
  start() {
    if (this._running) {
      log.warn('LearningScheduler', 'Already running');
      return;
    }

    this._running = true;
    log.info('LearningScheduler', 'Starting learning scheduler', {
      dpo: this.schedule.enableDPO,
      calibration: this.schedule.enableCalibration,
      governance: this.schedule.enableGovernance,
    });

    // Schedule DPO
    if (this.schedule.enableDPO) {
      this._scheduleDPO();
    }

    // Schedule calibration
    if (this.schedule.enableCalibration) {
      this._scheduleCalibration();
    }

    // Schedule governance
    if (this.schedule.enableGovernance) {
      this._scheduleGovernance();
    }

    this.emit('started');
  }

  /**
   * Stop the scheduler
   */
  stop() {
    this._running = false;

    for (const key of Object.keys(this._timers)) {
      if (this._timers[key]) {
        clearTimeout(this._timers[key]);
        this._timers[key] = null;
      }
    }

    log.info('LearningScheduler', 'Scheduler stopped');
    this.emit('stopped');
  }

  /**
   * Schedule next DPO run
   * @private
   */
  _scheduleDPO() {
    const msUntilNext = this._getMsUntilTime(
      this.schedule.dpoHour,
      this.schedule.dpoMinute
    );

    log.debug('LearningScheduler', `DPO scheduled in ${Math.round(msUntilNext / 60000)} minutes`);

    this._timers.dpo = setTimeout(async () => {
      await this._runDPO();
      if (this._running) {
        this._scheduleDPO();
      }
    }, msUntilNext);
  }

  /**
   * Schedule next calibration check
   * @private
   */
  _scheduleCalibration() {
    const intervalMs = this.schedule.calibrationIntervalHours * 60 * 60 * 1000;

    log.debug('LearningScheduler', `Calibration scheduled in ${this.schedule.calibrationIntervalHours} hours`);

    this._timers.calibration = setTimeout(async () => {
      await this._runCalibration();
      if (this._running) {
        this._scheduleCalibration();
      }
    }, intervalMs);
  }

  /**
   * Schedule next governance review
   * @private
   */
  _scheduleGovernance() {
    const msUntilNext = this._getMsUntilTime(
      this.schedule.governanceHour,
      this.schedule.governanceMinute
    );

    log.debug('LearningScheduler', `Governance scheduled in ${Math.round(msUntilNext / 60000)} minutes`);

    this._timers.governance = setTimeout(async () => {
      await this._runGovernance();
      if (this._running) {
        this._scheduleGovernance();
      }
    }, msUntilNext);
  }

  /**
   * Calculate ms until target time (next occurrence)
   * @private
   */
  _getMsUntilTime(hour, minute) {
    const now = new Date();
    const target = new Date(now);
    target.setHours(hour, minute, 0, 0);

    // If target time has passed today, schedule for tomorrow
    if (target <= now) {
      target.setDate(target.getDate() + 1);
    }

    return target.getTime() - now.getTime();
  }

  /**
   * Run DPO optimization
   * @private
   */
  async _runDPO() {
    // Check minimum interval
    if (this._lastRun.dpo && Date.now() - this._lastRun.dpo < this.schedule.minIntervalMs) {
      log.debug('LearningScheduler', 'DPO skipped - too soon since last run');
      return;
    }

    log.info('LearningScheduler', 'Running scheduled DPO optimization');

    try {
      let result;

      // Use LearningManager if available (runs full cycle including DPO)
      if (this.learningManager) {
        result = await this.learningManager.runLearningCycle();
      } else if (this.dpoOptimizer) {
        result = await this.dpoOptimizer.optimize();
      } else {
        log.warn('LearningScheduler', 'No DPO optimizer available');
        return;
      }

      this._lastRun.dpo = Date.now();
      this.stats.dpoRuns++;
      this.stats.lastDPOResult = {
        timestamp: new Date().toISOString(),
        success: true,
        ...result,
      };

      this.emit('dpo:complete', result);
      log.info('LearningScheduler', 'DPO optimization complete', result);

    } catch (err) {
      this.stats.errors++;
      this.stats.lastDPOResult = {
        timestamp: new Date().toISOString(),
        success: false,
        error: err.message,
      };

      log.error('LearningScheduler', 'DPO optimization failed', { error: err.message });
      this.emit('dpo:error', err);
    }
  }

  /**
   * Run calibration check
   * @private
   */
  async _runCalibration() {
    if (!this.calibrationTracker) {
      log.debug('LearningScheduler', 'No calibration tracker available');
      return;
    }

    log.info('LearningScheduler', 'Running scheduled calibration check');

    try {
      const result = await this.calibrationTracker.getCalibrationCurve(7);

      this._lastRun.calibration = Date.now();
      this.stats.calibrationRuns++;
      this.stats.lastCalibrationResult = {
        timestamp: new Date().toISOString(),
        success: true,
        ece: result.summary.ece,
        driftDetected: result.summary.driftDetected,
        totalSamples: result.summary.totalSamples,
      };

      this.emit('calibration:complete', result);

      // Log warning if drift detected
      if (result.summary.driftDetected) {
        log.warn('LearningScheduler', `Calibration drift detected: ECE=${(result.summary.ece * 100).toFixed(1)}%`);
      } else {
        log.info('LearningScheduler', `Calibration check complete: ECE=${(result.summary.ece * 100).toFixed(1)}%`);
      }

    } catch (err) {
      this.stats.errors++;
      this.stats.lastCalibrationResult = {
        timestamp: new Date().toISOString(),
        success: false,
        error: err.message,
      };

      log.error('LearningScheduler', 'Calibration check failed', { error: err.message });
      this.emit('calibration:error', err);
    }
  }

  /**
   * Run governance review
   * @private
   */
  async _runGovernance() {
    if (!this.residualGovernance) {
      log.debug('LearningScheduler', 'No governance service available');
      return;
    }

    log.info('LearningScheduler', 'Running scheduled governance review');

    try {
      const result = await this.residualGovernance.reviewCandidates();

      this._lastRun.governance = Date.now();
      this.stats.governanceRuns++;
      this.stats.lastGovernanceResult = {
        timestamp: new Date().toISOString(),
        success: true,
        ...result,
      };

      this.emit('governance:complete', result);
      log.info('LearningScheduler', 'Governance review complete', result);

    } catch (err) {
      this.stats.errors++;
      this.stats.lastGovernanceResult = {
        timestamp: new Date().toISOString(),
        success: false,
        error: err.message,
      };

      log.error('LearningScheduler', 'Governance review failed', { error: err.message });
      this.emit('governance:error', err);
    }
  }

  /**
   * Run all tasks immediately (for testing or manual trigger)
   *
   * @returns {Promise<Object>} Results of all runs
   */
  async runNow() {
    log.info('LearningScheduler', 'Running all learning tasks immediately');

    const results = {
      dpo: null,
      calibration: null,
      governance: null,
    };

    // Run DPO
    if (this.schedule.enableDPO) {
      await this._runDPO();
      results.dpo = this.stats.lastDPOResult;
    }

    // Run calibration
    if (this.schedule.enableCalibration) {
      await this._runCalibration();
      results.calibration = this.stats.lastCalibrationResult;
    }

    // Run governance
    if (this.schedule.enableGovernance) {
      await this._runGovernance();
      results.governance = this.stats.lastGovernanceResult;
    }

    return results;
  }

  /**
   * Get next scheduled run times
   *
   * @returns {Object} Next run times
   */
  getNextRuns() {
    const now = new Date();

    const getDailyTime = (hour, minute) => {
      const target = new Date(now);
      target.setHours(hour, minute, 0, 0);
      if (target <= now) {
        target.setDate(target.getDate() + 1);
      }
      return target;
    };

    return {
      dpo: this.schedule.enableDPO
        ? getDailyTime(this.schedule.dpoHour, this.schedule.dpoMinute).toISOString()
        : null,
      calibration: this.schedule.enableCalibration
        ? new Date(now.getTime() + this.schedule.calibrationIntervalHours * 60 * 60 * 1000).toISOString()
        : null,
      governance: this.schedule.enableGovernance
        ? getDailyTime(this.schedule.governanceHour, this.schedule.governanceMinute).toISOString()
        : null,
    };
  }

  /**
   * Get scheduler statistics
   *
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      running: this._running,
      lastRuns: this._lastRun,
      nextRuns: this._running ? this.getNextRuns() : null,
      schedule: this.schedule,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create the LearningScheduler singleton
 *
 * @param {Object} options - Scheduler options
 * @returns {LearningScheduler} Singleton instance
 */
export function getLearningScheduler(options = {}) {
  if (!_instance) {
    _instance = new LearningScheduler(options);
  }
  return _instance;
}

/**
 * Reset the singleton (for testing)
 */
export function resetLearningScheduler() {
  if (_instance) {
    _instance.stop();
  }
  _instance = null;
}

export default {
  LearningScheduler,
  getLearningScheduler,
  resetLearningScheduler,
  DEFAULT_SCHEDULE,
};

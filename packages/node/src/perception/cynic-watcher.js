/**
 * CYNIC Watcher - C6.1 (CYNIC × PERCEIVE)
 *
 * Self-observation: CYNIC watching its own internal state.
 * Monitors database health, memory usage, budget, learning maturity,
 * event throughput, and context compression.
 *
 * Feeds CynicEmergence (C6.7) for meta-pattern detection:
 * - Budget exhaustion patterns
 * - Memory pressure cycles
 * - Learning plateau detection
 * - Event storm detection
 *
 * "The dog who knows itself" - κυνικός
 *
 * @module @cynic/node/perception/cynic-watcher
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, globalEventBus, EventType, PHI_INV } from '@cynic/core';
import { getPool } from '@cynic/persistence';
import { getCostLedger } from '../accounting/cost-ledger.js';
import { getEventBus } from '../services/event-bus.js';
import contextCompressor from '../services/context-compressor.js';

const log = createLogger('CynicWatcher');

/**
 * Cynic event types (self-observation signals)
 * @readonly
 * @enum {string}
 */
export const CynicEventType = {
  HEALTH_UPDATE: 'perception:cynic:health',
  BUDGET_WARNING: 'perception:cynic:budget',
  MEMORY_PRESSURE: 'perception:cynic:memory',
  LEARNING_MILESTONE: 'perception:cynic:learning',
  EVENT_STORM: 'perception:cynic:events',
  DB_DEGRADATION: 'perception:cynic:db',
};

/**
 * Default poll intervals (φ-aligned)
 */
const POLL_INTERVALS = {
  health: 60 * 1000,        // 1 min (fast metrics)
  db: 5 * 60 * 1000,        // 5 min (slow queries)
  learning: 3 * 60 * 1000,  // 3 min (maturity signals)
};

/**
 * Health thresholds (φ-aligned)
 */
const THRESHOLDS = {
  memory: {
    cautious: 0.618,  // φ⁻¹ heap usage
    critical: 0.80,   // 80% heap usage
  },
  budget: {
    cautious: 0.618,  // φ⁻¹ budget consumed
    critical: 0.95,   // 95% budget consumed
  },
  events: {
    storm: 1000,      // events/min threshold
  },
  db: {
    slowQuery: 100,   // ms — queries slower than this
    sizeCritical: 500, // MB — database size warning
  },
};

/**
 * CynicWatcher - Observes CYNIC's own internal state
 *
 * Implements C6.1 (CYNIC × PERCEIVE) in 7×7 matrix.
 * Feeds CynicEmergence (C6.7) for self-awareness patterns.
 */
export class CynicWatcher extends EventEmitter {
  /**
   * Create CynicWatcher
   *
   * @param {Object} [options]
   * @param {Object} [options.db] - PostgreSQL pool (defaults to getPool())
   * @param {EventBus} [options.eventBus] - EventBus instance
   * @param {CostLedger} [options.costLedger] - CostLedger instance
   * @param {ContextCompressor} [options.contextCompressor] - ContextCompressor instance
   */
  constructor(options = {}) {
    super();

    this.db = options.db || null; // Lazy init on start
    this.eventBus = options.eventBus || getEventBus();
    this.costLedger = options.costLedger || null; // Lazy init
    this.contextCompressor = options.contextCompressor || contextCompressor;

    this._timers = new Map(); // type -> timer
    this._isRunning = false;

    // Event throughput tracking
    this._eventCounts = {
      lastMinute: 0,
      lastHour: 0,
      lastReset: Date.now(),
    };

    // EventBusBridge metrics (if available)
    this._eventBridgeMetrics = null;

    // SONA instance (lazy init)
    this._sona = null;

    // Stats
    this.stats = {
      healthChecks: 0,
      dbQueries: 0,
      warnings: 0,
      errors: 0,
      lastPollAt: null,
    };
  }

  /**
   * Start watching CYNIC's internal state
   */
  async start() {
    if (this._isRunning) {
      log.debug('CynicWatcher already running');
      return;
    }

    try {
      // Lazy init dependencies
      if (!this.db) {
        this.db = getPool();
      }
      if (!this.costLedger) {
        this.costLedger = getCostLedger();
      }

      // Test database connection
      await this.db.query('SELECT 1');
      log.info('CynicWatcher connected to PostgreSQL');

      this._isRunning = true;

      // Start polling loops
      this._startHealthPolling();
      this._startDbPolling();
      this._startLearningPolling();

      // Wire event listeners
      this._wireEventListeners();

      // Emit startup event
      globalEventBus.emit(EventType.WATCHER_STARTED || 'watcher:started', {
        watcher: 'cynic',
        timestamp: Date.now(),
      });

      log.info('CynicWatcher started');
    } catch (error) {
      log.error('Failed to start CynicWatcher', { error: error.message });
      this._isRunning = false;
      throw error;
    }
  }

  /**
   * Stop watching
   */
  async stop() {
    if (!this._isRunning) return;

    this._isRunning = false;

    // Clear all timers
    for (const timer of this._timers.values()) {
      clearInterval(timer);
    }
    this._timers.clear();

    globalEventBus.emit(EventType.WATCHER_STOPPED || 'watcher:stopped', {
      watcher: 'cynic',
      timestamp: Date.now(),
    });

    log.info('CynicWatcher stopped');
  }

  /**
   * Get current stats
   */
  getStats() {
    return { ...this.stats };
  }

  /**
   * Get current state (for concurrent polling)
   */
  getState() {
    return {
      isRunning: this._isRunning,
      stats: this.getStats(),
      timestamp: Date.now(),
    };
  }

  /**
   * Watch CYNIC's state - synchronous snapshot
   * This is the main API for perception layer polling.
   *
   * @returns {Promise<Object>} Complete state snapshot
   */
  async watch() {
    if (!this._isRunning) {
      throw new Error('CynicWatcher not running — call .start() first');
    }

    const snapshot = {
      db: await this._getDbMetrics(),
      memory: this._getMemoryMetrics(),
      budget: this._getBudgetMetrics(),
      learning: await this._getLearningMetrics(),
      events: this._getEventMetrics(),
      context: this._getContextMetrics(),
      timestamp: Date.now(),
    };

    // Emit health update
    this.eventBus.emit(CynicEventType.HEALTH_UPDATE, snapshot);
    globalEventBus.emit(EventType.CYNIC_HEALTH_UPDATE || 'cynic:health:update', snapshot);

    this.stats.healthChecks++;
    this.stats.lastPollAt = Date.now();

    return snapshot;
  }

  // =============================================================================
  // PRIVATE POLLING LOOPS
  // =============================================================================

  /**
   * Start health polling loop (memory + budget + events)
   * @private
   */
  _startHealthPolling() {
    const poll = async () => {
      if (!this._isRunning) return;

      try {
        const memory = this._getMemoryMetrics();
        const budget = this._getBudgetMetrics();
        const events = this._getEventMetrics();

        // Check memory thresholds
        const heapRatio = memory.heapUsed / memory.heapTotal;
        if (heapRatio > THRESHOLDS.memory.critical) {
          this._emitWarning('memory', {
            level: 'critical',
            heapRatio,
            heapUsedMB: (memory.heapUsed / 1024 / 1024).toFixed(1),
          });
        } else if (heapRatio > THRESHOLDS.memory.cautious) {
          this._emitWarning('memory', {
            level: 'cautious',
            heapRatio,
            heapUsedMB: (memory.heapUsed / 1024 / 1024).toFixed(1),
          });
        }

        // Check budget thresholds
        if (budget.remaining !== null) {
          const ratio = budget.spent / budget.limit;
          if (ratio > THRESHOLDS.budget.critical) {
            this._emitWarning('budget', {
              level: 'critical',
              ratio,
              spent: budget.spent,
              remaining: budget.remaining,
            });
          } else if (ratio > THRESHOLDS.budget.cautious) {
            this._emitWarning('budget', {
              level: 'cautious',
              ratio,
              spent: budget.spent,
              remaining: budget.remaining,
            });
          }
        }

        // Check event storm
        if (events.throughputPerHour > THRESHOLDS.events.storm * 60) {
          this._emitWarning('event_storm', {
            throughputPerHour: events.throughputPerHour,
            threshold: THRESHOLDS.events.storm * 60,
          });
        }
      } catch (error) {
        this.stats.errors++;
        log.debug('Health poll error', { error: error.message });
      }
    };

    // Initial poll
    poll();

    // Recurring poll
    const timer = setInterval(poll, POLL_INTERVALS.health);
    this._timers.set('health', timer);
  }

  /**
   * Start database polling loop
   * @private
   */
  _startDbPolling() {
    const poll = async () => {
      if (!this._isRunning) return;

      try {
        const dbMetrics = await this._getDbMetrics();

        // Check DB size threshold
        if (dbMetrics.sizeMB > THRESHOLDS.db.sizeCritical) {
          this._emitWarning('db_size', {
            sizeMB: dbMetrics.sizeMB,
            threshold: THRESHOLDS.db.sizeCritical,
          });
        }

        // Check query latency
        if (dbMetrics.avgQueryLatency > THRESHOLDS.db.slowQuery) {
          this._emitWarning('db_latency', {
            avgQueryLatency: dbMetrics.avgQueryLatency,
            threshold: THRESHOLDS.db.slowQuery,
          });
        }
      } catch (error) {
        this.stats.errors++;
        log.debug('DB poll error', { error: error.message });
      }
    };

    // Initial poll (offset by 30s to avoid collision)
    setTimeout(poll, 30000);

    // Recurring poll
    const timer = setInterval(poll, POLL_INTERVALS.db);
    this._timers.set('db', timer);
  }

  /**
   * Start learning maturity polling loop
   * @private
   */
  _startLearningPolling() {
    const poll = async () => {
      if (!this._isRunning) return;

      try {
        const learning = await this._getLearningMetrics();

        // Emit learning milestone if maturity crosses φ⁻¹
        if (learning.maturityPercent > PHI_INV * 100) {
          this.eventBus.emit(CynicEventType.LEARNING_MILESTONE, {
            maturityPercent: learning.maturityPercent,
            calibratedDimensions: learning.calibratedDimensions,
            timestamp: Date.now(),
          });
        }
      } catch (error) {
        this.stats.errors++;
        log.debug('Learning poll error', { error: error.message });
      }
    };

    // Initial poll (offset by 60s)
    setTimeout(poll, 60000);

    // Recurring poll
    const timer = setInterval(poll, POLL_INTERVALS.learning);
    this._timers.set('learning', timer);
  }

  // =============================================================================
  // METRIC COLLECTION
  // =============================================================================

  /**
   * Get database metrics
   * @private
   * @returns {Promise<Object>}
   */
  async _getDbMetrics() {
    try {
      // Database size
      const sizeResult = await this.db.query(`
        SELECT pg_database_size(current_database()) / 1024.0 / 1024.0 AS size_mb
      `);
      const sizeMB = parseFloat(sizeResult.rows[0]?.size_mb || 0);

      // Table count
      const tableResult = await this.db.query(`
        SELECT COUNT(*) AS count
        FROM information_schema.tables
        WHERE table_schema = 'public'
      `);
      const tableCount = parseInt(tableResult.rows[0]?.count || 0, 10);

      // Judgment count
      const judgmentResult = await this.db.query(`
        SELECT COUNT(*) AS count FROM judgments
      `);
      const judgmentCount = parseInt(judgmentResult.rows[0]?.count || 0, 10);

      // Learning event count
      const learningResult = await this.db.query(`
        SELECT COUNT(*) AS count FROM learning_events
      `);
      const learningEventCount = parseInt(learningResult.rows[0]?.count || 0, 10);

      // Average query latency (rough estimate from pg_stat_statements if available)
      let avgQueryLatency = 0;
      try {
        const latencyResult = await this.db.query(`
          SELECT COALESCE(AVG(mean_exec_time), 0) AS avg_latency
          FROM pg_stat_statements
          LIMIT 1
        `);
        avgQueryLatency = parseFloat(latencyResult.rows[0]?.avg_latency || 0);
      } catch {
        // pg_stat_statements not available — use fallback (simple ping)
        const start = Date.now();
        await this.db.query('SELECT 1');
        avgQueryLatency = Date.now() - start;
      }

      this.stats.dbQueries++;

      return {
        sizeMB: parseFloat(sizeMB.toFixed(2)),
        tableCount,
        avgQueryLatency: parseFloat(avgQueryLatency.toFixed(2)),
        judgmentCount,
        learningEventCount,
      };
    } catch (error) {
      log.debug('Failed to fetch DB metrics', { error: error.message });
      return {
        sizeMB: 0,
        tableCount: 0,
        avgQueryLatency: 0,
        judgmentCount: 0,
        learningEventCount: 0,
      };
    }
  }

  /**
   * Get memory metrics
   * @private
   * @returns {Object}
   */
  _getMemoryMetrics() {
    const mem = process.memoryUsage();
    return {
      heapUsed: mem.heapUsed,
      heapTotal: mem.heapTotal,
      rss: mem.rss,
      external: mem.external,
    };
  }

  /**
   * Get budget metrics
   * @private
   * @returns {Object}
   */
  _getBudgetMetrics() {
    const budget = this.costLedger.getBudgetStatus();
    const burnRate = this.costLedger.getBurnRate();

    // Calculate forecast exhaustion timestamp
    let forecastExhaustion = null;
    if (budget.timeToLimitMinutes !== null && budget.timeToLimitMinutes < Infinity) {
      forecastExhaustion = new Date(Date.now() + budget.timeToLimitMinutes * 60 * 1000).toISOString();
    }

    return {
      spent: budget.consumedCost || 0,
      limit: budget.budget || null,
      remaining: budget.budget ? budget.budget - (budget.consumedCost || 0) : null,
      forecastExhaustion,
      burnRate: {
        costPerMinute: burnRate.costPerMinute,
        costPerHour: burnRate.costPerHour,
        velocity: burnRate.velocity,
      },
    };
  }

  /**
   * Get learning metrics
   * @private
   * @returns {Promise<Object>}
   */
  async _getLearningMetrics() {
    try {
      // Try to load SONA instance
      if (!this._sona) {
        try {
          const { getSONA } = await import('../learning/sona.js');
          this._sona = getSONA();
        } catch {
          // SONA not available
        }
      }

      // Get maturity from SONA
      let maturity = 0;
      let qTableSize = 0;
      if (this._sona && typeof this._sona.getMaturity === 'function') {
        maturity = this._sona.getMaturity();
      }
      if (this._sona && typeof this._sona.getQTableSize === 'function') {
        qTableSize = this._sona.getQTableSize();
      }

      // Get calibrated dimensions (from database or Judge)
      let calibratedDimensions = 0;
      try {
        const result = await this.db.query(`
          SELECT COUNT(DISTINCT dimension_id) AS count
          FROM dimension_calibration
          WHERE is_calibrated = true
        `);
        calibratedDimensions = parseInt(result.rows[0]?.count || 0, 10);
      } catch {
        // Calibration table not available
      }

      return {
        maturityPercent: parseFloat((maturity * 100).toFixed(1)),
        calibratedDimensions,
        qTableSize,
      };
    } catch (error) {
      log.debug('Failed to fetch learning metrics', { error: error.message });
      return {
        maturityPercent: 0,
        calibratedDimensions: 0,
        qTableSize: 0,
      };
    }
  }

  /**
   * Get event metrics
   * @private
   * @returns {Object}
   */
  _getEventMetrics() {
    // Try to get EventBusBridge metrics
    let bridgeLatencyP50 = 0;
    let orphanCount = 0;

    try {
      // Dynamic import to avoid circular dependency
      if (!this._eventBridgeMetrics) {
        // Attempt to get EventBusBridge instance
        // (This is a simplification — real implementation would import getInstance)
        // For now, use placeholder metrics
      }
    } catch {
      // EventBusBridge not available
    }

    // Calculate throughput from event counts
    const now = Date.now();
    const elapsedMs = now - this._eventCounts.lastReset;
    const elapsedHours = elapsedMs / (60 * 60 * 1000);
    const throughputPerHour = elapsedHours > 0
      ? Math.round(this._eventCounts.lastHour / elapsedHours)
      : 0;

    return {
      throughputPerHour,
      orphanCount,
      bridgeLatencyP50,
    };
  }

  /**
   * Get context compression metrics
   * @private
   * @returns {Object}
   */
  _getContextMetrics() {
    const stats = this.contextCompressor.getStats();

    // Calculate compression ratio
    const compressionRatio = stats.session.compressionRatio / 100; // 0-1

    // Estimate context size (rough)
    // Average prompt size ~5000 chars, compression ratio applied
    const avgPromptChars = 5000;
    const compressedChars = avgPromptChars * (1 - compressionRatio);
    const sizeMB = (compressedChars * stats.session.injections) / 1024 / 1024;

    return {
      sizeMB: parseFloat(sizeMB.toFixed(2)),
      compressionRatio: parseFloat(compressionRatio.toFixed(2)),
    };
  }

  // =============================================================================
  // EVENT WIRING
  // =============================================================================

  /**
   * Wire event listeners for event counting
   * @private
   */
  _wireEventListeners() {
    // Count all events on globalEventBus
    const originalEmit = globalEventBus.emit.bind(globalEventBus);
    globalEventBus.emit = (...args) => {
      this._eventCounts.lastMinute++;
      this._eventCounts.lastHour++;
      return originalEmit(...args);
    };

    // Reset counters periodically
    setInterval(() => {
      this._eventCounts.lastMinute = 0;
    }, 60 * 1000); // Reset every minute

    setInterval(() => {
      this._eventCounts.lastHour = 0;
      this._eventCounts.lastReset = Date.now();
    }, 60 * 60 * 1000); // Reset every hour
  }

  /**
   * Emit warning event
   * @private
   */
  _emitWarning(type, data) {
    this.stats.warnings++;

    const event = {
      type,
      ...data,
      timestamp: Date.now(),
    };

    // Emit to automation bus
    this.eventBus.emit(CynicEventType[type.toUpperCase()] || CynicEventType.HEALTH_UPDATE, event);

    // Emit to global bus
    const globalEventName = `cynic:${type}`;
    globalEventBus.emit(globalEventName, event);

    log.warn(`CynicWatcher warning: ${type}`, data);
  }
}

// Singleton
let _instance = null;

/**
 * Get singleton instance
 */
export function getCynicWatcher() {
  if (!_instance) {
    _instance = new CynicWatcher();
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetCynicWatcher() {
  if (_instance) {
    _instance.stop();
  }
  _instance = null;
}

export default CynicWatcher;

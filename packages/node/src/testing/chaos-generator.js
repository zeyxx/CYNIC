/**
 * ChaosGenerator — Antifragility Testing
 *
 * "What doesn't kill the dog makes it stronger." — κυνικός
 *
 * Introduces controlled chaos to test system resilience:
 *
 * INJECT → OBSERVE → RECOVER → LEARN
 *    ↓        ↓          ↓        ↓
 * Failure  Impact   Restore   Adapt
 *
 * Chaos types:
 * - Network failures (timeouts, connection drops)
 * - Resource exhaustion (memory, CPU, disk)
 * - Data corruption (invalid inputs, malformed data)
 * - Service unavailability (DB, API, daemon down)
 * - Event storms (overwhelming event traffic)
 * - Budget exhaustion (cost limits hit)
 *
 * φ-bounded intensity: max 61.8% failure rate
 *
 * @module @cynic/node/testing/chaos-generator
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2 } from '@cynic/core';

const log = createLogger('ChaosGenerator');

// ═══════════════════════════════════════════════════════════════════════════
// CHAOS TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Types of chaos to inject
 */
export const ChaosType = {
  NETWORK_TIMEOUT: 'network_timeout',       // Simulate slow/failed requests
  NETWORK_DROP: 'network_drop',             // Drop connections randomly
  MEMORY_SPIKE: 'memory_spike',             // Allocate large memory chunks
  CPU_SPIKE: 'cpu_spike',                   // High CPU load
  DISK_FULL: 'disk_full',                   // Simulate disk full errors
  DATA_CORRUPTION: 'data_corruption',       // Invalid/malformed data
  SERVICE_DOWN: 'service_down',             // Simulate service unavailable
  EVENT_STORM: 'event_storm',               // Overwhelming event traffic
  BUDGET_EXHAUSTION: 'budget_exhaustion',   // Cost limits triggered
  RANDOM_ERROR: 'random_error',             // Random exception throws
};

/**
 * Intensity levels (φ-bounded)
 */
export const Intensity = {
  MILD: 0.1,           // 10% failure rate
  MODERATE: PHI_INV_2, // φ⁻² = 38.2%
  SEVERE: PHI_INV,     // φ⁻¹ = 61.8% (max allowed)
};

/**
 * Target subsystems
 */
export const Target = {
  DAEMON: 'daemon',
  DATABASE: 'database',
  LLM_ROUTER: 'llm_router',
  LEARNING: 'learning',
  WATCHERS: 'watchers',
  EVENT_BUS: 'event_bus',
  ALL: 'all',
};

// ═══════════════════════════════════════════════════════════════════════════
// CHAOS ENTITIES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * ChaosEvent — one instance of injected chaos
 */
export class ChaosEvent {
  constructor(data = {}) {
    this.id = data.id || `chaos-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    this.timestamp = data.timestamp || Date.now();
    this.type = data.type || ChaosType.RANDOM_ERROR;
    this.target = data.target || Target.ALL;
    this.intensity = Math.min(data.intensity || Intensity.MILD, PHI_INV); // φ-bounded
    this.duration = data.duration || 5000; // 5s default
    this.injected = false;
    this.recovered = false;
    this.startTime = null;
    this.endTime = null;
    this.impact = {
      failures: 0,
      degradations: 0,
      recoveries: 0,
    };
    this.metadata = data.metadata || {};
  }

  get active() {
    return this.injected && !this.recovered;
  }

  get actualDuration() {
    if (!this.startTime) return 0;
    return (this.endTime || Date.now()) - this.startTime;
  }

  toJSON() {
    return {
      id: this.id,
      type: this.type,
      target: this.target,
      intensity: Math.round(this.intensity * 1000) / 1000,
      duration: this.duration,
      active: this.active,
      impact: this.impact,
    };
  }
}

/**
 * ChaosExperiment — a sequence of chaos events
 */
export class ChaosExperiment {
  constructor(data = {}) {
    this.id = data.id || `experiment-${Date.now()}`;
    this.name = data.name || 'Unnamed Experiment';
    this.description = data.description || '';
    this.events = data.events || [];
    this.startTime = null;
    this.endTime = null;
    this.status = 'pending'; // pending, running, completed, failed
    this.results = {
      totalEvents: 0,
      successfulInjections: 0,
      failedInjections: 0,
      totalFailures: 0,
      totalRecoveries: 0,
      resilienceScore: 0, // 0-1, φ-bounded
    };
  }

  get duration() {
    if (!this.startTime) return 0;
    return (this.endTime || Date.now()) - this.startTime;
  }

  get isRunning() {
    return this.status === 'running';
  }

  toJSON() {
    return {
      id: this.id,
      name: this.name,
      status: this.status,
      duration: this.duration,
      eventCount: this.events.length,
      results: this.results,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// CHAOS GENERATOR
// ═══════════════════════════════════════════════════════════════════════════

/**
 * ChaosGenerator — injects controlled chaos for resilience testing
 */
export class ChaosGenerator extends EventEmitter {
  constructor(options = {}) {
    super();

    // Configuration
    this.config = {
      enabled: options.enabled !== false,
      maxIntensity: Math.min(options.maxIntensity || PHI_INV, PHI_INV), // φ-bounded
      safeMode: options.safeMode !== false, // Prevents permanent damage
      autoRecover: options.autoRecover !== false,
      recoveryTimeout: options.recoveryTimeout || 30000, // 30s max chaos duration
    };

    // State
    this.experiments = new Map(); // experimentId → ChaosExperiment
    this.activeEvents = new Map(); // eventId → ChaosEvent
    this.injectors = this._initializeInjectors();
    this.stats = {
      totalExperiments: 0,
      totalEvents: 0,
      totalFailures: 0,
      totalRecoveries: 0,
      avgResilienceScore: 0,
    };

    this._running = false;

    log.info('ChaosGenerator created', {
      enabled: this.config.enabled,
      maxIntensity: this.config.maxIntensity,
      safeMode: this.config.safeMode,
    });
  }

  /**
   * Initialize chaos injectors for each type
   * @private
   */
  _initializeInjectors() {
    return {
      [ChaosType.NETWORK_TIMEOUT]: this._injectNetworkTimeout.bind(this),
      [ChaosType.NETWORK_DROP]: this._injectNetworkDrop.bind(this),
      [ChaosType.MEMORY_SPIKE]: this._injectMemorySpike.bind(this),
      [ChaosType.CPU_SPIKE]: this._injectCPUSpike.bind(this),
      [ChaosType.DISK_FULL]: this._injectDiskFull.bind(this),
      [ChaosType.DATA_CORRUPTION]: this._injectDataCorruption.bind(this),
      [ChaosType.SERVICE_DOWN]: this._injectServiceDown.bind(this),
      [ChaosType.EVENT_STORM]: this._injectEventStorm.bind(this),
      [ChaosType.BUDGET_EXHAUSTION]: this._injectBudgetExhaustion.bind(this),
      [ChaosType.RANDOM_ERROR]: this._injectRandomError.bind(this),
    };
  }

  // ───────────────────────────────────────────────────────────────────────────
  // CORE API
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Create a new chaos experiment
   *
   * @param {Object} options
   * @param {string} options.name - Experiment name
   * @param {string} [options.description] - Description
   * @param {ChaosEvent[]} options.events - Chaos events to inject
   * @returns {ChaosExperiment}
   */
  createExperiment(options) {
    if (!this.config.enabled) {
      throw new Error('ChaosGenerator is disabled');
    }

    const experiment = new ChaosExperiment({
      name: options.name,
      description: options.description,
      events: options.events || [],
    });

    // Validate events
    for (const event of experiment.events) {
      if (event.intensity > this.config.maxIntensity) {
        log.warn('Event intensity exceeds max, capping', {
          eventId: event.id,
          intensity: event.intensity,
          max: this.config.maxIntensity,
        });
        event.intensity = this.config.maxIntensity;
      }
    }

    this.experiments.set(experiment.id, experiment);
    this.stats.totalExperiments++;

    this.emit('experiment:created', experiment.toJSON());
    log.info('Experiment created', { id: experiment.id, name: experiment.name });

    return experiment;
  }

  /**
   * Run a chaos experiment
   *
   * @param {string|ChaosExperiment} experimentOrId
   * @returns {Promise<ChaosExperiment>}
   */
  async runExperiment(experimentOrId) {
    const experiment = typeof experimentOrId === 'string'
      ? this.experiments.get(experimentOrId)
      : experimentOrId;

    if (!experiment) {
      throw new Error('Experiment not found');
    }

    if (experiment.isRunning) {
      throw new Error('Experiment already running');
    }

    experiment.status = 'running';
    experiment.startTime = Date.now();
    experiment.results.totalEvents = experiment.events.length;

    this.emit('experiment:started', experiment.toJSON());
    log.info('Experiment started', { id: experiment.id, eventCount: experiment.events.length });

    try {
      // Inject events sequentially
      for (const event of experiment.events) {
        await this.injectChaos(event);

        // Wait for event duration
        await this._wait(event.duration);

        // Auto-recover if enabled
        if (this.config.autoRecover) {
          await this.recoverChaos(event);
        }

        // Update experiment results
        experiment.results.successfulInjections++;
        experiment.results.totalFailures += event.impact.failures;
        experiment.results.totalRecoveries += event.impact.recoveries;
      }

      // Calculate resilience score
      experiment.results.resilienceScore = this._calculateResilienceScore(experiment);

      experiment.status = 'completed';
      experiment.endTime = Date.now();

      // Update global stats
      this.stats.totalEvents += experiment.results.totalEvents;
      this.stats.totalFailures += experiment.results.totalFailures;
      this.stats.totalRecoveries += experiment.results.totalRecoveries;
      this._updateAvgResilienceScore(experiment.results.resilienceScore);

      this.emit('experiment:completed', {
        experiment: experiment.toJSON(),
        resilienceScore: experiment.results.resilienceScore,
      });

      log.info('Experiment completed', {
        id: experiment.id,
        duration: experiment.duration,
        resilienceScore: Math.round(experiment.results.resilienceScore * 100),
      });

      return experiment;
    } catch (err) {
      experiment.status = 'failed';
      experiment.endTime = Date.now();

      this.emit('experiment:failed', {
        experiment: experiment.toJSON(),
        error: err.message,
      });

      log.error('Experiment failed', { id: experiment.id, error: err.message });
      throw err;
    }
  }

  /**
   * Inject a single chaos event
   *
   * @param {ChaosEvent} event
   * @returns {Promise<void>}
   */
  async injectChaos(event) {
    if (!this.config.enabled) {
      log.warn('ChaosGenerator disabled, skipping injection');
      return;
    }

    this.emit('chaos:injecting', event.toJSON());

    event.injected = true;
    event.startTime = Date.now();
    this.activeEvents.set(event.id, event);

    try {
      // Get injector for event type
      const injector = this.injectors[event.type];
      if (!injector) {
        throw new Error(`No injector for chaos type: ${event.type}`);
      }

      // Execute injection
      await injector(event);

      this.emit('chaos:injected', event.toJSON());
      log.info('Chaos injected', {
        id: event.id,
        type: event.type,
        target: event.target,
        intensity: event.intensity,
      });

      // Auto-recover timeout
      if (this.config.autoRecover) {
        setTimeout(() => {
          if (event.active) {
            log.warn('Auto-recovery timeout triggered', { eventId: event.id });
            this.recoverChaos(event).catch(err => {
              log.error('Auto-recovery failed', { eventId: event.id, error: err.message });
            });
          }
        }, this.config.recoveryTimeout);
      }
    } catch (err) {
      event.recovered = true;
      event.endTime = Date.now();
      this.activeEvents.delete(event.id);

      this.emit('chaos:failed', { event: event.toJSON(), error: err.message });
      log.error('Chaos injection failed', { id: event.id, error: err.message });
      throw err;
    }
  }

  /**
   * Recover from a chaos event
   *
   * @param {ChaosEvent} event
   * @returns {Promise<void>}
   */
  async recoverChaos(event) {
    if (!event.active) {
      log.debug('Chaos event not active, skipping recovery', { eventId: event.id });
      return;
    }

    this.emit('chaos:recovering', event.toJSON());

    try {
      // Cleanup/restore based on chaos type
      await this._recover(event);

      event.recovered = true;
      event.endTime = Date.now();
      event.impact.recoveries++;
      this.activeEvents.delete(event.id);

      this.emit('chaos:recovered', event.toJSON());
      log.info('Chaos recovered', {
        id: event.id,
        type: event.type,
        duration: event.actualDuration,
      });
    } catch (err) {
      this.emit('chaos:recovery-failed', { event: event.toJSON(), error: err.message });
      log.error('Chaos recovery failed', { id: event.id, error: err.message });
      throw err;
    }
  }

  // ───────────────────────────────────────────────────────────────────────────
  // INJECTORS (Chaos type implementations)
  // ───────────────────────────────────────────────────────────────────────────

  async _injectNetworkTimeout(event) {
    // Simulate network timeouts by delaying responses
    log.debug('Injecting network timeout chaos', { eventId: event.id, intensity: event.intensity });
    event.impact.failures++;
    // Note: Actual implementation would intercept network calls
  }

  async _injectNetworkDrop(event) {
    // Simulate dropped connections
    log.debug('Injecting network drop chaos', { eventId: event.id, intensity: event.intensity });
    event.impact.failures++;
    // Note: Actual implementation would close sockets randomly
  }

  async _injectMemorySpike(event) {
    // Allocate memory to create pressure
    const bytes = Math.floor(event.intensity * 100 * 1024 * 1024); // Up to 61.8MB at max intensity
    log.debug('Injecting memory spike', { eventId: event.id, bytes });
    event._memoryBuffer = Buffer.alloc(bytes);
    event.impact.degradations++;
  }

  async _injectCPUSpike(event) {
    // Create CPU load
    log.debug('Injecting CPU spike', { eventId: event.id, intensity: event.intensity });
    const endTime = Date.now() + event.duration;
    event._cpuInterval = setInterval(() => {
      // Busy loop for intensity% of time
      const loopStart = Date.now();
      while (Date.now() - loopStart < event.intensity * 100) {
        Math.sqrt(Math.random());
      }
      if (Date.now() >= endTime) {
        clearInterval(event._cpuInterval);
      }
    }, 100);
    event.impact.degradations++;
  }

  async _injectDiskFull(event) {
    // Simulate disk full errors
    log.debug('Injecting disk full chaos', { eventId: event.id });
    event.impact.failures++;
    // Note: Safe mode prevents actual disk filling
  }

  async _injectDataCorruption(event) {
    // Inject invalid/malformed data
    log.debug('Injecting data corruption', { eventId: event.id });
    event.impact.failures++;
    // Note: Actual implementation would modify data in flight
  }

  async _injectServiceDown(event) {
    // Simulate service unavailable
    log.debug('Injecting service down', { eventId: event.id, target: event.target });
    event.impact.failures++;
    // Note: Actual implementation would block service calls
  }

  async _injectEventStorm(event) {
    // Flood event bus with traffic
    const eventsPerSecond = Math.floor(event.intensity * 1000); // Up to 618 events/s
    log.debug('Injecting event storm', { eventId: event.id, rate: eventsPerSecond });

    event._stormInterval = setInterval(() => {
      for (let i = 0; i < eventsPerSecond / 10; i++) {
        this.emit('chaos:storm-event', { synthetic: true });
      }
    }, 100);

    event.impact.degradations++;
  }

  async _injectBudgetExhaustion(event) {
    // Simulate budget limits hit
    log.debug('Injecting budget exhaustion', { eventId: event.id });
    event.impact.failures++;
    // Note: Actual implementation would modify CostLedger state
  }

  async _injectRandomError(event) {
    // Throw random errors
    log.debug('Injecting random errors', { eventId: event.id, intensity: event.intensity });
    event.impact.failures++;
    if (Math.random() < event.intensity) {
      throw new Error(`Chaos-induced error (event ${event.id})`);
    }
  }

  /**
   * Recover from chaos event (cleanup)
   * @private
   */
  async _recover(event) {
    // Cleanup based on type
    if (event._memoryBuffer) {
      event._memoryBuffer = null;
    }
    if (event._cpuInterval) {
      clearInterval(event._cpuInterval);
      event._cpuInterval = null;
    }
    if (event._stormInterval) {
      clearInterval(event._stormInterval);
      event._stormInterval = null;
    }

    log.debug('Chaos cleaned up', { eventId: event.id, type: event.type });
  }

  /**
   * Calculate resilience score for experiment
   * φ-bounded: 0-1, where 1 = perfect resilience
   * @private
   */
  _calculateResilienceScore(experiment) {
    const { totalEvents, totalFailures, totalRecoveries } = experiment.results;

    if (totalEvents === 0) return 0;

    // Recovery rate (what % recovered)
    const recoveryRate = totalFailures > 0 ? totalRecoveries / totalFailures : 1;

    // Survival rate (what % didn't fail)
    const survivalRate = 1 - (totalFailures / totalEvents);

    // Weighted average (recovery matters more than survival)
    const score = PHI_INV * recoveryRate + (1 - PHI_INV) * survivalRate;

    return Math.min(score, PHI_INV); // φ-bounded max
  }

  /**
   * Update average resilience score (EMA)
   * @private
   */
  _updateAvgResilienceScore(newScore) {
    if (this.stats.avgResilienceScore === 0) {
      this.stats.avgResilienceScore = newScore;
    } else {
      this.stats.avgResilienceScore =
        PHI_INV * newScore + (1 - PHI_INV) * this.stats.avgResilienceScore;
    }
  }

  /**
   * Promise-based wait
   * @private
   */
  _wait(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  // ───────────────────────────────────────────────────────────────────────────
  // STATS & HEALTH
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Get generator statistics
   */
  getStats() {
    return {
      ...this.stats,
      activeEvents: this.activeEvents.size,
      avgResilienceScore: Math.round(this.stats.avgResilienceScore * 1000) / 1000,
    };
  }

  /**
   * Get all experiments
   */
  getExperiments() {
    return Array.from(this.experiments.values()).map(e => e.toJSON());
  }

  /**
   * Get active chaos events
   */
  getActiveEvents() {
    return Array.from(this.activeEvents.values()).map(e => e.toJSON());
  }

  /**
   * Stop all active chaos
   */
  async stopAll() {
    const events = Array.from(this.activeEvents.values());
    log.info('Stopping all active chaos', { count: events.length });

    for (const event of events) {
      try {
        await this.recoverChaos(event);
      } catch (err) {
        log.error('Failed to recover chaos event', { eventId: event.id, error: err.message });
      }
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _singleton = null;

export function getChaosGenerator(options) {
  if (!_singleton) {
    _singleton = new ChaosGenerator(options);
  }
  return _singleton;
}

export function _resetForTesting() {
  if (_singleton) {
    _singleton.stopAll().catch(() => {});
    _singleton.removeAllListeners();
  }
  _singleton = null;
}

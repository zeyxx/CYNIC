/**
 * Thermodynamics - Heat, Work, Temperature, Efficiency
 *
 * "La chaleur monte, le travail compte" - κυνικός
 *
 * Models CYNIC as a thermodynamic system:
 * - Heat (Q): Wasted energy (errors, retries, friction)
 * - Work (W): Useful output (successful operations)
 * - Temperature (T): Accumulated heat × decay
 * - Efficiency (η): W / (W + Q), max φ⁻¹ (61.8%)
 *
 * Laws:
 * 1. Energy is conserved: Input = Work + Heat
 * 2. Entropy always increases (disorder grows)
 * 3. Efficiency cannot exceed φ⁻¹ (61.8%)
 *
 * @module @cynic/node/organism/thermodynamics
 */

'use strict';

import { PHI, PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

export const THERMO_CONFIG = {
  // Temperature thresholds (φ-aligned)
  TEMP_OPTIMAL: 37,                    // Body temperature (metaphor)
  TEMP_WARNING: Math.round(PHI * 50),  // ~81° (φ × 50)
  TEMP_CRITICAL: 100,                  // Boiling point
  TEMP_COLLAPSE: 120,                  // System collapse

  // Heat decay rate per second
  HEAT_DECAY_RATE: PHI_INV_3,          // 23.6% decay per interval

  // Decay interval (ms)
  DECAY_INTERVAL: 1000,

  // Maximum efficiency (φ⁻¹)
  MAX_EFFICIENCY: PHI_INV,

  // Heat sources (multipliers)
  HEAT_ERROR: 5,                       // Errors generate 5x heat
  HEAT_RETRY: 3,                       // Retries generate 3x heat
  HEAT_TIMEOUT: 10,                    // Timeouts generate 10x heat
  HEAT_BLOCKED: 2,                     // Blocked actions generate 2x heat

  // Work sources (multipliers)
  WORK_SUCCESS: 1,                     // Successful operations = 1 work
  WORK_LEARNING: 0.5,                  // Learning = 0.5 work (investment)
};

// ═══════════════════════════════════════════════════════════════════════════════
// THERMODYNAMIC EVENT
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Event types for thermodynamic tracking
 */
export const ThermoEventType = Object.freeze({
  // Work events (useful output)
  SUCCESS: 'success',
  COMPLETION: 'completion',
  LEARNING: 'learning',
  INSIGHT: 'insight',

  // Heat events (wasted energy)
  ERROR: 'error',
  RETRY: 'retry',
  TIMEOUT: 'timeout',
  BLOCKED: 'blocked',
  FRICTION: 'friction',
});

/**
 * Single thermodynamic event
 */
export class ThermoEvent {
  /**
   * @param {string} type - Event type
   * @param {number} magnitude - Event magnitude
   * @param {Object} [metadata] - Additional data
   */
  constructor(type, magnitude = 1, metadata = {}) {
    this.timestamp = Date.now();
    this.type = type;
    this.magnitude = magnitude;
    this.metadata = metadata;

    // Classify as heat or work
    this.isHeat = [
      ThermoEventType.ERROR,
      ThermoEventType.RETRY,
      ThermoEventType.TIMEOUT,
      ThermoEventType.BLOCKED,
      ThermoEventType.FRICTION,
    ].includes(type);

    this.isWork = !this.isHeat;

    // Calculate energy contribution
    if (this.isHeat) {
      const multiplier = {
        [ThermoEventType.ERROR]: THERMO_CONFIG.HEAT_ERROR,
        [ThermoEventType.RETRY]: THERMO_CONFIG.HEAT_RETRY,
        [ThermoEventType.TIMEOUT]: THERMO_CONFIG.HEAT_TIMEOUT,
        [ThermoEventType.BLOCKED]: THERMO_CONFIG.HEAT_BLOCKED,
        [ThermoEventType.FRICTION]: 1,
      }[type] || 1;
      this.heat = magnitude * multiplier;
      this.work = 0;
    } else {
      const multiplier = {
        [ThermoEventType.SUCCESS]: THERMO_CONFIG.WORK_SUCCESS,
        [ThermoEventType.COMPLETION]: THERMO_CONFIG.WORK_SUCCESS,
        [ThermoEventType.LEARNING]: THERMO_CONFIG.WORK_LEARNING,
        [ThermoEventType.INSIGHT]: THERMO_CONFIG.WORK_LEARNING,
      }[type] || 1;
      this.work = magnitude * multiplier;
      this.heat = 0;
    }
  }

  toJSON() {
    return {
      timestamp: this.timestamp,
      type: this.type,
      magnitude: this.magnitude,
      heat: this.heat,
      work: this.work,
      metadata: this.metadata,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// THERMODYNAMIC STATE
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Tracks thermodynamic state over time
 */
export class ThermodynamicState {
  constructor(options = {}) {
    // Current state
    this.heat = 0;           // Q: accumulated heat
    this.work = 0;           // W: accumulated work
    this.temperature = THERMO_CONFIG.TEMP_OPTIMAL; // T: current temperature

    // History
    this.events = [];
    this.maxEvents = options.maxEvents || 1000;

    // Decay management
    this.lastDecay = Date.now();
    this.decayInterval = options.decayInterval || THERMO_CONFIG.DECAY_INTERVAL;

    // Session tracking
    this.sessionStart = Date.now();
  }

  /**
   * Apply heat decay (temperature drops over time)
   * @private
   */
  _applyDecay() {
    const now = Date.now();
    const elapsed = now - this.lastDecay;

    if (elapsed >= this.decayInterval) {
      const intervals = Math.floor(elapsed / this.decayInterval);
      const decayFactor = Math.pow(1 - THERMO_CONFIG.HEAT_DECAY_RATE, intervals);

      // Temperature decays toward optimal
      const excess = this.temperature - THERMO_CONFIG.TEMP_OPTIMAL;
      this.temperature = THERMO_CONFIG.TEMP_OPTIMAL + (excess * decayFactor);

      this.lastDecay = now;
    }
  }

  /**
   * Record a thermodynamic event
   * @param {string} type - Event type
   * @param {number} [magnitude=1] - Event magnitude
   * @param {Object} [metadata] - Additional data
   * @returns {ThermoEvent}
   */
  record(type, magnitude = 1, metadata = {}) {
    // Apply decay first
    this._applyDecay();

    const event = new ThermoEvent(type, magnitude, metadata);

    // Update state
    this.heat += event.heat;
    this.work += event.work;

    // Temperature rises with heat
    if (event.heat > 0) {
      this.temperature += event.heat * 0.5; // Heat raises temperature
    }

    // Cap temperature at collapse
    this.temperature = Math.min(THERMO_CONFIG.TEMP_COLLAPSE, this.temperature);

    // Store event
    this.events.push(event);

    // Prune old events
    while (this.events.length > this.maxEvents) {
      this.events.shift();
    }

    return event;
  }

  // Convenience methods for common events
  recordSuccess(magnitude = 1, metadata = {}) {
    return this.record(ThermoEventType.SUCCESS, magnitude, metadata);
  }

  recordError(magnitude = 1, metadata = {}) {
    return this.record(ThermoEventType.ERROR, magnitude, metadata);
  }

  recordRetry(magnitude = 1, metadata = {}) {
    return this.record(ThermoEventType.RETRY, magnitude, metadata);
  }

  recordTimeout(magnitude = 1, metadata = {}) {
    return this.record(ThermoEventType.TIMEOUT, magnitude, metadata);
  }

  recordBlocked(magnitude = 1, metadata = {}) {
    return this.record(ThermoEventType.BLOCKED, magnitude, metadata);
  }

  recordLearning(magnitude = 1, metadata = {}) {
    return this.record(ThermoEventType.LEARNING, magnitude, metadata);
  }

  /**
   * Get current temperature
   * @returns {number}
   */
  getTemperature() {
    this._applyDecay();
    return this.temperature;
  }

  /**
   * Get efficiency: η = W / (W + Q)
   * @returns {number} 0 to φ⁻¹ (max 61.8%)
   */
  getEfficiency() {
    const total = this.work + this.heat;
    if (total === 0) return 0;

    const raw = this.work / total;
    return Math.min(THERMO_CONFIG.MAX_EFFICIENCY, raw);
  }

  /**
   * Get temperature status
   * @returns {string}
   */
  getTemperatureStatus() {
    const temp = this.getTemperature();

    if (temp <= THERMO_CONFIG.TEMP_OPTIMAL + 5) return 'optimal';
    if (temp < THERMO_CONFIG.TEMP_WARNING) return 'warm';
    if (temp < THERMO_CONFIG.TEMP_CRITICAL) return 'hot';
    if (temp < THERMO_CONFIG.TEMP_COLLAPSE) return 'critical';
    return 'collapse';
  }

  /**
   * Get thermodynamic health
   * @returns {Object}
   */
  getHealth() {
    this._applyDecay();

    const temp = this.temperature;
    const efficiency = this.getEfficiency();
    const status = this.getTemperatureStatus();

    // Health score: combines efficiency and temperature stability
    // Penalize high temperature
    const tempPenalty = Math.max(0, (temp - THERMO_CONFIG.TEMP_OPTIMAL) / 100);
    const rawScore = efficiency * (1 - tempPenalty);
    const score = Math.min(PHI_INV, Math.max(0, rawScore));

    return {
      score,
      status,
      details: {
        temperature: temp,
        heat: this.heat,
        work: this.work,
        efficiency,
        eventCount: this.events.length,
      },
    };
  }

  /**
   * Get recent events of a specific type
   * @param {string} type
   * @param {number} [limit=10]
   * @returns {ThermoEvent[]}
   */
  getRecentEvents(type, limit = 10) {
    return this.events
      .filter(e => e.type === type)
      .slice(-limit);
  }

  /**
   * Get statistics
   * @returns {Object}
   */
  getStats() {
    this._applyDecay();

    const heatEvents = this.events.filter(e => e.isHeat);
    const workEvents = this.events.filter(e => e.isWork);

    return {
      temperature: this.temperature,
      heat: this.heat,
      work: this.work,
      efficiency: this.getEfficiency(),
      status: this.getTemperatureStatus(),
      events: {
        total: this.events.length,
        heat: heatEvents.length,
        work: workEvents.length,
      },
      session: {
        durationMs: Date.now() - this.sessionStart,
      },
    };
  }

  /**
   * Cool down (manual intervention)
   * @param {number} [amount=10]
   */
  coolDown(amount = 10) {
    this.temperature = Math.max(
      THERMO_CONFIG.TEMP_OPTIMAL,
      this.temperature - amount
    );
  }

  /**
   * Reset state
   */
  reset() {
    this.heat = 0;
    this.work = 0;
    this.temperature = THERMO_CONFIG.TEMP_OPTIMAL;
    this.events = [];
    this.lastDecay = Date.now();
    this.sessionStart = Date.now();
  }

  /**
   * Serialize for persistence
   */
  toJSON() {
    return {
      heat: this.heat,
      work: this.work,
      temperature: this.temperature,
      events: this.events.map(e => e.toJSON()),
      lastDecay: this.lastDecay,
      sessionStart: this.sessionStart,
    };
  }

  /**
   * Restore from persistence
   */
  static fromJSON(data) {
    const state = new ThermodynamicState();
    state.heat = data.heat || 0;
    state.work = data.work || 0;
    state.temperature = data.temperature || THERMO_CONFIG.TEMP_OPTIMAL;
    state.events = (data.events || []).map(e => {
      const event = new ThermoEvent(e.type, e.magnitude, e.metadata);
      event.timestamp = e.timestamp;
      event.heat = e.heat;
      event.work = e.work;
      return event;
    });
    state.lastDecay = data.lastDecay || Date.now();
    state.sessionStart = data.sessionStart || Date.now();
    return state;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FACTORY & SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Create thermodynamic state
 */
export function createThermodynamicState(options = {}) {
  return new ThermodynamicState(options);
}

let _thermoState = null;

/**
 * Get singleton thermodynamic state
 */
export function getThermodynamicState() {
  if (!_thermoState) {
    _thermoState = new ThermodynamicState();
  }
  return _thermoState;
}

/**
 * Reset singleton for testing
 */
export function resetThermodynamicState() {
  _thermoState = null;
}

export default {
  ThermoEvent,
  ThermoEventType,
  ThermodynamicState,
  createThermodynamicState,
  getThermodynamicState,
  resetThermodynamicState,
  THERMO_CONFIG,
};

/**
 * Thermodynamics Tracker for MCP Server
 *
 * Shared state with hooks via ~/.cynic/thermodynamics/state.json
 *
 * "Ἐνέργεια - the activity of being" - κυνικός
 *
 * Laws:
 * - First Law: Energy is conserved (focus + distraction = constant)
 * - Second Law: Entropy always increases (sessions tend toward chaos)
 * - Carnot Limit: Maximum efficiency is φ⁻¹ (61.8%)
 *
 * @module @cynic/mcp/thermodynamics-tracker
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.38196601125010515;
const PHI_INV_3 = 0.2360679774997896;

// =============================================================================
// CONSTANTS (φ-derived)
// =============================================================================

/** Maximum theoretical efficiency - Carnot limit at φ⁻¹ */
export const CARNOT_LIMIT = PHI_INV;

/** Heat dissipation rate per minute - φ⁻³ */
const HEAT_DISSIPATION_RATE = PHI_INV_3;

/** Entropy increase rate per action - φ⁻³ */
const ENTROPY_INCREASE_RATE = PHI_INV_3;

/** Critical temperature (frustration threshold) - φ × 50 ≈ 81 */
export const CRITICAL_TEMPERATURE = Math.round(PHI * 50);

/** Work units per successful action */
const WORK_PER_SUCCESS = 10;

/** Heat units per frustration event */
const HEAT_PER_FRUSTRATION = 15;

// =============================================================================
// STORAGE PATHS (shared with hooks)
// =============================================================================

const THERMO_DIR = path.join(os.homedir(), '.cynic', 'thermodynamics');
const STATE_FILE = path.join(THERMO_DIR, 'state.json');

// =============================================================================
// THERMODYNAMICS TRACKER CLASS
// =============================================================================

/**
 * Thermodynamics tracker for cognitive energy management
 */
export class ThermodynamicsTracker {
  constructor() {
    this.state = {
      session: {
        heat: 0,
        work: 0,
        entropy: 0,
        startTime: Date.now(),
        lastAction: Date.now(),
      },
      totals: {
        totalHeat: 0,
        totalWork: 0,
        totalEntropy: 0,
        sessions: 0,
      },
      heatSources: {},
      workSources: {},
      stats: {
        averageEfficiency: PHI_INV_2,
        peakEfficiency: 0,
        thermalRunaways: 0,
        entropyResets: 0,
      },
    };

    this._loadState();
  }

  // ===========================================================================
  // FILE OPERATIONS
  // ===========================================================================

  _ensureDir() {
    if (!fs.existsSync(THERMO_DIR)) {
      fs.mkdirSync(THERMO_DIR, { recursive: true });
    }
  }

  _loadState() {
    try {
      this._ensureDir();
      if (fs.existsSync(STATE_FILE)) {
        const saved = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));

        // Restore state from file
        this.state.totals = saved.totals || this.state.totals;
        this.state.stats = saved.stats || this.state.stats;

        // Restore session if recent (within 30 minutes)
        if (saved.session?.lastAction) {
          const timeSince = Date.now() - saved.session.lastAction;
          const thirtyMinutes = 30 * 60 * 1000;

          if (timeSince < thirtyMinutes) {
            this.state.session = saved.session;
            this.state.heatSources = saved.heatSources || {};
            this.state.workSources = saved.workSources || {};
          }
        }
      }
    } catch (e) {
      // Start fresh on any error
    }
  }

  _saveState() {
    try {
      this._ensureDir();
      fs.writeFileSync(STATE_FILE, JSON.stringify({
        session: this.state.session,
        totals: this.state.totals,
        stats: this.state.stats,
        heatSources: this.state.heatSources,
        workSources: this.state.workSources,
      }, null, 2));
    } catch (e) {
      // Best effort - don't fail operations on save error
    }
  }

  // ===========================================================================
  // THERMODYNAMIC OPERATIONS
  // ===========================================================================

  /**
   * Record heat (frustration, errors, failures)
   * @param {string} source - Source of heat
   * @param {number} amount - Heat amount (0-1 normalized, or raw value)
   * @returns {Object} Current state
   */
  recordHeat(source, amount = 1) {
    // Normalize: if amount < 1, scale to HEAT_PER_FRUSTRATION
    const heat = amount < 1 ? amount * HEAT_PER_FRUSTRATION : amount;

    this.state.session.heat += heat;
    this.state.totals.totalHeat += heat;

    // Track sources
    this.state.heatSources[source] = (this.state.heatSources[source] || 0) + heat;

    // Check for thermal runaway
    if (this.state.session.heat >= CRITICAL_TEMPERATURE) {
      this.state.stats.thermalRunaways++;
    }

    // Entropy increases with heat
    this.state.session.entropy += heat * ENTROPY_INCREASE_RATE;
    this.state.session.lastAction = Date.now();

    this._saveState();
    return this.getState();
  }

  /**
   * Record work (success, progress, achievements)
   * @param {string} source - Source of work
   * @param {number} amount - Work amount (0-1 normalized, or raw value)
   * @returns {Object} Current state
   */
  recordWork(source, amount = 1) {
    // Normalize: if amount < 1, scale to WORK_PER_SUCCESS
    const work = amount < 1 ? amount * WORK_PER_SUCCESS : amount;

    this.state.session.work += work;
    this.state.totals.totalWork += work;

    // Track sources
    this.state.workSources[source] = (this.state.workSources[source] || 0) + work;

    // Work also increases entropy (but less than heat)
    this.state.session.entropy += work * ENTROPY_INCREASE_RATE * PHI_INV_2;
    this.state.session.lastAction = Date.now();

    // Update peak efficiency
    const efficiency = this.calculateEfficiency();
    if (efficiency > this.state.stats.peakEfficiency) {
      this.state.stats.peakEfficiency = efficiency;
    }

    this._saveState();
    return this.getState();
  }

  /**
   * Dissipate heat over time (natural cooling)
   * @returns {number} Heat dissipated
   */
  dissipateHeat() {
    const minutesSinceAction = (Date.now() - this.state.session.lastAction) / (60 * 1000);

    if (minutesSinceAction < 1) {
      return 0;
    }

    // Heat dissipates logarithmically over time
    const dissipation = Math.min(
      this.state.session.heat,
      this.state.session.heat * HEAT_DISSIPATION_RATE * Math.log(1 + minutesSinceAction)
    );

    this.state.session.heat = Math.max(0, this.state.session.heat - dissipation);
    return dissipation;
  }

  /**
   * Calculate current efficiency
   * η = W / (W + Q), capped at Carnot limit
   * @returns {number} Efficiency 0-1
   */
  calculateEfficiency() {
    const { work, heat } = this.state.session;
    const total = work + heat;

    if (total === 0) {
      return CARNOT_LIMIT; // Perfect efficiency when nothing happened
    }

    const rawEfficiency = work / total;
    return Math.min(rawEfficiency, CARNOT_LIMIT);
  }

  /**
   * Calculate temperature (heat rate)
   * @returns {number} Temperature
   */
  calculateTemperature() {
    const sessionMinutes = (Date.now() - this.state.session.startTime) / (60 * 1000);
    if (sessionMinutes === 0) return 0;
    return this.state.session.heat / sessionMinutes;
  }

  /**
   * Check if system is in critical state
   * @returns {boolean}
   */
  isCritical() {
    return this.state.session.heat >= CRITICAL_TEMPERATURE;
  }

  /**
   * Check if efficiency is dangerously low
   * @returns {boolean}
   */
  isLowEfficiency() {
    const efficiency = this.calculateEfficiency();
    return efficiency < PHI_INV_2; // < 38.2%
  }

  /**
   * Get current thermodynamic state
   * @returns {Object} Current state
   */
  getState() {
    this.dissipateHeat(); // Apply natural cooling

    const efficiency = this.calculateEfficiency();
    const temperature = this.calculateTemperature();

    return {
      heat: Math.round(this.state.session.heat),
      work: Math.round(this.state.session.work),
      entropy: Math.round(this.state.session.entropy * 100) / 100,
      efficiency: Math.round(efficiency * 100),
      efficiencyRaw: efficiency,
      temperature: Math.round(temperature * 10) / 10,
      isCritical: this.isCritical(),
      isLowEfficiency: this.isLowEfficiency(),
      carnotLimit: Math.round(CARNOT_LIMIT * 100),
      sessionDuration: Math.round((Date.now() - this.state.session.startTime) / (60 * 1000)),
    };
  }

  /**
   * Get recommendation based on current state
   * @returns {Object} Recommendation
   */
  getRecommendation() {
    const state = this.getState();

    if (state.isCritical) {
      return {
        level: 'CRITICAL',
        message: '*GROWL* Thermal runaway! Take a break. Heat exceeds safe levels.',
        action: 'break',
        confidenceModifier: 0.5, // Halve confidence
      };
    }

    if (state.efficiency < PHI_INV_3 * 100) { // < 23.6%
      return {
        level: 'LOW',
        message: '*head tilt* Low efficiency. More frustration than progress. Change approach?',
        action: 'pivot',
        confidenceModifier: 0.7, // 30% reduction
      };
    }

    if (state.entropy > 50) {
      return {
        level: 'ENTROPY',
        message: '*sniff* High entropy. Session becoming chaotic. Consider reset.',
        action: 'reset',
        confidenceModifier: 0.9, // 10% reduction
      };
    }

    if (state.temperature > CRITICAL_TEMPERATURE * PHI_INV) { // > 50°
      return {
        level: 'WARM',
        message: '*ears perk* Getting warm. Pace yourself.',
        action: 'slow',
        confidenceModifier: 0.95, // 5% reduction
      };
    }

    return {
      level: 'GOOD',
      message: '*tail wag* Good thermodynamic balance.',
      action: 'continue',
      confidenceModifier: 1.0, // No modification
    };
  }

  /**
   * Get heat sources analysis
   * @returns {Object[]} Sorted heat sources
   */
  analyzeHeatSources() {
    return Object.entries(this.state.heatSources)
      .map(([source, heat]) => ({ source, heat }))
      .sort((a, b) => b.heat - a.heat);
  }

  /**
   * Get work sources analysis
   * @returns {Object[]} Sorted work sources
   */
  analyzeWorkSources() {
    return Object.entries(this.state.workSources)
      .map(([source, work]) => ({ source, work }))
      .sort((a, b) => b.work - a.work);
  }

  /**
   * Get statistics
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.state.stats,
      totals: this.state.totals,
      currentState: this.getState(),
    };
  }

  /**
   * Reset entropy (take a break, refresh)
   * @returns {Object} Result
   */
  resetEntropy() {
    const previous = this.state.session.entropy;
    this.state.session.entropy = 0;
    this.state.stats.entropyResets++;
    this._saveState();

    return {
      reset: true,
      previousEntropy: previous,
      message: '*yawn* Entropy reset. Fresh start.',
    };
  }
}

// =============================================================================
// FACTORY
// =============================================================================

/**
 * Create thermodynamics tracker
 * @returns {ThermodynamicsTracker}
 */
export function createThermodynamicsTracker() {
  return new ThermodynamicsTracker();
}

export default ThermodynamicsTracker;

/**
 * Organism Module - 7 Dimensions of CYNIC's Life Force
 *
 * "Le chien est vivant, pas juste actif" - κυνικός
 *
 * Models CYNIC as a living organism with measurable, falsifiable dimensions:
 *
 * ┌─────────────────────────────────────────────────────────────────┐
 * │                    7 DIMENSIONS OF CYNIC                        │
 * ├─────────────────────────────────────────────────────────────────┤
 * │  1. METABOLISM    - Energy consumption (tokens, CPU, latency)   │
 * │  2. HOMEOSTASIS   - Stability (variance in metrics)             │
 * │  3. TEMPERATURE   - Heat accumulation (T = Q × decay)           │
 * │  4. EFFICIENCY    - Useful work ratio (η = W/(W+Q), max 61.8%) │
 * │  5. GROWTH        - Adaptation rate (patterns/hour)             │
 * │  6. RESILIENCE    - Recovery capability (MTTR, success rate)    │
 * │  7. VITALITY      - Composite life force (φ-weighted mean)      │
 * └─────────────────────────────────────────────────────────────────┘
 *
 * Each dimension is:
 * - Measurable: Has concrete metrics
 * - Falsifiable: Can prove hypotheses wrong
 * - φ-bounded: Max confidence 61.8%
 *
 * @module @cynic/node/organism
 */

'use strict';

// ═══════════════════════════════════════════════════════════════════════════════
// METABOLISM - Energy Consumption
// ═══════════════════════════════════════════════════════════════════════════════

export {
  MetabolicSample,
  MetabolismTracker,
  createMetabolismTracker,
  getMetabolismTracker,
  resetMetabolismTracker,
  METABOLISM_CONFIG,
} from './metabolism.js';

// ═══════════════════════════════════════════════════════════════════════════════
// THERMODYNAMICS - Heat, Work, Temperature, Efficiency
// ═══════════════════════════════════════════════════════════════════════════════

export {
  ThermoEvent,
  ThermoEventType,
  ThermodynamicState,
  createThermodynamicState,
  getThermodynamicState,
  resetThermodynamicState,
  THERMO_CONFIG,
} from './thermodynamics.js';

// ═══════════════════════════════════════════════════════════════════════════════
// HOMEOSTASIS - Stability and Equilibrium
// ═══════════════════════════════════════════════════════════════════════════════

export {
  MetricBaseline,
  HomeostasisTracker,
  createHomeostasisTracker,
  getHomeostasisTracker,
  resetHomeostasisTracker,
  HOMEOSTASIS_CONFIG,
} from './homeostasis.js';

// ═══════════════════════════════════════════════════════════════════════════════
// GROWTH - Adaptation and Learning Rate
// ═══════════════════════════════════════════════════════════════════════════════

export {
  GrowthEvent,
  GrowthTracker,
  createGrowthTracker,
  getGrowthTracker,
  resetGrowthTracker,
  GROWTH_CONFIG,
} from './growth.js';

// ═══════════════════════════════════════════════════════════════════════════════
// RESILIENCE - Recovery and Fault Tolerance
// ═══════════════════════════════════════════════════════════════════════════════

export {
  Incident,
  ResilienceTracker,
  createResilienceTracker,
  getResilienceTracker,
  resetResilienceTracker,
  RESILIENCE_CONFIG,
} from './resilience.js';

// ═══════════════════════════════════════════════════════════════════════════════
// VITALITY - Composite Life Force
// ═══════════════════════════════════════════════════════════════════════════════

export {
  VitalityMonitor,
  calculateVitality,
  getVitalityStatus,
  getDimensionScores,
  getDimensionStatuses,
  getVitalitySummary,
  createVitalityMonitor,
  getVitalityMonitor,
  resetVitalityMonitor,
  VITALITY_CONFIG,
} from './vitality.js';

// ═══════════════════════════════════════════════════════════════════════════════
// UNIFIED ORGANISM INTERFACE
// ═══════════════════════════════════════════════════════════════════════════════

import { getMetabolismTracker } from './metabolism.js';
import { getThermodynamicState } from './thermodynamics.js';
import { getHomeostasisTracker } from './homeostasis.js';
import { getGrowthTracker } from './growth.js';
import { getResilienceTracker } from './resilience.js';
import { getVitalityMonitor, getVitalitySummary, calculateVitality } from './vitality.js';

/**
 * Get all organism trackers
 * @returns {Object}
 */
export function getOrganismTrackers() {
  return {
    metabolism: getMetabolismTracker(),
    thermodynamics: getThermodynamicState(),
    homeostasis: getHomeostasisTracker(),
    growth: getGrowthTracker(),
    resilience: getResilienceTracker(),
    vitality: getVitalityMonitor(),
  };
}

/**
 * Get complete organism state
 * @returns {Object}
 */
export function getOrganismState() {
  const trackers = getOrganismTrackers();

  return {
    vitality: getVitalitySummary(),
    metabolism: trackers.metabolism.getStats(),
    thermodynamics: trackers.thermodynamics.getStats(),
    homeostasis: trackers.homeostasis.getStats(),
    growth: trackers.growth.getStats(),
    resilience: trackers.resilience.getStats(),
  };
}

/**
 * Get organism health report
 * @returns {Object}
 */
export function getOrganismHealth() {
  const vitality = getVitalityMonitor();
  return vitality.getHealthReport();
}

/**
 * Reset all organism trackers
 */
export function resetOrganismState() {
  getMetabolismTracker().reset();
  getThermodynamicState().reset();
  getHomeostasisTracker().reset();
  getGrowthTracker().reset();
  getResilienceTracker().reset();
  getVitalityMonitor().reset();
}

/**
 * Start organism monitoring
 */
export function startOrganismMonitoring() {
  getVitalityMonitor().start();
}

/**
 * Stop organism monitoring
 */
export function stopOrganismMonitoring() {
  getVitalityMonitor().stop();
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONVENIENCE RECORDING FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Record a metabolic event (token usage)
 */
export function recordMetabolic(data) {
  return getMetabolismTracker().record(data);
}

/**
 * Record a thermodynamic event (work or heat)
 */
export function recordThermo(type, magnitude = 1, metadata = {}) {
  return getThermodynamicState().record(type, magnitude, metadata);
}

/**
 * Record success (work)
 */
export function recordSuccess(magnitude = 1, metadata = {}) {
  return getThermodynamicState().recordSuccess(magnitude, metadata);
}

/**
 * Record error (heat)
 */
export function recordError(magnitude = 1, metadata = {}) {
  getThermodynamicState().recordError(magnitude, metadata);
  return getResilienceTracker().recordError(metadata);
}

/**
 * Record growth event
 */
export function recordGrowth(type, data = {}) {
  return getGrowthTracker().record(type, data);
}

/**
 * Update homeostasis metric
 */
export function updateHomeostasis(name, value) {
  return getHomeostasisTracker().update(name, value);
}

/**
 * Mark incident as recovered
 */
export function markRecovered(incidentId, success = true) {
  return getResilienceTracker().markRecovered(incidentId, success);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DEFAULTS
// ═══════════════════════════════════════════════════════════════════════════════

export default {
  // Trackers
  getOrganismTrackers,
  getOrganismState,
  getOrganismHealth,
  resetOrganismState,
  startOrganismMonitoring,
  stopOrganismMonitoring,

  // Recording
  recordMetabolic,
  recordThermo,
  recordSuccess,
  recordError,
  recordGrowth,
  updateHomeostasis,
  markRecovered,

  // Quick access
  calculateVitality,
  getVitalitySummary,
};

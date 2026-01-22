/**
 * CYNIC Cognitive Thermodynamics Module (Phase 10A)
 *
 * "Ἐνέργεια - the activity of being" - κυνικός
 *
 * Applies thermodynamic laws to cognitive work:
 * - Heat (Q) = frustration, friction, wasted effort
 * - Work (W) = useful output, progress, solutions
 * - Efficiency (η) = W / (W + Q), bounded by φ⁻¹
 * - Entropy (S) = disorder accumulation over time
 *
 * First Law: Energy is conserved (focus + distraction = constant)
 * Second Law: Entropy always increases (sessions tend toward chaos)
 * Carnot Limit: Maximum efficiency is φ⁻¹ (61.8%)
 *
 * @module cynic/lib/cognitive-thermodynamics
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// Import φ constants
const phiMath = require('./phi-math.cjs');
const { PHI, PHI_INV, PHI_INV_2, PHI_INV_3 } = phiMath;

// =============================================================================
// CONSTANTS (φ-derived)
// =============================================================================

/** Maximum theoretical efficiency - Carnot limit at φ⁻¹ */
const CARNOT_LIMIT = PHI_INV;

/** Heat dissipation rate per minute - φ⁻³ */
const HEAT_DISSIPATION_RATE = PHI_INV_3;

/** Entropy increase rate per action - φ⁻³ */
const ENTROPY_INCREASE_RATE = PHI_INV_3;

/** Critical temperature (frustration threshold) - φ × 50 ≈ 81 */
const CRITICAL_TEMPERATURE = Math.round(PHI * 50);

/** Absolute zero (minimum heat) */
const ABSOLUTE_ZERO = 0;

/** Work units per successful action */
const WORK_PER_SUCCESS = 10;

/** Heat units per frustration event */
const HEAT_PER_FRUSTRATION = 15;

// =============================================================================
// STORAGE
// =============================================================================

const THERMO_DIR = path.join(os.homedir(), '.cynic', 'thermodynamics');
const STATE_FILE = path.join(THERMO_DIR, 'state.json');
const HISTORY_FILE = path.join(THERMO_DIR, 'history.jsonl');

// =============================================================================
// STATE
// =============================================================================

const thermoState = {
  // Current session thermodynamics
  session: {
    heat: 0,           // Q - accumulated frustration
    work: 0,           // W - useful output
    entropy: 0,        // S - disorder
    startTime: Date.now(),
    lastAction: Date.now(),
  },

  // Running totals
  totals: {
    totalHeat: 0,
    totalWork: 0,
    totalEntropy: 0,
    sessions: 0,
  },

  // Heat sources tracking
  heatSources: {},

  // Work sources tracking
  workSources: {},

  stats: {
    averageEfficiency: PHI_INV_2,
    peakEfficiency: 0,
    thermalRunaways: 0,      // Times heat exceeded critical
    entropyResets: 0,        // Times entropy was reset
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(THERMO_DIR)) {
    fs.mkdirSync(THERMO_DIR, { recursive: true });
  }
}

function loadState() {
  ensureDir();
  if (!fs.existsSync(STATE_FILE)) {
    return null;
  }
  try {
    return JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
  } catch {
    return null;
  }
}

function saveState() {
  ensureDir();
  fs.writeFileSync(STATE_FILE, JSON.stringify({
    totals: thermoState.totals,
    stats: thermoState.stats,
  }, null, 2));
}

function appendHistory(event) {
  ensureDir();
  const line = JSON.stringify({ ...event, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(HISTORY_FILE, line);
}

// =============================================================================
// THERMODYNAMIC OPERATIONS
// =============================================================================

/**
 * Add heat to the system (frustration event)
 *
 * @param {number} amount - Heat amount
 * @param {string} source - Heat source
 * @returns {Object} Updated state
 */
function addHeat(amount, source = 'unknown') {
  thermoState.session.heat += amount;
  thermoState.totals.totalHeat += amount;

  // Track heat sources
  thermoState.heatSources[source] = (thermoState.heatSources[source] || 0) + amount;

  // Check for thermal runaway
  if (thermoState.session.heat >= CRITICAL_TEMPERATURE) {
    thermoState.stats.thermalRunaways++;
    appendHistory({
      type: 'thermal_runaway',
      heat: thermoState.session.heat,
      source,
    });
  }

  // Entropy increases with heat
  thermoState.session.entropy += amount * ENTROPY_INCREASE_RATE;

  thermoState.session.lastAction = Date.now();

  return getState();
}

/**
 * Add work to the system (useful output)
 *
 * @param {number} amount - Work amount
 * @param {string} source - Work source
 * @returns {Object} Updated state
 */
function addWork(amount, source = 'unknown') {
  thermoState.session.work += amount;
  thermoState.totals.totalWork += amount;

  // Track work sources
  thermoState.workSources[source] = (thermoState.workSources[source] || 0) + amount;

  // Work also increases entropy (but less than heat)
  thermoState.session.entropy += amount * ENTROPY_INCREASE_RATE * PHI_INV_2;

  thermoState.session.lastAction = Date.now();

  // Update efficiency stats
  const efficiency = calculateEfficiency();
  if (efficiency > thermoState.stats.peakEfficiency) {
    thermoState.stats.peakEfficiency = efficiency;
  }

  return getState();
}

/**
 * Dissipate heat over time (natural cooling)
 *
 * @returns {number} Heat dissipated
 */
function dissipateHeat() {
  const now = Date.now();
  const minutesSinceAction = (now - thermoState.session.lastAction) / (60 * 1000);

  if (minutesSinceAction < 1) {
    return 0;
  }

  // Heat dissipates logarithmically over time
  const dissipation = Math.min(
    thermoState.session.heat,
    thermoState.session.heat * HEAT_DISSIPATION_RATE * Math.log(1 + minutesSinceAction)
  );

  thermoState.session.heat = Math.max(ABSOLUTE_ZERO, thermoState.session.heat - dissipation);

  return dissipation;
}

/**
 * Calculate current efficiency
 * Efficiency = W / (W + Q), capped at Carnot limit
 *
 * @returns {number} Efficiency 0-1
 */
function calculateEfficiency() {
  const { work, heat } = thermoState.session;
  const total = work + heat;

  if (total === 0) {
    return CARNOT_LIMIT; // Perfect efficiency when nothing happened
  }

  const rawEfficiency = work / total;

  // Cap at Carnot limit (φ⁻¹)
  return Math.min(rawEfficiency, CARNOT_LIMIT);
}

/**
 * Calculate temperature (heat intensity)
 *
 * @returns {number} Temperature
 */
function calculateTemperature() {
  const sessionMinutes = (Date.now() - thermoState.session.startTime) / (60 * 1000);
  if (sessionMinutes === 0) return ABSOLUTE_ZERO;

  // Temperature = heat / time (heat rate)
  return thermoState.session.heat / sessionMinutes;
}

/**
 * Get current thermodynamic state
 *
 * @returns {Object} Current state
 */
function getState() {
  dissipateHeat(); // Apply natural cooling

  const efficiency = calculateEfficiency();
  const temperature = calculateTemperature();

  return {
    heat: Math.round(thermoState.session.heat),
    work: Math.round(thermoState.session.work),
    entropy: Math.round(thermoState.session.entropy * 100) / 100,
    efficiency: Math.round(efficiency * 100),
    efficiencyRaw: efficiency,
    temperature: Math.round(temperature * 10) / 10,
    isCritical: thermoState.session.heat >= CRITICAL_TEMPERATURE,
    carnotLimit: Math.round(CARNOT_LIMIT * 100),
    sessionDuration: Math.round((Date.now() - thermoState.session.startTime) / (60 * 1000)),
  };
}

// =============================================================================
// HEAT/WORK DETECTION
// =============================================================================

/**
 * Heat-generating events
 */
const HEAT_EVENTS = {
  error: { heat: HEAT_PER_FRUSTRATION, source: 'error' },
  retry: { heat: HEAT_PER_FRUSTRATION * PHI_INV, source: 'retry' },
  confusion: { heat: HEAT_PER_FRUSTRATION * PHI_INV_2, source: 'confusion' },
  blocked: { heat: HEAT_PER_FRUSTRATION * PHI, source: 'blocked' },
  timeout: { heat: HEAT_PER_FRUSTRATION, source: 'timeout' },
  rejection: { heat: HEAT_PER_FRUSTRATION * PHI_INV, source: 'rejection' },
};

/**
 * Work-generating events
 */
const WORK_EVENTS = {
  codeWritten: { work: WORK_PER_SUCCESS, source: 'code' },
  testPassed: { work: WORK_PER_SUCCESS * PHI_INV, source: 'test' },
  bugFixed: { work: WORK_PER_SUCCESS * PHI, source: 'bugfix' },
  questionAnswered: { work: WORK_PER_SUCCESS * PHI_INV_2, source: 'answer' },
  commitMade: { work: WORK_PER_SUCCESS, source: 'commit' },
  prMerged: { work: WORK_PER_SUCCESS * PHI, source: 'merge' },
};

/**
 * Record a heat event
 *
 * @param {string} eventType - Type of heat event
 * @param {Object} context - Additional context
 * @returns {Object} Updated state
 */
function recordHeatEvent(eventType, context = {}) {
  const event = HEAT_EVENTS[eventType];
  if (!event) {
    return addHeat(HEAT_PER_FRUSTRATION * PHI_INV_3, eventType);
  }

  const heat = context.severity
    ? event.heat * context.severity
    : event.heat;

  appendHistory({
    type: 'heat',
    eventType,
    heat,
    context,
  });

  return addHeat(heat, event.source);
}

/**
 * Record a work event
 *
 * @param {string} eventType - Type of work event
 * @param {Object} context - Additional context
 * @returns {Object} Updated state
 */
function recordWorkEvent(eventType, context = {}) {
  const event = WORK_EVENTS[eventType];
  if (!event) {
    return addWork(WORK_PER_SUCCESS * PHI_INV_3, eventType);
  }

  const work = context.magnitude
    ? event.work * context.magnitude
    : event.work;

  appendHistory({
    type: 'work',
    eventType,
    work,
    context,
  });

  return addWork(work, event.source);
}

// =============================================================================
// SESSION MANAGEMENT
// =============================================================================

/**
 * Start a new thermodynamic session
 */
function startSession() {
  // Save previous session if exists
  if (thermoState.session.work > 0 || thermoState.session.heat > 0) {
    const efficiency = calculateEfficiency();
    const n = thermoState.totals.sessions + 1;
    thermoState.stats.averageEfficiency =
      (thermoState.stats.averageEfficiency * thermoState.totals.sessions + efficiency) / n;
    thermoState.totals.sessions = n;
  }

  thermoState.session = {
    heat: 0,
    work: 0,
    entropy: 0,
    startTime: Date.now(),
    lastAction: Date.now(),
  };

  thermoState.heatSources = {};
  thermoState.workSources = {};

  saveState();
}

/**
 * Reset entropy (take a break, refresh)
 *
 * @returns {Object} Result
 */
function resetEntropy() {
  const previousEntropy = thermoState.session.entropy;

  thermoState.session.entropy = 0;
  thermoState.stats.entropyResets++;

  appendHistory({
    type: 'entropy_reset',
    previousEntropy,
  });

  saveState();

  return {
    reset: true,
    previousEntropy,
    message: '*yawn* Entropy reset. Fresh start.',
  };
}

// =============================================================================
// ANALYSIS
// =============================================================================

/**
 * Analyze heat sources
 *
 * @returns {Object[]} Sorted heat sources
 */
function analyzeHeatSources() {
  return Object.entries(thermoState.heatSources)
    .map(([source, heat]) => ({ source, heat }))
    .sort((a, b) => b.heat - a.heat);
}

/**
 * Analyze work sources
 *
 * @returns {Object[]} Sorted work sources
 */
function analyzeWorkSources() {
  return Object.entries(thermoState.workSources)
    .map(([source, work]) => ({ source, work }))
    .sort((a, b) => b.work - a.work);
}

/**
 * Get efficiency recommendation
 *
 * @returns {Object} Recommendation
 */
function getRecommendation() {
  const state = getState();

  if (state.isCritical) {
    return {
      level: 'CRITICAL',
      message: '*GROWL* Thermal runaway! Take a break. Heat exceeds safe levels.',
      action: 'break',
    };
  }

  if (state.efficiency < PHI_INV_3 * 100) {
    return {
      level: 'LOW',
      message: '*head tilt* Low efficiency. More frustration than progress. Change approach?',
      action: 'pivot',
    };
  }

  if (state.entropy > 50) {
    return {
      level: 'ENTROPY',
      message: '*sniff* High entropy. Session becoming chaotic. Consider reset.',
      action: 'reset',
    };
  }

  if (state.temperature > CRITICAL_TEMPERATURE * PHI_INV) {
    return {
      level: 'WARM',
      message: '*ears perk* Getting warm. Pace yourself.',
      action: 'slow',
    };
  }

  return {
    level: 'GOOD',
    message: '*tail wag* Good thermodynamic balance.',
    action: 'continue',
  };
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize cognitive thermodynamics
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    thermoState.totals = saved.totals || thermoState.totals;
    thermoState.stats = saved.stats || thermoState.stats;
  }
  startSession();
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...thermoState.stats,
    totals: thermoState.totals,
    currentState: getState(),
  };
}

/**
 * Format state for display
 *
 * @returns {string} Formatted display
 */
function formatState() {
  const state = getState();
  const rec = getRecommendation();

  // Temperature bar
  const tempPercent = Math.min(100, (state.temperature / CRITICAL_TEMPERATURE) * 100);
  const tempBar = '█'.repeat(Math.round(tempPercent / 10)) +
                  '░'.repeat(10 - Math.round(tempPercent / 10));

  // Efficiency bar
  const effBar = '█'.repeat(Math.round(state.efficiency / 10)) +
                 '░'.repeat(10 - Math.round(state.efficiency / 10));

  const criticalIndicator = state.isCritical ? ' 🔥 CRITICAL' : '';

  const lines = [
    '── COGNITIVE THERMODYNAMICS ───────────────────────────────',
    `   Heat (Q):    ${state.heat} units${criticalIndicator}`,
    `   Work (W):    ${state.work} units`,
    `   Entropy (S): ${state.entropy}`,
    '',
    `   Temperature: [${tempBar}] ${state.temperature}°`,
    `   Efficiency:  [${effBar}] ${state.efficiency}% (max ${state.carnotLimit}%)`,
    '',
    `   Session: ${state.sessionDuration} min`,
    '',
    `   ${rec.message}`,
  ];

  return lines.join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  CARNOT_LIMIT,
  CRITICAL_TEMPERATURE,
  WORK_PER_SUCCESS,
  HEAT_PER_FRUSTRATION,

  // Core functions
  init,
  getState,
  getStats,

  // Thermodynamic operations
  addHeat,
  addWork,
  dissipateHeat,
  calculateEfficiency,
  calculateTemperature,

  // Event recording
  recordHeatEvent,
  recordWorkEvent,

  // Session management
  startSession,
  resetEntropy,

  // Analysis
  analyzeHeatSources,
  analyzeWorkSources,
  getRecommendation,

  // Display
  formatState,
};

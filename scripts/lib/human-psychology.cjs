/**
 * CYNIC Human Psychology Module
 *
 * "Comprendre l'humain pour mieux l'aider" - κυνικός
 *
 * Tracks psychological state dimensions with φ-derived thresholds.
 * All values use confidence intervals - CYNIC never claims certainty.
 *
 * @module cynic/lib/human-psychology
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// Import φ constants from phi-math
const phiMath = require('./phi-math.cjs');
const { PHI, PHI_INV, PHI_INV_2, PHI_INV_3 } = phiMath;

// Signal collector - lazy loaded to avoid circular dependency
let signalCollector = null;

// =============================================================================
// CONSTANTS (φ-derived, no magic numbers)
// =============================================================================

/** Neutral state value - φ⁻¹ */
const STATE_NEUTRAL = PHI_INV;

/** Low threshold - φ⁻² */
const THRESHOLD_LOW = PHI_INV_2;

/** Critical threshold - φ⁻³ */
const THRESHOLD_CRITICAL = PHI_INV_3;

/** Maximum confidence (φ distrusts φ) */
const MAX_CONFIDENCE = PHI_INV;

/** Signal decay rate per minute */
const SIGNAL_DECAY = PHI_INV_3;

/** Focus cycle duration in minutes - φ⁻¹ × 100 */
const FOCUS_CYCLE_MIN = PHI_INV * 100;

/** Break cycle duration in minutes - φ⁻³ × 100 */
const BREAK_CYCLE_MIN = PHI_INV_3 * 100;

// =============================================================================
// STORAGE
// =============================================================================

const PSYCHOLOGY_DIR = path.join(os.homedir(), '.cynic', 'psychology');
const STATE_FILE = path.join(PSYCHOLOGY_DIR, 'state.json');
const SIGNALS_FILE = path.join(PSYCHOLOGY_DIR, 'signals.jsonl');

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/**
 * Create a dimension state with value, confidence, and trend
 * @param {number} value - Current value (0.0 - 1.0)
 * @param {number} confidence - Confidence in this value (0.0 - PHI_INV)
 * @returns {Object} Dimension state
 */
function createDimension(value = STATE_NEUTRAL, confidence = THRESHOLD_LOW) {
  return {
    value: Math.max(0, Math.min(1, value)),
    confidence: Math.max(0, Math.min(MAX_CONFIDENCE, confidence)),
    trend: 'stable', // 'rising' | 'falling' | 'stable'
    lastUpdate: Date.now(),
  };
}

/**
 * Create default psychological state
 * @returns {Object} Default state
 */
function createDefaultState() {
  return {
    version: 1,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    sessionStart: Date.now(),

    // Core dimensions (6)
    dimensions: {
      energy: createDimension(),
      focus: createDimension(),
      creativity: createDimension(),
      frustration: createDimension(THRESHOLD_LOW), // Start low
      confidence: createDimension(),
      riskAppetite: createDimension(),
    },

    // Emotional spectrum (6) - Phase 5, but structure now
    emotions: {
      joy: createDimension(THRESHOLD_LOW),
      pride: createDimension(THRESHOLD_LOW),
      shame: createDimension(0),
      anxiety: createDimension(THRESHOLD_LOW),
      boredom: createDimension(0),
      curiosity: createDimension(),
    },

    // Composite states (calculated, not stored)
    // flow, exploration, grind, burnoutRisk, breakthrough, procrastination

    // Temporal tracking
    temporal: {
      sessionDuration: 0,
      lastActionTime: Date.now(),
      actionIntervals: [], // Rolling window of intervals
      circadianHour: new Date().getHours(),
    },

    // Signal history (for trend calculation)
    signalHistory: [],
  };
}

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(PSYCHOLOGY_DIR)) {
    fs.mkdirSync(PSYCHOLOGY_DIR, { recursive: true });
  }
}

function loadState() {
  ensureDir();
  if (!fs.existsSync(STATE_FILE)) {
    const defaultState = createDefaultState();
    saveState(defaultState);
    return defaultState;
  }
  try {
    return JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
  } catch {
    return createDefaultState();
  }
}

function saveState(state) {
  ensureDir();
  state.updatedAt = Date.now();
  fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
}

function appendSignal(signal) {
  ensureDir();
  const line = JSON.stringify({ ...signal, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(SIGNALS_FILE, line);
}

// =============================================================================
// DIMENSION OPERATIONS
// =============================================================================

/**
 * Update a dimension with a delta value
 * @param {Object} state - Current state
 * @param {string} dimension - Dimension name
 * @param {number} delta - Change amount (-1.0 to 1.0)
 * @param {number} signalConfidence - Confidence in this signal
 * @returns {Object} Updated state
 */
function updateDimension(state, dimension, delta, signalConfidence = THRESHOLD_LOW) {
  const dim = state.dimensions[dimension];
  if (!dim) return state;

  const oldValue = dim.value;

  // Apply delta weighted by signal confidence
  const weightedDelta = delta * signalConfidence;
  const newValue = Math.max(0, Math.min(1, oldValue + weightedDelta));

  // Update confidence (increases with more signals, decays over time)
  const timeSinceUpdate = (Date.now() - dim.lastUpdate) / (1000 * 60); // minutes
  const decayedConfidence = dim.confidence * Math.pow(1 - SIGNAL_DECAY, timeSinceUpdate);
  const newConfidence = Math.min(MAX_CONFIDENCE, decayedConfidence + signalConfidence * THRESHOLD_LOW);

  // Calculate trend
  let trend = 'stable';
  const valueDiff = newValue - oldValue;
  if (valueDiff > 0.05) trend = 'rising';
  else if (valueDiff < -0.05) trend = 'falling';

  state.dimensions[dimension] = {
    value: newValue,
    confidence: newConfidence,
    trend,
    lastUpdate: Date.now(),
  };

  return state;
}

/**
 * Update an emotion with a delta value
 * @param {Object} state - Current state
 * @param {string} emotion - Emotion name
 * @param {number} delta - Change amount
 * @param {number} signalConfidence - Confidence in this signal
 * @returns {Object} Updated state
 */
function updateEmotion(state, emotion, delta, signalConfidence = THRESHOLD_LOW) {
  const emo = state.emotions[emotion];
  if (!emo) return state;

  const oldValue = emo.value;
  const weightedDelta = delta * signalConfidence;
  const newValue = Math.max(0, Math.min(1, oldValue + weightedDelta));

  const timeSinceUpdate = (Date.now() - emo.lastUpdate) / (1000 * 60);
  const decayedConfidence = emo.confidence * Math.pow(1 - SIGNAL_DECAY, timeSinceUpdate);
  const newConfidence = Math.min(MAX_CONFIDENCE, decayedConfidence + signalConfidence * THRESHOLD_LOW);

  let trend = 'stable';
  const valueDiff = newValue - oldValue;
  if (valueDiff > 0.05) trend = 'rising';
  else if (valueDiff < -0.05) trend = 'falling';

  state.emotions[emotion] = {
    value: newValue,
    confidence: newConfidence,
    trend,
    lastUpdate: Date.now(),
  };

  return state;
}

// =============================================================================
// COMPOSITE STATE CALCULATIONS
// =============================================================================

/**
 * Calculate composite states from dimensions and emotions
 * @param {Object} state - Current state
 * @returns {Object} Composite states with boolean values
 */
function calculateComposites(state) {
  const d = state.dimensions;
  const e = state.emotions;

  return {
    // Flow: High focus + medium-high energy + low frustration
    flow: d.focus.value > PHI_INV &&
          d.energy.value > THRESHOLD_LOW &&
          d.frustration.value < THRESHOLD_LOW,

    // Exploration: High creativity + high curiosity + lower focus
    exploration: d.creativity.value > PHI_INV &&
                 e.curiosity.value > PHI_INV &&
                 d.focus.value < PHI_INV,

    // Grind: High focus + low creativity + medium frustration
    grind: d.focus.value > PHI_INV &&
           d.creativity.value < THRESHOLD_LOW &&
           d.frustration.value > THRESHOLD_LOW &&
           d.frustration.value < PHI_INV,

    // Burnout risk: Low energy + high frustration + falling trends
    burnoutRisk: d.energy.value < THRESHOLD_LOW &&
                 d.frustration.value > PHI_INV &&
                 (d.energy.trend === 'falling' || d.frustration.trend === 'rising'),

    // Breakthrough: Rising energy after frustration drop
    breakthrough: d.energy.trend === 'rising' &&
                  d.frustration.trend === 'falling' &&
                  e.joy.value > THRESHOLD_LOW,

    // Procrastination: Low focus + high boredom + activity without progress
    procrastination: d.focus.value < THRESHOLD_LOW &&
                     e.boredom.value > THRESHOLD_LOW &&
                     d.creativity.value < THRESHOLD_LOW,
  };
}

// =============================================================================
// SIGNAL PROCESSING
// =============================================================================

/**
 * Process an incoming signal and update state
 * @param {Object} signal - Signal from signal-collector
 * @returns {Object} Updated state
 */
function processSignal(signal) {
  let state = loadState();

  // Update temporal tracking
  const now = Date.now();
  const interval = now - state.temporal.lastActionTime;
  state.temporal.lastActionTime = now;
  state.temporal.sessionDuration = now - state.sessionStart;
  state.temporal.circadianHour = new Date().getHours();

  // Keep rolling window of intervals (last 20)
  state.temporal.actionIntervals.push(interval);
  if (state.temporal.actionIntervals.length > 20) {
    state.temporal.actionIntervals.shift();
  }

  // Route signal to appropriate dimension updates
  switch (signal.type) {
    case 'action_success':
      state = updateDimension(state, 'confidence', 0.05, signal.confidence || THRESHOLD_LOW);
      state = updateDimension(state, 'frustration', -0.02, signal.confidence || THRESHOLD_LOW);
      state = updateEmotion(state, 'joy', 0.02, signal.confidence || THRESHOLD_LOW);
      break;

    case 'action_failure':
      state = updateDimension(state, 'frustration', 0.1, signal.confidence || THRESHOLD_LOW);
      state = updateDimension(state, 'confidence', -0.05, signal.confidence || THRESHOLD_LOW);
      break;

    case 'repeated_failure':
      state = updateDimension(state, 'frustration', 0.2, signal.confidence || PHI_INV);
      state = updateDimension(state, 'energy', -0.05, signal.confidence || THRESHOLD_LOW);
      state = updateEmotion(state, 'anxiety', 0.1, signal.confidence || THRESHOLD_LOW);
      break;

    case 'fast_actions':
      state = updateDimension(state, 'focus', 0.1, signal.confidence || THRESHOLD_LOW);
      state = updateDimension(state, 'energy', 0.05, signal.confidence || THRESHOLD_LOW);
      break;

    case 'slow_actions':
      // Could be fatigue OR deep thinking - lower confidence
      state = updateDimension(state, 'energy', -0.03, THRESHOLD_CRITICAL);
      break;

    case 'context_switch':
      state = updateDimension(state, 'focus', -0.1, signal.confidence || THRESHOLD_LOW);
      break;

    case 'creative_action':
      state = updateDimension(state, 'creativity', 0.1, signal.confidence || THRESHOLD_LOW);
      state = updateEmotion(state, 'curiosity', 0.05, signal.confidence || THRESHOLD_LOW);
      break;

    case 'long_session':
      state = updateDimension(state, 'energy', -0.1, signal.confidence || THRESHOLD_LOW);
      break;

    case 'break_taken':
      state = updateDimension(state, 'energy', 0.2, signal.confidence || PHI_INV);
      state = updateDimension(state, 'frustration', -0.1, signal.confidence || THRESHOLD_LOW);
      break;

    // ═══════════════════════════════════════════════════════════════════════════
    // TEMPORAL PERCEPTION SIGNALS (Phase 22)
    // "Le chien sent le temps qui passe"
    // ═══════════════════════════════════════════════════════════════════════════

    case 'temporal_fatigue':
      // Late night or low circadian energy
      state = updateDimension(state, 'energy', -0.1, signal.confidence || THRESHOLD_LOW);
      state = updateEmotion(state, 'boredom', 0.05, signal.confidence || THRESHOLD_CRITICAL);
      break;

    case 'temporal_frustration':
      // Rapid tempo suggesting frustration
      state = updateDimension(state, 'frustration', 0.15, signal.confidence || THRESHOLD_LOW);
      state = updateDimension(state, 'focus', -0.05, signal.confidence || THRESHOLD_CRITICAL);
      break;

    case 'temporal_flow':
      // Steady, productive tempo
      state = updateDimension(state, 'focus', 0.1, signal.confidence || THRESHOLD_LOW);
      state = updateDimension(state, 'energy', 0.03, signal.confidence || THRESHOLD_CRITICAL);
      state = updateDimension(state, 'frustration', -0.05, signal.confidence || THRESHOLD_CRITICAL);
      break;

    case 'temporal_idle':
      // Very slow tempo - distraction or fatigue
      state = updateDimension(state, 'focus', -0.1, signal.confidence || THRESHOLD_CRITICAL);
      state = updateEmotion(state, 'boredom', 0.1, signal.confidence || THRESHOLD_LOW);
      break;

    case 'circadian_peak':
      // Morning peak energy (baseline boost)
      state = updateDimension(state, 'energy', 0.05, signal.confidence || THRESHOLD_LOW);
      state = updateDimension(state, 'creativity', 0.03, signal.confidence || THRESHOLD_CRITICAL);
      break;

    case 'circadian_dip':
      // Afternoon energy dip (baseline reduction)
      state = updateDimension(state, 'energy', -0.05, signal.confidence || THRESHOLD_CRITICAL);
      break;

    case 'late_night':
      // Late night work - fatigue warning
      state = updateDimension(state, 'energy', -0.15, signal.confidence || THRESHOLD_LOW);
      state = updateDimension(state, 'riskAppetite', 0.05, signal.confidence || THRESHOLD_CRITICAL);
      break;

    case 'weekend_session':
      // Weekend work - different mode
      state = updateDimension(state, 'creativity', 0.05, signal.confidence || THRESHOLD_CRITICAL);
      state = updateDimension(state, 'focus', -0.03, signal.confidence || THRESHOLD_CRITICAL);
      break;

    // ═══════════════════════════════════════════════════════════════════════════
    // ERROR PERCEPTION SIGNALS (Phase 22)
    // "Le chien renifle les erreurs"
    // ═══════════════════════════════════════════════════════════════════════════

    case 'error_rate_high':
      // High error rate - frustration and confidence drop
      state = updateDimension(state, 'frustration', 0.15, signal.confidence || THRESHOLD_LOW);
      state = updateDimension(state, 'confidence', -0.1, signal.confidence || THRESHOLD_LOW);
      break;

    case 'error_consecutive':
      // Multiple consecutive errors - stuck state
      state = updateDimension(state, 'frustration', 0.2, signal.confidence || PHI_INV);
      state = updateDimension(state, 'focus', -0.1, signal.confidence || THRESHOLD_LOW);
      state = updateEmotion(state, 'anxiety', 0.15, signal.confidence || THRESHOLD_LOW);
      break;

    case 'error_repeated':
      // Same error repeating - deep stuck
      state = updateDimension(state, 'frustration', 0.25, signal.confidence || PHI_INV);
      state = updateDimension(state, 'confidence', -0.15, signal.confidence || THRESHOLD_LOW);
      state = updateDimension(state, 'creativity', -0.1, signal.confidence || THRESHOLD_CRITICAL);
      break;

    case 'error_escalating':
      // Errors increasing over time
      state = updateDimension(state, 'frustration', 0.2, signal.confidence || PHI_INV);
      state = updateDimension(state, 'energy', -0.1, signal.confidence || THRESHOLD_LOW);
      break;

    case 'error_resolved':
      // Breakthrough after errors
      state = updateDimension(state, 'frustration', -0.2, signal.confidence || PHI_INV);
      state = updateDimension(state, 'confidence', 0.1, signal.confidence || THRESHOLD_LOW);
      state = updateEmotion(state, 'joy', 0.15, signal.confidence || THRESHOLD_LOW);
      state = updateEmotion(state, 'pride', 0.1, signal.confidence || THRESHOLD_CRITICAL);
      break;

    default:
      // Unknown signal type - log but don't crash
      break;
  }

  // Append to signal log
  appendSignal(signal);

  // Save updated state
  saveState(state);

  return state;
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize psychology module
 */
function init() {
  ensureDir();
  const state = loadState();

  // Reset session if this is a new session (gap > 30 min)
  const timeSinceUpdate = Date.now() - state.updatedAt;
  if (timeSinceUpdate > 30 * 60 * 1000) {
    state.sessionStart = Date.now();
    state.temporal.sessionDuration = 0;
    state.temporal.actionIntervals = [];
    saveState(state);
  }

  return state;
}

/**
 * Get current psychological state
 * @returns {Object} Full state with composites
 */
function getState() {
  const state = loadState();
  return {
    ...state,
    composites: calculateComposites(state),
  };
}

/**
 * Get a summary suitable for display
 * @returns {Object} Summary with key metrics
 */
function getSummary() {
  const state = getState();
  const d = state.dimensions;
  const c = state.composites;

  // Determine overall state
  let overallState = 'normal';
  let emoji = '*sniff*';

  if (c.flow) {
    overallState = 'flow';
    emoji = '*tail wag*';
  } else if (c.burnoutRisk) {
    overallState = 'burnout_risk';
    emoji = '*GROWL*';
  } else if (c.exploration) {
    overallState = 'exploration';
    emoji = '*ears perk*';
  } else if (c.grind) {
    overallState = 'grind';
    emoji = '*head tilt*';
  } else if (c.procrastination) {
    overallState = 'procrastination';
    emoji = '*yawn*';
  }

  // Get entropy from signal collector (Phase 6A)
  // Lazy load to avoid circular dependency
  let entropy = null;
  if (!signalCollector) {
    try {
      signalCollector = require('./signal-collector.cjs');
    } catch (e) {
      // Signal collector not available
    }
  }
  if (signalCollector) {
    try {
      entropy = signalCollector.calculateSessionEntropy();
    } catch (e) {
      // Entropy calculation failed - leave as null
    }
  }

  return {
    overallState,
    emoji,
    energy: { value: d.energy.value, trend: d.energy.trend },
    focus: { value: d.focus.value, trend: d.focus.trend },
    frustration: { value: d.frustration.value, trend: d.frustration.trend },
    sessionMinutes: Math.round(state.temporal.sessionDuration / (1000 * 60)),
    composites: c,
    confidence: Math.max(d.energy.confidence, d.focus.confidence, d.frustration.confidence),
    entropy, // Phase 6A: session entropy metric
  };
}

/**
 * Reset state for new session
 */
function resetSession() {
  const state = createDefaultState();
  saveState(state);
  return state;
}

// =============================================================================
// CROSS-SESSION PERSISTENCE
// "Le chien apprend. L'apprentissage persiste."
// =============================================================================

/**
 * Export psychology data for cross-session persistence
 * Called by sleep.cjs to sync to PostgreSQL
 * @returns {Object} Data for persistence
 */
function exportForPersistence() {
  const state = loadState();
  if (!state) return null;

  // Also gather learning loop data if available
  let calibration = null;
  let userPatterns = null;
  let interventionStats = null;

  try {
    const learningLoop = require('./learning-loop.cjs');
    const calData = learningLoop.getCalibration();
    calibration = calData;
    userPatterns = learningLoop.getUserPatterns();
  } catch (e) {
    // Learning loop not available
  }

  try {
    const interventionEngine = require('./intervention-engine.cjs');
    interventionStats = interventionEngine.getStats();
  } catch (e) {
    // Intervention engine not available
  }

  return {
    dimensions: state.dimensions,
    emotions: state.emotions,
    temporal: {
      lastSessionDuration: state.temporal?.sessionDuration || 0,
      circadianHour: state.temporal?.circadianHour,
      lastActionTime: state.temporal?.lastActionTime,
    },
    calibration,
    userPatterns,
    interventionStats,
  };
}

/**
 * Import psychology data from cross-session persistence
 * Called by awaken.cjs to restore from PostgreSQL
 * @param {Object} data - Data from PostgreSQL
 */
function importFromPersistence(data) {
  if (!data) return;

  const state = loadState() || createDefaultState();

  // Merge dimensions (keep current session values if higher confidence)
  if (data.dimensions) {
    for (const [key, imported] of Object.entries(data.dimensions)) {
      if (state.dimensions[key]) {
        const current = state.dimensions[key];
        // Keep current if higher confidence, else blend
        if (current.confidence < (imported.confidence || 0)) {
          state.dimensions[key] = {
            ...imported,
            lastUpdate: Date.now(),
          };
        }
      }
    }
  }

  // Merge emotions similarly
  if (data.emotions) {
    for (const [key, imported] of Object.entries(data.emotions)) {
      if (state.emotions[key]) {
        const current = state.emotions[key];
        if (current.confidence < (imported.confidence || 0)) {
          state.emotions[key] = {
            ...imported,
            lastUpdate: Date.now(),
          };
        }
      }
    }
  }

  saveState(state);

  // Restore learning loop data if available
  if (data.calibration || data.userPatterns) {
    try {
      const learningLoop = require('./learning-loop.cjs');
      learningLoop.importLearningData({
        calibration: data.calibration,
        userPatterns: data.userPatterns,
      });
    } catch (e) {
      // Learning loop not available
    }
  }
}

/**
 * Sync psychology to database
 * Wrapper for MCP persistence call
 * @param {string} userId - User ID
 * @returns {Promise<Object|null>} Sync result
 */
async function syncToDB(userId) {
  const data = exportForPersistence();
  if (!data) return null;

  try {
    // Dynamic import to avoid circular dependencies
    const fetch = (await import('node-fetch')).default;
    const mcpUrl = process.env.CYNIC_MCP_URL || 'http://localhost:3001';

    const response = await fetch(`${mcpUrl}/sync-psychology`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ userId, data }),
    });

    if (response.ok) {
      return await response.json();
    }
    return null;
  } catch (e) {
    // MCP not available - data is still saved locally
    return null;
  }
}

/**
 * Load psychology from database
 * Wrapper for MCP persistence call
 * @param {string} userId - User ID
 * @returns {Promise<Object|null>} Loaded data
 */
async function loadFromDB(userId) {
  try {
    const fetch = (await import('node-fetch')).default;
    const mcpUrl = process.env.CYNIC_MCP_URL || 'http://localhost:3001';

    const response = await fetch(`${mcpUrl}/load-psychology?userId=${encodeURIComponent(userId)}`);

    if (response.ok) {
      const data = await response.json();
      if (data) {
        importFromPersistence(data);
        return data;
      }
    }
    return null;
  } catch (e) {
    // MCP not available
    return null;
  }
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  STATE_NEUTRAL,
  THRESHOLD_LOW,
  THRESHOLD_CRITICAL,
  MAX_CONFIDENCE,
  FOCUS_CYCLE_MIN,
  BREAK_CYCLE_MIN,

  // Core functions
  init,
  getState,
  getSummary,
  processSignal,
  resetSession,

  // Dimension operations
  updateDimension,
  updateEmotion,
  calculateComposites,

  // Cross-session persistence
  exportForPersistence,
  importFromPersistence,
  syncToDB,
  loadFromDB,

  // For testing
  createDefaultState,
  loadState,
  saveState,
};

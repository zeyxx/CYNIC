/**
 * CYNIC Attractor Mapping (Phase 13B)
 *
 * "Ἕλξις - the pull toward stability" - κυνικός
 *
 * Maps stable behavioral states (attractors):
 * - Fixed points: stable states that persist
 * - Limit cycles: periodic behavioral loops
 * - Strange attractors: chaotic but bounded patterns
 * - Basins: regions that flow toward attractors
 *
 * From dynamical systems theory:
 * "All systems settle into attractors eventually."
 *
 * @module cynic/lib/attractor-mapping
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

/** Minimum observations for attractor detection - φ × 5 ≈ 8 */
const MIN_OBSERVATIONS = Math.round(PHI * 5);

/** Stability threshold for fixed point - φ⁻² */
const STABILITY_THRESHOLD = PHI_INV_2;

/** Maximum tracked attractors - φ × 20 ≈ 32 */
const MAX_ATTRACTORS = Math.round(PHI * 20);

/** Cycle detection window - φ × 10 ≈ 16 */
const CYCLE_WINDOW = Math.round(PHI * 10);

/** State dimensions - φ × 3 ≈ 5 */
const STATE_DIMENSIONS = Math.round(PHI * 3);

// =============================================================================
// ATTRACTOR TYPES
// =============================================================================

const ATTRACTOR_TYPES = {
  fixedPoint: {
    name: 'Fixed Point',
    description: 'Stable state that persists',
    symbol: '●',
    examples: ['Consistent coding style', 'Stable review process'],
  },
  limitCycle: {
    name: 'Limit Cycle',
    description: 'Periodic oscillation between states',
    symbol: '○',
    examples: ['Sprint cycles', 'Test-fix-test loops'],
  },
  strangeAttractor: {
    name: 'Strange Attractor',
    description: 'Chaotic but bounded behavior',
    symbol: '◐',
    examples: ['Creative exploration', 'Debugging complex issues'],
  },
  repeller: {
    name: 'Repeller',
    description: 'Unstable state that pushes away',
    symbol: '✕',
    examples: ['Burnout state', 'Technical debt accumulation'],
  },
};

// =============================================================================
// BEHAVIORAL DIMENSIONS
// =============================================================================

/**
 * Dimensions of behavioral state space
 */
const DIMENSIONS = {
  focus: {
    name: 'Focus',
    range: [0, 100],
    description: 'Concentration vs distraction',
  },
  pace: {
    name: 'Pace',
    range: [0, 100],
    description: 'Speed of iteration',
  },
  depth: {
    name: 'Depth',
    range: [0, 100],
    description: 'Surface vs deep work',
  },
  breadth: {
    name: 'Breadth',
    range: [0, 100],
    description: 'Narrow vs wide exploration',
  },
  risk: {
    name: 'Risk',
    range: [0, 100],
    description: 'Conservative vs experimental',
  },
};

// =============================================================================
// STORAGE
// =============================================================================

const ATTRACTOR_DIR = path.join(os.homedir(), '.cynic', 'attractors');
const STATE_FILE = path.join(ATTRACTOR_DIR, 'state.json');
const ATTRACTORS_FILE = path.join(ATTRACTOR_DIR, 'attractors.json');
const HISTORY_FILE = path.join(ATTRACTOR_DIR, 'history.jsonl');

// =============================================================================
// STATE
// =============================================================================

const attractorState = {
  // State history (trajectory through state space)
  trajectory: [],

  // Detected attractors
  attractors: {},

  // Current state vector
  currentState: null,

  // Basin assignments
  basins: {},

  // Statistics
  stats: {
    statesRecorded: 0,
    attractorsFound: 0,
    cyclesDetected: 0,
    basinTransitions: 0,
    currentBasin: null,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(ATTRACTOR_DIR)) {
    fs.mkdirSync(ATTRACTOR_DIR, { recursive: true });
  }
}

function loadState() {
  ensureDir();
  if (!fs.existsSync(STATE_FILE)) {
    return null;
  }
  try {
    const state = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
    if (fs.existsSync(ATTRACTORS_FILE)) {
      state.attractors = JSON.parse(fs.readFileSync(ATTRACTORS_FILE, 'utf8'));
    }
    return state;
  } catch {
    return null;
  }
}

function saveState() {
  ensureDir();
  fs.writeFileSync(STATE_FILE, JSON.stringify({
    stats: attractorState.stats,
    trajectory: attractorState.trajectory.slice(-100),
    currentState: attractorState.currentState,
    basins: attractorState.basins,
  }, null, 2));
  fs.writeFileSync(ATTRACTORS_FILE, JSON.stringify(attractorState.attractors, null, 2));
}

function appendHistory(event) {
  ensureDir();
  const line = JSON.stringify({ ...event, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(HISTORY_FILE, line);
}

// =============================================================================
// STATE RECORDING
// =============================================================================

/**
 * Record a behavioral state
 *
 * @param {Object} state - State vector {focus, pace, depth, breadth, risk}
 * @returns {Object} Recording result
 */
function recordState(state) {
  // Normalize state
  const normalizedState = {};
  for (const dim of Object.keys(DIMENSIONS)) {
    normalizedState[dim] = Math.max(0, Math.min(100, state[dim] || 50));
  }

  normalizedState.timestamp = Date.now();

  // Add to trajectory
  attractorState.trajectory.push(normalizedState);
  if (attractorState.trajectory.length > 1000) {
    attractorState.trajectory = attractorState.trajectory.slice(-500);
  }

  attractorState.currentState = normalizedState;
  attractorState.stats.statesRecorded++;

  // Check for attractors
  const attractorCheck = checkForAttractors();

  // Assign to basin
  const basin = assignBasin(normalizedState);

  saveState();

  return {
    state: normalizedState,
    attractors: attractorCheck,
    basin,
  };
}

/**
 * Infer state from behavior
 *
 * @param {Object} behavior - Observed behaviors
 * @returns {Object} Inferred state
 */
function inferState(behavior) {
  const state = {
    focus: 50,
    pace: 50,
    depth: 50,
    breadth: 50,
    risk: 50,
  };

  // Focus inference
  if (behavior.singleFile) state.focus += 20;
  if (behavior.multipleFiles) state.focus -= 15;
  if (behavior.longReads) state.focus += 10;

  // Pace inference
  if (behavior.rapidEdits) state.pace += 25;
  if (behavior.longPauses) state.pace -= 20;
  if (behavior.quickCommits) state.pace += 15;

  // Depth inference
  if (behavior.deepStack) state.depth += 25;
  if (behavior.surfaceChanges) state.depth -= 15;
  if (behavior.refactoring) state.depth += 20;

  // Breadth inference
  if (behavior.manyFiles) state.breadth += 25;
  if (behavior.exploration) state.breadth += 20;
  if (behavior.narrowFocus) state.breadth -= 15;

  // Risk inference
  if (behavior.newPatterns) state.risk += 25;
  if (behavior.deletions) state.risk += 15;
  if (behavior.safeEdits) state.risk -= 20;

  return state;
}

// =============================================================================
// ATTRACTOR DETECTION
// =============================================================================

/**
 * Check for attractors in recent trajectory
 *
 * @returns {Object[]} Detected attractors
 */
function checkForAttractors() {
  if (attractorState.trajectory.length < MIN_OBSERVATIONS) {
    return [];
  }

  const recent = attractorState.trajectory.slice(-CYCLE_WINDOW);
  const detected = [];

  // Check for fixed point (stable state)
  const fixedPoint = detectFixedPoint(recent);
  if (fixedPoint) {
    detected.push(fixedPoint);
  }

  // Check for limit cycle (periodic behavior)
  const limitCycle = detectLimitCycle(recent);
  if (limitCycle) {
    detected.push(limitCycle);
  }

  // Check for strange attractor (chaotic but bounded)
  const strange = detectStrangeAttractor(recent);
  if (strange) {
    detected.push(strange);
  }

  return detected;
}

/**
 * Detect fixed point attractor
 */
function detectFixedPoint(states) {
  if (states.length < MIN_OBSERVATIONS) return null;

  // Calculate variance in each dimension
  const variances = {};
  for (const dim of Object.keys(DIMENSIONS)) {
    const values = states.map(s => s[dim]);
    const mean = values.reduce((a, b) => a + b, 0) / values.length;
    const variance = values.reduce((sum, v) => sum + Math.pow(v - mean, 2), 0) / values.length;
    variances[dim] = variance;
  }

  // Check if all variances are below threshold
  const totalVariance = Object.values(variances).reduce((a, b) => a + b, 0);
  const maxVariance = Object.keys(DIMENSIONS).length * 100 * 100; // Max possible variance

  const stability = 1 - (totalVariance / maxVariance);

  if (stability >= STABILITY_THRESHOLD) {
    // Calculate centroid
    const centroid = {};
    for (const dim of Object.keys(DIMENSIONS)) {
      centroid[dim] = Math.round(
        states.map(s => s[dim]).reduce((a, b) => a + b, 0) / states.length
      );
    }

    return createAttractor('fixedPoint', centroid, stability);
  }

  return null;
}

/**
 * Detect limit cycle attractor
 */
function detectLimitCycle(states) {
  if (states.length < MIN_OBSERVATIONS * 2) return null;

  // Look for periodicity using autocorrelation
  const periods = [2, 3, 4, 5, 6, 7, 8]; // Candidate periods
  let bestPeriod = null;
  let bestCorrelation = 0;

  for (const period of periods) {
    if (states.length < period * 2) continue;

    let correlation = 0;
    let count = 0;

    for (let i = period; i < states.length; i++) {
      const dist = stateDistance(states[i], states[i - period]);
      correlation += 1 / (1 + dist);
      count++;
    }

    correlation /= count;

    if (correlation > bestCorrelation && correlation > PHI_INV) {
      bestCorrelation = correlation;
      bestPeriod = period;
    }
  }

  if (bestPeriod) {
    // Extract cycle states
    const cycleStates = states.slice(-bestPeriod);
    attractorState.stats.cyclesDetected++;

    return createAttractor('limitCycle', cycleStates[0], bestCorrelation, {
      period: bestPeriod,
      cycleStates,
    });
  }

  return null;
}

/**
 * Detect strange attractor
 */
function detectStrangeAttractor(states) {
  if (states.length < MIN_OBSERVATIONS) return null;

  // Strange attractor: bounded but not periodic
  // High dimensionality in trajectory

  // Calculate bounds
  const bounds = {};
  for (const dim of Object.keys(DIMENSIONS)) {
    const values = states.map(s => s[dim]);
    bounds[dim] = {
      min: Math.min(...values),
      max: Math.max(...values),
      range: Math.max(...values) - Math.min(...values),
    };
  }

  // Check if bounded (range < φ × 50 in each dimension)
  const isBounded = Object.values(bounds).every(b => b.range < PHI * 50);

  // Check if complex (not simple fixed point or cycle)
  const hasFixedPoint = detectFixedPoint(states);
  const hasLimitCycle = detectLimitCycle(states);

  if (isBounded && !hasFixedPoint && !hasLimitCycle) {
    // Calculate centroid for strange attractor
    const centroid = {};
    for (const dim of Object.keys(DIMENSIONS)) {
      centroid[dim] = Math.round(
        (bounds[dim].min + bounds[dim].max) / 2
      );
    }

    const boundedness = Object.values(bounds).reduce((sum, b) =>
      sum + (1 - b.range / 100), 0) / Object.keys(DIMENSIONS).length;

    return createAttractor('strangeAttractor', centroid, boundedness, { bounds });
  }

  return null;
}

/**
 * Calculate distance between two states
 */
function stateDistance(s1, s2) {
  let sum = 0;
  for (const dim of Object.keys(DIMENSIONS)) {
    sum += Math.pow((s1[dim] || 50) - (s2[dim] || 50), 2);
  }
  return Math.sqrt(sum);
}

/**
 * Create an attractor record
 */
function createAttractor(type, center, strength, extra = {}) {
  const id = `att-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

  // Check if similar attractor exists
  for (const existing of Object.values(attractorState.attractors)) {
    if (existing.type === type) {
      const dist = stateDistance(existing.center, center);
      if (dist < 20) {
        // Update existing
        existing.strength = (existing.strength + strength) / 2;
        existing.observations++;
        existing.lastSeen = Date.now();
        return existing;
      }
    }
  }

  // Check capacity
  if (Object.keys(attractorState.attractors).length >= MAX_ATTRACTORS) {
    pruneWeakAttractors();
  }

  const attractor = {
    id,
    type,
    typeName: ATTRACTOR_TYPES[type].name,
    symbol: ATTRACTOR_TYPES[type].symbol,
    center,
    strength: Math.round(strength * 100) / 100,
    observations: 1,
    detectedAt: Date.now(),
    lastSeen: Date.now(),
    ...extra,
  };

  attractorState.attractors[id] = attractor;
  attractorState.stats.attractorsFound++;

  appendHistory({
    type: 'attractor_detected',
    attractorId: id,
    attractorType: type,
    strength,
  });

  saveState();

  return attractor;
}

/**
 * Prune weakest attractors
 */
function pruneWeakAttractors() {
  const sorted = Object.entries(attractorState.attractors)
    .sort((a, b) => a[1].strength - b[1].strength);

  const toRemove = sorted.slice(0, Math.round(MAX_ATTRACTORS * PHI_INV_3));
  for (const [id] of toRemove) {
    delete attractorState.attractors[id];
  }
}

// =============================================================================
// BASIN ASSIGNMENT
// =============================================================================

/**
 * Assign current state to a basin of attraction
 *
 * @param {Object} state - Current state
 * @returns {Object} Basin info
 */
function assignBasin(state) {
  let nearestAttractor = null;
  let minDistance = Infinity;

  for (const attractor of Object.values(attractorState.attractors)) {
    const dist = stateDistance(state, attractor.center);
    if (dist < minDistance) {
      minDistance = dist;
      nearestAttractor = attractor;
    }
  }

  if (!nearestAttractor) {
    return { basin: null, distance: null };
  }

  // Check for basin transition
  const previousBasin = attractorState.stats.currentBasin;
  if (previousBasin && previousBasin !== nearestAttractor.id) {
    attractorState.stats.basinTransitions++;
    appendHistory({
      type: 'basin_transition',
      from: previousBasin,
      to: nearestAttractor.id,
    });
  }

  attractorState.stats.currentBasin = nearestAttractor.id;

  return {
    basin: nearestAttractor.id,
    attractorName: nearestAttractor.typeName,
    distance: Math.round(minDistance),
    isNearAttractor: minDistance < 30,
  };
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize attractor mapping
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    attractorState.stats = saved.stats || attractorState.stats;
    attractorState.trajectory = saved.trajectory || [];
    attractorState.currentState = saved.currentState || null;
    attractorState.basins = saved.basins || {};
    attractorState.attractors = saved.attractors || {};
  }
}

/**
 * Get all attractors
 *
 * @returns {Object[]} Attractors
 */
function getAttractors() {
  return Object.values(attractorState.attractors);
}

/**
 * Get a specific attractor
 *
 * @param {string} id - Attractor ID
 * @returns {Object|null} Attractor
 */
function getAttractor(id) {
  return attractorState.attractors[id] || null;
}

/**
 * Get current state
 *
 * @returns {Object|null} Current state
 */
function getCurrentState() {
  return attractorState.currentState;
}

/**
 * Get trajectory
 *
 * @param {number} limit - Max states to return
 * @returns {Object[]} Trajectory
 */
function getTrajectory(limit = 50) {
  return attractorState.trajectory.slice(-limit);
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...attractorState.stats,
    attractorCount: Object.keys(attractorState.attractors).length,
    trajectoryLength: attractorState.trajectory.length,
  };
}

/**
 * Format status for display
 *
 * @returns {string} Formatted status
 */
function formatStatus() {
  const stats = getStats();
  const attractors = getAttractors();
  const current = attractorState.currentState;

  const lines = [
    '── ATTRACTOR MAPPING ──────────────────────────────────────',
    `   States Recorded: ${stats.statesRecorded}`,
    `   Attractors Found: ${stats.attractorCount}`,
    `   Cycles Detected: ${stats.cyclesDetected}`,
    `   Basin Transitions: ${stats.basinTransitions}`,
  ];

  if (current) {
    lines.push('');
    lines.push('   Current State:');
    for (const [dim, config] of Object.entries(DIMENSIONS)) {
      const val = current[dim] || 50;
      const bar = '█'.repeat(Math.round(val / 10)) + '░'.repeat(10 - Math.round(val / 10));
      lines.push(`   ${dim.padEnd(8)}: [${bar}] ${val}`);
    }
  }

  if (attractors.length > 0) {
    lines.push('');
    lines.push('   Known Attractors:');
    for (const att of attractors.slice(0, 5)) {
      lines.push(`   ${att.symbol} ${att.typeName} (strength: ${Math.round(att.strength * 100)}%)`);
    }
  }

  if (stats.currentBasin) {
    const currentAtt = attractorState.attractors[stats.currentBasin];
    if (currentAtt) {
      lines.push('');
      lines.push(`   Current Basin: ${currentAtt.typeName}`);
    }
  }

  lines.push('');
  lines.push('   *sniff* Systems settle into attractors.');

  return lines.join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  MIN_OBSERVATIONS,
  STABILITY_THRESHOLD,
  ATTRACTOR_TYPES,
  DIMENSIONS,

  // Core functions
  init,
  recordState,
  inferState,

  // Attractor access
  getAttractors,
  getAttractor,
  getCurrentState,
  getTrajectory,

  // Basin assignment
  assignBasin,

  // Stats and display
  getStats,
  formatStatus,
};

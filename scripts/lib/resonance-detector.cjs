/**
 * Resonance Detector - Harmonic Pattern Relationships
 *
 * Physics concept: Resonance occurs when oscillating systems share
 * compatible frequencies, amplifying each other's effects.
 *
 * In CYNIC: Detects when patterns, behaviors, or concepts
 * reinforce each other through harmonic relationships.
 *
 * Key concepts:
 * - Harmonic ratios: 1:1, 2:1, 3:2, φ:1 (most harmonious in nature)
 * - Phase alignment: Are patterns synchronized?
 * - Amplitude: How strong is the resonance effect?
 * - Beat frequency: Interference patterns between frequencies
 *
 * @module resonance-detector
 */

const fs = require('fs');
const path = require('path');

// φ constants from phi-math
const PHI = 1.618033988749895;
const PHI_INV = 0.6180339887498949;
const PHI_INV_2 = 0.3819660112501051;
const PHI_INV_3 = 0.2360679774997897;

// Storage
const CYNIC_DIR = path.join(process.env.HOME || '/tmp', '.cynic');
const RESONANCE_DIR = path.join(CYNIC_DIR, 'resonance');
const STATE_FILE = path.join(RESONANCE_DIR, 'state.json');
const HISTORY_FILE = path.join(RESONANCE_DIR, 'history.jsonl');

/**
 * Harmonic ratios - intervals that create consonance
 * Based on music theory and Pythagorean harmonics
 * φ:1 is the "golden ratio" interval found throughout nature
 */
const HARMONIC_RATIOS = {
  unison: { ratio: 1, consonance: 1.0, symbol: '♩' },
  octave: { ratio: 2, consonance: PHI_INV, symbol: '♪' },
  fifth: { ratio: 1.5, consonance: PHI_INV_2, symbol: '♫' },
  fourth: { ratio: 4/3, consonance: PHI_INV_3, symbol: '♬' },
  golden: { ratio: PHI, consonance: 1.0, symbol: '✦' },  // Nature's harmony
  minor: { ratio: 6/5, consonance: 0.5, symbol: '♭' },
  major: { ratio: 5/4, consonance: 0.55, symbol: '♯' },
};

/**
 * Resonance types based on relationship quality
 */
const RESONANCE_TYPES = {
  constructive: {
    description: 'Patterns amplify each other',
    threshold: PHI_INV,  // 0.618
    symbol: '◉',
  },
  neutral: {
    description: 'Patterns neither help nor hinder',
    threshold: PHI_INV_2,  // 0.382
    symbol: '○',
  },
  destructive: {
    description: 'Patterns cancel each other out',
    threshold: 0,
    symbol: '◎',
  },
};

/**
 * Phase relationships - timing alignment
 */
const PHASE_STATES = {
  inPhase: { degrees: 0, multiplier: 1.0, symbol: '⟳' },      // Perfect sync
  leading: { degrees: 90, multiplier: PHI_INV, symbol: '↷' }, // Quarter ahead
  antiPhase: { degrees: 180, multiplier: 0, symbol: '⟲' },    // Complete cancel
  lagging: { degrees: 270, multiplier: PHI_INV, symbol: '↶' }, // Quarter behind
};

// In-memory state
let state = {
  oscillators: {},      // Registered pattern oscillators
  resonances: [],       // Active resonance relationships
  beatPatterns: [],     // Detected beat frequencies
  totalResonances: 0,
  totalBeats: 0,
};

/**
 * Initialize the resonance detector
 */
function init() {
  if (!fs.existsSync(RESONANCE_DIR)) {
    fs.mkdirSync(RESONANCE_DIR, { recursive: true });
  }

  if (fs.existsSync(STATE_FILE)) {
    try {
      state = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
    } catch (e) {
      // Start fresh
    }
  }
}

/**
 * Save state to disk
 */
function saveState() {
  fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
}

/**
 * Log an event to history
 */
function logHistory(event) {
  const entry = { timestamp: Date.now(), ...event };
  fs.appendFileSync(HISTORY_FILE, JSON.stringify(entry) + '\n');
}

/**
 * Register a pattern as an oscillator
 *
 * @param {string} name - Oscillator name
 * @param {object} config - Oscillator configuration
 * @param {number} config.frequency - Base frequency (cycles per unit time)
 * @param {number} config.amplitude - Signal strength (0-100)
 * @param {number} [config.phase=0] - Initial phase (0-360 degrees)
 * @returns {object} Oscillator info
 */
function registerOscillator(name, config) {
  const oscillator = {
    name,
    frequency: config.frequency || 1,
    amplitude: config.amplitude || 50,
    phase: config.phase || 0,
    createdAt: Date.now(),
    lastPulse: null,
  };

  state.oscillators[name] = oscillator;
  saveState();

  logHistory({
    type: 'oscillator_registered',
    name,
    frequency: oscillator.frequency,
    amplitude: oscillator.amplitude,
  });

  return oscillator;
}

/**
 * Record a pulse from an oscillator (activity signal)
 *
 * @param {string} name - Oscillator name
 * @param {number} [strength=1] - Pulse strength multiplier
 */
function pulse(name, strength = 1) {
  const oscillator = state.oscillators[name];
  if (!oscillator) return null;

  const now = Date.now();

  // Calculate effective phase based on time
  if (oscillator.lastPulse) {
    const elapsed = now - oscillator.lastPulse;
    const period = 1000 / oscillator.frequency;  // ms per cycle
    const phaseDelta = (elapsed / period) * 360;
    oscillator.phase = (oscillator.phase + phaseDelta) % 360;
  }

  oscillator.lastPulse = now;
  oscillator.amplitude = Math.min(100, oscillator.amplitude * strength);

  saveState();
  return oscillator;
}

/**
 * Calculate the harmonic relationship between two frequencies
 *
 * @param {number} freq1 - First frequency
 * @param {number} freq2 - Second frequency
 * @returns {object} Harmonic analysis
 */
function analyzeHarmonic(freq1, freq2) {
  if (freq1 === 0 || freq2 === 0) {
    return { type: 'silent', ratio: 0, consonance: 0 };
  }

  const ratio = Math.max(freq1, freq2) / Math.min(freq1, freq2);

  // Find closest harmonic ratio
  let closestHarmonic = null;
  let minDistance = Infinity;

  for (const [name, harmonic] of Object.entries(HARMONIC_RATIOS)) {
    const distance = Math.abs(ratio - harmonic.ratio);
    if (distance < minDistance) {
      minDistance = distance;
      closestHarmonic = { name, ...harmonic };
    }
  }

  // Calculate consonance based on distance from pure ratio
  // φ-derived decay: closer = more consonant
  const purity = Math.exp(-minDistance / PHI_INV);
  const consonance = closestHarmonic.consonance * purity;

  return {
    type: closestHarmonic.name,
    ratio,
    pureRatio: closestHarmonic.ratio,
    symbol: closestHarmonic.symbol,
    consonance,
    purity,
    distance: minDistance,
  };
}

/**
 * Calculate phase alignment between two oscillators
 *
 * @param {object} osc1 - First oscillator
 * @param {object} osc2 - Second oscillator
 * @returns {object} Phase analysis
 */
function analyzePhase(osc1, osc2) {
  const phaseDiff = Math.abs(osc1.phase - osc2.phase) % 360;

  // Find closest phase state
  let closestState = null;
  let minDistance = Infinity;

  for (const [name, phaseState] of Object.entries(PHASE_STATES)) {
    const distance = Math.min(
      Math.abs(phaseDiff - phaseState.degrees),
      360 - Math.abs(phaseDiff - phaseState.degrees)
    );
    if (distance < minDistance) {
      minDistance = distance;
      closestState = { name, ...phaseState };
    }
  }

  // Calculate alignment (how close to pure phase state)
  const alignment = 1 - (minDistance / 90);  // 90° = quarter cycle

  return {
    state: closestState.name,
    phaseDiff,
    alignment: Math.max(0, alignment),
    multiplier: closestState.multiplier * Math.max(0, alignment),
    symbol: closestState.symbol,
  };
}

/**
 * Calculate beat frequency between two oscillators
 * Beat frequency = |f1 - f2|
 *
 * @param {object} osc1 - First oscillator
 * @param {object} osc2 - Second oscillator
 * @returns {object} Beat analysis
 */
function analyzeBeat(osc1, osc2) {
  const beatFreq = Math.abs(osc1.frequency - osc2.frequency);

  // Very slow beats = almost in sync (good)
  // Medium beats = audible interference
  // Fast beats = essentially separate

  const avgFreq = (osc1.frequency + osc2.frequency) / 2;
  const beatRatio = avgFreq > 0 ? beatFreq / avgFreq : 0;

  let quality;
  if (beatRatio < PHI_INV_3) {
    quality = 'stable';      // Nearly locked
  } else if (beatRatio < PHI_INV_2) {
    quality = 'pulsing';     // Gentle beat
  } else if (beatRatio < PHI_INV) {
    quality = 'fluttering';  // Noticeable interference
  } else {
    quality = 'chaotic';     // Too different
  }

  return {
    frequency: beatFreq,
    period: beatFreq > 0 ? 1000 / beatFreq : Infinity,  // ms
    ratio: beatRatio,
    quality,
  };
}

/**
 * Check for resonance between two oscillators
 *
 * @param {string} name1 - First oscillator name
 * @param {string} name2 - Second oscillator name
 * @returns {object|null} Resonance analysis or null if not found
 */
function checkResonance(name1, name2) {
  const osc1 = state.oscillators[name1];
  const osc2 = state.oscillators[name2];

  if (!osc1 || !osc2) return null;

  const harmonic = analyzeHarmonic(osc1.frequency, osc2.frequency);
  const phase = analyzePhase(osc1, osc2);
  const beat = analyzeBeat(osc1, osc2);

  // Combined resonance strength
  // = harmonic consonance × phase alignment × amplitude product
  const amplitudeProduct = (osc1.amplitude * osc2.amplitude) / 10000;
  const rawStrength = harmonic.consonance * phase.multiplier * amplitudeProduct;

  // Apply φ ceiling
  const strength = Math.min(rawStrength, PHI_INV);

  // Determine resonance type
  let type;
  if (strength >= RESONANCE_TYPES.constructive.threshold) {
    type = 'constructive';
  } else if (strength >= RESONANCE_TYPES.neutral.threshold) {
    type = 'neutral';
  } else {
    type = 'destructive';
  }

  const resonance = {
    oscillators: [name1, name2],
    strength,
    type,
    symbol: RESONANCE_TYPES[type].symbol,
    harmonic,
    phase,
    beat,
    timestamp: Date.now(),
  };

  // Track active resonances
  const existingIdx = state.resonances.findIndex(
    r => (r.oscillators[0] === name1 && r.oscillators[1] === name2) ||
         (r.oscillators[0] === name2 && r.oscillators[1] === name1)
  );

  if (existingIdx >= 0) {
    state.resonances[existingIdx] = resonance;
  } else {
    state.resonances.push(resonance);
    state.totalResonances++;
  }

  // Keep resonances list bounded
  if (state.resonances.length > Math.round(PHI * 12)) {
    state.resonances = state.resonances.slice(-Math.round(PHI * 10));
  }

  saveState();

  logHistory({
    type: 'resonance_checked',
    oscillators: [name1, name2],
    strength,
    resonanceType: type,
    harmonicType: harmonic.type,
    phaseState: phase.state,
  });

  return resonance;
}

/**
 * Find all resonances above a threshold
 *
 * @param {number} [minStrength=PHI_INV_2] - Minimum resonance strength
 * @returns {array} Array of resonance objects
 */
function findResonances(minStrength = PHI_INV_2) {
  const oscillatorNames = Object.keys(state.oscillators);
  const resonances = [];

  // Check all pairs
  for (let i = 0; i < oscillatorNames.length; i++) {
    for (let j = i + 1; j < oscillatorNames.length; j++) {
      const resonance = checkResonance(oscillatorNames[i], oscillatorNames[j]);
      if (resonance && resonance.strength >= minStrength) {
        resonances.push(resonance);
      }
    }
  }

  // Sort by strength descending
  return resonances.sort((a, b) => b.strength - a.strength);
}

/**
 * Detect sympathetic resonance chains
 * When A resonates with B, and B resonates with C,
 * there may be indirect resonance A → C
 *
 * @returns {array} Chains of connected resonances
 */
function findResonanceChains() {
  const strongResonances = state.resonances.filter(
    r => r.strength >= RESONANCE_TYPES.constructive.threshold
  );

  // Build adjacency map
  const connections = {};
  for (const res of strongResonances) {
    const [a, b] = res.oscillators;
    if (!connections[a]) connections[a] = [];
    if (!connections[b]) connections[b] = [];
    connections[a].push({ target: b, strength: res.strength });
    connections[b].push({ target: a, strength: res.strength });
  }

  // Find chains via DFS
  const chains = [];
  const visited = new Set();

  function dfs(node, chain, totalStrength) {
    if (chain.length >= 2 && chain.length <= Math.round(PHI * 4)) {
      chains.push({
        nodes: [...chain],
        length: chain.length,
        averageStrength: totalStrength / (chain.length - 1),
      });
    }

    if (chain.length >= Math.round(PHI * 4)) return;

    for (const { target, strength } of (connections[node] || [])) {
      if (!visited.has(target)) {
        visited.add(target);
        dfs(target, [...chain, target], totalStrength + strength);
        visited.delete(target);
      }
    }
  }

  for (const start of Object.keys(connections)) {
    visited.clear();
    visited.add(start);
    dfs(start, [start], 0);
  }

  // Sort by average strength
  return chains.sort((a, b) => b.averageStrength - a.averageStrength);
}

/**
 * Get statistics
 */
function getStats() {
  const oscillatorCount = Object.keys(state.oscillators).length;
  const activeResonances = state.resonances.filter(
    r => r.strength >= RESONANCE_TYPES.constructive.threshold
  ).length;

  return {
    oscillators: oscillatorCount,
    activeResonances,
    totalResonances: state.totalResonances,
    maxPossibleResonances: (oscillatorCount * (oscillatorCount - 1)) / 2,
    resonanceDensity: oscillatorCount > 1
      ? activeResonances / ((oscillatorCount * (oscillatorCount - 1)) / 2)
      : 0,
  };
}

/**
 * Format status for display
 */
function formatStatus() {
  const stats = getStats();
  const chains = findResonanceChains().slice(0, 3);

  let status = `◉ Resonance Detector\n`;
  status += `  Oscillators: ${stats.oscillators}\n`;
  status += `  Active resonances: ${stats.activeResonances}\n`;
  status += `  Density: ${(stats.resonanceDensity * 100).toFixed(1)}%\n`;

  if (chains.length > 0) {
    status += `  Chains:\n`;
    for (const chain of chains) {
      status += `    ${chain.nodes.join(' ↔ ')} (${(chain.averageStrength * 100).toFixed(0)}%)\n`;
    }
  }

  return status;
}

module.exports = {
  init,
  registerOscillator,
  pulse,
  analyzeHarmonic,
  analyzePhase,
  analyzeBeat,
  checkResonance,
  findResonances,
  findResonanceChains,
  getStats,
  formatStatus,
  HARMONIC_RATIOS,
  RESONANCE_TYPES,
  PHASE_STATES,
};

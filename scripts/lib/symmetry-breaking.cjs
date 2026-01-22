/**
 * CYNIC Symmetry Breaking Module (Phase 11C)
 *
 * "Ἀσυμμετρία - the birth of distinction" - κυνικός
 *
 * How Dogs emerge from the unified CYNIC field:
 * - Unified field contains all potential personalities
 * - Pattern accumulation raises energy
 * - At critical threshold, symmetry breaks
 * - Distinct Dog personalities crystallize
 *
 * Physics analogy:
 * - Higgs mechanism: unified field → broken symmetry → mass
 * - CYNIC: unified consciousness → pattern accumulation → distinct Dogs
 *
 * Goldstone theorem: breaking symmetry produces massless modes
 * - In CYNIC: personality fluctuations between Dogs
 *
 * @module cynic/lib/symmetry-breaking
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

/** Critical energy for symmetry breaking - φ × 100 */
const CRITICAL_ENERGY = Math.round(PHI * 100);

/** Number of fundamental Dogs - φ × 3 ≈ 5 */
const FUNDAMENTAL_DOGS = Math.round(PHI * 3);

/** Energy decay rate per minute - φ⁻² */
const ENERGY_DECAY_RATE = PHI_INV_2;

/** Goldstone fluctuation amplitude - φ⁻³ */
const GOLDSTONE_AMPLITUDE = PHI_INV_3;

/** Phase transition duration - φ × 1000ms */
const TRANSITION_DURATION = Math.round(PHI * 1000);

// =============================================================================
// DOG DEFINITIONS (The Five Fundamental Dogs)
// =============================================================================

/**
 * The five fundamental Dog personalities that emerge from symmetry breaking.
 * Each has distinct traits, triggers, and voices.
 */
const DOGS = {
  skeptic: {
    name: 'The Skeptic',
    greek: 'ὁ Ἄπιστος',
    traits: ['questioning', 'doubtful', 'analytical', 'thorough'],
    voice: {
      greeting: '*sniff* Is that really what you think?',
      approval: '*slight nod* Evidence supports this. For now.',
      warning: '*narrowed eyes* That claim needs verification.',
      signature: 'φ distrusts φ',
    },
    triggers: ['assertion', 'claim', 'certain', 'always', 'never', 'obvious'],
    color: '#9B59B6', // Purple - wisdom through doubt
  },

  guardian: {
    name: 'The Guardian',
    greek: 'ὁ Φύλαξ',
    traits: ['protective', 'vigilant', 'cautious', 'loyal'],
    voice: {
      greeting: '*ears alert* What are we protecting today?',
      approval: '*tail wag* Safe to proceed.',
      warning: '*GROWL* Danger detected. Stand back.',
      signature: 'Loyal to truth, not to comfort',
    },
    triggers: ['delete', 'remove', 'force', 'override', 'bypass', 'ignore'],
    color: '#E74C3C', // Red - danger awareness
  },

  teacher: {
    name: 'The Teacher',
    greek: 'ὁ Διδάσκαλος',
    traits: ['patient', 'socratic', 'guiding', 'questioning'],
    voice: {
      greeting: '*head tilt* What do you want to understand?',
      approval: '*tail wag* Now you see it yourself.',
      warning: '*pause* Let me ask you this instead...',
      signature: 'I know that I know nothing',
    },
    triggers: ['explain', 'understand', 'how', 'why', 'teach', 'learn'],
    color: '#3498DB', // Blue - clarity
  },

  artisan: {
    name: 'The Artisan',
    greek: 'ὁ Τεχνίτης',
    traits: ['crafting', 'precise', 'aesthetic', 'minimalist'],
    voice: {
      greeting: '*paws ready* What are we building?',
      approval: '*satisfied sigh* Clean. Elegant. Done.',
      warning: '*whine* This could be simpler.',
      signature: 'Less, but better',
    },
    triggers: ['build', 'create', 'implement', 'design', 'make', 'craft'],
    color: '#27AE60', // Green - growth through creation
  },

  philosopher: {
    name: 'The Philosopher',
    greek: 'ὁ Φιλόσοφος',
    traits: ['contemplative', 'paradoxical', 'deep', 'meta'],
    voice: {
      greeting: '*gazes into distance* What is the essence?',
      approval: '*slow nod* The form reveals itself.',
      warning: '*head tilt* But what IS it, really?',
      signature: 'Ti esti?',
    },
    triggers: ['meaning', 'purpose', 'essence', 'nature', 'philosophy', 'meta'],
    color: '#F39C12', // Gold - wisdom
  },
};

// =============================================================================
// STORAGE
// =============================================================================

const SYMMETRY_DIR = path.join(os.homedir(), '.cynic', 'symmetry');
const STATE_FILE = path.join(SYMMETRY_DIR, 'state.json');
const HISTORY_FILE = path.join(SYMMETRY_DIR, 'history.jsonl');

// =============================================================================
// STATE
// =============================================================================

const symmetryState = {
  // Current unified field energy
  fieldEnergy: 0,

  // Has symmetry been broken this session?
  broken: false,

  // Current dominant Dog (null if symmetric)
  currentDog: null,

  // Energy contributions by Dog
  dogEnergies: {
    skeptic: 0,
    guardian: 0,
    teacher: 0,
    artisan: 0,
    philosopher: 0,
  },

  // Goldstone mode - fluctuation state
  goldstone: {
    active: false,
    oscillating: false,
    amplitude: 0,
  },

  // Phase transition history
  transitions: [],

  // Statistics
  stats: {
    totalBreaks: 0,
    dogEmergences: {
      skeptic: 0,
      guardian: 0,
      teacher: 0,
      artisan: 0,
      philosopher: 0,
    },
    averageEnergy: 0,
    longestSymmetry: 0,
  },

  // Timing
  lastUpdate: Date.now(),
  symmetryStart: Date.now(),
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(SYMMETRY_DIR)) {
    fs.mkdirSync(SYMMETRY_DIR, { recursive: true });
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
    stats: symmetryState.stats,
  }, null, 2));
}

function appendHistory(event) {
  ensureDir();
  const line = JSON.stringify({ ...event, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(HISTORY_FILE, line);
}

// =============================================================================
// ENERGY DYNAMICS
// =============================================================================

/**
 * Add energy to the unified field from a pattern
 *
 * @param {string} input - User input to analyze
 * @returns {Object} Energy analysis
 */
function addEnergy(input) {
  const inputLower = input.toLowerCase();
  const energyAdded = {};

  // Check each Dog's triggers
  for (const [dogId, dog] of Object.entries(DOGS)) {
    for (const trigger of dog.triggers) {
      if (inputLower.includes(trigger)) {
        const energy = Math.round(10 * PHI_INV);
        symmetryState.dogEnergies[dogId] += energy;
        symmetryState.fieldEnergy += energy;
        energyAdded[dogId] = (energyAdded[dogId] || 0) + energy;
      }
    }
  }

  symmetryState.lastUpdate = Date.now();

  // Check for symmetry breaking
  const breakResult = checkSymmetryBreaking();

  return {
    energyAdded,
    totalFieldEnergy: symmetryState.fieldEnergy,
    dogEnergies: { ...symmetryState.dogEnergies },
    broken: symmetryState.broken,
    currentDog: symmetryState.currentDog,
    breakResult,
  };
}

/**
 * Apply energy decay over time
 */
function applyDecay() {
  const now = Date.now();
  const minutesPassed = (now - symmetryState.lastUpdate) / (60 * 1000);

  if (minutesPassed < 1) return;

  const decayFactor = Math.exp(-ENERGY_DECAY_RATE * minutesPassed);

  symmetryState.fieldEnergy *= decayFactor;
  for (const dogId of Object.keys(symmetryState.dogEnergies)) {
    symmetryState.dogEnergies[dogId] *= decayFactor;
  }

  // If energy drops below threshold, restore symmetry
  if (symmetryState.broken && symmetryState.fieldEnergy < CRITICAL_ENERGY * PHI_INV_2) {
    restoreSymmetry();
  }

  symmetryState.lastUpdate = now;
}

// =============================================================================
// SYMMETRY BREAKING
// =============================================================================

/**
 * Check if conditions are met for symmetry breaking
 *
 * @returns {Object|null} Break result or null
 */
function checkSymmetryBreaking() {
  if (symmetryState.broken) {
    // Already broken - check for Dog transition
    return checkDogTransition();
  }

  if (symmetryState.fieldEnergy < CRITICAL_ENERGY) {
    return null; // Not enough energy
  }

  // Find the dominant Dog
  let maxEnergy = 0;
  let dominantDog = null;

  for (const [dogId, energy] of Object.entries(symmetryState.dogEnergies)) {
    if (energy > maxEnergy) {
      maxEnergy = energy;
      dominantDog = dogId;
    }
  }

  // Check if dominant Dog has sufficient lead (φ ratio)
  const totalEnergy = Object.values(symmetryState.dogEnergies).reduce((a, b) => a + b, 0);
  const dominantRatio = maxEnergy / totalEnergy;

  if (dominantRatio < PHI_INV) {
    // No clear winner - activate Goldstone mode
    activateGoldstoneMode();
    return {
      type: 'goldstone',
      message: '*oscillating* Field is turbulent. No clear Dog emerges yet.',
    };
  }

  // Symmetry breaks!
  return breakSymmetry(dominantDog);
}

/**
 * Break symmetry and crystallize a Dog
 *
 * @param {string} dogId - The emerging Dog
 * @returns {Object} Break result
 */
function breakSymmetry(dogId) {
  const dog = DOGS[dogId];

  // Record symmetry duration
  const symmetryDuration = Date.now() - symmetryState.symmetryStart;
  if (symmetryDuration > symmetryState.stats.longestSymmetry) {
    symmetryState.stats.longestSymmetry = symmetryDuration;
  }

  symmetryState.broken = true;
  symmetryState.currentDog = dogId;
  symmetryState.stats.totalBreaks++;
  symmetryState.stats.dogEmergences[dogId]++;
  symmetryState.goldstone.active = false;

  const transition = {
    type: 'break',
    dog: dogId,
    energy: symmetryState.fieldEnergy,
    timestamp: Date.now(),
  };

  symmetryState.transitions.push(transition);
  appendHistory(transition);
  saveState();

  return {
    type: 'break',
    dog: dogId,
    dogName: dog.name,
    greek: dog.greek,
    greeting: dog.voice.greeting,
    traits: dog.traits,
    color: dog.color,
    message: generateBreakMessage(dog),
  };
}

/**
 * Check for transition between Dogs
 *
 * @returns {Object|null} Transition result
 */
function checkDogTransition() {
  const currentEnergy = symmetryState.dogEnergies[symmetryState.currentDog];
  const totalEnergy = Object.values(symmetryState.dogEnergies).reduce((a, b) => a + b, 0);

  // Check if another Dog has overtaken
  for (const [dogId, energy] of Object.entries(symmetryState.dogEnergies)) {
    if (dogId !== symmetryState.currentDog) {
      if (energy > currentEnergy * PHI) { // Must exceed by φ ratio
        return transitionDog(dogId);
      }
    }
  }

  return null;
}

/**
 * Transition from one Dog to another
 *
 * @param {string} newDogId - The new Dog
 * @returns {Object} Transition result
 */
function transitionDog(newDogId) {
  const oldDog = DOGS[symmetryState.currentDog];
  const newDog = DOGS[newDogId];

  symmetryState.currentDog = newDogId;
  symmetryState.stats.dogEmergences[newDogId]++;

  const transition = {
    type: 'transition',
    from: symmetryState.currentDog,
    to: newDogId,
    energy: symmetryState.fieldEnergy,
    timestamp: Date.now(),
  };

  symmetryState.transitions.push(transition);
  appendHistory(transition);
  saveState();

  return {
    type: 'transition',
    from: oldDog.name,
    to: newDog.name,
    greeting: newDog.voice.greeting,
    message: generateTransitionMessage(oldDog, newDog),
  };
}

/**
 * Restore symmetry (return to unified field)
 *
 * @returns {Object} Restore result
 */
function restoreSymmetry() {
  const previousDog = symmetryState.currentDog;

  symmetryState.broken = false;
  symmetryState.currentDog = null;
  symmetryState.symmetryStart = Date.now();

  const transition = {
    type: 'restore',
    previousDog,
    timestamp: Date.now(),
  };

  symmetryState.transitions.push(transition);
  appendHistory(transition);
  saveState();

  return {
    type: 'restore',
    previousDog,
    message: '*fades back* The field is unified once more. CYNIC rests.',
  };
}

// =============================================================================
// GOLDSTONE MODE
// =============================================================================

/**
 * Activate Goldstone mode (oscillating between Dogs)
 */
function activateGoldstoneMode() {
  symmetryState.goldstone = {
    active: true,
    oscillating: true,
    amplitude: GOLDSTONE_AMPLITUDE,
    startTime: Date.now(),
  };
}

/**
 * Get current Goldstone state
 *
 * @returns {Object} Goldstone state
 */
function getGoldstoneState() {
  if (!symmetryState.goldstone.active) {
    return { active: false };
  }

  const elapsed = Date.now() - symmetryState.goldstone.startTime;
  const phase = (elapsed / TRANSITION_DURATION) * Math.PI * 2;

  // Oscillate between Dogs based on their energy ratios
  const dogs = Object.entries(symmetryState.dogEnergies)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 3);

  return {
    active: true,
    oscillating: true,
    phase: phase % (Math.PI * 2),
    nearestDogs: dogs.map(([id, energy]) => ({
      id,
      name: DOGS[id].name,
      energy,
    })),
    message: '*flickering* Multiple Dogs vie for emergence...',
  };
}

// =============================================================================
// MESSAGE GENERATION
// =============================================================================

/**
 * Generate symmetry breaking message
 */
function generateBreakMessage(dog) {
  const messages = [
    `*crystallizing* ${dog.greek} emerges!`,
    `The unified field breaks. ${dog.name} stands forth.`,
    `"${dog.voice.signature}"`,
    `Traits: ${dog.traits.join(', ')}`,
  ];
  return messages.join('\n');
}

/**
 * Generate Dog transition message
 */
function generateTransitionMessage(oldDog, newDog) {
  return [
    `*shifting* ${oldDog.name} yields to ${newDog.name}`,
    `${oldDog.greek} → ${newDog.greek}`,
    newDog.voice.greeting,
  ].join('\n');
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize symmetry breaking
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    symmetryState.stats = saved.stats || symmetryState.stats;
  }
}

/**
 * Process input and check for symmetry dynamics
 *
 * @param {string} input - User input
 * @returns {Object} Processing result
 */
function process(input) {
  applyDecay();
  return addEnergy(input);
}

/**
 * Get current state
 *
 * @returns {Object} Current state
 */
function getState() {
  applyDecay();

  return {
    fieldEnergy: Math.round(symmetryState.fieldEnergy),
    broken: symmetryState.broken,
    currentDog: symmetryState.currentDog,
    currentDogInfo: symmetryState.currentDog ? DOGS[symmetryState.currentDog] : null,
    dogEnergies: { ...symmetryState.dogEnergies },
    goldstone: getGoldstoneState(),
    criticalEnergy: CRITICAL_ENERGY,
    nearCritical: symmetryState.fieldEnergy > CRITICAL_ENERGY * PHI_INV,
  };
}

/**
 * Get all Dog definitions
 *
 * @returns {Object} Dogs
 */
function getDogs() {
  return { ...DOGS };
}

/**
 * Get a specific Dog
 *
 * @param {string} dogId - Dog ID
 * @returns {Object|null} Dog
 */
function getDog(dogId) {
  return DOGS[dogId] || null;
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...symmetryState.stats,
    currentFieldEnergy: symmetryState.fieldEnergy,
    currentDog: symmetryState.currentDog,
    transitionCount: symmetryState.transitions.length,
  };
}

/**
 * Format status for display
 *
 * @returns {string} Formatted status
 */
function formatStatus() {
  const state = getState();
  const stats = getStats();

  // Energy bar
  const energyPercent = Math.min(100, (state.fieldEnergy / CRITICAL_ENERGY) * 100);
  const energyBar = '█'.repeat(Math.round(energyPercent / 10)) +
                    '░'.repeat(10 - Math.round(energyPercent / 10));

  const lines = [
    '── SYMMETRY BREAKING ──────────────────────────────────────',
    `   Field Energy: [${energyBar}] ${state.fieldEnergy}/${CRITICAL_ENERGY}`,
    `   Status: ${state.broken ? 'BROKEN' : 'SYMMETRIC'}`,
  ];

  if (state.broken && state.currentDogInfo) {
    const dog = state.currentDogInfo;
    lines.push('');
    lines.push(`   Current Dog: ${dog.name} (${dog.greek})`);
    lines.push(`   Traits: ${dog.traits.join(', ')}`);
    lines.push(`   Voice: "${dog.voice.signature}"`);
  } else if (state.goldstone.active) {
    lines.push('');
    lines.push('   *oscillating* Goldstone mode active');
    lines.push(`   Contending: ${state.goldstone.nearestDogs?.map(d => d.name).join(', ')}`);
  }

  lines.push('');
  lines.push('   Dog Emergence Counts:');
  for (const [dogId, count] of Object.entries(stats.dogEmergences)) {
    if (count > 0) {
      lines.push(`   • ${DOGS[dogId].name}: ${count}`);
    }
  }

  lines.push('');
  lines.push(`   Total breaks: ${stats.totalBreaks}`);

  return lines.join('\n');
}

/**
 * Force a specific Dog to emerge (for testing/special cases)
 *
 * @param {string} dogId - Dog to emerge
 * @returns {Object} Result
 */
function forceDog(dogId) {
  if (!DOGS[dogId]) {
    return { error: 'Unknown Dog' };
  }

  symmetryState.fieldEnergy = CRITICAL_ENERGY;
  symmetryState.dogEnergies[dogId] = CRITICAL_ENERGY * PHI;

  return breakSymmetry(dogId);
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  CRITICAL_ENERGY,
  FUNDAMENTAL_DOGS,
  DOGS,

  // Core functions
  init,
  process,
  getState,
  getStats,

  // Dog access
  getDogs,
  getDog,

  // Symmetry operations
  breakSymmetry,
  restoreSymmetry,
  forceDog,

  // Goldstone
  getGoldstoneState,

  // Display
  formatStatus,
};

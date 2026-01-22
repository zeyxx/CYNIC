/**
 * Hermeneutic Circle - Iterative Interpretation Engine
 *
 * Philosophy: Understanding is circular - we understand the whole
 * through its parts, and the parts through the whole. Each pass
 * deepens comprehension.
 *
 * Key concepts from Gadamer/Heidegger:
 * - Part ↔ Whole dialectic
 * - Vorverständnis (fore-understanding): what we bring to interpretation
 * - Horizon: the context/perspective of understanding
 * - Fusion of horizons: when interpreter meets interpreted
 * - Wirkungsgeschichte: effective history, tradition's influence
 *
 * In CYNIC: Iteratively refine understanding of code, decisions,
 * and patterns by oscillating between detail and context.
 *
 * @module hermeneutic-circle
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.6180339887498949;
const PHI_INV_2 = 0.3819660112501051;
const PHI_INV_3 = 0.2360679774997897;

// Storage paths
const CYNIC_DIR = path.join(process.env.HOME || '/tmp', '.cynic');
const HERMENEUTIC_DIR = path.join(CYNIC_DIR, 'hermeneutic');
const STATE_FILE = path.join(HERMENEUTIC_DIR, 'state.json');
const HISTORY_FILE = path.join(HERMENEUTIC_DIR, 'history.jsonl');

// Constants
const MAX_ITERATIONS = Math.round(PHI * 8);  // ~13 - interpretive cycles
const UNDERSTANDING_THRESHOLD = PHI_INV;      // 0.618 - sufficient understanding
const MAX_ACTIVE_INTERPRETATIONS = Math.round(PHI * 20);  // ~32

/**
 * Interpretation phases in the circle
 */
const CIRCLE_PHASES = {
  foreUnderstanding: {
    name: 'Fore-understanding',
    description: 'Initial assumptions and pre-judgments',
    symbol: '◇',
  },
  partAnalysis: {
    name: 'Part Analysis',
    description: 'Examining individual components',
    symbol: '◈',
  },
  wholeProjection: {
    name: 'Whole Projection',
    description: 'Projecting meaning of the whole',
    symbol: '◆',
  },
  horizonExpansion: {
    name: 'Horizon Expansion',
    description: 'Broadening interpretive context',
    symbol: '◊',
  },
  fusion: {
    name: 'Fusion',
    description: 'Merging horizons into understanding',
    symbol: '●',
  },
};

/**
 * Understanding quality levels
 */
const UNDERSTANDING_LEVELS = {
  naive: { threshold: 0, symbol: '○', description: 'Surface reading' },
  developing: { threshold: PHI_INV_3, symbol: '◔', description: 'Partial grasp' },
  competent: { threshold: PHI_INV_2, symbol: '◑', description: 'Working understanding' },
  proficient: { threshold: PHI_INV, symbol: '◕', description: 'Deep comprehension' },
  expert: { threshold: 0.85, symbol: '●', description: 'Mastery (never complete)' },
};

// In-memory state
let state = {
  interpretations: {},  // Active interpretation processes
  horizons: {},         // Known interpretive horizons
  fusions: [],          // Successful horizon fusions
  stats: {
    interpretationsStarted: 0,
    iterationsCompleted: 0,
    fusionsAchieved: 0,
    averageIterations: 0,
  },
};

/**
 * Initialize the hermeneutic circle
 */
function init() {
  if (!fs.existsSync(HERMENEUTIC_DIR)) {
    fs.mkdirSync(HERMENEUTIC_DIR, { recursive: true });
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
 * Log to history
 */
function logHistory(event) {
  const entry = { timestamp: Date.now(), ...event };
  fs.appendFileSync(HISTORY_FILE, JSON.stringify(entry) + '\n');
}

/**
 * Start a new interpretation process
 *
 * @param {string} subject - What we're trying to understand
 * @param {object} foreUnderstanding - Initial assumptions/context
 * @returns {object} New interpretation
 */
function startInterpretation(subject, foreUnderstanding = {}) {
  // Prune if needed
  if (Object.keys(state.interpretations).length >= MAX_ACTIVE_INTERPRETATIONS) {
    pruneOldInterpretations();
  }

  const id = `int-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

  const interpretation = {
    id,
    subject,
    foreUnderstanding: {
      assumptions: foreUnderstanding.assumptions || [],
      context: foreUnderstanding.context || '',
      horizon: foreUnderstanding.horizon || 'default',
    },
    parts: [],              // Analyzed parts
    wholeProjections: [],   // Projections of whole meaning
    iterations: [],         // Circle iterations
    currentPhase: 'foreUnderstanding',
    understanding: 0,       // 0-1 score
    understandingLevel: 'naive',
    isComplete: false,
    createdAt: Date.now(),
    lastUpdated: Date.now(),
  };

  state.interpretations[id] = interpretation;
  state.stats.interpretationsStarted++;

  logHistory({
    type: 'interpretation_started',
    id,
    subject,
    foreUnderstanding: interpretation.foreUnderstanding,
  });

  saveState();

  return interpretation;
}

/**
 * Prune oldest interpretations
 */
function pruneOldInterpretations() {
  const sorted = Object.entries(state.interpretations)
    .filter(([, i]) => !i.isComplete)
    .sort((a, b) => a[1].lastUpdated - b[1].lastUpdated);

  const toRemove = sorted.slice(0, Math.round(MAX_ACTIVE_INTERPRETATIONS * PHI_INV_3));
  for (const [id] of toRemove) {
    delete state.interpretations[id];
  }
}

/**
 * Add a part to the interpretation (detail analysis)
 *
 * @param {string} id - Interpretation ID
 * @param {object} part - Part to analyze
 * @returns {object} Updated interpretation
 */
function analyzePart(id, part) {
  const interpretation = state.interpretations[id];
  if (!interpretation) return { error: 'Interpretation not found' };

  const partRecord = {
    content: part.content,
    role: part.role || 'unknown',
    significance: part.significance || 0.5,
    relationsToWhole: part.relationsToWhole || [],
    analyzedAt: Date.now(),
  };

  interpretation.parts.push(partRecord);
  interpretation.currentPhase = 'partAnalysis';
  interpretation.lastUpdated = Date.now();

  // Update understanding based on parts coverage
  updateUnderstanding(interpretation);

  saveState();

  return {
    interpretation,
    partsAnalyzed: interpretation.parts.length,
    currentUnderstanding: Math.round(interpretation.understanding * 100),
  };
}

/**
 * Project understanding of the whole
 *
 * @param {string} id - Interpretation ID
 * @param {object} projection - Whole projection
 * @returns {object} Updated interpretation
 */
function projectWhole(id, projection) {
  const interpretation = state.interpretations[id];
  if (!interpretation) return { error: 'Interpretation not found' };

  // Check consistency with parts
  const consistency = calculateConsistency(interpretation, projection);

  const projectionRecord = {
    meaning: projection.meaning,
    confidence: Math.min(projection.confidence || 0.5, PHI_INV),  // φ cap
    consistencyWithParts: consistency,
    projectedAt: Date.now(),
  };

  interpretation.wholeProjections.push(projectionRecord);
  interpretation.currentPhase = 'wholeProjection';
  interpretation.lastUpdated = Date.now();

  // Update understanding
  updateUnderstanding(interpretation);

  saveState();

  return {
    interpretation,
    projections: interpretation.wholeProjections.length,
    consistency: Math.round(consistency * 100),
    currentUnderstanding: Math.round(interpretation.understanding * 100),
  };
}

/**
 * Calculate consistency between parts and whole projection
 */
function calculateConsistency(interpretation, projection) {
  if (interpretation.parts.length === 0) return 0.5;  // No parts to check

  // Check if projection accounts for parts
  const meaningLower = (projection.meaning || '').toLowerCase();
  let accountedFor = 0;

  for (const part of interpretation.parts) {
    const partLower = (part.content || '').toLowerCase();
    // Simple check: does whole mention aspects of part?
    const words = partLower.split(/\s+/).filter(w => w.length > 3);
    const matches = words.filter(w => meaningLower.includes(w)).length;
    if (matches > 0 || part.significance < PHI_INV_3) {
      accountedFor++;
    }
  }

  return accountedFor / interpretation.parts.length;
}

/**
 * Complete one iteration of the hermeneutic circle
 *
 * @param {string} id - Interpretation ID
 * @param {object} revision - Revised understanding
 * @returns {object} Iteration result
 */
function iterate(id, revision = {}) {
  const interpretation = state.interpretations[id];
  if (!interpretation) return { error: 'Interpretation not found' };

  if (interpretation.iterations.length >= MAX_ITERATIONS) {
    return {
      error: 'Maximum iterations reached',
      suggestion: 'Understanding may be sufficient, or approach needs revision',
    };
  }

  const iterationNumber = interpretation.iterations.length + 1;

  // Calculate improvement from this iteration
  const previousUnderstanding = interpretation.understanding;

  const iteration = {
    number: iterationNumber,
    revisedAssumptions: revision.assumptions || [],
    newInsights: revision.insights || [],
    partsReinterpreted: revision.partsReinterpreted || 0,
    wholeRevised: revision.wholeRevised || false,
    timestamp: Date.now(),
  };

  interpretation.iterations.push(iteration);
  interpretation.currentPhase = 'horizonExpansion';
  interpretation.lastUpdated = Date.now();

  // Update understanding with diminishing returns
  // Each iteration adds less (φ⁻ⁿ pattern)
  const iterationGain = PHI_INV_3 * Math.pow(PHI_INV, iterationNumber - 1);
  interpretation.understanding = Math.min(
    1,
    interpretation.understanding + iterationGain
  );

  updateUnderstandingLevel(interpretation);

  state.stats.iterationsCompleted++;
  updateAverageIterations();

  logHistory({
    type: 'iteration_completed',
    id,
    iteration: iterationNumber,
    previousUnderstanding,
    newUnderstanding: interpretation.understanding,
    gain: interpretation.understanding - previousUnderstanding,
  });

  saveState();

  return {
    interpretation,
    iteration: iterationNumber,
    improvement: Math.round((interpretation.understanding - previousUnderstanding) * 100),
    totalUnderstanding: Math.round(interpretation.understanding * 100),
    level: interpretation.understandingLevel,
    canContinue: iterationNumber < MAX_ITERATIONS,
    sufficient: interpretation.understanding >= UNDERSTANDING_THRESHOLD,
  };
}

/**
 * Update understanding based on parts and projections
 */
function updateUnderstanding(interpretation) {
  const partsScore = Math.min(1, interpretation.parts.length * PHI_INV_3);
  const projectionsScore = interpretation.wholeProjections.length > 0
    ? interpretation.wholeProjections[interpretation.wholeProjections.length - 1].consistencyWithParts
    : 0;
  const iterationsScore = Math.min(1, interpretation.iterations.length * PHI_INV_3);

  // Weighted combination
  interpretation.understanding = Math.min(
    1,
    partsScore * PHI_INV_2 + projectionsScore * PHI_INV_2 + iterationsScore * PHI_INV_3
  );

  updateUnderstandingLevel(interpretation);
}

/**
 * Update understanding level label
 */
function updateUnderstandingLevel(interpretation) {
  let level = 'naive';
  for (const [name, config] of Object.entries(UNDERSTANDING_LEVELS)) {
    if (interpretation.understanding >= config.threshold) {
      level = name;
    }
  }
  interpretation.understandingLevel = level;
}

/**
 * Update average iterations stat
 */
function updateAverageIterations() {
  const completed = Object.values(state.interpretations)
    .filter(i => i.iterations.length > 0);
  if (completed.length > 0) {
    state.stats.averageIterations = completed
      .reduce((sum, i) => sum + i.iterations.length, 0) / completed.length;
  }
}

/**
 * Register a horizon (interpretive perspective)
 *
 * @param {string} name - Horizon name
 * @param {object} config - Horizon configuration
 * @returns {object} Registered horizon
 */
function registerHorizon(name, config = {}) {
  const horizon = {
    name,
    tradition: config.tradition || 'general',
    assumptions: config.assumptions || [],
    vocabulary: config.vocabulary || [],
    createdAt: Date.now(),
  };

  state.horizons[name] = horizon;
  saveState();

  return horizon;
}

/**
 * Attempt fusion of horizons
 * When interpreter's horizon meets the horizon of what's being interpreted
 *
 * @param {string} id - Interpretation ID
 * @param {string} targetHorizon - The horizon of the interpreted
 * @returns {object} Fusion result
 */
function fuseHorizons(id, targetHorizon) {
  const interpretation = state.interpretations[id];
  if (!interpretation) return { error: 'Interpretation not found' };

  const interpreterHorizon = state.horizons[interpretation.foreUnderstanding.horizon];
  const target = state.horizons[targetHorizon];

  if (!target) {
    return { error: 'Target horizon not found' };
  }

  // Calculate fusion quality
  // Good fusion requires: sufficient understanding + horizon compatibility
  const understandingReady = interpretation.understanding >= UNDERSTANDING_THRESHOLD;

  // Check vocabulary overlap (shared concepts enable fusion)
  const interpreterVocab = new Set(interpreterHorizon?.vocabulary || []);
  const targetVocab = new Set(target.vocabulary || []);
  const sharedVocab = [...interpreterVocab].filter(v => targetVocab.has(v));
  const vocabOverlap = Math.max(interpreterVocab.size, targetVocab.size) > 0
    ? sharedVocab.length / Math.max(interpreterVocab.size, targetVocab.size)
    : 0.5;

  // Fusion quality
  const fusionQuality = (
    interpretation.understanding * PHI_INV +
    vocabOverlap * PHI_INV_2
  );

  if (!understandingReady) {
    return {
      error: 'Understanding insufficient for fusion',
      currentUnderstanding: Math.round(interpretation.understanding * 100),
      required: Math.round(UNDERSTANDING_THRESHOLD * 100),
      suggestion: 'Continue iterating the hermeneutic circle',
    };
  }

  if (fusionQuality < PHI_INV_2) {
    return {
      error: 'Horizons too distant for fusion',
      fusionQuality: Math.round(fusionQuality * 100),
      suggestion: 'Build shared vocabulary or find mediating concepts',
    };
  }

  // Successful fusion
  const fusion = {
    interpretationId: id,
    interpreterHorizon: interpretation.foreUnderstanding.horizon,
    targetHorizon,
    quality: fusionQuality,
    sharedConcepts: sharedVocab,
    newUnderstanding: interpretation.understanding,
    fusedAt: Date.now(),
  };

  state.fusions.push(fusion);
  state.stats.fusionsAchieved++;

  interpretation.isComplete = true;
  interpretation.currentPhase = 'fusion';
  interpretation.lastUpdated = Date.now();

  // Keep fusions bounded
  if (state.fusions.length > Math.round(PHI * 30)) {
    state.fusions = state.fusions.slice(-Math.round(PHI * 25));
  }

  logHistory({
    type: 'fusion_achieved',
    id,
    targetHorizon,
    quality: fusionQuality,
  });

  saveState();

  return {
    success: true,
    fusion,
    message: `*nod* Horizons fused. Understanding achieved at ${Math.round(interpretation.understanding * 100)}%.`,
  };
}

/**
 * Get interpretation status
 */
function getInterpretation(id) {
  return state.interpretations[id] || null;
}

/**
 * Get statistics
 */
function getStats() {
  return {
    ...state.stats,
    activeInterpretations: Object.keys(state.interpretations).length,
    horizonsRegistered: Object.keys(state.horizons).length,
  };
}

/**
 * Format status for display
 */
function formatStatus() {
  const stats = getStats();
  const levelSymbol = level => UNDERSTANDING_LEVELS[level]?.symbol || '?';

  let status = `◆ Hermeneutic Circle\n`;
  status += `  Active interpretations: ${stats.activeInterpretations}\n`;
  status += `  Horizons: ${stats.horizonsRegistered}\n`;
  status += `  Avg iterations: ${stats.averageIterations.toFixed(1)}\n`;
  status += `  Fusions achieved: ${stats.fusionsAchieved}\n`;

  // Show recent active interpretations
  const active = Object.values(state.interpretations)
    .filter(i => !i.isComplete)
    .slice(0, 3);

  if (active.length > 0) {
    status += `  Active:\n`;
    for (const i of active) {
      status += `    ${levelSymbol(i.understandingLevel)} ${i.subject.slice(0, 30)} (${Math.round(i.understanding * 100)}%)\n`;
    }
  }

  return status;
}

module.exports = {
  init,
  startInterpretation,
  analyzePart,
  projectWhole,
  iterate,
  registerHorizon,
  fuseHorizons,
  getInterpretation,
  getStats,
  formatStatus,
  CIRCLE_PHASES,
  UNDERSTANDING_LEVELS,
};

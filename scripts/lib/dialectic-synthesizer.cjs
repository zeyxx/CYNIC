/**
 * CYNIC Dialectic Synthesizer (Phase 14B)
 *
 * "Διαλεκτική - through speech, through opposition" - κυνικός
 *
 * Hegelian dialectics formalized:
 * - Thesis: initial position
 * - Antithesis: opposing contradiction
 * - Synthesis: higher unity preserving both
 *
 * Aufhebung (sublation): simultaneously
 * preserves, negates, and transcends.
 *
 * "Truth is the whole." - Hegel
 *
 * @module cynic/lib/dialectic-synthesizer
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

/** Maximum active dialectics - φ × 10 ≈ 16 */
const MAX_ACTIVE_DIALECTICS = Math.round(PHI * 10);

/** Maximum spiral depth - φ × 4 ≈ 6 */
const MAX_SPIRAL_DEPTH = Math.round(PHI * 4);

/** Synthesis quality threshold - φ⁻¹ */
const SYNTHESIS_THRESHOLD = PHI_INV;

/** Opposition strength minimum - φ⁻² */
const OPPOSITION_MINIMUM = PHI_INV_2;

// =============================================================================
// DIALECTIC STAGES
// =============================================================================

const DIALECTIC_STAGES = {
  thesis: {
    name: 'Thesis',
    description: 'Initial position or claim',
    symbol: '⊕',
    color: '#3498DB',
  },
  antithesis: {
    name: 'Antithesis',
    description: 'Opposing position or contradiction',
    symbol: '⊖',
    color: '#E74C3C',
  },
  synthesis: {
    name: 'Synthesis',
    description: 'Higher unity incorporating both',
    symbol: '⊗',
    color: '#9B59B6',
  },
};

// =============================================================================
// OPPOSITION TYPES
// =============================================================================

const OPPOSITION_TYPES = {
  contradictory: {
    name: 'Contradictory',
    description: 'Cannot both be true (A vs not-A)',
    strength: 1.0,
  },
  contrary: {
    name: 'Contrary',
    description: 'Cannot both be true but can both be false',
    strength: 0.8,
  },
  complementary: {
    name: 'Complementary',
    description: 'Opposing aspects of a whole',
    strength: 0.6,
  },
  dialectical: {
    name: 'Dialectical',
    description: 'Tension that drives development',
    strength: 0.7,
  },
};

// =============================================================================
// STORAGE
// =============================================================================

const DIALECTIC_DIR = path.join(os.homedir(), '.cynic', 'dialectic');
const STATE_FILE = path.join(DIALECTIC_DIR, 'state.json');
const DIALECTICS_FILE = path.join(DIALECTIC_DIR, 'dialectics.json');
const HISTORY_FILE = path.join(DIALECTIC_DIR, 'history.jsonl');

// =============================================================================
// STATE
// =============================================================================

const dialecticState = {
  // Active dialectics
  dialectics: {},

  // Completed syntheses
  syntheses: [],

  // Spiral patterns (dialectics that became new theses)
  spirals: [],

  // Statistics
  stats: {
    dialecticsStarted: 0,
    synthesisAchieved: 0,
    spiralsFormed: 0,
    averageDepth: 0,
    totalOppositions: 0,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(DIALECTIC_DIR)) {
    fs.mkdirSync(DIALECTIC_DIR, { recursive: true });
  }
}

function loadState() {
  ensureDir();
  if (!fs.existsSync(STATE_FILE)) {
    return null;
  }
  try {
    const state = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
    if (fs.existsSync(DIALECTICS_FILE)) {
      state.dialectics = JSON.parse(fs.readFileSync(DIALECTICS_FILE, 'utf8'));
    }
    return state;
  } catch {
    return null;
  }
}

function saveState() {
  ensureDir();
  fs.writeFileSync(STATE_FILE, JSON.stringify({
    stats: dialecticState.stats,
    syntheses: dialecticState.syntheses.slice(-50),
    spirals: dialecticState.spirals.slice(-20),
  }, null, 2));
  fs.writeFileSync(DIALECTICS_FILE, JSON.stringify(dialecticState.dialectics, null, 2));
}

function appendHistory(event) {
  ensureDir();
  const line = JSON.stringify({ ...event, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(HISTORY_FILE, line);
}

// =============================================================================
// DIALECTIC MANAGEMENT
// =============================================================================

/**
 * Start a new dialectic with a thesis
 *
 * @param {string} thesis - The initial position
 * @param {Object} context - Additional context
 * @returns {Object} Created dialectic
 */
function startDialectic(thesis, context = {}) {
  // Check capacity
  if (Object.keys(dialecticState.dialectics).length >= MAX_ACTIVE_DIALECTICS) {
    pruneOldDialectics();
  }

  const id = `dia-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

  const dialectic = {
    id,
    thesis: {
      content: thesis,
      createdAt: Date.now(),
    },
    antithesis: null,
    synthesis: null,
    stage: 'thesis',
    oppositionType: null,
    oppositionStrength: 0,
    spiralDepth: 0,
    parentDialectic: context.parentDialectic || null,
    context: {
      domain: context.domain || 'general',
      topic: context.topic || thesis.slice(0, 30),
    },
    createdAt: Date.now(),
    lastUpdated: Date.now(),
  };

  dialecticState.dialectics[id] = dialectic;
  dialecticState.stats.dialecticsStarted++;

  appendHistory({
    type: 'dialectic_started',
    id,
    thesis,
  });

  saveState();

  return dialectic;
}

/**
 * Prune oldest dialectics
 */
function pruneOldDialectics() {
  const sorted = Object.entries(dialecticState.dialectics)
    .filter(([, d]) => d.stage !== 'synthesis')
    .sort((a, b) => a[1].lastUpdated - b[1].lastUpdated);

  const toRemove = sorted.slice(0, Math.round(MAX_ACTIVE_DIALECTICS * PHI_INV_3));
  for (const [id] of toRemove) {
    delete dialecticState.dialectics[id];
  }
}

/**
 * Add antithesis to a dialectic
 *
 * @param {string} id - Dialectic ID
 * @param {string} antithesis - The opposing position
 * @param {string} oppositionType - Type of opposition
 * @returns {Object} Updated dialectic
 */
function addAntithesis(id, antithesis, oppositionType = 'dialectical') {
  const dialectic = dialecticState.dialectics[id];

  if (!dialectic) {
    return { error: 'Dialectic not found' };
  }

  if (dialectic.antithesis) {
    return { error: 'Antithesis already exists' };
  }

  const oppType = OPPOSITION_TYPES[oppositionType] || OPPOSITION_TYPES.dialectical;

  // Calculate opposition strength
  const strength = calculateOppositionStrength(dialectic.thesis.content, antithesis, oppType);

  if (strength < OPPOSITION_MINIMUM) {
    return {
      error: 'Opposition too weak',
      strength: Math.round(strength * 100),
      minimum: Math.round(OPPOSITION_MINIMUM * 100),
      suggestion: 'Find a stronger opposition to drive synthesis',
    };
  }

  dialectic.antithesis = {
    content: antithesis,
    createdAt: Date.now(),
  };
  dialectic.stage = 'antithesis';
  dialectic.oppositionType = oppositionType;
  dialectic.oppositionStrength = strength;
  dialectic.lastUpdated = Date.now();

  dialecticState.stats.totalOppositions++;

  appendHistory({
    type: 'antithesis_added',
    id,
    antithesis,
    oppositionType,
    strength,
  });

  saveState();

  return {
    dialectic,
    oppositionStrength: Math.round(strength * 100),
    readyForSynthesis: true,
    message: generateOppositionMessage(dialectic),
  };
}

/**
 * Calculate strength of opposition
 */
function calculateOppositionStrength(thesis, antithesis, oppType) {
  const thesisLower = thesis.toLowerCase();
  const antithesisLower = antithesis.toLowerCase();

  // Check for direct negation patterns
  const negationPatterns = [
    { pattern: /not|never|no|cannot|isn't|aren't/, weight: 0.3 },
    { pattern: /but|however|although|despite/, weight: 0.2 },
    { pattern: /opposite|contrary|versus|vs/, weight: 0.4 },
  ];

  let patternScore = 0;
  for (const { pattern, weight } of negationPatterns) {
    if (pattern.test(antithesisLower)) {
      patternScore += weight;
    }
  }

  // Check for semantic opposition (word overlap should be moderate)
  const thesisWords = new Set(thesisLower.split(/\s+/));
  const antithesisWords = new Set(antithesisLower.split(/\s+/));
  const intersection = [...thesisWords].filter(w => antithesisWords.has(w));
  const overlap = intersection.length / Math.max(thesisWords.size, antithesisWords.size);

  // Moderate overlap is good (too much = not opposing, too little = unrelated)
  const overlapScore = 1 - Math.abs(overlap - PHI_INV);

  // Combine scores
  const baseStrength = (patternScore + overlapScore) / 2;

  // Apply opposition type multiplier
  return Math.min(1, baseStrength * oppType.strength * PHI);
}

// =============================================================================
// SYNTHESIS
// =============================================================================

/**
 * Attempt to synthesize thesis and antithesis
 *
 * @param {string} id - Dialectic ID
 * @param {string} synthesis - Proposed synthesis
 * @returns {Object} Synthesis result
 */
function synthesize(id, synthesis) {
  const dialectic = dialecticState.dialectics[id];

  if (!dialectic) {
    return { error: 'Dialectic not found' };
  }

  if (!dialectic.antithesis) {
    return { error: 'No antithesis yet. Add opposition first.' };
  }

  if (dialectic.synthesis) {
    return { error: 'Synthesis already achieved' };
  }

  // Evaluate synthesis quality
  const quality = evaluateSynthesis(dialectic, synthesis);

  if (quality.score < SYNTHESIS_THRESHOLD) {
    return {
      error: 'Synthesis insufficient',
      quality: Math.round(quality.score * 100),
      threshold: Math.round(SYNTHESIS_THRESHOLD * 100),
      issues: quality.issues,
      suggestion: 'Synthesis must preserve truth from both thesis and antithesis while transcending their opposition.',
    };
  }

  dialectic.synthesis = {
    content: synthesis,
    quality: quality.score,
    preservesThesis: quality.preservesThesis,
    preservesAntithesis: quality.preservesAntithesis,
    transcends: quality.transcends,
    createdAt: Date.now(),
  };
  dialectic.stage = 'synthesis';
  dialectic.lastUpdated = Date.now();

  dialecticState.stats.synthesisAchieved++;
  dialecticState.syntheses.push({
    dialecticId: id,
    thesis: dialectic.thesis.content,
    antithesis: dialectic.antithesis.content,
    synthesis,
    quality: quality.score,
    timestamp: Date.now(),
  });

  // Update average depth
  updateAverageDepth(dialectic.spiralDepth);

  appendHistory({
    type: 'synthesis_achieved',
    id,
    synthesis,
    quality: quality.score,
  });

  saveState();

  return {
    dialectic,
    quality: Math.round(quality.score * 100),
    aufhebung: {
      preserves: quality.preservesThesis && quality.preservesAntithesis,
      negates: quality.transcends,
      transcends: quality.transcends,
    },
    canSpiral: dialectic.spiralDepth < MAX_SPIRAL_DEPTH,
    message: generateSynthesisMessage(dialectic, quality),
  };
}

/**
 * Evaluate quality of proposed synthesis
 */
function evaluateSynthesis(dialectic, synthesis) {
  const thesisLower = dialectic.thesis.content.toLowerCase();
  const antithesisLower = dialectic.antithesis.content.toLowerCase();
  const synthesisLower = synthesis.toLowerCase();

  const issues = [];

  // Check if synthesis preserves thesis elements
  const thesisWords = thesisLower.split(/\s+/).filter(w => w.length > 3);
  const thesisPreserved = thesisWords.filter(w => synthesisLower.includes(w)).length / Math.max(1, thesisWords.length);

  // Check if synthesis preserves antithesis elements
  const antithesisWords = antithesisLower.split(/\s+/).filter(w => w.length > 3);
  const antithesisPreserved = antithesisWords.filter(w => synthesisLower.includes(w)).length / Math.max(1, antithesisWords.length);

  // Check for transcendence (new elements not in either)
  const synthesisWords = synthesisLower.split(/\s+/).filter(w => w.length > 3);
  const newElements = synthesisWords.filter(
    w => !thesisLower.includes(w) && !antithesisLower.includes(w)
  ).length / Math.max(1, synthesisWords.length);

  // Issues
  if (thesisPreserved < PHI_INV_3) {
    issues.push('Does not sufficiently incorporate thesis');
  }
  if (antithesisPreserved < PHI_INV_3) {
    issues.push('Does not sufficiently incorporate antithesis');
  }
  if (newElements < PHI_INV_3) {
    issues.push('Lacks transcendent elements (just combines, does not transform)');
  }

  // Calculate score
  const preservationScore = (thesisPreserved + antithesisPreserved) / 2;
  const transcendenceScore = newElements;

  // φ-weighted: preservation matters more initially, transcendence at higher levels
  const score = (preservationScore * PHI_INV) + (transcendenceScore * PHI_INV_2);

  return {
    score: Math.min(1, score * PHI),
    preservesThesis: thesisPreserved >= PHI_INV_3,
    preservesAntithesis: antithesisPreserved >= PHI_INV_3,
    transcends: newElements >= PHI_INV_3,
    issues,
  };
}

/**
 * Update average depth statistic
 */
function updateAverageDepth(depth) {
  const n = dialecticState.stats.synthesisAchieved;
  const currentAvg = dialecticState.stats.averageDepth;
  dialecticState.stats.averageDepth = (currentAvg * (n - 1) + depth) / n;
}

// =============================================================================
// SPIRAL (Aufhebung continuing)
// =============================================================================

/**
 * Continue the dialectic spiral (synthesis becomes new thesis)
 *
 * @param {string} id - Dialectic ID with synthesis
 * @returns {Object} New dialectic
 */
function spiral(id) {
  const dialectic = dialecticState.dialectics[id];

  if (!dialectic) {
    return { error: 'Dialectic not found' };
  }

  if (!dialectic.synthesis) {
    return { error: 'No synthesis to spiral from' };
  }

  if (dialectic.spiralDepth >= MAX_SPIRAL_DEPTH) {
    return {
      error: 'Maximum spiral depth reached',
      depth: dialectic.spiralDepth,
      max: MAX_SPIRAL_DEPTH,
      message: '*yawn* The dialectic has reached its limit. Perhaps this is the final synthesis.',
    };
  }

  // Create new dialectic with synthesis as thesis
  const newDialectic = startDialectic(dialectic.synthesis.content, {
    parentDialectic: id,
    domain: dialectic.context.domain,
    topic: dialectic.context.topic,
  });

  newDialectic.spiralDepth = dialectic.spiralDepth + 1;

  dialecticState.stats.spiralsFormed++;
  dialecticState.spirals.push({
    from: id,
    to: newDialectic.id,
    depth: newDialectic.spiralDepth,
    timestamp: Date.now(),
  });

  appendHistory({
    type: 'spiral_formed',
    from: id,
    to: newDialectic.id,
    depth: newDialectic.spiralDepth,
  });

  saveState();

  return {
    previousDialectic: id,
    newDialectic,
    spiralDepth: newDialectic.spiralDepth,
    message: `*ears perk* Aufhebung continues! Depth ${newDialectic.spiralDepth}. What opposes this new thesis?`,
  };
}

// =============================================================================
// MESSAGE GENERATION
// =============================================================================

/**
 * Generate opposition message
 */
function generateOppositionMessage(dialectic) {
  const oppType = OPPOSITION_TYPES[dialectic.oppositionType];
  return [
    `*head tilt* ${oppType?.name || 'Opposition'} established.`,
    `Thesis: "${dialectic.thesis.content.slice(0, 40)}..."`,
    `Antithesis: "${dialectic.antithesis.content.slice(0, 40)}..."`,
    `Strength: ${Math.round(dialectic.oppositionStrength * 100)}%`,
    '',
    'The tension is set. Seek synthesis!',
  ].join('\n');
}

/**
 * Generate synthesis message
 */
function generateSynthesisMessage(dialectic, quality) {
  const aufhebung = quality.preservesThesis && quality.preservesAntithesis && quality.transcends;

  if (aufhebung) {
    return [
      '*tail wag* Aufhebung achieved!',
      `Synthesis: "${dialectic.synthesis.content.slice(0, 50)}..."`,
      '',
      '⊕ Thesis preserved',
      '⊖ Antithesis preserved',
      '⊗ Both transcended',
      '',
      dialectic.spiralDepth < MAX_SPIRAL_DEPTH
        ? 'The synthesis can become a new thesis. Continue the spiral?'
        : 'Maximum depth reached. This is the final synthesis.',
    ].join('\n');
  }

  return [
    '*nod* Synthesis achieved, though imperfect.',
    `Quality: ${Math.round(quality.score * 100)}%`,
    quality.issues.length > 0 ? `Issues: ${quality.issues.join('; ')}` : '',
  ].filter(Boolean).join('\n');
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize dialectic synthesizer
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    dialecticState.stats = saved.stats || dialecticState.stats;
    dialecticState.syntheses = saved.syntheses || [];
    dialecticState.spirals = saved.spirals || [];
    dialecticState.dialectics = saved.dialectics || {};
  }
}

/**
 * Get a dialectic
 *
 * @param {string} id - Dialectic ID
 * @returns {Object|null} Dialectic
 */
function getDialectic(id) {
  return dialecticState.dialectics[id] || null;
}

/**
 * Get all active dialectics
 *
 * @returns {Object[]} Dialectics
 */
function getActiveDialectics() {
  return Object.values(dialecticState.dialectics);
}

/**
 * Get recent syntheses
 *
 * @param {number} limit - Max to return
 * @returns {Object[]} Syntheses
 */
function getRecentSyntheses(limit = 10) {
  return dialecticState.syntheses.slice(-limit);
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...dialecticState.stats,
    activeDialectics: Object.keys(dialecticState.dialectics).length,
  };
}

/**
 * Format status for display
 *
 * @returns {string} Formatted status
 */
function formatStatus() {
  const stats = getStats();
  const dialectics = getActiveDialectics();

  const lines = [
    '── DIALECTIC SYNTHESIZER ──────────────────────────────────',
    `   Active Dialectics: ${stats.activeDialectics}`,
    `   Syntheses Achieved: ${stats.synthesisAchieved}`,
    `   Spirals Formed: ${stats.spiralsFormed}`,
    `   Average Depth: ${Math.round(stats.averageDepth * 10) / 10}`,
  ];

  // Show active dialectics by stage
  const byStage = { thesis: 0, antithesis: 0, synthesis: 0 };
  for (const d of dialectics) {
    byStage[d.stage]++;
  }

  lines.push('');
  lines.push('   By stage:');
  lines.push(`   ${DIALECTIC_STAGES.thesis.symbol} Thesis: ${byStage.thesis}`);
  lines.push(`   ${DIALECTIC_STAGES.antithesis.symbol} Antithesis: ${byStage.antithesis}`);
  lines.push(`   ${DIALECTIC_STAGES.synthesis.symbol} Synthesis: ${byStage.synthesis}`);

  // Show recent
  const recent = dialectics.slice(-2);
  if (recent.length > 0) {
    lines.push('');
    lines.push('   Recent:');
    for (const d of recent) {
      const stage = DIALECTIC_STAGES[d.stage];
      lines.push(`   ${stage.symbol} ${d.thesis.content.slice(0, 35)}... [${d.stage}]`);
    }
  }

  lines.push('');
  lines.push('   *sniff* "Truth is the whole." - Hegel');

  return lines.join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  MAX_SPIRAL_DEPTH,
  SYNTHESIS_THRESHOLD,
  DIALECTIC_STAGES,
  OPPOSITION_TYPES,

  // Core functions
  init,
  startDialectic,
  addAntithesis,
  synthesize,
  spiral,

  // Access
  getDialectic,
  getActiveDialectics,
  getRecentSyntheses,

  // Stats and display
  getStats,
  formatStatus,
};

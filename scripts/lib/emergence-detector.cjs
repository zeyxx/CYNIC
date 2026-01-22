/**
 * CYNIC Emergence Detector (Phase 13A)
 *
 * "Ἀνάδυσις - rising up from below" - κυνικός
 *
 * Detects emergent patterns - when complex behavior
 * arises from simpler components:
 * - The whole is more than the sum of parts
 * - Novel properties appear at higher levels
 * - Self-organization without central control
 *
 * "More is different." - P.W. Anderson
 *
 * @module cynic/lib/emergence-detector
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

/** Minimum components for emergence - φ × 3 ≈ 5 */
const MIN_COMPONENTS = Math.round(PHI * 3);

/** Emergence threshold (complexity ratio) - φ */
const EMERGENCE_THRESHOLD = PHI;

/** Maximum tracked emergent patterns - φ × 30 ≈ 49 */
const MAX_EMERGENT_PATTERNS = Math.round(PHI * 30);

/** Level count for hierarchical analysis - φ × 3 ≈ 5 */
const HIERARCHY_LEVELS = Math.round(PHI * 3);

/** Novelty decay rate per hour - φ⁻³ */
const NOVELTY_DECAY_RATE = PHI_INV_3;

// =============================================================================
// EMERGENCE TYPES
// =============================================================================

const EMERGENCE_TYPES = {
  weak: {
    name: 'Weak Emergence',
    description: 'Predictable from components but not obvious',
    threshold: PHI_INV_2,
    example: 'Code style emerging from multiple contributors',
  },
  strong: {
    name: 'Strong Emergence',
    description: 'Cannot be predicted from components alone',
    threshold: PHI_INV,
    example: 'Unexpected behavior from interacting modules',
  },
  radical: {
    name: 'Radical Emergence',
    description: 'Qualitatively new properties appear',
    threshold: PHI,
    example: 'Self-modifying behavior, consciousness indicators',
  },
};

// =============================================================================
// STORAGE
// =============================================================================

const EMERGENCE_DIR = path.join(os.homedir(), '.cynic', 'emergence');
const STATE_FILE = path.join(EMERGENCE_DIR, 'state.json');
const PATTERNS_FILE = path.join(EMERGENCE_DIR, 'patterns.json');
const HISTORY_FILE = path.join(EMERGENCE_DIR, 'history.jsonl');

// =============================================================================
// STATE
// =============================================================================

const emergenceState = {
  // Tracked components (building blocks)
  components: {},

  // Detected emergent patterns
  patterns: {},

  // Interaction graph (which components interact)
  interactions: {},

  // Hierarchy levels
  levels: {
    micro: [],    // Individual actions/lines
    meso: [],     // Functions/modules
    macro: [],    // Systems/architectures
    meta: [],     // Cross-system patterns
    cosmic: [],   // Universal patterns
  },

  // Statistics
  stats: {
    componentsTracked: 0,
    interactionsRecorded: 0,
    emergentPatterns: 0,
    byType: {
      weak: 0,
      strong: 0,
      radical: 0,
    },
    phaseTransitions: 0,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(EMERGENCE_DIR)) {
    fs.mkdirSync(EMERGENCE_DIR, { recursive: true });
  }
}

function loadState() {
  ensureDir();
  if (!fs.existsSync(STATE_FILE)) {
    return null;
  }
  try {
    const state = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
    if (fs.existsSync(PATTERNS_FILE)) {
      state.patterns = JSON.parse(fs.readFileSync(PATTERNS_FILE, 'utf8'));
    }
    return state;
  } catch {
    return null;
  }
}

function saveState() {
  ensureDir();
  fs.writeFileSync(STATE_FILE, JSON.stringify({
    stats: emergenceState.stats,
    components: emergenceState.components,
    interactions: emergenceState.interactions,
  }, null, 2));
  fs.writeFileSync(PATTERNS_FILE, JSON.stringify(emergenceState.patterns, null, 2));
}

function appendHistory(event) {
  ensureDir();
  const line = JSON.stringify({ ...event, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(HISTORY_FILE, line);
}

// =============================================================================
// COMPONENT TRACKING
// =============================================================================

/**
 * Register a component (building block)
 *
 * @param {string} id - Component identifier
 * @param {Object} properties - Component properties
 * @returns {Object} Registered component
 */
function registerComponent(id, properties = {}) {
  emergenceState.components[id] = {
    id,
    properties,
    level: properties.level || 'micro',
    registeredAt: Date.now(),
    interactions: [],
    partOf: [], // Higher-level patterns this belongs to
  };

  emergenceState.stats.componentsTracked++;

  // Add to appropriate level
  const level = properties.level || 'micro';
  if (emergenceState.levels[level]) {
    emergenceState.levels[level].push(id);
  }

  return emergenceState.components[id];
}

/**
 * Record an interaction between components
 *
 * @param {string} id1 - First component
 * @param {string} id2 - Second component
 * @param {string} type - Interaction type
 * @returns {Object} Interaction record
 */
function recordInteraction(id1, id2, type = 'generic') {
  const key = [id1, id2].sort().join('↔');

  if (!emergenceState.interactions[key]) {
    emergenceState.interactions[key] = {
      components: [id1, id2],
      type,
      count: 0,
      firstSeen: Date.now(),
      lastSeen: Date.now(),
    };
  }

  emergenceState.interactions[key].count++;
  emergenceState.interactions[key].lastSeen = Date.now();
  emergenceState.stats.interactionsRecorded++;

  // Update component records
  if (emergenceState.components[id1]) {
    if (!emergenceState.components[id1].interactions.includes(id2)) {
      emergenceState.components[id1].interactions.push(id2);
    }
  }
  if (emergenceState.components[id2]) {
    if (!emergenceState.components[id2].interactions.includes(id1)) {
      emergenceState.components[id2].interactions.push(id1);
    }
  }

  // Check for emergence after interaction
  const emergence = checkEmergence([id1, id2]);

  return {
    interaction: emergenceState.interactions[key],
    emergence,
  };
}

// =============================================================================
// EMERGENCE DETECTION
// =============================================================================

/**
 * Check if components exhibit emergent behavior
 *
 * @param {string[]} componentIds - Components to check
 * @returns {Object|null} Emergence if detected
 */
function checkEmergence(componentIds) {
  if (componentIds.length < MIN_COMPONENTS) {
    // Try to expand with interacting components
    componentIds = expandComponentSet(componentIds);
  }

  if (componentIds.length < MIN_COMPONENTS) {
    return null;
  }

  // Calculate complexity metrics
  const individualComplexity = calculateIndividualComplexity(componentIds);
  const collectiveComplexity = calculateCollectiveComplexity(componentIds);

  // Emergence ratio: collective / sum of individual
  const emergenceRatio = collectiveComplexity / Math.max(1, individualComplexity);

  // Determine emergence type
  let emergenceType = null;
  for (const [type, config] of Object.entries(EMERGENCE_TYPES)) {
    if (emergenceRatio >= config.threshold) {
      emergenceType = type;
    }
  }

  if (!emergenceType) {
    return null;
  }

  // New emergent pattern detected!
  return createEmergentPattern(componentIds, emergenceType, emergenceRatio);
}

/**
 * Expand component set with interacting components
 */
function expandComponentSet(ids) {
  const expanded = new Set(ids);

  for (const id of ids) {
    const component = emergenceState.components[id];
    if (component) {
      for (const interacting of component.interactions) {
        expanded.add(interacting);
      }
    }
  }

  return Array.from(expanded);
}

/**
 * Calculate individual complexity (sum of parts)
 */
function calculateIndividualComplexity(componentIds) {
  let total = 0;

  for (const id of componentIds) {
    const component = emergenceState.components[id];
    if (component) {
      // Complexity based on properties and interactions
      const propCount = Object.keys(component.properties).length;
      const interactionCount = component.interactions.length;
      total += propCount + interactionCount;
    }
  }

  return total;
}

/**
 * Calculate collective complexity (whole system)
 */
function calculateCollectiveComplexity(componentIds) {
  // Count unique interaction patterns
  const interactionSet = new Set();

  for (const id of componentIds) {
    const component = emergenceState.components[id];
    if (component) {
      for (const other of component.interactions) {
        if (componentIds.includes(other)) {
          const key = [id, other].sort().join('↔');
          interactionSet.add(key);
        }
      }
    }
  }

  // Collective complexity includes:
  // - Number of internal interactions
  // - Graph connectivity
  // - Potential for feedback loops

  const internalInteractions = interactionSet.size;
  const maxPossible = (componentIds.length * (componentIds.length - 1)) / 2;
  const connectivity = maxPossible > 0 ? internalInteractions / maxPossible : 0;

  // φ-weighted formula
  return (internalInteractions * PHI) + (connectivity * componentIds.length * PHI_INV);
}

/**
 * Create an emergent pattern record
 */
function createEmergentPattern(componentIds, type, ratio) {
  const id = `emg-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

  const pattern = {
    id,
    type,
    typeName: EMERGENCE_TYPES[type].name,
    components: componentIds,
    componentCount: componentIds.length,
    emergenceRatio: Math.round(ratio * 100) / 100,
    level: determineLevel(componentIds),
    novelty: 1.0, // Starts fully novel
    detectedAt: Date.now(),
    lastSeen: Date.now(),
  };

  // Check capacity
  if (Object.keys(emergenceState.patterns).length >= MAX_EMERGENT_PATTERNS) {
    pruneOldPatterns();
  }

  emergenceState.patterns[id] = pattern;
  emergenceState.stats.emergentPatterns++;
  emergenceState.stats.byType[type]++;

  // Update components' partOf
  for (const compId of componentIds) {
    if (emergenceState.components[compId]) {
      emergenceState.components[compId].partOf.push(id);
    }
  }

  appendHistory({
    type: 'emergence_detected',
    patternId: id,
    emergenceType: type,
    ratio,
    componentCount: componentIds.length,
  });

  saveState();

  return pattern;
}

/**
 * Determine hierarchy level of emergent pattern
 */
function determineLevel(componentIds) {
  const levels = componentIds.map(id => {
    const comp = emergenceState.components[id];
    return comp?.level || 'micro';
  });

  const levelOrder = ['micro', 'meso', 'macro', 'meta', 'cosmic'];
  const maxLevel = levels.reduce((max, level) => {
    const maxIdx = levelOrder.indexOf(max);
    const levelIdx = levelOrder.indexOf(level);
    return levelIdx > maxIdx ? level : max;
  }, 'micro');

  // Emergence happens one level up from components
  const idx = levelOrder.indexOf(maxLevel);
  return levelOrder[Math.min(idx + 1, levelOrder.length - 1)];
}

/**
 * Prune oldest patterns
 */
function pruneOldPatterns() {
  const sorted = Object.entries(emergenceState.patterns)
    .sort((a, b) => a[1].lastSeen - b[1].lastSeen);

  const toRemove = sorted.slice(0, Math.round(MAX_EMERGENT_PATTERNS * PHI_INV_3));
  for (const [id] of toRemove) {
    delete emergenceState.patterns[id];
  }
}

// =============================================================================
// PHASE TRANSITIONS
// =============================================================================

/**
 * Detect phase transition (sudden qualitative change)
 *
 * @param {string} patternId - Pattern to check
 * @returns {Object|null} Phase transition if detected
 */
function detectPhaseTransition(patternId) {
  const pattern = emergenceState.patterns[patternId];
  if (!pattern) return null;

  // Check if pattern type has upgraded
  const currentRatio = pattern.emergenceRatio;
  const componentIds = pattern.components;

  // Recalculate with current state
  const individualComplexity = calculateIndividualComplexity(componentIds);
  const collectiveComplexity = calculateCollectiveComplexity(componentIds);
  const newRatio = collectiveComplexity / Math.max(1, individualComplexity);

  // Phase transition if ratio crosses a threshold
  for (const [type, config] of Object.entries(EMERGENCE_TYPES)) {
    if (newRatio >= config.threshold && currentRatio < config.threshold) {
      // Transition detected!
      emergenceState.stats.phaseTransitions++;

      const transition = {
        patternId,
        from: pattern.type,
        to: type,
        oldRatio: currentRatio,
        newRatio: Math.round(newRatio * 100) / 100,
        timestamp: Date.now(),
      };

      pattern.type = type;
      pattern.typeName = config.name;
      pattern.emergenceRatio = newRatio;

      appendHistory({
        type: 'phase_transition',
        ...transition,
      });

      saveState();

      return transition;
    }
  }

  return null;
}

// =============================================================================
// ANALYSIS
// =============================================================================

/**
 * Analyze emergence across all components
 *
 * @returns {Object} Analysis
 */
function analyzeEmergence() {
  const patterns = Object.values(emergenceState.patterns);

  // Group by type
  const byType = {};
  for (const type of Object.keys(EMERGENCE_TYPES)) {
    byType[type] = patterns.filter(p => p.type === type);
  }

  // Find largest emergent patterns
  const largest = patterns
    .sort((a, b) => b.componentCount - a.componentCount)
    .slice(0, 5);

  // Find most novel
  const mostNovel = patterns
    .sort((a, b) => b.novelty - a.novelty)
    .slice(0, 5);

  // Hierarchy distribution
  const levelDist = {};
  for (const pattern of patterns) {
    levelDist[pattern.level] = (levelDist[pattern.level] || 0) + 1;
  }

  return {
    totalPatterns: patterns.length,
    byType: Object.fromEntries(
      Object.entries(byType).map(([k, v]) => [k, v.length])
    ),
    largest: largest.map(p => ({
      id: p.id,
      type: p.type,
      components: p.componentCount,
      ratio: p.emergenceRatio,
    })),
    mostNovel: mostNovel.map(p => ({
      id: p.id,
      type: p.type,
      novelty: Math.round(p.novelty * 100),
    })),
    levelDistribution: levelDist,
    phaseTransitions: emergenceState.stats.phaseTransitions,
  };
}

/**
 * Apply novelty decay
 */
function applyNoveltyDecay() {
  const now = Date.now();

  for (const pattern of Object.values(emergenceState.patterns)) {
    const hoursSinceDetection = (now - pattern.detectedAt) / (3600 * 1000);
    pattern.novelty = Math.exp(-NOVELTY_DECAY_RATE * hoursSinceDetection);
  }
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize emergence detector
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    emergenceState.stats = saved.stats || emergenceState.stats;
    emergenceState.components = saved.components || {};
    emergenceState.interactions = saved.interactions || {};
    emergenceState.patterns = saved.patterns || {};
  }
}

/**
 * Get all emergent patterns
 *
 * @returns {Object[]} Patterns
 */
function getPatterns() {
  applyNoveltyDecay();
  return Object.values(emergenceState.patterns);
}

/**
 * Get a specific pattern
 *
 * @param {string} id - Pattern ID
 * @returns {Object|null} Pattern
 */
function getPattern(id) {
  return emergenceState.patterns[id] || null;
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...emergenceState.stats,
    activePatterns: Object.keys(emergenceState.patterns).length,
    componentCount: Object.keys(emergenceState.components).length,
    interactionCount: Object.keys(emergenceState.interactions).length,
  };
}

/**
 * Format status for display
 *
 * @returns {string} Formatted status
 */
function formatStatus() {
  const stats = getStats();
  const analysis = analyzeEmergence();

  const lines = [
    '── EMERGENCE DETECTOR ─────────────────────────────────────',
    `   Components: ${stats.componentCount}`,
    `   Interactions: ${stats.interactionCount}`,
    `   Emergent Patterns: ${stats.activePatterns}`,
    `   Phase Transitions: ${stats.phaseTransitions}`,
  ];

  if (stats.activePatterns > 0) {
    lines.push('');
    lines.push('   By type:');
    for (const [type, count] of Object.entries(analysis.byType)) {
      if (count > 0) {
        lines.push(`   • ${EMERGENCE_TYPES[type].name}: ${count}`);
      }
    }
  }

  if (analysis.largest.length > 0) {
    lines.push('');
    lines.push('   Largest emergent patterns:');
    for (const p of analysis.largest.slice(0, 3)) {
      lines.push(`   • ${p.id}: ${p.components} components (${p.type})`);
    }
  }

  lines.push('');
  lines.push('   *sniff* "More is different." - Anderson');

  return lines.join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  MIN_COMPONENTS,
  EMERGENCE_THRESHOLD,
  EMERGENCE_TYPES,
  HIERARCHY_LEVELS,

  // Core functions
  init,
  registerComponent,
  recordInteraction,

  // Emergence detection
  checkEmergence,
  detectPhaseTransition,

  // Pattern access
  getPatterns,
  getPattern,

  // Analysis
  analyzeEmergence,

  // Stats and display
  getStats,
  formatStatus,
};

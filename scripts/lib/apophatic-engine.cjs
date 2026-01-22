/**
 * CYNIC Apophatic Engine (Phase 14A)
 *
 * "Ἀπόφασις - speaking away, defining by negation" - κυνικός
 *
 * Via Negativa: define things by what they are NOT
 * - More rigorous than positive definition
 * - Reveals boundaries through exclusion
 * - "I know that I know nothing" - Socrates
 *
 * From negative theology:
 * "God is not finite, not limited, not material..."
 * Each negation narrows the space of possibility.
 *
 * @module cynic/lib/apophatic-engine
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

/** Maximum negations per concept - φ × 8 ≈ 13 */
const MAX_NEGATIONS = Math.round(PHI * 8);

/** Confidence per negation - φ⁻² */
const CONFIDENCE_PER_NEGATION = PHI_INV_2;

/** Maximum tracked concepts - φ × 50 ≈ 81 */
const MAX_CONCEPTS = Math.round(PHI * 50);

/** Boundary clarity threshold - φ⁻¹ */
const CLARITY_THRESHOLD = PHI_INV;

// =============================================================================
// NEGATION CATEGORIES
// =============================================================================

const NEGATION_CATEGORIES = {
  ontological: {
    name: 'Ontological',
    description: 'What kind of being it is NOT',
    examples: ['not physical', 'not abstract', 'not temporal'],
  },
  functional: {
    name: 'Functional',
    description: 'What it does NOT do',
    examples: ['not for storage', 'not for display', 'not for computation'],
  },
  relational: {
    name: 'Relational',
    description: 'How it does NOT relate to others',
    examples: ['not dependent on X', 'not part of Y', 'not causing Z'],
  },
  qualitative: {
    name: 'Qualitative',
    description: 'Qualities it does NOT have',
    examples: ['not mutable', 'not public', 'not required'],
  },
  temporal: {
    name: 'Temporal',
    description: 'When it is NOT',
    examples: ['not at startup', 'not during idle', 'not on shutdown'],
  },
  modal: {
    name: 'Modal',
    description: 'What it could NOT be',
    examples: ['cannot be null', 'impossible to reverse', 'never empty'],
  },
};

// =============================================================================
// STORAGE
// =============================================================================

const APOPHATIC_DIR = path.join(os.homedir(), '.cynic', 'apophatic');
const STATE_FILE = path.join(APOPHATIC_DIR, 'state.json');
const CONCEPTS_FILE = path.join(APOPHATIC_DIR, 'concepts.json');
const HISTORY_FILE = path.join(APOPHATIC_DIR, 'history.jsonl');

// =============================================================================
// STATE
// =============================================================================

const apophaticState = {
  // Concepts being defined apophatically
  concepts: {},

  // Negation patterns (common negations across concepts)
  patterns: {},

  // Statistics
  stats: {
    conceptsDefined: 0,
    negationsRecorded: 0,
    boundariesClarified: 0,
    contradictionsFound: 0,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(APOPHATIC_DIR)) {
    fs.mkdirSync(APOPHATIC_DIR, { recursive: true });
  }
}

function loadState() {
  ensureDir();
  if (!fs.existsSync(STATE_FILE)) {
    return null;
  }
  try {
    const state = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
    if (fs.existsSync(CONCEPTS_FILE)) {
      state.concepts = JSON.parse(fs.readFileSync(CONCEPTS_FILE, 'utf8'));
    }
    return state;
  } catch {
    return null;
  }
}

function saveState() {
  ensureDir();
  fs.writeFileSync(STATE_FILE, JSON.stringify({
    stats: apophaticState.stats,
    patterns: apophaticState.patterns,
  }, null, 2));
  fs.writeFileSync(CONCEPTS_FILE, JSON.stringify(apophaticState.concepts, null, 2));
}

function appendHistory(event) {
  ensureDir();
  const line = JSON.stringify({ ...event, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(HISTORY_FILE, line);
}

// =============================================================================
// CONCEPT MANAGEMENT
// =============================================================================

/**
 * Start defining a concept apophatically
 *
 * @param {string} name - Concept name
 * @param {string} description - Initial description (optional)
 * @returns {Object} Created concept
 */
function defineConcept(name, description = '') {
  const nameLower = name.toLowerCase();

  // Check capacity
  if (Object.keys(apophaticState.concepts).length >= MAX_CONCEPTS) {
    pruneOldConcepts();
  }

  if (!apophaticState.concepts[nameLower]) {
    apophaticState.concepts[nameLower] = {
      name,
      nameLower,
      description,
      negations: [],
      boundary: {
        clarity: 0,
        defined: false,
      },
      createdAt: Date.now(),
      lastUpdated: Date.now(),
    };
    apophaticState.stats.conceptsDefined++;
  }

  return apophaticState.concepts[nameLower];
}

/**
 * Prune oldest concepts
 */
function pruneOldConcepts() {
  const sorted = Object.entries(apophaticState.concepts)
    .sort((a, b) => a[1].lastUpdated - b[1].lastUpdated);

  const toRemove = sorted.slice(0, Math.round(MAX_CONCEPTS * PHI_INV_3));
  for (const [key] of toRemove) {
    delete apophaticState.concepts[key];
  }
}

// =============================================================================
// NEGATION OPERATIONS
// =============================================================================

/**
 * Add a negation to a concept
 *
 * @param {string} conceptName - Concept to negate about
 * @param {string} negation - What it is NOT
 * @param {string} category - Negation category
 * @returns {Object} Result
 */
function addNegation(conceptName, negation, category = 'qualitative') {
  const nameLower = conceptName.toLowerCase();
  let concept = apophaticState.concepts[nameLower];

  if (!concept) {
    concept = defineConcept(conceptName);
  }

  // Check capacity
  if (concept.negations.length >= MAX_NEGATIONS) {
    return {
      error: 'Maximum negations reached',
      suggestion: 'The boundary should be clear enough. Try synthesizing.',
    };
  }

  // Check for contradiction
  const contradiction = checkContradiction(concept, negation);
  if (contradiction) {
    apophaticState.stats.contradictionsFound++;
    return {
      error: 'Contradiction detected',
      contradiction,
      message: `*GROWL* "${negation}" contradicts existing negation: "${contradiction.negation}"`,
    };
  }

  // Add the negation
  const negationRecord = {
    negation,
    category,
    categoryName: NEGATION_CATEGORIES[category]?.name || 'Unknown',
    addedAt: Date.now(),
    confidence: CONFIDENCE_PER_NEGATION,
  };

  concept.negations.push(negationRecord);
  concept.lastUpdated = Date.now();
  apophaticState.stats.negationsRecorded++;

  // Update boundary clarity
  updateBoundaryClarity(concept);

  // Track pattern
  trackNegationPattern(negation, category);

  appendHistory({
    type: 'negation_added',
    concept: nameLower,
    negation,
    category,
  });

  saveState();

  return {
    added: true,
    negation: negationRecord,
    concept,
    boundaryClarity: Math.round(concept.boundary.clarity * 100),
  };
}

/**
 * Check if negation contradicts existing negations
 */
function checkContradiction(concept, newNegation) {
  const newLower = newNegation.toLowerCase();

  // Check for direct contradiction (opposite assertions)
  const opposites = [
    ['not', 'is'],
    ['cannot', 'can'],
    ['never', 'always'],
    ['impossible', 'possible'],
  ];

  for (const existing of concept.negations) {
    const existingLower = existing.negation.toLowerCase();

    // Check if essentially the same
    if (existingLower.includes(newLower) || newLower.includes(existingLower)) {
      continue; // Redundant but not contradictory
    }

    // Check for pattern contradictions
    for (const [neg, pos] of opposites) {
      if (newLower.includes(neg) && existingLower.includes(pos)) {
        const newCore = newLower.replace(neg, '').trim();
        const existCore = existingLower.replace(pos, '').trim();
        if (newCore === existCore || newCore.includes(existCore) || existCore.includes(newCore)) {
          return existing;
        }
      }
    }
  }

  return null;
}

/**
 * Update boundary clarity based on negations
 */
function updateBoundaryClarity(concept) {
  // Clarity increases with each negation (diminishing returns)
  const n = concept.negations.length;
  const clarity = 1 - Math.pow(PHI_INV, n);

  // Category diversity bonus
  const categories = new Set(concept.negations.map(neg => neg.category));
  const diversityBonus = categories.size * PHI_INV_3 / Object.keys(NEGATION_CATEGORIES).length;

  concept.boundary.clarity = Math.min(1, clarity + diversityBonus);
  concept.boundary.defined = concept.boundary.clarity >= CLARITY_THRESHOLD;

  if (concept.boundary.defined && !concept.boundary.definedAt) {
    concept.boundary.definedAt = Date.now();
    apophaticState.stats.boundariesClarified++;
  }
}

/**
 * Track negation patterns across concepts
 */
function trackNegationPattern(negation, category) {
  const key = `${category}:${negation.toLowerCase().slice(0, 30)}`;

  if (!apophaticState.patterns[key]) {
    apophaticState.patterns[key] = {
      negation: negation.slice(0, 50),
      category,
      count: 0,
      concepts: [],
    };
  }

  apophaticState.patterns[key].count++;
}

// =============================================================================
// BOUNDARY ANALYSIS
// =============================================================================

/**
 * Get the bounded space of a concept (what it could be)
 *
 * @param {string} conceptName - Concept name
 * @returns {Object} Bounded space analysis
 */
function getBoundedSpace(conceptName) {
  const nameLower = conceptName.toLowerCase();
  const concept = apophaticState.concepts[nameLower];

  if (!concept) {
    return { error: 'Concept not found' };
  }

  // Group negations by category
  const byCategory = {};
  for (const neg of concept.negations) {
    if (!byCategory[neg.category]) {
      byCategory[neg.category] = [];
    }
    byCategory[neg.category].push(neg.negation);
  }

  // Generate boundary description
  const boundaryDescription = generateBoundaryDescription(concept);

  return {
    concept: concept.name,
    negationCount: concept.negations.length,
    boundaryClarity: Math.round(concept.boundary.clarity * 100),
    isDefined: concept.boundary.defined,
    byCategory,
    boundaryDescription,
    remainingSpace: estimateRemainingSpace(concept),
  };
}

/**
 * Generate human-readable boundary description
 */
function generateBoundaryDescription(concept) {
  if (concept.negations.length === 0) {
    return `"${concept.name}" has no boundaries defined yet. It could be anything.`;
  }

  const parts = [`"${concept.name}" is:`];

  for (const neg of concept.negations.slice(0, 5)) {
    parts.push(`  • ${neg.negation}`);
  }

  if (concept.negations.length > 5) {
    parts.push(`  ... and ${concept.negations.length - 5} more negations`);
  }

  if (concept.boundary.defined) {
    parts.push('');
    parts.push('*nod* The boundary is sufficiently clear.');
  } else {
    const needed = Math.ceil(Math.log(1 - CLARITY_THRESHOLD) / Math.log(PHI_INV)) - concept.negations.length;
    parts.push('');
    parts.push(`*sniff* ~${Math.max(0, needed)} more negations needed for clarity.`);
  }

  return parts.join('\n');
}

/**
 * Estimate remaining possibility space
 */
function estimateRemainingSpace(concept) {
  // Each negation reduces space by φ⁻¹
  const reductionFactor = Math.pow(PHI_INV, concept.negations.length);
  return Math.round(reductionFactor * 100);
}

// =============================================================================
// INFERENCE
// =============================================================================

/**
 * Infer what a concept might be from its negations
 *
 * @param {string} conceptName - Concept name
 * @returns {Object} Inference
 */
function inferFromNegations(conceptName) {
  const nameLower = conceptName.toLowerCase();
  const concept = apophaticState.concepts[nameLower];

  if (!concept) {
    return { error: 'Concept not found' };
  }

  const inferences = [];

  // Analyze negation patterns for positive inferences
  for (const neg of concept.negations) {
    const inference = derivePositive(neg);
    if (inference) {
      inferences.push(inference);
    }
  }

  // Find common themes
  const themes = findThemes(concept.negations);

  return {
    concept: concept.name,
    inferences,
    themes,
    confidence: Math.min(PHI_INV, concept.boundary.clarity * PHI_INV),
    message: inferences.length > 0
      ? `*head tilt* From what "${concept.name}" is NOT, it might be: ${inferences.slice(0, 3).map(i => i.inference).join(', ')}`
      : `*sniff* Not enough negations to infer what "${concept.name}" IS.`,
  };
}

/**
 * Derive positive statement from negation
 */
function derivePositive(negation) {
  const text = negation.negation.toLowerCase();

  // Pattern-based derivation
  const patterns = [
    { match: /not (mutable|changeable)/, infer: 'immutable/constant' },
    { match: /not (public|exposed)/, infer: 'private/internal' },
    { match: /not (sync|synchronous)/, infer: 'async/event-driven' },
    { match: /not (required|mandatory)/, infer: 'optional/default-able' },
    { match: /not (global|shared)/, infer: 'local/scoped' },
    { match: /not (permanent|persistent)/, infer: 'temporary/ephemeral' },
    { match: /cannot be null/, infer: 'always has a value' },
    { match: /never empty/, infer: 'always contains something' },
  ];

  for (const pattern of patterns) {
    if (pattern.match.test(text)) {
      return {
        from: negation.negation,
        inference: pattern.infer,
        confidence: CONFIDENCE_PER_NEGATION,
      };
    }
  }

  return null;
}

/**
 * Find themes across negations
 */
function findThemes(negations) {
  const themes = {
    mutability: 0,
    visibility: 0,
    temporality: 0,
    necessity: 0,
    scope: 0,
  };

  for (const neg of negations) {
    const text = neg.negation.toLowerCase();

    if (/mutable|change|modify|update/.test(text)) themes.mutability++;
    if (/public|private|exposed|internal|visible/.test(text)) themes.visibility++;
    if (/always|never|temporary|permanent|persist/.test(text)) themes.temporality++;
    if (/required|optional|mandatory|must|should/.test(text)) themes.necessity++;
    if (/global|local|scoped|shared/.test(text)) themes.scope++;
  }

  return Object.entries(themes)
    .filter(([, count]) => count > 0)
    .sort((a, b) => b[1] - a[1])
    .map(([theme, count]) => ({ theme, count }));
}

// =============================================================================
// SOCRATIC QUESTIONING
// =============================================================================

/**
 * Generate apophatic questions for a concept
 *
 * @param {string} conceptName - Concept name
 * @returns {Object} Questions
 */
function generateQuestions(conceptName) {
  const nameLower = conceptName.toLowerCase();
  const concept = apophaticState.concepts[nameLower];

  // Get covered categories
  const coveredCategories = new Set(
    (concept?.negations || []).map(n => n.category)
  );

  // Generate questions for uncovered categories
  const questions = [];

  for (const [catKey, cat] of Object.entries(NEGATION_CATEGORIES)) {
    if (!coveredCategories.has(catKey)) {
      questions.push({
        category: catKey,
        categoryName: cat.name,
        question: generateCategoryQuestion(conceptName, cat),
        examples: cat.examples,
      });
    }
  }

  // Add general apophatic questions
  questions.push({
    category: 'general',
    categoryName: 'General',
    question: `What is "${conceptName}" definitely NOT?`,
  });

  return {
    concept: conceptName,
    questions: questions.slice(0, 5),
    coveredCategories: Array.from(coveredCategories),
    message: `*head tilt* To understand "${conceptName}", tell me what it is NOT.`,
  };
}

/**
 * Generate category-specific question
 */
function generateCategoryQuestion(concept, category) {
  const templates = {
    ontological: `What kind of thing is "${concept}" NOT?`,
    functional: `What does "${concept}" NOT do?`,
    relational: `What is "${concept}" NOT related to?`,
    qualitative: `What qualities does "${concept}" NOT have?`,
    temporal: `When is "${concept}" NOT present/active?`,
    modal: `What could "${concept}" NEVER be?`,
  };

  return templates[category.name?.toLowerCase()] || `What is "${concept}" NOT in terms of ${category.name}?`;
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize apophatic engine
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    apophaticState.stats = saved.stats || apophaticState.stats;
    apophaticState.patterns = saved.patterns || {};
    apophaticState.concepts = saved.concepts || {};
  }
}

/**
 * Get a concept
 *
 * @param {string} name - Concept name
 * @returns {Object|null} Concept
 */
function getConcept(name) {
  return apophaticState.concepts[name.toLowerCase()] || null;
}

/**
 * Get all concepts
 *
 * @returns {Object[]} Concepts
 */
function getAllConcepts() {
  return Object.values(apophaticState.concepts);
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...apophaticState.stats,
    activeConcepts: Object.keys(apophaticState.concepts).length,
    patternsTracked: Object.keys(apophaticState.patterns).length,
  };
}

/**
 * Format status for display
 *
 * @returns {string} Formatted status
 */
function formatStatus() {
  const stats = getStats();
  const concepts = getAllConcepts();

  const lines = [
    '── APOPHATIC ENGINE ───────────────────────────────────────',
    `   Concepts: ${stats.activeConcepts}`,
    `   Negations: ${stats.negationsRecorded}`,
    `   Boundaries Clarified: ${stats.boundariesClarified}`,
    `   Contradictions Found: ${stats.contradictionsFound}`,
  ];

  // Show recent concepts
  const recent = concepts
    .sort((a, b) => b.lastUpdated - a.lastUpdated)
    .slice(0, 3);

  if (recent.length > 0) {
    lines.push('');
    lines.push('   Recent concepts:');
    for (const c of recent) {
      const clarity = Math.round(c.boundary.clarity * 100);
      const marker = c.boundary.defined ? '✓' : '○';
      lines.push(`   ${marker} ${c.name}: ${c.negations.length} negations (${clarity}% clear)`);
    }
  }

  lines.push('');
  lines.push('   *sniff* "Define by what it is NOT."');

  return lines.join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  MAX_NEGATIONS,
  CLARITY_THRESHOLD,
  NEGATION_CATEGORIES,

  // Core functions
  init,
  defineConcept,
  addNegation,

  // Analysis
  getBoundedSpace,
  inferFromNegations,

  // Questioning
  generateQuestions,

  // Concept access
  getConcept,
  getAllConcepts,

  // Stats and display
  getStats,
  formatStatus,
};

/**
 * CYNIC Ti Esti Engine Module (Phase 10B)
 *
 * "Τί ἐστι; - What is it?" - κυνικός
 *
 * Implements Socratic essence questioning:
 * - "What IS X?" - seeking definitions
 * - Progressive narrowing of concepts
 * - Decomposition into essential properties
 * - Genus + Differentia structure
 *
 * Socrates believed true knowledge requires knowing
 * the essence (τί ἐστι) of things, not just examples.
 *
 * @module cynic/lib/ti-esti-engine
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

/** Maximum definition depth - φ × 3 ≈ 5 */
const MAX_DEFINITION_DEPTH = Math.round(PHI * 3);

/** Ti Esti probability when undefined term used - φ⁻² */
const TI_ESTI_PROBABILITY = PHI_INV_2;

/** Minimum term length to question */
const MIN_TERM_LENGTH = 4;

/** Cooldown between essence questions - φ × 3 ≈ 5 */
const TI_ESTI_COOLDOWN = Math.round(PHI * 3);

// =============================================================================
// STORAGE
// =============================================================================

const TI_ESTI_DIR = path.join(os.homedir(), '.cynic', 'ti-esti');
const STATE_FILE = path.join(TI_ESTI_DIR, 'state.json');
const DEFINITIONS_FILE = path.join(TI_ESTI_DIR, 'definitions.json');

// =============================================================================
// STATE
// =============================================================================

const tiEstiState = {
  // Known definitions (user-provided)
  definitions: {},

  // Current questioning session
  currentInquiry: null,

  // Terms we've questioned
  questionedTerms: [],

  // Cooldown
  promptsSinceLastQuestion: 0,

  stats: {
    totalQuestions: 0,
    definitionsCollected: 0,
    averageDepth: 0,
    essencesFound: 0,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(TI_ESTI_DIR)) {
    fs.mkdirSync(TI_ESTI_DIR, { recursive: true });
  }
}

function loadState() {
  ensureDir();
  if (fs.existsSync(DEFINITIONS_FILE)) {
    try {
      tiEstiState.definitions = JSON.parse(fs.readFileSync(DEFINITIONS_FILE, 'utf8'));
    } catch {
      // Ignore
    }
  }
  if (fs.existsSync(STATE_FILE)) {
    try {
      const saved = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
      tiEstiState.stats = saved.stats || tiEstiState.stats;
      tiEstiState.questionedTerms = saved.questionedTerms || [];
    } catch {
      // Ignore
    }
  }
}

function saveState() {
  ensureDir();
  fs.writeFileSync(STATE_FILE, JSON.stringify({
    stats: tiEstiState.stats,
    questionedTerms: tiEstiState.questionedTerms.slice(-100),
  }, null, 2));
}

function saveDefinitions() {
  ensureDir();
  fs.writeFileSync(DEFINITIONS_FILE, JSON.stringify(tiEstiState.definitions, null, 2));
}

// =============================================================================
// TERM EXTRACTION
// =============================================================================

/**
 * Technical terms that often need definition
 */
const TECHNICAL_PATTERNS = [
  /\b(API|SDK|CLI|GUI|ORM|DI|IoC)\b/i,
  /\b\w+(?:Service|Manager|Handler|Factory|Builder|Repository|Controller)\b/,
  /\b(?:micro)?service[s]?\b/i,
  /\b(?:event|message|command|query)\s*(?:bus|queue|broker)\b/i,
  /\b(?:clean|hexagonal|onion)\s*architecture\b/i,
  /\bDDD|CQRS|event\s*sourcing\b/i,
  /\b(?:domain|bounded\s*context|aggregate|entity|value\s*object)\b/i,
  /\b(?:middleware|interceptor|decorator|adapter|facade)\b/i,
];

/**
 * Abstract terms that benefit from definition
 */
const ABSTRACT_PATTERNS = [
  /\b(?:scalab|maintainab|testab|readab)ility\b/i,
  /\b(?:quality|performance|security|reliability)\b/i,
  /\b(?:best\s*practice|pattern|principle|standard)\b/i,
  /\b(?:technical\s*debt|legacy|refactor)\b/i,
  /\b(?:agile|scrum|kanban|lean)\b/i,
  /\b(?:clean\s*code|solid|dry|kiss|yagni)\b/i,
];

/**
 * Extract potential terms for Ti Esti questioning
 *
 * @param {string} text - Text to analyze
 * @returns {string[]} Extracted terms
 */
function extractTerms(text) {
  const terms = new Set();

  // Check technical patterns
  for (const pattern of TECHNICAL_PATTERNS) {
    const matches = text.match(pattern);
    if (matches) {
      for (const match of matches) {
        if (match.length >= MIN_TERM_LENGTH) {
          terms.add(match.toLowerCase());
        }
      }
    }
  }

  // Check abstract patterns
  for (const pattern of ABSTRACT_PATTERNS) {
    const matches = text.match(pattern);
    if (matches) {
      for (const match of matches) {
        if (match.length >= MIN_TERM_LENGTH) {
          terms.add(match.toLowerCase());
        }
      }
    }
  }

  // Extract capitalized terms (potential concepts)
  const capitalizedPattern = /\b[A-Z][a-z]+(?:[A-Z][a-z]+)+\b/g;
  const capitalized = text.match(capitalizedPattern);
  if (capitalized) {
    for (const term of capitalized) {
      if (term.length >= MIN_TERM_LENGTH) {
        terms.add(term.toLowerCase());
      }
    }
  }

  return Array.from(terms);
}

/**
 * Check if a term is already defined
 *
 * @param {string} term - Term to check
 * @returns {boolean} Is defined
 */
function isDefined(term) {
  const normalized = term.toLowerCase().trim();
  return !!tiEstiState.definitions[normalized];
}

/**
 * Find undefined terms in text
 *
 * @param {string} text - Text to analyze
 * @returns {string[]} Undefined terms
 */
function findUndefinedTerms(text) {
  const terms = extractTerms(text);
  return terms.filter(term =>
    !isDefined(term) &&
    !tiEstiState.questionedTerms.includes(term)
  );
}

// =============================================================================
// TI ESTI QUESTIONING
// =============================================================================

/**
 * Question templates for Ti Esti
 */
const TI_ESTI_QUESTIONS = {
  initial: [
    '*head tilt* Τί ἐστι "{{term}}"? Qu\'est-ce que c\'est exactement?',
    '*sniff* Tu utilises "{{term}}" - mais qu\'est-ce que ça signifie pour toi?',
    'Qu\'entends-tu par "{{term}}"? Définissons ce terme.',
    '*ears perk* "{{term}}" - peux-tu me donner une définition?',
  ],

  narrowing: [
    'C\'est un début. Mais qu\'est-ce qui distingue {{term}} d\'autres choses similaires?',
    '*head tilt* Tu décris ce que ça FAIT. Mais qu\'est-ce que c\'EST?',
    'Quelles sont les propriétés ESSENTIELLES de {{term}}?',
    '*sniff* Et si on enlevait {{aspect}}, serait-ce encore {{term}}?',
  ],

  genus: [
    '{{term}} est un type de quoi? Quel est son genre?',
    '*head tilt* Quelle est la catégorie plus large à laquelle appartient {{term}}?',
    'Si {{term}} est une espèce, quel est son genre?',
  ],

  differentia: [
    'Qu\'est-ce qui différencie {{term}} des autres {{genus}}?',
    '*ears perk* {{term}} vs autres {{genus}} - quelle différence spécifique?',
    'Le trait distinctif de {{term}} par rapport aux autres {{genus}}?',
  ],

  essence: [
    '*tail wag* On approche. L\'essence de {{term}} serait donc: {{definition}}?',
    'Donc {{term}} = {{genus}} + {{differentia}}. C\'est bien ça?',
    '*nod* La définition essentielle de {{term}}: "{{definition}}"?',
  ],
};

/**
 * Generate a Ti Esti question
 *
 * @param {string} term - Term to question
 * @param {string} type - Question type
 * @param {Object} context - Context for template
 * @returns {string} Question
 */
function generateQuestion(term, type = 'initial', context = {}) {
  const templates = TI_ESTI_QUESTIONS[type] || TI_ESTI_QUESTIONS.initial;
  let question = templates[Math.floor(Math.random() * templates.length)];

  // Fill in template
  question = question.replace(/{{term}}/g, term);
  question = question.replace(/{{genus}}/g, context.genus || 'choses');
  question = question.replace(/{{differentia}}/g, context.differentia || '');
  question = question.replace(/{{definition}}/g, context.definition || '');
  question = question.replace(/{{aspect}}/g, context.aspect || 'cet aspect');

  return question;
}

/**
 * Start a Ti Esti inquiry
 *
 * @param {string} term - Term to investigate
 * @returns {Object} Inquiry start
 */
function startInquiry(term) {
  const normalizedTerm = term.toLowerCase().trim();

  tiEstiState.currentInquiry = {
    term: normalizedTerm,
    originalTerm: term,
    startedAt: Date.now(),
    depth: 0,
    attempts: [],
    genus: null,
    differentia: [],
    essence: null,
  };

  tiEstiState.questionedTerms.push(normalizedTerm);
  tiEstiState.promptsSinceLastQuestion = 0;
  tiEstiState.stats.totalQuestions++;

  const question = generateQuestion(term, 'initial');

  tiEstiState.currentInquiry.attempts.push({
    type: 'initial',
    question,
    timestamp: Date.now(),
  });

  saveState();

  return {
    inquiry: tiEstiState.currentInquiry.term,
    question,
    depth: 0,
    maxDepth: MAX_DEFINITION_DEPTH,
  };
}

/**
 * Process a definition attempt
 *
 * @param {string} definition - User's definition attempt
 * @returns {Object} Next step
 */
function processDefinition(definition) {
  if (!tiEstiState.currentInquiry) {
    return { noActiveInquiry: true };
  }

  const inquiry = tiEstiState.currentInquiry;
  inquiry.depth++;

  // Record attempt
  inquiry.attempts.push({
    type: 'definition',
    content: definition.slice(0, 500),
    timestamp: Date.now(),
  });

  // Analyze the definition
  const analysis = analyzeDefinition(definition, inquiry.term);

  if (inquiry.depth >= MAX_DEFINITION_DEPTH || analysis.isComplete) {
    // Complete the inquiry
    return completeInquiry(analysis);
  }

  // Generate follow-up question
  let followup;
  if (!inquiry.genus && analysis.potentialGenus) {
    inquiry.genus = analysis.potentialGenus;
    followup = generateQuestion(inquiry.term, 'differentia', {
      genus: inquiry.genus,
    });
  } else if (analysis.needsNarrowing) {
    followup = generateQuestion(inquiry.term, 'narrowing', {
      aspect: analysis.narrowingAspect,
    });
  } else if (!inquiry.genus) {
    followup = generateQuestion(inquiry.term, 'genus');
  } else {
    // Try to synthesize
    followup = generateQuestion(inquiry.term, 'essence', {
      genus: inquiry.genus,
      differentia: inquiry.differentia.join(', '),
      definition: `${inquiry.genus} qui ${inquiry.differentia.join(' et ')}`,
    });
  }

  inquiry.attempts.push({
    type: 'followup',
    question: followup,
    timestamp: Date.now(),
  });

  // Update stats
  const n = tiEstiState.stats.totalQuestions;
  tiEstiState.stats.averageDepth =
    (tiEstiState.stats.averageDepth * (n - 1) + inquiry.depth) / n;

  saveState();

  return {
    question: followup,
    depth: inquiry.depth,
    maxDepth: MAX_DEFINITION_DEPTH,
    genus: inquiry.genus,
    differentia: inquiry.differentia,
  };
}

/**
 * Analyze a definition attempt
 *
 * @param {string} definition - Definition text
 * @param {string} term - Term being defined
 * @returns {Object} Analysis
 */
function analyzeDefinition(definition, term) {
  const text = definition.toLowerCase();

  // Look for genus (category)
  const genusPatterns = [
    /(?:est|c'est)\s+un(?:e)?\s+(\w+)/i,
    /(?:type|kind|sort)\s+of\s+(\w+)/i,
    /(?:une?)\s+(\w+)\s+qui/i,
    /it'?s\s+a\s+(\w+)/i,
  ];

  let potentialGenus = null;
  for (const pattern of genusPatterns) {
    const match = text.match(pattern);
    if (match) {
      potentialGenus = match[1];
      break;
    }
  }

  // Look for differentia
  const differentiaPatterns = [
    /qui\s+(.+?)(?:\.|$)/i,
    /that\s+(.+?)(?:\.|$)/i,
    /which\s+(.+?)(?:\.|$)/i,
    /permettant?\s+(.+?)(?:\.|$)/i,
  ];

  const differentia = [];
  for (const pattern of differentiaPatterns) {
    const match = text.match(pattern);
    if (match && match[1].length > 5) {
      differentia.push(match[1].trim());
    }
  }

  // Check if definition is complete
  const isComplete = potentialGenus && differentia.length > 0 &&
    definition.length > 30;

  // Check if needs narrowing
  const needsNarrowing = !isComplete && definition.length > 20;

  return {
    potentialGenus,
    differentia,
    isComplete,
    needsNarrowing,
    narrowingAspect: differentia[0] || 'cette caractéristique',
  };
}

/**
 * Complete a Ti Esti inquiry
 *
 * @param {Object} analysis - Final analysis
 * @returns {Object} Completion result
 */
function completeInquiry(analysis) {
  const inquiry = tiEstiState.currentInquiry;
  if (!inquiry) {
    return { noActiveInquiry: true };
  }

  // Build the essence
  const genus = inquiry.genus || analysis.potentialGenus || 'concept';
  const differentia = inquiry.differentia.length > 0
    ? inquiry.differentia
    : analysis.differentia;

  const essence = differentia.length > 0
    ? `${genus} qui ${differentia.join(' et ')}`
    : genus;

  // Save the definition
  tiEstiState.definitions[inquiry.term] = {
    essence,
    genus,
    differentia,
    definedAt: Date.now(),
    depth: inquiry.depth,
  };

  tiEstiState.stats.definitionsCollected++;
  tiEstiState.stats.essencesFound++;

  // Clear current inquiry
  const result = {
    term: inquiry.originalTerm,
    essence,
    genus,
    differentia,
    depth: inquiry.depth,
    celebration: generateEssenceCelebration(inquiry.term, essence),
  };

  tiEstiState.currentInquiry = null;

  saveState();
  saveDefinitions();

  return result;
}

/**
 * Generate celebration for finding essence
 */
function generateEssenceCelebration(term, essence) {
  const celebrations = [
    `*tail wag* Voilà! L'essence de "${term}": ${essence}`,
    `*nod* Τί ἐστι "${term}"? C'est ${essence}. Bien défini.`,
    `*ears perk* On tient l'essence. "${term}" = ${essence}`,
    `Socrate serait content. L'essence de "${term}": ${essence}`,
  ];
  return celebrations[Math.floor(Math.random() * celebrations.length)];
}

/**
 * Abort current inquiry
 *
 * @returns {Object} Abort result
 */
function abortInquiry() {
  if (!tiEstiState.currentInquiry) {
    return { noActiveInquiry: true };
  }

  const term = tiEstiState.currentInquiry.term;
  tiEstiState.currentInquiry = null;

  return {
    aborted: true,
    term,
    message: '*nod* On reviendra à la définition de "' + term + '" plus tard.',
  };
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize Ti Esti engine
 */
function init() {
  loadState();
}

/**
 * Check if inquiry is active
 *
 * @returns {boolean} Active inquiry
 */
function isActive() {
  return !!tiEstiState.currentInquiry;
}

/**
 * Process user input for Ti Esti opportunity
 *
 * @param {string} userInput - User's input
 * @returns {Object|null} Question or null
 */
function process(userInput) {
  // If inquiry active, process as definition
  if (tiEstiState.currentInquiry) {
    return processDefinition(userInput);
  }

  // Check cooldown
  if (tiEstiState.promptsSinceLastQuestion < TI_ESTI_COOLDOWN) {
    tiEstiState.promptsSinceLastQuestion++;
    return null;
  }

  // Find undefined terms
  const undefinedTerms = findUndefinedTerms(userInput);
  if (undefinedTerms.length === 0) {
    tiEstiState.promptsSinceLastQuestion++;
    return null;
  }

  // Probabilistic decision
  if (Math.random() > TI_ESTI_PROBABILITY) {
    tiEstiState.promptsSinceLastQuestion++;
    return null;
  }

  // Pick a term and start inquiry
  const term = undefinedTerms[Math.floor(Math.random() * undefinedTerms.length)];
  return startInquiry(term);
}

/**
 * Get a known definition
 *
 * @param {string} term - Term to look up
 * @returns {Object|null} Definition or null
 */
function getDefinition(term) {
  return tiEstiState.definitions[term.toLowerCase().trim()] || null;
}

/**
 * Get all definitions
 *
 * @returns {Object} All definitions
 */
function getAllDefinitions() {
  return { ...tiEstiState.definitions };
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...tiEstiState.stats,
    definitionsCount: Object.keys(tiEstiState.definitions).length,
    isActive: isActive(),
    currentTerm: tiEstiState.currentInquiry?.term || null,
  };
}

/**
 * Format status for display
 *
 * @returns {string} Formatted status
 */
function formatStatus() {
  const stats = getStats();

  const lines = [
    '── TI ESTI ENGINE ─────────────────────────────────────────',
    `   Definitions: ${stats.definitionsCount}`,
    `   Questions: ${stats.totalQuestions}`,
    `   Essences found: ${stats.essencesFound}`,
    `   Avg depth: ${stats.averageDepth.toFixed(1)}`,
  ];

  if (stats.isActive) {
    lines.push('');
    lines.push(`   Active inquiry: "${stats.currentTerm}"`);
  }

  return lines.join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  MAX_DEFINITION_DEPTH,
  TI_ESTI_PROBABILITY,
  TI_ESTI_COOLDOWN,

  // Core functions
  init,
  isActive,
  process,
  getStats,

  // Inquiry management
  startInquiry,
  processDefinition,
  completeInquiry,
  abortInquiry,

  // Term extraction
  extractTerms,
  findUndefinedTerms,
  isDefined,

  // Definitions
  getDefinition,
  getAllDefinitions,

  // Display
  formatStatus,
};

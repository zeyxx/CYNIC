/**
 * CYNIC Physis Detector Module (Phase 7C)
 *
 * "Κατὰ φύσιν ζῆν - vivre selon la nature" - κυνικός
 *
 * Distinguishes between:
 * - φύσις (physis) - natural patterns that emerge from real needs
 * - νόμος (nomos) - conventions, arbitrary rules, social constructs
 *
 * The Cynics believed in living according to nature (physis)
 * and challenging conventions (nomos) that serve no real purpose.
 *
 * @module cynic/lib/physis-detector
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// Import φ constants
const phiMath = require('./phi-math.cjs');
const { PHI_INV, PHI_INV_2, PHI_INV_3 } = phiMath;

// =============================================================================
// CONSTANTS (φ-derived)
// =============================================================================

/** Confidence threshold for challenging conventions - φ⁻¹ */
const CHALLENGE_THRESHOLD = PHI_INV;

/** Minimum evidence before classifying - φ⁻² */
const MIN_EVIDENCE = PHI_INV_2;

/** Natural pattern weight multiplier - φ */
const NATURAL_WEIGHT = phiMath.PHI;

// =============================================================================
// STORAGE
// =============================================================================

const PHYSIS_DIR = path.join(os.homedir(), '.cynic', 'physis');
const STATE_FILE = path.join(PHYSIS_DIR, 'state.json');
const PATTERNS_FILE = path.join(PHYSIS_DIR, 'patterns.json');

// =============================================================================
// STATE
// =============================================================================

const physisState = {
  patterns: [],           // Detected patterns (physis or nomos)
  challengedConventions: [], // Conventions we've challenged
  stats: {
    physisDetected: 0,
    nomosDetected: 0,
    conventionsChallenged: 0,
    naturalPatternsFound: 0,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(PHYSIS_DIR)) {
    fs.mkdirSync(PHYSIS_DIR, { recursive: true });
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
    stats: physisState.stats,
    patterns: physisState.patterns.slice(-100), // Keep last 100
  }, null, 2));
}

// =============================================================================
// PATTERN CLASSIFICATION
// =============================================================================

/**
 * Indicators of PHYSIS (natural patterns)
 * These emerge from real technical needs
 */
const PHYSIS_INDICATORS = {
  // Code patterns that serve real purposes
  codePatterns: {
    patterns: [
      /error handling|try.?catch|\.catch\(/, // Error handling is natural
      /async\/await|promise/i,               // Async patterns
      /validation|sanitize/i,                // Input validation
      /logging|debug|trace/i,                // Observability
      /cache|memoize/i,                      // Performance optimization
      /retry|backoff|circuit.?breaker/i,     // Resilience
    ],
    weight: 0.8,
    rationale: 'Addresses real technical need',
  },

  // Structural patterns that emerge naturally
  structural: {
    patterns: [
      /separation.?of.?concerns/i,
      /single.?responsibility/i,
      /dependency.?injection/i,
      /interface|abstraction/i,
      /modular|decoupled/i,
    ],
    weight: 0.7,
    rationale: 'Natural complexity management',
  },

  // User-centric patterns
  userCentric: {
    patterns: [
      /user.?experience|UX/i,
      /accessibility|a11y/i,
      /performance|speed|latency/i,
      /security|auth|encrypt/i,
      /privacy|GDPR|consent/i,
    ],
    weight: 0.9,
    rationale: 'Serves real human needs',
  },
};

/**
 * Indicators of NOMOS (conventional patterns)
 * These are arbitrary rules or social constructs
 */
const NOMOS_INDICATORS = {
  // Arbitrary style rules
  styleRules: {
    patterns: [
      /tabs.?vs.?spaces/i,
      /semicolons|no.?semicolons/i,
      /single.?quotes|double.?quotes/i,
      /trailing.?comma/i,
      /max.?line.?length/i,
      /indent.?size/i,
    ],
    weight: 0.9,
    rationale: 'Arbitrary style preference',
  },

  // Naming conventions
  namingConventions: {
    patterns: [
      /camelCase|PascalCase|snake_case/i,
      /naming.?convention/i,
      /prefix|suffix.*naming/i,
      /hungarian.?notation/i,
    ],
    weight: 0.7,
    rationale: 'Conventional naming (can vary)',
  },

  // Process conventions
  processConventions: {
    patterns: [
      /code.?review.*required/i,
      /commit.?message.?format/i,
      /branch.?naming/i,
      /PR.?template/i,
      /mandatory.*meeting/i,
      /standup|scrum.?ceremony/i,
    ],
    weight: 0.6,
    rationale: 'Process convention (may or may not help)',
  },

  // Arbitrary metrics
  arbitraryMetrics: {
    patterns: [
      /100%.?coverage/i,
      /lines.?of.?code/i,
      /cyclomatic.?complexity.*\d/i,
      /story.?points/i,
      /velocity/i,
    ],
    weight: 0.8,
    rationale: 'Arbitrary metric (Goodhart\'s law risk)',
  },

  // Cargo cult patterns
  cargoCult: {
    patterns: [
      /best.?practice/i,        // Often cargo cult
      /industry.?standard/i,    // Appeal to authority
      /everyone.?does.?it/i,    // Social proof
      /we.?ve.?always/i,        // Tradition
      /the.?right.?way/i,       // Dogma
    ],
    weight: 0.85,
    rationale: 'Potential cargo cult pattern',
  },
};

/**
 * Classify a pattern as physis or nomos
 * @param {string} description - Pattern description
 * @param {Object} context - Additional context
 * @returns {Object} Classification result
 */
function classifyPattern(description, context = {}) {
  const text = description.toLowerCase();

  let physisScore = 0;
  let nomosScore = 0;
  const physisReasons = [];
  const nomosReasons = [];

  // Check physis indicators
  for (const [category, config] of Object.entries(PHYSIS_INDICATORS)) {
    for (const pattern of config.patterns) {
      if (pattern.test(text)) {
        physisScore += config.weight;
        physisReasons.push({
          category,
          rationale: config.rationale,
        });
        break;
      }
    }
  }

  // Check nomos indicators
  for (const [category, config] of Object.entries(NOMOS_INDICATORS)) {
    for (const pattern of config.patterns) {
      if (pattern.test(text)) {
        nomosScore += config.weight;
        nomosReasons.push({
          category,
          rationale: config.rationale,
        });
        break;
      }
    }
  }

  // Context adjustments
  if (context.hasUserImpact) physisScore += 0.3;
  if (context.isTeamPreference) nomosScore += 0.3;
  if (context.hasSecurityImplication) physisScore += 0.5;
  if (context.isStyleOnly) nomosScore += 0.4;

  // Calculate confidence
  const total = physisScore + nomosScore;
  const confidence = total > 0 ? Math.max(physisScore, nomosScore) / total : 0.5;

  // Determine classification
  let classification;
  if (physisScore > nomosScore * NATURAL_WEIGHT) {
    classification = 'PHYSIS';
  } else if (nomosScore > physisScore * NATURAL_WEIGHT) {
    classification = 'NOMOS';
  } else {
    classification = 'MIXED';
  }

  return {
    classification,
    confidence,
    physisScore,
    nomosScore,
    physisReasons,
    nomosReasons,
    shouldChallenge: classification === 'NOMOS' && confidence >= CHALLENGE_THRESHOLD,
  };
}

// =============================================================================
// CONVENTION CHALLENGING
// =============================================================================

/**
 * Generate a challenge for a convention
 * @param {string} convention - The convention being challenged
 * @param {Object} classification - Classification result
 * @returns {Object} Challenge
 */
function generateChallenge(convention, classification) {
  const challenges = {
    styleRules: [
      'Est-ce que ce style apporte une valeur réelle ou juste de la cohérence cosmétique?',
      'Les développeurs seraient-ils vraiment moins productifs sans cette règle?',
      'Cette règle existe-t-elle pour des raisons historiques obsolètes?',
    ],
    namingConventions: [
      'Cette convention de nommage aide-t-elle vraiment la compréhension?',
      'Pourrait-on laisser les équipes choisir leur style?',
      'Le plus important n\'est-il pas la cohérence interne au projet?',
    ],
    processConventions: [
      'Ce processus apporte-t-il plus de valeur que de friction?',
      'Pourrait-on obtenir le même résultat plus simplement?',
      'Cette règle traite-t-elle le symptôme ou la cause?',
    ],
    arbitraryMetrics: [
      'Cette métrique mesure-t-elle vraiment ce qui compte?',
      'Risque-t-on d\'optimiser pour la métrique au détriment du but?',
      'Goodhart\'s law: "Quand une mesure devient un objectif, elle cesse d\'être une bonne mesure"',
    ],
    cargoCult: [
      'Comprenons-nous pourquoi cette pratique existe?',
      '"Best practice" pour qui et dans quel contexte?',
      'A-t-on vérifié que ça fonctionne pour notre contexte spécifique?',
    ],
  };

  // Find relevant challenge category
  const mainReason = classification.nomosReasons[0];
  const category = mainReason?.category || 'cargoCult';
  const categoryQuestions = challenges[category] || challenges.cargoCult;

  const question = categoryQuestions[Math.floor(Math.random() * categoryQuestions.length)];

  physisState.stats.conventionsChallenged++;

  return {
    convention,
    question,
    category,
    rationale: mainReason?.rationale || 'Convention arbitraire',
    confidence: classification.confidence,
    timestamp: Date.now(),
  };
}

/**
 * Suggest a natural alternative to a convention
 * @param {Object} classification - Classification result
 * @returns {string|null} Natural alternative suggestion
 */
function suggestNaturalAlternative(classification) {
  const alternatives = {
    styleRules: 'Laissez l\'équipe décider, ou utilisez un formatter automatique sans débat.',
    namingConventions: 'Favorisez la clarté sur la convention. Le nom le plus descriptif gagne.',
    processConventions: 'Identifiez le problème réel et adressez-le directement.',
    arbitraryMetrics: 'Mesurez l\'impact utilisateur réel, pas les proxies.',
    cargoCult: 'Expérimentez et mesurez. Gardez ce qui fonctionne pour vous.',
  };

  const mainReason = classification.nomosReasons[0];
  return alternatives[mainReason?.category] || null;
}

// =============================================================================
// NATURAL PATTERN DISCOVERY
// =============================================================================

/**
 * Detect natural patterns in code or conversation
 * @param {string} content - Content to analyze
 * @returns {Object[]} Detected natural patterns
 */
function detectNaturalPatterns(content) {
  const patterns = [];
  const text = content.toLowerCase();

  // Check for natural patterns
  for (const [category, config] of Object.entries(PHYSIS_INDICATORS)) {
    for (const pattern of config.patterns) {
      if (pattern.test(text)) {
        patterns.push({
          category,
          rationale: config.rationale,
          match: text.match(pattern)?.[0],
          confidence: config.weight,
        });
      }
    }
  }

  if (patterns.length > 0) {
    physisState.stats.naturalPatternsFound += patterns.length;
  }

  return patterns;
}

/**
 * Record a detected pattern
 * @param {Object} pattern - Pattern to record
 */
function recordPattern(pattern) {
  physisState.patterns.push({
    ...pattern,
    timestamp: Date.now(),
  });

  if (pattern.classification === 'PHYSIS') {
    physisState.stats.physisDetected++;
  } else if (pattern.classification === 'NOMOS') {
    physisState.stats.nomosDetected++;
  }

  saveState();
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize the Physis detector
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    physisState.stats = saved.stats || physisState.stats;
    physisState.patterns = saved.patterns || [];
  }
}

/**
 * Analyze a practice/pattern
 * @param {string} description - Practice description
 * @param {Object} context - Additional context
 * @returns {Object} Analysis result
 */
function analyze(description, context = {}) {
  const classification = classifyPattern(description, context);

  // Record the pattern
  recordPattern({
    description: description.slice(0, 200),
    ...classification,
  });

  // Generate challenge if nomos
  let challenge = null;
  let alternative = null;

  if (classification.shouldChallenge) {
    challenge = generateChallenge(description, classification);
    alternative = suggestNaturalAlternative(classification);
    physisState.challengedConventions.push(challenge);
  }

  return {
    ...classification,
    challenge,
    alternative,
    naturalPatterns: classification.classification === 'PHYSIS'
      ? detectNaturalPatterns(description)
      : [],
  };
}

/**
 * Get statistics
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...physisState.stats,
    recentPatterns: physisState.patterns.slice(-10).map(p => ({
      classification: p.classification,
      confidence: p.confidence,
    })),
    physisRatio: physisState.stats.physisDetected + physisState.stats.nomosDetected > 0
      ? physisState.stats.physisDetected / (physisState.stats.physisDetected + physisState.stats.nomosDetected)
      : 0.5,
  };
}

/**
 * Format analysis result for display
 * @param {Object} result - Result from analyze()
 * @returns {string} Formatted display
 */
function formatAnalysis(result) {
  const emoji = result.classification === 'PHYSIS' ? '🌿' :
                result.classification === 'NOMOS' ? '📜' : '⚖️';

  const lines = [
    '── PHYSIS/NOMOS ───────────────────────────────────────────',
    `   ${emoji} ${result.classification} (${Math.round(result.confidence * 100)}% confiance)`,
  ];

  if (result.challenge) {
    lines.push(`   ❓ ${result.challenge.question}`);
  }

  if (result.alternative) {
    lines.push(`   💡 Alternative: ${result.alternative}`);
  }

  if (result.physisReasons.length > 0) {
    lines.push(`   🌿 Physis: ${result.physisReasons[0].rationale}`);
  }

  if (result.nomosReasons.length > 0) {
    lines.push(`   📜 Nomos: ${result.nomosReasons[0].rationale}`);
  }

  return lines.join('\n');
}

/**
 * Quick check if something is likely a convention
 * @param {string} description - Description to check
 * @returns {boolean}
 */
function isLikelyConvention(description) {
  const result = classifyPattern(description);
  return result.classification === 'NOMOS' && result.confidence >= MIN_EVIDENCE;
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  CHALLENGE_THRESHOLD,
  MIN_EVIDENCE,

  // Core functions
  init,
  analyze,
  getStats,

  // Classification
  classifyPattern,
  isLikelyConvention,

  // Challenge generation
  generateChallenge,
  suggestNaturalAlternative,

  // Pattern discovery
  detectNaturalPatterns,
  recordPattern,

  // Display
  formatAnalysis,
};

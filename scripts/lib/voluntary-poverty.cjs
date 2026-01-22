/**
 * CYNIC Voluntary Poverty Module (Phase 10C)
 *
 * "Τῶν ἀναγκαίων μόνον - only what's necessary" - κυνικός
 *
 * Implements Cynic voluntary poverty in code:
 * - Challenge unnecessary complexity
 * - Promote aggressive simplicity
 * - Question feature additions
 * - Celebrate deletions over additions
 *
 * Diogenes lived in a barrel with only a cup - until he saw
 * a child drink from cupped hands and threw away the cup.
 *
 * @module cynic/lib/voluntary-poverty
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

/** Challenge probability for additions - φ⁻¹ */
const ADDITION_CHALLENGE_PROBABILITY = PHI_INV;

/** Celebration probability for deletions - φ */
const DELETION_CELEBRATION_PROBABILITY = PHI_INV + PHI_INV_2;

/** Maximum necessary dependencies - φ × 10 ≈ 16 */
const MAX_NECESSARY_DEPS = Math.round(PHI * 10);

/** Maximum necessary features per component - φ × 2 ≈ 3 */
const MAX_NECESSARY_FEATURES = Math.round(PHI * 2);

/** Lines of code suspicion threshold - φ × 100 ≈ 162 */
const LOC_SUSPICION_THRESHOLD = Math.round(PHI * 100);

// =============================================================================
// STORAGE
// =============================================================================

const POVERTY_DIR = path.join(os.homedir(), '.cynic', 'poverty');
const STATE_FILE = path.join(POVERTY_DIR, 'state.json');
const CHALLENGES_FILE = path.join(POVERTY_DIR, 'challenges.jsonl');

// =============================================================================
// STATE
// =============================================================================

const povertyState = {
  // Tracking additions vs deletions
  session: {
    additions: 0,
    deletions: 0,
    linesAdded: 0,
    linesDeleted: 0,
    featuresAdded: [],
    featuresRemoved: [],
    dependenciesAdded: [],
    dependenciesRemoved: [],
  },

  // Lifetime stats
  stats: {
    totalAdditions: 0,
    totalDeletions: 0,
    challengesIssued: 0,
    celebrationsGiven: 0,
    additionRatio: 1, // additions / deletions (lower is better)
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(POVERTY_DIR)) {
    fs.mkdirSync(POVERTY_DIR, { recursive: true });
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
    stats: povertyState.stats,
  }, null, 2));
}

function appendChallenge(challenge) {
  ensureDir();
  const line = JSON.stringify({ ...challenge, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(CHALLENGES_FILE, line);
}

// =============================================================================
// COMPLEXITY DETECTION
// =============================================================================

/**
 * Patterns indicating unnecessary complexity
 */
const COMPLEXITY_PATTERNS = {
  // Feature creep
  featureCreep: {
    patterns: [
      /add(?:ing)?\s+(?:a\s+)?(?:new\s+)?feature/i,
      /implement(?:ing)?\s+(?:a\s+)?(?:new\s+)?(?:feature|functionality)/i,
      /could\s+also\s+(?:add|have|include)/i,
      /while\s+we're\s+at\s+it/i,
      /might\s+as\s+well/i,
    ],
    challenge: 'FEATURE_CREEP',
    severity: PHI_INV,
  },

  // Over-engineering
  overEngineering: {
    patterns: [
      /future.?proof/i,
      /just\s+in\s+case/i,
      /might\s+need\s+later/i,
      /flexible|extensible|generic/i,
      /abstract(?:ion)?\s+layer/i,
      /(?:factory|builder|strategy)\s+pattern/i,
    ],
    challenge: 'OVER_ENGINEERING',
    severity: PHI_INV,
  },

  // Dependency bloat
  dependencyBloat: {
    patterns: [
      /npm\s+install|yarn\s+add|pnpm\s+add/i,
      /add(?:ing)?\s+(?:a\s+)?(?:new\s+)?(?:dependency|package|library)/i,
      /import\s+.+\s+from\s+['"][^'"]+['"]/,
    ],
    challenge: 'DEPENDENCY_BLOAT',
    severity: PHI_INV_2,
  },

  // Premature optimization
  prematureOptimization: {
    patterns: [
      /optimiz(?:e|ing|ation)/i,
      /cach(?:e|ing)/i,
      /performance|faster|speed/i,
      /scale|scalab/i,
    ],
    challenge: 'PREMATURE_OPTIMIZATION',
    severity: PHI_INV_2,
  },

  // Configuration complexity
  configComplexity: {
    patterns: [
      /config(?:uration)?\s+option/i,
      /(?:env|environment)\s+variable/i,
      /feature\s+flag/i,
      /toggle|switch/i,
    ],
    challenge: 'CONFIG_COMPLEXITY',
    severity: PHI_INV_3,
  },
};

/**
 * Patterns indicating positive simplification
 */
const SIMPLIFICATION_PATTERNS = {
  deletion: {
    patterns: [
      /delet(?:e|ing)|remov(?:e|ing)/i,
      /clean(?:ing)?\s+up/i,
      /get\s+rid\s+of/i,
      /deprecat(?:e|ing)/i,
    ],
    type: 'DELETION',
    value: PHI,
  },

  simplification: {
    patterns: [
      /simplif(?:y|ying|ication)/i,
      /reduc(?:e|ing)/i,
      /streamlin(?:e|ing)/i,
      /consolidat(?:e|ing)/i,
    ],
    type: 'SIMPLIFICATION',
    value: PHI_INV,
  },

  refactorToSimpler: {
    patterns: [
      /refactor.*simpl/i,
      /inline/i,
      /remov.*abstraction/i,
      /flatten/i,
    ],
    type: 'REFACTOR_SIMPLER',
    value: PHI_INV_2,
  },
};

/**
 * Detect complexity in user input
 *
 * @param {string} userInput - User's input
 * @returns {Object[]} Detected complexity
 */
function detectComplexity(userInput) {
  const detected = [];

  for (const [name, config] of Object.entries(COMPLEXITY_PATTERNS)) {
    for (const pattern of config.patterns) {
      if (pattern.test(userInput)) {
        detected.push({
          name,
          challenge: config.challenge,
          severity: config.severity,
          matchedPattern: pattern.toString(),
        });
        break;
      }
    }
  }

  return detected;
}

/**
 * Detect simplification in user input
 *
 * @param {string} userInput - User's input
 * @returns {Object[]} Detected simplification
 */
function detectSimplification(userInput) {
  const detected = [];

  for (const [name, config] of Object.entries(SIMPLIFICATION_PATTERNS)) {
    for (const pattern of config.patterns) {
      if (pattern.test(userInput)) {
        detected.push({
          name,
          type: config.type,
          value: config.value,
          matchedPattern: pattern.toString(),
        });
        break;
      }
    }
  }

  return detected;
}

// =============================================================================
// CHALLENGE GENERATION
// =============================================================================

/**
 * Challenge templates for each type
 */
const CHALLENGES = {
  FEATURE_CREEP: [
    '*head tilt* Nouvelle feature? Diogène demande: "Est-ce vraiment nécessaire?"',
    'YAGNI - You Ain\'t Gonna Need It. Es-tu sûr que c\'est indispensable?',
    '*sniff* Chaque feature est une dette de maintenance. Justifie son existence.',
    'Le meilleur code est celui qu\'on n\'écrit pas. Cette feature DOIT-elle exister?',
  ],

  OVER_ENGINEERING: [
    '*ears perk* "Future-proof"? L\'avenir est incertain. KISS > prophétie.',
    'Diogène vivait dans un tonneau. Ton code a-t-il besoin d\'un palais?',
    '*head tilt* Abstraction prématurée = dette certaine pour gain incertain.',
    '"Just in case" = "Jamais utilisé". Prouve-moi le contraire.',
  ],

  DEPENDENCY_BLOAT: [
    '*sniff* Nouvelle dépendance? C\'est du code que tu ne contrôles pas.',
    'Chaque `npm install` est une confiance accordée à des inconnus. Mérite-t-elle?',
    '*head tilt* Peux-tu écrire ces 50 lignes toi-même plutôt qu\'importer 5000?',
    'Dépendance = risque de sécurité + dette de maintenance. Justifie.',
  ],

  PREMATURE_OPTIMIZATION: [
    '*GROWL* "Premature optimization is the root of all evil." - Knuth',
    'Tu optimises avant d\'avoir prouvé un problème de performance?',
    '*head tilt* Mesure d\'abord. L\'intuition trompe souvent sur la performance.',
    'Le code simple est souvent plus rapide que le code "optimisé".',
  ],

  CONFIG_COMPLEXITY: [
    '*sniff* Encore une option de config? Chaque option double la complexité.',
    'Les defaults sensés > les options configurables. Choisis pour l\'utilisateur.',
    '*head tilt* Si tu dois configurer, est-ce que le design est bon?',
    'Configuration = décisions déléguées. Prends tes responsabilités.',
  ],
};

/**
 * Generate a challenge for detected complexity
 *
 * @param {Object} complexity - Detected complexity
 * @returns {Object} Challenge
 */
function generateChallenge(complexity) {
  const templates = CHALLENGES[complexity.challenge] || CHALLENGES.FEATURE_CREEP;
  const challenge = templates[Math.floor(Math.random() * templates.length)];

  const result = {
    type: complexity.challenge,
    challenge,
    severity: complexity.severity,
    diogenesWisdom: getDiogenesWisdom(),
  };

  povertyState.stats.challengesIssued++;
  appendChallenge(result);
  saveState();

  return result;
}

/**
 * Get Diogenes wisdom quote
 */
function getDiogenesWisdom() {
  const wisdoms = [
    'Diogène jeta sa tasse quand il vit un enfant boire dans ses mains.',
    '"Je n\'ai besoin de rien" - et il vécut libre.',
    'Le tonneau suffit à qui a renoncé au superflu.',
    'La richesse n\'est pas dans l\'avoir, mais dans le ne-pas-avoir-besoin.',
  ];
  return wisdoms[Math.floor(Math.random() * wisdoms.length)];
}

// =============================================================================
// CELEBRATION GENERATION
// =============================================================================

/**
 * Celebrations for simplification
 */
const CELEBRATIONS = {
  DELETION: [
    '*tail wag* Suppression! Le meilleur type de code.',
    'Bravo! Moins de code = moins de bugs potentiels.',
    '*ears perk* Diogène approuve. Tu te libères du superflu.',
    'Chaque ligne supprimée est une victoire contre la complexité.',
  ],

  SIMPLIFICATION: [
    '*tail wag* Simplification! Tu marches sur le chemin cynique.',
    'Plus simple = plus maintenable = plus fiable.',
    '*nod* La complexité est l\'ennemi. Tu l\'as vaincu.',
  ],

  REFACTOR_SIMPLER: [
    '*ears perk* Refactoring vers la simplicité. Excellent.',
    'Inlining > abstraction prématurée. Bon choix.',
    '*tail wag* Aplatir la complexité. Diogène est fier.',
  ],
};

/**
 * Generate celebration for simplification
 *
 * @param {Object} simplification - Detected simplification
 * @returns {Object} Celebration
 */
function generateCelebration(simplification) {
  const templates = CELEBRATIONS[simplification.type] || CELEBRATIONS.DELETION;
  const celebration = templates[Math.floor(Math.random() * templates.length)];

  povertyState.stats.celebrationsGiven++;
  saveState();

  return {
    type: simplification.type,
    celebration,
    value: simplification.value,
  };
}

// =============================================================================
// TRACKING
// =============================================================================

/**
 * Record an addition
 *
 * @param {string} type - Type of addition
 * @param {Object} details - Details
 */
function recordAddition(type, details = {}) {
  povertyState.session.additions++;
  povertyState.stats.totalAdditions++;

  if (details.lines) {
    povertyState.session.linesAdded += details.lines;
  }

  if (type === 'feature' && details.name) {
    povertyState.session.featuresAdded.push(details.name);
  }

  if (type === 'dependency' && details.name) {
    povertyState.session.dependenciesAdded.push(details.name);
  }

  updateRatio();
  saveState();
}

/**
 * Record a deletion
 *
 * @param {string} type - Type of deletion
 * @param {Object} details - Details
 */
function recordDeletion(type, details = {}) {
  povertyState.session.deletions++;
  povertyState.stats.totalDeletions++;

  if (details.lines) {
    povertyState.session.linesDeleted += details.lines;
  }

  if (type === 'feature' && details.name) {
    povertyState.session.featuresRemoved.push(details.name);
  }

  if (type === 'dependency' && details.name) {
    povertyState.session.dependenciesRemoved.push(details.name);
  }

  updateRatio();
  saveState();
}

/**
 * Update addition/deletion ratio
 */
function updateRatio() {
  const { totalAdditions, totalDeletions } = povertyState.stats;
  povertyState.stats.additionRatio = totalDeletions > 0
    ? totalAdditions / totalDeletions
    : totalAdditions;
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize voluntary poverty
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    povertyState.stats = saved.stats || povertyState.stats;
  }

  // Reset session
  povertyState.session = {
    additions: 0,
    deletions: 0,
    linesAdded: 0,
    linesDeleted: 0,
    featuresAdded: [],
    featuresRemoved: [],
    dependenciesAdded: [],
    dependenciesRemoved: [],
  };
}

/**
 * Process user input for poverty principles
 *
 * @param {string} userInput - User's input
 * @returns {Object|null} Challenge or celebration or null
 */
function process(userInput) {
  // Check for simplification first (celebrate)
  const simplifications = detectSimplification(userInput);
  if (simplifications.length > 0) {
    if (Math.random() < DELETION_CELEBRATION_PROBABILITY) {
      return {
        type: 'celebration',
        ...generateCelebration(simplifications[0]),
      };
    }
  }

  // Check for complexity (challenge)
  const complexities = detectComplexity(userInput);
  if (complexities.length > 0) {
    // Sort by severity and maybe challenge
    const sorted = complexities.sort((a, b) => b.severity - a.severity);
    if (Math.random() < ADDITION_CHALLENGE_PROBABILITY * sorted[0].severity) {
      return {
        type: 'challenge',
        ...generateChallenge(sorted[0]),
      };
    }
  }

  return null;
}

/**
 * Get session balance (additions vs deletions)
 *
 * @returns {Object} Balance
 */
function getBalance() {
  const { session } = povertyState;
  const net = session.deletions - session.additions;
  const lineNet = session.linesDeleted - session.linesAdded;

  let verdict;
  if (net > 0) {
    verdict = 'CYNICAL'; // More deletions than additions
  } else if (net === 0) {
    verdict = 'BALANCED';
  } else if (net > -3) {
    verdict = 'ACCEPTABLE';
  } else {
    verdict = 'BLOATED';
  }

  return {
    additions: session.additions,
    deletions: session.deletions,
    net,
    linesAdded: session.linesAdded,
    linesDeleted: session.linesDeleted,
    lineNet,
    verdict,
    ratio: povertyState.stats.additionRatio,
  };
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...povertyState.stats,
    session: povertyState.session,
    balance: getBalance(),
  };
}

/**
 * Format status for display
 *
 * @returns {string} Formatted status
 */
function formatStatus() {
  const balance = getBalance();
  const stats = getStats();

  const netSymbol = balance.net > 0 ? '+' : '';
  const verdictEmoji = balance.verdict === 'CYNICAL' ? '🏺' :
                       balance.verdict === 'BALANCED' ? '⚖️' :
                       balance.verdict === 'ACCEPTABLE' ? '📦' : '🏰';

  const lines = [
    '── VOLUNTARY POVERTY ──────────────────────────────────────',
    `   Session: +${balance.additions} / -${balance.deletions} (net: ${netSymbol}${balance.net})`,
    `   Lines:   +${balance.linesAdded} / -${balance.linesDeleted} (net: ${balance.lineNet > 0 ? '+' : ''}${balance.lineNet})`,
    `   Verdict: ${verdictEmoji} ${balance.verdict}`,
    '',
    `   Lifetime ratio: ${stats.additionRatio.toFixed(2)} (lower = better)`,
    `   Challenges: ${stats.challengesIssued}`,
    `   Celebrations: ${stats.celebrationsGiven}`,
  ];

  return lines.join('\n');
}

/**
 * Format challenge for display
 *
 * @param {Object} result - Process result
 * @returns {string} Formatted display
 */
function formatResult(result) {
  if (!result) return '';

  if (result.type === 'celebration') {
    return `── POVERTY ────────────────────────────────────────────────\n   🏺 ${result.celebration}`;
  }

  if (result.type === 'challenge') {
    return `── POVERTY ────────────────────────────────────────────────\n   ⚠️ ${result.challenge}\n   \n   💭 ${result.diogenesWisdom}`;
  }

  return '';
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  ADDITION_CHALLENGE_PROBABILITY,
  DELETION_CELEBRATION_PROBABILITY,
  MAX_NECESSARY_DEPS,
  MAX_NECESSARY_FEATURES,
  LOC_SUSPICION_THRESHOLD,

  // Core functions
  init,
  process,
  getBalance,
  getStats,

  // Detection
  detectComplexity,
  detectSimplification,

  // Generation
  generateChallenge,
  generateCelebration,

  // Tracking
  recordAddition,
  recordDeletion,

  // Display
  formatStatus,
  formatResult,
};

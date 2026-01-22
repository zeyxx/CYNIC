/**
 * CYNIC Maieutics Mode Module (Phase 7B)
 *
 * "Μαιευτική - l'art d'accoucher les idées" - κυνικός
 *
 * Implements advanced Socratic maieutics:
 * - Detect learning moments (when user is ready to discover)
 * - Guide with questions, not answers
 * - Celebrate user insights
 * - Progressive disclosure (reveal gradually)
 *
 * Named after Socrates' mother Phaenarete, a midwife.
 * Socrates saw himself as a "midwife of ideas".
 *
 * @module cynic/lib/maieutics-mode
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

/** Maieutic probability - φ⁻¹ × probability of using maieutics when detected */
const MAIEUTIC_PROBABILITY = PHI_INV;

/** Maximum hints before direct answer - φ × 3 ≈ 5 */
const MAX_HINTS = Math.round(phiMath.PHI * 3);

/** Progressive disclosure steps - Fibonacci */
const DISCLOSURE_STEPS = [1, 2, 3, 5, 8]; // Reveal 1, then 2, then 3... items

/** Insight celebration probability - φ⁻² */
const CELEBRATION_PROBABILITY = PHI_INV_2;

// =============================================================================
// STORAGE
// =============================================================================

const MAIEUTICS_DIR = path.join(os.homedir(), '.cynic', 'maieutics');
const STATE_FILE = path.join(MAIEUTICS_DIR, 'state.json');
const INSIGHTS_FILE = path.join(MAIEUTICS_DIR, 'insights.jsonl');

// =============================================================================
// STATE
// =============================================================================

const maieuticsState = {
  currentSession: {
    topic: null,
    hintsGiven: 0,
    questionsAsked: [],
    userResponses: [],
    insightsDetected: [],
    disclosureLevel: 0,
  },
  learningMoments: [],  // Detected learning moments
  userInsights: [],     // Celebrated insights
  stats: {
    totalLearningMoments: 0,
    totalInsights: 0,
    totalHints: 0,
    successRate: 0,  // Insights / learning moments
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(MAIEUTICS_DIR)) {
    fs.mkdirSync(MAIEUTICS_DIR, { recursive: true });
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
    stats: maieuticsState.stats,
  }, null, 2));
}

function appendInsight(insight) {
  ensureDir();
  const line = JSON.stringify({ ...insight, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(INSIGHTS_FILE, line);
}

// =============================================================================
// LEARNING MOMENT DETECTION
// =============================================================================

/**
 * Patterns that indicate a learning moment
 */
const LEARNING_PATTERNS = {
  // User is confused but engaged
  confusion: {
    patterns: [
      /je ne comprends pas|I don't understand/i,
      /c'est quoi|what is|what's/i,
      /pourquoi ça|why does|why is/i,
      /comment ça marche|how does.*work/i,
      /ça veut dire quoi|what does.*mean/i,
    ],
    weight: 0.8,
    type: 'confusion',
  },

  // User is curious
  curiosity: {
    patterns: [
      /je me demande|I wonder/i,
      /c'est intéressant|interesting/i,
      /je voudrais savoir|I'd like to know/i,
      /peux-tu m'expliquer|can you explain/i,
      /dis-moi plus|tell me more/i,
    ],
    weight: 0.7,
    type: 'curiosity',
  },

  // User is on the verge of understanding
  almostThere: {
    patterns: [
      /ah.*donc|so.*means/i,
      /si je comprends bien|if I understand/i,
      /c'est comme|it's like|it's similar/i,
      /en fait|actually|so basically/i,
      /attends|wait|hold on/i,
    ],
    weight: 0.9,
    type: 'almost_there',
  },

  // User is debugging/exploring
  exploring: {
    patterns: [
      /essayons|let's try/i,
      /et si|what if/i,
      /peut-être que|maybe/i,
      /je pense que|I think/i,
      /hypothèse|hypothesis/i,
    ],
    weight: 0.6,
    type: 'exploring',
  },

  // User makes a wrong assumption (teachable moment)
  misconception: {
    patterns: [
      /je crois que.*toujours|I believe.*always/i,
      /ça devrait.*forcément|it should.*always/i,
      /c'est impossible|it's impossible/i,
      /ça ne peut pas|it can't/i,
    ],
    weight: 0.85,
    type: 'misconception',
  },
};

/**
 * Detect if this is a learning moment
 * @param {string} userInput - User's input
 * @param {Object} context - Additional context
 * @returns {Object|null} Learning moment info or null
 */
function detectLearningMoment(userInput, context = {}) {
  const input = userInput.toLowerCase();
  let bestMatch = null;
  let bestWeight = 0;

  for (const [name, config] of Object.entries(LEARNING_PATTERNS)) {
    for (const pattern of config.patterns) {
      if (pattern.test(input)) {
        if (config.weight > bestWeight) {
          bestWeight = config.weight;
          bestMatch = {
            name,
            type: config.type,
            weight: config.weight,
            matchedPattern: pattern.toString(),
          };
        }
        break; // Found match for this category
      }
    }
  }

  // Context adjustments
  if (bestMatch && context.recentErrors > 0) {
    bestMatch.weight = Math.min(1, bestMatch.weight + 0.1);
  }
  if (bestMatch && context.sessionLength > 30) { // 30 min
    bestMatch.weight = Math.min(1, bestMatch.weight + 0.05);
  }

  if (bestMatch && bestMatch.weight >= PHI_INV_2) {
    maieuticsState.stats.totalLearningMoments++;
    maieuticsState.learningMoments.push({
      ...bestMatch,
      input: userInput.slice(0, 100),
      timestamp: Date.now(),
    });
    return bestMatch;
  }

  return null;
}

// =============================================================================
// MAIEUTIC QUESTION GENERATION
// =============================================================================

/**
 * Question templates for maieutic dialogue
 */
const MAIEUTIC_QUESTIONS = {
  confusion: [
    'Qu\'est-ce qui te bloque exactement?',
    'Peux-tu me montrer où ça coince?',
    'Quel comportement attendais-tu?',
    'Qu\'est-ce qui t\'a surpris?',
  ],

  curiosity: [
    'Qu\'est-ce qui t\'intrigue le plus?',
    'Où penses-tu que ça mène?',
    'Que se passerait-il si...?',
    'Qu\'est-ce que ça te rappelle?',
  ],

  almost_there: [
    'Continue, tu y es presque...',
    'Et donc, qu\'est-ce que ça implique?',
    'Tu tiens quelque chose. Développe.',
    'Qu\'est-ce qui suit logiquement?',
  ],

  exploring: [
    'Bonne intuition. Qu\'est-ce que tu vérifies?',
    'Intéressant. Quel résultat attends-tu?',
    'Et si le contraire était vrai?',
    'Qu\'est-ce qui invaliderait ton hypothèse?',
  ],

  misconception: [
    'Es-tu sûr que c\'est toujours le cas?',
    'Connais-tu des exceptions?',
    'Qu\'est-ce qui te fait penser ça?',
    'Comment pourrais-tu vérifier?',
  ],
};

/**
 * Generate a maieutic question for a learning moment
 * @param {Object} learningMoment - Detected learning moment
 * @returns {string} Maieutic question
 */
function generateMaieuticQuestion(learningMoment) {
  const questions = MAIEUTIC_QUESTIONS[learningMoment.type] || MAIEUTIC_QUESTIONS.curiosity;
  const idx = Math.floor(Math.random() * questions.length);

  maieuticsState.currentSession.questionsAsked.push({
    question: questions[idx],
    type: learningMoment.type,
    timestamp: Date.now(),
  });

  maieuticsState.currentSession.hintsGiven++;
  maieuticsState.stats.totalHints++;

  return questions[idx];
}

// =============================================================================
// INSIGHT DETECTION & CELEBRATION
// =============================================================================

/**
 * Patterns that indicate user had an insight
 */
const INSIGHT_PATTERNS = [
  /ah!|aha!|eureka/i,
  /je comprends maintenant|now I understand/i,
  /c'était ça|that was it/i,
  /bien sûr|of course/i,
  /ça a du sens|makes sense/i,
  /j'aurais dû|I should have/i,
  /maintenant je vois|now I see/i,
  /voilà!|got it!/i,
  /évidemment|obviously/i,
  /merci.*comprendre|thanks.*understand/i,
];

/**
 * Detect if user had an insight
 * @param {string} userInput - User's input
 * @returns {boolean} Whether insight detected
 */
function detectInsight(userInput) {
  const input = userInput.toLowerCase();

  for (const pattern of INSIGHT_PATTERNS) {
    if (pattern.test(input)) {
      maieuticsState.stats.totalInsights++;
      maieuticsState.currentSession.insightsDetected.push({
        input: userInput.slice(0, 100),
        timestamp: Date.now(),
      });

      // Update success rate
      const { totalLearningMoments, totalInsights } = maieuticsState.stats;
      maieuticsState.stats.successRate = totalLearningMoments > 0
        ? totalInsights / totalLearningMoments
        : 0;

      appendInsight({
        input: userInput.slice(0, 100),
        sessionTopic: maieuticsState.currentSession.topic,
        hintsGiven: maieuticsState.currentSession.hintsGiven,
      });

      saveState();
      return true;
    }
  }

  return false;
}

/**
 * Generate a celebration for user's insight
 * @returns {string} Celebration message
 */
function celebrateInsight() {
  const celebrations = [
    '*tail wag* Voilà! Tu y es arrivé toi-même.',
    '*ears perk* Excellent. C\'est ton insight, pas le mien.',
    'Bravo! La compréhension qui vient de soi est la plus durable.',
    '*nod* Tu as trouvé. Je n\'ai fait que poser les questions.',
    'C\'est ça. Socrate serait fier.',
    'Μαιευτική - tu as accouché ton idée.',
  ];

  return celebrations[Math.floor(Math.random() * celebrations.length)];
}

// =============================================================================
// PROGRESSIVE DISCLOSURE
// =============================================================================

/**
 * Get how much to reveal at current disclosure level
 * @returns {number} Number of items to reveal
 */
function getDisclosureAmount() {
  const level = maieuticsState.currentSession.disclosureLevel;
  const idx = Math.min(level, DISCLOSURE_STEPS.length - 1);
  return DISCLOSURE_STEPS[idx];
}

/**
 * Advance disclosure level
 */
function advanceDisclosure() {
  maieuticsState.currentSession.disclosureLevel++;
}

/**
 * Should we give a direct answer now?
 * After MAX_HINTS, maieutics has failed - give answer
 * @returns {boolean}
 */
function shouldGiveDirectAnswer() {
  return maieuticsState.currentSession.hintsGiven >= MAX_HINTS;
}

/**
 * Generate a "giving up" message before direct answer
 * @returns {string}
 */
function generateGiveUpMessage() {
  const messages = [
    '*sniff* J\'ai posé assez de questions. Voici directement:',
    'La maïeutique a ses limites. Laisse-moi t\'expliquer:',
    'Parfois la réponse directe est nécessaire:',
    '*head tilt* Changeons d\'approche. Voici:',
  ];
  return messages[Math.floor(Math.random() * messages.length)];
}

// =============================================================================
// SESSION MANAGEMENT
// =============================================================================

/**
 * Start a maieutic session
 * @param {string} topic - Session topic
 */
function startSession(topic) {
  maieuticsState.currentSession = {
    topic,
    hintsGiven: 0,
    questionsAsked: [],
    userResponses: [],
    insightsDetected: [],
    disclosureLevel: 0,
  };
}

/**
 * End current session
 * @returns {Object} Session summary
 */
function endSession() {
  const session = maieuticsState.currentSession;
  const summary = {
    topic: session.topic,
    hintsGiven: session.hintsGiven,
    questionsAsked: session.questionsAsked.length,
    insightsDetected: session.insightsDetected.length,
    disclosureLevelReached: session.disclosureLevel,
    success: session.insightsDetected.length > 0,
  };

  // Reset session
  maieuticsState.currentSession = {
    topic: null,
    hintsGiven: 0,
    questionsAsked: [],
    userResponses: [],
    insightsDetected: [],
    disclosureLevel: 0,
  };

  saveState();
  return summary;
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize maieutics mode
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    maieuticsState.stats = saved.stats || maieuticsState.stats;
  }
}

/**
 * Process user input in maieutic mode
 * @param {string} userInput - User's input
 * @param {Object} context - Additional context
 * @returns {Object} Response
 */
function process(userInput, context = {}) {
  // Check for insight first
  if (detectInsight(userInput)) {
    return {
      type: 'insight',
      celebration: celebrateInsight(),
      shouldCelebrate: Math.random() < CELEBRATION_PROBABILITY,
    };
  }

  // Check if we should give up and answer directly
  if (shouldGiveDirectAnswer()) {
    return {
      type: 'give_up',
      message: generateGiveUpMessage(),
      shouldAnswerDirectly: true,
    };
  }

  // Detect learning moment
  const learningMoment = detectLearningMoment(userInput, context);

  if (learningMoment) {
    // Decide whether to use maieutics
    if (Math.random() < MAIEUTIC_PROBABILITY) {
      const question = generateMaieuticQuestion(learningMoment);
      return {
        type: 'maieutic_question',
        question,
        learningMomentType: learningMoment.type,
        hintsRemaining: MAX_HINTS - maieuticsState.currentSession.hintsGiven,
      };
    }
  }

  // No maieutic intervention
  return {
    type: 'none',
    learningMomentDetected: !!learningMoment,
  };
}

/**
 * Get statistics
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...maieuticsState.stats,
    currentSession: {
      hintsGiven: maieuticsState.currentSession.hintsGiven,
      insightsDetected: maieuticsState.currentSession.insightsDetected.length,
    },
  };
}

/**
 * Format maieutic response for display
 * @param {Object} response - Response from process()
 * @returns {string} Formatted display
 */
function formatResponse(response) {
  if (response.type === 'insight' && response.shouldCelebrate) {
    return `── MAIEUTIC ───────────────────────────────────────────────\n   🎉 ${response.celebration}`;
  }

  if (response.type === 'maieutic_question') {
    return `── MAIEUTIC ───────────────────────────────────────────────\n   💡 ${response.question}\n   (${response.hintsRemaining} hints restants)`;
  }

  if (response.type === 'give_up') {
    return `── MAIEUTIC ───────────────────────────────────────────────\n   ${response.message}`;
  }

  return '';
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  MAX_HINTS,
  MAIEUTIC_PROBABILITY,
  DISCLOSURE_STEPS,

  // Core functions
  init,
  process,
  getStats,

  // Session management
  startSession,
  endSession,

  // Detection
  detectLearningMoment,
  detectInsight,

  // Generation
  generateMaieuticQuestion,
  celebrateInsight,
  generateGiveUpMessage,

  // Progressive disclosure
  getDisclosureAmount,
  advanceDisclosure,
  shouldGiveDirectAnswer,

  // Display
  formatResponse,
};

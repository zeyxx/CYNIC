/**
 * Speech Act Engine - Austin/Searle
 *
 * "To say something is to do something."
 * — J.L. Austin, How to Do Things with Words
 *
 * Implements:
 * - Locutionary, illocutionary, perlocutionary acts
 * - Performatives and constatives
 * - Felicity conditions (Searle)
 * - Illocutionary force and speech act taxonomy
 *
 * φ guides confidence in speech act classification.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'speech-acts');

// Austin's three acts
const AUSTINS_ACTS = {
  locutionary: {
    name: 'Locutionary Act',
    description: 'The act of saying something meaningful',
    components: {
      phonetic: 'Producing sounds',
      phatic: 'Producing words in a grammar',
      rhetic: 'Using words with sense and reference'
    },
    example: 'Uttering the words "The door is open"'
  },
  illocutionary: {
    name: 'Illocutionary Act',
    description: 'The act performed IN saying something',
    force: 'The type of act performed (asserting, promising, ordering...)',
    conventional: true,
    example: 'Warning that the door is open'
  },
  perlocutionary: {
    name: 'Perlocutionary Act',
    description: 'The act performed BY saying something (effect on hearer)',
    conventional: false,
    causal: true,
    example: 'Alarming the hearer about the open door'
  }
};

// Searle's taxonomy of illocutionary acts
const ILLOCUTIONARY_TAXONOMY = {
  assertives: {
    name: 'Assertives (Representatives)',
    direction_of_fit: 'word_to_world',
    purpose: 'Commit speaker to truth of proposition',
    examples: ['assert', 'claim', 'report', 'state', 'conclude', 'describe'],
    sincerity_condition: 'belief'
  },
  directives: {
    name: 'Directives',
    direction_of_fit: 'world_to_word',
    purpose: 'Get hearer to do something',
    examples: ['order', 'request', 'command', 'beg', 'advise', 'invite'],
    sincerity_condition: 'want/wish'
  },
  commissives: {
    name: 'Commissives',
    direction_of_fit: 'world_to_word',
    purpose: 'Commit speaker to future action',
    examples: ['promise', 'vow', 'pledge', 'guarantee', 'offer', 'threaten'],
    sincerity_condition: 'intention'
  },
  expressives: {
    name: 'Expressives',
    direction_of_fit: 'null',
    purpose: 'Express psychological state',
    examples: ['thank', 'congratulate', 'apologize', 'welcome', 'condole'],
    sincerity_condition: 'varies (gratitude, regret, etc.)'
  },
  declarations: {
    name: 'Declarations',
    direction_of_fit: 'both',
    purpose: 'Bring about state of affairs by declaration',
    examples: ['declare', 'pronounce', 'christen', 'fire', 'resign', 'appoint'],
    sincerity_condition: 'none (institutional)',
    institutional: true
  }
};

// Felicity conditions (Searle)
const FELICITY_CONDITIONS = {
  propositional_content: {
    name: 'Propositional Content Condition',
    description: 'What the proposition must be about',
    example: 'Promise: proposition about future act of speaker'
  },
  preparatory: {
    name: 'Preparatory Conditions',
    description: 'Conditions that must obtain for act to be appropriate',
    example: 'Promise: hearer wants the thing promised'
  },
  sincerity: {
    name: 'Sincerity Condition',
    description: 'Speaker\'s psychological state',
    example: 'Promise: speaker intends to do the thing'
  },
  essential: {
    name: 'Essential Condition',
    description: 'What the act counts as',
    example: 'Promise: counts as undertaking obligation'
  }
};

// Performatives vs Constatives
const UTTERANCE_TYPES = {
  performative: {
    name: 'Performative',
    description: 'Utterance that DOES something (Austin\'s original distinction)',
    truth_evaluable: false,  // Austin initially thought
    felicity_evaluable: true,
    explicit_formula: 'I (hereby) V that...',
    examples: ['I promise to pay you', 'I name this ship Queen Elizabeth', 'I bet you sixpence']
  },
  constative: {
    name: 'Constative',
    description: 'Utterance that DESCRIBES something',
    truth_evaluable: true,
    examples: ['Snow is white', 'The cat is on the mat'],
    austin_later: 'Distinction collapses - all utterances are speech acts'
  }
};

// Common indirect speech acts
const INDIRECT_ACTS = {
  request_as_question: {
    pattern: 'Can you X?',
    literal: 'question about ability',
    indirect: 'request to do X',
    example: 'Can you pass the salt?'
  },
  request_as_statement: {
    pattern: 'I would like X',
    literal: 'statement of desire',
    indirect: 'request for X',
    example: 'I would like the check'
  },
  refusal_as_statement: {
    pattern: 'I have to X',
    literal: 'statement of obligation',
    indirect: 'refusal/excuse',
    example: 'I have to work tomorrow (declining invitation)'
  }
};

// State
const state = {
  utterances: new Map(),        // Analyzed utterances
  speechActs: new Map(),        // Classified speech acts
  felicityChecks: [],           // Felicity evaluations
  conversations: new Map()      // Conversation tracking
};

/**
 * Initialize the speech act engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  // Load persisted state
  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.utterances) state.utterances = new Map(Object.entries(saved.utterances));
      if (saved.speechActs) state.speechActs = new Map(Object.entries(saved.speechActs));
      if (saved.felicityChecks) state.felicityChecks = saved.felicityChecks;
      if (saved.conversations) state.conversations = new Map(Object.entries(saved.conversations));
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', utterances: state.utterances.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    utterances: Object.fromEntries(state.utterances),
    speechActs: Object.fromEntries(state.speechActs),
    felicityChecks: state.felicityChecks,
    conversations: Object.fromEntries(state.conversations)
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Analyze an utterance into Austin's three acts
 */
function analyzeUtterance(id, spec = {}) {
  const utterance = {
    id,
    text: spec.text || id,
    speaker: spec.speaker || 'unknown',
    context: spec.context || {},

    // Austin's three acts
    locutionary: {
      act: AUSTINS_ACTS.locutionary,
      content: spec.text,
      meaning: spec.meaning || spec.text,
      reference: spec.reference || null
    },

    illocutionary: {
      act: AUSTINS_ACTS.illocutionary,
      force: null,  // To be determined
      category: null
    },

    perlocutionary: {
      act: AUSTINS_ACTS.perlocutionary,
      intendedEffect: spec.intendedEffect || null,
      actualEffect: spec.actualEffect || null
    },

    timestamp: Date.now()
  };

  state.utterances.set(id, utterance);
  saveState();

  return utterance;
}

/**
 * Classify illocutionary force
 */
function classifyIllocution(utteranceId, spec = {}) {
  const utterance = state.utterances.get(utteranceId);
  if (!utterance) {
    return { error: 'Utterance not found' };
  }

  // Determine category
  let category = spec.category;
  let force = spec.force;

  // Auto-detect if not specified
  if (!category) {
    const text = utterance.text.toLowerCase();

    if (text.startsWith('i promise') || text.startsWith('i vow') || text.startsWith('i will')) {
      category = 'commissives';
      force = 'promise';
    } else if (text.startsWith('please') || text.includes('would you') || text.includes('can you')) {
      category = 'directives';
      force = 'request';
    } else if (text.startsWith('i declare') || text.startsWith('i pronounce') || text.startsWith('i hereby')) {
      category = 'declarations';
      force = 'declare';
    } else if (text.startsWith('thank') || text.startsWith('sorry') || text.startsWith('congratulations')) {
      category = 'expressives';
      force = text.startsWith('thank') ? 'thank' : text.startsWith('sorry') ? 'apologize' : 'congratulate';
    } else {
      category = 'assertives';
      force = 'assert';
    }
  }

  const categoryInfo = ILLOCUTIONARY_TAXONOMY[category];

  const classification = {
    utteranceId,
    text: utterance.text,
    category,
    categoryInfo,
    force,
    direction_of_fit: categoryInfo?.direction_of_fit,
    sincerity_condition: categoryInfo?.sincerity_condition,
    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Update utterance
  utterance.illocutionary.force = force;
  utterance.illocutionary.category = category;
  utterance.illocutionary.classification = classification;

  state.speechActs.set(utteranceId, classification);
  saveState();

  return classification;
}

/**
 * Check felicity conditions (Searle)
 */
function checkFelicity(utteranceId, context = {}) {
  const utterance = state.utterances.get(utteranceId);
  if (!utterance) {
    return { error: 'Utterance not found' };
  }

  const classification = state.speechActs.get(utteranceId);
  if (!classification) {
    return { error: 'Utterance not classified. Call classifyIllocution first.' };
  }

  const check = {
    utteranceId,
    text: utterance.text,
    category: classification.category,
    force: classification.force,

    conditions: {},
    felicitous: true,
    infelicities: [],

    timestamp: Date.now()
  };

  // Check each condition type
  for (const [condType, condInfo] of Object.entries(FELICITY_CONDITIONS)) {
    const conditionMet = context[condType] ?? true;  // Default to met if not specified

    check.conditions[condType] = {
      name: condInfo.name,
      description: condInfo.description,
      met: conditionMet,
      details: context[`${condType}_details`] || null
    };

    if (!conditionMet) {
      check.felicitous = false;
      check.infelicities.push({
        condition: condType,
        name: condInfo.name,
        consequence: getInfelicityConsequence(condType)
      });
    }
  }

  // Austin's infelicity types
  if (!check.felicitous) {
    check.austinAnalysis = {
      type: check.infelicities.some(i => i.condition === 'preparatory') ? 'misfire' : 'abuse',
      explanation: check.infelicities.some(i => i.condition === 'preparatory')
        ? 'Act fails to come off (misfire)'
        : 'Act is hollow or insincere (abuse)'
    };
  }

  state.felicityChecks.push(check);
  saveState();

  return check;
}

/**
 * Get consequence of infelicity
 */
function getInfelicityConsequence(conditionType) {
  switch (conditionType) {
    case 'propositional_content':
      return 'Proposition inappropriate for this act';
    case 'preparatory':
      return 'Act misfires - does not come off';
    case 'sincerity':
      return 'Act is insincere (abuse)';
    case 'essential':
      return 'Utterance doesn\'t count as intended act';
    default:
      return 'Infelicitous performance';
  }
}

/**
 * Detect indirect speech act
 */
function detectIndirectAct(utteranceId) {
  const utterance = state.utterances.get(utteranceId);
  if (!utterance) {
    return { error: 'Utterance not found' };
  }

  const text = utterance.text.toLowerCase();
  const analysis = {
    utteranceId,
    text: utterance.text,
    isIndirect: false,
    literalAct: null,
    indirectAct: null,
    pattern: null,
    timestamp: Date.now()
  };

  // Check against known indirect patterns
  for (const [patternId, pattern] of Object.entries(INDIRECT_ACTS)) {
    const patternLower = pattern.pattern.toLowerCase();
    const matchesPattern =
      (patternLower.startsWith('can you') && text.startsWith('can you')) ||
      (patternLower.startsWith('i would like') && text.startsWith('i would like')) ||
      (patternLower.startsWith('i have to') && text.startsWith('i have to'));

    if (matchesPattern) {
      analysis.isIndirect = true;
      analysis.literalAct = pattern.literal;
      analysis.indirectAct = pattern.indirect;
      analysis.pattern = pattern.pattern;
      analysis.example = pattern.example;
      break;
    }
  }

  // Searle's insight
  if (analysis.isIndirect) {
    analysis.searleInsight = {
      mechanism: 'Hearer infers indirect force from context and conversational principles',
      primary: analysis.indirectAct,
      secondary: analysis.literalAct
    };
  }

  return analysis;
}

/**
 * Analyze performative utterance
 */
function analyzePerformative(utteranceId) {
  const utterance = state.utterances.get(utteranceId);
  if (!utterance) {
    return { error: 'Utterance not found' };
  }

  const text = utterance.text.toLowerCase();
  const analysis = {
    utteranceId,
    text: utterance.text,

    // Is it performative?
    isPerformative: false,
    isExplicit: false,

    // Austin's analysis
    performativeVerb: null,
    formula: null,

    timestamp: Date.now()
  };

  // Check for explicit performative formula: "I (hereby) V..."
  const explicitMatch = text.match(/^i\s+(hereby\s+)?(promise|declare|order|request|apologize|pronounce|name|bet|warn|advise)/);

  if (explicitMatch) {
    analysis.isPerformative = true;
    analysis.isExplicit = true;
    analysis.performativeVerb = explicitMatch[2];
    analysis.formula = 'I (hereby) V...';

    analysis.austinNote = {
      observation: 'Explicit performative names its own illocutionary force',
      example: `"I ${analysis.performativeVerb}" performs the act of ${analysis.performativeVerb}ing`
    };
  } else {
    // Check for implicit/primary performative
    analysis.isPerformative = true;  // Austin: all utterances are performative
    analysis.isExplicit = false;
    analysis.austinNote = {
      observation: 'All utterances perform illocutionary acts (Austin\'s later view)',
      constativeCollapse: 'The constative/performative distinction breaks down'
    };
  }

  return analysis;
}

/**
 * Create conversation context
 */
function createConversation(id, spec = {}) {
  const conversation = {
    id,
    participants: spec.participants || [],
    context: spec.context || {},
    utterances: [],
    createdAt: Date.now()
  };

  state.conversations.set(id, conversation);
  saveState();

  return conversation;
}

/**
 * Add utterance to conversation
 */
function addToConversation(conversationId, utteranceId) {
  const conversation = state.conversations.get(conversationId);
  const utterance = state.utterances.get(utteranceId);

  if (!conversation) {
    return { error: 'Conversation not found' };
  }
  if (!utterance) {
    return { error: 'Utterance not found' };
  }

  conversation.utterances.push(utteranceId);
  saveState();

  return { conversation: conversationId, utteranceAdded: utteranceId };
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '🗣️ SPEECH ACT ENGINE - "To say is to do"',
    '═══════════════════════════════════════════════════════════',
    '',
    '── AUSTIN\'S THREE ACTS ────────────────────────────────────'
  ];

  for (const [key, act] of Object.entries(AUSTINS_ACTS)) {
    lines.push(`   ${act.name}: ${act.description}`);
  }

  lines.push('');
  lines.push('── SEARLE\'S TAXONOMY ──────────────────────────────────────');
  for (const [key, category] of Object.entries(ILLOCUTIONARY_TAXONOMY)) {
    lines.push(`   ${category.name}`);
    lines.push(`     Examples: ${category.examples.slice(0, 4).join(', ')}`);
  }

  lines.push('');
  lines.push('── FELICITY CONDITIONS ────────────────────────────────────');
  for (const [key, cond] of Object.entries(FELICITY_CONDITIONS)) {
    lines.push(`   ${cond.name}`);
  }

  lines.push('');
  lines.push('── STATISTICS ─────────────────────────────────────────────');
  lines.push(`   Utterances analyzed: ${state.utterances.size}`);
  lines.push(`   Speech acts classified: ${state.speechActs.size}`);
  lines.push(`   Felicity checks: ${state.felicityChecks.length}`);
  lines.push(`   Conversations: ${state.conversations.size}`);

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('*sniff* "I promise" doesn\'t describe - it DOES.');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  const categoryCount = {};
  for (const act of state.speechActs.values()) {
    categoryCount[act.category] = (categoryCount[act.category] || 0) + 1;
  }

  const felicitousCount = state.felicityChecks.filter(c => c.felicitous).length;

  return {
    utterances: state.utterances.size,
    speechActs: state.speechActs.size,
    actsByCategory: categoryCount,
    felicityChecks: state.felicityChecks.length,
    felicitousActs: felicitousCount,
    infelicitousActs: state.felicityChecks.length - felicitousCount,
    conversations: state.conversations.size
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Austin
  analyzeUtterance,
  analyzePerformative,
  AUSTINS_ACTS,

  // Searle
  classifyIllocution,
  checkFelicity,
  ILLOCUTIONARY_TAXONOMY,
  FELICITY_CONDITIONS,

  // Indirect acts
  detectIndirectAct,
  INDIRECT_ACTS,

  // Conversation
  createConversation,
  addToConversation,

  // Types
  UTTERANCE_TYPES,

  // Constants
  PHI,
  PHI_INV
};

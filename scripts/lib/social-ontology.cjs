/**
 * Social Ontology - CYNIC Philosophy Integration
 *
 * Implements collective intentionality, social facts, and institutions
 * following Searle and Tuomela.
 *
 * "X counts as Y in context C" - Searle's constitutive rule formula
 *
 * @module social-ontology
 */

const fs = require('fs');
const path = require('path');

// φ-derived constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const PHI_INV_3 = 0.236067977499790;

// Configuration
const STORAGE_DIR = path.join(
  process.env.HOME || process.env.USERPROFILE,
  '.cynic',
  'social-ontology'
);
const STATE_FILE = path.join(STORAGE_DIR, 'state.json');
const HISTORY_FILE = path.join(STORAGE_DIR, 'history.jsonl');

const MAX_FACTS = 200;
const MAX_INSTITUTIONS = 50;

/**
 * Intentionality Modes (Individual vs Collective)
 */
const INTENTIONALITY_MODES = {
  i_intention: {
    name: 'I-Intention',
    description: 'Individual intention - "I intend to do X"',
    symbol: 'I',
    collective: false,
  },
  we_intention: {
    name: 'We-Intention',
    description: 'Collective intention - "We intend to do X together"',
    symbol: 'We',
    collective: true,
    // Irreducible to I-intentions (Searle's claim)
  },
  joint_intention: {
    name: 'Joint Intention',
    description: 'Shared plan with interlocking intentions (Bratman)',
    symbol: 'J',
    collective: true,
    requirements: ['mutual_responsiveness', 'commitment_to_joint_activity', 'commitment_to_mutual_support'],
  },
};

/**
 * Social Fact Types (Searle's ontology)
 */
const SOCIAL_FACT_TYPES = {
  brute: {
    name: 'Brute Fact',
    description: 'Physical fact independent of human agreement',
    examples: ['Mountain exists', 'Water is H2O'],
    requiresInstitution: false,
    symbol: '●',
  },
  institutional: {
    name: 'Institutional Fact',
    description: 'Fact that exists by collective acceptance',
    examples: ['This is money', 'She is president', 'We are married'],
    requiresInstitution: true,
    symbol: '◈',
  },
  social: {
    name: 'Social Fact',
    description: 'Fact involving collective intentionality',
    examples: ['This is a party', 'We are having a conversation'],
    requiresInstitution: false,
    symbol: '◉',
  },
};

/**
 * Status Function Types
 * "X counts as Y in context C"
 */
const STATUS_FUNCTION_TYPES = {
  symbolic: {
    name: 'Symbolic',
    description: 'Represents something else',
    examples: ['words', 'signs', 'flags'],
    weight: PHI_INV,
  },
  deontic: {
    name: 'Deontic',
    description: 'Creates rights, duties, obligations',
    examples: ['property', 'marriage', 'contracts'],
    weight: PHI_INV + PHI_INV_3,
  },
  honorific: {
    name: 'Honorific',
    description: 'Confers honor or status',
    examples: ['knighthood', 'citizenship', 'membership'],
    weight: PHI_INV_2,
  },
  procedural: {
    name: 'Procedural',
    description: 'Enables procedures and actions',
    examples: ['voting', 'promising', 'appointing'],
    weight: PHI_INV,
  },
};

/**
 * Collective Acceptance Levels
 */
const ACCEPTANCE_LEVELS = {
  universal: {
    name: 'Universal',
    description: 'All members accept',
    threshold: 1.0,
    strength: PHI_INV + PHI_INV_2,
  },
  consensus: {
    name: 'Consensus',
    description: 'Strong majority accepts',
    threshold: PHI_INV + PHI_INV_2, // ~85%
    strength: PHI_INV,
  },
  majority: {
    name: 'Majority',
    description: 'More than half accept',
    threshold: 0.5,
    strength: PHI_INV_2,
  },
  plurality: {
    name: 'Plurality',
    description: 'Largest group accepts',
    threshold: PHI_INV_3,
    strength: PHI_INV_3,
  },
};

// State
let state = {
  groups: {},           // Collective agents
  facts: {},            // Social and institutional facts
  institutions: {},     // Institutions (systems of status functions)
  statusFunctions: [],  // Active status functions
  stats: {
    groupsFormed: 0,
    factsCreated: 0,
    institutionsCreated: 0,
    statusFunctionsAssigned: 0,
  },
  lastUpdated: null,
};

/**
 * Initialize module
 */
function init() {
  try {
    if (!fs.existsSync(STORAGE_DIR)) {
      fs.mkdirSync(STORAGE_DIR, { recursive: true });
    }

    if (fs.existsSync(STATE_FILE)) {
      const saved = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
      state = { ...state, ...saved };
    }
  } catch (err) {
    console.error('Social ontology init error:', err.message);
  }
}

/**
 * Save state
 */
function saveState() {
  try {
    state.lastUpdated = Date.now();
    fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
  } catch (err) {
    console.error('Social ontology save error:', err.message);
  }
}

/**
 * Log to history
 */
function logHistory(entry) {
  try {
    const record = {
      ...entry,
      timestamp: Date.now(),
    };
    fs.appendFileSync(HISTORY_FILE, JSON.stringify(record) + '\n');
  } catch (err) {
    // Silent fail for history
  }
}

/**
 * Form a collective (group with we-intentions)
 *
 * @param {string} id - Collective ID
 * @param {object} spec - Specification with members, name, etc.
 * @returns {object} Formed collective
 */
function formCollective(id, spec = {}) {
  const members = spec.members || [];
  const name = spec.name || id;
  const config = spec;

  const collective = {
    id,
    name,
    members: [...members],
    // Collective intentionality
    weIntentions: [],
    jointCommitments: [],
    // Acceptance tracking
    collectiveAcceptance: {},
    // Structure
    roles: config.roles || {},
    norms: config.norms || [],
    // Status
    isActive: true,
    formedAt: Date.now(),
  };

  state.groups[id] = collective;
  state.stats.groupsFormed++;

  logHistory({
    type: 'collective_formed',
    id,
    memberCount: members.length,
  });

  saveState();

  return {
    collective,
    message: `We-group formed: "${name}" with ${members.length} members`,
  };
}

/**
 * Add a we-intention to a collective
 *
 * @param {string} groupId - Group ID
 * @param {string} content - What "we intend" to do
 * @param {object} config - Configuration
 * @returns {object} We-intention result
 */
function addWeIntention(groupId, content, config = {}) {
  const group = state.groups[groupId];
  if (!group) return { error: 'Group not found' };

  const id = `wei-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

  const weIntention = {
    id,
    content,
    // Searle's analysis
    mode: 'we_intention',
    modeInfo: INTENTIONALITY_MODES.we_intention,
    // Component I-intentions (each member's contribution)
    memberContributions: config.contributions || {},
    // Status
    isActive: true,
    acceptanceLevel: config.acceptanceLevel || 'consensus',
    createdAt: Date.now(),
  };

  group.weIntentions.push(weIntention);

  logHistory({
    type: 'we_intention_added',
    groupId,
    content: content.slice(0, 50),
  });

  saveState();

  return {
    intention: weIntention,
    group: groupId,
    message: `${INTENTIONALITY_MODES.we_intention.symbol}-intention: "${content.slice(0, 40)}..."`,
  };
}

/**
 * Create an institutional fact
 * "X counts as Y in context C"
 *
 * @param {string} x - The brute or lower-level fact
 * @param {string} y - The institutional status
 * @param {string} context - The context/institution
 * @param {object} config - Configuration
 * @returns {object} Created institutional fact
 */
function createInstitutionalFact(x, y, context, config = {}) {
  if (Object.keys(state.facts).length >= MAX_FACTS) {
    // Prune old facts
    const ids = Object.keys(state.facts);
    for (let i = 0; i < Math.floor(ids.length * PHI_INV_2); i++) {
      delete state.facts[ids[i]];
    }
  }

  const id = `fact-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
  const statusType = STATUS_FUNCTION_TYPES[config.statusType] || STATUS_FUNCTION_TYPES.deontic;

  const fact = {
    id,
    // Constitutive rule components
    x,  // Physical/base layer
    y,  // Institutional status
    c: context, // Context
    formula: `"${x}" counts as "${y}" in ${context}`,
    // Type
    factType: 'institutional',
    factTypeInfo: SOCIAL_FACT_TYPES.institutional,
    statusFunctionType: config.statusType || 'deontic',
    statusFunctionInfo: statusType,
    // Deontic powers created
    deonticPowers: config.deonticPowers || [],
    // Collective acceptance
    acceptance: {
      level: config.acceptanceLevel || 'consensus',
      levelInfo: ACCEPTANCE_LEVELS[config.acceptanceLevel] || ACCEPTANCE_LEVELS.consensus,
      acceptors: config.acceptors || [],
    },
    // Validity
    isValid: true,
    validFrom: Date.now(),
    validUntil: config.validUntil || null,
    createdAt: Date.now(),
  };

  state.facts[id] = fact;
  state.stats.factsCreated++;

  // Add to status functions
  state.statusFunctions.push({
    factId: id,
    x,
    y,
    context,
    type: config.statusType || 'deontic',
  });
  state.stats.statusFunctionsAssigned++;

  logHistory({
    type: 'institutional_fact_created',
    id,
    formula: fact.formula,
  });

  saveState();

  return {
    fact,
    message: `${SOCIAL_FACT_TYPES.institutional.symbol} ${fact.formula}`,
  };
}

/**
 * Create an institution (system of constitutive rules)
 *
 * @param {string} name - Institution name
 * @param {array} constitutiveRules - Array of {x, y, c} rules
 * @param {object} config - Configuration
 * @returns {object} Created institution
 */
function createInstitution(name, constitutiveRules, config = {}) {
  if (Object.keys(state.institutions).length >= MAX_INSTITUTIONS) {
    return { error: 'Maximum institutions reached' };
  }

  const id = name.toLowerCase().replace(/\s+/g, '_');

  // Create facts for each rule
  const factIds = [];
  for (const rule of constitutiveRules) {
    const factResult = createInstitutionalFact(rule.x, rule.y, name, {
      statusType: rule.statusType,
      deonticPowers: rule.deonticPowers,
      acceptanceLevel: config.acceptanceLevel,
    });
    if (factResult.fact) {
      factIds.push(factResult.fact.id);
    }
  }

  const institution = {
    id,
    name,
    // Constitutive rules
    rules: constitutiveRules,
    factIds,
    // Hierarchy
    dependsOn: config.dependsOn || [], // Other institutions required
    // Participants
    participants: config.participants || [],
    roles: config.roles || [],
    // Status
    isActive: true,
    createdAt: Date.now(),
  };

  state.institutions[id] = institution;
  state.stats.institutionsCreated++;

  logHistory({
    type: 'institution_created',
    id,
    ruleCount: constitutiveRules.length,
  });

  saveState();

  return {
    institution,
    factIds,
    message: `Institution "${name}" created with ${constitutiveRules.length} constitutive rules`,
  };
}

/**
 * Check if a status function is valid
 *
 * @param {string} x - The base
 * @param {string} y - The claimed status
 * @param {string} context - The context
 * @returns {object} Validity check
 */
function checkStatusFunction(x, y, context) {
  // Find matching status function
  const sf = state.statusFunctions.find(s =>
    s.x.toLowerCase() === x.toLowerCase() &&
    s.y.toLowerCase() === y.toLowerCase() &&
    s.context.toLowerCase() === context.toLowerCase()
  );

  if (!sf) {
    return {
      isValid: false,
      x, y, context,
      message: `No status function: "${x}" does not count as "${y}" in ${context}`,
    };
  }

  // Check underlying fact
  const fact = state.facts[sf.factId];
  if (!fact || !fact.isValid) {
    return {
      isValid: false,
      x, y, context,
      reason: 'Underlying fact is no longer valid',
      message: `Status function expired or revoked`,
    };
  }

  return {
    isValid: true,
    x, y, context,
    fact,
    message: `✓ "${x}" counts as "${y}" in ${context}`,
  };
}

/**
 * Create a joint commitment (Tuomela/Gilbert)
 *
 * @param {string} groupId - Group making the commitment
 * @param {string} content - What the group commits to
 * @param {object} config - Configuration
 * @returns {object} Joint commitment
 */
function createJointCommitment(groupId, content, config = {}) {
  const group = state.groups[groupId];
  if (!group) return { error: 'Group not found' };

  const id = `jc-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

  const commitment = {
    id,
    groupId,
    content,
    // Gilbert's analysis
    type: 'joint_commitment',
    // Members are jointly committed - none can unilaterally release
    members: [...group.members],
    // Status
    isActive: true,
    // Obligations created
    obligations: config.obligations || [
      `Each member is obligated to do their part`,
      `Each member has right to rebuke non-compliance`,
    ],
    createdAt: Date.now(),
  };

  group.jointCommitments.push(commitment);

  logHistory({
    type: 'joint_commitment_created',
    groupId,
    content: content.slice(0, 50),
  });

  saveState();

  return {
    commitment,
    message: `Joint commitment: Group "${group.name}" commits to: "${content.slice(0, 40)}..."`,
  };
}

/**
 * Record collective acceptance of a fact
 *
 * @param {string} groupId - Group ID
 * @param {string} factId - Fact being accepted
 * @param {array} acceptors - Members who accept
 * @returns {object} Acceptance result
 */
function recordAcceptance(groupId, factId, acceptors) {
  const group = state.groups[groupId];
  const fact = state.facts[factId];

  if (!group || !fact) {
    return { error: 'Group or fact not found' };
  }

  const acceptanceRatio = acceptors.length / group.members.length;

  // Determine acceptance level achieved
  let achievedLevel = null;
  for (const [key, level] of Object.entries(ACCEPTANCE_LEVELS)) {
    if (acceptanceRatio >= level.threshold) {
      achievedLevel = { key, ...level };
      break;
    }
  }

  // Update fact's acceptance
  fact.acceptance.acceptors = acceptors;
  fact.acceptance.ratio = acceptanceRatio;
  fact.acceptance.achievedLevel = achievedLevel?.key || 'insufficient';

  saveState();

  return {
    factId,
    groupId,
    ratio: acceptanceRatio,
    achievedLevel: achievedLevel?.key || 'insufficient',
    message: achievedLevel
      ? `${achievedLevel.name} acceptance achieved (${(acceptanceRatio * 100).toFixed(0)}%)`
      : `Insufficient acceptance (${(acceptanceRatio * 100).toFixed(0)}%)`,
  };
}

/**
 * Assign a role within a collective
 *
 * @param {string} groupId - Group ID
 * @param {string} memberId - Member to assign role
 * @param {string} role - Role name
 * @param {object} config - Configuration
 * @returns {object} Role assignment
 */
function assignRole(groupId, memberId, role, config = {}) {
  const group = state.groups[groupId];
  if (!group) return { error: 'Group not found' };

  if (!group.members.includes(memberId)) {
    return { error: 'Not a member of the group' };
  }

  if (!group.roles[role]) {
    group.roles[role] = {
      name: role,
      holders: [],
      powers: config.powers || [],
      duties: config.duties || [],
    };
  }

  if (!group.roles[role].holders.includes(memberId)) {
    group.roles[role].holders.push(memberId);
  }

  saveState();

  return {
    groupId,
    memberId,
    role,
    powers: group.roles[role].powers,
    duties: group.roles[role].duties,
    message: `${memberId} assigned role "${role}" in ${group.name}`,
  };
}

/**
 * Get group by ID
 */
function getGroup(groupId) {
  return state.groups[groupId] || null;
}

/**
 * Get fact by ID
 */
function getFact(factId) {
  return state.facts[factId] || null;
}

/**
 * Get institution by ID
 */
function getInstitution(institutionId) {
  return state.institutions[institutionId] || null;
}

/**
 * Get all groups
 */
function getGroups() {
  return Object.values(state.groups);
}

/**
 * Get all institutions
 */
function getInstitutions() {
  return Object.values(state.institutions);
}

/**
 * Format status for display
 */
function formatStatus() {
  const activeGroups = Object.values(state.groups).filter(g => g.isActive).length;
  const activeInst = Object.values(state.institutions).filter(i => i.isActive).length;
  const validFacts = Object.values(state.facts).filter(f => f.isValid).length;

  return `◈ Social Ontology (Searle/Tuomela)
  Collectives: ${state.stats.groupsFormed} (${activeGroups} active)
  Institutions: ${state.stats.institutionsCreated} (${activeInst} active)
  Institutional facts: ${state.stats.factsCreated} (${validFacts} valid)
  Status functions: ${state.stats.statusFunctionsAssigned}`;
}

/**
 * Get stats
 */
function getStats() {
  return {
    ...state.stats,
    activeGroups: Object.values(state.groups).filter(g => g.isActive).length,
    activeInstitutions: Object.values(state.institutions).filter(i => i.isActive).length,
    validFacts: Object.values(state.facts).filter(f => f.isValid).length,
  };
}

module.exports = {
  init,
  formCollective,
  addWeIntention,
  createInstitutionalFact,
  createInstitution,
  checkStatusFunction,
  createJointCommitment,
  recordAcceptance,
  assignRole,
  getGroup,
  getFact,
  getInstitution,
  getGroups,
  getInstitutions,
  formatStatus,
  getStats,
  INTENTIONALITY_MODES,
  SOCIAL_FACT_TYPES,
  STATUS_FUNCTION_TYPES,
  ACCEPTANCE_LEVELS,
};

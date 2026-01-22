/**
 * CYNIC Modality Engine
 *
 * "Necessity, possibility, and possible worlds"
 *
 * Philosophical foundations:
 * - Kripke: Possible worlds semantics, rigid designators
 * - Lewis: Modal realism, counterparts
 * - Plantinga: Actualism, possible worlds as states of affairs
 * - Necessitism vs Contingentism
 *
 * φ guides all ratios: 61.8% confidence max
 */

const fs = require('fs');
const path = require('path');
const os = require('os');

// ─────────────────────────────────────────────────────────────
// φ CONSTANTS
// ─────────────────────────────────────────────────────────────

const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2% - uncertainty threshold
const PHI_INV_3 = 0.236067977499790;    // 23.6% - minimum threshold

// ─────────────────────────────────────────────────────────────
// MODAL OPERATORS
// ─────────────────────────────────────────────────────────────

const MODAL_OPERATORS = {
  necessary: {
    name: 'Necessary',
    symbol: '□',
    description: 'True in all possible worlds',
    alethic: true,
    strength: PHI_INV + PHI_INV_3
  },
  possible: {
    name: 'Possible',
    symbol: '◇',
    description: 'True in at least one possible world',
    alethic: true,
    strength: PHI_INV_2
  },
  impossible: {
    name: 'Impossible',
    symbol: '¬◇',
    description: 'True in no possible world',
    alethic: true,
    strength: PHI_INV + PHI_INV_3
  },
  contingent: {
    name: 'Contingent',
    symbol: '◇∧◇¬',
    description: 'True in some worlds, false in others',
    alethic: true,
    strength: PHI_INV_2
  },
  actual: {
    name: 'Actually',
    symbol: '@',
    description: 'True in the actual world',
    alethic: true,
    strength: PHI_INV
  }
};

// ─────────────────────────────────────────────────────────────
// MODAL CATEGORIES (Kripke)
// ─────────────────────────────────────────────────────────────

const MODAL_CATEGORIES = {
  a_priori_necessary: {
    name: 'A Priori Necessary',
    description: 'Knowable without experience, true in all worlds',
    examples: ['2+2=4', 'All bachelors are unmarried'],
    kripkeExample: 'Mathematical truths'
  },
  a_priori_contingent: {
    name: 'A Priori Contingent',
    description: 'Knowable without experience, but could be false',
    examples: ['The standard meter is one meter long'],
    kripkeExample: 'Reference-fixing descriptions'
  },
  a_posteriori_necessary: {
    name: 'A Posteriori Necessary',
    description: 'Knowable only by experience, but necessarily true',
    examples: ['Water is H₂O', 'Hesperus is Phosphorus'],
    kripkeExample: 'Theoretical identities, natural kind terms'
  },
  a_posteriori_contingent: {
    name: 'A Posteriori Contingent',
    description: 'Knowable by experience, could be otherwise',
    examples: ['The sky is blue', 'Biden is president'],
    kripkeExample: 'Ordinary empirical facts'
  }
};

// ─────────────────────────────────────────────────────────────
// NECESSITY TYPES
// ─────────────────────────────────────────────────────────────

const NECESSITY_TYPES = {
  logical: {
    name: 'Logical Necessity',
    description: 'True by logic alone',
    scope: 'All logically possible worlds',
    examples: ['Either P or not-P', 'If P then P'],
    strength: PHI_INV + PHI_INV_3
  },
  metaphysical: {
    name: 'Metaphysical Necessity',
    description: 'True given the nature of reality',
    scope: 'All metaphysically possible worlds',
    examples: ['Water is H₂O', 'Nothing is both red and green all over'],
    strength: PHI_INV
  },
  natural: {
    name: 'Natural/Nomic Necessity',
    description: 'True given laws of nature',
    scope: 'Nomically possible worlds',
    examples: ['Nothing travels faster than light', 'Energy is conserved'],
    strength: PHI_INV_2
  },
  conceptual: {
    name: 'Conceptual Necessity',
    description: 'True by meaning/concept',
    scope: 'All worlds respecting our concepts',
    examples: ['Bachelors are unmarried', 'Triangles have three sides'],
    strength: PHI_INV
  },
  practical: {
    name: 'Practical Necessity',
    description: 'True given practical constraints',
    scope: 'Practically possible worlds',
    examples: ['You must eat to survive', 'Time is limited'],
    strength: PHI_INV_3
  }
};

// ─────────────────────────────────────────────────────────────
// POSSIBLE WORLDS THEORIES
// ─────────────────────────────────────────────────────────────

const POSSIBLE_WORLDS_THEORIES = {
  modal_realism: {
    name: 'Modal Realism (Lewis)',
    description: 'Possible worlds are concrete, real universes',
    worlds_are: 'Real, concrete, spatiotemporally isolated',
    counterparts: true,
    transworld_identity: false
  },
  actualism: {
    name: 'Actualism (Plantinga)',
    description: 'Only the actual world exists; possibilia are abstract',
    worlds_are: 'Maximal states of affairs or properties',
    counterparts: false,
    transworld_identity: true
  },
  fictionalism: {
    name: 'Modal Fictionalism',
    description: 'Possible world talk is useful fiction',
    worlds_are: 'Fictions useful for analysis',
    counterparts: false,
    transworld_identity: 'fictional'
  },
  combinatorialism: {
    name: 'Combinatorialism',
    description: 'Possible worlds are recombinations of actual elements',
    worlds_are: 'Recombinations of actual objects and properties',
    counterparts: false,
    transworld_identity: true
  }
};

// ─────────────────────────────────────────────────────────────
// RIGID DESIGNATOR TYPES (Kripke)
// ─────────────────────────────────────────────────────────────

const DESIGNATOR_TYPES = {
  rigid: {
    name: 'Rigid Designator',
    description: 'Refers to same object in all possible worlds',
    examples: ['Proper names', 'Natural kind terms', 'Indexicals'],
    kripkeExample: '"Nixon", "water", "gold"'
  },
  non_rigid: {
    name: 'Non-Rigid (Flaccid) Designator',
    description: 'May refer to different objects in different worlds',
    examples: ['Definite descriptions'],
    kripkeExample: '"The president in 1970"'
  },
  obstinately_rigid: {
    name: 'Obstinately Rigid',
    description: 'Refers to same object and nothing in worlds where it doesn\'t exist',
    examples: ['Some proper names on certain views']
  },
  persistently_rigid: {
    name: 'Persistently Rigid',
    description: 'Refers to same object only in worlds where it exists',
    examples: ['Most proper names']
  }
};

// ─────────────────────────────────────────────────────────────
// STATE
// ─────────────────────────────────────────────────────────────

const state = {
  worlds: new Map(),              // Possible worlds
  propositions: new Map(),        // Modal propositions
  designators: new Map(),         // Rigid/non-rigid designators
  evaluations: [],                // Modal evaluations
  counterparts: new Map(),        // Counterpart relations
  stats: {
    worldsCreated: 0,
    propositionsTracked: 0,
    evaluationsPerformed: 0,
    counterpartsLinked: 0
  }
};

// The actual world
const ACTUAL_WORLD = '@';

// Storage
const STORAGE_DIR = path.join(os.homedir(), '.cynic', 'modality-engine');
const STATE_FILE = path.join(STORAGE_DIR, 'state.json');
const HISTORY_FILE = path.join(STORAGE_DIR, 'history.jsonl');

// ─────────────────────────────────────────────────────────────
// CORE FUNCTIONS
// ─────────────────────────────────────────────────────────────

/**
 * Create a possible world
 */
function createWorld(id, spec) {
  const world = {
    id,
    name: spec.name || id,
    description: spec.description || '',

    // Is this the actual world?
    actual: id === ACTUAL_WORLD || spec.actual === true,

    // Facts true in this world
    facts: spec.facts || [],

    // Objects existing in this world
    domain: spec.domain || [],

    // Accessibility relations to other worlds
    accessibleFrom: spec.accessibleFrom || [],
    accessibleTo: spec.accessibleTo || [],

    // Distance from actual (for Lewis-style analysis)
    distanceFromActual: spec.distanceFromActual || (id === ACTUAL_WORLD ? 0 : 1),

    created: Date.now()
  };

  state.worlds.set(id, world);
  state.stats.worldsCreated++;

  // Ensure actual world exists
  if (!state.worlds.has(ACTUAL_WORLD)) {
    state.worlds.set(ACTUAL_WORLD, {
      id: ACTUAL_WORLD,
      name: 'Actual World',
      actual: true,
      facts: [],
      domain: [],
      accessibleFrom: [],
      accessibleTo: [id],
      distanceFromActual: 0,
      created: Date.now()
    });
  }

  // Make accessible from actual by default
  if (!world.actual) {
    const actualWorld = state.worlds.get(ACTUAL_WORLD);
    if (!actualWorld.accessibleTo.includes(id)) {
      actualWorld.accessibleTo.push(id);
    }
    world.accessibleFrom.push(ACTUAL_WORLD);
  }

  appendHistory({
    type: 'world_created',
    worldId: id,
    actual: world.actual,
    timestamp: Date.now()
  });

  return world;
}

/**
 * Add a fact to a world
 */
function addFact(worldId, fact) {
  const world = state.worlds.get(worldId);
  if (!world) return null;

  world.facts.push({
    content: fact,
    addedAt: Date.now()
  });

  return { worldId, fact };
}

/**
 * Register a proposition for modal tracking
 */
function registerProposition(id, spec) {
  const proposition = {
    id,
    content: spec.content,

    // Modal status (to be evaluated)
    necessary: null,
    possible: null,
    contingent: null,

    // Category
    category: spec.category || null,
    categoryInfo: MODAL_CATEGORIES[spec.category] || null,

    // Truth in specific worlds
    truthValues: new Map(),

    created: Date.now()
  };

  state.propositions.set(id, proposition);
  state.stats.propositionsTracked++;

  return proposition;
}

/**
 * Evaluate proposition's truth in a world
 */
function evaluateInWorld(propositionId, worldId, truthValue) {
  const proposition = state.propositions.get(propositionId);
  const world = state.worlds.get(worldId);

  if (!proposition || !world) return null;

  proposition.truthValues.set(worldId, truthValue);

  return {
    propositionId,
    worldId,
    truthValue,
    timestamp: Date.now()
  };
}

/**
 * Evaluate modal status of proposition
 */
function evaluateModalStatus(propositionId) {
  const proposition = state.propositions.get(propositionId);
  if (!proposition) return null;

  const evaluation = {
    propositionId,
    content: proposition.content,
    timestamp: Date.now()
  };

  // Get all worlds
  const worlds = Array.from(state.worlds.values());
  const accessibleWorlds = worlds.filter(w =>
    w.actual || state.worlds.get(ACTUAL_WORLD)?.accessibleTo.includes(w.id)
  );

  // Get truth values
  const trueIn = [];
  const falseIn = [];
  const unknownIn = [];

  for (const world of accessibleWorlds) {
    const tv = proposition.truthValues.get(world.id);
    if (tv === true) trueIn.push(world.id);
    else if (tv === false) falseIn.push(world.id);
    else unknownIn.push(world.id);
  }

  // Determine modal status
  if (trueIn.length === accessibleWorlds.length && unknownIn.length === 0) {
    evaluation.necessary = true;
    evaluation.possible = true;
    evaluation.contingent = false;
    evaluation.status = 'necessary';
    evaluation.operator = '□';
  } else if (falseIn.length === accessibleWorlds.length && unknownIn.length === 0) {
    evaluation.necessary = false;
    evaluation.possible = false;
    evaluation.contingent = false;
    evaluation.status = 'impossible';
    evaluation.operator = '¬◇';
  } else if (trueIn.length > 0 && falseIn.length > 0) {
    evaluation.necessary = false;
    evaluation.possible = true;
    evaluation.contingent = true;
    evaluation.status = 'contingent';
    evaluation.operator = '◇∧◇¬';
  } else if (trueIn.length > 0) {
    evaluation.necessary = unknownIn.length === 0 ? false : null;
    evaluation.possible = true;
    evaluation.contingent = unknownIn.length === 0 ? true : null;
    evaluation.status = 'at_least_possible';
    evaluation.operator = '◇';
  } else {
    evaluation.status = 'undetermined';
    evaluation.operator = '?';
  }

  evaluation.distribution = {
    trueIn,
    falseIn,
    unknownIn,
    totalWorlds: accessibleWorlds.length
  };

  evaluation.confidence = unknownIn.length === 0
    ? PHI_INV
    : PHI_INV * (1 - unknownIn.length / accessibleWorlds.length);

  // Update proposition
  proposition.necessary = evaluation.necessary;
  proposition.possible = evaluation.possible;
  proposition.contingent = evaluation.contingent;

  state.evaluations.push(evaluation);
  state.stats.evaluationsPerformed++;

  appendHistory({
    type: 'modal_evaluation',
    evaluation,
    timestamp: Date.now()
  });

  return evaluation;
}

/**
 * Register a designator (name/term)
 */
function registerDesignator(id, spec) {
  const designator = {
    id,
    term: spec.term,
    type: spec.type || 'rigid',
    typeInfo: DESIGNATOR_TYPES[spec.type || 'rigid'],

    // What it refers to in each world
    referents: new Map(),

    // For rigid designators: the fixed referent
    rigidReferent: spec.rigidReferent || null,

    created: Date.now()
  };

  // If rigid, set same referent for all worlds
  if (designator.type === 'rigid' && designator.rigidReferent) {
    for (const [worldId] of state.worlds) {
      designator.referents.set(worldId, designator.rigidReferent);
    }
  }

  state.designators.set(id, designator);

  return designator;
}

/**
 * Get referent of designator in world
 */
function getReferent(designatorId, worldId) {
  const designator = state.designators.get(designatorId);
  if (!designator) return null;

  if (designator.type === 'rigid') {
    return {
      designator: designatorId,
      world: worldId,
      referent: designator.rigidReferent,
      rigid: true,
      note: 'Rigid designator: same referent in all worlds'
    };
  }

  return {
    designator: designatorId,
    world: worldId,
    referent: designator.referents.get(worldId),
    rigid: false,
    note: 'Non-rigid designator: may vary across worlds'
  };
}

/**
 * Link counterparts across worlds (Lewis style)
 */
function linkCounterparts(entity1, world1Id, entity2, world2Id) {
  const key = `${entity1}@${world1Id}`;

  if (!state.counterparts.has(key)) {
    state.counterparts.set(key, []);
  }

  state.counterparts.get(key).push({
    counterpart: entity2,
    world: world2Id,
    linkedAt: Date.now()
  });

  state.stats.counterpartsLinked++;

  return {
    entity: entity1,
    world: world1Id,
    counterpart: entity2,
    counterpartWorld: world2Id
  };
}

/**
 * Get counterparts of entity
 */
function getCounterparts(entity, worldId) {
  const key = `${entity}@${worldId}`;
  return state.counterparts.get(key) || [];
}

/**
 * Evaluate necessity claim (Kripke style)
 */
function evaluateNecessity(claim, necessityType = 'metaphysical') {
  const typeInfo = NECESSITY_TYPES[necessityType];

  const evaluation = {
    claim,
    necessityType,
    typeInfo,
    timestamp: Date.now()
  };

  // Check if claim matches pattern for this necessity type
  // (Simplified - full implementation would need proposition analysis)

  // Logical necessity: tautologies
  if (necessityType === 'logical') {
    const tautologyPatterns = [
      /either .* or not/i,
      /if .* then/i,
      /= .*/
    ];
    evaluation.isNecessary = tautologyPatterns.some(p => p.test(claim));
  }

  // Metaphysical necessity: identity statements, natural kinds
  if (necessityType === 'metaphysical') {
    const necessaryPatterns = [
      /is identical to/i,
      /is H2O/i,
      /is gold/i,
      /= /
    ];
    evaluation.isNecessary = necessaryPatterns.some(p => p.test(claim));
  }

  evaluation.confidence = evaluation.isNecessary !== undefined
    ? PHI_INV_2
    : PHI_INV_3;

  evaluation.kripkeNote = evaluation.isNecessary
    ? 'If true, necessarily true (Kripke)'
    : 'Contingent or undetermined';

  return evaluation;
}

/**
 * Check a priori / a posteriori status
 */
function classifyEpistemicModal(propositionId) {
  const proposition = state.propositions.get(propositionId);
  if (!proposition) return null;

  const evaluation = evaluateModalStatus(propositionId);

  // Determine epistemic status (simplified heuristic)
  const content = proposition.content.toLowerCase();

  let aPriori = false;

  // A priori patterns
  const aPrioriPatterns = [
    /\d+ \+ \d+ =/,          // Math
    /all .* are/i,           // Analytic
    /by definition/i,
    /bachelor/i              // Classic example
  ];

  aPriori = aPrioriPatterns.some(p => p.test(content));

  const classification = {
    propositionId,
    content: proposition.content,
    aPriori,
    necessary: evaluation?.necessary,
    category: null,
    confidence: PHI_INV_2
  };

  // Determine Kripkean category
  if (aPriori && classification.necessary) {
    classification.category = 'a_priori_necessary';
  } else if (aPriori && !classification.necessary) {
    classification.category = 'a_priori_contingent';
  } else if (!aPriori && classification.necessary) {
    classification.category = 'a_posteriori_necessary';
  } else {
    classification.category = 'a_posteriori_contingent';
  }

  classification.categoryInfo = MODAL_CATEGORIES[classification.category];
  classification.kripkeInsight = classification.category === 'a_posteriori_necessary'
    ? 'Necessary truth discoverable only empirically!'
    : null;

  return classification;
}

// ─────────────────────────────────────────────────────────────
// PERSISTENCE
// ─────────────────────────────────────────────────────────────

function ensureStorageDir() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }
}

function saveState() {
  ensureStorageDir();

  const serializable = {
    worlds: Array.from(state.worlds.entries()),
    propositions: Array.from(state.propositions.entries()).map(([id, p]) => [id, {
      ...p,
      truthValues: Array.from(p.truthValues.entries())
    }]),
    designators: Array.from(state.designators.entries()).map(([id, d]) => [id, {
      ...d,
      referents: Array.from(d.referents.entries())
    }]),
    evaluations: state.evaluations.slice(-50),
    counterparts: Array.from(state.counterparts.entries()),
    stats: state.stats
  };

  fs.writeFileSync(STATE_FILE, JSON.stringify(serializable, null, 2));
}

function loadState() {
  ensureStorageDir();

  if (fs.existsSync(STATE_FILE)) {
    try {
      const data = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
      state.worlds = new Map(data.worlds || []);
      state.propositions = new Map(
        (data.propositions || []).map(([id, p]) => [id, {
          ...p,
          truthValues: new Map(p.truthValues || [])
        }])
      );
      state.designators = new Map(
        (data.designators || []).map(([id, d]) => [id, {
          ...d,
          referents: new Map(d.referents || [])
        }])
      );
      state.evaluations = data.evaluations || [];
      state.counterparts = new Map(data.counterparts || []);
      state.stats = data.stats || state.stats;
    } catch (e) {
      console.error('Failed to load modality engine state:', e.message);
    }
  }

  // Ensure actual world exists
  if (!state.worlds.has(ACTUAL_WORLD)) {
    createWorld(ACTUAL_WORLD, { actual: true });
  }
}

function appendHistory(entry) {
  ensureStorageDir();
  fs.appendFileSync(HISTORY_FILE, JSON.stringify(entry) + '\n');
}

// ─────────────────────────────────────────────────────────────
// FORMATTING
// ─────────────────────────────────────────────────────────────

function formatStatus() {
  const lines = [
    '── MODALITY ENGINE ────────────────────────────────────────',
    ''
  ];

  lines.push(`   Worlds: ${state.worlds.size} | Propositions: ${state.propositions.size}`);
  lines.push(`   Evaluations: ${state.stats.evaluationsPerformed} | Counterparts: ${state.stats.counterpartsLinked}`);
  lines.push('');

  // Worlds
  if (state.worlds.size > 0) {
    lines.push('   Worlds:');
    const worlds = Array.from(state.worlds.values()).slice(0, 3);
    for (const w of worlds) {
      const marker = w.actual ? '@' : '◇';
      lines.push(`   ${marker} ${w.name} (${w.facts.length} facts)`);
    }
  }

  lines.push('');
  return lines.join('\n');
}

function getStats() {
  return {
    ...state.stats,
    worldCount: state.worlds.size,
    propositionCount: state.propositions.size,
    designatorCount: state.designators.size
  };
}

// ─────────────────────────────────────────────────────────────
// INITIALIZATION
// ─────────────────────────────────────────────────────────────

function init() {
  loadState();

  // Auto-save periodically
  setInterval(() => saveState(), 60000);

  return {
    initialized: true,
    worlds: state.worlds.size,
    propositions: state.propositions.size
  };
}

// ─────────────────────────────────────────────────────────────
// EXPORTS
// ─────────────────────────────────────────────────────────────

module.exports = {
  // Constants
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,
  ACTUAL_WORLD,

  // Type definitions
  MODAL_OPERATORS,
  MODAL_CATEGORIES,
  NECESSITY_TYPES,
  POSSIBLE_WORLDS_THEORIES,
  DESIGNATOR_TYPES,

  // Core functions
  createWorld,
  addFact,
  registerProposition,
  evaluateInWorld,
  evaluateModalStatus,
  registerDesignator,
  getReferent,
  linkCounterparts,
  getCounterparts,
  evaluateNecessity,
  classifyEpistemicModal,

  // State access
  getWorld: (id) => state.worlds.get(id),
  getProposition: (id) => state.propositions.get(id),
  getDesignator: (id) => state.designators.get(id),
  getAllWorlds: () => Array.from(state.worlds.values()),

  // Persistence
  saveState,
  loadState,

  // Formatting
  formatStatus,
  getStats,
  init
};

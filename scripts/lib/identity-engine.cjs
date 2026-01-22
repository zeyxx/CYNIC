/**
 * CYNIC Identity Engine
 *
 * "What makes something the same thing over time"
 *
 * Philosophical foundations:
 * - Leibniz: Identity of indiscernibles
 * - Locke: Personal identity through memory
 * - Parfit: Psychological continuity
 * - Ship of Theseus: Material constitution
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
// IDENTITY CRITERIA
// ─────────────────────────────────────────────────────────────

const IDENTITY_CRITERIA = {
  // Leibniz's Law
  indiscernibility: {
    name: 'Identity of Indiscernibles',
    description: 'If x and y share all properties, x = y',
    formula: '∀P(Px ↔ Py) → x = y',
    source: 'Leibniz',
    strength: PHI_INV
  },

  // Numerical identity
  numerical: {
    name: 'Numerical Identity',
    description: 'x and y are the very same object',
    formula: 'x = y',
    source: 'Classical logic',
    strength: PHI_INV + PHI_INV_3
  },

  // Qualitative identity
  qualitative: {
    name: 'Qualitative Identity',
    description: 'x and y share all qualitative properties',
    formula: '∀Q(Qx ↔ Qy) where Q is qualitative',
    source: 'Classical',
    strength: PHI_INV_2
  },

  // Spatiotemporal continuity
  spatiotemporal: {
    name: 'Spatiotemporal Continuity',
    description: 'Continuous path through space-time',
    formula: 'Continuous trajectory from t1 to t2',
    source: 'Material constitution',
    strength: PHI_INV
  },

  // Causal continuity
  causal: {
    name: 'Causal Continuity',
    description: 'Later states causally connected to earlier',
    formula: 'x-at-t2 caused by x-at-t1',
    source: 'Material persistence',
    strength: PHI_INV_2
  }
};

// ─────────────────────────────────────────────────────────────
// PERSONAL IDENTITY THEORIES
// ─────────────────────────────────────────────────────────────

const PERSONAL_IDENTITY_THEORIES = {
  psychological: {
    name: 'Psychological Continuity',
    description: 'Identity via connected mental states',
    criterion: 'Memory chains and psychological connections',
    source: 'Locke, Parfit',
    strength: PHI_INV
  },
  biological: {
    name: 'Biological Continuity',
    description: 'Same living organism',
    criterion: 'Continuous biological life',
    source: 'Animalism',
    strength: PHI_INV_2
  },
  soul: {
    name: 'Soul Theory',
    description: 'Same immaterial soul',
    criterion: 'Persistence of immaterial substance',
    source: 'Cartesian',
    strength: PHI_INV_3 // φ skepticism
  },
  narrative: {
    name: 'Narrative Identity',
    description: 'Connected life story',
    criterion: 'Unified autobiographical narrative',
    source: 'MacIntyre, Ricoeur',
    strength: PHI_INV_2
  },
  no_self: {
    name: 'No-Self View',
    description: 'Personal identity is illusion',
    criterion: 'Bundle of experiences with no owner',
    source: 'Hume, Buddhism, Parfit',
    strength: PHI_INV_3
  }
};

// ─────────────────────────────────────────────────────────────
// PERSISTENCE THEORIES (for objects)
// ─────────────────────────────────────────────────────────────

const PERSISTENCE_THEORIES = {
  endurantism: {
    name: 'Endurantism',
    description: 'Objects wholly present at each moment',
    criterion: 'Three-dimensional entity persisting',
    metaphor: 'Object moving through time',
    strength: PHI_INV_2
  },
  perdurantism: {
    name: 'Perdurantism',
    description: 'Objects have temporal parts',
    criterion: 'Four-dimensional "worm" through spacetime',
    metaphor: 'Object extended in time like space',
    strength: PHI_INV_2
  },
  exdurantism: {
    name: 'Exdurantism (Stage Theory)',
    description: 'Only momentary stages exist',
    criterion: 'Temporal counterpart relation',
    metaphor: 'Series of related stages',
    strength: PHI_INV_3
  }
};

// ─────────────────────────────────────────────────────────────
// CHANGE TYPES
// ─────────────────────────────────────────────────────────────

const CHANGE_TYPES = {
  qualitative: {
    name: 'Qualitative Change',
    description: 'Change in properties',
    preserves_identity: true,
    examples: ['Color change', 'Shape change']
  },
  quantitative: {
    name: 'Quantitative Change',
    description: 'Change in amount/degree',
    preserves_identity: true,
    examples: ['Growth', 'Shrinkage']
  },
  substantial: {
    name: 'Substantial Change',
    description: 'Change in fundamental nature',
    preserves_identity: false,
    examples: ['Death', 'Destruction', 'Transformation']
  },
  mereological: {
    name: 'Mereological Change',
    description: 'Change in parts',
    preserves_identity: 'debated', // Ship of Theseus
    examples: ['Part replacement', 'Addition', 'Removal']
  }
};

// ─────────────────────────────────────────────────────────────
// STATE
// ─────────────────────────────────────────────────────────────

const state = {
  entities: new Map(),            // Tracked entities
  identityJudgments: [],          // Identity assessments
  changes: [],                    // Change records
  persistenceRecords: [],         // Persistence tracking
  stats: {
    entitiesTracked: 0,
    judgmentsMade: 0,
    changesRecorded: 0,
    puzzlesAnalyzed: 0
  }
};

// Storage
const STORAGE_DIR = path.join(os.homedir(), '.cynic', 'identity-engine');
const STATE_FILE = path.join(STORAGE_DIR, 'state.json');
const HISTORY_FILE = path.join(STORAGE_DIR, 'history.jsonl');

// ─────────────────────────────────────────────────────────────
// CORE FUNCTIONS
// ─────────────────────────────────────────────────────────────

/**
 * Register an entity for identity tracking
 */
function registerEntity(id, spec) {
  const entity = {
    id,
    name: spec.name || id,
    type: spec.type || 'object', // 'object', 'person', 'abstract'

    // Current state
    properties: spec.properties || {},

    // Essential vs accidental properties
    essentialProperties: spec.essentialProperties || [],
    accidentalProperties: spec.accidentalProperties || [],

    // History
    states: [{
      timestamp: Date.now(),
      properties: { ...spec.properties }
    }],

    // Parts (for mereological tracking)
    parts: spec.parts || [],

    // For persons
    memories: spec.memories || [],
    psychologicalTraits: spec.psychologicalTraits || [],

    created: Date.now()
  };

  state.entities.set(id, entity);
  state.stats.entitiesTracked++;

  appendHistory({
    type: 'entity_registered',
    entityId: id,
    timestamp: Date.now()
  });

  return entity;
}

/**
 * Record a change to an entity
 */
function recordChange(entityId, changeSpec) {
  const entity = state.entities.get(entityId);
  if (!entity) return null;

  const change = {
    id: `change_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
    entityId,

    type: changeSpec.type || 'qualitative',
    typeInfo: CHANGE_TYPES[changeSpec.type || 'qualitative'],

    // What changed
    property: changeSpec.property,
    oldValue: changeSpec.oldValue,
    newValue: changeSpec.newValue,

    // For mereological change
    partAdded: changeSpec.partAdded,
    partRemoved: changeSpec.partRemoved,

    timestamp: Date.now()
  };

  // Update entity
  if (change.property) {
    entity.properties[change.property] = change.newValue;
  }
  if (change.partAdded) {
    entity.parts.push(change.partAdded);
  }
  if (change.partRemoved) {
    const idx = entity.parts.indexOf(change.partRemoved);
    if (idx !== -1) entity.parts.splice(idx, 1);
  }

  // Record new state
  entity.states.push({
    timestamp: Date.now(),
    properties: { ...entity.properties },
    parts: [...entity.parts]
  });

  state.changes.push(change);
  state.stats.changesRecorded++;

  // Check if identity preserved
  change.identityPreserved = assessIdentityThroughChange(entity, change);

  appendHistory({
    type: 'change_recorded',
    change,
    timestamp: Date.now()
  });

  return change;
}

/**
 * Assess whether identity is preserved through change
 */
function assessIdentityThroughChange(entity, change) {
  // Essential property change breaks identity
  if (entity.essentialProperties.includes(change.property)) {
    return {
      preserved: false,
      reason: 'Essential property changed',
      confidence: PHI_INV
    };
  }

  // Substantial change breaks identity
  if (change.type === 'substantial') {
    return {
      preserved: false,
      reason: 'Substantial change occurred',
      confidence: PHI_INV
    };
  }

  // Mereological change - debated
  if (change.type === 'mereological') {
    return {
      preserved: 'debated',
      reason: 'Part change - identity debated (Ship of Theseus)',
      confidence: PHI_INV_2
    };
  }

  return {
    preserved: true,
    reason: 'Non-essential change',
    confidence: PHI_INV
  };
}

/**
 * Judge numerical identity between two entities
 */
function judgeIdentity(entity1Id, entity2Id, criterion = 'indiscernibility') {
  const e1 = state.entities.get(entity1Id);
  const e2 = state.entities.get(entity2Id);

  if (!e1 || !e2) return null;

  const criterionInfo = IDENTITY_CRITERIA[criterion];

  const judgment = {
    id: `judgment_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
    entities: [entity1Id, entity2Id],
    criterion,
    criterionInfo,

    result: null,
    confidence: PHI_INV_2,

    timestamp: Date.now()
  };

  switch (criterion) {
    case 'indiscernibility': {
      // Leibniz's Law: all properties same?
      const props1 = JSON.stringify(e1.properties);
      const props2 = JSON.stringify(e2.properties);
      judgment.result = props1 === props2;
      judgment.reason = judgment.result
        ? 'All properties match (Leibniz satisfied)'
        : 'Properties differ';
      judgment.confidence = judgment.result ? PHI_INV : PHI_INV_2;
      break;
    }

    case 'numerical': {
      // Same ID = same object
      judgment.result = entity1Id === entity2Id;
      judgment.reason = judgment.result
        ? 'Same identifier'
        : 'Different identifiers';
      judgment.confidence = PHI_INV;
      break;
    }

    case 'qualitative': {
      // All qualitative properties same?
      const qual1 = Object.entries(e1.properties).filter(([k]) =>
        !k.startsWith('_') // Exclude internal properties
      );
      const qual2 = Object.entries(e2.properties).filter(([k]) =>
        !k.startsWith('_')
      );
      judgment.result = JSON.stringify(qual1) === JSON.stringify(qual2);
      judgment.reason = judgment.result
        ? 'Qualitatively identical'
        : 'Qualitative differences exist';
      judgment.confidence = PHI_INV_2;
      break;
    }

    case 'spatiotemporal': {
      // Check for continuous path
      judgment.result = checkSpatiotemporalContinuity(e1, e2);
      judgment.reason = judgment.result
        ? 'Spatiotemporal continuity found'
        : 'No continuous path';
      judgment.confidence = PHI_INV_2;
      break;
    }

    default:
      judgment.result = null;
      judgment.reason = 'Unknown criterion';
      judgment.confidence = PHI_INV_3;
  }

  state.identityJudgments.push(judgment);
  state.stats.judgmentsMade++;

  return judgment;
}

/**
 * Check spatiotemporal continuity between entity states
 */
function checkSpatiotemporalContinuity(e1, e2) {
  // If they're the same entity tracked over time, check state history
  if (e1.id === e2.id) {
    return e1.states.length > 1;
  }

  // Otherwise, check if they could be same entity at different times
  // (simplified - would need location tracking for full implementation)
  return false;
}

/**
 * Assess personal identity (Locke/Parfit style)
 */
function assessPersonalIdentity(person1Id, person2Id, theory = 'psychological') {
  const p1 = state.entities.get(person1Id);
  const p2 = state.entities.get(person2Id);

  if (!p1 || !p2) return null;

  const theoryInfo = PERSONAL_IDENTITY_THEORIES[theory];

  const assessment = {
    persons: [person1Id, person2Id],
    theory,
    theoryInfo,
    result: null,
    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  switch (theory) {
    case 'psychological': {
      // Memory chains and psychological connections
      const sharedMemories = p1.memories.filter(m =>
        p2.memories.includes(m)
      );
      const sharedTraits = p1.psychologicalTraits.filter(t =>
        p2.psychologicalTraits.includes(t)
      );

      const memoryRatio = p1.memories.length > 0
        ? sharedMemories.length / p1.memories.length
        : 0;
      const traitRatio = p1.psychologicalTraits.length > 0
        ? sharedTraits.length / p1.psychologicalTraits.length
        : 0;

      const continuityScore = (memoryRatio + traitRatio) / 2;

      assessment.result = continuityScore >= PHI_INV_2;
      assessment.continuityScore = continuityScore;
      assessment.reason = assessment.result
        ? `Sufficient psychological continuity (${(continuityScore * 100).toFixed(0)}%)`
        : `Insufficient psychological continuity (${(continuityScore * 100).toFixed(0)}%)`;
      assessment.confidence = Math.min(continuityScore + PHI_INV_3, PHI_INV);
      break;
    }

    case 'biological': {
      // Check for biological continuity markers
      const sameBiology = p1.properties.biologicalId === p2.properties.biologicalId;
      assessment.result = sameBiology;
      assessment.reason = sameBiology
        ? 'Same biological organism'
        : 'Different biological organisms';
      assessment.confidence = PHI_INV_2;
      break;
    }

    case 'narrative': {
      // Check narrative connection
      const narrativeConnected = p1.properties.narrative &&
        p2.properties.narrative &&
        p1.properties.narrative === p2.properties.narrative;
      assessment.result = narrativeConnected;
      assessment.reason = narrativeConnected
        ? 'Connected life narrative'
        : 'Disconnected narratives';
      assessment.confidence = PHI_INV_3;
      break;
    }

    case 'no_self': {
      // Parfit's view: identity doesn't matter
      assessment.result = null;
      assessment.reason = 'Personal identity is not what matters; only continuity/connectedness';
      assessment.parfitianInsight = 'Survival is not all-or-nothing';
      assessment.confidence = PHI_INV_2;
      break;
    }

    default:
      assessment.result = null;
      assessment.reason = 'Unknown theory';
  }

  return assessment;
}

/**
 * Analyze Ship of Theseus puzzle
 */
function analyzeShipOfTheseus(originalId, repairedId, reassembledId) {
  const original = state.entities.get(originalId);
  const repaired = state.entities.get(repairedId);
  const reassembled = state.entities.get(reassembledId);

  state.stats.puzzlesAnalyzed++;

  const analysis = {
    puzzle: 'Ship of Theseus',
    entities: {
      original: originalId,
      repaired: repairedId,
      reassembled: reassembledId
    },
    perspectives: {},
    timestamp: Date.now()
  };

  // Spatiotemporal continuity view
  analysis.perspectives.spatiotemporal = {
    answer: `${repairedId} is the ship`,
    reason: 'Spatiotemporally continuous with original'
  };

  // Sortal essence view
  analysis.perspectives.sortal = {
    answer: `${repairedId} is the ship`,
    reason: 'Maintained functional/sortal essence'
  };

  // Material constitution view
  analysis.perspectives.material = {
    answer: `${reassembledId} is the ship`,
    reason: 'Made of original matter'
  };

  // Four-dimensionalist view
  analysis.perspectives.perdurantist = {
    answer: 'Both are temporal parts of different four-dimensional objects',
    reason: 'No strict identity over time'
  };

  analysis.verdict = {
    text: 'No determinate answer - shows limits of ordinary identity concept',
    confidence: PHI_INV_3,
    lesson: 'Identity may be indeterminate or vague'
  };

  return analysis;
}

/**
 * Track persistence over time
 */
function trackPersistence(entityId, theory = 'endurantism') {
  const entity = state.entities.get(entityId);
  if (!entity) return null;

  const theoryInfo = PERSISTENCE_THEORIES[theory];

  const record = {
    entityId,
    theory,
    theoryInfo,
    stateCount: entity.states.length,
    timespan: entity.states.length > 1
      ? entity.states[entity.states.length - 1].timestamp - entity.states[0].timestamp
      : 0,
    interpretation: null,
    timestamp: Date.now()
  };

  switch (theory) {
    case 'endurantism':
      record.interpretation = {
        view: 'Entity wholly present at each moment',
        states: `${entity.states.length} observed states of same 3D entity`,
        identity: 'Strict identity across time'
      };
      break;

    case 'perdurantism':
      record.interpretation = {
        view: 'Entity has temporal parts',
        states: `${entity.states.length} temporal parts observed`,
        identity: 'Genidentity (different stages of 4D worm)'
      };
      break;

    case 'exdurantism':
      record.interpretation = {
        view: 'Only momentary stages exist',
        states: `${entity.states.length} distinct stages, related by counterpart relation`,
        identity: 'No strict identity - counterparts'
      };
      break;
  }

  state.persistenceRecords.push(record);

  return record;
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
    entities: Array.from(state.entities.entries()),
    identityJudgments: state.identityJudgments.slice(-50),
    changes: state.changes.slice(-100),
    persistenceRecords: state.persistenceRecords.slice(-50),
    stats: state.stats
  };

  fs.writeFileSync(STATE_FILE, JSON.stringify(serializable, null, 2));
}

function loadState() {
  ensureStorageDir();

  if (fs.existsSync(STATE_FILE)) {
    try {
      const data = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
      state.entities = new Map(data.entities || []);
      state.identityJudgments = data.identityJudgments || [];
      state.changes = data.changes || [];
      state.persistenceRecords = data.persistenceRecords || [];
      state.stats = data.stats || state.stats;
    } catch (e) {
      console.error('Failed to load identity engine state:', e.message);
    }
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
    '── IDENTITY ENGINE ────────────────────────────────────────',
    ''
  ];

  lines.push(`   Entities: ${state.entities.size} | Judgments: ${state.stats.judgmentsMade}`);
  lines.push(`   Changes: ${state.stats.changesRecorded} | Puzzles: ${state.stats.puzzlesAnalyzed}`);
  lines.push('');

  return lines.join('\n');
}

function getStats() {
  return {
    ...state.stats,
    entityCount: state.entities.size,
    recentJudgments: state.identityJudgments.slice(-5)
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
    entities: state.entities.size
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

  // Type definitions
  IDENTITY_CRITERIA,
  PERSONAL_IDENTITY_THEORIES,
  PERSISTENCE_THEORIES,
  CHANGE_TYPES,

  // Core functions
  registerEntity,
  recordChange,
  judgeIdentity,
  assessPersonalIdentity,
  analyzeShipOfTheseus,
  trackPersistence,

  // State access
  getEntity: (id) => state.entities.get(id),
  getChanges: (entityId) => state.changes.filter(c => c.entityId === entityId),
  getJudgments: () => [...state.identityJudgments],

  // Persistence
  saveState,
  loadState,

  // Formatting
  formatStatus,
  getStats,
  init
};

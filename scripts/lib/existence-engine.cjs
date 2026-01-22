/**
 * CYNIC Existence Engine
 *
 * "What there is and how we know"
 *
 * Philosophical foundations:
 * - Quine: Ontological commitment ("To be is to be the value of a variable")
 * - Heidegger: Being and beings, Dasein
 * - Meinong: Non-existent objects
 * - Carnap: Internal/external questions
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
// ONTOLOGICAL CATEGORIES
// ─────────────────────────────────────────────────────────────

const ONTOLOGICAL_CATEGORIES = {
  // Concrete entities
  physical_object: {
    name: 'Physical Object',
    description: 'Spatiotemporally located material entity',
    abstract: false,
    examples: ['tables', 'electrons', 'planets'],
    weight: PHI_INV
  },
  event: {
    name: 'Event',
    description: 'Occurrence in space-time',
    abstract: false,
    examples: ['explosions', 'births', 'decisions'],
    weight: PHI_INV_2
  },
  process: {
    name: 'Process',
    description: 'Temporally extended occurrence',
    abstract: false,
    examples: ['erosion', 'evolution', 'computation'],
    weight: PHI_INV_2
  },

  // Abstract entities
  number: {
    name: 'Number',
    description: 'Mathematical abstract object',
    abstract: true,
    examples: ['2', 'π', '∞'],
    weight: PHI_INV_2
  },
  property: {
    name: 'Property',
    description: 'Universal or attribute',
    abstract: true,
    examples: ['redness', 'wisdom', 'triangularity'],
    weight: PHI_INV_2
  },
  proposition: {
    name: 'Proposition',
    description: 'Abstract truth-bearer',
    abstract: true,
    examples: ['that snow is white', 'that 2+2=4'],
    weight: PHI_INV_2
  },
  set: {
    name: 'Set',
    description: 'Collection of objects',
    abstract: true,
    examples: ['∅', '{1,2,3}', 'ℕ'],
    weight: PHI_INV_3
  },

  // Mental entities
  mental_state: {
    name: 'Mental State',
    description: 'Psychological entity',
    abstract: false, // Debated
    examples: ['beliefs', 'desires', 'pains'],
    weight: PHI_INV_2
  },

  // Social entities
  social_entity: {
    name: 'Social Entity',
    description: 'Socially constructed object',
    abstract: false, // Debated
    examples: ['money', 'marriages', 'nations'],
    weight: PHI_INV_2
  },

  // Fictional entities
  fictional: {
    name: 'Fictional Entity',
    description: 'Exists in fiction only',
    abstract: true,
    examples: ['Sherlock Holmes', 'unicorns'],
    weight: PHI_INV_3
  }
};

// ─────────────────────────────────────────────────────────────
// EXISTENCE MODES (Heidegger-inspired)
// ─────────────────────────────────────────────────────────────

const EXISTENCE_MODES = {
  actual: {
    name: 'Actual Existence',
    description: 'Exists in the actual world',
    symbol: '∃',
    strength: PHI_INV
  },
  possible: {
    name: 'Possible Existence',
    description: 'Could exist in some possible world',
    symbol: '◇∃',
    strength: PHI_INV_2
  },
  necessary: {
    name: 'Necessary Existence',
    description: 'Must exist in all possible worlds',
    symbol: '□∃',
    strength: PHI_INV + PHI_INV_3
  },
  subsistent: {
    name: 'Subsistence',
    description: 'Has being without full existence (Meinong)',
    symbol: '≈∃',
    strength: PHI_INV_3
  },
  intentional: {
    name: 'Intentional Existence',
    description: 'Exists as object of thought',
    symbol: '∃ᵢ',
    strength: PHI_INV_3
  }
};

// ─────────────────────────────────────────────────────────────
// ONTOLOGICAL COMMITMENT TYPES (Quine)
// ─────────────────────────────────────────────────────────────

const COMMITMENT_TYPES = {
  explicit: {
    name: 'Explicit Commitment',
    description: 'Theory explicitly quantifies over entity',
    strength: PHI_INV
  },
  implicit: {
    name: 'Implicit Commitment',
    description: 'Commitment implied by theory',
    strength: PHI_INV_2
  },
  apparent: {
    name: 'Apparent Commitment',
    description: 'Surface grammar suggests commitment',
    strength: PHI_INV_3
  },
  reducible: {
    name: 'Reducible Commitment',
    description: 'Can be paraphrased away',
    strength: PHI_INV_3
  }
};

// ─────────────────────────────────────────────────────────────
// STATE
// ─────────────────────────────────────────────────────────────

const state = {
  entities: new Map(),            // Registered entities
  ontologies: new Map(),          // Ontological theories
  commitments: [],                // Ontological commitments
  existenceQueries: [],           // Existence questions
  categories: new Map(),          // Category assignments
  stats: {
    entitiesRegistered: 0,
    ontologiesCreated: 0,
    commitmentsTracked: 0,
    queriesProcessed: 0
  }
};

// Storage
const STORAGE_DIR = path.join(os.homedir(), '.cynic', 'existence-engine');
const STATE_FILE = path.join(STORAGE_DIR, 'state.json');
const HISTORY_FILE = path.join(STORAGE_DIR, 'history.jsonl');

// ─────────────────────────────────────────────────────────────
// CORE FUNCTIONS
// ─────────────────────────────────────────────────────────────

/**
 * Register an entity for ontological tracking
 */
function registerEntity(id, spec) {
  const entity = {
    id,
    name: spec.name || id,
    description: spec.description || '',

    // Ontological status
    category: spec.category || 'physical_object',
    categoryInfo: ONTOLOGICAL_CATEGORIES[spec.category || 'physical_object'],

    // Existence mode
    existenceMode: spec.existenceMode || 'actual',
    existenceModeInfo: EXISTENCE_MODES[spec.existenceMode || 'actual'],

    // Properties
    properties: spec.properties || [],
    relations: spec.relations || [],

    // Metaphysical status
    abstract: spec.abstract || false,
    necessary: spec.necessary || false,
    contingent: !spec.necessary,

    // Quine: what theory commits to this?
    commitmentSources: [],

    created: Date.now()
  };

  state.entities.set(id, entity);
  state.stats.entitiesRegistered++;

  // Track category
  if (!state.categories.has(entity.category)) {
    state.categories.set(entity.category, []);
  }
  state.categories.get(entity.category).push(id);

  appendHistory({
    type: 'entity_registered',
    entityId: id,
    category: entity.category,
    timestamp: Date.now()
  });

  return entity;
}

/**
 * Create an ontology (theory about what exists)
 */
function createOntology(id, spec) {
  const ontology = {
    id,
    name: spec.name || id,
    description: spec.description || '',

    // What categories are admitted
    admittedCategories: spec.admittedCategories || Object.keys(ONTOLOGICAL_CATEGORIES),

    // Specific commitments
    commitments: [],

    // Ontological principles
    principles: spec.principles || [],

    // Position on abstract entities
    abstractRealism: spec.abstractRealism !== false,

    // Parsimony level (Occam's razor)
    parsimony: spec.parsimony || 'moderate', // 'strict', 'moderate', 'liberal'

    created: Date.now()
  };

  state.ontologies.set(id, ontology);
  state.stats.ontologiesCreated++;

  appendHistory({
    type: 'ontology_created',
    ontologyId: id,
    timestamp: Date.now()
  });

  return ontology;
}

/**
 * Commit to existence of entity within an ontology
 * (Quine: "To be is to be the value of a variable")
 */
function commitToExistence(ontologyId, entityId, commitmentType = 'explicit') {
  const ontology = state.ontologies.get(ontologyId);
  const entity = state.entities.get(entityId);

  if (!ontology) return null;

  const commitment = {
    id: `commit_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
    ontologyId,
    entityId,

    type: commitmentType,
    typeInfo: COMMITMENT_TYPES[commitmentType],

    // Is this commitment compatible with ontology's principles?
    compatible: entity
      ? ontology.admittedCategories.includes(entity.category)
      : true,

    // Quine's criterion
    quantifiedOver: true,

    timestamp: Date.now()
  };

  ontology.commitments.push(commitment);
  state.commitments.push(commitment);
  state.stats.commitmentsTracked++;

  if (entity) {
    entity.commitmentSources.push({
      ontology: ontologyId,
      commitment: commitment.id
    });
  }

  appendHistory({
    type: 'commitment',
    commitment,
    timestamp: Date.now()
  });

  return commitment;
}

/**
 * Query existence status
 */
function queryExistence(query) {
  const result = {
    query,
    timestamp: Date.now(),
    findings: []
  };

  // Search entities
  for (const [id, entity] of state.entities) {
    if (id.includes(query) || entity.name.toLowerCase().includes(query.toLowerCase())) {
      result.findings.push({
        entityId: id,
        entity,
        existenceMode: entity.existenceMode,
        category: entity.category,
        confidence: entity.existenceModeInfo?.strength || PHI_INV_2
      });
    }
  }

  // Categorize findings
  result.actuallyExisting = result.findings.filter(f => f.existenceMode === 'actual');
  result.possiblyExisting = result.findings.filter(f => f.existenceMode === 'possible');
  result.necessarilyExisting = result.findings.filter(f => f.existenceMode === 'necessary');

  result.verdict = result.findings.length > 0
    ? `Found ${result.findings.length} entities matching "${query}"`
    : `No entities found matching "${query}"`;

  state.existenceQueries.push(result);
  state.stats.queriesProcessed++;

  return result;
}

/**
 * Analyze ontological commitments of a theory/statement
 */
function analyzeCommitments(statement) {
  const analysis = {
    statement,
    commitments: [],
    confidence: PHI_INV_2
  };

  // Simple pattern matching for existence claims
  const patterns = [
    { regex: /there (is|are|exists?)\s+(\w+)/gi, type: 'explicit' },
    { regex: /(\w+)\s+(is|are)\s+(\w+)/gi, type: 'implicit' },
    { regex: /the\s+(\w+)/gi, type: 'apparent' }
  ];

  for (const pattern of patterns) {
    const matches = statement.match(pattern.regex);
    if (matches) {
      for (const match of matches) {
        analysis.commitments.push({
          expression: match,
          type: pattern.type,
          typeInfo: COMMITMENT_TYPES[pattern.type]
        });
      }
    }
  }

  // Quine's analysis
  analysis.quineanVerdict = analysis.commitments.length > 0
    ? `Statement appears to commit to ${analysis.commitments.length} entities`
    : 'No clear ontological commitments detected';

  // Check if commitments can be paraphrased away
  analysis.paraphrasable = analysis.commitments.every(c => c.type !== 'explicit');

  return analysis;
}

/**
 * Check category membership
 */
function categorize(entityId) {
  const entity = state.entities.get(entityId);
  if (!entity) return null;

  const category = ONTOLOGICAL_CATEGORIES[entity.category];

  return {
    entityId,
    category: entity.category,
    categoryInfo: category,
    isAbstract: category?.abstract || false,
    examples: category?.examples || [],
    confidence: PHI_INV
  };
}

/**
 * Compare ontologies
 */
function compareOntologies(ont1Id, ont2Id) {
  const o1 = state.ontologies.get(ont1Id);
  const o2 = state.ontologies.get(ont2Id);

  if (!o1 || !o2) return null;

  const comparison = {
    ontologies: [ont1Id, ont2Id],

    // Category comparison
    sharedCategories: o1.admittedCategories.filter(c =>
      o2.admittedCategories.includes(c)
    ),
    uniqueTo1: o1.admittedCategories.filter(c =>
      !o2.admittedCategories.includes(c)
    ),
    uniqueTo2: o2.admittedCategories.filter(c =>
      !o1.admittedCategories.includes(c)
    ),

    // Commitment comparison
    commitmentCount1: o1.commitments.length,
    commitmentCount2: o2.commitments.length,

    // Parsimony
    moreParsimoniuous: o1.commitments.length < o2.commitments.length
      ? ont1Id
      : o2.commitments.length < o1.commitments.length
        ? ont2Id
        : 'equal',

    // Abstract entities
    abstractRealism: {
      [ont1Id]: o1.abstractRealism,
      [ont2Id]: o2.abstractRealism
    },

    timestamp: Date.now()
  };

  // Quine's advice: prefer parsimony
  comparison.quineanRecommendation = comparison.moreParsimoniuous !== 'equal'
    ? `Prefer ${comparison.moreParsimoniuous} (fewer commitments)`
    : 'Ontologies are equally parsimonious';

  return comparison;
}

/**
 * Check if entity exists (in various senses)
 */
function doesExist(entityId, mode = 'actual') {
  const entity = state.entities.get(entityId);

  if (!entity) {
    return {
      entityId,
      exists: false,
      reason: 'Not registered in ontology',
      confidence: PHI_INV_3
    };
  }

  const modeInfo = EXISTENCE_MODES[mode];
  const entityMode = EXISTENCE_MODES[entity.existenceMode];

  // Check if entity's existence mode satisfies the query mode
  const exists = entity.existenceMode === mode ||
    (mode === 'possible' && ['actual', 'necessary'].includes(entity.existenceMode)) ||
    (mode === 'actual' && entity.existenceMode === 'necessary');

  return {
    entityId,
    mode,
    modeInfo,
    exists,
    actualMode: entity.existenceMode,
    reason: exists
      ? `Entity has ${entity.existenceMode} existence`
      : `Entity has ${entity.existenceMode} existence, not ${mode}`,
    confidence: Math.min(entityMode?.strength || PHI_INV_2, PHI_INV)
  };
}

/**
 * Carnapian internal vs external question
 */
function classifyQuestion(question) {
  // Internal: within a framework
  // External: about the framework itself
  const internalPatterns = [
    /is there a/i,
    /does .* exist/i,
    /are there/i,
    /how many/i
  ];

  const externalPatterns = [
    /should we accept/i,
    /is it useful to/i,
    /what framework/i,
    /which ontology/i
  ];

  let type = 'unclear';

  for (const pattern of internalPatterns) {
    if (pattern.test(question)) {
      type = 'internal';
      break;
    }
  }

  for (const pattern of externalPatterns) {
    if (pattern.test(question)) {
      type = 'external';
      break;
    }
  }

  return {
    question,
    type,
    carnapianAnalysis: type === 'internal'
      ? 'Answerable within framework by standard methods'
      : type === 'external'
        ? 'Pragmatic question about framework adoption'
        : 'Question type unclear',
    confidence: PHI_INV_2
  };
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
    ontologies: Array.from(state.ontologies.entries()),
    commitments: state.commitments.slice(-100),
    existenceQueries: state.existenceQueries.slice(-50),
    categories: Array.from(state.categories.entries()),
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
      state.ontologies = new Map(data.ontologies || []);
      state.commitments = data.commitments || [];
      state.existenceQueries = data.existenceQueries || [];
      state.categories = new Map(data.categories || []);
      state.stats = data.stats || state.stats;
    } catch (e) {
      console.error('Failed to load existence engine state:', e.message);
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
    '── EXISTENCE ENGINE ───────────────────────────────────────',
    ''
  ];

  lines.push(`   Entities: ${state.entities.size} | Ontologies: ${state.ontologies.size}`);
  lines.push(`   Commitments: ${state.stats.commitmentsTracked} | Queries: ${state.stats.queriesProcessed}`);
  lines.push('');

  // Category breakdown
  if (state.categories.size > 0) {
    lines.push('   Categories:');
    for (const [cat, entities] of state.categories) {
      if (entities.length > 0) {
        lines.push(`   └─ ${cat}: ${entities.length} entities`);
      }
    }
  }

  lines.push('');
  return lines.join('\n');
}

function getStats() {
  return {
    ...state.stats,
    entityCount: state.entities.size,
    ontologyCount: state.ontologies.size,
    categoriesUsed: state.categories.size
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
    entities: state.entities.size,
    ontologies: state.ontologies.size
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
  ONTOLOGICAL_CATEGORIES,
  EXISTENCE_MODES,
  COMMITMENT_TYPES,

  // Core functions
  registerEntity,
  createOntology,
  commitToExistence,
  queryExistence,
  analyzeCommitments,
  categorize,
  compareOntologies,
  doesExist,
  classifyQuestion,

  // State access
  getEntity: (id) => state.entities.get(id),
  getOntology: (id) => state.ontologies.get(id),
  getEntitiesByCategory: (cat) => state.categories.get(cat) || [],

  // Persistence
  saveState,
  loadState,

  // Formatting
  formatStatus,
  getStats,
  init
};

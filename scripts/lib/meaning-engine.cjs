/**
 * Meaning Engine - Frege/Kripke
 *
 * "The Morning Star is the Evening Star."
 * — Frege's Puzzle: Same reference, different sense
 *
 * "Hesperus is Phosphorus" — necessary a posteriori (Kripke)
 *
 * Implements:
 * - Sense vs reference distinction (Frege)
 * - Rigid designators (Kripke)
 * - Descriptivism vs direct reference
 * - Meaning theories
 *
 * φ guides confidence in meaning assignments.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'meaning');

// Frege's semantic components
const FREGEAN_COMPONENTS = {
  sense: {
    name: 'Sense (Sinn)',
    description: 'The mode of presentation of the referent',
    determines: 'reference',
    cognitiveValue: true,
    examples: ['the morning star', 'the evening star'],
    frege: 'Sense is the way in which the reference is given to us'
  },
  reference: {
    name: 'Reference (Bedeutung)',
    description: 'The object or truth-value denoted',
    determinedBy: 'sense',
    examples: ['Venus (the planet)', 'the True', 'the False'],
    frege: 'The reference is the actual object in the world'
  },
  thought: {
    name: 'Thought (Gedanke)',
    description: 'The sense of a complete sentence',
    truthBearing: true,
    objective: true,
    frege: 'Thoughts are abstract, timeless entities'
  }
};

// Types of expressions
const EXPRESSION_TYPES = {
  proper_name: {
    name: 'Proper Name',
    sense: 'mode of presentation',
    reference: 'object',
    examples: ['Aristotle', 'Venus', 'Paris'],
    kripke: 'Rigid designator in Kripke\'s view'
  },
  definite_description: {
    name: 'Definite Description',
    sense: 'descriptive content',
    reference: 'unique satisfier (if any)',
    examples: ['the author of Hamlet', 'the current King of France'],
    russell: 'Quantified, not genuinely referential (Russell)'
  },
  predicate: {
    name: 'Predicate',
    sense: 'concept (unsaturated)',
    reference: 'function from objects to truth-values',
    examples: ['is wise', 'is red', 'loves'],
    frege: 'Concepts are functions'
  },
  sentence: {
    name: 'Sentence',
    sense: 'thought (proposition)',
    reference: 'truth-value',
    examples: ['Snow is white', 'Socrates is wise'],
    frege: 'Sentences refer to the True or the False'
  }
};

// Kripke's categories
const KRIPKE_CATEGORIES = {
  rigid_designator: {
    name: 'Rigid Designator',
    definition: 'Designates the same object in all possible worlds where it exists',
    examples: ['proper names', 'natural kind terms', 'indexicals'],
    contrast: 'definite descriptions (typically non-rigid)'
  },
  a_priori: {
    name: 'A Priori',
    definition: 'Knowable independently of experience',
    examples: ['2+2=4', 'All bachelors are unmarried'],
    epistemological: true
  },
  a_posteriori: {
    name: 'A Posteriori',
    definition: 'Knowable only through experience',
    examples: ['Water is H2O', 'Hesperus is Phosphorus'],
    epistemological: true
  },
  necessary: {
    name: 'Necessary',
    definition: 'True in all possible worlds',
    examples: ['2+2=4', 'Hesperus is Phosphorus'],
    metaphysical: true
  },
  contingent: {
    name: 'Contingent',
    definition: 'True in some worlds but not others',
    examples: ['Biden is President', 'Snow is white'],
    metaphysical: true
  }
};

// Theories of meaning
const MEANING_THEORIES = {
  referential: {
    name: 'Referential Theory',
    claim: 'Meaning is reference',
    problem: 'Frege\'s puzzle: co-referential terms differ in cognitive value',
    proponent: 'Mill'
  },
  fregean: {
    name: 'Fregean Theory',
    claim: 'Meaning includes both sense and reference',
    strength: 'Explains cognitive value differences',
    problem: 'What are senses? (metaphysical worry)',
    proponent: 'Frege'
  },
  descriptivist: {
    name: 'Descriptivist Theory',
    claim: 'Names are disguised definite descriptions',
    strength: 'Explains apparent reference to non-existents',
    problem: 'Modal and epistemic objections (Kripke)',
    proponent: 'Russell, early Searle'
  },
  causal: {
    name: 'Causal Theory',
    claim: 'Reference is fixed by causal chains to baptism',
    strength: 'Handles Kripke\'s objections',
    problem: 'Qua problem, reference magnetism',
    proponent: 'Kripke, Putnam'
  },
  use: {
    name: 'Use Theory',
    claim: 'Meaning is use in language games',
    strength: 'Anti-metaphysical, practical',
    problem: 'Systematic theory?',
    proponent: 'Later Wittgenstein'
  }
};

// State
const state = {
  expressions: new Map(),      // Registered expressions
  senseAssignments: new Map(), // Sense assignments
  referenceAssignments: new Map(), // Reference assignments
  identityStatements: [],      // Identity puzzles
  rigidDesignators: new Map()  // Tracked rigid designators
};

/**
 * Initialize the meaning engine
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
      if (saved.expressions) state.expressions = new Map(Object.entries(saved.expressions));
      if (saved.senseAssignments) state.senseAssignments = new Map(Object.entries(saved.senseAssignments));
      if (saved.referenceAssignments) state.referenceAssignments = new Map(Object.entries(saved.referenceAssignments));
      if (saved.identityStatements) state.identityStatements = saved.identityStatements;
      if (saved.rigidDesignators) state.rigidDesignators = new Map(Object.entries(saved.rigidDesignators));
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', expressions: state.expressions.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    expressions: Object.fromEntries(state.expressions),
    senseAssignments: Object.fromEntries(state.senseAssignments),
    referenceAssignments: Object.fromEntries(state.referenceAssignments),
    identityStatements: state.identityStatements,
    rigidDesignators: Object.fromEntries(state.rigidDesignators)
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Register an expression
 */
function registerExpression(id, spec = {}) {
  const type = EXPRESSION_TYPES[spec.type] || EXPRESSION_TYPES.proper_name;

  const expression = {
    id,
    surface: spec.surface || id,   // The linguistic form
    type: spec.type || 'proper_name',
    typeInfo: type,

    // Fregean semantics
    sense: null,
    reference: null,

    // Kripke
    isRigid: spec.isRigid ?? (spec.type === 'proper_name'),

    // Metadata
    language: spec.language || 'en',
    registeredAt: Date.now()
  };

  state.expressions.set(id, expression);
  saveState();

  return expression;
}

/**
 * Assign sense to an expression (Frege)
 */
function assignSense(expressionId, sense) {
  const expression = state.expressions.get(expressionId);
  if (!expression) {
    return { error: 'Expression not found' };
  }

  const assignment = {
    expressionId,
    surface: expression.surface,
    sense: sense.content || sense,
    modeOfPresentation: sense.mode || sense.content || sense,
    cognitiveValue: sense.cognitiveValue || true,
    timestamp: Date.now()
  };

  expression.sense = assignment;
  state.senseAssignments.set(expressionId, assignment);
  saveState();

  return {
    expression: expression.surface,
    sense: assignment.sense,
    frege: 'Sense determines cognitive value and (partially) reference'
  };
}

/**
 * Assign reference to an expression
 */
function assignReference(expressionId, reference) {
  const expression = state.expressions.get(expressionId);
  if (!expression) {
    return { error: 'Expression not found' };
  }

  const assignment = {
    expressionId,
    surface: expression.surface,
    reference: reference.object || reference,
    referenceType: reference.type || 'object',
    exists: reference.exists ?? true,
    timestamp: Date.now()
  };

  expression.reference = assignment;
  state.referenceAssignments.set(expressionId, assignment);

  // Track rigid designators
  if (expression.isRigid) {
    state.rigidDesignators.set(expressionId, {
      expression: expression.surface,
      reference: assignment.reference,
      rigid: true,
      kripke: 'Designates same object in all possible worlds'
    });
  }

  saveState();

  return {
    expression: expression.surface,
    reference: assignment.reference,
    isRigid: expression.isRigid
  };
}

/**
 * Analyze Frege's puzzle (identity statements)
 */
function analyzeIdentity(expr1Id, expr2Id) {
  const expr1 = state.expressions.get(expr1Id);
  const expr2 = state.expressions.get(expr2Id);

  if (!expr1 || !expr2) {
    return { error: 'One or both expressions not found' };
  }

  const analysis = {
    expression1: expr1.surface,
    expression2: expr2.surface,

    // Reference comparison
    sameReference: expr1.reference?.reference === expr2.reference?.reference,
    reference: expr1.reference?.reference,

    // Sense comparison
    sameSense: expr1.sense?.sense === expr2.sense?.sense,
    sense1: expr1.sense?.sense,
    sense2: expr2.sense?.sense,

    // Frege's puzzle analysis
    puzzle: null,
    explanation: null,

    timestamp: Date.now()
  };

  if (analysis.sameReference && !analysis.sameSense) {
    // Classic Frege puzzle case
    analysis.puzzle = 'fregean';
    analysis.explanation = {
      problem: `"${expr1.surface} = ${expr2.surface}" is informative, not trivial`,
      solution: 'The expressions have different senses (modes of presentation)',
      example: '"Hesperus = Phosphorus" is informative because senses differ',
      frege: 'Cognitive value comes from sense, not reference alone'
    };

    // Kripke's addition
    if (expr1.isRigid && expr2.isRigid) {
      analysis.kripke = {
        modal_status: 'necessary',
        epistemic_status: 'a_posteriori',
        insight: 'If true, necessarily true (both rigid) but knowable only empirically'
      };
    }
  } else if (analysis.sameReference && analysis.sameSense) {
    analysis.puzzle = 'none';
    analysis.explanation = {
      note: 'Same sense and reference: trivial identity'
    };
  } else if (!analysis.sameReference) {
    analysis.puzzle = 'false_identity';
    analysis.explanation = {
      note: 'Different references: identity statement is false'
    };
  }

  state.identityStatements.push(analysis);
  saveState();

  return analysis;
}

/**
 * Classify expression as rigid/non-rigid (Kripke)
 */
function classifyRigidity(expressionId) {
  const expression = state.expressions.get(expressionId);
  if (!expression) {
    return { error: 'Expression not found' };
  }

  const classification = {
    expression: expression.surface,
    type: expression.type,

    // Rigidity determination
    isRigid: false,
    reason: null,

    // Kripke's tests
    tests: {
      counterfactual: null,  // Does it refer to same object in counterfactuals?
      modal: null            // Same object in all possible worlds?
    },

    timestamp: Date.now()
  };

  // Determine rigidity based on type
  switch (expression.type) {
    case 'proper_name':
      classification.isRigid = true;
      classification.reason = 'Proper names are rigid designators (Kripke)';
      classification.tests.counterfactual = 'Refers to same individual in counterfactuals';
      classification.tests.modal = 'Same reference in all possible worlds where referent exists';
      break;

    case 'definite_description':
      classification.isRigid = false;
      classification.reason = 'Descriptions typically non-rigid (pick out different objects in different worlds)';
      classification.tests.counterfactual = 'May refer to different objects in counterfactuals';
      classification.tests.modal = '"The tallest person" picks out different people in different worlds';

      // Exception: rigidified descriptions
      if (expression.surface.includes('actual')) {
        classification.isRigid = true;
        classification.reason = 'Rigidified by "actual" operator';
      }
      break;

    default:
      classification.isRigid = expression.isRigid;
      classification.reason = 'Determined by specification';
  }

  // Update expression
  expression.isRigid = classification.isRigid;
  saveState();

  return classification;
}

/**
 * Evaluate meaning theory for an expression
 */
function evaluateMeaningTheory(expressionId, theoryId) {
  const expression = state.expressions.get(expressionId);
  const theory = MEANING_THEORIES[theoryId];

  if (!expression) {
    return { error: 'Expression not found' };
  }
  if (!theory) {
    return { error: 'Theory not found', available: Object.keys(MEANING_THEORIES) };
  }

  const evaluation = {
    expression: expression.surface,
    theory: theory.name,
    claim: theory.claim,

    // How well does theory handle this expression?
    analysis: null,
    verdict: null,
    confidence: PHI_INV_2,

    timestamp: Date.now()
  };

  switch (theoryId) {
    case 'referential':
      evaluation.analysis = `Meaning of "${expression.surface}" = its reference`;
      if (expression.reference) {
        evaluation.verdict = `Meaning is: ${expression.reference.reference}`;
      } else {
        evaluation.verdict = 'No reference assigned; problematic for referential theory';
      }
      evaluation.problem = 'Cannot explain cognitive value differences';
      break;

    case 'fregean':
      evaluation.analysis = `Meaning includes sense: "${expression.sense?.sense || 'unassigned'}"`;
      evaluation.verdict = expression.sense
        ? `Sense: ${expression.sense.sense}, Reference: ${expression.reference?.reference || 'none'}`
        : 'Sense not assigned';
      evaluation.strength = 'Explains informativeness of identity statements';
      break;

    case 'descriptivist':
      if (expression.type === 'proper_name') {
        evaluation.analysis = `"${expression.surface}" abbreviates a description`;
        evaluation.problem = 'Kripke\'s modal/epistemic objections apply';
      } else {
        evaluation.analysis = 'Theory designed for descriptions';
        evaluation.verdict = 'Natural fit';
      }
      break;

    case 'causal':
      if (expression.type === 'proper_name') {
        evaluation.analysis = `"${expression.surface}" refers via causal chain to baptism`;
        evaluation.verdict = 'Reference fixed historically, not descriptively';
        evaluation.strength = 'Handles rigid designation';
      } else {
        evaluation.analysis = 'Less natural for descriptions';
      }
      break;

    case 'use':
      evaluation.analysis = `Meaning of "${expression.surface}" is its use in language games`;
      evaluation.verdict = 'Look at how the expression is used in practice';
      evaluation.wittgenstein = 'Don\'t ask for meaning, ask for use';
      break;
  }

  return evaluation;
}

/**
 * Analyze Kripke's necessary a posteriori
 */
function analyzeNecessaryAPosteriori(statement) {
  const analysis = {
    statement,

    // Classification
    epistemic: null,      // a priori or a posteriori
    metaphysical: null,   // necessary or contingent

    // Kripke's insight
    kripkeCategory: null,
    explanation: null,

    // Examples
    paradigmCase: null,

    timestamp: Date.now()
  };

  // Detect pattern
  const lowerStatement = statement.toLowerCase();

  // Identity statements with rigid designators
  if (lowerStatement.includes(' is ') &&
      (lowerStatement.includes('hesperus') || lowerStatement.includes('phosphorus') ||
       lowerStatement.includes('water') || lowerStatement.includes('h2o'))) {
    analysis.epistemic = 'a_posteriori';
    analysis.metaphysical = 'necessary';
    analysis.kripkeCategory = 'necessary_a_posteriori';
    analysis.explanation = {
      why_necessary: 'Identity between rigid designators: if true, true in all possible worlds',
      why_a_posteriori: 'Required empirical discovery to learn the identity',
      kripke: 'Breaks the traditional a_priori = necessary assumption'
    };
    analysis.paradigmCase = 'Hesperus is Phosphorus';
  }
  // Mathematical truths
  else if (lowerStatement.includes('+') || lowerStatement.includes('=') ||
           lowerStatement.includes('prime') || lowerStatement.includes('number')) {
    analysis.epistemic = 'a_priori';
    analysis.metaphysical = 'necessary';
    analysis.kripkeCategory = 'necessary_a_priori';
    analysis.explanation = {
      note: 'Traditional category: knowable a priori and necessarily true'
    };
    analysis.paradigmCase = '2 + 2 = 4';
  }
  // Contingent a priori (Kripke's other surprising category)
  else if (lowerStatement.includes('meter') || lowerStatement.includes('standard')) {
    analysis.epistemic = 'a_priori';
    analysis.metaphysical = 'contingent';
    analysis.kripkeCategory = 'contingent_a_priori';
    analysis.explanation = {
      why_contingent: 'The standard meter stick could have been different length',
      why_a_priori: 'We know by stipulation, not experience',
      kripke: 'Another break from traditional categories'
    };
    analysis.paradigmCase = 'The standard meter stick is one meter long';
  }
  // Default: contingent a posteriori
  else {
    analysis.epistemic = 'a_posteriori';
    analysis.metaphysical = 'contingent';
    analysis.kripkeCategory = 'contingent_a_posteriori';
    analysis.explanation = {
      note: 'Traditional category: empirical and contingent'
    };
    analysis.paradigmCase = 'Snow is white';
  }

  return analysis;
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '📖 MEANING ENGINE - "Sense determines reference"',
    '═══════════════════════════════════════════════════════════',
    '',
    '── FREGE\'S COMPONENTS ─────────────────────────────────────'
  ];

  for (const [key, component] of Object.entries(FREGEAN_COMPONENTS)) {
    lines.push(`   ${component.name}: ${component.description}`);
  }

  lines.push('');
  lines.push('── KRIPKE\'S CATEGORIES ────────────────────────────────────');
  lines.push('   Necessary A Posteriori: "Hesperus = Phosphorus"');
  lines.push('   Contingent A Priori: "Standard meter = 1 meter"');
  lines.push(`   Rigid designators tracked: ${state.rigidDesignators.size}`);

  lines.push('');
  lines.push('── MEANING THEORIES ───────────────────────────────────────');
  for (const [key, theory] of Object.entries(MEANING_THEORIES)) {
    lines.push(`   ${theory.name} (${theory.proponent})`);
  }

  lines.push('');
  lines.push('── STATISTICS ─────────────────────────────────────────────');
  lines.push(`   Expressions: ${state.expressions.size}`);
  lines.push(`   Sense assignments: ${state.senseAssignments.size}`);
  lines.push(`   Reference assignments: ${state.referenceAssignments.size}`);
  lines.push(`   Identity puzzles: ${state.identityStatements.length}`);

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('*sniff* "The morning star is the evening star."');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  const typeCount = {};
  const rigidCount = { rigid: 0, nonRigid: 0 };

  for (const expr of state.expressions.values()) {
    typeCount[expr.type] = (typeCount[expr.type] || 0) + 1;
    if (expr.isRigid) rigidCount.rigid++;
    else rigidCount.nonRigid++;
  }

  return {
    expressions: state.expressions.size,
    expressionsByType: typeCount,
    rigidDesignators: rigidCount.rigid,
    nonRigidExpressions: rigidCount.nonRigid,
    senseAssignments: state.senseAssignments.size,
    referenceAssignments: state.referenceAssignments.size,
    identityPuzzles: state.identityStatements.length
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Expressions
  registerExpression,
  assignSense,
  assignReference,
  EXPRESSION_TYPES,

  // Frege
  analyzeIdentity,
  FREGEAN_COMPONENTS,

  // Kripke
  classifyRigidity,
  analyzeNecessaryAPosteriori,
  KRIPKE_CATEGORIES,

  // Theories
  evaluateMeaningTheory,
  MEANING_THEORIES,

  // Constants
  PHI,
  PHI_INV
};

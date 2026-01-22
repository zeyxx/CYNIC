/**
 * Rights Engine - Natural/Positive/Negative
 *
 * "We hold these truths to be self-evident, that all men are endowed
 * with certain unalienable Rights."
 * — Declaration of Independence
 *
 * Implements:
 * - Natural vs legal/positive rights
 * - Positive rights (entitlements) vs negative rights (freedoms)
 * - Hohfeldian analysis (claims, privileges, powers, immunities)
 * - Rights conflicts and resolution
 *
 * φ guides confidence in rights assessments.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'rights');

// Natural vs Legal Rights
const RIGHTS_ONTOLOGY = {
  natural: {
    name: 'Natural Rights',
    description: 'Rights that exist prior to and independent of law',
    source: 'Nature, God, or Reason',
    characteristics: [
      'Universal (apply to all humans)',
      'Inalienable (cannot be given away)',
      'Pre-political (exist before government)',
      'Self-evident or discoverable by reason'
    ],
    examples: ['Life', 'Liberty', 'Property (Locke)', 'Pursuit of happiness'],
    proponents: ['Locke', 'Jefferson', 'Natural law tradition'],
    critiques: [
      'Bentham: "Nonsense upon stilts"',
      'No consensus on content',
      'Cultural variation suggests conventionality'
    ]
  },
  legal: {
    name: 'Legal/Positive Rights',
    description: 'Rights that exist because of law',
    source: 'Legal system, constitution, legislation',
    characteristics: [
      'Created by law',
      'Can be modified or revoked',
      'Vary by jurisdiction',
      'Enforceable through legal system'
    ],
    examples: ['Right to vote', 'Right to counsel', 'Right to education'],
    proponents: ['Legal positivists', 'Bentham', 'Austin'],
    advantage: 'Clear, enforceable, determinable'
  },
  moral: {
    name: 'Moral Rights',
    description: 'Rights grounded in morality',
    source: 'Moral principles (may or may not be encoded in law)',
    characteristics: [
      'May not be legally recognized',
      'Provide grounds for legal reform',
      'Based on moral reasoning'
    ],
    examples: ['Right not to be tortured', 'Right to subsistence welfare'],
    relation: 'May become legal rights through advocacy'
  }
};

// Positive vs Negative Rights
const POSITIVE_NEGATIVE = {
  negative: {
    name: 'Negative Rights',
    description: 'Rights to non-interference',
    correlativeDuty: 'Duty of others to refrain from action',
    examples: [
      'Right to life (don\'t kill me)',
      'Right to property (don\'t steal from me)',
      'Right to liberty (don\'t imprison me)',
      'Freedom of speech (don\'t silence me)'
    ],
    cost: 'Low - only requires inaction',
    political: 'Favored by libertarians, classical liberals',
    quote: 'Leave me alone'
  },
  positive: {
    name: 'Positive Rights',
    description: 'Rights to be provided with something',
    correlativeDuty: 'Duty of others (usually state) to act',
    examples: [
      'Right to education (provide schools)',
      'Right to healthcare (provide treatment)',
      'Right to housing (provide shelter)',
      'Right to welfare (provide subsistence)'
    ],
    cost: 'High - requires resources and action',
    political: 'Favored by social democrats, welfare liberals',
    quote: 'Provide for me'
  },
  debate: {
    libertarianView: 'Only negative rights are genuine rights',
    socialistView: 'Positive rights essential for real freedom',
    rawlsianView: 'Both needed for fair equality of opportunity',
    question: 'Can negative rights be effective without positive provisions?'
  }
};

// Hohfeldian Analysis
const HOHFELD = {
  description: 'Wesley Hohfeld\'s analysis of legal relations (1919)',
  fundamentalRelations: {
    claim: {
      name: 'Claim-Right',
      definition: 'A has a claim against B if B has a duty to A',
      correlative: 'Duty',
      opposite: 'No-right',
      example: 'A\'s claim to payment → B\'s duty to pay'
    },
    privilege: {
      name: 'Privilege (Liberty)',
      definition: 'A has a privilege if A has no duty not to act',
      correlative: 'No-right',
      opposite: 'Duty',
      example: 'A\'s privilege to walk on common → Others have no-right to prevent'
    },
    power: {
      name: 'Power',
      definition: 'A has power to change legal relations',
      correlative: 'Liability',
      opposite: 'Disability',
      example: 'A\'s power to make a will → B\'s liability to be disinherited'
    },
    immunity: {
      name: 'Immunity',
      definition: 'A is immune if B lacks power to change A\'s relations',
      correlative: 'Disability',
      opposite: 'Liability',
      example: 'Constitutional rights create immunities against government'
    }
  },
  relations: {
    correlatives: [
      ['Claim', 'Duty'],
      ['Privilege', 'No-right'],
      ['Power', 'Liability'],
      ['Immunity', 'Disability']
    ],
    opposites: [
      ['Claim', 'No-right'],
      ['Privilege', 'Duty'],
      ['Power', 'Disability'],
      ['Immunity', 'Liability']
    ]
  },
  importance: 'Disambiguates loose talk of "rights"'
};

// Rights Conflict Theories
const CONFLICT_RESOLUTION = {
  absolutism: {
    name: 'Rights Absolutism',
    view: 'Some rights cannot be overridden',
    example: 'Right not to be tortured (Nozick\'s side constraints)',
    problem: 'What about conflict between absolute rights?'
  },
  balancing: {
    name: 'Rights Balancing',
    view: 'Rights must be weighed against each other',
    method: 'Proportionality analysis',
    factors: ['Importance of right', 'Degree of infringement', 'Alternatives'],
    problem: 'How to compare incommensurable rights?'
  },
  specificationism: {
    name: 'Specificationism',
    view: 'Rights don\'t really conflict - apparent conflicts resolved by specifying content',
    method: 'Clarify precise scope of each right',
    example: 'Free speech doesn\'t include incitement - no conflict with safety',
    problem: 'Specification can be ad hoc'
  },
  hierarchy: {
    name: 'Hierarchical',
    view: 'Some rights take priority over others',
    example: 'Rawls: liberty over economic equality',
    problem: 'How to establish the hierarchy?'
  }
};

// State
const state = {
  rights: new Map(),
  conflicts: [],
  analyses: [],
  resolutions: []
};

/**
 * Initialize the rights engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.rights) state.rights = new Map(Object.entries(saved.rights));
      if (saved.conflicts) state.conflicts = saved.conflicts;
      if (saved.analyses) state.analyses = saved.analyses;
      if (saved.resolutions) state.resolutions = saved.resolutions;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', rights: state.rights.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    rights: Object.fromEntries(state.rights),
    conflicts: state.conflicts,
    analyses: state.analyses,
    resolutions: state.resolutions
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Define a right
 */
function defineRight(id, spec = {}) {
  const right = {
    id,
    name: spec.name || id,
    description: spec.description || '',

    // Classification
    ontology: spec.ontology || 'unknown',  // natural, legal, moral
    polarity: spec.polarity || 'unknown',  // positive, negative

    // Hohfeldian analysis
    hohfeldian: {
      type: spec.hohfeldType || 'claim',   // claim, privilege, power, immunity
      holder: spec.holder || null,
      correlative: {
        type: null,
        bearer: spec.dutyBearer || null
      }
    },

    // Content
    content: spec.content || null,
    scope: spec.scope || 'universal',
    exceptions: spec.exceptions || [],

    // Strength
    weight: spec.weight || 0.5,
    derogable: spec.derogable !== false,

    createdAt: Date.now()
  };

  // Set correlative based on Hohfeld
  const correlatives = {
    claim: 'duty',
    privilege: 'no-right',
    power: 'liability',
    immunity: 'disability'
  };
  right.hohfeldian.correlative.type = correlatives[right.hohfeldian.type] || null;

  state.rights.set(id, right);
  saveState();

  return right;
}

/**
 * Perform Hohfeldian analysis of a right
 */
function hohfeldianAnalysis(rightId) {
  const right = state.rights.get(rightId);
  if (!right) {
    return { error: 'Right not found' };
  }

  const analysis = {
    rightId,
    name: right.name,

    // Primary relation
    primaryRelation: {
      type: right.hohfeldian.type,
      holder: right.hohfeldian.holder,
      explanation: null
    },

    // Correlative relation
    correlativeRelation: {
      type: right.hohfeldian.correlative.type,
      bearer: right.hohfeldian.correlative.bearer,
      explanation: null
    },

    // Opposite relation
    oppositeRelation: {
      type: null,
      explanation: null
    },

    // Full picture
    hohfeldianPicture: null,

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Set explanations based on type
  switch (right.hohfeldian.type) {
    case 'claim':
      analysis.primaryRelation.explanation =
        `${right.hohfeldian.holder} has a claim against ${right.hohfeldian.correlative.bearer}`;
      analysis.correlativeRelation.explanation =
        `${right.hohfeldian.correlative.bearer} has a duty to ${right.hohfeldian.holder}`;
      analysis.oppositeRelation.type = 'no-right';
      analysis.oppositeRelation.explanation =
        `Absence of claim would mean ${right.hohfeldian.holder} has no-right`;
      break;

    case 'privilege':
      analysis.primaryRelation.explanation =
        `${right.hohfeldian.holder} has no duty not to do X`;
      analysis.correlativeRelation.explanation =
        `Others have no-right to prevent ${right.hohfeldian.holder}`;
      analysis.oppositeRelation.type = 'duty';
      analysis.oppositeRelation.explanation =
        `Opposite would be a duty not to do X`;
      break;

    case 'power':
      analysis.primaryRelation.explanation =
        `${right.hohfeldian.holder} can change legal relations`;
      analysis.correlativeRelation.explanation =
        `${right.hohfeldian.correlative.bearer} is liable to have relations changed`;
      analysis.oppositeRelation.type = 'disability';
      analysis.oppositeRelation.explanation =
        `Opposite would be disability to change relations`;
      break;

    case 'immunity':
      analysis.primaryRelation.explanation =
        `${right.hohfeldian.holder}'s relations cannot be changed by ${right.hohfeldian.correlative.bearer}`;
      analysis.correlativeRelation.explanation =
        `${right.hohfeldian.correlative.bearer} has disability to change ${right.hohfeldian.holder}'s relations`;
      analysis.oppositeRelation.type = 'liability';
      analysis.oppositeRelation.explanation =
        `Opposite would be liability to have relations changed`;
      break;
  }

  analysis.hohfeldianPicture = {
    relation: `${right.hohfeldian.type} ↔ ${right.hohfeldian.correlative.type}`,
    parties: `${right.hohfeldian.holder} ↔ ${right.hohfeldian.correlative.bearer}`,
    summary: `${analysis.primaryRelation.explanation}; therefore ${analysis.correlativeRelation.explanation}`
  };

  state.analyses.push(analysis);
  saveState();

  return analysis;
}

/**
 * Classify a right (natural/legal, positive/negative)
 */
function classifyRight(rightId) {
  const right = state.rights.get(rightId);
  if (!right) {
    return { error: 'Right not found' };
  }

  const classification = {
    rightId,
    name: right.name,

    // Ontological classification
    ontological: {
      type: right.ontology,
      characteristics: null,
      implications: null
    },

    // Polarity classification
    polarity: {
      type: right.polarity,
      correlativeDuty: null,
      cost: null
    },

    // Combined assessment
    combined: null,

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Set ontological characteristics
  if (right.ontology === 'natural') {
    classification.ontological.characteristics = RIGHTS_ONTOLOGY.natural.characteristics;
    classification.ontological.implications = 'Pre-political, inalienable, universal';
  } else if (right.ontology === 'legal') {
    classification.ontological.characteristics = RIGHTS_ONTOLOGY.legal.characteristics;
    classification.ontological.implications = 'Created by law, jurisdiction-specific, modifiable';
  } else if (right.ontology === 'moral') {
    classification.ontological.characteristics = RIGHTS_ONTOLOGY.moral.characteristics;
    classification.ontological.implications = 'May ground claims for legal recognition';
  }

  // Set polarity characteristics
  if (right.polarity === 'negative') {
    classification.polarity.correlativeDuty = 'Duty to refrain from interference';
    classification.polarity.cost = 'Low (requires only inaction)';
  } else if (right.polarity === 'positive') {
    classification.polarity.correlativeDuty = 'Duty to provide/act';
    classification.polarity.cost = 'High (requires resources and action)';
  }

  // Combined assessment
  classification.combined = `${right.ontology} ${right.polarity} right`;

  return classification;
}

/**
 * Analyze rights conflict
 */
function analyzeConflict(right1Id, right2Id, scenario = {}) {
  const r1 = state.rights.get(right1Id);
  const r2 = state.rights.get(right2Id);

  if (!r1 || !r2) {
    return { error: 'Right not found' };
  }

  const conflict = {
    rights: [right1Id, right2Id],
    scenario: scenario.description || 'Rights conflict scenario',

    // Nature of conflict
    nature: {
      genuine: null,
      type: null,
      description: null
    },

    // Analysis under different frameworks
    absolutist: {
      approach: CONFLICT_RESOLUTION.absolutism.view,
      result: null
    },

    balancing: {
      approach: CONFLICT_RESOLUTION.balancing.view,
      weights: {
        [right1Id]: r1.weight,
        [right2Id]: r2.weight
      },
      result: null
    },

    specificationist: {
      approach: CONFLICT_RESOLUTION.specificationism.view,
      result: null
    },

    hierarchical: {
      approach: CONFLICT_RESOLUTION.hierarchy.view,
      result: null
    },

    // Recommendation
    recommendation: null,

    confidence: PHI_INV_3,
    timestamp: Date.now()
  };

  // Determine if genuine conflict
  if (r1.exceptions.includes(right2Id) || r2.exceptions.includes(right1Id)) {
    conflict.nature.genuine = false;
    conflict.nature.type = 'apparent';
    conflict.nature.description = 'One right already has exception for the other';
  } else {
    conflict.nature.genuine = true;
    conflict.nature.type = 'genuine';
    conflict.nature.description = 'Rights genuinely conflict in this scenario';
  }

  // Absolutist analysis
  if (!r1.derogable && !r2.derogable) {
    conflict.absolutist.result = 'DILEMMA - both rights absolute';
  } else if (!r1.derogable) {
    conflict.absolutist.result = `${right1Id} prevails (absolute)`;
  } else if (!r2.derogable) {
    conflict.absolutist.result = `${right2Id} prevails (absolute)`;
  } else {
    conflict.absolutist.result = 'Neither absolute - balancing needed';
  }

  // Balancing analysis
  if (r1.weight > r2.weight) {
    conflict.balancing.result = `${right1Id} prevails (weight: ${r1.weight} > ${r2.weight})`;
  } else if (r2.weight > r1.weight) {
    conflict.balancing.result = `${right2Id} prevails (weight: ${r2.weight} > ${r1.weight})`;
  } else {
    conflict.balancing.result = 'Equal weight - consider context';
  }

  // Specificationist analysis
  conflict.specificationist.result =
    'Specify precise scope of each right to dissolve apparent conflict';

  // Hierarchical analysis
  if (r1.ontology === 'natural' && r2.ontology === 'legal') {
    conflict.hierarchical.result = `${right1Id} prevails (natural > legal)`;
  } else if (r2.ontology === 'natural' && r1.ontology === 'legal') {
    conflict.hierarchical.result = `${right2Id} prevails (natural > legal)`;
  } else {
    conflict.hierarchical.result = 'Same level in hierarchy - no clear priority';
  }

  // Recommendation
  if (!conflict.nature.genuine) {
    conflict.recommendation = 'No genuine conflict - rights already accommodate each other';
  } else if (!r1.derogable || !r2.derogable) {
    conflict.recommendation = conflict.absolutist.result;
  } else {
    conflict.recommendation = `Consider balancing: ${conflict.balancing.result}`;
  }

  state.conflicts.push(conflict);
  saveState();

  return conflict;
}

/**
 * Resolve rights conflict
 */
function resolveConflict(conflictIndex, resolution) {
  if (conflictIndex >= state.conflicts.length) {
    return { error: 'Conflict not found' };
  }

  const conflict = state.conflicts[conflictIndex];

  const result = {
    conflictIndex,
    rights: conflict.rights,

    resolution: {
      method: resolution.method || 'balancing',
      outcome: resolution.outcome || null,
      reasoning: resolution.reasoning || null,
      rightPrevails: resolution.rightPrevails || null
    },

    // Record any specification or exception added
    modifications: [],

    confidence: resolution.confidence || PHI_INV_3,
    timestamp: Date.now()
  };

  // If specification method, record modifications
  if (resolution.method === 'specification' && resolution.specifications) {
    for (const spec of resolution.specifications) {
      const right = state.rights.get(spec.rightId);
      if (right && spec.exception) {
        right.exceptions.push(spec.exception);
        result.modifications.push({
          rightId: spec.rightId,
          added: `exception: ${spec.exception}`
        });
      }
    }
    saveState();
  }

  state.resolutions.push(result);
  saveState();

  return result;
}

/**
 * Compare positive and negative rights
 */
function comparePositiveNegative() {
  return {
    comparison: 'Positive vs Negative Rights',

    negative: {
      ...POSITIVE_NEGATIVE.negative,
      examples: POSITIVE_NEGATIVE.negative.examples.slice(0, 3),
      philosophical: 'Libertarians argue only these are genuine rights'
    },

    positive: {
      ...POSITIVE_NEGATIVE.positive,
      examples: POSITIVE_NEGATIVE.positive.examples.slice(0, 3),
      philosophical: 'Social democrats argue these are necessary for real freedom'
    },

    keyQuestions: [
      'Can negative rights be effective without positive provisions?',
      'Does the cost difference matter morally?',
      'Are positive rights too demanding?',
      'Is the distinction even coherent? (All rights require enforcement)'
    ],

    synthesis: {
      capability: 'Capability approach (Sen/Nussbaum) bridges both',
      rawlsian: 'Fundamental liberties (negative) + fair equality of opportunity (positive)',
      pragmatic: 'Most legal systems recognize both types'
    },

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };
}

/**
 * Evaluate a rights claim
 */
function evaluateClaim(spec) {
  const evaluation = {
    claim: spec.description || 'Rights claim',
    claimant: spec.claimant || 'unspecified',

    // Is this a right?
    isRight: {
      hasHolder: Boolean(spec.claimant),
      hasContent: Boolean(spec.content),
      hasCorrelative: Boolean(spec.dutyBearer),
      conclusion: null
    },

    // Classification
    classification: {
      ontology: spec.ontology || 'claimed as natural',
      polarity: spec.polarity || 'unclear',
      hohfeldType: spec.hohfeldType || 'claim'
    },

    // Validity assessment
    validity: {
      naturalRightsView: null,
      legalPositivistView: null,
      hohfeldianView: null
    },

    // Overall
    assessment: null,

    confidence: PHI_INV_3,
    timestamp: Date.now()
  };

  // Is this a right?
  evaluation.isRight.conclusion =
    evaluation.isRight.hasHolder &&
    evaluation.isRight.hasContent &&
    evaluation.isRight.hasCorrelative
      ? 'Meets formal criteria for a right'
      : 'Missing elements - may not be a right';

  // Natural rights view
  if (spec.universal && spec.inalienable) {
    evaluation.validity.naturalRightsView = 'Plausibly a natural right';
  } else {
    evaluation.validity.naturalRightsView = 'Does not meet natural rights criteria';
  }

  // Legal positivist view
  if (spec.legallyRecognized) {
    evaluation.validity.legalPositivistView = 'Valid legal right';
  } else {
    evaluation.validity.legalPositivistView = 'Not a legal right (may be moral claim)';
  }

  // Hohfeldian view
  if (evaluation.isRight.hasCorrelative) {
    evaluation.validity.hohfeldianView =
      `Valid ${spec.hohfeldType || 'claim'}-right with correlative ${
        spec.hohfeldType === 'claim' ? 'duty' : 'relation'
      }`;
  } else {
    evaluation.validity.hohfeldianView = 'Lacks clear correlative - not a right in strict sense';
  }

  // Overall assessment
  const validViews = [
    evaluation.validity.naturalRightsView.includes('Plausibly'),
    evaluation.validity.legalPositivistView.includes('Valid'),
    evaluation.isRight.conclusion.includes('Meets')
  ].filter(Boolean).length;

  if (validViews >= 2) {
    evaluation.assessment = 'VALID RIGHT (multiple frameworks)';
  } else if (validViews === 1) {
    evaluation.assessment = 'CONTESTED - valid under some frameworks';
  } else {
    evaluation.assessment = 'NOT A RIGHT in standard sense';
  }

  return evaluation;
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '🏛️  RIGHTS ENGINE - Natural, Positive, Negative',
    '═══════════════════════════════════════════════════════════',
    '',
    '── ONTOLOGY ───────────────────────────────────────────────',
    '   Natural: Pre-political, inalienable, universal',
    '   Legal: Created by law, enforceable, modifiable',
    '   Moral: Grounded in ethics, may lack legal force',
    '',
    '── POLARITY ───────────────────────────────────────────────',
    '   Negative: Non-interference (don\'t harm me)',
    '   Positive: Provision (provide for me)',
    '',
    '── HOHFELD ────────────────────────────────────────────────',
    '   Claim ↔ Duty    |    Privilege ↔ No-right',
    '   Power ↔ Liability |  Immunity ↔ Disability',
    '',
    '── CONFLICT RESOLUTION ────────────────────────────────────',
    '   Absolutism | Balancing | Specification | Hierarchy',
    '',
    '── STATISTICS ─────────────────────────────────────────────',
    `   Rights defined: ${state.rights.size}`,
    `   Conflicts analyzed: ${state.conflicts.length}`,
    `   Resolutions: ${state.resolutions.length}`,
    `   Analyses: ${state.analyses.length}`,
    '',
    '═══════════════════════════════════════════════════════════',
    '*sniff* Rights without correlative duties are mere rhetoric.',
    '═══════════════════════════════════════════════════════════'
  ];

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  return {
    rights: state.rights.size,
    conflicts: state.conflicts.length,
    resolutions: state.resolutions.length,
    analyses: state.analyses.length
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Rights
  defineRight,
  classifyRight,
  evaluateClaim,

  // Hohfeld
  hohfeldianAnalysis,
  HOHFELD,

  // Conflicts
  analyzeConflict,
  resolveConflict,

  // Comparisons
  comparePositiveNegative,

  // Theory
  RIGHTS_ONTOLOGY,
  POSITIVE_NEGATIVE,
  CONFLICT_RESOLUTION,

  // Constants
  PHI,
  PHI_INV
};

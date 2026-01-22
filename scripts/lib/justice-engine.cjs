/**
 * Justice Engine - Rawls/Nozick
 *
 * "Justice is the first virtue of social institutions."
 * — John Rawls
 *
 * "From each as they choose, to each as they are chosen."
 * — Robert Nozick
 *
 * Implements:
 * - Rawlsian justice (original position, veil of ignorance, difference principle)
 * - Nozickian justice (entitlement theory, minimal state)
 * - Distributive justice frameworks (equality, desert, need)
 *
 * φ guides confidence in justice assessments.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'justice');

// Rawls's Theory of Justice
const RAWLS = {
  originalPosition: {
    name: 'Original Position',
    description: 'Hypothetical situation for choosing principles of justice',
    features: [
      'Rational agents choosing principles',
      'Behind the veil of ignorance',
      'Free and equal persons',
      'Choosing for a well-ordered society'
    ],
    quote: 'The principles of justice are chosen behind a veil of ignorance.'
  },
  veilOfIgnorance: {
    name: 'Veil of Ignorance',
    description: 'Agents do not know their place in society',
    unknown: [
      'Social class and status',
      'Natural talents and abilities',
      'Conception of the good',
      'Risk tolerance (special psychology)',
      'Generation they belong to'
    ],
    known: [
      'General facts about society',
      'Laws of economics and psychology',
      'Basis of social organization'
    ],
    purpose: 'Ensures impartiality in choosing principles'
  },
  principles: {
    first: {
      name: 'Equal Liberty Principle',
      statement: 'Each person has an equal right to the most extensive fundamental liberties compatible with similar liberties for all',
      priority: 1,
      liberties: ['Political liberty', 'Freedom of speech', 'Liberty of conscience', 'Freedom from oppression']
    },
    second: {
      name: 'Difference Principle (with Fair Equality of Opportunity)',
      statement: 'Social and economic inequalities are to be arranged so that they are to the greatest benefit of the least advantaged members of society',
      priority: 2,
      subPrinciples: {
        fairEquality: 'Positions open to all under fair equality of opportunity',
        difference: 'Inequalities must benefit the least advantaged'
      }
    }
  },
  lexicalPriority: {
    description: 'First principle takes absolute priority over second',
    rule: 'Liberty can only be restricted for the sake of liberty',
    implication: 'Cannot trade liberty for economic gain'
  },
  maximin: {
    name: 'Maximin Rule',
    description: 'Choose the option whose worst outcome is best',
    rationale: 'Under veil of ignorance, rational to secure best minimum',
    formula: 'max(min outcomes)'
  }
};

// Nozick's Entitlement Theory
const NOZICK = {
  entitlementTheory: {
    name: 'Entitlement Theory',
    description: 'Justice in holdings based on how they were acquired',
    principles: {
      acquisition: {
        name: 'Justice in Acquisition',
        description: 'How unheld things may be acquired',
        lockeanProviso: 'Enough and as good left for others'
      },
      transfer: {
        name: 'Justice in Transfer',
        description: 'How holdings may be transferred',
        requirement: 'Voluntary exchange without fraud or force'
      },
      rectification: {
        name: 'Rectification of Injustice',
        description: 'How to correct past violations',
        scope: 'Historical injustices in acquisition or transfer'
      }
    },
    historicalVsPatterned: {
      historical: 'Justice depends on how distribution came about',
      patterned: 'Justice according to some pattern (rejected)',
      nozickCritique: 'Liberty upsets patterns'
    }
  },
  minimalState: {
    name: 'Minimal State',
    description: 'Only state justified is limited to protection',
    functions: ['Protection against force', 'Theft', 'Fraud', 'Enforcement of contracts'],
    excludes: ['Redistribution', 'Paternalism', 'Moral legislation'],
    quote: 'The minimal state is the most extensive state that can be justified'
  },
  wiltChamberlainArgument: {
    name: 'Wilt Chamberlain Argument',
    setup: 'Start with any just distribution D1',
    process: 'People voluntarily pay to see Wilt play',
    result: 'New distribution D2 where Wilt is much richer',
    conclusion: 'If D1 was just and transfers were just, D2 must be just',
    implication: 'Patterned principles require constant interference',
    quote: 'Liberty upsets patterns'
  },
  selfOwnership: {
    name: 'Self-Ownership',
    thesis: 'Each person owns themselves and their talents',
    implication: 'Taxation of earnings is akin to forced labor',
    foundation: 'Kantian respect for persons as ends'
  }
};

// Distributive Justice Principles
const DISTRIBUTIVE_PRINCIPLES = {
  equality: {
    name: 'Strict Equality',
    formula: 'Everyone gets equal share',
    proponents: ['Radical egalitarians'],
    challenge: 'Ignores effort, merit, need'
  },
  desert: {
    name: 'Desert-Based',
    formula: 'Distribution according to contribution/effort',
    proponents: ['Meritocrats'],
    challenge: 'How to measure desert? Talents are undeserved'
  },
  need: {
    name: 'Need-Based',
    formula: 'Distribution according to need',
    proponents: ['Marx ("to each according to need")'],
    challenge: 'How to define and measure need?'
  },
  utilitarian: {
    name: 'Utilitarian',
    formula: 'Distribution that maximizes total/average utility',
    proponents: ['Bentham', 'Mill', 'Singer'],
    challenge: 'May justify extreme inequality'
  },
  libertarian: {
    name: 'Libertarian',
    formula: 'Whatever results from free exchanges',
    proponents: ['Nozick', 'Hayek'],
    challenge: 'Ignores starting inequalities'
  },
  sufficientarian: {
    name: 'Sufficientarianism',
    formula: 'Ensure everyone has enough',
    proponents: ['Frankfurt', 'Crisp'],
    challenge: 'What counts as "enough"?'
  }
};

// State
const state = {
  distributions: new Map(),
  evaluations: [],
  comparisons: [],
  contracts: []
};

/**
 * Initialize the justice engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.distributions) state.distributions = new Map(Object.entries(saved.distributions));
      if (saved.evaluations) state.evaluations = saved.evaluations;
      if (saved.comparisons) state.comparisons = saved.comparisons;
      if (saved.contracts) state.contracts = saved.contracts;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', distributions: state.distributions.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    distributions: Object.fromEntries(state.distributions),
    evaluations: state.evaluations,
    comparisons: state.comparisons,
    contracts: state.contracts
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Create a distribution for evaluation
 */
function createDistribution(id, spec = {}) {
  const distribution = {
    id,
    description: spec.description || id,

    // Holdings by agent/group
    holdings: spec.holdings || {},

    // Total resources
    total: 0,

    // Metrics
    giniCoefficient: null,
    leastAdvantaged: null,
    averageHolding: null,

    // History (for entitlement theory)
    history: spec.history || [],

    createdAt: Date.now()
  };

  // Calculate metrics
  const values = Object.values(distribution.holdings);
  if (values.length > 0) {
    distribution.total = values.reduce((a, b) => a + b, 0);
    distribution.averageHolding = distribution.total / values.length;
    distribution.leastAdvantaged = Math.min(...values);
    distribution.giniCoefficient = calculateGini(values);
  }

  state.distributions.set(id, distribution);
  saveState();

  return distribution;
}

/**
 * Calculate Gini coefficient (measure of inequality)
 */
function calculateGini(values) {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const n = sorted.length;
  const mean = sorted.reduce((a, b) => a + b, 0) / n;
  if (mean === 0) return 0;

  let sumDiff = 0;
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      sumDiff += Math.abs(sorted[i] - sorted[j]);
    }
  }

  return sumDiff / (2 * n * n * mean);
}

/**
 * Evaluate distribution from Rawlsian perspective
 */
function evaluateRawlsian(distributionId) {
  const dist = state.distributions.get(distributionId);
  if (!dist) {
    return { error: 'Distribution not found' };
  }

  const values = Object.values(dist.holdings);
  const evaluation = {
    distributionId,
    framework: 'Rawlsian',

    // Veil of Ignorance perspective
    veilOfIgnorance: {
      worstPosition: Math.min(...values),
      bestPosition: Math.max(...values),
      expectedIfRandom: dist.averageHolding,
      risk: 'Unknown position - could be anyone'
    },

    // Difference Principle assessment
    differencePrinciple: {
      leastAdvantaged: dist.leastAdvantaged,
      benefitsLeastAdvantaged: null,
      assessment: null
    },

    // Maximin evaluation
    maximin: {
      worstOutcome: Math.min(...values),
      principle: 'Choose distribution that maximizes minimum',
      confidence: Math.min(PHI_INV_2, PHI_INV)
    },

    // Overall verdict
    verdict: null,
    confidence: null,

    timestamp: Date.now()
  };

  // Assess difference principle
  if (dist.giniCoefficient < 0.1) {
    evaluation.differencePrinciple.assessment = 'Very equal - likely just';
    evaluation.differencePrinciple.benefitsLeastAdvantaged = true;
  } else if (dist.leastAdvantaged >= dist.averageHolding * 0.5) {
    evaluation.differencePrinciple.assessment = 'Inequality present but least advantaged reasonably well off';
    evaluation.differencePrinciple.benefitsLeastAdvantaged = true;
  } else {
    evaluation.differencePrinciple.assessment = 'Inequality may not benefit least advantaged';
    evaluation.differencePrinciple.benefitsLeastAdvantaged = false;
  }

  // Overall verdict
  if (evaluation.differencePrinciple.benefitsLeastAdvantaged) {
    evaluation.verdict = 'JUST (Rawlsian)';
    evaluation.confidence = PHI_INV_2;
  } else {
    evaluation.verdict = 'UNJUST (Rawlsian) - fails difference principle';
    evaluation.confidence = PHI_INV_2;
  }

  state.evaluations.push(evaluation);
  saveState();

  return evaluation;
}

/**
 * Evaluate distribution from Nozickian perspective
 */
function evaluateNozickian(distributionId, history = []) {
  const dist = state.distributions.get(distributionId);
  if (!dist) {
    return { error: 'Distribution not found' };
  }

  const evaluation = {
    distributionId,
    framework: 'Nozickian',

    // Historical assessment
    historical: {
      acquisitions: [],
      transfers: [],
      violations: []
    },

    // Entitlement analysis
    entitlement: {
      justInAcquisition: null,
      justInTransfer: null,
      needsRectification: false
    },

    // Pattern critique
    patternCritique: {
      isPatternedDistribution: false,
      libertyConcern: 'Any pattern requires constant interference with liberty'
    },

    // Self-ownership consideration
    selfOwnership: {
      respected: null,
      assessment: null
    },

    // Overall verdict
    verdict: null,
    confidence: null,

    timestamp: Date.now()
  };

  // Analyze history
  const usedHistory = history.length > 0 ? history : dist.history;
  for (const event of usedHistory) {
    if (event.type === 'acquisition') {
      evaluation.historical.acquisitions.push(event);
      if (!event.just) {
        evaluation.historical.violations.push(event);
      }
    } else if (event.type === 'transfer') {
      evaluation.historical.transfers.push(event);
      if (!event.voluntary || event.fraud || event.force) {
        evaluation.historical.violations.push(event);
      }
    }
  }

  // Assess entitlement
  evaluation.entitlement.justInAcquisition =
    evaluation.historical.acquisitions.every(a => a.just !== false);
  evaluation.entitlement.justInTransfer =
    evaluation.historical.transfers.every(t => t.voluntary && !t.fraud && !t.force);
  evaluation.entitlement.needsRectification = evaluation.historical.violations.length > 0;

  // Self-ownership
  evaluation.selfOwnership.respected = !usedHistory.some(e => e.forcedLabor || e.theft);
  evaluation.selfOwnership.assessment = evaluation.selfOwnership.respected
    ? 'Self-ownership appears respected'
    : 'Self-ownership violations detected';

  // Overall verdict
  if (evaluation.entitlement.justInAcquisition &&
      evaluation.entitlement.justInTransfer &&
      evaluation.selfOwnership.respected) {
    evaluation.verdict = 'JUST (Nozickian) - entitled holdings';
    evaluation.confidence = PHI_INV_2;
  } else if (evaluation.entitlement.needsRectification) {
    evaluation.verdict = 'REQUIRES RECTIFICATION';
    evaluation.confidence = PHI_INV_2;
  } else {
    evaluation.verdict = 'UNJUST (Nozickian) - entitlement violations';
    evaluation.confidence = PHI_INV_2;
  }

  state.evaluations.push(evaluation);
  saveState();

  return evaluation;
}

/**
 * Compare two distributions (Rawlsian)
 */
function compareDistributions(dist1Id, dist2Id) {
  const d1 = state.distributions.get(dist1Id);
  const d2 = state.distributions.get(dist2Id);

  if (!d1 || !d2) {
    return { error: 'Distribution not found' };
  }

  const comparison = {
    distributions: [dist1Id, dist2Id],

    // Rawlsian comparison (difference principle)
    rawlsian: {
      leastAdvantaged: {
        [dist1Id]: d1.leastAdvantaged,
        [dist2Id]: d2.leastAdvantaged
      },
      preferred: d1.leastAdvantaged >= d2.leastAdvantaged ? dist1Id : dist2Id,
      reason: 'Maximin: prefer distribution with best worst-off position'
    },

    // Utilitarian comparison
    utilitarian: {
      total: {
        [dist1Id]: d1.total,
        [dist2Id]: d2.total
      },
      preferred: d1.total >= d2.total ? dist1Id : dist2Id,
      reason: 'Maximize total utility'
    },

    // Equality comparison
    equality: {
      gini: {
        [dist1Id]: d1.giniCoefficient,
        [dist2Id]: d2.giniCoefficient
      },
      preferred: d1.giniCoefficient <= d2.giniCoefficient ? dist1Id : dist2Id,
      reason: 'Lower Gini = more equal'
    },

    // Conflict analysis
    conflict: null,

    confidence: PHI_INV_3,
    timestamp: Date.now()
  };

  // Check for conflict
  const prefs = [
    comparison.rawlsian.preferred,
    comparison.utilitarian.preferred,
    comparison.equality.preferred
  ];

  if (new Set(prefs).size > 1) {
    comparison.conflict = {
      exists: true,
      description: 'Different frameworks prefer different distributions',
      resolution: 'Depends on which theory of justice one accepts'
    };
  } else {
    comparison.conflict = {
      exists: false,
      description: 'All frameworks agree'
    };
  }

  state.comparisons.push(comparison);
  saveState();

  return comparison;
}

/**
 * Apply Wilt Chamberlain argument
 */
function wiltChamberlainTest(initialDistId, transfers) {
  const initial = state.distributions.get(initialDistId);
  if (!initial) {
    return { error: 'Initial distribution not found' };
  }

  const test = {
    name: 'Wilt Chamberlain Argument',
    initialDistribution: initialDistId,

    // Assume initial is just
    step1: {
      distribution: initialDistId,
      assumption: 'D1 is just (any favored pattern)'
    },

    // Apply voluntary transfers
    step2: {
      transfers,
      nature: 'Voluntary payments to see Wilt play',
      eachTransferJust: transfers.every(t => t.voluntary && !t.force && !t.fraud)
    },

    // Result
    step3: {
      description: 'New distribution D2 emerges',
      wiltGains: transfers.reduce((sum, t) => sum + t.amount, 0)
    },

    // Nozick's conclusion
    conclusion: {
      ifD1JustAndTransfersJust: 'Then D2 must be just',
      patternDisrupted: true,
      nozickPoint: 'Liberty upsets patterns',
      implication: 'Maintaining patterns requires constant interference'
    },

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  return test;
}

/**
 * Evaluate policy against justice frameworks
 */
function evaluatePolicy(policy) {
  const evaluation = {
    policy: policy.name || policy.description,

    rawlsian: {
      framework: 'Rawls',
      questions: [
        'Would this be chosen behind the veil of ignorance?',
        'Does it benefit the least advantaged?',
        'Does it preserve fundamental liberties?'
      ],
      assessment: null,
      verdict: null
    },

    nozickian: {
      framework: 'Nozick',
      questions: [
        'Does it respect self-ownership?',
        'Does it stay within minimal state functions?',
        'Does it interfere with voluntary transfers?'
      ],
      assessment: null,
      verdict: null
    },

    synthesisChallenge: 'Rawls and Nozick often conflict - redistribution vs liberty',

    confidence: PHI_INV_3,
    timestamp: Date.now()
  };

  // Assess Rawlsian
  if (policy.redistributive) {
    evaluation.rawlsian.assessment = 'Redistribution can be just if it benefits least advantaged';
    evaluation.rawlsian.verdict = policy.benefitsLeastAdvantaged ? 'JUST' : 'UNCERTAIN';
  } else {
    evaluation.rawlsian.assessment = 'Non-redistributive - assess impact on least advantaged';
    evaluation.rawlsian.verdict = 'ASSESS EMPIRICALLY';
  }

  // Assess Nozickian
  if (policy.redistributive) {
    evaluation.nozickian.assessment = 'Redistribution violates self-ownership and liberty';
    evaluation.nozickian.verdict = 'UNJUST';
  } else if (policy.protectiveOnly) {
    evaluation.nozickian.assessment = 'Within minimal state functions';
    evaluation.nozickian.verdict = 'JUST';
  } else {
    evaluation.nozickian.assessment = 'Exceeds minimal state - assess scope';
    evaluation.nozickian.verdict = 'LIKELY UNJUST';
  }

  return evaluation;
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '⚖️  JUSTICE ENGINE - Rawls & Nozick',
    '═══════════════════════════════════════════════════════════',
    '',
    '── RAWLS ──────────────────────────────────────────────────',
    '   Veil of Ignorance → Original Position → Principles',
    '   1. Equal Liberty (priority)',
    '   2. Difference Principle (benefit least advantaged)',
    '',
    '── NOZICK ─────────────────────────────────────────────────',
    '   Entitlement Theory: Acquisition → Transfer → Holdings',
    '   Self-ownership + Minimal state',
    '   "Liberty upsets patterns"',
    '',
    '── TENSION ────────────────────────────────────────────────',
    '   Rawls: Redistribution can be just (difference principle)',
    '   Nozick: Redistribution violates self-ownership',
    '',
    '── STATISTICS ─────────────────────────────────────────────',
    `   Distributions: ${state.distributions.size}`,
    `   Evaluations: ${state.evaluations.length}`,
    `   Comparisons: ${state.comparisons.length}`,
    '',
    '═══════════════════════════════════════════════════════════',
    '*sniff* Justice demands both fairness AND respect for persons.',
    '═══════════════════════════════════════════════════════════'
  ];

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  return {
    distributions: state.distributions.size,
    evaluations: state.evaluations.length,
    comparisons: state.comparisons.length,
    contracts: state.contracts.length
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Distributions
  createDistribution,
  calculateGini,

  // Evaluations
  evaluateRawlsian,
  evaluateNozickian,
  compareDistributions,

  // Arguments
  wiltChamberlainTest,
  evaluatePolicy,

  // Theories
  RAWLS,
  NOZICK,
  DISTRIBUTIVE_PRINCIPLES,

  // Constants
  PHI,
  PHI_INV
};

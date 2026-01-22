/**
 * Explanation Engine - DN Model, Causation
 *
 * "To explain a phenomenon is to show that it was to be expected."
 * — Carl Hempel
 *
 * Implements:
 * - Deductive-Nomological (DN) model (Hempel)
 * - Causal-mechanical explanation
 * - Unificationist explanation
 * - Statistical explanation (IS model)
 *
 * φ guides confidence in explanatory assessments.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'explanation');

// Deductive-Nomological Model (Hempel)
const DN_MODEL = {
  name: 'Deductive-Nomological Model',
  aka: 'Covering Law Model',
  proponent: 'Carl Hempel (with Paul Oppenheim)',

  structure: {
    explanans: {
      description: 'The explaining part',
      components: [
        'General laws (L1, L2, ...)',
        'Initial/antecedent conditions (C1, C2, ...)'
      ]
    },
    explanandum: {
      description: 'The phenomenon to be explained (E)',
      derivation: 'E is logically deduced from explanans'
    }
  },

  requirements: {
    deductive: 'E must follow logically from premises',
    lawlike: 'Must contain at least one general law',
    empirical: 'Premises must have empirical content',
    truePremises: 'Premises must be true'
  },

  example: {
    laws: ['All metals expand when heated'],
    conditions: ['This rod is metal', 'This rod was heated'],
    explanandum: 'This rod expanded'
  },

  quote: 'To explain is to subsume under general laws'
};

// Problems with DN Model
const DN_PROBLEMS = {
  asymmetry: {
    name: 'Asymmetry Problem',
    description: 'DN allows explanations that get direction wrong',
    example: {
      correct: 'Flagpole height + sun angle → shadow length',
      incorrect: 'Shadow length + sun angle → flagpole height',
      problem: 'Both are valid DN explanations, but only first is genuine'
    },
    lesson: 'Explanation requires causal asymmetry'
  },
  irrelevance: {
    name: 'Irrelevance Problem',
    description: 'DN allows irrelevant factors in explanation',
    example: {
      silly: 'All men who take birth control pills fail to get pregnant. John takes pills. John fails to get pregnant.',
      problem: 'Valid DN form but pills are irrelevant'
    },
    lesson: 'Need relevance criterion beyond logical deduction'
  },
  accidentalGeneralizations: {
    name: 'Accidental Generalizations',
    description: 'DN cannot distinguish laws from accidents',
    example: {
      law: 'All copper conducts electricity',
      accident: 'All coins in my pocket are silver',
      problem: 'Both have same logical form'
    },
    lesson: 'Need account of lawhood'
  }
};

// Alternative Explanation Models
const EXPLANATION_MODELS = {
  causalMechanical: {
    name: 'Causal-Mechanical Model',
    proponent: 'Wesley Salmon',
    core: 'Explanation traces causal mechanisms',
    components: [
      'Causal processes (worldlines)',
      'Causal interactions (intersections)',
      'Mechanisms connecting cause to effect'
    ],
    advantage: 'Handles asymmetry - causes precede effects',
    example: 'Ball breaks window: trace ball trajectory, impact mechanism'
  },
  unificationist: {
    name: 'Unificationist Model',
    proponents: ['Friedman', 'Kitcher'],
    core: 'Explanation unifies diverse phenomena under few patterns',
    measure: 'Better explanation = more phenomena with fewer patterns',
    example: 'Newton unified falling apples and planetary orbits',
    advantage: 'Captures explanatory power of grand theories'
  },
  pragmatic: {
    name: 'Pragmatic/Contrastive Model',
    proponent: 'Bas van Fraassen',
    core: 'Explanation answers why-questions in context',
    structure: 'Why P rather than Q? (contrast class)',
    contextual: 'What counts as explanation depends on questioner',
    advantage: 'Handles context-sensitivity of explanation'
  },
  statisticalRelevance: {
    name: 'Statistical Relevance Model',
    proponent: 'Salmon',
    core: 'Explanation cites statistically relevant factors',
    criterion: 'Factor F explains E if P(E|F) ≠ P(E)',
    improvement: 'Goes beyond IS model high probability requirement'
  }
};

// Inductive-Statistical Model (Hempel)
const IS_MODEL = {
  name: 'Inductive-Statistical Model',
  description: 'Explanation using statistical laws',

  example: {
    law: 'P(recovery|takes antibiotic) = 0.95',
    condition: 'Jones took antibiotic',
    explanandum: 'Jones recovered'
  },

  requirements: {
    highProbability: 'Probability must be high (how high?)',
    maximalSpecificity: 'Use most specific reference class available'
  },

  problems: {
    lowProbability: 'Cannot explain unlikely events (but they happen)',
    referenceClass: 'Which reference class? (single-case problem)'
  }
};

// Causation Theories
const CAUSATION = {
  regularityTheory: {
    name: 'Regularity Theory',
    proponent: 'Hume',
    thesis: 'Causation is constant conjunction',
    definition: 'A causes B iff events like A are regularly followed by events like B',
    problems: [
      'Confuses correlation with causation',
      'Cannot handle single-case causation',
      'No direction (why not B causes A?)'
    ]
  },
  counterfactualTheory: {
    name: 'Counterfactual Theory',
    proponent: 'David Lewis',
    thesis: 'Causation is counterfactual dependence',
    definition: 'A causes B iff: if A had not occurred, B would not have occurred',
    advantages: ['Handles single cases', 'Captures causal relevance'],
    problems: [
      'Preemption cases',
      'Overdetermination',
      'Relies on controversial possible worlds semantics'
    ]
  },
  manipulationist: {
    name: 'Manipulationist/Interventionist Theory',
    proponent: 'James Woodward',
    thesis: 'Causation is potential for manipulation',
    definition: 'A causes B iff intervening on A would change B',
    slogan: 'No causation without manipulation',
    advantages: ['Connects to experimental method', 'Handles causal direction'],
    problems: ['Anthropocentric?', 'Unmanipulable causes (big bang)?']
  },
  processCausation: {
    name: 'Process Causation',
    proponent: 'Salmon',
    thesis: 'Causation involves transmission of conserved quantities',
    definition: 'Causal process transmits mark/conserved quantity',
    advantages: ['Physical foundation', 'Direction from temporal order'],
    problems: ['Negative causation (absences as causes)']
  }
};

// State
const state = {
  explanations: new Map(),
  analyses: [],
  causalClaims: []
};

/**
 * Initialize the explanation engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.explanations) state.explanations = new Map(Object.entries(saved.explanations));
      if (saved.analyses) state.analyses = saved.analyses;
      if (saved.causalClaims) state.causalClaims = saved.causalClaims;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', explanations: state.explanations.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    explanations: Object.fromEntries(state.explanations),
    analyses: state.analyses,
    causalClaims: state.causalClaims
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Create a DN-style explanation
 */
function createDNExplanation(id, spec = {}) {
  const explanation = {
    id,
    type: 'deductive-nomological',

    // Explanans
    laws: spec.laws || [],
    conditions: spec.conditions || [],

    // Explanandum
    explanandum: spec.explanandum || null,

    // Validity check
    validity: {
      hasLaws: false,
      hasConditions: false,
      deductivelyValid: false,
      empiricalContent: false
    },

    // Problems check
    problems: {
      asymmetry: false,
      irrelevance: false
    },

    // Status
    status: 'proposed',

    createdAt: Date.now()
  };

  // Check validity
  explanation.validity.hasLaws = explanation.laws.length > 0;
  explanation.validity.hasConditions = explanation.conditions.length > 0;
  explanation.validity.deductivelyValid = spec.deductivelyValid !== false;
  explanation.validity.empiricalContent = spec.empiricalContent !== false;

  state.explanations.set(id, explanation);
  saveState();

  return explanation;
}

/**
 * Evaluate a DN explanation
 */
function evaluateDNExplanation(explanationId) {
  const exp = state.explanations.get(explanationId);
  if (!exp) {
    return { error: 'Explanation not found' };
  }

  const evaluation = {
    explanationId,

    // Hempel requirements
    hempelRequirements: {
      deductive: exp.validity.deductivelyValid,
      containsLaws: exp.validity.hasLaws,
      empiricalContent: exp.validity.empiricalContent,
      truthOfPremises: 'assumed true'
    },

    // All requirements met?
    validDN: null,

    // Problem check
    knownProblems: {
      asymmetryRisk: 'Check if explanation could run in reverse',
      irrelevanceRisk: 'Check if all factors are genuinely relevant',
      accidentRisk: 'Check if laws are genuine laws or accidents'
    },

    // Alternative evaluation
    alternativeViews: {
      causalMechanical: 'Does explanation cite causal mechanism?',
      unificationist: 'Does explanation unify diverse phenomena?',
      pragmatic: 'Is explanation appropriate to context?'
    },

    // Overall assessment
    assessment: null,

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Check if valid DN
  evaluation.validDN =
    evaluation.hempelRequirements.deductive &&
    evaluation.hempelRequirements.containsLaws &&
    evaluation.hempelRequirements.empiricalContent;

  // Overall assessment
  if (evaluation.validDN) {
    evaluation.assessment = 'VALID DN EXPLANATION (subject to known problems)';
  } else {
    const missing = [];
    if (!evaluation.hempelRequirements.deductive) missing.push('non-deductive');
    if (!evaluation.hempelRequirements.containsLaws) missing.push('no laws');
    if (!evaluation.hempelRequirements.empiricalContent) missing.push('no empirical content');
    evaluation.assessment = 'INVALID DN: ' + missing.join(', ');
  }

  state.analyses.push(evaluation);
  saveState();

  return evaluation;
}

/**
 * Create a causal explanation
 */
function createCausalExplanation(id, spec = {}) {
  const explanation = {
    id,
    type: 'causal-mechanical',

    // Causal structure
    cause: spec.cause || null,
    effect: spec.effect || null,
    mechanism: spec.mechanism || null,

    // Causal chain
    chain: spec.chain || [],

    // Counterfactual test
    counterfactual: {
      ifCauseAbsent: spec.ifCauseAbsent || null,
      wouldEffectOccur: spec.wouldEffectOccur || null
    },

    // Causal theory used
    theory: spec.theory || 'counterfactual',

    createdAt: Date.now()
  };

  state.explanations.set(id, explanation);
  saveState();

  return explanation;
}

/**
 * Analyze a causal claim
 */
function analyzeCausalClaim(spec) {
  const analysis = {
    claim: spec.cause + ' causes ' + spec.effect,
    cause: spec.cause,
    effect: spec.effect,

    // Test under different theories
    theories: {
      regularity: {
        name: 'Regularity (Hume)',
        question: 'Is there constant conjunction?',
        answer: spec.constantConjunction || 'unknown',
        verdict: null
      },
      counterfactual: {
        name: 'Counterfactual (Lewis)',
        question: 'Would effect occur if cause absent?',
        answer: spec.counterfactualDependence || 'unknown',
        verdict: null
      },
      manipulationist: {
        name: 'Interventionist (Woodward)',
        question: 'Would intervening on cause change effect?',
        answer: spec.interventionWorks || 'unknown',
        verdict: null
      },
      process: {
        name: 'Process (Salmon)',
        question: 'Is there causal process connecting them?',
        answer: spec.causalProcess || 'unknown',
        verdict: null
      }
    },

    // Overall verdict
    causalClaim: null,
    confidence: PHI_INV_3,

    timestamp: Date.now()
  };

  // Evaluate under each theory
  for (const [key, theory] of Object.entries(analysis.theories)) {
    if (theory.answer === true || theory.answer === 'yes') {
      theory.verdict = 'SUPPORTED';
    } else if (theory.answer === false || theory.answer === 'no') {
      theory.verdict = 'NOT SUPPORTED';
    } else {
      theory.verdict = 'UNKNOWN';
    }
  }

  // Overall verdict
  const supported = Object.values(analysis.theories)
    .filter(t => t.verdict === 'SUPPORTED').length;

  if (supported >= 3) {
    analysis.causalClaim = 'STRONG CAUSAL CLAIM';
    analysis.confidence = PHI_INV_2;
  } else if (supported >= 2) {
    analysis.causalClaim = 'MODERATE CAUSAL CLAIM';
    analysis.confidence = PHI_INV_3;
  } else if (supported >= 1) {
    analysis.causalClaim = 'WEAK CAUSAL CLAIM';
    analysis.confidence = PHI_INV_3 * 0.5;
  } else {
    analysis.causalClaim = 'INSUFFICIENT EVIDENCE FOR CAUSATION';
    analysis.confidence = PHI_INV_3 * 0.25;
  }

  state.causalClaims.push(analysis);
  saveState();

  return analysis;
}

/**
 * Compare explanation models
 */
function compareExplanationModels() {
  return {
    comparison: 'Models of Scientific Explanation',

    dnModel: {
      name: DN_MODEL.name,
      core: 'Subsume under covering laws',
      strength: 'Clear logical structure',
      weakness: 'Asymmetry, irrelevance problems'
    },

    causalMechanical: {
      name: EXPLANATION_MODELS.causalMechanical.name,
      core: 'Trace causal mechanisms',
      strength: 'Handles asymmetry',
      weakness: 'What counts as mechanism?'
    },

    unificationist: {
      name: EXPLANATION_MODELS.unificationist.name,
      core: 'Unify phenomena under few patterns',
      strength: 'Captures explanatory power',
      weakness: 'How to measure unification?'
    },

    pragmatic: {
      name: EXPLANATION_MODELS.pragmatic.name,
      core: 'Answer context-dependent why-questions',
      strength: 'Handles context-sensitivity',
      weakness: 'Too relativistic?'
    },

    synthesis: {
      observation: 'Each model captures something important',
      pluralism: 'Perhaps explanation is multi-faceted',
      cynicView: 'Use all models, trust none absolutely'
    },

    confidence: PHI_INV,
    timestamp: Date.now()
  };
}

/**
 * Analyze explanation type
 */
function analyzeExplanationType(spec) {
  const analysis = {
    description: spec.description || 'Explanation',

    // Classify type
    typeAnalysis: {
      isDN: spec.hasLaws && spec.deductive,
      isCausal: spec.citesCause && spec.citesMechanism,
      isUnificationist: spec.unifiesPhenomena,
      isPragmatic: spec.answersWhyQuestion,
      isStatistical: spec.usesStatistics
    },

    // Best model
    bestModel: null,
    reasoning: null,

    // Quality indicators
    quality: {
      specificity: spec.specific || false,
      depth: spec.providesDepth || false,
      scope: spec.broadScope || false,
      predictive: spec.enablesPrediction || false
    },

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Determine best model
  if (analysis.typeAnalysis.isCausal) {
    analysis.bestModel = 'Causal-Mechanical';
    analysis.reasoning = 'Cites cause and mechanism';
  } else if (analysis.typeAnalysis.isDN) {
    analysis.bestModel = 'Deductive-Nomological';
    analysis.reasoning = 'Subsumes under laws';
  } else if (analysis.typeAnalysis.isUnificationist) {
    analysis.bestModel = 'Unificationist';
    analysis.reasoning = 'Unifies diverse phenomena';
  } else if (analysis.typeAnalysis.isStatistical) {
    analysis.bestModel = 'Inductive-Statistical';
    analysis.reasoning = 'Uses statistical regularities';
  } else if (analysis.typeAnalysis.isPragmatic) {
    analysis.bestModel = 'Pragmatic';
    analysis.reasoning = 'Context-sensitive why-question answer';
  } else {
    analysis.bestModel = 'Unclear';
    analysis.reasoning = 'Does not fit standard models';
  }

  return analysis;
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '📖 EXPLANATION ENGINE - DN Model & Causation',
    '═══════════════════════════════════════════════════════════',
    '',
    '── DN MODEL (Hempel) ──────────────────────────────────────',
    '   Laws + Conditions → Explanandum (deductively)',
    '   Problems: Asymmetry, Irrelevance, Accidents',
    '',
    '── ALTERNATIVES ───────────────────────────────────────────',
    '   Causal-Mechanical: Trace mechanisms (Salmon)',
    '   Unificationist: Unify phenomena (Kitcher)',
    '   Pragmatic: Context-dependent (van Fraassen)',
    '',
    '── CAUSATION THEORIES ─────────────────────────────────────',
    '   Regularity (Hume) | Counterfactual (Lewis)',
    '   Interventionist (Woodward) | Process (Salmon)',
    '',
    '── STATISTICS ─────────────────────────────────────────────',
    '   Explanations: ' + state.explanations.size,
    '   Analyses: ' + state.analyses.length,
    '   Causal claims: ' + state.causalClaims.length,
    '',
    '═══════════════════════════════════════════════════════════',
    '*sniff* To explain is to make expected, but how?',
    '═══════════════════════════════════════════════════════════'
  ];

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  return {
    explanations: state.explanations.size,
    analyses: state.analyses.length,
    causalClaims: state.causalClaims.length
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // DN Explanations
  createDNExplanation,
  evaluateDNExplanation,

  // Causal Explanations
  createCausalExplanation,
  analyzeCausalClaim,

  // Analysis
  analyzeExplanationType,
  compareExplanationModels,

  // Theory
  DN_MODEL,
  DN_PROBLEMS,
  EXPLANATION_MODELS,
  IS_MODEL,
  CAUSATION,

  // Constants
  PHI,
  PHI_INV
};

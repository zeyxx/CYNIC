/**
 * Scientific Method Engine - Popper/Kuhn
 *
 * "A theory which is not refutable by any conceivable event is non-scientific."
 * — Karl Popper
 *
 * "Normal science does not aim at novelties of fact or theory."
 * — Thomas Kuhn
 *
 * Implements:
 * - Popper's falsificationism (demarcation, corroboration, conjectures)
 * - Kuhn's paradigms (normal science, anomalies, crisis, revolution)
 * - Scientific method analysis and evaluation
 *
 * φ guides confidence in scientific assessments.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'scientific-method');

// Popper's Philosophy of Science
const POPPER = {
  falsificationism: {
    name: 'Falsificationism',
    description: 'Scientific theories must be falsifiable',
    criterion: 'A statement is scientific iff it can be refuted by observation',
    quote: 'The criterion of the scientific status of a theory is its falsifiability',
    contrast: {
      verificationism: 'Logical positivists sought verification (impossible for universals)',
      inductivism: 'Induction cannot justify universal laws (Hume\'s problem)'
    }
  },
  demarcation: {
    name: 'Demarcation Problem',
    question: 'What distinguishes science from non-science?',
    popperAnswer: 'Falsifiability',
    scientific: ['Relativity (predicts light bending)', 'Evolution (predicts fossil record)'],
    nonScientific: ['Psychoanalysis (explains everything)', 'Marxism (unfalsifiable)', 'Astrology'],
    note: 'Non-scientific ≠ meaningless, just not empirical science'
  },
  corroboration: {
    name: 'Corroboration',
    description: 'Surviving severe tests increases confidence (not probability)',
    notVerification: 'A theory is never verified, only corroborated',
    severity: 'Tests must be severe - theory risks refutation',
    degrees: {
      high: 'Survived many severe, varied tests',
      medium: 'Survived some tests',
      low: 'Few or easy tests',
      zero: 'Untested or unfalsifiable'
    }
  },
  conjecturesRefutations: {
    name: 'Conjectures and Refutations',
    method: [
      '1. Start with a problem',
      '2. Propose bold conjecture (theory)',
      '3. Deduce testable predictions',
      '4. Attempt refutation through experiment',
      '5. If refuted, modify or replace theory',
      '6. If corroborated, continue testing'
    ],
    growth: 'Knowledge grows through error elimination',
    quote: 'We learn from our mistakes'
  },
  adhocHypotheses: {
    name: 'Ad Hoc Hypotheses',
    description: 'Auxiliary hypotheses added to save a theory',
    problem: 'Reduce falsifiability',
    example: 'Epicycles in Ptolemaic astronomy',
    criterion: 'Ad hoc if it only saves the theory without new predictions'
  },
  verisimilitude: {
    name: 'Verisimilitude (Truth-likeness)',
    description: 'Theories can approach truth without reaching it',
    comparison: 'Some theories are closer to truth than others',
    problem: 'Popper\'s formal definition was shown to be flawed',
    intuition: 'Newton closer to truth than Aristotle, Einstein closer than Newton'
  }
};

// Kuhn's Philosophy of Science
const KUHN = {
  paradigm: {
    name: 'Paradigm',
    description: 'Shared framework of theories, methods, and values',
    components: [
      'Symbolic generalizations (laws, equations)',
      'Metaphysical commitments (ontology)',
      'Values (accuracy, consistency, scope)',
      'Exemplars (model problems and solutions)'
    ],
    examples: {
      newtonian: 'Newtonian mechanics paradigm',
      relativistic: 'Einsteinian/relativistic paradigm',
      quantum: 'Quantum mechanical paradigm'
    },
    quote: 'Scientists work within paradigms'
  },
  normalScience: {
    name: 'Normal Science',
    description: 'Puzzle-solving within the paradigm',
    characteristics: [
      'Articulate and extend paradigm',
      'Solve puzzles using paradigm tools',
      'Do not question fundamental assumptions',
      'Anomalies set aside as puzzles to solve later'
    ],
    puzzleSolving: {
      criterion: 'Success is solving puzzles, not discovering novelty',
      failure: 'Reflects on scientist, not paradigm'
    },
    quote: 'Normal science does not aim at novelties'
  },
  anomalies: {
    name: 'Anomalies',
    description: 'Observations that resist paradigm explanation',
    response: [
      'Initially ignored or set aside',
      'Attempts to resolve within paradigm',
      'If persistent, may trigger crisis'
    ],
    examples: {
      mercury: 'Mercury\'s orbit anomaly for Newtonian physics',
      blackbody: 'Black-body radiation for classical physics',
      michelsonMorley: 'Michelson-Morley experiment for ether theory'
    }
  },
  crisis: {
    name: 'Crisis',
    description: 'Accumulation of anomalies undermines confidence',
    characteristics: [
      'Proliferation of competing theories',
      'Questioning of fundamentals',
      'Willingness to try anything',
      'Philosophical debates intensify'
    ],
    resolution: 'Either paradigm adjusts or revolution occurs'
  },
  scientificRevolution: {
    name: 'Scientific Revolution',
    description: 'Paradigm shift - replacement of one paradigm by another',
    characteristics: [
      'Non-cumulative change',
      'Incommensurability between paradigms',
      'Gestalt shift in worldview',
      'New puzzles, methods, standards'
    ],
    examples: [
      'Copernican revolution',
      'Chemical revolution (Lavoisier)',
      'Darwinian revolution',
      'Quantum revolution',
      'Plate tectonics revolution'
    ],
    quote: 'Scientific revolutions are like political revolutions'
  },
  incommensurability: {
    name: 'Incommensurability',
    description: 'Paradigms cannot be directly compared',
    reasons: [
      'Different problems considered important',
      'Terms change meaning across paradigms',
      'Different standards of evaluation',
      'Different worldviews'
    ],
    implication: 'No neutral standpoint for comparison',
    controversy: 'Critics: leads to relativism; Kuhn: not quite'
  }
};

// Scientific Method Models
const METHOD_MODELS = {
  hypotheticoDeductive: {
    name: 'Hypothetico-Deductive Method',
    steps: [
      '1. Observe phenomenon',
      '2. Form hypothesis',
      '3. Deduce predictions',
      '4. Test predictions',
      '5. Confirm or disconfirm hypothesis'
    ],
    proponents: ['Popper', 'Hempel'],
    problems: ['Duhem-Quine problem', 'Underdetermination']
  },
  inductivism: {
    name: 'Inductivism',
    steps: [
      '1. Collect observations',
      '2. Find patterns',
      '3. Formulate general laws'
    ],
    proponents: ['Bacon', 'Mill', 'Logical positivists'],
    problems: ['Problem of induction (Hume)', 'Theory-ladenness of observation']
  },
  bayesian: {
    name: 'Bayesian Confirmation',
    description: 'Update probabilities based on evidence',
    formula: 'P(H|E) = P(E|H) × P(H) / P(E)',
    advantages: ['Quantitative', 'Handles degrees of belief'],
    problems: ['Where do priors come from?', 'Problem of old evidence']
  }
};

// State
const state = {
  theories: new Map(),
  tests: [],
  paradigmAnalyses: [],
  demarcations: []
};

/**
 * Initialize the scientific method engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.theories) state.theories = new Map(Object.entries(saved.theories));
      if (saved.tests) state.tests = saved.tests;
      if (saved.paradigmAnalyses) state.paradigmAnalyses = saved.paradigmAnalyses;
      if (saved.demarcations) state.demarcations = saved.demarcations;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', theories: state.theories.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    theories: Object.fromEntries(state.theories),
    tests: state.tests,
    paradigmAnalyses: state.paradigmAnalyses,
    demarcations: state.demarcations
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Register a scientific theory
 */
function registerTheory(id, spec = {}) {
  const theory = {
    id,
    name: spec.name || id,
    description: spec.description || '',

    // Falsifiability (Popper)
    falsifiable: spec.falsifiable !== false,
    falsificationConditions: spec.falsificationConditions || [],
    predictions: spec.predictions || [],

    // Paradigm context (Kuhn)
    paradigm: spec.paradigm || null,

    // Test history
    tests: [],
    corroborationLevel: 'untested',

    // Status
    status: 'proposed',
    refuted: false,
    refutationDetails: null,

    createdAt: Date.now()
  };

  state.theories.set(id, theory);
  saveState();

  return theory;
}

/**
 * Assess falsifiability (Popper's demarcation)
 */
function assessFalsifiability(theoryId) {
  const theory = state.theories.get(theoryId);
  if (!theory) {
    return { error: 'Theory not found' };
  }

  const assessment = {
    theoryId,
    name: theory.name,

    // Popper's criterion
    falsifiable: theory.falsifiable,
    falsificationConditions: theory.falsificationConditions,

    // Analysis
    analysis: {
      hasPredictions: theory.predictions.length > 0,
      predictionsTestable: null,
      conditionsSpecific: theory.falsificationConditions.length > 0,
      potentiallyRefutable: null
    },

    // Verdict
    scientific: null,
    popperVerdict: null,

    // Comparison with paradigm cases
    comparison: {
      likeRelativity: null,
      likePsychoanalysis: null
    },

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Analyze testability
  assessment.analysis.predictionsTestable =
    theory.predictions.some(p => p.testable !== false);

  assessment.analysis.potentiallyRefutable =
    assessment.analysis.hasPredictions &&
    assessment.analysis.predictionsTestable &&
    assessment.analysis.conditionsSpecific;

  // Verdict
  if (assessment.analysis.potentiallyRefutable) {
    assessment.scientific = true;
    assessment.popperVerdict = 'SCIENTIFIC - falsifiable theory';
    assessment.comparison.likeRelativity = true;
    assessment.comparison.likePsychoanalysis = false;
  } else {
    assessment.scientific = false;
    assessment.popperVerdict = 'NON-SCIENTIFIC - not falsifiable';
    assessment.comparison.likeRelativity = false;
    assessment.comparison.likePsychoanalysis = true;
  }

  state.demarcations.push(assessment);
  saveState();

  return assessment;
}

/**
 * Record a test of a theory
 */
function recordTest(theoryId, test) {
  const theory = state.theories.get(theoryId);
  if (!theory) {
    return { error: 'Theory not found' };
  }

  const testRecord = {
    theoryId,
    testId: `test-${Date.now()}`,

    // Test details
    description: test.description || 'Test',
    prediction: test.prediction || null,
    observation: test.observation || null,

    // Severity (Popper)
    severity: test.severity || 'medium',
    riskLevel: test.riskLevel || 'moderate',

    // Result
    result: test.result || 'inconclusive',  // 'passed', 'failed', 'inconclusive'
    refutes: test.result === 'failed',
    corroborates: test.result === 'passed',

    // Analysis
    analysis: {
      severeTest: test.severity === 'high',
      boldPrediction: test.riskLevel === 'high',
      novelPrediction: test.novel || false
    },

    timestamp: Date.now()
  };

  // Update theory
  theory.tests.push(testRecord);

  if (testRecord.refutes) {
    theory.status = 'refuted';
    theory.refuted = true;
    theory.refutationDetails = testRecord;
  } else if (testRecord.corroborates) {
    // Update corroboration level
    const passedTests = theory.tests.filter(t => t.corroborates);
    const severeTests = passedTests.filter(t => t.analysis.severeTest);

    if (severeTests.length >= 3) {
      theory.corroborationLevel = 'high';
    } else if (passedTests.length >= 3 || severeTests.length >= 1) {
      theory.corroborationLevel = 'medium';
    } else {
      theory.corroborationLevel = 'low';
    }
    theory.status = 'corroborated';
  }

  state.tests.push(testRecord);
  saveState();

  return testRecord;
}

/**
 * Evaluate corroboration level (Popper)
 */
function evaluateCorroboration(theoryId) {
  const theory = state.theories.get(theoryId);
  if (!theory) {
    return { error: 'Theory not found' };
  }

  const evaluation = {
    theoryId,
    name: theory.name,

    // Test summary
    testSummary: {
      total: theory.tests.length,
      passed: theory.tests.filter(t => t.corroborates).length,
      failed: theory.tests.filter(t => t.refutes).length,
      inconclusive: theory.tests.filter(t => !t.corroborates && !t.refutes).length
    },

    // Severity analysis
    severityAnalysis: {
      severeTests: theory.tests.filter(t => t.analysis.severeTest).length,
      severePassed: theory.tests.filter(t => t.analysis.severeTest && t.corroborates).length,
      boldPredictions: theory.tests.filter(t => t.analysis.boldPrediction).length,
      novelPredictions: theory.tests.filter(t => t.analysis.novelPrediction).length
    },

    // Corroboration level
    corroborationLevel: theory.corroborationLevel,
    corroborationDetails: null,

    // Popper's caveat
    popperCaveat: 'Corroboration ≠ probability of truth; just survival of tests',

    confidence: Math.min(PHI_INV_2, PHI_INV),
    timestamp: Date.now()
  };

  // Detail corroboration
  switch (theory.corroborationLevel) {
    case 'high':
      evaluation.corroborationDetails = 'Survived multiple severe tests - well corroborated';
      break;
    case 'medium':
      evaluation.corroborationDetails = 'Survived some tests - moderately corroborated';
      break;
    case 'low':
      evaluation.corroborationDetails = 'Few tests passed - weakly corroborated';
      break;
    default:
      evaluation.corroborationDetails = 'Not yet tested';
  }

  return evaluation;
}

/**
 * Analyze paradigm status (Kuhn)
 */
function analyzeParadigm(spec) {
  const analysis = {
    paradigmName: spec.name || 'Paradigm',
    field: spec.field || 'Unknown field',

    // Current state
    state: spec.state || 'normal',  // 'pre-paradigm', 'normal', 'crisis', 'revolution'

    // Normal science indicators
    normalScienceIndicators: {
      sharedFramework: spec.sharedFramework || false,
      puzzleSolving: spec.puzzleSolving || false,
      textbookConsensus: spec.textbookConsensus || false,
      cumulativeProgress: spec.cumulativeProgress || false
    },

    // Crisis indicators
    crisisIndicators: {
      persistentAnomalies: spec.anomalies || [],
      competingTheories: spec.competingTheories || [],
      philosophicalDebates: spec.philosophicalDebates || false,
      questioningFundamentals: spec.questioningFundamentals || false
    },

    // Diagnosis
    diagnosis: null,
    kuhnAnalysis: null,

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Diagnose state
  const normalIndicators = Object.values(analysis.normalScienceIndicators).filter(Boolean).length;
  const crisisIndicatorCount =
    analysis.crisisIndicators.persistentAnomalies.length +
    analysis.crisisIndicators.competingTheories.length +
    (analysis.crisisIndicators.philosophicalDebates ? 1 : 0) +
    (analysis.crisisIndicators.questioningFundamentals ? 1 : 0);

  if (normalIndicators < 2) {
    analysis.diagnosis = 'PRE-PARADIGM';
    analysis.kuhnAnalysis = 'No dominant paradigm yet - competing schools';
  } else if (crisisIndicatorCount >= 3) {
    analysis.diagnosis = 'CRISIS';
    analysis.kuhnAnalysis = 'Paradigm in crisis - revolution may be imminent';
  } else if (crisisIndicatorCount >= 1) {
    analysis.diagnosis = 'NORMAL SCIENCE (with anomalies)';
    analysis.kuhnAnalysis = 'Dominant paradigm with some unresolved puzzles';
  } else {
    analysis.diagnosis = 'NORMAL SCIENCE';
    analysis.kuhnAnalysis = 'Healthy paradigm - puzzle-solving proceeding';
  }

  state.paradigmAnalyses.push(analysis);
  saveState();

  return analysis;
}

/**
 * Assess incommensurability between paradigms
 */
function assessIncommensurability(paradigm1, paradigm2) {
  const assessment = {
    paradigms: [paradigm1.name, paradigm2.name],

    // Dimensions of incommensurability
    dimensions: {
      problemDifference: {
        description: 'Different problems considered important',
        paradigm1Problems: paradigm1.problems || [],
        paradigm2Problems: paradigm2.problems || [],
        overlap: null
      },
      meaningChange: {
        description: 'Key terms change meaning',
        examples: paradigm1.termChanges || [],
        significance: null
      },
      standardsDifference: {
        description: 'Different standards of evaluation',
        paradigm1Standards: paradigm1.standards || [],
        paradigm2Standards: paradigm2.standards || [],
        compatible: null
      },
      worldviewDifference: {
        description: 'Different ways of seeing the world',
        paradigm1Ontology: paradigm1.ontology || null,
        paradigm2Ontology: paradigm2.ontology || null,
        gestaltShift: null
      }
    },

    // Overall assessment
    incommensurabilityLevel: null,
    canCompare: null,
    kuhnPoint: 'No neutral standpoint for comparison',

    confidence: PHI_INV_3,
    timestamp: Date.now()
  };

  // Calculate problem overlap
  if (assessment.dimensions.problemDifference.paradigm1Problems.length > 0 &&
      assessment.dimensions.problemDifference.paradigm2Problems.length > 0) {
    const p1 = new Set(assessment.dimensions.problemDifference.paradigm1Problems);
    const p2 = new Set(assessment.dimensions.problemDifference.paradigm2Problems);
    const intersection = [...p1].filter(x => p2.has(x));
    assessment.dimensions.problemDifference.overlap =
      intersection.length / Math.max(p1.size, p2.size);
  }

  // Assess standards compatibility
  assessment.dimensions.standardsDifference.compatible =
    assessment.dimensions.standardsDifference.paradigm1Standards.some(s =>
      assessment.dimensions.standardsDifference.paradigm2Standards.includes(s)
    );

  // Overall level
  let incommScore = 0;
  if (assessment.dimensions.problemDifference.overlap !== null &&
      assessment.dimensions.problemDifference.overlap < 0.3) incommScore++;
  if (assessment.dimensions.meaningChange.examples.length > 0) incommScore++;
  if (!assessment.dimensions.standardsDifference.compatible) incommScore++;
  if (assessment.dimensions.worldviewDifference.paradigm1Ontology !==
      assessment.dimensions.worldviewDifference.paradigm2Ontology) incommScore++;

  if (incommScore >= 3) {
    assessment.incommensurabilityLevel = 'HIGH';
    assessment.canCompare = false;
  } else if (incommScore >= 2) {
    assessment.incommensurabilityLevel = 'MODERATE';
    assessment.canCompare = 'partially';
  } else {
    assessment.incommensurabilityLevel = 'LOW';
    assessment.canCompare = true;
  }

  return assessment;
}

/**
 * Compare Popper and Kuhn views
 */
function comparePopperKuhn() {
  return {
    comparison: 'Popper vs Kuhn',

    popper: {
      view: 'Falsificationism',
      scienceProgress: 'Through bold conjectures and refutations',
      demarcation: 'Falsifiability distinguishes science',
      rationality: 'Science is rational - objective standards',
      history: 'History shows rational progress toward truth',
      keywords: ['Falsification', 'Corroboration', 'Conjectures', 'Critical rationalism']
    },

    kuhn: {
      view: 'Paradigm theory',
      scienceProgress: 'Through paradigm shifts (revolutionary)',
      demarcation: 'Paradigm membership (sociological)',
      rationality: 'Paradigm-relative standards (not universal)',
      history: 'History shows discontinuous revolutions',
      keywords: ['Paradigm', 'Normal science', 'Anomaly', 'Incommensurability']
    },

    keyDifferences: [
      {
        topic: 'How does science progress?',
        popper: 'Cumulatively through error elimination',
        kuhn: 'Through revolutionary paradigm shifts'
      },
      {
        topic: 'Is there a neutral standard for theory choice?',
        popper: 'Yes - falsifiability and corroboration',
        kuhn: 'No - standards are paradigm-relative'
      },
      {
        topic: 'Role of scientific community?',
        popper: 'Critical testing of theories',
        kuhn: 'Puzzle-solving within paradigm'
      },
      {
        topic: 'What happens when theory meets counterevidence?',
        popper: 'Theory should be rejected (falsified)',
        kuhn: 'Anomaly set aside during normal science'
      }
    ],

    synthesis: {
      lakatosAttempt: 'Lakatos tried to combine both views',
      possibleIntegration: 'Falsification works within paradigms; revolutions require paradigm shift'
    },

    confidence: PHI_INV,
    timestamp: Date.now()
  };
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '🔬 SCIENTIFIC METHOD ENGINE - Popper & Kuhn',
    '═══════════════════════════════════════════════════════════',
    '',
    '── POPPER ─────────────────────────────────────────────────',
    '   Falsificationism: Science = falsifiable theories',
    '   Method: Conjectures and refutations',
    '   Progress: Through error elimination',
    '',
    '── KUHN ───────────────────────────────────────────────────',
    '   Paradigms: Shared frameworks for science',
    '   Normal Science → Anomaly → Crisis → Revolution',
    '   Incommensurability between paradigms',
    '',
    '── TENSION ────────────────────────────────────────────────',
    '   Popper: Rational progress toward truth',
    '   Kuhn: Revolutionary shifts, paradigm-relative standards',
    '',
    '── STATISTICS ─────────────────────────────────────────────',
    `   Theories: ${state.theories.size}`,
    `   Tests: ${state.tests.length}`,
    `   Paradigm analyses: ${state.paradigmAnalyses.length}`,
    `   Demarcations: ${state.demarcations.length}`,
    '',
    '═══════════════════════════════════════════════════════════',
    '*sniff* Bold conjectures, severe tests, paradigm awareness.',
    '═══════════════════════════════════════════════════════════'
  ];

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  return {
    theories: state.theories.size,
    tests: state.tests.length,
    paradigmAnalyses: state.paradigmAnalyses.length,
    demarcations: state.demarcations.length
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Theories
  registerTheory,
  assessFalsifiability,
  recordTest,
  evaluateCorroboration,

  // Paradigms (Kuhn)
  analyzeParadigm,
  assessIncommensurability,

  // Comparison
  comparePopperKuhn,

  // Theory
  POPPER,
  KUHN,
  METHOD_MODELS,

  // Constants
  PHI,
  PHI_INV
};

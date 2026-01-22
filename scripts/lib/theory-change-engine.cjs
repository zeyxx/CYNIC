/**
 * Theory Change Engine - Paradigm Shifts, Lakatos
 *
 * "The history of science is the judge."
 * — Imre Lakatos
 *
 * Implements:
 * - Lakatos's research programmes (hard core, protective belt)
 * - Theory change dynamics (progressive vs degenerating)
 * - Paradigm shift analysis
 * - Theory comparison and succession
 *
 * φ guides confidence in theory change assessments.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'theory-change');

// Lakatos's Methodology of Scientific Research Programmes
const LAKATOS = {
  researchProgramme: {
    name: 'Research Programme',
    description: 'A series of theories with shared core assumptions',
    components: {
      hardCore: {
        name: 'Hard Core',
        description: 'Fundamental assumptions that define the programme',
        characteristic: 'Irrefutable by methodological decision',
        examples: {
          newtonian: 'Absolute space and time, three laws of motion, universal gravitation',
          darwinian: 'Natural selection, common descent, gradualism',
          freudian: 'Unconscious, repression, psychosexual development'
        }
      },
      protectiveBelt: {
        name: 'Protective Belt',
        description: 'Auxiliary hypotheses that can be modified',
        function: 'Absorbs anomalies by adjustment',
        examples: 'Perturbations in orbits explained by new planets'
      },
      heuristics: {
        positive: {
          name: 'Positive Heuristic',
          description: 'Guidelines for developing the programme',
          role: 'Directs research toward new predictions'
        },
        negative: {
          name: 'Negative Heuristic',
          description: 'Do not attack the hard core',
          role: 'Protects fundamental assumptions'
        }
      }
    }
  },

  progressiveDegenerate: {
    progressive: {
      name: 'Progressive Programme',
      criteria: [
        'Theoretically progressive: predicts novel facts',
        'Empirically progressive: some novel facts confirmed',
        'Excess content over predecessor'
      ],
      characteristic: 'Growing explanatory and predictive power'
    },
    degenerating: {
      name: 'Degenerating Programme',
      criteria: [
        'Only accommodates known anomalies',
        'No novel predictions',
        'Ad hoc adjustments dominate'
      ],
      characteristic: 'Defensive, backward-looking'
    },
    quote: 'A programme is progressive if it leads to novel facts'
  },

  methodologicalRules: {
    rule1: 'Never abandon a programme just because of anomalies',
    rule2: 'A programme should be abandoned only when a better one is available',
    rule3: 'Novel predictions are the mark of good science',
    rule4: 'History judges programmes in hindsight'
  },

  versusPopper: {
    agreement: 'Science should make bold, testable predictions',
    disagreement: 'Single refutations do not kill theories',
    lakatos: 'History shows theories survive anomalies',
    synthesis: 'Sophisticated falsificationism'
  },

  versusKuhn: {
    agreement: 'Scientific change is complex, not instant refutation',
    disagreement: 'There are rational standards for theory choice',
    lakatos: 'Progressive vs degenerating is objective',
    kuhn: 'Incommensurability makes comparison problematic'
  }
};

// Theory Change Patterns
const CHANGE_PATTERNS = {
  revolution: {
    name: 'Revolutionary Change',
    description: 'Wholesale replacement of theoretical framework',
    characteristics: [
      'Fundamental assumptions change',
      'New ontology',
      'New problems and standards',
      'Discontinuous with predecessor'
    ],
    examples: ['Copernican', 'Darwinian', 'Einsteinian'],
    kuhnian: true
  },
  evolution: {
    name: 'Evolutionary Change',
    description: 'Gradual modification within framework',
    characteristics: [
      'Core preserved',
      'Auxiliary hypotheses refined',
      'Cumulative growth',
      'Continuous with predecessor'
    ],
    examples: ['Refinement of Newtonian mechanics', 'Modern synthesis in biology'],
    kuhnian: false
  },
  incorporation: {
    name: 'Incorporation/Reduction',
    description: 'Old theory becomes special case of new',
    characteristics: [
      'New theory explains old success',
      'Old theory has limited domain',
      'Approximate truth preserved'
    ],
    examples: ['Newtonian mechanics as limit of relativity', 'Classical as limit of quantum'],
    nagelian: true
  },
  elimination: {
    name: 'Elimination',
    description: 'Old theory rejected as fundamentally wrong',
    characteristics: [
      'No reduction possible',
      'Old ontology abandoned',
      'No correspondence with new'
    ],
    examples: ['Phlogiston', 'Caloric', 'Luminiferous ether'],
    feyerabend: true
  }
};

// Theory Comparison Criteria
const COMPARISON_CRITERIA = {
  empirical: {
    name: 'Empirical Adequacy',
    description: 'Fits observable data',
    measure: 'Number and variety of confirmed predictions'
  },
  scope: {
    name: 'Scope/Breadth',
    description: 'Range of phenomena explained',
    measure: 'Diversity of domains covered'
  },
  precision: {
    name: 'Precision',
    description: 'Specificity of predictions',
    measure: 'Quantitative vs qualitative predictions'
  },
  fruitfulness: {
    name: 'Fruitfulness',
    description: 'Generates new research',
    measure: 'Novel predictions, new questions'
  },
  simplicity: {
    name: 'Simplicity/Parsimony',
    description: 'Fewer assumptions, entities',
    measure: 'Occam\'s razor compliance'
  },
  consistency: {
    name: 'Internal/External Consistency',
    description: 'No contradictions, fits other knowledge',
    measure: 'Logical coherence, integration'
  },
  unification: {
    name: 'Unifying Power',
    description: 'Connects disparate phenomena',
    measure: 'Reduction of independent assumptions'
  }
};

// State
const state = {
  programmes: new Map(),
  theorySuccessions: [],
  changeAnalyses: [],
  comparisons: []
};

/**
 * Initialize the theory change engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.programmes) state.programmes = new Map(Object.entries(saved.programmes));
      if (saved.theorySuccessions) state.theorySuccessions = saved.theorySuccessions;
      if (saved.changeAnalyses) state.changeAnalyses = saved.changeAnalyses;
      if (saved.comparisons) state.comparisons = saved.comparisons;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', programmes: state.programmes.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    programmes: Object.fromEntries(state.programmes),
    theorySuccessions: state.theorySuccessions,
    changeAnalyses: state.changeAnalyses,
    comparisons: state.comparisons
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Create a research programme (Lakatos)
 */
function createResearchProgramme(id, spec = {}) {
  const programme = {
    id,
    name: spec.name || id,
    field: spec.field || 'unspecified',

    // Lakatos structure
    hardCore: spec.hardCore || [],
    protectiveBelt: spec.protectiveBelt || [],
    positiveHeuristic: spec.positiveHeuristic || [],
    negativeHeuristic: spec.negativeHeuristic || ['Do not attack hard core'],

    // History
    theories: spec.theories || [],
    novelPredictions: spec.novelPredictions || [],
    confirmedPredictions: spec.confirmedPredictions || [],
    anomalies: spec.anomalies || [],
    adHocModifications: spec.adHocModifications || [],

    // Status
    status: 'active',  // 'active', 'progressive', 'degenerating', 'abandoned'
    progressivenessScore: null,

    createdAt: Date.now()
  };

  state.programmes.set(id, programme);
  saveState();

  return programme;
}

/**
 * Evaluate programme progressiveness (Lakatos)
 */
function evaluateProgressiveness(programmeId) {
  const prog = state.programmes.get(programmeId);
  if (!prog) {
    return { error: 'Programme not found' };
  }

  const evaluation = {
    programmeId,
    name: prog.name,

    // Lakatos criteria
    theoreticalProgress: {
      question: 'Does it predict novel facts?',
      novelPredictions: prog.novelPredictions.length,
      assessment: null
    },

    empiricalProgress: {
      question: 'Are novel facts confirmed?',
      confirmedNovel: prog.confirmedPredictions.length,
      totalNovel: prog.novelPredictions.length,
      ratio: null,
      assessment: null
    },

    adHocAnalysis: {
      question: 'Is it mostly defensive adjustments?',
      adHocCount: prog.adHocModifications.length,
      anomalyCount: prog.anomalies.length,
      ratio: null,
      assessment: null
    },

    // Overall verdict
    status: null,
    lakatosVerdict: null,

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Theoretical progress
  if (evaluation.theoreticalProgress.novelPredictions >= 3) {
    evaluation.theoreticalProgress.assessment = 'THEORETICALLY PROGRESSIVE';
  } else if (evaluation.theoreticalProgress.novelPredictions >= 1) {
    evaluation.theoreticalProgress.assessment = 'SOME THEORETICAL PROGRESS';
  } else {
    evaluation.theoreticalProgress.assessment = 'NOT THEORETICALLY PROGRESSIVE';
  }

  // Empirical progress
  if (evaluation.empiricalProgress.totalNovel > 0) {
    evaluation.empiricalProgress.ratio =
      evaluation.empiricalProgress.confirmedNovel / evaluation.empiricalProgress.totalNovel;

    if (evaluation.empiricalProgress.ratio >= 0.5) {
      evaluation.empiricalProgress.assessment = 'EMPIRICALLY PROGRESSIVE';
    } else if (evaluation.empiricalProgress.ratio > 0) {
      evaluation.empiricalProgress.assessment = 'SOME EMPIRICAL PROGRESS';
    } else {
      evaluation.empiricalProgress.assessment = 'NOT EMPIRICALLY PROGRESSIVE';
    }
  } else {
    evaluation.empiricalProgress.ratio = 0;
    evaluation.empiricalProgress.assessment = 'NO NOVEL PREDICTIONS TO TEST';
  }

  // Ad hoc analysis
  if (evaluation.adHocAnalysis.anomalyCount > 0) {
    evaluation.adHocAnalysis.ratio =
      evaluation.adHocAnalysis.adHocCount / evaluation.adHocAnalysis.anomalyCount;

    if (evaluation.adHocAnalysis.ratio > 0.7) {
      evaluation.adHocAnalysis.assessment = 'HEAVILY AD HOC - degenerating sign';
    } else if (evaluation.adHocAnalysis.ratio > 0.3) {
      evaluation.adHocAnalysis.assessment = 'SOME AD HOC ADJUSTMENTS';
    } else {
      evaluation.adHocAnalysis.assessment = 'MINIMAL AD HOC - good sign';
    }
  } else {
    evaluation.adHocAnalysis.ratio = 0;
    evaluation.adHocAnalysis.assessment = 'NO ANOMALIES YET';
  }

  // Overall status
  const theoreticallyProg = evaluation.theoreticalProgress.assessment.includes('PROGRESSIVE');
  const empiricallyProg = evaluation.empiricalProgress.assessment.includes('PROGRESSIVE');
  const notAdHoc = !evaluation.adHocAnalysis.assessment.includes('HEAVILY');

  if (theoreticallyProg && empiricallyProg && notAdHoc) {
    evaluation.status = 'PROGRESSIVE';
    evaluation.lakatosVerdict = 'Progressive research programme - pursue it';
  } else if (!theoreticallyProg && !empiricallyProg) {
    evaluation.status = 'DEGENERATING';
    evaluation.lakatosVerdict = 'Degenerating programme - consider alternatives';
  } else {
    evaluation.status = 'STAGNANT';
    evaluation.lakatosVerdict = 'Neither clearly progressive nor degenerating';
  }

  // Update programme
  prog.status = evaluation.status.toLowerCase();
  prog.progressivenessScore = (theoreticallyProg ? 1 : 0) + (empiricallyProg ? 1 : 0) + (notAdHoc ? 1 : 0);
  saveState();

  state.changeAnalyses.push(evaluation);
  saveState();

  return evaluation;
}

/**
 * Record a theory succession
 */
function recordSuccession(spec) {
  const succession = {
    id: 'succession-' + Date.now(),

    predecessor: {
      name: spec.predecessorName,
      programmeId: spec.predecessorProgramme || null
    },
    successor: {
      name: spec.successorName,
      programmeId: spec.successorProgramme || null
    },

    // Type of change
    changeType: spec.changeType || 'unknown',  // revolution, evolution, incorporation, elimination
    changePattern: CHANGE_PATTERNS[spec.changeType] || null,

    // Analysis
    analysis: {
      hardCoreChanged: spec.hardCoreChanged || false,
      protectiveBeltChanged: spec.protectiveBeltChanged || false,
      ontologyChanged: spec.ontologyChanged || false,
      standardsChanged: spec.standardsChanged || false
    },

    // Reasons for change
    reasons: spec.reasons || [],
    anomaliesDriving: spec.anomalies || [],

    // Correspondence
    correspondence: {
      reducible: spec.reducible || false,
      limitCase: spec.limitCase || false,
      approximation: spec.approximation || false
    },

    timestamp: Date.now()
  };

  // Determine change type if not specified
  if (succession.changeType === 'unknown') {
    if (succession.analysis.hardCoreChanged && succession.analysis.ontologyChanged) {
      succession.changeType = 'revolution';
    } else if (succession.correspondence.limitCase) {
      succession.changeType = 'incorporation';
    } else if (succession.analysis.hardCoreChanged && !succession.correspondence.reducible) {
      succession.changeType = 'elimination';
    } else {
      succession.changeType = 'evolution';
    }
    succession.changePattern = CHANGE_PATTERNS[succession.changeType];
  }

  state.theorySuccessions.push(succession);
  saveState();

  return succession;
}

/**
 * Compare two programmes (Lakatos)
 */
function compareProgrammes(prog1Id, prog2Id) {
  const p1 = state.programmes.get(prog1Id);
  const p2 = state.programmes.get(prog2Id);

  if (!p1 || !p2) {
    return { error: 'Programme not found' };
  }

  const comparison = {
    programmes: [prog1Id, prog2Id],

    // Lakatos comparison
    lakatosian: {
      p1Progressiveness: p1.progressivenessScore || 0,
      p2Progressiveness: p2.progressivenessScore || 0,
      moreProgressive: null,
      lakatosAdvice: null
    },

    // Criteria comparison
    criteria: {},

    // Hard core comparison
    hardCoreComparison: {
      p1Core: p1.hardCore,
      p2Core: p2.hardCore,
      overlap: null,
      compatible: null
    },

    // Overall
    betterProgramme: null,
    reasoning: null,

    confidence: PHI_INV_3,
    timestamp: Date.now()
  };

  // Lakatos comparison
  if (comparison.lakatosian.p1Progressiveness > comparison.lakatosian.p2Progressiveness) {
    comparison.lakatosian.moreProgressive = prog1Id;
    comparison.lakatosian.lakatosAdvice = 'Pursue ' + prog1Id + ' (more progressive)';
  } else if (comparison.lakatosian.p2Progressiveness > comparison.lakatosian.p1Progressiveness) {
    comparison.lakatosian.moreProgressive = prog2Id;
    comparison.lakatosian.lakatosAdvice = 'Pursue ' + prog2Id + ' (more progressive)';
  } else {
    comparison.lakatosian.moreProgressive = 'neither';
    comparison.lakatosian.lakatosAdvice = 'Both equally progressive - continue both';
  }

  // Criteria comparison
  comparison.criteria = {
    novelPredictions: {
      p1: p1.novelPredictions.length,
      p2: p2.novelPredictions.length,
      better: p1.novelPredictions.length >= p2.novelPredictions.length ? prog1Id : prog2Id
    },
    confirmedPredictions: {
      p1: p1.confirmedPredictions.length,
      p2: p2.confirmedPredictions.length,
      better: p1.confirmedPredictions.length >= p2.confirmedPredictions.length ? prog1Id : prog2Id
    },
    adHocRatio: {
      p1: p1.anomalies.length > 0 ? p1.adHocModifications.length / p1.anomalies.length : 0,
      p2: p2.anomalies.length > 0 ? p2.adHocModifications.length / p2.anomalies.length : 0,
      better: null  // lower is better
    }
  };

  comparison.criteria.adHocRatio.better =
    comparison.criteria.adHocRatio.p1 <= comparison.criteria.adHocRatio.p2 ? prog1Id : prog2Id;

  // Hard core comparison
  const coreOverlap = p1.hardCore.filter(c => p2.hardCore.includes(c));
  comparison.hardCoreComparison.overlap = coreOverlap.length;
  comparison.hardCoreComparison.compatible =
    coreOverlap.length > 0 || (p1.hardCore.length === 0 && p2.hardCore.length === 0);

  // Overall
  const p1Wins = Object.values(comparison.criteria).filter(c => c.better === prog1Id).length;
  const p2Wins = Object.values(comparison.criteria).filter(c => c.better === prog2Id).length;

  if (p1Wins > p2Wins) {
    comparison.betterProgramme = prog1Id;
    comparison.reasoning = 'Better on ' + p1Wins + ' of 3 criteria';
  } else if (p2Wins > p1Wins) {
    comparison.betterProgramme = prog2Id;
    comparison.reasoning = 'Better on ' + p2Wins + ' of 3 criteria';
  } else {
    comparison.betterProgramme = 'tied';
    comparison.reasoning = 'Equal on criteria - context matters';
  }

  state.comparisons.push(comparison);
  saveState();

  return comparison;
}

/**
 * Analyze a paradigm shift (Kuhn-style)
 */
function analyzeParadigmShift(spec) {
  const analysis = {
    description: spec.description || 'Paradigm shift',
    field: spec.field || 'unspecified',

    // Old and new paradigms
    oldParadigm: spec.oldParadigm || null,
    newParadigm: spec.newParadigm || null,

    // Kuhnian analysis
    kuhnianMarkers: {
      crisis: spec.crisis || false,
      anomalies: spec.anomalies || [],
      competingViews: spec.competingViews || false,
      gestaltShift: spec.gestaltShift || false,
      incommensurability: spec.incommensurability || false
    },

    // Changes
    changesIdentified: {
      ontological: spec.ontologyChange || false,
      methodological: spec.methodChange || false,
      problems: spec.problemChange || false,
      standards: spec.standardChange || false,
      terminology: spec.terminologyChange || false
    },

    // Verdict
    isParadigmShift: null,
    kuhnianScore: 0,
    classification: null,

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Calculate Kuhnian score
  const markers = analysis.kuhnianMarkers;
  if (markers.crisis) analysis.kuhnianScore++;
  if (markers.anomalies.length > 2) analysis.kuhnianScore++;
  if (markers.competingViews) analysis.kuhnianScore++;
  if (markers.gestaltShift) analysis.kuhnianScore++;
  if (markers.incommensurability) analysis.kuhnianScore++;

  const changes = analysis.changesIdentified;
  const changeCount = Object.values(changes).filter(Boolean).length;

  // Verdict
  if (analysis.kuhnianScore >= 4 && changeCount >= 3) {
    analysis.isParadigmShift = true;
    analysis.classification = 'FULL PARADIGM SHIFT (Kuhnian revolution)';
  } else if (analysis.kuhnianScore >= 2 || changeCount >= 2) {
    analysis.isParadigmShift = 'partial';
    analysis.classification = 'PARTIAL PARADIGM SHIFT';
  } else {
    analysis.isParadigmShift = false;
    analysis.classification = 'NOT A PARADIGM SHIFT (normal science change)';
  }

  state.changeAnalyses.push(analysis);
  saveState();

  return analysis;
}

/**
 * Get Lakatos methodology summary
 */
function getLakatosSummary() {
  return {
    methodology: 'Methodology of Scientific Research Programmes',

    keyIdeas: [
      'Programmes have hard core (protected) and protective belt (adjustable)',
      'Progressive programmes predict and confirm novel facts',
      'Degenerating programmes only make ad hoc adjustments',
      'Never abandon a programme without a better alternative'
    ],

    comparisonCriteria: [
      'Theoretical progress: novel predictions',
      'Empirical progress: confirmed novel predictions',
      'Not ad hoc: genuine content increase'
    ],

    synthesisOfPopper: {
      agreement: 'Falsifiability matters, bold predictions are good',
      refinement: 'Single refutations do not kill programmes',
      result: 'Sophisticated falsificationism'
    },

    historicalExamples: {
      progressive: 'Newtonian mechanics (predicted Neptune)',
      degenerating: 'Ptolemaic astronomy (epicycles on epicycles)'
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
    '🔄 THEORY CHANGE ENGINE - Lakatos & Paradigm Shifts',
    '═══════════════════════════════════════════════════════════',
    '',
    '── LAKATOS ────────────────────────────────────────────────',
    '   Research Programmes: Hard Core + Protective Belt',
    '   Progressive: Novel predictions confirmed',
    '   Degenerating: Ad hoc adjustments only',
    '',
    '── CHANGE PATTERNS ────────────────────────────────────────',
    '   Revolution: Wholesale replacement',
    '   Evolution: Gradual refinement',
    '   Incorporation: Old becomes special case',
    '   Elimination: Old rejected entirely',
    '',
    '── COMPARISON CRITERIA ────────────────────────────────────',
    '   Empirical adequacy | Scope | Fruitfulness',
    '   Simplicity | Consistency | Unification',
    '',
    '── STATISTICS ─────────────────────────────────────────────',
    '   Programmes: ' + state.programmes.size,
    '   Successions: ' + state.theorySuccessions.length,
    '   Analyses: ' + state.changeAnalyses.length,
    '   Comparisons: ' + state.comparisons.length,
    '',
    '═══════════════════════════════════════════════════════════',
    '*sniff* History judges - pursue progressive programmes.',
    '═══════════════════════════════════════════════════════════'
  ];

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  return {
    programmes: state.programmes.size,
    successions: state.theorySuccessions.length,
    analyses: state.changeAnalyses.length,
    comparisons: state.comparisons.length
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Research Programmes (Lakatos)
  createResearchProgramme,
  evaluateProgressiveness,
  compareProgrammes,
  getLakatosSummary,

  // Theory Change
  recordSuccession,
  analyzeParadigmShift,

  // Theory
  LAKATOS,
  CHANGE_PATTERNS,
  COMPARISON_CRITERIA,

  // Constants
  PHI,
  PHI_INV
};

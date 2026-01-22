/**
 * Practical Reason Engine - Aristotle/Kant
 *
 * "The origin of action is choice, and that of choice is desire
 *  and reasoning with a view to an end."
 * — Aristotle, Nicomachean Ethics
 *
 * "Act only according to that maxim whereby you can will that it
 *  should become a universal law."
 * — Kant, Groundwork
 *
 * Implements:
 * - Practical syllogism (Aristotle)
 * - Categorical vs hypothetical imperatives (Kant)
 * - Means-end reasoning
 * - Akrasia (weakness of will)
 *
 * φ guides rationality assessments.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'practical-reason');

// Aristotle's practical syllogism
const PRACTICAL_SYLLOGISM = {
  name: 'Practical Syllogism',
  structure: {
    majorPremise: 'Universal desire/goal premise (e.g., "I want health")',
    minorPremise: 'Particular belief about means (e.g., "Walking is healthy")',
    conclusion: 'Action (e.g., walking)',
    aristotle: 'The conclusion of a practical syllogism is an action'
  },
  example: {
    major: 'I desire to stay dry',
    minor: 'This umbrella will keep me dry',
    conclusion: 'I take the umbrella'
  }
};

// Kant's imperative types
const IMPERATIVE_TYPES = {
  categorical: {
    name: 'Categorical Imperative',
    description: 'Commands unconditionally, regardless of desires',
    formula: 'Act only on maxims you could will as universal laws',
    source: 'Pure practical reason',
    binds: 'All rational beings',
    kant: 'Duty for duty\'s sake',
    formulations: {
      universal_law: 'Act only according to that maxim whereby you can will that it should become a universal law',
      humanity: 'Treat humanity, in your own person or that of another, always as an end and never merely as a means',
      kingdom_of_ends: 'Act according to maxims of a universally legislating member of a merely possible kingdom of ends'
    }
  },
  hypothetical: {
    name: 'Hypothetical Imperative',
    description: 'Commands conditionally, given certain desires/ends',
    formula: 'If you want X, do Y',
    source: 'Empirical desires',
    binds: 'Only those who have the relevant desire',
    types: {
      technical: 'Rules of skill (if you want Z, do A)',
      pragmatic: 'Counsels of prudence (if you want happiness, do B)'
    }
  }
};

// Means-end reasoning
const MEANS_END_REASONING = {
  name: 'Means-End Reasoning',
  description: 'Reasoning about how to achieve one\'s ends',
  structure: {
    end: 'Goal or desired state',
    means: 'Actions that would bring about the end',
    deliberation: 'Evaluation of available means',
    choice: 'Selection of means to pursue'
  },
  principles: {
    instrumental: 'If you will the end, you must will the necessary means',
    transmission: 'Reasons for ends transmit to reasons for means',
    efficiency: 'Prefer more efficient means (ceteris paribus)'
  },
  aristotle: 'We deliberate not about ends but about means'
};

// Akrasia (weakness of will)
const AKRASIA = {
  name: 'Akrasia (Weakness of Will)',
  description: 'Acting against one\'s own better judgment',
  greek: 'ἀκρασία - lack of command over oneself',
  puzzle: {
    statement: 'How can someone knowingly act against their own best judgment?',
    socrates: 'Impossible - no one errs willingly (akrasia denied)',
    aristotle: 'Possible - practical knowledge can be "bound" by passion',
    davidson: 'Possible - all-things-considered judgment vs unconditional judgment'
  },
  conditions: {
    judgment: 'Agent judges A is better than B, all things considered',
    action: 'Agent intentionally does B instead of A',
    voluntary: 'The action is free, not compelled'
  },
  explanations: {
    socratic: 'Apparent akrasia is really ignorance',
    aristotelian: 'Passion temporarily disables practical knowledge',
    davidsonian: 'Weakness in inferring from judgment to intention',
    partitioned_mind: 'Different mental subsystems in conflict'
  }
};

// Practical rationality norms
const RATIONALITY_NORMS = {
  consistency: {
    name: 'Consistency',
    description: 'Intentions should not contradict each other',
    requirement: 'If intend A and intend B, intend (A and B)'
  },
  means_end: {
    name: 'Means-End Coherence',
    description: 'If you intend E, intend necessary means M',
    requirement: 'Instrumental rationality'
  },
  enkrasia: {
    name: 'Enkrasia',
    description: 'Intend to do what you judge you ought to do',
    requirement: 'Self-governance',
    opposite: 'Akrasia'
  },
  proportionality: {
    name: 'Proportionality',
    description: 'Strength of intention proportional to reasons',
    requirement: 'Reasons-responsiveness'
  }
};

// State
const state = {
  agents: new Map(),
  deliberations: [],
  imperativeTests: [],
  akrasiaAnalyses: [],
  syllogisms: []
};

/**
 * Initialize the practical reason engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.agents) state.agents = new Map(Object.entries(saved.agents));
      if (saved.deliberations) state.deliberations = saved.deliberations;
      if (saved.imperativeTests) state.imperativeTests = saved.imperativeTests;
      if (saved.akrasiaAnalyses) state.akrasiaAnalyses = saved.akrasiaAnalyses;
      if (saved.syllogisms) state.syllogisms = saved.syllogisms;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', deliberations: state.deliberations.length };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    agents: Object.fromEntries(state.agents),
    deliberations: state.deliberations,
    imperativeTests: state.imperativeTests,
    akrasiaAnalyses: state.akrasiaAnalyses,
    syllogisms: state.syllogisms
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Register an agent for practical reasoning
 */
function registerAgent(id, spec = {}) {
  const agent = {
    id,
    name: spec.name || id,

    // Ends
    ends: spec.ends || [],       // What agent wants
    values: spec.values || [],    // What agent values

    // Capacities
    capacities: {
      deliberation: spec.deliberation ?? true,
      selfControl: spec.selfControl ?? true,   // For akrasia
      moralReasoning: spec.moralReasoning ?? true
    },

    registeredAt: Date.now()
  };

  state.agents.set(id, agent);
  saveState();

  return agent;
}

/**
 * Construct practical syllogism (Aristotle)
 */
function constructSyllogism(agentId, spec = {}) {
  const agent = state.agents.get(agentId);

  const syllogism = {
    id: `syl_${Date.now()}`,
    agentId,
    agentName: agent?.name || agentId,

    // Structure
    majorPremise: spec.desire || spec.majorPremise,  // The goal/desire
    minorPremise: spec.belief || spec.minorPremise,  // Belief about means
    conclusion: spec.action || spec.conclusion,       // The action

    // Validity
    valid: null,
    soundness: null,

    // Aristotle's insight
    aristotle: 'The conclusion of a practical syllogism is an action, not a proposition',

    timestamp: Date.now()
  };

  // Check validity
  if (syllogism.majorPremise && syllogism.minorPremise && syllogism.conclusion) {
    // Simplified check: does minor connect major to conclusion?
    syllogism.valid = true;
    syllogism.structure = {
      desire: syllogism.majorPremise,
      means: syllogism.minorPremise,
      result: syllogism.conclusion
    };
  } else {
    syllogism.valid = false;
    syllogism.missing = [];
    if (!syllogism.majorPremise) syllogism.missing.push('major premise (desire)');
    if (!syllogism.minorPremise) syllogism.missing.push('minor premise (belief)');
    if (!syllogism.conclusion) syllogism.missing.push('conclusion (action)');
  }

  state.syllogisms.push(syllogism);
  saveState();

  return syllogism;
}

/**
 * Test imperative (Kant's categorical imperative)
 */
function testCategoricalImperative(maxim, spec = {}) {
  const test = {
    id: `ci_${Date.now()}`,
    maxim,

    // Test universalizability
    universalLaw: {
      formulation: IMPERATIVE_TYPES.categorical.formulations.universal_law,
      question: `Can everyone act on "${maxim}" without contradiction?`,
      universalizable: null,
      contradiction: null
    },

    // Test humanity formula
    humanityFormula: {
      formulation: IMPERATIVE_TYPES.categorical.formulations.humanity,
      question: `Does "${maxim}" treat humanity as an end?`,
      respectsHumanity: null
    },

    // Overall verdict
    permissible: null,

    // Kant's insight
    kantInsight: 'Morality is about what reason alone commands',

    timestamp: Date.now()
  };

  // Simple universalizability check
  const lower = maxim.toLowerCase();

  // Check for obvious contradictions
  if (lower.includes('lie') || lower.includes('deceive') || lower.includes('break promise')) {
    test.universalLaw.universalizable = false;
    test.universalLaw.contradiction = 'conceptual';
    test.universalLaw.explanation = 'If universalized, the practice itself would be undermined';
  } else if (lower.includes('kill') || lower.includes('harm') || lower.includes('steal')) {
    test.humanityFormula.respectsHumanity = false;
    test.humanityFormula.explanation = 'Treats others merely as means';
  } else if (lower.includes('help') || lower.includes('respect') || lower.includes('honest')) {
    test.universalLaw.universalizable = true;
    test.humanityFormula.respectsHumanity = true;
  } else {
    // Default: requires more analysis
    test.universalLaw.universalizable = spec.universalizable ?? null;
    test.humanityFormula.respectsHumanity = spec.respectsHumanity ?? null;
  }

  // Overall verdict
  if (test.universalLaw.universalizable === false ||
      test.humanityFormula.respectsHumanity === false) {
    test.permissible = false;
    test.verdict = 'Impermissible - fails categorical imperative';
  } else if (test.universalLaw.universalizable === true &&
             test.humanityFormula.respectsHumanity === true) {
    test.permissible = true;
    test.verdict = 'Permissible - passes categorical imperative';
  } else {
    test.permissible = 'undetermined';
    test.verdict = 'Requires further analysis';
  }

  state.imperativeTests.push(test);
  saveState();

  return test;
}

/**
 * Classify imperative type
 */
function classifyImperative(imperative) {
  const classification = {
    imperative,

    type: null,
    reasoning: null,

    // Tests
    isCategorical: null,
    isHypothetical: null,

    timestamp: Date.now()
  };

  const lower = imperative.toLowerCase();

  // Check for hypothetical structure
  if (lower.includes('if you want') || lower.includes('in order to') ||
      lower.startsWith('to ') || lower.includes('should') && lower.includes('want')) {
    classification.type = 'hypothetical';
    classification.isHypothetical = true;
    classification.isCategorical = false;
    classification.reasoning = 'Commands conditionally, given certain desires';

    // Subtype
    if (lower.includes('happiness') || lower.includes('well-being') || lower.includes('flourish')) {
      classification.subtype = 'pragmatic';
      classification.note = 'Counsel of prudence';
    } else {
      classification.subtype = 'technical';
      classification.note = 'Rule of skill';
    }
  } else if (lower.includes('must') || lower.includes('ought') ||
             lower.includes('duty') || lower.includes('always')) {
    classification.type = 'categorical';
    classification.isCategorical = true;
    classification.isHypothetical = false;
    classification.reasoning = 'Commands unconditionally, regardless of desires';
    classification.kant = 'Binds all rational beings as such';
  } else {
    classification.type = 'indeterminate';
    classification.reasoning = 'Could be either depending on context';
  }

  return classification;
}

/**
 * Analyze means-end deliberation
 */
function deliberate(agentId, spec = {}) {
  const agent = state.agents.get(agentId);
  if (!agent) {
    return { error: 'Agent not found' };
  }

  const deliberation = {
    id: `delib_${Date.now()}`,
    agentId,
    agentName: agent.name,

    // The end
    end: spec.end || spec.goal,

    // Available means
    availableMeans: spec.means || [],

    // Evaluation
    evaluation: [],

    // Choice
    chosenMeans: null,
    reasoning: null,

    // Aristotle's constraint
    aristotle: 'We deliberate about means, not ends',

    timestamp: Date.now()
  };

  // Evaluate each means
  for (const means of deliberation.availableMeans) {
    const eval_ = {
      means: means.description || means,
      effectiveness: means.effectiveness ?? PHI_INV_2,
      cost: means.cost ?? PHI_INV_2,
      morally_permissible: means.permissible ?? true,
      score: 0
    };

    // Calculate score (effectiveness - cost, if permissible)
    if (eval_.morally_permissible) {
      eval_.score = eval_.effectiveness - (eval_.cost * PHI_INV);
    } else {
      eval_.score = -1;  // Impermissible means are worst
    }

    deliberation.evaluation.push(eval_);
  }

  // Choose best means
  if (deliberation.evaluation.length > 0) {
    const best = deliberation.evaluation.reduce((a, b) => a.score > b.score ? a : b);

    if (best.score >= 0) {
      deliberation.chosenMeans = best.means;
      deliberation.reasoning = `Chose "${best.means}" - highest score (${best.score.toFixed(2)})`;
    } else {
      deliberation.chosenMeans = null;
      deliberation.reasoning = 'No permissible means available';
    }
  }

  state.deliberations.push(deliberation);
  saveState();

  return deliberation;
}

/**
 * Analyze akrasia (weakness of will)
 */
function analyzeAkrasia(spec = {}) {
  const analysis = {
    id: `akrasia_${Date.now()}`,

    // The case
    judgment: spec.judgment,      // What agent judged best
    action: spec.action,          // What agent actually did
    voluntary: spec.voluntary ?? true,

    // Is it akrasia?
    isAkrasia: null,
    explanation: null,

    // Theoretical perspectives
    perspectives: {
      socratic: {
        verdict: 'Not genuine akrasia',
        explanation: 'Agent must not have truly known A was better (ignorance)'
      },
      aristotelian: {
        verdict: 'Genuine akrasia',
        explanation: 'Passion temporarily bound/disabled practical knowledge'
      },
      davidsonian: {
        verdict: 'Genuine akrasia',
        explanation: 'All-things-considered judgment ≠ unconditional intention'
      }
    },

    timestamp: Date.now()
  };

  // Check akrasia conditions
  if (spec.judgment && spec.action && spec.judgment !== spec.action && spec.voluntary) {
    analysis.isAkrasia = true;
    analysis.explanation = `Agent judged "${spec.judgment}" best but did "${spec.action}" instead`;

    // Davidson's analysis
    analysis.davidsonAnalysis = {
      allThingsConsidered: spec.judgment,
      unconditionalIntention: spec.action,
      gap: 'Weakness in practical reasoning - failed to derive intention from judgment'
    };
  } else if (!spec.voluntary) {
    analysis.isAkrasia = false;
    analysis.explanation = 'Action was not voluntary - not akrasia but compulsion';
  } else if (spec.judgment === spec.action) {
    analysis.isAkrasia = false;
    analysis.explanation = 'Agent acted on their judgment - no akrasia';
  }

  state.akrasiaAnalyses.push(analysis);
  saveState();

  return analysis;
}

/**
 * Check practical rationality
 */
function checkRationality(agentId, intentions) {
  const agent = state.agents.get(agentId);

  const check = {
    agentId,
    agentName: agent?.name || agentId,
    intentions,

    // Norms
    norms: {
      consistency: {
        name: 'Consistency',
        met: true,
        violations: []
      },
      meansEnd: {
        name: 'Means-End Coherence',
        met: true,
        violations: []
      },
      enkrasia: {
        name: 'Enkrasia',
        met: true,
        violations: []
      }
    },

    overall: true,

    timestamp: Date.now()
  };

  // Check consistency (simplified)
  for (let i = 0; i < intentions.length; i++) {
    for (let j = i + 1; j < intentions.length; j++) {
      const int1 = intentions[i].toLowerCase();
      const int2 = intentions[j].toLowerCase();

      if ((int1.includes('not') && int2 === int1.replace('not ', '')) ||
          (int2.includes('not') && int1 === int2.replace('not ', ''))) {
        check.norms.consistency.met = false;
        check.norms.consistency.violations.push([intentions[i], intentions[j]]);
        check.overall = false;
      }
    }
  }

  return check;
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '🎯 PRACTICAL REASON ENGINE - "What ought I to do?"',
    '═══════════════════════════════════════════════════════════',
    '',
    '── ARISTOTLE\'S PRACTICAL SYLLOGISM ────────────────────────'
  ];

  lines.push('   Major: Desire/Goal premise');
  lines.push('   Minor: Belief about means');
  lines.push('   Conclusion: ACTION (not proposition)');

  lines.push('');
  lines.push('── KANT\'S IMPERATIVES ─────────────────────────────────────');
  lines.push('   Categorical: Unconditional ("Duty for duty\'s sake")');
  lines.push('   Hypothetical: Conditional ("If you want X, do Y")');

  lines.push('');
  lines.push('── AKRASIA ────────────────────────────────────────────────');
  lines.push('   Definition: Acting against one\'s better judgment');
  lines.push('   Socrates: Impossible (ignorance)');
  lines.push('   Aristotle: Possible (passion binds knowledge)');

  lines.push('');
  lines.push('── STATISTICS ─────────────────────────────────────────────');
  lines.push(`   Syllogisms: ${state.syllogisms.length}`);
  lines.push(`   Deliberations: ${state.deliberations.length}`);
  lines.push(`   Imperative tests: ${state.imperativeTests.length}`);
  lines.push(`   Akrasia analyses: ${state.akrasiaAnalyses.length}`);

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('*sniff* "We deliberate about means, not ends." (Aristotle)');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  const permissible = state.imperativeTests.filter(t => t.permissible === true).length;
  const impermissible = state.imperativeTests.filter(t => t.permissible === false).length;
  const akrasiaCount = state.akrasiaAnalyses.filter(a => a.isAkrasia).length;

  return {
    agents: state.agents.size,
    syllogisms: state.syllogisms.length,
    deliberations: state.deliberations.length,
    imperativeTests: state.imperativeTests.length,
    permissible,
    impermissible,
    akrasiaAnalyses: state.akrasiaAnalyses.length,
    akrasiaCases: akrasiaCount
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Agents
  registerAgent,

  // Practical Syllogism (Aristotle)
  constructSyllogism,
  PRACTICAL_SYLLOGISM,

  // Imperatives (Kant)
  testCategoricalImperative,
  classifyImperative,
  IMPERATIVE_TYPES,

  // Means-End
  deliberate,
  MEANS_END_REASONING,

  // Akrasia
  analyzeAkrasia,
  AKRASIA,

  // Rationality
  checkRationality,
  RATIONALITY_NORMS,

  // Constants
  PHI,
  PHI_INV
};

/**
 * Free Will Engine - The Great Debate
 *
 * "Man can do what he wills but he cannot will what he wills."
 * — Schopenhauer
 *
 * "We are condemned to be free."
 * — Sartre
 *
 * Implements:
 * - Determinism, libertarianism, compatibilism
 * - Moral responsibility conditions
 * - Frankfurt cases
 * - Alternative possibilities
 *
 * φ guides uncertainty about free will claims.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'free-will');

// Major positions
const FREE_WILL_POSITIONS = {
  hard_determinism: {
    name: 'Hard Determinism',
    thesis: 'Determinism is true, therefore free will and moral responsibility are illusions',
    determinism: true,
    freeWill: false,
    moralResponsibility: false,
    proponents: ['Spinoza', 'Holbach', 'Pereboom'],
    quote: 'If we could know all causes, we could predict all effects'
  },
  libertarianism: {
    name: 'Libertarianism',
    thesis: 'Free will exists and is incompatible with determinism; determinism is false',
    determinism: false,
    freeWill: true,
    moralResponsibility: true,
    proponents: ['Kane', 'Chisholm', 'O\'Connor'],
    requires: 'Agent causation or indeterministic causation',
    quote: 'We are ultimate sources of our actions'
  },
  compatibilism: {
    name: 'Compatibilism',
    thesis: 'Free will is compatible with determinism; both can be true',
    determinism: 'possibly',
    freeWill: true,
    moralResponsibility: true,
    proponents: ['Hume', 'Frankfurt', 'Dennett', 'Fischer'],
    key: 'Redefines free will in terms of responsiveness to reasons',
    quote: 'Freedom is acting on your own desires without external constraint'
  },
  hard_incompatibilism: {
    name: 'Hard Incompatibilism',
    thesis: 'Free will is impossible whether determinism is true or not',
    determinism: 'irrelevant',
    freeWill: false,
    moralResponsibility: false,
    proponents: ['Pereboom', 'Strawson'],
    reasoning: 'Luck swallows everything: determined = not responsible, random = not responsible'
  }
};

// Determinism types
const DETERMINISM_TYPES = {
  causal: {
    name: 'Causal Determinism',
    claim: 'Every event is necessitated by prior causes plus laws of nature',
    laplace: 'A demon knowing all facts could predict all future events'
  },
  logical: {
    name: 'Logical Determinism',
    claim: 'Truth of future-tense propositions determines what will happen',
    example: 'If "There will be a sea battle tomorrow" is true now, the battle is inevitable'
  },
  theological: {
    name: 'Theological Determinism',
    claim: 'God\'s foreknowledge or predestination determines all events',
    dilemma: 'If God knows what I\'ll do, can I do otherwise?'
  },
  psychological: {
    name: 'Psychological Determinism',
    claim: 'All choices are determined by psychological states',
    note: 'Strongest desires always win'
  }
};

// Frankfurt cases
const FRANKFURT_CASES = {
  original: {
    name: 'Original Frankfurt Case',
    scenario: {
      setup: 'Black wants Jones to vote for Democrat. Black has a device to ensure this.',
      condition: 'If Jones shows ANY sign of voting Republican, Black will intervene.',
      actual: 'Jones decides on his own to vote Democrat. Black never intervenes.',
      question: 'Is Jones morally responsible?'
    },
    analysis: {
      alternative_possibilities: 'Jones couldn\'t have done otherwise (Black would have intervened)',
      moral_responsibility: 'Intuitively, Jones IS responsible (he chose freely)',
      conclusion: 'Moral responsibility doesn\'t require alternative possibilities (PAP is false)'
    },
    implications: {
      for_compatibilism: 'Supports compatibilism - what matters is actual sequence',
      against_libertarianism: 'Challenges the requirement of alternative possibilities'
    }
  },
  flicker: {
    name: 'Flicker Strategy',
    objection: 'Jones had some alternative: showing the sign or not',
    response: 'The flicker isn\'t a robust alternative - not enough for responsibility'
  },
  prior_sign: {
    name: 'Prior Sign Problem',
    objection: 'How can Black know what Jones will do before Jones decides?',
    dilemma: 'If determinism: no libertarian free will anyway. If indeterminism: Black can\'t reliably predict.'
  }
};

// Principle of Alternative Possibilities
const PAP = {
  name: 'Principle of Alternative Possibilities',
  statement: 'A person is morally responsible for an action only if they could have done otherwise',
  status: 'Contested by Frankfurt cases',
  versions: {
    strong: 'Must have been able to do OTHERWISE',
    weak: 'Must have been able to DO otherwise (robust alternative)',
    leeway: 'Free will requires leeway/elbow room in the universe'
  }
};

// Moral responsibility conditions
const RESPONSIBILITY_CONDITIONS = {
  control: {
    name: 'Control Condition',
    description: 'Agent must have control over the action',
    types: {
      regulative: 'Ability to do otherwise (challenged by Frankfurt)',
      guidance: 'Guiding action through reasons-responsiveness'
    }
  },
  epistemic: {
    name: 'Epistemic Condition',
    description: 'Agent must know (or should know) what they\'re doing',
    elements: ['Awareness of action', 'Awareness of consequences', 'Awareness of moral significance']
  },
  freedom: {
    name: 'Freedom Condition',
    description: 'Action must be free from external compulsion',
    types: {
      external: 'No physical coercion',
      internal: 'No irresistible psychological compulsion'
    }
  },
  sourcehood: {
    name: 'Sourcehood Condition',
    description: 'Agent must be the source/origin of the action',
    fischer: 'Reasons-responsiveness sufficient for sourcehood'
  }
};

// State
const state = {
  agents: new Map(),
  actions: new Map(),
  responsibilityAssessments: [],
  frankfurtAnalyses: [],
  positionAssessments: []
};

/**
 * Initialize the free will engine
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
      if (saved.actions) state.actions = new Map(Object.entries(saved.actions));
      if (saved.responsibilityAssessments) state.responsibilityAssessments = saved.responsibilityAssessments;
      if (saved.frankfurtAnalyses) state.frankfurtAnalyses = saved.frankfurtAnalyses;
      if (saved.positionAssessments) state.positionAssessments = saved.positionAssessments;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', assessments: state.responsibilityAssessments.length };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    agents: Object.fromEntries(state.agents),
    actions: Object.fromEntries(state.actions),
    responsibilityAssessments: state.responsibilityAssessments,
    frankfurtAnalyses: state.frankfurtAnalyses,
    positionAssessments: state.positionAssessments
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Register an agent for free will analysis
 */
function registerAgent(id, spec = {}) {
  const agent = {
    id,
    name: spec.name || id,
    type: spec.type || 'human',

    // Capacities relevant to free will
    capacities: {
      deliberation: spec.deliberation ?? true,
      reasonsResponsive: spec.reasonsResponsive ?? true,
      selfControl: spec.selfControl ?? true,
      alternativePossibilities: spec.alternativePossibilities ?? 'unknown'
    },

    registeredAt: Date.now()
  };

  state.agents.set(id, agent);
  saveState();

  return agent;
}

/**
 * Register an action for responsibility assessment
 */
function registerAction(id, spec = {}) {
  const action = {
    id,
    description: spec.description || id,
    agent: spec.agent,

    // Circumstances
    couldHaveDoneOtherwise: spec.couldHaveDoneOtherwise ?? 'unknown',
    determined: spec.determined ?? 'unknown',
    external_compulsion: spec.externalCompulsion ?? false,
    internal_compulsion: spec.internalCompulsion ?? false,

    // Knowledge
    knewAction: spec.knewAction ?? true,
    knewConsequences: spec.knewConsequences ?? true,
    knewMoralSignificance: spec.knewMoralSignificance ?? true,

    timestamp: Date.now()
  };

  state.actions.set(id, action);
  saveState();

  return action;
}

/**
 * Assess moral responsibility
 */
function assessResponsibility(actionId) {
  const action = state.actions.get(actionId);
  if (!action) {
    return { error: 'Action not found' };
  }

  const agent = state.agents.get(action.agent);

  const assessment = {
    id: `resp_${Date.now()}`,
    actionId,
    action: action.description,
    agent: action.agent,

    // Condition checks
    conditions: {
      control: {
        name: 'Control',
        met: !action.external_compulsion && !action.internal_compulsion,
        details: action.external_compulsion ? 'External compulsion present' :
          action.internal_compulsion ? 'Internal compulsion present' : 'No compulsion'
      },
      epistemic: {
        name: 'Epistemic',
        met: action.knewAction && action.knewConsequences,
        details: !action.knewAction ? 'Didn\'t know action' :
          !action.knewConsequences ? 'Didn\'t know consequences' : 'Full awareness'
      },
      freedom: {
        name: 'Freedom',
        met: !action.external_compulsion,
        details: action.external_compulsion ? 'Externally coerced' : 'Free from external constraint'
      },
      reasonsResponsive: {
        name: 'Reasons-Responsive',
        met: agent?.capacities?.reasonsResponsive ?? true,
        details: 'Agent responds to reasons (Fischer)'
      }
    },

    // Alternative possibilities (PAP)
    alternativePossibilities: {
      available: action.couldHaveDoneOtherwise,
      relevance: 'Contested by Frankfurt cases'
    },

    // Verdicts by theory
    verdictByTheory: {},

    // Overall
    responsible: false,
    confidence: PHI_INV_2,

    timestamp: Date.now()
  };

  // Check all conditions
  const conditionsMet = Object.values(assessment.conditions).every(c => c.met);

  // Verdicts by theory
  assessment.verdictByTheory = {
    compatibilist: {
      verdict: conditionsMet ? 'responsible' : 'not_responsible',
      reasoning: conditionsMet ?
        'Agent met control and epistemic conditions (PAP not required)' :
        'Failed control or epistemic conditions'
    },
    libertarian: {
      verdict: conditionsMet && action.couldHaveDoneOtherwise === true ? 'responsible' : 'not_responsible',
      reasoning: action.couldHaveDoneOtherwise === true ?
        'Agent had alternative possibilities' :
        'No alternative possibilities - no libertarian freedom'
    },
    hard_determinist: {
      verdict: 'not_responsible',
      reasoning: 'If determinism, no one is ever truly responsible'
    }
  };

  // Overall verdict (compatibilist-leaning)
  assessment.responsible = conditionsMet;
  assessment.confidence = Math.min(
    conditionsMet ? PHI_INV : PHI_INV_3,
    PHI_INV
  );

  state.responsibilityAssessments.push(assessment);
  saveState();

  return assessment;
}

/**
 * Apply Frankfurt case analysis
 */
function applyFrankfurtCase(actionId, spec = {}) {
  const action = state.actions.get(actionId);
  if (!action) {
    return { error: 'Action not found' };
  }

  const analysis = {
    id: `frankfurt_${Date.now()}`,
    actionId,
    action: action.description,

    // Frankfurt case structure
    structure: {
      counterfactualIntervenor: spec.intervener || 'hypothetical Black',
      wouldHaveIntervened: spec.wouldHaveIntervened ?? true,
      actuallyIntervened: spec.actuallyIntervened ?? false,
      agentActedOnOwn: !spec.actuallyIntervened
    },

    // Analysis
    analysis: {
      couldHaveDoneOtherwise: false,  // Intervener would have prevented
      responsibleAnyway: spec.actuallyIntervened === false,  // If acted on own, still responsible
      papRefuted: true
    },

    // Implications
    implications: {
      forPAP: 'PAP appears false - responsibility without alternative possibilities',
      forCompatibilism: 'Supports compatibilism - actual sequence matters',
      forLibertarianism: 'Challenges requirement of alternative possibilities'
    },

    // Frankfurt's insight
    frankfurtInsight: 'What matters for responsibility is the actual causal history, not alternative possibilities',

    timestamp: Date.now()
  };

  // Verdict
  if (analysis.structure.agentActedOnOwn) {
    analysis.verdict = 'Agent is responsible despite lacking alternatives';
  } else {
    analysis.verdict = 'Agent not responsible - intervener actually controlled action';
  }

  state.frankfurtAnalyses.push(analysis);
  saveState();

  return analysis;
}

/**
 * Evaluate a free will position
 */
function evaluatePosition(positionId) {
  const position = FREE_WILL_POSITIONS[positionId];
  if (!position) {
    return { error: 'Position not found', available: Object.keys(FREE_WILL_POSITIONS) };
  }

  const evaluation = {
    positionId,
    position: position.name,
    thesis: position.thesis,

    // Commitments
    commitments: {
      determinism: position.determinism,
      freeWill: position.freeWill,
      moralResponsibility: position.moralResponsibility
    },

    // Arguments
    strengths: [],
    weaknesses: [],

    // CYNIC's assessment
    assessment: {
      plausibility: PHI_INV_2,  // All positions have serious arguments
      confidence: PHI_INV_3,    // Free will is deeply uncertain
      note: null
    },

    timestamp: Date.now()
  };

  // Position-specific analysis
  switch (positionId) {
    case 'hard_determinism':
      evaluation.strengths = ['Scientific worldview', 'Explanatory parsimony'];
      evaluation.weaknesses = ['Seems to undermine all responsibility', 'Self-undermining?'];
      evaluation.assessment.note = 'If true, even believing it is determined';
      break;

    case 'libertarianism':
      evaluation.strengths = ['Preserves robust free will', 'Matches phenomenology'];
      evaluation.weaknesses = ['Requires indeterminism or agent causation', 'Luck objection'];
      evaluation.assessment.note = 'How does indeterminism help? Random ≠ free';
      break;

    case 'compatibilism':
      evaluation.strengths = ['Preserves responsibility', 'Compatible with science'];
      evaluation.weaknesses = ['Is it "real" free will?', 'Manipulation arguments'];
      evaluation.assessment.note = 'Most popular among philosophers';
      evaluation.assessment.plausibility = PHI_INV;  // Slightly higher
      break;

    case 'hard_incompatibilism':
      evaluation.strengths = ['Takes luck seriously', 'Consistent'];
      evaluation.weaknesses = ['Undermines moral practice', 'Counterintuitive'];
      evaluation.assessment.note = 'Luck swallows everything';
      break;
  }

  state.positionAssessments.push(evaluation);
  saveState();

  return evaluation;
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '⚖️ FREE WILL ENGINE - "Condemned to be free"',
    '═══════════════════════════════════════════════════════════',
    '',
    '── POSITIONS ──────────────────────────────────────────────'
  ];

  for (const [key, pos] of Object.entries(FREE_WILL_POSITIONS)) {
    const fw = pos.freeWill ? '✓' : '✗';
    const mr = pos.moralResponsibility ? '✓' : '✗';
    lines.push(`   ${pos.name}: FW=${fw} MR=${mr}`);
  }

  lines.push('');
  lines.push('── FRANKFURT CASES ────────────────────────────────────────');
  lines.push('   PAP: "Responsible only if could have done otherwise"');
  lines.push('   Frankfurt: PAP is FALSE - actual sequence matters');

  lines.push('');
  lines.push('── RESPONSIBILITY CONDITIONS ──────────────────────────────');
  for (const [key, cond] of Object.entries(RESPONSIBILITY_CONDITIONS)) {
    lines.push(`   ${cond.name}: ${cond.description}`);
  }

  lines.push('');
  lines.push('── STATISTICS ─────────────────────────────────────────────');
  lines.push(`   Responsibility assessments: ${state.responsibilityAssessments.length}`);
  lines.push(`   Frankfurt analyses: ${state.frankfurtAnalyses.length}`);
  lines.push(`   Position evaluations: ${state.positionAssessments.length}`);

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('*sniff* "We cannot will what we will." (Schopenhauer)');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  const responsibleCount = state.responsibilityAssessments.filter(a => a.responsible).length;

  return {
    agents: state.agents.size,
    actions: state.actions.size,
    responsibilityAssessments: state.responsibilityAssessments.length,
    foundResponsible: responsibleCount,
    foundNotResponsible: state.responsibilityAssessments.length - responsibleCount,
    frankfurtAnalyses: state.frankfurtAnalyses.length,
    positionEvaluations: state.positionAssessments.length
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Agents/Actions
  registerAgent,
  registerAction,

  // Responsibility
  assessResponsibility,
  RESPONSIBILITY_CONDITIONS,

  // Frankfurt
  applyFrankfurtCase,
  FRANKFURT_CASES,
  PAP,

  // Positions
  evaluatePosition,
  FREE_WILL_POSITIONS,
  DETERMINISM_TYPES,

  // Constants
  PHI,
  PHI_INV
};

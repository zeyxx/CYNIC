/**
 * Social Contract Engine - Hobbes/Locke/Rousseau
 *
 * "The condition of man... is a condition of war of everyone against everyone."
 * — Thomas Hobbes
 *
 * "The state of nature has a law of nature to govern it."
 * — John Locke
 *
 * "Man is born free, and everywhere he is in chains."
 * — Jean-Jacques Rousseau
 *
 * Implements:
 * - State of nature theories (Hobbes, Locke, Rousseau)
 * - Contract formation and validity
 * - Political legitimacy assessment
 *
 * φ guides confidence in contract assessments.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'social-contract');

// Hobbes's Political Philosophy
const HOBBES = {
  stateOfNature: {
    name: 'State of Nature (Hobbes)',
    description: 'Condition without political authority',
    characteristics: [
      'War of all against all (bellum omnium contra omnes)',
      'Life is solitary, poor, nasty, brutish, and short',
      'No industry, culture, or society',
      'Equality of vulnerability (anyone can kill anyone)',
      'Scarcity and competition'
    ],
    lawsOfNature: [
      'Seek peace when possible',
      'Be willing to give up some rights for peace',
      'Keep covenants made'
    ],
    problem: 'Without enforcement, no one will unilaterally disarm',
    quote: 'The life of man: solitary, poor, nasty, brutish, and short'
  },
  sovereign: {
    name: 'Leviathan (Sovereign)',
    description: 'Absolute authority to enforce peace',
    powers: [
      'Absolute and undivided',
      'Cannot be resisted or judged',
      'Defines justice (no injustice before law)',
      'Controls religion and opinion'
    ],
    justification: 'Necessary to escape state of nature',
    formation: 'Mutual covenant: each authorizes sovereign',
    limits: 'Only if sovereign fails to protect life',
    quote: 'Covenants without the sword are but words'
  },
  contract: {
    type: 'Authorization',
    parties: 'Subjects covenant with each other (not with sovereign)',
    content: 'Authorize sovereign to act for all',
    irrevocable: true,
    exception: 'Right to self-preservation cannot be given up'
  }
};

// Locke's Political Philosophy
const LOCKE = {
  stateOfNature: {
    name: 'State of Nature (Locke)',
    description: 'Pre-political but not pre-moral condition',
    characteristics: [
      'Governed by natural law (reason)',
      'Natural equality and freedom',
      'Property exists (through labor)',
      'Generally peaceful but inconvenient',
      'No common judge to settle disputes'
    ],
    naturalLaw: {
      source: 'Reason/God',
      content: 'Preserve mankind, do not harm others',
      enforceability: 'Each person has right to punish violations'
    },
    problems: [
      'No settled, known law',
      'No impartial judge',
      'No power to execute judgment'
    ],
    quote: 'A state of perfect freedom... within the bounds of the law of nature'
  },
  naturalRights: {
    name: 'Natural Rights',
    rights: ['Life', 'Liberty', 'Property (Estate)'],
    origin: 'Given by God/Nature, not by government',
    inalienable: 'Cannot be legitimately taken without consent',
    property: {
      origin: 'Labor mixed with nature',
      limitation: 'Enough and as good left for others (initially)',
      money: 'Allows unlimited accumulation (consent to inequality)'
    }
  },
  government: {
    purpose: 'Preserve natural rights (life, liberty, property)',
    basis: 'Consent of the governed',
    limits: 'Cannot take property without consent (taxation requires representation)',
    separation: 'Legislative and executive powers should be separate',
    revolution: {
      when: 'Government violates trust by attacking rights',
      who: 'People retain ultimate sovereignty',
      nature: 'Appeal to heaven - resistance justified'
    },
    quote: 'Government has no other end but the preservation of property'
  },
  contract: {
    type: 'Trust/Consent',
    parties: 'People and government (government is trustee)',
    content: 'Government protects rights; people obey laws',
    revocable: true,
    condition: 'Only while government fulfills trust'
  }
};

// Rousseau's Political Philosophy
const ROUSSEAU = {
  stateOfNature: {
    name: 'State of Nature (Rousseau)',
    description: 'Original condition of humanity',
    characteristics: [
      'Solitary, peaceful, and free',
      'Noble savage - not yet corrupted',
      'Self-sufficient, few needs',
      'Natural compassion (pitié)',
      'No society, language, or reason yet'
    ],
    corruption: {
      cause: 'Development of society and property',
      mechanism: 'Comparison, competition, inequality',
      result: 'Amour-propre (self-love through others\' eyes)'
    },
    quote: 'Man is born free, and everywhere he is in chains'
  },
  generalWill: {
    name: 'General Will (Volonté Générale)',
    description: 'The will of the body politic as a whole',
    versus: {
      willOfAll: 'Mere aggregation of private interests',
      privateWill: 'Individual self-interest'
    },
    characteristics: [
      'Always tends toward common good',
      'Cannot be represented',
      'Inalienable and indivisible',
      'Sovereign expresses general will'
    ],
    discovery: 'Through deliberation, not voting alone',
    paradox: 'Can be "forced to be free" if one resists general will',
    quote: 'The general will is always right'
  },
  socialContract: {
    purpose: 'Form a legitimate political association',
    formula: 'Total alienation of each to all',
    result: {
      lose: 'Natural liberty (limited by own strength)',
      gain: 'Civil liberty (limited by general will) + moral liberty'
    },
    sovereign: 'The people themselves (direct democracy)',
    government: 'Executive that implements general will (can be replaced)'
  },
  freedom: {
    natural: 'Pre-social, limited by strength',
    civil: 'In society, limited by general will',
    moral: 'Obedience to law one has prescribed to oneself',
    quote: 'Obedience to a law which we prescribe to ourselves is liberty'
  },
  contract: {
    type: 'Total alienation',
    parties: 'Each individual with the whole community',
    content: 'Give up natural freedom, gain civil and moral freedom',
    revocable: 'The sovereign (people) can change anything',
    condition: 'Participation in general will'
  }
};

// Contract Validity Criteria
const CONTRACT_VALIDITY = {
  consent: {
    express: 'Explicit agreement (oath, signature)',
    tacit: 'Implied by residence, acceptance of benefits',
    hypothetical: 'Would consent under ideal conditions (Rawls)',
    problem: 'Does tacit consent bind? (Hume\'s critique)'
  },
  mutuality: {
    requirement: 'Both parties must benefit/be bound',
    problem: 'Did the powerless really have a choice?'
  },
  fairConditions: {
    requirement: 'Contract made under fair conditions',
    noCoercion: 'No force or threat',
    noDeception: 'Full information',
    equalPower: 'Roughly equal bargaining position (problematic)'
  },
  content: {
    requirement: 'Terms must be acceptable',
    noImmoral: 'Cannot contract for immoral acts',
    noInalienable: 'Cannot give up inalienable rights'
  }
};

// State
const state = {
  contracts: new Map(),
  legitimacyEvaluations: [],
  stateOfNatureAnalyses: [],
  revolutions: []
};

/**
 * Initialize the social contract engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.contracts) state.contracts = new Map(Object.entries(saved.contracts));
      if (saved.legitimacyEvaluations) state.legitimacyEvaluations = saved.legitimacyEvaluations;
      if (saved.stateOfNatureAnalyses) state.stateOfNatureAnalyses = saved.stateOfNatureAnalyses;
      if (saved.revolutions) state.revolutions = saved.revolutions;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', contracts: state.contracts.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    contracts: Object.fromEntries(state.contracts),
    legitimacyEvaluations: state.legitimacyEvaluations,
    stateOfNatureAnalyses: state.stateOfNatureAnalyses,
    revolutions: state.revolutions
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Analyze a state of nature scenario
 */
function analyzeStateOfNature(scenario) {
  const analysis = {
    scenario: scenario.description || 'Generic state of nature',

    hobbesianAnalysis: {
      framework: 'Hobbes',
      characteristics: [],
      warLikelihood: null,
      sovereignNeeded: null
    },

    lockeanAnalysis: {
      framework: 'Locke',
      characteristics: [],
      naturalLawPresent: null,
      inconveniences: []
    },

    rousseauianAnalysis: {
      framework: 'Rousseau',
      characteristics: [],
      corruptionLevel: null,
      generalWillPossible: null
    },

    timestamp: Date.now()
  };

  // Hobbesian analysis
  const hobbesFactors = {
    scarcity: scenario.scarcity || false,
    equalVulnerability: scenario.equalPower || true,
    noEnforcement: scenario.noAuthority || true,
    competition: scenario.competition || false
  };

  analysis.hobbesianAnalysis.characteristics = Object.entries(hobbesFactors)
    .filter(([_, v]) => v)
    .map(([k]) => k);

  analysis.hobbesianAnalysis.warLikelihood =
    Object.values(hobbesFactors).filter(Boolean).length >= 3
      ? 'HIGH - conditions for war of all against all'
      : 'MODERATE - some stability possible';

  analysis.hobbesianAnalysis.sovereignNeeded =
    analysis.hobbesianAnalysis.warLikelihood.startsWith('HIGH');

  // Lockean analysis
  analysis.lockeanAnalysis.naturalLawPresent = scenario.moralNorms || false;
  analysis.lockeanAnalysis.inconveniences = [];

  if (!scenario.settledLaw) analysis.lockeanAnalysis.inconveniences.push('No settled law');
  if (!scenario.impartialJudge) analysis.lockeanAnalysis.inconveniences.push('No impartial judge');
  if (!scenario.enforcement) analysis.lockeanAnalysis.inconveniences.push('No enforcement power');

  analysis.lockeanAnalysis.characteristics =
    analysis.lockeanAnalysis.naturalLawPresent
      ? ['Some moral order', 'Property possible']
      : ['No natural law', 'Greater disorder'];

  // Rousseauian analysis
  analysis.rousseauianAnalysis.corruptionLevel =
    scenario.inequality ? 'HIGH' : scenario.society ? 'MODERATE' : 'LOW (pristine)';

  analysis.rousseauianAnalysis.generalWillPossible =
    !scenario.extremeInequality && scenario.commonIdentity;

  analysis.rousseauianAnalysis.characteristics = [
    scenario.society ? 'Social corruption present' : 'Pre-social innocence',
    scenario.inequality ? 'Inequality has developed' : 'Natural equality'
  ];

  state.stateOfNatureAnalyses.push(analysis);
  saveState();

  return analysis;
}

/**
 * Create a social contract
 */
function createContract(id, spec = {}) {
  const contract = {
    id,
    description: spec.description || id,

    // Parties
    parties: spec.parties || ['citizens', 'government'],

    // Type (based on philosopher)
    type: spec.type || 'unknown',
    philosopher: spec.philosopher || null,

    // Content
    termsGiven: spec.termsGiven || [],   // What parties give up
    termsReceived: spec.termsReceived || [], // What parties receive

    // Consent
    consentType: spec.consentType || 'tacit',
    consentGiven: spec.consentGiven || false,

    // Validity
    validity: {
      noCoercion: spec.noCoercion !== false,
      noDeception: spec.noDeception !== false,
      fairConditions: spec.fairConditions || false,
      mutualBenefit: spec.mutualBenefit || false
    },

    // Revocability
    revocable: spec.revocable !== false,
    revocationCondition: spec.revocationCondition || null,

    createdAt: Date.now()
  };

  state.contracts.set(id, contract);
  saveState();

  return contract;
}

/**
 * Evaluate legitimacy of political authority
 */
function evaluateLegitimacy(spec) {
  const evaluation = {
    entity: spec.name || spec.description || 'Political authority',

    // Hobbesian legitimacy
    hobbesianView: {
      criterion: 'Provides security and peace',
      assessment: null,
      legitimate: null
    },

    // Lockean legitimacy
    lockeanView: {
      criterion: 'Protects natural rights with consent',
      consentPresent: spec.consent || false,
      rightsProtected: spec.protectsRights || false,
      legitimate: null,
      revolutionJustified: null
    },

    // Rousseauian legitimacy
    rousseauianView: {
      criterion: 'Expresses general will',
      generalWillExpressed: spec.generalWill || false,
      participationEnabled: spec.participation || false,
      legitimate: null
    },

    // Overall assessment
    overall: {
      legitimateBy: [],
      notLegitimateBy: [],
      contested: false
    },

    confidence: PHI_INV_3,
    timestamp: Date.now()
  };

  // Hobbesian assessment
  if (spec.providesSecurity) {
    evaluation.hobbesianView.assessment = 'Fulfills primary function';
    evaluation.hobbesianView.legitimate = true;
    evaluation.overall.legitimateBy.push('Hobbes');
  } else {
    evaluation.hobbesianView.assessment = 'Fails to provide security';
    evaluation.hobbesianView.legitimate = false;
    evaluation.overall.notLegitimateBy.push('Hobbes');
  }

  // Lockean assessment
  if (spec.consent && spec.protectsRights) {
    evaluation.lockeanView.legitimate = true;
    evaluation.lockeanView.revolutionJustified = false;
    evaluation.overall.legitimateBy.push('Locke');
  } else if (!spec.protectsRights) {
    evaluation.lockeanView.legitimate = false;
    evaluation.lockeanView.revolutionJustified = true;
    evaluation.overall.notLegitimateBy.push('Locke');
  } else {
    evaluation.lockeanView.legitimate = false;
    evaluation.lockeanView.revolutionJustified = spec.violatesRights || false;
    evaluation.overall.notLegitimateBy.push('Locke');
  }

  // Rousseauian assessment
  if (spec.generalWill && spec.participation) {
    evaluation.rousseauianView.legitimate = true;
    evaluation.overall.legitimateBy.push('Rousseau');
  } else {
    evaluation.rousseauianView.legitimate = false;
    evaluation.overall.notLegitimateBy.push('Rousseau');
  }

  // Check for contested status
  evaluation.overall.contested =
    evaluation.overall.legitimateBy.length > 0 &&
    evaluation.overall.notLegitimateBy.length > 0;

  state.legitimacyEvaluations.push(evaluation);
  saveState();

  return evaluation;
}

/**
 * Assess revolution justification
 */
function assessRevolution(spec) {
  const assessment = {
    context: spec.description || 'Revolution scenario',

    // Lockean assessment (primary framework for revolution)
    lockean: {
      framework: 'Locke - Right of Revolution',
      conditions: {
        governmentViolatesTrust: spec.violatesTrust || false,
        attacksOnRights: spec.attacksRights || false,
        systematicAbuse: spec.systematic || false,
        appealToHeavenExhausted: spec.peacefulMeansTried || false
      },
      justified: null,
      quote: 'When legislators act contrary to their trust, the people have a right to remove them'
    },

    // Hobbesian assessment (very limited)
    hobbesian: {
      framework: 'Hobbes - Right of Self-Preservation',
      condition: 'Only if sovereign threatens your life',
      personalThreat: spec.personalThreat || false,
      justified: null,
      note: 'No general right of revolution - that recreates state of nature'
    },

    // Rousseauian assessment
    rousseauian: {
      framework: 'Rousseau - Sovereign People',
      condition: 'People can always change government (they are sovereign)',
      generalWillExpressed: spec.popularWill || false,
      justified: null,
      note: 'Not really "revolution" - people exercising their sovereignty'
    },

    overall: {
      justified: null,
      frameworks: []
    },

    confidence: PHI_INV_3,
    timestamp: Date.now()
  };

  // Lockean justification
  const lockeanConditions = Object.values(assessment.lockean.conditions);
  const lockeanMet = lockeanConditions.filter(Boolean).length;

  if (lockeanMet >= 3) {
    assessment.lockean.justified = true;
    assessment.overall.frameworks.push('Lockean');
  } else if (lockeanMet >= 2) {
    assessment.lockean.justified = 'POSSIBLY - weigh carefully';
  } else {
    assessment.lockean.justified = false;
  }

  // Hobbesian justification
  assessment.hobbesian.justified = spec.personalThreat
    ? 'Yes (self-preservation only)'
    : 'No (general revolution not permitted)';

  // Rousseauian justification
  assessment.rousseauian.justified = spec.popularWill
    ? 'Yes (people exercising sovereignty)'
    : 'Unclear (is this the general will?)';

  if (spec.popularWill) {
    assessment.overall.frameworks.push('Rousseauian');
  }

  // Overall
  assessment.overall.justified = assessment.overall.frameworks.length > 0;

  state.revolutions.push(assessment);
  saveState();

  return assessment;
}

/**
 * Compare state of nature theories
 */
function compareTheories() {
  return {
    comparison: 'State of Nature Theories',

    hobbes: {
      nature: 'Pessimistic - war of all against all',
      humanNature: 'Self-interested, competitive',
      preopoliticalMorality: 'None (justice comes after sovereign)',
      solution: 'Absolute sovereign',
      keywords: ['Leviathan', 'bellum omnium', 'covenant']
    },

    locke: {
      nature: 'Moderate - inconvenient but not hellish',
      humanNature: 'Rational, capable of morality',
      preopoliticalMorality: 'Natural law exists',
      solution: 'Limited government by consent',
      keywords: ['Natural rights', 'property', 'consent', 'revolution']
    },

    rousseau: {
      nature: 'Optimistic (original) - noble savage',
      humanNature: 'Good until corrupted by society',
      preopoliticalMorality: 'Natural compassion (pitié)',
      solution: 'Direct democracy expressing general will',
      keywords: ['General will', 'chains', 'civil liberty']
    },

    keyDifferences: [
      {
        topic: 'Is the state of nature war?',
        hobbes: 'Yes, necessarily',
        locke: 'Not necessarily, but inconvenient',
        rousseau: 'No, peaceful until corrupted'
      },
      {
        topic: 'Can the contract be broken?',
        hobbes: 'No (except self-defense)',
        locke: 'Yes (government violates trust)',
        rousseau: 'Yes (people are always sovereign)'
      },
      {
        topic: 'What do we give up?',
        hobbes: 'All rights except self-preservation',
        locke: 'Only right to enforce natural law',
        rousseau: 'Natural liberty (gain civil liberty)'
      }
    ],

    confidence: PHI_INV,
    timestamp: Date.now()
  };
}

/**
 * Evaluate contract validity
 */
function evaluateContractValidity(contractId) {
  const contract = state.contracts.get(contractId);
  if (!contract) {
    return { error: 'Contract not found' };
  }

  const evaluation = {
    contractId,

    consent: {
      type: contract.consentType,
      express: contract.consentType === 'express',
      tacit: contract.consentType === 'tacit',
      hypothetical: contract.consentType === 'hypothetical',
      humeanCritique: contract.consentType === 'tacit'
        ? 'Tacit consent may not be genuine consent (Hume)'
        : null,
      valid: contract.consentGiven
    },

    conditions: {
      noCoercion: contract.validity.noCoercion,
      noDeception: contract.validity.noDeception,
      fairConditions: contract.validity.fairConditions,
      mutualBenefit: contract.validity.mutualBenefit,
      allMet: null
    },

    overall: {
      valid: null,
      concerns: []
    },

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Check conditions
  evaluation.conditions.allMet =
    evaluation.conditions.noCoercion &&
    evaluation.conditions.noDeception &&
    evaluation.conditions.fairConditions &&
    evaluation.conditions.mutualBenefit;

  // Identify concerns
  if (!evaluation.conditions.noCoercion) {
    evaluation.overall.concerns.push('Coercion present');
  }
  if (!evaluation.conditions.noDeception) {
    evaluation.overall.concerns.push('Deception present');
  }
  if (!evaluation.conditions.fairConditions) {
    evaluation.overall.concerns.push('Unfair conditions');
  }
  if (!evaluation.conditions.mutualBenefit) {
    evaluation.overall.concerns.push('Not mutually beneficial');
  }
  if (contract.consentType === 'tacit') {
    evaluation.overall.concerns.push('Tacit consent problematic (Hume)');
  }

  // Overall validity
  evaluation.overall.valid =
    evaluation.consent.valid && evaluation.conditions.allMet;

  return evaluation;
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '📜 SOCIAL CONTRACT ENGINE - Hobbes, Locke, Rousseau',
    '═══════════════════════════════════════════════════════════',
    '',
    '── HOBBES ─────────────────────────────────────────────────',
    '   State of Nature: War of all against all',
    '   Solution: Absolute sovereign (Leviathan)',
    '   "Covenants without the sword are but words"',
    '',
    '── LOCKE ──────────────────────────────────────────────────',
    '   State of Nature: Inconvenient but governed by natural law',
    '   Solution: Limited government by consent',
    '   Right of revolution if trust violated',
    '',
    '── ROUSSEAU ───────────────────────────────────────────────',
    '   State of Nature: Noble savage, corrupted by society',
    '   Solution: General will, direct democracy',
    '   "Man is born free, and everywhere he is in chains"',
    '',
    '── STATISTICS ─────────────────────────────────────────────',
    `   Contracts: ${state.contracts.size}`,
    `   Legitimacy evaluations: ${state.legitimacyEvaluations.length}`,
    `   State of nature analyses: ${state.stateOfNatureAnalyses.length}`,
    `   Revolution assessments: ${state.revolutions.length}`,
    '',
    '═══════════════════════════════════════════════════════════',
    '*sniff* Consent legitimates power. Or does it?',
    '═══════════════════════════════════════════════════════════'
  ];

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  return {
    contracts: state.contracts.size,
    legitimacyEvaluations: state.legitimacyEvaluations.length,
    stateOfNatureAnalyses: state.stateOfNatureAnalyses.length,
    revolutions: state.revolutions.length
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // State of Nature
  analyzeStateOfNature,
  compareTheories,

  // Contracts
  createContract,
  evaluateContractValidity,

  // Legitimacy
  evaluateLegitimacy,
  assessRevolution,

  // Theories
  HOBBES,
  LOCKE,
  ROUSSEAU,
  CONTRACT_VALIDITY,

  // Constants
  PHI,
  PHI_INV
};

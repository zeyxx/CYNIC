/**
 * CYNIC Collective Reasoning Engine
 *
 * "Group agency and judgment aggregation"
 *
 * Philosophical foundations:
 * - List & Pettit: Group agency and collective intentionality
 * - Arrow: Impossibility theorem
 * - Condorcet: Jury theorem
 * - Discursive dilemma: Premise vs conclusion-based aggregation
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
// GROUP AGENCY CRITERIA (List & Pettit)
// ─────────────────────────────────────────────────────────────

const GROUP_AGENCY_CONDITIONS = {
  representational: {
    name: 'Representational States',
    description: 'Group has beliefs about the world',
    required: true,
    weight: PHI_INV
  },
  motivational: {
    name: 'Motivational States',
    description: 'Group has desires/goals',
    required: true,
    weight: PHI_INV
  },
  capacity: {
    name: 'Capacity to Act',
    description: 'Group can intervene in the world',
    required: true,
    weight: PHI_INV
  },
  rational: {
    name: 'Rational Integration',
    description: 'Attitudes form coherent whole',
    required: true,
    weight: PHI_INV + PHI_INV_3
  }
};

// ─────────────────────────────────────────────────────────────
// AGGREGATION METHODS
// ─────────────────────────────────────────────────────────────

const AGGREGATION_METHODS = {
  majority: {
    name: 'Simple Majority',
    description: 'More than 50% agreement',
    threshold: 0.5,
    properties: ['anonymity', 'neutrality', 'positive_responsiveness']
  },
  supermajority: {
    name: 'Supermajority',
    description: 'Requires higher threshold (e.g., 2/3)',
    threshold: PHI_INV, // 61.8% - φ threshold
    properties: ['anonymity', 'neutrality']
  },
  unanimity: {
    name: 'Unanimity',
    description: 'Full agreement required',
    threshold: 1.0,
    properties: ['pareto', 'non_dictatorial']
  },
  plurality: {
    name: 'Plurality',
    description: 'Most votes wins',
    threshold: null,
    properties: ['anonymity']
  },
  borda: {
    name: 'Borda Count',
    description: 'Ranked preferences with points',
    threshold: null,
    properties: ['neutrality']
  },
  condorcet: {
    name: 'Condorcet Method',
    description: 'Pairwise comparison winner',
    threshold: null,
    properties: ['condorcet_criterion']
  },
  premise_based: {
    name: 'Premise-Based',
    description: 'Aggregate premises, derive conclusion',
    threshold: 0.5,
    properties: ['deductive_closure'],
    discursiveDilemmaResolution: true
  },
  conclusion_based: {
    name: 'Conclusion-Based',
    description: 'Aggregate conclusions directly',
    threshold: 0.5,
    properties: ['propositionwise_independence'],
    discursiveDilemmaResolution: true
  }
};

// ─────────────────────────────────────────────────────────────
// ARROW'S CONDITIONS
// ─────────────────────────────────────────────────────────────

const ARROW_CONDITIONS = {
  unrestricted_domain: {
    name: 'Unrestricted Domain',
    description: 'All preference orderings allowed',
    symbol: 'U'
  },
  pareto: {
    name: 'Pareto Efficiency',
    description: 'If all prefer A>B, group prefers A>B',
    symbol: 'P'
  },
  independence: {
    name: 'Independence of Irrelevant Alternatives',
    description: 'A vs B depends only on A vs B preferences',
    symbol: 'I'
  },
  non_dictatorship: {
    name: 'Non-Dictatorship',
    description: 'No single voter determines outcome',
    symbol: 'D'
  }
};

// Arrow's impossibility: No aggregation satisfies all four for 3+ options

// ─────────────────────────────────────────────────────────────
// JUDGMENT TYPES
// ─────────────────────────────────────────────────────────────

const JUDGMENT_TYPES = {
  factual: {
    name: 'Factual Judgment',
    description: 'Belief about facts',
    weight: PHI_INV
  },
  evaluative: {
    name: 'Evaluative Judgment',
    description: 'Value assessment',
    weight: PHI_INV_2
  },
  normative: {
    name: 'Normative Judgment',
    description: 'Ought claims',
    weight: PHI_INV_2
  },
  practical: {
    name: 'Practical Judgment',
    description: 'What to do',
    weight: PHI_INV
  }
};

// ─────────────────────────────────────────────────────────────
// STATE
// ─────────────────────────────────────────────────────────────

const state = {
  groups: new Map(),              // Defined groups
  judgmentSets: new Map(),        // Individual judgment sets
  aggregations: [],               // Performed aggregations
  discursiveDilemmas: [],         // Identified dilemmas
  groupBeliefs: new Map(),        // Aggregated group beliefs
  groupDesires: new Map(),        // Aggregated group desires
  stats: {
    groupsFormed: 0,
    aggregationsPerformed: 0,
    dilemmasIdentified: 0,
    collectiveJudgments: 0
  }
};

// Storage
const STORAGE_DIR = path.join(os.homedir(), '.cynic', 'collective-reasoning');
const STATE_FILE = path.join(STORAGE_DIR, 'state.json');
const HISTORY_FILE = path.join(STORAGE_DIR, 'history.jsonl');

// ─────────────────────────────────────────────────────────────
// GROUP FORMATION
// ─────────────────────────────────────────────────────────────

/**
 * Form a reasoning group
 */
function formGroup(id, spec) {
  const group = {
    id,
    name: spec.name || id,
    members: spec.members || [],
    decisionProcedure: spec.decisionProcedure || 'majority',
    aggregationMethod: spec.aggregationMethod || 'majority',
    created: Date.now(),
    agencyStatus: assessGroupAgency(spec),
    beliefs: new Map(),
    desires: new Map(),
    intentions: new Map()
  };

  state.groups.set(id, group);
  state.stats.groupsFormed++;

  appendHistory({
    type: 'group_formed',
    group: id,
    members: spec.members,
    timestamp: Date.now()
  });

  return group;
}

/**
 * Assess whether group meets agency conditions
 */
function assessGroupAgency(spec) {
  const assessment = {};
  let totalWeight = 0;
  let satisfiedWeight = 0;

  for (const [key, condition] of Object.entries(GROUP_AGENCY_CONDITIONS)) {
    const satisfied = spec[key] !== false; // Assume true unless explicitly false
    assessment[key] = {
      condition: condition.name,
      satisfied,
      required: condition.required
    };

    totalWeight += condition.weight;
    if (satisfied) {
      satisfiedWeight += condition.weight;
    }
  }

  assessment.agencyScore = satisfiedWeight / totalWeight;
  assessment.isAgent = assessment.agencyScore >= PHI_INV_2;
  assessment.confidence = Math.min(assessment.agencyScore, PHI_INV);

  return assessment;
}

/**
 * Add member to group
 */
function addMember(groupId, memberId) {
  const group = state.groups.get(groupId);
  if (!group) return null;

  if (!group.members.includes(memberId)) {
    group.members.push(memberId);
    group.agencyStatus = assessGroupAgency({
      ...group,
      representational: true,
      motivational: true,
      capacity: group.members.length >= 2,
      rational: true
    });
  }

  return group;
}

// ─────────────────────────────────────────────────────────────
// JUDGMENT AGGREGATION
// ─────────────────────────────────────────────────────────────

/**
 * Submit individual judgment
 */
function submitJudgment(memberId, proposition, judgment, type = 'factual') {
  if (!state.judgmentSets.has(memberId)) {
    state.judgmentSets.set(memberId, new Map());
  }

  state.judgmentSets.get(memberId).set(proposition, {
    judgment, // true/false or ranking
    type,
    confidence: PHI_INV,
    timestamp: Date.now()
  });

  return { memberId, proposition, judgment, type };
}

/**
 * Aggregate judgments for a proposition
 */
function aggregateJudgments(groupId, proposition, method = null) {
  const group = state.groups.get(groupId);
  if (!group) return null;

  const aggregationMethod = method || group.aggregationMethod;
  const methodSpec = AGGREGATION_METHODS[aggregationMethod];

  // Collect member judgments
  const votes = [];
  for (const memberId of group.members) {
    const judgments = state.judgmentSets.get(memberId);
    if (judgments && judgments.has(proposition)) {
      votes.push({
        member: memberId,
        judgment: judgments.get(proposition).judgment
      });
    }
  }

  if (votes.length === 0) {
    return { proposition, result: null, reason: 'no_votes' };
  }

  // Aggregate based on method
  let result = null;
  let support = 0;
  let details = {};

  const trueVotes = votes.filter(v => v.judgment === true).length;
  const falseVotes = votes.filter(v => v.judgment === false).length;
  const totalVotes = votes.length;

  switch (aggregationMethod) {
    case 'majority':
    case 'supermajority': {
      const threshold = methodSpec.threshold;
      support = trueVotes / totalVotes;
      result = support > threshold;
      details = { trueVotes, falseVotes, totalVotes, threshold, support };
      break;
    }

    case 'unanimity': {
      result = trueVotes === totalVotes;
      support = result ? 1.0 : trueVotes / totalVotes;
      details = { trueVotes, totalVotes, unanimous: result };
      break;
    }

    case 'plurality': {
      result = trueVotes > falseVotes;
      support = Math.max(trueVotes, falseVotes) / totalVotes;
      details = { trueVotes, falseVotes, winner: result ? 'true' : 'false' };
      break;
    }

    default:
      result = trueVotes > falseVotes;
      support = trueVotes / totalVotes;
  }

  const aggregation = {
    groupId,
    proposition,
    method: aggregationMethod,
    result,
    support,
    details,
    confidence: Math.min(support, PHI_INV),
    timestamp: Date.now()
  };

  state.aggregations.push(aggregation);
  state.stats.aggregationsPerformed++;
  state.stats.collectiveJudgments++;

  // Store as group belief
  state.groupBeliefs.set(`${groupId}:${proposition}`, aggregation);

  appendHistory({
    type: 'aggregation',
    aggregation,
    timestamp: Date.now()
  });

  return aggregation;
}

/**
 * Check for discursive dilemma
 *
 * The dilemma: premise-based and conclusion-based aggregation can yield
 * different results, creating inconsistency.
 *
 * Classic example:
 * - P1: Contract valid? P2: Breach occurred? C: Liable (P1 ∧ P2)?
 * - Individual judgments can be consistent, but group judgment inconsistent
 */
function checkDiscursiveDilemma(groupId, premises, conclusion, derivation) {
  const group = state.groups.get(groupId);
  if (!group) return null;

  // Aggregate premises
  const premiseResults = {};
  for (const premise of premises) {
    const agg = aggregateJudgments(groupId, premise);
    premiseResults[premise] = agg ? agg.result : null;
  }

  // Derive conclusion from aggregated premises
  const derivedConclusion = derivation(premiseResults);

  // Aggregate conclusion directly
  const directConclusion = aggregateJudgments(groupId, conclusion);

  // Check for dilemma
  const dilemmaExists = derivedConclusion !== (directConclusion ? directConclusion.result : null);

  const dilemma = {
    groupId,
    premises,
    conclusion,
    premiseResults,
    derivedConclusion,
    directConclusion: directConclusion ? directConclusion.result : null,
    dilemmaExists,
    timestamp: Date.now()
  };

  if (dilemmaExists) {
    dilemma.resolution = {
      premiseBased: {
        result: derivedConclusion,
        rationale: 'Follows from aggregated premises via deductive closure'
      },
      conclusionBased: {
        result: directConclusion ? directConclusion.result : null,
        rationale: 'Respects propositionwise independence'
      },
      recommendation: 'premise_based', // List & Pettit's recommendation
      confidence: PHI_INV_2
    };

    state.discursiveDilemmas.push(dilemma);
    state.stats.dilemmasIdentified++;
  }

  appendHistory({
    type: 'discursive_dilemma_check',
    dilemma,
    timestamp: Date.now()
  });

  return dilemma;
}

// ─────────────────────────────────────────────────────────────
// CONDORCET JURY THEOREM
// ─────────────────────────────────────────────────────────────

/**
 * Calculate probability of correct majority decision
 * (Condorcet's Jury Theorem)
 *
 * If each voter has probability p > 0.5 of being correct,
 * the probability that majority is correct approaches 1 as n → ∞
 */
function calculateCondorcetProbability(n, p) {
  if (p <= 0.5) {
    return {
      probability: p,
      warning: 'Individual competence p ≤ 0.5: majority may be worse',
      juryTheorem: false
    };
  }

  // For odd n, probability = sum of binomial(n,k) * p^k * (1-p)^(n-k) for k > n/2
  let probability = 0;
  const majority = Math.ceil((n + 1) / 2);

  for (let k = majority; k <= n; k++) {
    probability += binomial(n, k) * Math.pow(p, k) * Math.pow(1 - p, n - k);
  }

  return {
    n,
    p,
    probability: Math.min(probability, PHI_INV), // Cap at φ⁻¹
    majority,
    juryTheorem: true,
    insight: probability > p
      ? `Group wisdom: ${(probability * 100).toFixed(1)}% > individual ${(p * 100).toFixed(1)}%`
      : 'Group not improving on individual'
  };
}

function binomial(n, k) {
  if (k < 0 || k > n) return 0;
  if (k === 0 || k === n) return 1;
  let result = 1;
  for (let i = 1; i <= k; i++) {
    result = result * (n - k + i) / i;
  }
  return result;
}

// ─────────────────────────────────────────────────────────────
// PREFERENCE AGGREGATION
// ─────────────────────────────────────────────────────────────

/**
 * Submit ranked preferences
 */
function submitPreferences(memberId, ranking) {
  // ranking is array of options from most to least preferred
  if (!state.judgmentSets.has(memberId)) {
    state.judgmentSets.set(memberId, new Map());
  }

  state.judgmentSets.get(memberId).set('_preferences', {
    ranking,
    type: 'preferences',
    timestamp: Date.now()
  });

  return { memberId, ranking };
}

/**
 * Aggregate preferences using Borda count
 */
function bordaCount(groupId, options) {
  const group = state.groups.get(groupId);
  if (!group) return null;

  const scores = {};
  for (const opt of options) {
    scores[opt] = 0;
  }

  const n = options.length;

  for (const memberId of group.members) {
    const judgments = state.judgmentSets.get(memberId);
    if (judgments && judgments.has('_preferences')) {
      const ranking = judgments.get('_preferences').ranking;

      // Award points: n-1 for first, n-2 for second, etc.
      for (let i = 0; i < ranking.length; i++) {
        const option = ranking[i];
        if (Object.hasOwn(scores, option)) {
          scores[option] += (n - 1 - i);
        }
      }
    }
  }

  // Rank by scores
  const ranked = Object.entries(scores)
    .sort((a, b) => b[1] - a[1])
    .map(([option, score]) => ({ option, score }));

  return {
    groupId,
    method: 'borda',
    scores,
    ranking: ranked,
    winner: ranked[0] ? ranked[0].option : null,
    confidence: PHI_INV_2
  };
}

/**
 * Find Condorcet winner (if exists)
 */
function condorcetWinner(groupId, options) {
  const group = state.groups.get(groupId);
  if (!group) return null;

  // Build pairwise comparison matrix
  const pairwise = {};
  for (const a of options) {
    pairwise[a] = {};
    for (const b of options) {
      pairwise[a][b] = 0;
    }
  }

  // Count pairwise preferences
  for (const memberId of group.members) {
    const judgments = state.judgmentSets.get(memberId);
    if (judgments && judgments.has('_preferences')) {
      const ranking = judgments.get('_preferences').ranking;

      for (let i = 0; i < ranking.length; i++) {
        for (let j = i + 1; j < ranking.length; j++) {
          const preferred = ranking[i];
          const lessPreferred = ranking[j];
          if (pairwise[preferred] && pairwise[preferred][lessPreferred] !== undefined) {
            pairwise[preferred][lessPreferred]++;
          }
        }
      }
    }
  }

  // Find Condorcet winner: beats all others in pairwise comparison
  let winner = null;
  const totalVoters = group.members.length;

  for (const candidate of options) {
    let beatsAll = true;

    for (const opponent of options) {
      if (candidate === opponent) continue;

      const winsAgainst = pairwise[candidate][opponent];
      const losesTo = pairwise[opponent][candidate];

      if (winsAgainst <= losesTo) {
        beatsAll = false;
        break;
      }
    }

    if (beatsAll) {
      winner = candidate;
      break;
    }
  }

  return {
    groupId,
    method: 'condorcet',
    pairwiseMatrix: pairwise,
    winner,
    condorcetWinnerExists: winner !== null,
    insight: winner
      ? `${winner} defeats all alternatives in pairwise comparison`
      : 'No Condorcet winner exists (cycle possible)',
    confidence: winner ? PHI_INV : PHI_INV_3
  };
}

/**
 * Check Arrow's conditions for aggregation procedure
 */
function checkArrowConditions(procedure) {
  const violations = [];

  // All procedures violate at least one condition (Arrow's theorem)
  switch (procedure) {
    case 'majority':
      // Can violate transitivity (Condorcet paradox)
      violations.push({
        condition: 'rationality',
        description: 'May produce intransitive social preferences'
      });
      break;

    case 'borda':
      violations.push({
        condition: 'independence',
        description: 'Ranking depends on irrelevant alternatives'
      });
      break;

    case 'dictator':
      violations.push({
        condition: 'non_dictatorship',
        description: 'Single individual determines outcome'
      });
      break;

    default:
      violations.push({
        condition: 'unknown',
        description: 'Arrow\'s theorem: some condition must be violated'
      });
  }

  return {
    procedure,
    arrowConditions: ARROW_CONDITIONS,
    violations,
    insight: 'Arrow\'s Impossibility: No procedure satisfies all desirable properties',
    confidence: PHI_INV
  };
}

// ─────────────────────────────────────────────────────────────
// GROUP ATTITUDES
// ─────────────────────────────────────────────────────────────

/**
 * Form group belief through aggregation
 */
function formGroupBelief(groupId, proposition) {
  const agg = aggregateJudgments(groupId, proposition);
  if (!agg) return null;

  const group = state.groups.get(groupId);
  if (group) {
    group.beliefs.set(proposition, {
      value: agg.result,
      support: agg.support,
      confidence: agg.confidence,
      timestamp: Date.now()
    });
  }

  return {
    groupId,
    proposition,
    belief: agg.result,
    support: agg.support,
    confidence: agg.confidence,
    type: 'collective_belief'
  };
}

/**
 * Form group desire/goal
 */
function formGroupDesire(groupId, goal, method = 'majority') {
  const group = state.groups.get(groupId);
  if (!group) return null;

  const agg = aggregateJudgments(groupId, `desire:${goal}`, method);

  if (agg && agg.result) {
    group.desires.set(goal, {
      value: true,
      support: agg.support,
      confidence: agg.confidence,
      timestamp: Date.now()
    });

    state.groupDesires.set(`${groupId}:${goal}`, {
      groupId,
      goal,
      support: agg.support,
      confidence: agg.confidence
    });
  }

  return {
    groupId,
    goal,
    adopted: agg ? agg.result : false,
    support: agg ? agg.support : 0,
    confidence: agg ? agg.confidence : 0
  };
}

/**
 * Form group intention (for action)
 */
function formGroupIntention(groupId, action, conditions = {}) {
  const group = state.groups.get(groupId);
  if (!group) return null;

  // Check preconditions: relevant belief and desire
  const relevantBelief = conditions.belief
    ? group.beliefs.get(conditions.belief)
    : { value: true, confidence: PHI_INV };

  const relevantDesire = conditions.desire
    ? group.desires.get(conditions.desire)
    : { value: true, confidence: PHI_INV };

  // Form intention if belief and desire support it
  const canIntend = relevantBelief && relevantBelief.value &&
                    relevantDesire && relevantDesire.value;

  if (canIntend) {
    const confidence = Math.min(
      relevantBelief.confidence || PHI_INV,
      relevantDesire.confidence || PHI_INV
    );

    group.intentions.set(action, {
      value: true,
      basedOn: { belief: conditions.belief, desire: conditions.desire },
      confidence,
      timestamp: Date.now()
    });

    return {
      groupId,
      action,
      intends: true,
      confidence,
      rationale: `Based on belief (${conditions.belief}) and desire (${conditions.desire})`
    };
  }

  return {
    groupId,
    action,
    intends: false,
    reason: 'Prerequisites not met',
    confidence: 0
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
    groups: Array.from(state.groups.entries()).map(([id, g]) => [id, {
      ...g,
      beliefs: Array.from(g.beliefs.entries()),
      desires: Array.from(g.desires.entries()),
      intentions: Array.from(g.intentions.entries())
    }]),
    judgmentSets: Array.from(state.judgmentSets.entries()).map(([id, js]) =>
      [id, Array.from(js.entries())]),
    aggregations: state.aggregations.slice(-100),
    discursiveDilemmas: state.discursiveDilemmas.slice(-20),
    groupBeliefs: Array.from(state.groupBeliefs.entries()),
    groupDesires: Array.from(state.groupDesires.entries()),
    stats: state.stats
  };

  fs.writeFileSync(STATE_FILE, JSON.stringify(serializable, null, 2));
}

function loadState() {
  ensureStorageDir();

  if (fs.existsSync(STATE_FILE)) {
    try {
      const data = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));

      state.groups = new Map(
        (data.groups || []).map(([id, g]) => [id, {
          ...g,
          beliefs: new Map(g.beliefs || []),
          desires: new Map(g.desires || []),
          intentions: new Map(g.intentions || [])
        }])
      );
      state.judgmentSets = new Map(
        (data.judgmentSets || []).map(([id, js]) => [id, new Map(js)])
      );
      state.aggregations = data.aggregations || [];
      state.discursiveDilemmas = data.discursiveDilemmas || [];
      state.groupBeliefs = new Map(data.groupBeliefs || []);
      state.groupDesires = new Map(data.groupDesires || []);
      state.stats = data.stats || state.stats;
    } catch (e) {
      console.error('Failed to load collective reasoning state:', e.message);
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
    '── COLLECTIVE REASONING ───────────────────────────────────',
    ''
  ];

  // Stats
  lines.push(`   Groups: ${state.groups.size} | Aggregations: ${state.stats.aggregationsPerformed}`);
  lines.push(`   Collective judgments: ${state.stats.collectiveJudgments}`);
  lines.push(`   Discursive dilemmas: ${state.stats.dilemmasIdentified}`);
  lines.push('');

  // Groups
  if (state.groups.size > 0) {
    lines.push('   Groups:');
    for (const [id, group] of state.groups) {
      const agencyStatus = group.agencyStatus.isAgent ? '✓ agent' : '○ proto';
      lines.push(`   └─ ${group.name}: ${group.members.length} members (${agencyStatus})`);
    }
  }

  lines.push('');
  return lines.join('\n');
}

function getStats() {
  return {
    ...state.stats,
    groupsCount: state.groups.size,
    judgmentSetsCount: state.judgmentSets.size,
    beliefCount: state.groupBeliefs.size,
    desireCount: state.groupDesires.size
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
    groups: state.groups.size
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
  GROUP_AGENCY_CONDITIONS,
  AGGREGATION_METHODS,
  ARROW_CONDITIONS,
  JUDGMENT_TYPES,

  // Group formation
  formGroup,
  assessGroupAgency,
  addMember,

  // Judgment aggregation
  submitJudgment,
  aggregateJudgments,
  checkDiscursiveDilemma,

  // Condorcet
  calculateCondorcetProbability,

  // Preference aggregation
  submitPreferences,
  bordaCount,
  condorcetWinner,
  checkArrowConditions,

  // Group attitudes
  formGroupBelief,
  formGroupDesire,
  formGroupIntention,

  // State access
  getGroup: (id) => state.groups.get(id),
  getGroupBelief: (groupId, prop) => state.groupBeliefs.get(`${groupId}:${prop}`),
  getGroupDesire: (groupId, goal) => state.groupDesires.get(`${groupId}:${goal}`),

  // Persistence
  saveState,
  loadState,

  // Formatting
  formatStatus,
  getStats,
  init
};

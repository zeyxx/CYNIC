/**
 * CYNIC Hypothesis Testing Module (Phase 11B)
 *
 * "Ὑπόθεσις - foundation of knowledge" - κυνικός
 *
 * Implements Socratic hypothesis testing:
 * - Formulate clear hypotheses
 * - Define falsification criteria
 * - Propose experiments to test
 * - Track results and update beliefs
 *
 * Popper's falsificationism meets Socratic method:
 * "A theory that cannot be refuted is not scientific."
 *
 * @module cynic/lib/hypothesis-testing
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// Import φ constants
const phiMath = require('./phi-math.cjs');
const { PHI, PHI_INV, PHI_INV_2, PHI_INV_3 } = phiMath;

// =============================================================================
// CONSTANTS (φ-derived)
// =============================================================================

/** Maximum active hypotheses - φ × 5 ≈ 8 */
const MAX_ACTIVE_HYPOTHESES = Math.round(PHI * 5);

/** Default prior probability - φ⁻¹ */
const DEFAULT_PRIOR = PHI_INV;

/** Falsification threshold - below this, hypothesis rejected */
const FALSIFICATION_THRESHOLD = PHI_INV_3;

/** Confirmation threshold - above this, hypothesis accepted */
const CONFIRMATION_THRESHOLD = 1 - PHI_INV_3;

/** Max experiments per hypothesis - φ × 3 ≈ 5 */
const MAX_EXPERIMENTS = Math.round(PHI * 3);

// =============================================================================
// STORAGE
// =============================================================================

const HYPOTHESIS_DIR = path.join(os.homedir(), '.cynic', 'hypothesis');
const STATE_FILE = path.join(HYPOTHESIS_DIR, 'state.json');
const HISTORY_FILE = path.join(HYPOTHESIS_DIR, 'history.jsonl');

// =============================================================================
// STATE
// =============================================================================

const hypothesisState = {
  // Active hypotheses
  hypotheses: {},

  // Completed hypotheses
  completed: [],

  stats: {
    totalHypotheses: 0,
    confirmed: 0,
    falsified: 0,
    inconclusive: 0,
    experimentsRun: 0,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(HYPOTHESIS_DIR)) {
    fs.mkdirSync(HYPOTHESIS_DIR, { recursive: true });
  }
}

function loadState() {
  ensureDir();
  if (!fs.existsSync(STATE_FILE)) {
    return null;
  }
  try {
    return JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
  } catch {
    return null;
  }
}

function saveState() {
  ensureDir();
  fs.writeFileSync(STATE_FILE, JSON.stringify({
    hypotheses: hypothesisState.hypotheses,
    stats: hypothesisState.stats,
  }, null, 2));
}

function appendHistory(event) {
  ensureDir();
  const line = JSON.stringify({ ...event, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(HISTORY_FILE, line);
}

// =============================================================================
// HYPOTHESIS CREATION
// =============================================================================

/**
 * Create a new hypothesis
 *
 * @param {string} statement - Hypothesis statement
 * @param {Object} options - Options
 * @returns {Object} Created hypothesis
 */
function createHypothesis(statement, options = {}) {
  // Check capacity
  if (Object.keys(hypothesisState.hypotheses).length >= MAX_ACTIVE_HYPOTHESES) {
    return {
      error: 'Max active hypotheses reached',
      suggestion: 'Complete or abandon some hypotheses first',
    };
  }

  const id = `hyp-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

  const hypothesis = {
    id,
    statement,
    prior: options.prior || DEFAULT_PRIOR,
    posterior: options.prior || DEFAULT_PRIOR,
    falsificationCriteria: options.falsificationCriteria || [],
    confirmationCriteria: options.confirmationCriteria || [],
    experiments: [],
    status: 'active',
    createdAt: Date.now(),
    domain: options.domain || 'general',
  };

  hypothesisState.hypotheses[id] = hypothesis;
  hypothesisState.stats.totalHypotheses++;

  appendHistory({
    type: 'hypothesis_created',
    id,
    statement,
  });

  saveState();

  return hypothesis;
}

/**
 * Add falsification criteria to a hypothesis
 *
 * @param {string} id - Hypothesis ID
 * @param {string} criterion - What would prove hypothesis false
 * @returns {Object} Updated hypothesis
 */
function addFalsificationCriterion(id, criterion) {
  const hypothesis = hypothesisState.hypotheses[id];
  if (!hypothesis) {
    return { error: 'Hypothesis not found' };
  }

  hypothesis.falsificationCriteria.push({
    criterion,
    addedAt: Date.now(),
    tested: false,
    result: null,
  });

  saveState();

  return hypothesis;
}

/**
 * Add confirmation criteria to a hypothesis
 *
 * @param {string} id - Hypothesis ID
 * @param {string} criterion - What would support hypothesis
 * @returns {Object} Updated hypothesis
 */
function addConfirmationCriterion(id, criterion) {
  const hypothesis = hypothesisState.hypotheses[id];
  if (!hypothesis) {
    return { error: 'Hypothesis not found' };
  }

  hypothesis.confirmationCriteria.push({
    criterion,
    addedAt: Date.now(),
    tested: false,
    result: null,
  });

  saveState();

  return hypothesis;
}

// =============================================================================
// EXPERIMENT MANAGEMENT
// =============================================================================

/**
 * Propose an experiment for a hypothesis
 *
 * @param {string} id - Hypothesis ID
 * @param {Object} experiment - Experiment details
 * @returns {Object} Proposed experiment
 */
function proposeExperiment(id, experiment) {
  const hypothesis = hypothesisState.hypotheses[id];
  if (!hypothesis) {
    return { error: 'Hypothesis not found' };
  }

  if (hypothesis.experiments.length >= MAX_EXPERIMENTS) {
    return {
      error: 'Max experiments reached for this hypothesis',
      suggestion: 'Evaluate existing results before proposing more',
    };
  }

  const exp = {
    id: `exp-${Date.now()}-${Math.random().toString(36).slice(2, 4)}`,
    description: experiment.description,
    expectedOutcome: experiment.expectedOutcome,
    wouldFalsify: experiment.wouldFalsify || false,
    wouldConfirm: experiment.wouldConfirm || false,
    status: 'proposed',
    proposedAt: Date.now(),
    result: null,
  };

  hypothesis.experiments.push(exp);
  saveState();

  return {
    experiment: exp,
    message: generateExperimentPrompt(exp),
  };
}

/**
 * Record experiment result
 *
 * @param {string} hypothesisId - Hypothesis ID
 * @param {string} experimentId - Experiment ID
 * @param {Object} result - Result
 * @returns {Object} Updated state
 */
function recordResult(hypothesisId, experimentId, result) {
  const hypothesis = hypothesisState.hypotheses[hypothesisId];
  if (!hypothesis) {
    return { error: 'Hypothesis not found' };
  }

  const experiment = hypothesis.experiments.find(e => e.id === experimentId);
  if (!experiment) {
    return { error: 'Experiment not found' };
  }

  experiment.status = 'completed';
  experiment.completedAt = Date.now();
  experiment.result = {
    outcome: result.outcome,
    matchedExpected: result.outcome === experiment.expectedOutcome,
    notes: result.notes,
  };

  hypothesisState.stats.experimentsRun++;

  // Update posterior probability (Bayesian update simplified)
  updatePosterior(hypothesis, experiment);

  // Check if hypothesis should be concluded
  const conclusion = checkConclusion(hypothesis);

  appendHistory({
    type: 'experiment_completed',
    hypothesisId,
    experimentId,
    result: experiment.result,
    newPosterior: hypothesis.posterior,
  });

  saveState();

  return {
    experiment,
    hypothesis,
    conclusion,
    newPosterior: hypothesis.posterior,
  };
}

/**
 * Update posterior probability based on experiment
 *
 * @param {Object} hypothesis - Hypothesis
 * @param {Object} experiment - Completed experiment
 */
function updatePosterior(hypothesis, experiment) {
  const prior = hypothesis.posterior;
  let likelihood;

  if (experiment.result.matchedExpected) {
    // Result matches expectation
    if (experiment.wouldConfirm) {
      likelihood = PHI_INV + PHI_INV_2; // Strong confirmation
    } else if (experiment.wouldFalsify) {
      // Expected to falsify but didn't - slight confirmation
      likelihood = PHI_INV;
    } else {
      likelihood = 0.5 + PHI_INV_3; // Neutral positive
    }
  } else {
    // Result doesn't match expectation
    if (experiment.wouldFalsify) {
      likelihood = PHI_INV_3; // Strong falsification
    } else if (experiment.wouldConfirm) {
      // Expected to confirm but didn't
      likelihood = PHI_INV_2;
    } else {
      likelihood = 0.5 - PHI_INV_3; // Neutral negative
    }
  }

  // Simplified Bayesian update: P(H|E) ∝ P(E|H) × P(H)
  const unnormalized = likelihood * prior;
  hypothesis.posterior = Math.max(0.01, Math.min(0.99, unnormalized));
}

/**
 * Check if hypothesis should be concluded
 *
 * @param {Object} hypothesis - Hypothesis
 * @returns {Object|null} Conclusion or null
 */
function checkConclusion(hypothesis) {
  if (hypothesis.posterior <= FALSIFICATION_THRESHOLD) {
    return concludeHypothesis(hypothesis.id, 'falsified');
  }

  if (hypothesis.posterior >= CONFIRMATION_THRESHOLD) {
    return concludeHypothesis(hypothesis.id, 'confirmed');
  }

  if (hypothesis.experiments.length >= MAX_EXPERIMENTS) {
    return concludeHypothesis(hypothesis.id, 'inconclusive');
  }

  return null;
}

/**
 * Conclude a hypothesis
 *
 * @param {string} id - Hypothesis ID
 * @param {string} verdict - confirmed|falsified|inconclusive|abandoned
 * @returns {Object} Conclusion
 */
function concludeHypothesis(id, verdict) {
  const hypothesis = hypothesisState.hypotheses[id];
  if (!hypothesis) {
    return { error: 'Hypothesis not found' };
  }

  hypothesis.status = verdict;
  hypothesis.concludedAt = Date.now();

  // Update stats
  if (verdict === 'confirmed') {
    hypothesisState.stats.confirmed++;
  } else if (verdict === 'falsified') {
    hypothesisState.stats.falsified++;
  } else {
    hypothesisState.stats.inconclusive++;
  }

  // Move to completed
  hypothesisState.completed.push(hypothesis);
  delete hypothesisState.hypotheses[id];

  appendHistory({
    type: 'hypothesis_concluded',
    id,
    verdict,
    finalPosterior: hypothesis.posterior,
  });

  saveState();

  return {
    hypothesis,
    verdict,
    message: generateConclusionMessage(hypothesis, verdict),
  };
}

// =============================================================================
// MESSAGE GENERATION
// =============================================================================

/**
 * Generate experiment prompt
 */
function generateExperimentPrompt(experiment) {
  const prompts = [
    `*head tilt* Expérience proposée: ${experiment.description}`,
    `Résultat attendu: ${experiment.expectedOutcome}`,
    experiment.wouldFalsify
      ? '⚠️ Cette expérience pourrait FALSIFIER l\'hypothèse.'
      : experiment.wouldConfirm
        ? '✓ Cette expérience pourrait CONFIRMER l\'hypothèse.'
        : 'ℹ️ Expérience exploratoire.',
  ];
  return prompts.join('\n');
}

/**
 * Generate conclusion message
 */
function generateConclusionMessage(hypothesis, verdict) {
  const messages = {
    confirmed: [
      `*tail wag* Hypothèse CONFIRMÉE: "${hypothesis.statement}"`,
      `Probabilité finale: ${Math.round(hypothesis.posterior * 100)}%`,
      'Popper approuverait: l\'hypothèse a survécu aux tests.',
    ],
    falsified: [
      `*nod* Hypothèse FALSIFIÉE: "${hypothesis.statement}"`,
      `Probabilité finale: ${Math.round(hypothesis.posterior * 100)}%`,
      'C\'est une victoire! Falsifier est plus informatif que confirmer.',
    ],
    inconclusive: [
      `*head tilt* Hypothèse INCONCLUANTE: "${hypothesis.statement}"`,
      `Probabilité finale: ${Math.round(hypothesis.posterior * 100)}%`,
      'Plus de données nécessaires. Reformule ou abandonne.',
    ],
    abandoned: [
      `*sniff* Hypothèse ABANDONNÉE: "${hypothesis.statement}"`,
      'Parfois il faut savoir arrêter de creuser.',
    ],
  };

  return messages[verdict]?.join('\n') || 'Hypothèse conclue.';
}

/**
 * Generate hypothesis question prompts
 */
function generateHypothesisQuestions(statement) {
  return [
    {
      type: 'falsification',
      question: `Qu'est-ce qui PROUVERAIT que "${statement}" est FAUX?`,
      importance: 'critique',
    },
    {
      type: 'confirmation',
      question: `Qu'est-ce qui SOUTIENDRAIT que "${statement}" est vrai?`,
      importance: 'important',
    },
    {
      type: 'experiment',
      question: `Comment pourrais-tu TESTER "${statement}"?`,
      importance: 'pratique',
    },
    {
      type: 'alternative',
      question: `Quelle serait une hypothèse ALTERNATIVE à "${statement}"?`,
      importance: 'perspective',
    },
  ];
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize hypothesis testing
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    hypothesisState.hypotheses = saved.hypotheses || {};
    hypothesisState.stats = saved.stats || hypothesisState.stats;
  }
}

/**
 * Get active hypotheses
 *
 * @returns {Object[]} Active hypotheses
 */
function getActiveHypotheses() {
  return Object.values(hypothesisState.hypotheses);
}

/**
 * Get a specific hypothesis
 *
 * @param {string} id - Hypothesis ID
 * @returns {Object|null} Hypothesis
 */
function getHypothesis(id) {
  return hypothesisState.hypotheses[id] || null;
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...hypothesisState.stats,
    activeCount: Object.keys(hypothesisState.hypotheses).length,
    completedCount: hypothesisState.completed.length,
  };
}

/**
 * Format hypothesis for display
 *
 * @param {Object} hypothesis - Hypothesis
 * @returns {string} Formatted display
 */
function formatHypothesis(hypothesis) {
  const posteriorBar = '█'.repeat(Math.round(hypothesis.posterior * 10)) +
                       '░'.repeat(10 - Math.round(hypothesis.posterior * 10));

  const lines = [
    '── HYPOTHESIS ─────────────────────────────────────────────',
    `   "${hypothesis.statement}"`,
    '',
    `   P(H): [${posteriorBar}] ${Math.round(hypothesis.posterior * 100)}%`,
    `   Status: ${hypothesis.status.toUpperCase()}`,
    `   Experiments: ${hypothesis.experiments.length}/${MAX_EXPERIMENTS}`,
  ];

  if (hypothesis.falsificationCriteria.length > 0) {
    lines.push('');
    lines.push('   Falsification criteria:');
    for (const fc of hypothesis.falsificationCriteria) {
      const marker = fc.tested ? (fc.result ? '✓' : '✗') : '?';
      lines.push(`   ${marker} ${fc.criterion}`);
    }
  }

  return lines.join('\n');
}

/**
 * Format status for display
 *
 * @returns {string} Formatted status
 */
function formatStatus() {
  const stats = getStats();
  const active = getActiveHypotheses();

  const lines = [
    '── HYPOTHESIS TESTING ─────────────────────────────────────',
    `   Active: ${stats.activeCount}/${MAX_ACTIVE_HYPOTHESES}`,
    `   Confirmed: ${stats.confirmed}`,
    `   Falsified: ${stats.falsified}`,
    `   Inconclusive: ${stats.inconclusive}`,
    `   Experiments: ${stats.experimentsRun}`,
  ];

  if (active.length > 0) {
    lines.push('');
    lines.push('   Active hypotheses:');
    for (const h of active.slice(0, 3)) {
      lines.push(`   • ${h.statement.slice(0, 40)}... (${Math.round(h.posterior * 100)}%)`);
    }
  }

  return lines.join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  MAX_ACTIVE_HYPOTHESES,
  DEFAULT_PRIOR,
  FALSIFICATION_THRESHOLD,
  CONFIRMATION_THRESHOLD,
  MAX_EXPERIMENTS,

  // Core functions
  init,
  getStats,
  getActiveHypotheses,
  getHypothesis,

  // Hypothesis management
  createHypothesis,
  addFalsificationCriterion,
  addConfirmationCriterion,
  concludeHypothesis,

  // Experiments
  proposeExperiment,
  recordResult,

  // Helpers
  generateHypothesisQuestions,

  // Display
  formatHypothesis,
  formatStatus,
};

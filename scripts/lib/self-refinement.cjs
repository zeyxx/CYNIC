#!/usr/bin/env node
/**
 * CYNIC Self-Refinement System (CommonJS)
 *
 * "φ distrusts φ" - CYNIC critiques even itself
 *
 * Local wrapper for the core refinement module.
 * Used by hooks for autonomous self-correction.
 *
 * @module scripts/lib/self-refinement
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// Constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const THRESHOLDS = {
  HOWL: 80,   // Excellent
  WAG: 50,    // Good
  GROWL: 30,  // Warning
  BARK: 0,    // Danger
};

// Critique types
const CRITIQUE_TYPES = {
  WEAK_AXIOM: 'weak_axiom',
  IMBALANCE: 'imbalance',
  BIAS_DETECTED: 'bias_detected',
  MISSING_CONTEXT: 'missing_context',
  OVERCONFIDENCE: 'overconfidence',
  THRESHOLD_EDGE: 'threshold_edge',
};

// =============================================================================
// UTILITIES
// =============================================================================

function getDataDir() {
  return path.join(process.cwd(), '.cynic');
}

function ensureDir(dir) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

function loadRefinementHistory() {
  const historyPath = path.join(getDataDir(), 'refinement-history.json');
  if (fs.existsSync(historyPath)) {
    try {
      return JSON.parse(fs.readFileSync(historyPath, 'utf-8'));
    } catch (e) {
      return { refinements: [], stats: { total: 0, improved: 0 } };
    }
  }
  return { refinements: [], stats: { total: 0, improved: 0 } };
}

function saveRefinementHistory(history) {
  ensureDir(getDataDir());
  const historyPath = path.join(getDataDir(), 'refinement-history.json');
  fs.writeFileSync(historyPath, JSON.stringify(history, null, 2));
}

// =============================================================================
// CRITIQUE
// =============================================================================

/**
 * Critique a judgment result
 */
function critiqueJudgment(judgment, context = {}) {
  const critiques = [];
  const recommendations = [];

  const breakdown = judgment.breakdown || judgment.axiomScores || {};
  const Q = judgment.Q || judgment.qScore || judgment.q_score || 50;
  const verdict = judgment.verdict;

  if (!breakdown || Object.keys(breakdown).length === 0) {
    return {
      critiques: [{ type: CRITIQUE_TYPES.MISSING_CONTEXT, message: 'No axiom breakdown available' }],
      recommendations: ['Re-judge with explicit axiom scoring'],
      severity: 'high',
      refinable: false,
    };
  }

  // 1. Analyze axiom weaknesses
  const axiomScores = Object.entries(breakdown);
  const sorted = axiomScores.sort((a, b) => a[1] - b[1]);
  const weakest = sorted[0];
  const strongest = sorted[sorted.length - 1];
  const imbalance = strongest[1] - weakest[1];

  if (imbalance > 30) {
    critiques.push({
      type: CRITIQUE_TYPES.IMBALANCE,
      message: `Large imbalance: ${strongest[0]}(${strongest[1]}) vs ${weakest[0]}(${weakest[1]})`,
      data: { strongestAxiom: strongest[0], strongestScore: strongest[1], weakestAxiom: weakest[0], weakestScore: weakest[1], imbalance },
    });
    recommendations.push(`Re-evaluate ${weakest[0]} - may be underscored`);
  }

  if (weakest[1] < THRESHOLDS.GROWL) {
    critiques.push({
      type: CRITIQUE_TYPES.WEAK_AXIOM,
      message: `${weakest[0]} critically low at ${weakest[1]}`,
      data: { axiom: weakest[0], score: weakest[1] },
    });
    recommendations.push(`Consider if ${weakest[0]} reflects actual quality`);
  }

  // 2. Threshold edge cases
  const thresholdList = [THRESHOLDS.HOWL, THRESHOLDS.WAG, THRESHOLDS.GROWL];
  for (const threshold of thresholdList) {
    const distance = Math.abs(Q - threshold);
    if (distance < 3) {
      critiques.push({
        type: CRITIQUE_TYPES.THRESHOLD_EDGE,
        message: `Q-Score ${Q} within 3 points of threshold ${threshold}`,
        data: { Q, threshold, distance },
      });
      recommendations.push('Edge case: verify scores carefully');
      break;
    }
  }

  // 3. Confidence check
  const confidence = judgment.confidence || 0.5;
  if (confidence > PHI_INV) {
    critiques.push({
      type: CRITIQUE_TYPES.OVERCONFIDENCE,
      message: `Confidence ${(confidence * 100).toFixed(1)}% exceeds φ⁻¹ ceiling`,
      data: { confidence, ceiling: PHI_INV },
    });
    recommendations.push('Apply φ⁻¹ ceiling (61.8% max)');
  }

  // Calculate severity
  const hasHigh = critiques.some(c => c.type === CRITIQUE_TYPES.OVERCONFIDENCE || c.type === CRITIQUE_TYPES.WEAK_AXIOM);
  const hasMedium = critiques.some(c => c.type === CRITIQUE_TYPES.IMBALANCE || c.type === CRITIQUE_TYPES.THRESHOLD_EDGE);
  const severity = hasHigh ? 'high' : hasMedium ? 'medium' : critiques.length > 0 ? 'low' : 'none';

  // Refinable if has actionable critiques
  const refinable = critiques.some(c =>
    c.type === CRITIQUE_TYPES.WEAK_AXIOM ||
    c.type === CRITIQUE_TYPES.IMBALANCE ||
    c.type === CRITIQUE_TYPES.THRESHOLD_EDGE
  );

  return {
    critiques,
    recommendations,
    severity,
    refinable,
    originalQ: Q,
    originalVerdict: verdict,
  };
}

// =============================================================================
// REFINEMENT
// =============================================================================

/**
 * Suggest refined scores based on critique
 */
function suggestRefinement(originalJudgment, critique) {
  const breakdown = originalJudgment.breakdown || originalJudgment.axiomScores || {};
  const refined = { ...breakdown };
  const adjustments = [];

  for (const crit of critique.critiques) {
    switch (crit.type) {
      case CRITIQUE_TYPES.WEAK_AXIOM: {
        const axiom = crit.data.axiom;
        const currentScore = crit.data.score;
        const suggested = Math.min(currentScore + 10, THRESHOLDS.WAG);
        adjustments.push({ axiom, from: currentScore, to: suggested, reason: 'Potentially underscored' });
        refined[axiom] = suggested;
        break;
      }
      case CRITIQUE_TYPES.IMBALANCE: {
        const { strongestAxiom, strongestScore } = crit.data;
        if (strongestScore > 85) {
          const suggested = strongestScore - 5;
          adjustments.push({ axiom: strongestAxiom, from: strongestScore, to: suggested, reason: 'Very high - verify not over-rated' });
          refined[strongestAxiom] = suggested;
        }
        break;
      }
      case CRITIQUE_TYPES.OVERCONFIDENCE: {
        adjustments.push({ field: 'confidence', from: crit.data.confidence, to: PHI_INV, reason: 'Apply φ⁻¹ ceiling' });
        break;
      }
    }
  }

  // Calculate new Q
  const weights = { PHI: 0.382, VERIFY: 0.236, CULTURE: 0.236, BURN: 0.146 };
  let refinedQ = 0;
  for (const [axiom, score] of Object.entries(refined)) {
    refinedQ += (score * (weights[axiom] || 0.25));
  }
  refinedQ = Math.round(refinedQ);

  // Determine verdict
  const getVerdict = (q) => {
    if (q >= THRESHOLDS.HOWL) return 'HOWL';
    if (q >= THRESHOLDS.WAG) return 'WAG';
    if (q >= THRESHOLDS.GROWL) return 'GROWL';
    return 'BARK';
  };

  return {
    originalBreakdown: breakdown,
    refinedBreakdown: refined,
    adjustments,
    originalQ: critique.originalQ,
    refinedQ,
    improvement: refinedQ - critique.originalQ,
    newVerdict: getVerdict(refinedQ),
  };
}

// =============================================================================
// SELF-REFINEMENT LOOP
// =============================================================================

/**
 * Run complete self-refinement loop
 */
function selfRefine(judgment, context = {}, options = {}) {
  const { maxIterations = 3 } = options;
  const iterations = [];
  let currentJudgment = judgment;
  let improved = false;

  for (let i = 0; i < maxIterations; i++) {
    const critique = critiqueJudgment(currentJudgment, context);

    if (!critique.refinable) {
      iterations.push({ iteration: i + 1, critique, refinement: null, status: 'stable' });
      break;
    }

    const refinement = suggestRefinement(currentJudgment, critique);
    iterations.push({
      iteration: i + 1,
      critique,
      refinement,
      status: refinement.improvement > 0 ? 'improved' : 'no_improvement',
    });

    if (refinement.improvement <= 0) break;

    improved = true;
    currentJudgment = {
      ...currentJudgment,
      breakdown: refinement.refinedBreakdown,
      Q: refinement.refinedQ,
      qScore: refinement.refinedQ,
      verdict: refinement.newVerdict,
    };
  }

  const originalQ = judgment.Q || judgment.qScore || 50;
  // Use currentJudgment which accumulates all improvements
  const finalQ = currentJudgment.Q || currentJudgment.qScore || originalQ;
  const finalVerdict = currentJudgment.verdict || judgment.verdict;
  const finalBreakdown = currentJudgment.breakdown || judgment.breakdown || judgment.axiomScores;

  // Store in history
  const history = loadRefinementHistory();
  history.refinements.push({
    timestamp: Date.now(),
    originalQ,
    finalQ,
    improvement: finalQ - originalQ,
    improved,
    iterations: iterations.length,
  });
  history.stats.total++;
  if (improved) history.stats.improved++;

  // Keep only last 100 refinements
  if (history.refinements.length > 100) {
    history.refinements = history.refinements.slice(-100);
  }
  saveRefinementHistory(history);

  return {
    original: { Q: originalQ, verdict: judgment.verdict, breakdown: judgment.breakdown || judgment.axiomScores },
    final: { Q: finalQ, verdict: finalVerdict, breakdown: finalBreakdown },
    iterations,
    improved,
    totalImprovement: finalQ - originalQ,
    iterationCount: iterations.length,
  };
}

/**
 * Extract learning from refinement
 */
function extractLearning(refinementResult) {
  const learnings = [];

  for (const iter of refinementResult.iterations) {
    if (!iter.refinement) continue;
    for (const adj of iter.refinement.adjustments) {
      if (adj.axiom) {
        learnings.push({ type: 'axiom_adjustment', axiom: adj.axiom, pattern: adj.reason, adjustment: adj.to - adj.from });
      } else if (adj.field === 'confidence') {
        learnings.push({ type: 'confidence_calibration', pattern: 'Overconfidence detected', correction: 'Apply φ⁻¹ ceiling' });
      }
    }
  }

  return {
    learnings,
    improved: refinementResult.improved,
    improvementMagnitude: refinementResult.totalImprovement,
  };
}

/**
 * Get refinement statistics
 */
function getStats() {
  const history = loadRefinementHistory();
  const avgImprovement = history.refinements.length > 0
    ? history.refinements.reduce((sum, r) => sum + r.improvement, 0) / history.refinements.length
    : 0;

  return {
    total: history.stats.total,
    improved: history.stats.improved,
    improvementRate: history.stats.total > 0 ? history.stats.improved / history.stats.total : 0,
    avgImprovement,
    recentRefinements: history.refinements.slice(-5),
  };
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  critiqueJudgment,
  suggestRefinement,
  selfRefine,
  extractLearning,
  getStats,
  loadRefinementHistory,
  CRITIQUE_TYPES,
  THRESHOLDS,
  PHI_INV,
};

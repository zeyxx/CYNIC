/**
 * Bayesian Inference for CYNIC
 *
 * "Mettre à jour les croyances avec l'évidence" - κυνικός
 *
 * P(H|E) = P(E|H) × P(H) / P(E)
 *
 * posterior = likelihood × prior / marginal
 *
 * Uses Bayesian inference for:
 * - Belief updating based on new evidence
 * - Pattern confidence refinement
 * - Hypothesis testing with multiple candidates
 * - Anomaly detection with priors
 * - Dog consensus with weighted beliefs
 *
 * φ-aligned: All posteriors capped at 61.8%
 *
 * @module @cynic/node/inference/bayes
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * φ-aligned Bayesian configuration
 */
export const BAYES_CONFIG = {
  // Prior bounds (φ-aligned)
  MIN_PRIOR: PHI_INV_3,        // 23.6% - minimum prior probability
  DEFAULT_PRIOR: 0.5,          // 50% - uninformative prior
  MAX_PRIOR: PHI_INV,          // 61.8% - maximum prior (φ⁻¹)

  // Posterior bounds
  MAX_POSTERIOR: PHI_INV,      // 61.8% - never exceed φ⁻¹
  MIN_POSTERIOR: 0.01,         // 1% - never completely rule out

  // Evidence thresholds
  STRONG_EVIDENCE: PHI_INV,    // 61.8% - strong evidence threshold
  MODERATE_EVIDENCE: PHI_INV_2, // 38.2% - moderate evidence
  WEAK_EVIDENCE: PHI_INV_3,    // 23.6% - weak evidence

  // Learning rates (φ-aligned)
  LEARNING_RATE: PHI_INV_3,    // 23.6% - how fast to update beliefs
  DECAY_RATE: PHI_INV_3 * 0.1, // 2.36% - how fast beliefs decay

  // Beta distribution defaults (for conjugate priors)
  BETA_ALPHA_INIT: 1,          // Initial successes (uniform prior)
  BETA_BETA_INIT: 1,           // Initial failures (uniform prior)

  // Numerical stability
  EPSILON: 1e-10,              // Prevent division by zero
};

// ═══════════════════════════════════════════════════════════════════════════════
// CORE BAYES THEOREM
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Apply Bayes theorem: P(H|E) = P(E|H) × P(H) / P(E)
 *
 * @param {number} likelihood - P(E|H) - probability of evidence given hypothesis
 * @param {number} prior - P(H) - prior probability of hypothesis
 * @param {number} marginal - P(E) - probability of evidence (can be computed)
 * @returns {number} Posterior probability P(H|E), capped at φ⁻¹
 */
export function bayesTheorem(likelihood, prior, marginal) {
  if (marginal < BAYES_CONFIG.EPSILON) {
    return prior; // No evidence, return prior
  }

  const posterior = (likelihood * prior) / marginal;

  // φ-bound the posterior
  return Math.min(BAYES_CONFIG.MAX_POSTERIOR, Math.max(BAYES_CONFIG.MIN_POSTERIOR, posterior));
}

/**
 * Compute marginal probability P(E) from all hypotheses
 * P(E) = Σ P(E|Hi) × P(Hi)
 *
 * @param {Object[]} hypotheses - Array of {likelihood, prior} objects
 * @returns {number} Marginal probability
 */
export function computeMarginal(hypotheses) {
  return hypotheses.reduce(
    (sum, h) => sum + (h.likelihood * h.prior),
    BAYES_CONFIG.EPSILON
  );
}

/**
 * Update belief with new evidence (single hypothesis)
 *
 * @param {number} currentBelief - Current probability belief
 * @param {number} likelihood - P(E|H) for the new evidence
 * @param {number} baserate - P(E) base rate of evidence
 * @returns {number} Updated belief
 */
export function updateBelief(currentBelief, likelihood, baserate) {
  return bayesTheorem(likelihood, currentBelief, baserate);
}

/**
 * Batch update belief with multiple pieces of evidence
 * Applies sequential Bayesian updates
 *
 * @param {number} initialBelief - Starting probability
 * @param {Object[]} evidences - Array of {likelihood, baserate} objects
 * @returns {number} Final updated belief
 */
export function batchUpdateBelief(initialBelief, evidences) {
  let belief = initialBelief;

  for (const evidence of evidences) {
    belief = updateBelief(belief, evidence.likelihood, evidence.baserate);
  }

  return belief;
}

// ═══════════════════════════════════════════════════════════════════════════════
// MULTI-HYPOTHESIS INFERENCE
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Hypothesis with probability and metadata
 */
export class Hypothesis {
  /**
   * @param {string} id - Unique identifier
   * @param {string} name - Human-readable name
   * @param {number} [prior=0.5] - Initial probability
   */
  constructor(id, name, prior = BAYES_CONFIG.DEFAULT_PRIOR) {
    this.id = id;
    this.name = name;
    this.prior = Math.min(BAYES_CONFIG.MAX_PRIOR, Math.max(BAYES_CONFIG.MIN_PRIOR, prior));
    this.posterior = this.prior;
    this.evidenceHistory = [];
    this.updatedAt = Date.now();
  }

  /**
   * Update with evidence
   * @param {number} likelihood - P(E|H)
   * @param {number} marginal - P(E)
   * @param {string} [evidenceId] - Optional evidence identifier
   * @returns {number} New posterior
   */
  update(likelihood, marginal, evidenceId) {
    const oldPosterior = this.posterior;
    this.posterior = bayesTheorem(likelihood, this.posterior, marginal);
    this.updatedAt = Date.now();

    this.evidenceHistory.push({
      evidenceId,
      likelihood,
      marginal,
      priorBefore: oldPosterior,
      posteriorAfter: this.posterior,
      timestamp: Date.now(),
    });

    // Keep history bounded
    if (this.evidenceHistory.length > 100) {
      this.evidenceHistory = this.evidenceHistory.slice(-100);
    }

    return this.posterior;
  }

  /**
   * Get confidence category (φ-aligned)
   * @returns {string} 'strong' | 'moderate' | 'weak' | 'uncertain'
   */
  getConfidenceCategory() {
    if (this.posterior >= BAYES_CONFIG.STRONG_EVIDENCE) return 'strong';
    if (this.posterior >= BAYES_CONFIG.MODERATE_EVIDENCE) return 'moderate';
    if (this.posterior >= BAYES_CONFIG.WEAK_EVIDENCE) return 'weak';
    return 'uncertain';
  }

  /**
   * Serialize to plain object
   * @returns {Object}
   */
  toJSON() {
    return {
      id: this.id,
      name: this.name,
      prior: this.prior,
      posterior: this.posterior,
      confidence: this.getConfidenceCategory(),
      evidenceCount: this.evidenceHistory.length,
      updatedAt: this.updatedAt,
    };
  }
}

/**
 * Bayesian hypothesis set - manages competing hypotheses
 */
export class HypothesisSet {
  /**
   * @param {string} name - Name of this hypothesis set
   */
  constructor(name = 'unnamed') {
    this.name = name;
    this.hypotheses = new Map();
    this.evidenceLog = [];
    this.createdAt = Date.now();
  }

  /**
   * Add a hypothesis
   * @param {string} id - Hypothesis ID
   * @param {string} name - Hypothesis name
   * @param {number} [prior] - Prior probability
   * @returns {Hypothesis} The created hypothesis
   */
  addHypothesis(id, name, prior) {
    const h = new Hypothesis(id, name, prior);
    this.hypotheses.set(id, h);
    this._normalizePriors();
    return h;
  }

  /**
   * Get hypothesis by ID
   * @param {string} id - Hypothesis ID
   * @returns {Hypothesis|undefined}
   */
  getHypothesis(id) {
    return this.hypotheses.get(id);
  }

  /**
   * Update all hypotheses with new evidence
   *
   * @param {Object} likelihoods - Map of hypothesis ID to likelihood P(E|Hi)
   * @param {string} [evidenceId] - Optional evidence identifier
   * @returns {Object} Updated posteriors
   */
  updateWithEvidence(likelihoods, evidenceId) {
    // Build array for marginal computation
    const hypothesesArray = Array.from(this.hypotheses.values()).map(h => ({
      id: h.id,
      likelihood: likelihoods[h.id] ?? 0.5, // Default to 0.5 if not specified
      prior: h.posterior, // Use current posterior as prior
    }));

    // Compute marginal P(E)
    const marginal = computeMarginal(hypothesesArray);

    // Update each hypothesis
    const results = {};
    for (const h of this.hypotheses.values()) {
      const likelihood = likelihoods[h.id] ?? 0.5;
      h.update(likelihood, marginal, evidenceId);
      results[h.id] = h.posterior;
    }

    // Log evidence
    this.evidenceLog.push({
      evidenceId,
      likelihoods,
      marginal,
      posteriors: { ...results },
      timestamp: Date.now(),
    });

    // Keep log bounded
    if (this.evidenceLog.length > 100) {
      this.evidenceLog = this.evidenceLog.slice(-100);
    }

    // Normalize posteriors to sum to 1 (if exhaustive)
    this._normalizePosteriors();

    return results;
  }

  /**
   * Get the most likely hypothesis
   * @returns {Hypothesis|null}
   */
  getMostLikely() {
    let best = null;
    let bestPosterior = -1;

    for (const h of this.hypotheses.values()) {
      if (h.posterior > bestPosterior) {
        bestPosterior = h.posterior;
        best = h;
      }
    }

    return best;
  }

  /**
   * Get all hypotheses sorted by posterior
   * @returns {Hypothesis[]}
   */
  getSorted() {
    return Array.from(this.hypotheses.values())
      .sort((a, b) => b.posterior - a.posterior);
  }

  /**
   * Check if a hypothesis is significantly more likely than others
   * Uses φ-ratio: most likely should be φ times more likely than second
   *
   * @returns {Object} {decisive: boolean, winner: Hypothesis|null, ratio: number}
   */
  isDecisive() {
    const sorted = this.getSorted();
    if (sorted.length < 2) {
      return { decisive: true, winner: sorted[0] || null, ratio: Infinity };
    }

    const ratio = sorted[0].posterior / (sorted[1].posterior + BAYES_CONFIG.EPSILON);
    const decisive = ratio >= (1 / PHI_INV); // ≈ 1.618

    return {
      decisive,
      winner: decisive ? sorted[0] : null,
      ratio,
      first: sorted[0],
      second: sorted[1],
    };
  }

  /**
   * Normalize priors to sum to 1
   * @private
   */
  _normalizePriors() {
    const total = Array.from(this.hypotheses.values())
      .reduce((sum, h) => sum + h.prior, 0);

    if (total > 0) {
      for (const h of this.hypotheses.values()) {
        h.prior = h.prior / total;
        h.posterior = h.prior;
      }
    }
  }

  /**
   * Normalize posteriors to sum to 1
   * @private
   */
  _normalizePosteriors() {
    const total = Array.from(this.hypotheses.values())
      .reduce((sum, h) => sum + h.posterior, 0);

    if (total > 0) {
      for (const h of this.hypotheses.values()) {
        h.posterior = h.posterior / total;
        // Still cap at φ⁻¹
        h.posterior = Math.min(BAYES_CONFIG.MAX_POSTERIOR, h.posterior);
      }
    }
  }

  /**
   * Get stats
   * @returns {Object}
   */
  getStats() {
    const sorted = this.getSorted();
    return {
      name: this.name,
      hypothesisCount: this.hypotheses.size,
      evidenceCount: this.evidenceLog.length,
      mostLikely: sorted[0]?.toJSON() || null,
      decisiveness: this.isDecisive(),
      createdAt: this.createdAt,
    };
  }

  /**
   * Serialize to plain object
   * @returns {Object}
   */
  toJSON() {
    return {
      name: this.name,
      hypotheses: this.getSorted().map(h => h.toJSON()),
      stats: this.getStats(),
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// BETA-BINOMIAL MODEL (CONJUGATE PRIOR)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Beta distribution for learning from binary outcomes (success/failure)
 *
 * Perfect for learning pattern reliability, Dog accuracy, etc.
 *
 * Beta(α, β) where:
 * - α = successes + 1
 * - β = failures + 1
 * - mean = α / (α + β)
 * - mode = (α - 1) / (α + β - 2) for α, β > 1
 */
export class BetaDistribution {
  /**
   * @param {number} [alpha=1] - Initial α (successes + 1)
   * @param {number} [beta=1] - Initial β (failures + 1)
   */
  constructor(alpha = BAYES_CONFIG.BETA_ALPHA_INIT, beta = BAYES_CONFIG.BETA_BETA_INIT) {
    this.alpha = alpha;
    this.beta = beta;
    this.observationHistory = [];
  }

  /**
   * Record a success
   * @returns {number} New mean
   */
  recordSuccess() {
    this.alpha += 1;
    this.observationHistory.push({ outcome: 'success', timestamp: Date.now() });
    return this.getMean();
  }

  /**
   * Record a failure
   * @returns {number} New mean
   */
  recordFailure() {
    this.beta += 1;
    this.observationHistory.push({ outcome: 'failure', timestamp: Date.now() });
    return this.getMean();
  }

  /**
   * Record outcome
   * @param {boolean} success - True for success, false for failure
   * @returns {number} New mean
   */
  record(success) {
    return success ? this.recordSuccess() : this.recordFailure();
  }

  /**
   * Record multiple outcomes at once
   * @param {number} successes - Number of successes
   * @param {number} failures - Number of failures
   * @returns {number} New mean
   */
  recordBatch(successes, failures) {
    this.alpha += successes;
    this.beta += failures;
    return this.getMean();
  }

  /**
   * Get mean (expected value)
   * E[X] = α / (α + β)
   * @returns {number}
   */
  getMean() {
    const mean = this.alpha / (this.alpha + this.beta);
    // φ-bound the mean
    return Math.min(BAYES_CONFIG.MAX_POSTERIOR, mean);
  }

  /**
   * Get mode (most likely value)
   * Mode = (α - 1) / (α + β - 2) for α, β > 1
   * @returns {number}
   */
  getMode() {
    if (this.alpha <= 1 || this.beta <= 1) {
      return this.getMean(); // Mode not well-defined, use mean
    }
    const mode = (this.alpha - 1) / (this.alpha + this.beta - 2);
    return Math.min(BAYES_CONFIG.MAX_POSTERIOR, mode);
  }

  /**
   * Get variance
   * Var[X] = αβ / [(α+β)²(α+β+1)]
   * @returns {number}
   */
  getVariance() {
    const ab = this.alpha + this.beta;
    return (this.alpha * this.beta) / (ab * ab * (ab + 1));
  }

  /**
   * Get standard deviation
   * @returns {number}
   */
  getStdDev() {
    return Math.sqrt(this.getVariance());
  }

  /**
   * Get confidence interval (approximate)
   * Uses Wilson score interval for better small-sample behavior
   *
   * @param {number} [z=1.96] - Z-score (1.96 for 95% CI)
   * @returns {Object} {lower, upper}
   */
  getConfidenceInterval(z = 1.96) {
    const n = this.alpha + this.beta - 2; // Total observations
    const p = (this.alpha - 1) / (n || 1);

    if (n <= 0) {
      return { lower: 0, upper: 1 };
    }

    // Wilson score interval
    const denominator = 1 + z * z / n;
    const center = (p + z * z / (2 * n)) / denominator;
    const margin = (z / denominator) * Math.sqrt(p * (1 - p) / n + z * z / (4 * n * n));

    return {
      lower: Math.max(0, center - margin),
      upper: Math.min(BAYES_CONFIG.MAX_POSTERIOR, center + margin),
    };
  }

  /**
   * Get strength of belief (how many observations)
   * @returns {number}
   */
  getStrength() {
    return this.alpha + this.beta - 2; // Subtract initial values
  }

  /**
   * Check if belief is confident (variance is low, observations are high)
   * @param {number} [minObservations=10] - Minimum observations for confidence
   * @param {number} [maxVariance=0.05] - Maximum acceptable variance
   * @returns {boolean}
   */
  isConfident(minObservations = 10, maxVariance = 0.05) {
    return this.getStrength() >= minObservations && this.getVariance() <= maxVariance;
  }

  /**
   * Decay the distribution (reduce certainty over time)
   * Moves α and β toward uniform prior
   *
   * @param {number} [decayRate=BAYES_CONFIG.DECAY_RATE] - How much to decay
   * @returns {number} New mean
   */
  decay(decayRate = BAYES_CONFIG.DECAY_RATE) {
    // Move toward uniform prior (α=1, β=1)
    this.alpha = 1 + (this.alpha - 1) * (1 - decayRate);
    this.beta = 1 + (this.beta - 1) * (1 - decayRate);

    return this.getMean();
  }

  /**
   * Reset to uniform prior
   */
  reset() {
    this.alpha = BAYES_CONFIG.BETA_ALPHA_INIT;
    this.beta = BAYES_CONFIG.BETA_BETA_INIT;
    this.observationHistory = [];
  }

  /**
   * Serialize to plain object
   * @returns {Object}
   */
  toJSON() {
    return {
      alpha: this.alpha,
      beta: this.beta,
      mean: this.getMean(),
      mode: this.getMode(),
      variance: this.getVariance(),
      stdDev: this.getStdDev(),
      strength: this.getStrength(),
      confident: this.isConfident(),
      ci95: this.getConfidenceInterval(),
    };
  }

  /**
   * Create from serialized data
   * @param {Object} data - Serialized data
   * @returns {BetaDistribution}
   */
  static fromJSON(data) {
    return new BetaDistribution(data.alpha, data.beta);
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// NAIVE BAYES CLASSIFIER
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Naive Bayes Classifier for categorization
 *
 * P(class|features) ∝ P(class) × Π P(feature|class)
 *
 * Useful for:
 * - Pattern categorization
 * - Anomaly type detection
 * - User intent classification
 */
export class NaiveBayesClassifier {
  /**
   * @param {Object} [options={}]
   * @param {number} [options.smoothing=1] - Laplace smoothing factor
   */
  constructor(options = {}) {
    this.smoothing = options.smoothing ?? 1;
    this.classes = new Map(); // class -> {count, featureCounts}
    this.vocabulary = new Set(); // All seen features
    this.totalDocuments = 0;
  }

  /**
   * Train on a single example
   * @param {string[]} features - Feature array
   * @param {string} className - Class label
   */
  train(features, className) {
    if (!this.classes.has(className)) {
      this.classes.set(className, {
        count: 0,
        featureCounts: new Map(),
        totalFeatures: 0,
      });
    }

    const classData = this.classes.get(className);
    classData.count += 1;
    this.totalDocuments += 1;

    for (const feature of features) {
      this.vocabulary.add(feature);
      classData.totalFeatures += 1;
      classData.featureCounts.set(
        feature,
        (classData.featureCounts.get(feature) || 0) + 1
      );
    }
  }

  /**
   * Train on multiple examples
   * @param {Object[]} examples - Array of {features, class} objects
   */
  trainBatch(examples) {
    for (const example of examples) {
      this.train(example.features, example.class);
    }
  }

  /**
   * Predict class probabilities for features
   * @param {string[]} features - Feature array
   * @returns {Object[]} Array of {class, probability, logProbability} sorted by probability
   */
  predict(features) {
    const vocabSize = this.vocabulary.size;
    const results = [];

    for (const [className, classData] of this.classes) {
      // Prior P(class)
      const priorLog = Math.log(classData.count / this.totalDocuments);

      // Likelihood P(features|class) with Laplace smoothing
      let likelihoodLog = 0;
      for (const feature of features) {
        const featureCount = classData.featureCounts.get(feature) || 0;
        const probability = (featureCount + this.smoothing) /
          (classData.totalFeatures + this.smoothing * vocabSize);
        likelihoodLog += Math.log(probability);
      }

      const logProbability = priorLog + likelihoodLog;

      results.push({
        class: className,
        logProbability,
        probability: 0, // Will be normalized
      });
    }

    // Convert log probabilities to probabilities and normalize
    const maxLog = Math.max(...results.map(r => r.logProbability));
    let total = 0;

    for (const result of results) {
      result.probability = Math.exp(result.logProbability - maxLog);
      total += result.probability;
    }

    for (const result of results) {
      result.probability = result.probability / total;
      // φ-bound
      result.probability = Math.min(BAYES_CONFIG.MAX_POSTERIOR, result.probability);
    }

    // Sort by probability descending
    results.sort((a, b) => b.probability - a.probability);

    return results;
  }

  /**
   * Get the most likely class
   * @param {string[]} features - Feature array
   * @returns {Object} {class, probability, confidence}
   */
  classify(features) {
    const predictions = this.predict(features);
    if (predictions.length === 0) {
      return { class: null, probability: 0, confidence: 'uncertain' };
    }

    const best = predictions[0];
    let confidence;
    if (best.probability >= BAYES_CONFIG.STRONG_EVIDENCE) {
      confidence = 'strong';
    } else if (best.probability >= BAYES_CONFIG.MODERATE_EVIDENCE) {
      confidence = 'moderate';
    } else if (best.probability >= BAYES_CONFIG.WEAK_EVIDENCE) {
      confidence = 'weak';
    } else {
      confidence = 'uncertain';
    }

    return {
      class: best.class,
      probability: best.probability,
      confidence,
      alternatives: predictions.slice(1, 3),
    };
  }

  /**
   * Get classifier stats
   * @returns {Object}
   */
  getStats() {
    return {
      classes: Array.from(this.classes.keys()),
      classCount: this.classes.size,
      vocabularySize: this.vocabulary.size,
      totalDocuments: this.totalDocuments,
      classCounts: Object.fromEntries(
        Array.from(this.classes.entries()).map(([c, d]) => [c, d.count])
      ),
    };
  }

  /**
   * Serialize to plain object
   * @returns {Object}
   */
  toJSON() {
    const classesData = {};
    for (const [className, data] of this.classes) {
      classesData[className] = {
        count: data.count,
        featureCounts: Object.fromEntries(data.featureCounts),
        totalFeatures: data.totalFeatures,
      };
    }

    return {
      smoothing: this.smoothing,
      classes: classesData,
      vocabulary: Array.from(this.vocabulary),
      totalDocuments: this.totalDocuments,
    };
  }

  /**
   * Create from serialized data
   * @param {Object} data - Serialized data
   * @returns {NaiveBayesClassifier}
   */
  static fromJSON(data) {
    const classifier = new NaiveBayesClassifier({ smoothing: data.smoothing });
    classifier.totalDocuments = data.totalDocuments;
    classifier.vocabulary = new Set(data.vocabulary);

    for (const [className, classData] of Object.entries(data.classes)) {
      classifier.classes.set(className, {
        count: classData.count,
        featureCounts: new Map(Object.entries(classData.featureCounts)),
        totalFeatures: classData.totalFeatures,
      });
    }

    return classifier;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// BELIEF NETWORK (SIMPLE BAYESIAN NETWORK)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Simple Bayesian Network node
 */
export class BeliefNode {
  /**
   * @param {string} id - Node identifier
   * @param {string} name - Human-readable name
   * @param {number} [prior=0.5] - Prior probability
   */
  constructor(id, name, prior = BAYES_CONFIG.DEFAULT_PRIOR) {
    this.id = id;
    this.name = name;
    this.prior = Math.min(BAYES_CONFIG.MAX_PRIOR, prior);
    this.belief = this.prior;
    this.parents = new Map();  // parent ID -> CPT entry
    this.children = new Set(); // child IDs
    this.observed = false;
    this.observedValue = null;
  }

  /**
   * Add a parent with conditional probability table entry
   * @param {string} parentId - Parent node ID
   * @param {number} probIfTrue - P(this|parent=true)
   * @param {number} probIfFalse - P(this|parent=false)
   */
  addParent(parentId, probIfTrue, probIfFalse) {
    this.parents.set(parentId, { probIfTrue, probIfFalse });
  }

  /**
   * Set this node as observed
   * @param {boolean} value - Observed value
   */
  observe(value) {
    this.observed = true;
    this.observedValue = value;
    this.belief = value ? 1.0 : 0.0;
  }

  /**
   * Clear observation
   */
  clearObservation() {
    this.observed = false;
    this.observedValue = null;
    this.belief = this.prior;
  }

  /**
   * Update belief based on parent beliefs
   * @param {Map<string, number>} parentBeliefs - Map of parent ID to belief
   */
  updateFromParents(parentBeliefs) {
    if (this.observed) return; // Don't update observed nodes

    if (this.parents.size === 0) {
      // No parents, use prior
      this.belief = this.prior;
      return;
    }

    // Compute expected value given parent beliefs
    // For each parent configuration, weight by its probability
    let expectedBelief = 0;

    // Simple case: single parent
    if (this.parents.size === 1) {
      const [parentId, cpt] = this.parents.entries().next().value;
      const parentBelief = parentBeliefs.get(parentId) ?? 0.5;

      expectedBelief =
        parentBelief * cpt.probIfTrue +
        (1 - parentBelief) * cpt.probIfFalse;
    } else {
      // Multiple parents: use noisy-OR approximation
      let probFalse = 1.0;
      for (const [parentId, cpt] of this.parents) {
        const parentBelief = parentBeliefs.get(parentId) ?? 0.5;
        // Probability this parent doesn't cause true
        const probNotCaused = 1 - (parentBelief * cpt.probIfTrue + (1 - parentBelief) * cpt.probIfFalse);
        probFalse *= probNotCaused;
      }
      expectedBelief = 1 - probFalse;
    }

    // φ-bound the belief
    this.belief = Math.min(BAYES_CONFIG.MAX_POSTERIOR, Math.max(BAYES_CONFIG.MIN_POSTERIOR, expectedBelief));
  }

  /**
   * Serialize to plain object
   * @returns {Object}
   */
  toJSON() {
    return {
      id: this.id,
      name: this.name,
      prior: this.prior,
      belief: this.belief,
      observed: this.observed,
      observedValue: this.observedValue,
      parentIds: Array.from(this.parents.keys()),
      childIds: Array.from(this.children),
    };
  }
}

/**
 * Simple Bayesian Belief Network
 */
export class BeliefNetwork {
  /**
   * @param {string} name - Network name
   */
  constructor(name = 'unnamed') {
    this.name = name;
    this.nodes = new Map();
  }

  /**
   * Add a node
   * @param {string} id - Node ID
   * @param {string} name - Node name
   * @param {number} [prior=0.5] - Prior probability
   * @returns {BeliefNode}
   */
  addNode(id, name, prior) {
    const node = new BeliefNode(id, name, prior);
    this.nodes.set(id, node);
    return node;
  }

  /**
   * Add an edge (parent-child relationship)
   * @param {string} parentId - Parent node ID
   * @param {string} childId - Child node ID
   * @param {number} probIfTrue - P(child|parent=true)
   * @param {number} probIfFalse - P(child|parent=false)
   */
  addEdge(parentId, childId, probIfTrue, probIfFalse) {
    const parent = this.nodes.get(parentId);
    const child = this.nodes.get(childId);

    if (!parent || !child) {
      throw new Error(`Node not found: ${!parent ? parentId : childId}`);
    }

    child.addParent(parentId, probIfTrue, probIfFalse);
    parent.children.add(childId);
  }

  /**
   * Set observation on a node
   * @param {string} nodeId - Node ID
   * @param {boolean} value - Observed value
   */
  observe(nodeId, value) {
    const node = this.nodes.get(nodeId);
    if (!node) throw new Error(`Node not found: ${nodeId}`);
    node.observe(value);
  }

  /**
   * Clear all observations
   */
  clearObservations() {
    for (const node of this.nodes.values()) {
      node.clearObservation();
    }
  }

  /**
   * Propagate beliefs through the network
   * Uses simple forward propagation (not full inference)
   *
   * @param {number} [iterations=3] - Number of propagation iterations
   * @returns {Map<string, number>} Final beliefs
   */
  propagate(iterations = 3) {
    // Topological sort for proper ordering
    const sorted = this._topologicalSort();

    for (let iter = 0; iter < iterations; iter++) {
      // Collect current beliefs
      const beliefs = new Map();
      for (const [id, node] of this.nodes) {
        beliefs.set(id, node.belief);
      }

      // Update in topological order
      for (const nodeId of sorted) {
        const node = this.nodes.get(nodeId);
        node.updateFromParents(beliefs);
      }
    }

    // Return final beliefs
    const results = new Map();
    for (const [id, node] of this.nodes) {
      results.set(id, node.belief);
    }
    return results;
  }

  /**
   * Query a node's belief
   * @param {string} nodeId - Node ID
   * @returns {number} Current belief
   */
  query(nodeId) {
    const node = this.nodes.get(nodeId);
    if (!node) throw new Error(`Node not found: ${nodeId}`);
    return node.belief;
  }

  /**
   * Topological sort of nodes
   * @private
   * @returns {string[]} Sorted node IDs
   */
  _topologicalSort() {
    const visited = new Set();
    const result = [];

    const visit = (nodeId) => {
      if (visited.has(nodeId)) return;
      visited.add(nodeId);

      const node = this.nodes.get(nodeId);
      if (!node) return;

      // Visit parents first
      for (const parentId of node.parents.keys()) {
        visit(parentId);
      }

      result.push(nodeId);
    };

    for (const nodeId of this.nodes.keys()) {
      visit(nodeId);
    }

    return result;
  }

  /**
   * Get network stats
   * @returns {Object}
   */
  getStats() {
    const beliefs = this.propagate(1);
    const sorted = Array.from(beliefs.entries())
      .map(([id, belief]) => ({ id, belief, name: this.nodes.get(id).name }))
      .sort((a, b) => b.belief - a.belief);

    return {
      name: this.name,
      nodeCount: this.nodes.size,
      observedNodes: Array.from(this.nodes.values()).filter(n => n.observed).length,
      topBeliefs: sorted.slice(0, 5),
    };
  }

  /**
   * Serialize to plain object
   * @returns {Object}
   */
  toJSON() {
    return {
      name: this.name,
      nodes: Array.from(this.nodes.values()).map(n => n.toJSON()),
      stats: this.getStats(),
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// UTILITY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Compute likelihood ratio (Bayes factor)
 * K = P(E|H1) / P(E|H0)
 *
 * @param {number} likelihoodH1 - P(E|H1)
 * @param {number} likelihoodH0 - P(E|H0)
 * @returns {Object} {ratio, interpretation}
 */
export function likelihoodRatio(likelihoodH1, likelihoodH0) {
  const ratio = likelihoodH1 / (likelihoodH0 + BAYES_CONFIG.EPSILON);

  let interpretation;
  if (ratio > 100) interpretation = 'decisive';
  else if (ratio > 30) interpretation = 'very_strong';
  else if (ratio > 10) interpretation = 'strong';
  else if (ratio > 3) interpretation = 'substantial';
  else if (ratio > 1) interpretation = 'anecdotal';
  else if (ratio === 1) interpretation = 'none';
  else if (ratio > 1 / 3) interpretation = 'anecdotal_against';
  else interpretation = 'against';

  return { ratio, interpretation };
}

/**
 * Compute odds from probability
 * odds = p / (1 - p)
 *
 * @param {number} probability
 * @returns {number}
 */
export function probabilityToOdds(probability) {
  const p = Math.min(0.999, Math.max(0.001, probability));
  return p / (1 - p);
}

/**
 * Compute probability from odds
 * p = odds / (1 + odds)
 *
 * @param {number} odds
 * @returns {number}
 */
export function oddsToProbability(odds) {
  const p = odds / (1 + odds);
  return Math.min(BAYES_CONFIG.MAX_POSTERIOR, p);
}

/**
 * Compute log odds (logit)
 * logit(p) = log(p / (1 - p))
 *
 * @param {number} probability
 * @returns {number}
 */
export function logOdds(probability) {
  const p = Math.min(0.999, Math.max(0.001, probability));
  return Math.log(p / (1 - p));
}

/**
 * Compute probability from log odds (inverse logit / sigmoid)
 * p = 1 / (1 + e^(-logit))
 *
 * @param {number} logit
 * @returns {number}
 */
export function sigmoid(logit) {
  const p = 1 / (1 + Math.exp(-logit));
  return Math.min(BAYES_CONFIG.MAX_POSTERIOR, p);
}

/**
 * Update odds with likelihood ratio
 * posterior_odds = prior_odds × likelihood_ratio
 *
 * @param {number} priorOdds
 * @param {number} likelihoodRatioValue
 * @returns {number} Posterior odds
 */
export function updateOdds(priorOdds, likelihoodRatioValue) {
  return priorOdds * likelihoodRatioValue;
}

// ═══════════════════════════════════════════════════════════════════════════════
// FACTORY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Create a hypothesis set for competing hypotheses
 * @param {string} name - Set name
 * @param {Object[]} hypotheses - Array of {id, name, prior} objects
 * @returns {HypothesisSet}
 */
export function createHypothesisSet(name, hypotheses = []) {
  const set = new HypothesisSet(name);
  for (const h of hypotheses) {
    set.addHypothesis(h.id, h.name, h.prior);
  }
  return set;
}

/**
 * Create a beta distribution for tracking success rates
 * @param {number} [initialSuccesses=0] - Initial successes to record
 * @param {number} [initialFailures=0] - Initial failures to record
 * @returns {BetaDistribution}
 */
export function createBetaTracker(initialSuccesses = 0, initialFailures = 0) {
  const beta = new BetaDistribution();
  if (initialSuccesses > 0 || initialFailures > 0) {
    beta.recordBatch(initialSuccesses, initialFailures);
  }
  return beta;
}

/**
 * Create a Naive Bayes classifier
 * @param {Object} [options={}]
 * @returns {NaiveBayesClassifier}
 */
export function createClassifier(options = {}) {
  return new NaiveBayesClassifier(options);
}

/**
 * Create a belief network
 * @param {string} name - Network name
 * @returns {BeliefNetwork}
 */
export function createBeliefNetwork(name) {
  return new BeliefNetwork(name);
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════════

export default {
  // Core Bayes
  bayesTheorem,
  computeMarginal,
  updateBelief,
  batchUpdateBelief,

  // Multi-hypothesis
  Hypothesis,
  HypothesisSet,
  createHypothesisSet,

  // Beta-Binomial
  BetaDistribution,
  createBetaTracker,

  // Naive Bayes
  NaiveBayesClassifier,
  createClassifier,

  // Belief Networks
  BeliefNode,
  BeliefNetwork,
  createBeliefNetwork,

  // Utilities
  likelihoodRatio,
  probabilityToOdds,
  oddsToProbability,
  logOdds,
  sigmoid,
  updateOdds,

  // Config
  BAYES_CONFIG,
};

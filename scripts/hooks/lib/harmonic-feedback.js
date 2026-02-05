/**
 * Harmonic Feedback System - CYNIC's Unified Learning Framework
 *
 * Synthesizing:
 * - Kabbalah: Ohr/Kelim (Light/Vessels), Shevirah/Tikkun (Breaking/Repair)
 * - CIA Gateway Process: Resonance, Coherence, Entrainment
 * - Cybernetics: Feedback loops, Second-order observation
 * - Thompson Sampling: Bayesian exploration/exploitation
 *
 * "Le chien apprend par résonance, pas par force"
 *
 * @module scripts/hooks/lib/harmonic-feedback
 */

'use strict';

// ═══════════════════════════════════════════════════════════════════════════
// φ CONSTANTS - The Golden Foundation
// ═══════════════════════════════════════════════════════════════════════════

const PHI = 1.618033988749895;         // φ - The Golden Ratio
const PHI_INV = 0.618033988749895;     // φ⁻¹ = 61.8% - Max confidence
const PHI_INV_2 = 0.381966011250105;   // φ⁻² = 38.2%
const PHI_INV_3 = 0.236067977499790;   // φ⁻³ = 23.6%

// ═══════════════════════════════════════════════════════════════════════════
// KABBALISTIC FRAMEWORK
// "Ohr needs Kelim" - Light needs Vessels to be received
// ═══════════════════════════════════════════════════════════════════════════

/**
 * The Sefirot as feedback channels
 * Each maps to a different type of learning signal
 */
const SEFIROT_CHANNELS = Object.freeze({
  KETER: {
    name: 'Crown',
    domain: 'orchestration',
    signalWeight: PHI,        // Highest weight - strategic decisions
    decayRate: PHI_INV_3,     // Slowest decay - long-term memory
  },
  CHOCHMAH: {
    name: 'Wisdom',
    domain: 'insight',
    signalWeight: PHI_INV,
    decayRate: PHI_INV_2,
  },
  BINAH: {
    name: 'Understanding',
    domain: 'analysis',
    signalWeight: PHI_INV,
    decayRate: PHI_INV_2,
  },
  CHESED: {
    name: 'Kindness',
    domain: 'creation',
    signalWeight: PHI_INV_2,
    decayRate: PHI_INV_2,
  },
  GEVURAH: {
    name: 'Judgment',
    domain: 'protection',
    signalWeight: PHI,        // High weight - safety critical
    decayRate: PHI_INV_3,     // Slow decay - remember dangers
  },
  TIFERET: {
    name: 'Beauty',
    domain: 'harmony',
    signalWeight: PHI_INV,
    decayRate: PHI_INV_2,
  },
  NETZACH: {
    name: 'Victory',
    domain: 'persistence',
    signalWeight: PHI_INV_2,
    decayRate: PHI_INV,
  },
  HOD: {
    name: 'Glory',
    domain: 'acknowledgment',
    signalWeight: PHI_INV_2,
    decayRate: PHI_INV,
  },
  YESOD: {
    name: 'Foundation',
    domain: 'connection',
    signalWeight: PHI_INV_2,
    decayRate: PHI_INV_2,
  },
  MALKHUT: {
    name: 'Kingdom',
    domain: 'manifestation',
    signalWeight: 1.0,        // Base weight - concrete actions
    decayRate: PHI_INV,       // Faster decay - recent matters most
  },
});

/**
 * Feedback states in Kabbalistic terms
 */
const FeedbackState = Object.freeze({
  // Ohr (Light) states - suggestion flow
  OHR_YASHAR: 'direct_light',      // Direct suggestion, user follows
  OHR_CHOZER: 'returning_light',   // User acts, creates reflection

  // Kelim (Vessel) states - reception
  KELIM_MEKABEL: 'receiving',      // User is receptive
  KELIM_DOCHEH: 'rejecting',       // User is not receptive

  // Shevirah (Breaking) - negative feedback
  SHEVIRAH: 'breaking',            // Suggestion fails/rejected

  // Tikkun (Repair) - learning/correction
  TIKKUN_ATZMI: 'self_repair',     // CYNIC corrects itself
  TIKKUN_OLAM: 'world_repair',     // User benefits from correction
});

// ═══════════════════════════════════════════════════════════════════════════
// GATEWAY PROCESS: RESONANCE MATHEMATICS
// From CIA Gateway Process: Coherence and Resonance
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate coherence between two signal series
 * Coherence = how well CYNIC's suggestions align with user's actual needs
 *
 * Based on: Σ(s × a) / √(Σs² × Σa²)
 *
 * @param {number[]} suggestions - Suggestion confidence scores
 * @param {number[]} actions - User action alignment scores
 * @returns {number} Coherence value (0-1)
 */
function calculateCoherence(suggestions, actions) {
  if (suggestions.length === 0 || actions.length === 0) return 0;
  if (suggestions.length !== actions.length) {
    // Pad shorter array
    const maxLen = Math.max(suggestions.length, actions.length);
    while (suggestions.length < maxLen) suggestions.push(0);
    while (actions.length < maxLen) actions.push(0);
  }

  let dotProduct = 0;
  let sumS2 = 0;
  let sumA2 = 0;

  for (let i = 0; i < suggestions.length; i++) {
    dotProduct += suggestions[i] * actions[i];
    sumS2 += suggestions[i] * suggestions[i];
    sumA2 += actions[i] * actions[i];
  }

  const denominator = Math.sqrt(sumS2 * sumA2);
  if (denominator === 0) return 0;

  return Math.min(dotProduct / denominator, PHI_INV); // Cap at φ⁻¹
}

/**
 * Calculate resonance between suggestion and action vectors
 * Resonance = cos(θ) between suggestion intent and user action
 *
 * High resonance = CYNIC and user are "in sync"
 *
 * @param {Object} suggestion - Suggestion vector {type, confidence, urgency}
 * @param {Object} action - Action vector {tool, followed, timing}
 * @returns {number} Resonance value (-1 to 1)
 */
function calculateResonance(suggestion, action) {
  // Convert to comparable vectors
  const sVec = [
    suggestion.confidence || 0.5,
    suggestion.urgency || 0.5,
    suggestion.complexity || 0.5,
  ];

  const aVec = [
    action.followed ? 1 : 0,
    action.timing === 'immediate' ? 1 : (action.timing === 'delayed' ? 0.5 : 0),
    action.effort || 0.5,
  ];

  // Cosine similarity
  let dot = 0, magS = 0, magA = 0;
  for (let i = 0; i < 3; i++) {
    dot += sVec[i] * aVec[i];
    magS += sVec[i] * sVec[i];
    magA += aVec[i] * aVec[i];
  }

  const magnitude = Math.sqrt(magS) * Math.sqrt(magA);
  if (magnitude === 0) return 0;

  return dot / magnitude;
}

/**
 * Calculate entrainment rate
 * Entrainment = how quickly user syncs with CYNIC's rhythm
 *
 * From Gateway: "Hemi-sync" = hemispheric synchronization
 * Here: Suggestion-Action synchronization
 *
 * @param {Object[]} history - History of suggestion-action pairs
 * @returns {number} Entrainment rate (0-1)
 */
function calculateEntrainment(history) {
  if (history.length < 3) return 0.5; // Not enough data

  // Calculate lag between suggestion and following action
  const lags = history
    .filter(h => h.followed)
    .map(h => h.actionTime - h.suggestionTime);

  if (lags.length === 0) return 0;

  // Decreasing lag = increasing entrainment
  let entrainmentSum = 0;
  for (let i = 1; i < lags.length; i++) {
    const lagDecrease = lags[i - 1] - lags[i];
    entrainmentSum += lagDecrease > 0 ? 1 : (lagDecrease < 0 ? -1 : 0);
  }

  const entrainment = (entrainmentSum / (lags.length - 1) + 1) / 2; // Normalize to 0-1
  return Math.min(entrainment, PHI_INV);
}

// ═══════════════════════════════════════════════════════════════════════════
// CYBERNETIC FEEDBACK: FIRST AND SECOND ORDER
// From Wiener & von Foerster
// ═══════════════════════════════════════════════════════════════════════════

/**
 * First-order feedback: Direct observation → adjustment
 * Observer → System → Feedback → Adjust
 */
class FirstOrderFeedback {
  constructor() {
    this.observations = [];
    this.adjustments = [];
  }

  /**
   * Record an observation and compute adjustment
   * @param {Object} observation - What was observed
   * @param {Object} expected - What was expected
   * @returns {Object} Adjustment to make
   */
  observe(observation, expected) {
    const error = this._computeError(observation, expected);
    const adjustment = this._computeAdjustment(error);

    this.observations.push({ observation, expected, error, timestamp: Date.now() });
    this.adjustments.push(adjustment);

    // Keep bounded history
    if (this.observations.length > 100) {
      this.observations.shift();
      this.adjustments.shift();
    }

    return adjustment;
  }

  _computeError(observation, expected) {
    if (typeof observation === 'number' && typeof expected === 'number') {
      return expected - observation;
    }
    // Binary: did it match?
    if (observation.followed !== undefined) {
      return observation.followed === expected.followed ? 0 : 1;
    }
    return 0;
  }

  _computeAdjustment(error) {
    // Simple proportional adjustment with φ damping
    return {
      magnitude: Math.abs(error) * PHI_INV,
      direction: error > 0 ? 'increase' : 'decrease',
      confidence: Math.max(PHI_INV_3, PHI_INV - Math.abs(error)),
    };
  }
}

/**
 * Second-order feedback: Observer observes itself observing
 * The cybernetics of observing systems (von Foerster)
 */
class SecondOrderFeedback {
  constructor() {
    this.firstOrder = new FirstOrderFeedback();
    this.metaObservations = [];  // Observations about the observation process
    this.biases = [];            // Detected biases in our observations
    this.calibration = 1.0;      // How well-calibrated are our predictions
  }

  /**
   * Observe with meta-awareness
   * @param {Object} observation
   * @param {Object} expected
   * @param {Object} context - Context about the observation
   */
  observe(observation, expected, context = {}) {
    const firstOrderResult = this.firstOrder.observe(observation, expected);

    // Meta-observation: observe our prediction accuracy
    const predictionError = this._assessPredictionQuality(observation, expected);

    // Detect systematic biases
    this._updateBiases(observation, expected, context);

    // Update calibration
    this._updateCalibration(predictionError);

    this.metaObservations.push({
      firstOrder: firstOrderResult,
      predictionError,
      calibration: this.calibration,
      context,
      timestamp: Date.now(),
    });

    // Keep bounded
    if (this.metaObservations.length > 50) {
      this.metaObservations.shift();
    }

    return {
      adjustment: firstOrderResult,
      metaAdjustment: {
        calibrationFactor: this.calibration,
        detectedBiases: this.biases.slice(-3),
      },
    };
  }

  _assessPredictionQuality(observation, expected) {
    // How far off were we?
    if (observation.followed !== undefined && expected.probability !== undefined) {
      // Brier score: (prediction - outcome)²
      const outcome = observation.followed ? 1 : 0;
      return Math.pow(expected.probability - outcome, 2);
    }
    return 0;
  }

  _updateBiases(observation, expected, context) {
    // Detect optimism bias (we predict success too often)
    const recentObservations = this.firstOrder.observations.slice(-20);
    if (recentObservations.length >= 10) {
      const successPredictions = recentObservations.filter(o => o.expected.followed).length;
      const actualSuccesses = recentObservations.filter(o => o.observation.followed).length;

      if (successPredictions > actualSuccesses * 1.3) {
        this._recordBias('optimism', successPredictions / actualSuccesses);
      } else if (actualSuccesses > successPredictions * 1.3) {
        this._recordBias('pessimism', actualSuccesses / successPredictions);
      }
    }

    // Detect recency bias (we weight recent events too heavily)
    // This is actually intentional (temporal decay), but we track it
    const recentAdjustments = this.firstOrder.adjustments.slice(-10);
    const olderAdjustments = this.firstOrder.adjustments.slice(-20, -10);
    if (recentAdjustments.length >= 5 && olderAdjustments.length >= 5) {
      const recentMag = recentAdjustments.reduce((s, a) => s + a.magnitude, 0) / recentAdjustments.length;
      const olderMag = olderAdjustments.reduce((s, a) => s + a.magnitude, 0) / olderAdjustments.length;

      if (recentMag > olderMag * PHI) {
        this._recordBias('recency', recentMag / olderMag);
      }
    }
  }

  _recordBias(type, magnitude) {
    this.biases.push({ type, magnitude, timestamp: Date.now() });
    if (this.biases.length > 20) this.biases.shift();
  }

  _updateCalibration(brierScore) {
    // Perfect calibration = 0 Brier score
    // Adjust calibration factor based on recent accuracy
    const adjustment = (1 - brierScore) * PHI_INV_3;
    this.calibration = this.calibration * (1 - PHI_INV_3) + adjustment;
    this.calibration = Math.max(PHI_INV_3, Math.min(PHI_INV, this.calibration));
  }

  /**
   * Get meta-learning insights
   */
  getInsights() {
    return {
      calibration: this.calibration,
      recentBiases: this.biases.slice(-5),
      observationCount: this.firstOrder.observations.length,
      avgPredictionError: this._getAvgPredictionError(),
    };
  }

  _getAvgPredictionError() {
    if (this.metaObservations.length === 0) return 0;
    const sum = this.metaObservations.reduce((s, m) => s + m.predictionError, 0);
    return sum / this.metaObservations.length;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// THOMPSON SAMPLING: BAYESIAN EXPLORATION/EXPLOITATION
// "Le chien explore et exploite en harmonie"
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Thompson Sampling for suggestion selection
 * Uses Beta distribution for binary outcomes (followed/not followed)
 */
class ThompsonSampler {
  constructor() {
    // Beta distribution parameters for each suggestion type
    // Beta(α, β) where α = successes + 1, β = failures + 1
    this.arms = new Map();
    this.totalPulls = 0;
  }

  /**
   * Initialize or update an arm (suggestion type)
   * @param {string} armId - Suggestion type identifier
   * @param {number} [priorAlpha=1] - Prior successes
   * @param {number} [priorBeta=1] - Prior failures
   */
  initArm(armId, priorAlpha = 1, priorBeta = 1) {
    if (!this.arms.has(armId)) {
      this.arms.set(armId, {
        alpha: priorAlpha,
        beta: priorBeta,
        pulls: 0,
        lastPull: null,
      });
    }
  }

  /**
   * Sample from Beta distribution using Box-Muller transform approximation
   * @param {number} alpha
   * @param {number} beta
   * @returns {number} Sample value
   */
  _sampleBeta(alpha, beta) {
    // Use gamma sampling to get beta sample: Beta(α,β) = Gamma(α,1) / (Gamma(α,1) + Gamma(β,1))
    const gammaA = this._sampleGamma(alpha);
    const gammaB = this._sampleGamma(beta);
    return gammaA / (gammaA + gammaB);
  }

  /**
   * Sample from Gamma distribution (Marsaglia and Tsang's method)
   */
  _sampleGamma(shape) {
    if (shape < 1) {
      return this._sampleGamma(1 + shape) * Math.pow(Math.random(), 1 / shape);
    }
    const d = shape - 1/3;
    const c = 1 / Math.sqrt(9 * d);

    while (true) {
      let x, v;
      do {
        x = this._sampleNormal();
        v = 1 + c * x;
      } while (v <= 0);

      v = v * v * v;
      const u = Math.random();

      if (u < 1 - 0.0331 * (x * x) * (x * x)) return d * v;
      if (Math.log(u) < 0.5 * x * x + d * (1 - v + Math.log(v))) return d * v;
    }
  }

  _sampleNormal() {
    // Box-Muller transform
    const u1 = Math.random();
    const u2 = Math.random();
    return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
  }

  /**
   * Select best arm using Thompson Sampling
   * @param {string[]} availableArms - Arms to choose from
   * @returns {string} Selected arm ID
   */
  selectArm(availableArms = null) {
    const arms = availableArms || Array.from(this.arms.keys());
    if (arms.length === 0) return null;

    let bestArm = null;
    let bestSample = -1;

    for (const armId of arms) {
      this.initArm(armId); // Ensure exists
      const arm = this.arms.get(armId);

      // Sample from posterior
      const sample = this._sampleBeta(arm.alpha, arm.beta);

      // Cap at φ⁻¹ (CYNIC never fully certain)
      const cappedSample = Math.min(sample, PHI_INV);

      if (cappedSample > bestSample) {
        bestSample = cappedSample;
        bestArm = armId;
      }
    }

    return bestArm;
  }

  /**
   * Update arm with reward
   * @param {string} armId
   * @param {boolean} success - Whether suggestion was followed
   */
  update(armId, success) {
    this.initArm(armId);
    const arm = this.arms.get(armId);

    if (success) {
      arm.alpha += 1;
    } else {
      arm.beta += 1;
    }

    arm.pulls += 1;
    arm.lastPull = Date.now();
    this.totalPulls += 1;
  }

  /**
   * Get expected value for an arm
   * @param {string} armId
   * @returns {number} Expected success probability
   */
  getExpectedValue(armId) {
    const arm = this.arms.get(armId);
    if (!arm) return 0.5;

    // E[Beta(α,β)] = α / (α + β)
    const expected = arm.alpha / (arm.alpha + arm.beta);
    return Math.min(expected, PHI_INV);
  }

  /**
   * Get uncertainty for an arm
   * @param {string} armId
   * @returns {number} Uncertainty (variance)
   */
  getUncertainty(armId) {
    const arm = this.arms.get(armId);
    if (!arm) return 0.25; // Max uncertainty

    // Var[Beta(α,β)] = αβ / ((α+β)²(α+β+1))
    const sum = arm.alpha + arm.beta;
    const variance = (arm.alpha * arm.beta) / (sum * sum * (sum + 1));
    return variance;
  }

  /**
   * Get statistics about arms
   * @returns {Object} Stats including armCount, totalPulls, arms
   */
  getStats() {
    const arms = {};
    for (const [id, data] of this.arms) {
      arms[id] = {
        alpha: data.alpha,
        beta: data.beta,
        pulls: data.pulls,
        expectedValue: this.getExpectedValue(id),
      };
    }
    return {
      armCount: this.arms.size,
      totalPulls: this.totalPulls,
      arms,
    };
  }

  /**
   * Export state for persistence
   */
  exportState() {
    const armsData = {};
    for (const [id, data] of this.arms) {
      armsData[id] = data;
    }
    return { arms: armsData, totalPulls: this.totalPulls };
  }

  /**
   * Import state
   */
  importState(state) {
    if (state.arms) {
      this.arms = new Map(Object.entries(state.arms));
    }
    if (state.totalPulls) {
      this.totalPulls = state.totalPulls;
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// TEMPORAL DECAY: RECENCY WEIGHTING
// TD(λ) style exponential decay, weighted by φ
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate temporal decay weight
 * More recent events weighted higher
 *
 * @param {number} age - Age in milliseconds
 * @param {number} halfLife - Half-life in milliseconds (default: 30 min)
 * @returns {number} Weight (0-1)
 */
function temporalDecay(age, halfLife = 30 * 60 * 1000) {
  // Exponential decay: w(t) = e^(-λt) where λ = ln(2)/halfLife
  const lambda = Math.log(2) / halfLife;
  const weight = Math.exp(-lambda * age);

  // Scale by φ⁻¹ to never reach full weight
  return weight * PHI_INV;
}

/**
 * Calculate eligibility trace (TD(λ) style)
 * @param {Object[]} events - List of events with timestamps
 * @param {number} lambda - Decay parameter (0-1), default φ⁻¹
 * @returns {number[]} Eligibility weights for each event
 */
function eligibilityTrace(events, lambda = PHI_INV) {
  const now = Date.now();
  const traces = [];

  for (let i = 0; i < events.length; i++) {
    const age = now - events[i].timestamp;
    const recencyWeight = temporalDecay(age);
    const positionWeight = Math.pow(lambda, events.length - 1 - i);
    traces.push(recencyWeight * positionWeight);
  }

  // Normalize
  const sum = traces.reduce((s, t) => s + t, 0);
  if (sum === 0) return traces;
  return traces.map(t => t / sum);
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIDENCE CALIBRATION SYSTEM (Task #71)
// "φ distrusts φ" - CYNIC calibrates its own confidence
// Based on Brier scores and reliability diagrams
// ═══════════════════════════════════════════════════════════════════════════

const CALIBRATION_CONFIG = Object.freeze({
  BUCKET_COUNT: 10,              // 10 buckets for reliability diagram (0-10%, 10-20%, etc.)
  MIN_SAMPLES: 5,                // Minimum samples per bucket for calibration
  PERFECT_CALIBRATION: 0,        // Brier score for perfect calibration
  MAX_BRIER: 0.25,               // Worst case Brier score
  ADJUSTMENT_RATE: PHI_INV_3,    // How fast to adjust (23.6%)
  OVERCONFIDENCE_PENALTY: PHI_INV, // Penalty for being overconfident
});

/**
 * Confidence Calibration System
 * Ensures CYNIC's confidence estimates match actual outcomes
 *
 * Uses:
 * - Brier Score: Mean squared error between predictions and outcomes
 * - Reliability Diagram: Maps predicted probability → actual frequency
 * - Calibration Factor: Multiplier to adjust raw confidence
 */
class ConfidenceCalibrator {
  constructor() {
    // Reliability diagram buckets (0-10%, 10-20%, ..., 90-100%)
    this.buckets = Array(CALIBRATION_CONFIG.BUCKET_COUNT).fill(null).map(() => ({
      predictions: [],  // Predicted probabilities
      outcomes: [],     // Actual outcomes (0 or 1)
      count: 0,
    }));

    // Overall metrics
    this.metrics = {
      brierScore: 0.125,       // Start at expected random baseline
      calibrationError: 0,     // Average calibration error
      resolution: 0,           // Skill beyond base rate
      reliability: 0.5,        // Reliability index
      sharpness: 0,            // Confidence spread
      overconfidenceRate: 0,   // How often we're overconfident
      underconfidenceRate: 0,  // How often we're underconfident
    };

    // History for trend analysis
    this.history = [];

    // Calibration adjustment factor
    this.calibrationFactor = 1.0;
  }

  /**
   * Get bucket index for a predicted probability
   * @private
   */
  _getBucketIndex(probability) {
    const index = Math.floor(probability * CALIBRATION_CONFIG.BUCKET_COUNT);
    return Math.max(0, Math.min(CALIBRATION_CONFIG.BUCKET_COUNT - 1, index));
  }

  /**
   * Record a prediction and its outcome
   * @param {number} predictedProbability - Confidence (0-1)
   * @param {boolean} actualOutcome - What actually happened
   * @param {Object} context - Additional context
   */
  record(predictedProbability, actualOutcome, context = {}) {
    // Cap at φ⁻¹ (CYNIC never fully certain)
    const cappedPrediction = Math.min(predictedProbability, PHI_INV);
    const outcome = actualOutcome ? 1 : 0;

    // Add to appropriate bucket
    const bucketIndex = this._getBucketIndex(cappedPrediction);
    const bucket = this.buckets[bucketIndex];

    bucket.predictions.push(cappedPrediction);
    bucket.outcomes.push(outcome);
    bucket.count += 1;

    // Keep bounded
    if (bucket.predictions.length > 100) {
      bucket.predictions.shift();
      bucket.outcomes.shift();
    }

    // Record to history
    this.history.push({
      prediction: cappedPrediction,
      outcome,
      timestamp: Date.now(),
      context,
    });

    // Keep bounded history
    if (this.history.length > 500) {
      this.history.shift();
    }

    // Recalculate metrics
    this._updateMetrics();

    // Adjust calibration factor
    this._adjustCalibrationFactor(cappedPrediction, outcome);
  }

  /**
   * Update all calibration metrics
   * @private
   */
  _updateMetrics() {
    if (this.history.length < 10) return;

    // Calculate Brier Score
    this.metrics.brierScore = this._calculateBrierScore();

    // Calculate calibration error (reliability)
    const { calibrationError, reliability, overconfidence, underconfidence } = this._calculateCalibrationError();
    this.metrics.calibrationError = calibrationError;
    this.metrics.reliability = reliability;
    this.metrics.overconfidenceRate = overconfidence;
    this.metrics.underconfidenceRate = underconfidence;

    // Calculate resolution (skill)
    this.metrics.resolution = this._calculateResolution();

    // Calculate sharpness
    this.metrics.sharpness = this._calculateSharpness();
  }

  /**
   * Calculate Brier Score
   * BS = (1/N) Σ(prediction - outcome)²
   * Lower is better (0 = perfect)
   * @private
   */
  _calculateBrierScore() {
    if (this.history.length === 0) return CALIBRATION_CONFIG.MAX_BRIER;

    let sumSquaredError = 0;
    for (const record of this.history) {
      const error = record.prediction - record.outcome;
      sumSquaredError += error * error;
    }

    return sumSquaredError / this.history.length;
  }

  /**
   * Calculate calibration error from reliability diagram
   * Perfect calibration: predicted probability = actual frequency
   * @private
   */
  _calculateCalibrationError() {
    let totalError = 0;
    let totalCount = 0;
    let overconfidentBuckets = 0;
    let underconfidentBuckets = 0;
    let bucketsWithData = 0;

    for (let i = 0; i < CALIBRATION_CONFIG.BUCKET_COUNT; i++) {
      const bucket = this.buckets[i];
      if (bucket.count < CALIBRATION_CONFIG.MIN_SAMPLES) continue;

      bucketsWithData += 1;
      const avgPrediction = bucket.predictions.reduce((s, p) => s + p, 0) / bucket.count;
      const actualFrequency = bucket.outcomes.reduce((s, o) => s + o, 0) / bucket.count;

      const error = Math.abs(avgPrediction - actualFrequency);
      totalError += error * bucket.count;
      totalCount += bucket.count;

      // Track over/under confidence
      if (avgPrediction > actualFrequency + 0.1) {
        overconfidentBuckets += 1;
      } else if (avgPrediction < actualFrequency - 0.1) {
        underconfidentBuckets += 1;
      }
    }

    const calibrationError = totalCount > 0 ? totalError / totalCount : 0;
    const reliability = 1 - calibrationError;

    return {
      calibrationError,
      reliability,
      overconfidence: bucketsWithData > 0 ? overconfidentBuckets / bucketsWithData : 0,
      underconfidence: bucketsWithData > 0 ? underconfidentBuckets / bucketsWithData : 0,
    };
  }

  /**
   * Calculate resolution (skill beyond base rate)
   * @private
   */
  _calculateResolution() {
    if (this.history.length === 0) return 0;

    // Base rate
    const baseRate = this.history.reduce((s, r) => s + r.outcome, 0) / this.history.length;

    // Resolution = how much better than always predicting base rate
    let resolution = 0;
    for (let i = 0; i < CALIBRATION_CONFIG.BUCKET_COUNT; i++) {
      const bucket = this.buckets[i];
      if (bucket.count < CALIBRATION_CONFIG.MIN_SAMPLES) continue;

      const bucketFrequency = bucket.outcomes.reduce((s, o) => s + o, 0) / bucket.count;
      resolution += bucket.count * Math.pow(bucketFrequency - baseRate, 2);
    }

    return resolution / this.history.length;
  }

  /**
   * Calculate sharpness (how spread out predictions are)
   * Higher = more decisive predictions
   * @private
   */
  _calculateSharpness() {
    if (this.history.length === 0) return 0;

    const predictions = this.history.map(r => r.prediction);
    const mean = predictions.reduce((s, p) => s + p, 0) / predictions.length;

    let variance = 0;
    for (const p of predictions) {
      variance += Math.pow(p - mean, 2);
    }
    variance /= predictions.length;

    return Math.sqrt(variance);
  }

  /**
   * Adjust calibration factor based on recent performance
   * @private
   */
  _adjustCalibrationFactor(prediction, outcome) {
    // If overconfident (predicted high, got low), reduce factor
    // If underconfident (predicted low, got high), increase factor
    const error = prediction - outcome;

    if (error > 0.2) {
      // Overconfident - reduce calibration factor
      this.calibrationFactor *= (1 - CALIBRATION_CONFIG.ADJUSTMENT_RATE);
    } else if (error < -0.2) {
      // Underconfident - increase calibration factor
      this.calibrationFactor *= (1 + CALIBRATION_CONFIG.ADJUSTMENT_RATE);
    }

    // Keep factor bounded
    this.calibrationFactor = Math.max(PHI_INV_3, Math.min(PHI, this.calibrationFactor));
  }

  /**
   * Calibrate a raw confidence value
   * @param {number} rawConfidence - Original confidence estimate
   * @returns {number} Calibrated confidence
   */
  calibrate(rawConfidence) {
    // Apply calibration factor
    let calibrated = rawConfidence * this.calibrationFactor;

    // If we tend to be overconfident, apply additional penalty
    if (this.metrics.overconfidenceRate > 0.3) {
      calibrated *= (1 - CALIBRATION_CONFIG.OVERCONFIDENCE_PENALTY * this.metrics.overconfidenceRate);
    }

    // Never exceed φ⁻¹
    return Math.min(PHI_INV, Math.max(0, calibrated));
  }

  /**
   * Get reliability diagram data for visualization
   * @returns {Object[]} Bucket data
   */
  getReliabilityDiagram() {
    return this.buckets.map((bucket, i) => {
      const rangeStart = i / CALIBRATION_CONFIG.BUCKET_COUNT;
      const rangeEnd = (i + 1) / CALIBRATION_CONFIG.BUCKET_COUNT;

      if (bucket.count < CALIBRATION_CONFIG.MIN_SAMPLES) {
        return {
          range: `${Math.round(rangeStart * 100)}-${Math.round(rangeEnd * 100)}%`,
          avgPrediction: null,
          actualFrequency: null,
          count: bucket.count,
          calibrated: null,
        };
      }

      const avgPrediction = bucket.predictions.reduce((s, p) => s + p, 0) / bucket.count;
      const actualFrequency = bucket.outcomes.reduce((s, o) => s + o, 0) / bucket.count;

      return {
        range: `${Math.round(rangeStart * 100)}-${Math.round(rangeEnd * 100)}%`,
        avgPrediction,
        actualFrequency,
        count: bucket.count,
        calibrated: Math.abs(avgPrediction - actualFrequency) < 0.1,
      };
    });
  }

  /**
   * Get calibration recommendations
   * @returns {Object} Recommendations
   */
  getRecommendations() {
    const recommendations = [];

    if (this.metrics.overconfidenceRate > 0.3) {
      recommendations.push({
        type: 'overconfidence',
        severity: 'high',
        message: `Overconfident ${Math.round(this.metrics.overconfidenceRate * 100)}% of the time. Reduce confidence estimates.`,
      });
    }

    if (this.metrics.underconfidenceRate > 0.3) {
      recommendations.push({
        type: 'underconfidence',
        severity: 'medium',
        message: `Underconfident ${Math.round(this.metrics.underconfidenceRate * 100)}% of the time. Can be more decisive.`,
      });
    }

    if (this.metrics.sharpness < 0.1 && this.history.length > 50) {
      recommendations.push({
        type: 'low_sharpness',
        severity: 'low',
        message: 'Predictions cluster around 50%. More distinctive confidence would help.',
      });
    }

    if (this.metrics.brierScore > 0.2) {
      recommendations.push({
        type: 'poor_accuracy',
        severity: 'high',
        message: `Brier score ${this.metrics.brierScore.toFixed(3)} indicates poor prediction accuracy.`,
      });
    }

    return {
      recommendations,
      metrics: this.metrics,
      calibrationFactor: this.calibrationFactor,
      summary: recommendations.length === 0 ?
        'Calibration is within acceptable range.' :
        `${recommendations.length} calibration issue(s) detected.`,
    };
  }

  /**
   * Export state for persistence
   */
  exportState() {
    return {
      buckets: this.buckets,
      metrics: this.metrics,
      history: this.history.slice(-100), // Keep last 100
      calibrationFactor: this.calibrationFactor,
    };
  }

  /**
   * Import state
   */
  importState(data) {
    if (data.buckets) this.buckets = data.buckets;
    if (data.metrics) this.metrics = data.metrics;
    if (data.history) this.history = data.history;
    if (data.calibrationFactor) this.calibrationFactor = data.calibrationFactor;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// GIRSANOV MULTI-MEASURE SYSTEM (Task #23)
// "Change of measure transforms probability without changing reality"
//
// Girsanov theorem: dQ/dP = exp(θ·W - ½θ²t) (Radon-Nikodym derivative)
//
// Three parallel probability measures:
// - P (Prior): Raw confidence from Thompson Sampling
// - Q_risk: Risk-averse measure (pessimistic) - for dangerous commands
// - Q_opt: Optimistic measure - for routine tasks (still capped at φ⁻¹)
// ═══════════════════════════════════════════════════════════════════════════

const GIRSANOV_CONFIG = Object.freeze({
  // Drift parameters (θ) for measure transformation
  DRIFT_RISK: -0.3,           // Risk-averse: negative drift
  DRIFT_NEUTRAL: 0,           // Neutral: no drift
  DRIFT_OPTIMISTIC: 0.2,      // Optimistic: positive drift (but bounded)

  // Volatility parameter (σ) - uncertainty scaling
  VOLATILITY_BASE: 0.2,       // Base volatility

  // Time decay for drift intensity
  TIME_DECAY: PHI_INV_3,      // 23.6% per interval

  // Context-specific drift adjustments
  CONTEXT_DANGER: -0.5,       // Additional negative drift for dangerous contexts
  CONTEXT_ROUTINE: 0.1,       // Slight positive drift for routine tasks
  CONTEXT_NOVEL: -0.2,        // Negative drift for novel/unknown contexts
});

/**
 * GirsanovMeasureTransformer - Multi-scenario confidence system
 *
 * Maintains 3 parallel probability measures (worldviews) and transforms
 * confidence between them using Girsanov-style measure changes.
 *
 * "Le chien voit trois futurs possibles"
 */
class GirsanovMeasureTransformer {
  constructor() {
    // Three measures with their current state
    this.measures = {
      P: {    // Prior/Neutral measure
        name: 'Neutral',
        drift: GIRSANOV_CONFIG.DRIFT_NEUTRAL,
        history: [],
        brierScore: 0.125,
      },
      Q_risk: {  // Risk-averse measure
        name: 'Risk-Averse',
        drift: GIRSANOV_CONFIG.DRIFT_RISK,
        history: [],
        brierScore: 0.125,
      },
      Q_opt: {   // Optimistic measure
        name: 'Optimistic',
        drift: GIRSANOV_CONFIG.DRIFT_OPTIMISTIC,
        history: [],
        brierScore: 0.125,
      },
    };

    // Track which measure has been most accurate
    this.accuracyRanking = ['P', 'Q_risk', 'Q_opt'];
    this.totalObservations = 0;
    this.contextHistory = [];
  }

  /**
   * Calculate Radon-Nikodym derivative (dQ/dP)
   * This transforms probability from P to Q measure
   *
   * Using simplified Girsanov: dQ/dP = exp(θ·x - ½θ²)
   * where θ is drift and x is current state
   *
   * @param {number} drift - Drift parameter (θ)
   * @param {number} state - Current state value (0-1)
   * @returns {number} Likelihood ratio
   */
  _radonNikodym(drift, state) {
    // Simplified Girsanov transformation
    const theta = drift;
    const x = state - 0.5;  // Center at 0.5
    const likelihood = Math.exp(theta * x - 0.5 * theta * theta);
    return likelihood;
  }

  /**
   * Transform confidence from P measure to specified Q measure
   *
   * @param {number} pConfidence - Confidence under P measure (raw)
   * @param {string} targetMeasure - 'Q_risk' or 'Q_opt'
   * @param {Object} context - Context for adjustments
   * @returns {number} Transformed confidence under Q measure
   */
  transformConfidence(pConfidence, targetMeasure = 'Q_risk', context = {}) {
    const measure = this.measures[targetMeasure];
    if (!measure) return pConfidence;

    // Get base drift from measure
    let drift = measure.drift;

    // Adjust drift based on context
    if (context.isDangerous) {
      drift += GIRSANOV_CONFIG.CONTEXT_DANGER;
    }
    if (context.isRoutine) {
      drift += GIRSANOV_CONFIG.CONTEXT_ROUTINE;
    }
    if (context.isNovel) {
      drift += GIRSANOV_CONFIG.CONTEXT_NOVEL;
    }

    // Calculate Radon-Nikodym derivative
    const likelihood = this._radonNikodym(drift, pConfidence);

    // Transform confidence: Q(A) ∝ P(A) × dQ/dP
    // Normalize to [0, 1] range
    let qConfidence = pConfidence * likelihood;

    // Normalize using logistic transformation to keep in [0, 1]
    qConfidence = 1 / (1 + Math.exp(-(qConfidence * 4 - 2)));

    // Always cap at φ⁻¹ (CYNIC's axiom)
    return Math.min(PHI_INV, Math.max(0, qConfidence));
  }

  /**
   * Get confidence under all three measures
   *
   * @param {number} rawConfidence - Raw confidence from Thompson Sampling
   * @param {Object} context - Context for adjustments
   * @returns {Object} Confidence under each measure
   */
  getMultiMeasureConfidence(rawConfidence, context = {}) {
    const pConfidence = Math.min(PHI_INV, rawConfidence);  // Cap P at φ⁻¹

    return {
      P: pConfidence,
      Q_risk: this.transformConfidence(pConfidence, 'Q_risk', context),
      Q_opt: this.transformConfidence(pConfidence, 'Q_opt', context),
      range: {
        min: this.transformConfidence(pConfidence, 'Q_risk', context),
        mid: pConfidence,
        max: this.transformConfidence(pConfidence, 'Q_opt', context),
      },
      context,
      recommendation: this._recommendMeasure(context),
    };
  }

  /**
   * Recommend which measure to use based on context
   * @private
   */
  _recommendMeasure(context) {
    if (context.isDangerous) return 'Q_risk';
    if (context.isRoutine && !context.isNovel) return 'Q_opt';
    return 'P';
  }

  /**
   * Record outcome and update measure accuracy
   *
   * @param {number} predictedConfidence - What was predicted (under P)
   * @param {boolean} actualOutcome - What actually happened
   * @param {Object} context - Context of the prediction
   */
  recordOutcome(predictedConfidence, actualOutcome, context = {}) {
    const outcome = actualOutcome ? 1 : 0;
    const timestamp = Date.now();

    // Get predictions under all measures
    const multiMeasure = this.getMultiMeasureConfidence(predictedConfidence, context);

    // Calculate Brier score for each measure
    for (const measureName of ['P', 'Q_risk', 'Q_opt']) {
      const predicted = multiMeasure[measureName];
      const brierContrib = Math.pow(predicted - outcome, 2);

      const measure = this.measures[measureName];
      measure.history.push({
        predicted,
        outcome,
        brierContrib,
        timestamp,
      });

      // Keep bounded history
      if (measure.history.length > 100) {
        measure.history.shift();
      }

      // Update rolling Brier score
      const recentHistory = measure.history.slice(-50);
      measure.brierScore = recentHistory.length > 0
        ? recentHistory.reduce((s, h) => s + h.brierContrib, 0) / recentHistory.length
        : 0.125;
    }

    // Update accuracy ranking
    this._updateAccuracyRanking();

    // Record context
    this.contextHistory.push({ context, timestamp, outcome });
    if (this.contextHistory.length > 100) {
      this.contextHistory.shift();
    }

    this.totalObservations++;
  }

  /**
   * Update ranking of measures by accuracy
   * @private
   */
  _updateAccuracyRanking() {
    const scores = [
      { measure: 'P', brier: this.measures.P.brierScore },
      { measure: 'Q_risk', brier: this.measures.Q_risk.brierScore },
      { measure: 'Q_opt', brier: this.measures.Q_opt.brierScore },
    ];

    // Lower Brier score = better
    scores.sort((a, b) => a.brier - b.brier);
    this.accuracyRanking = scores.map(s => s.measure);
  }

  /**
   * Get the currently best-calibrated measure
   * @returns {string} Name of most accurate measure
   */
  getBestMeasure() {
    return this.accuracyRanking[0];
  }

  /**
   * Get adaptive confidence using best measure for context
   *
   * @param {number} rawConfidence - Raw confidence
   * @param {Object} context - Context
   * @returns {number} Best confidence estimate
   */
  getAdaptiveConfidence(rawConfidence, context = {}) {
    // If dangerous context, always use risk-averse regardless of accuracy
    if (context.isDangerous) {
      return this.transformConfidence(rawConfidence, 'Q_risk', context);
    }

    // Otherwise use best-performing measure
    const bestMeasure = this.getBestMeasure();
    if (bestMeasure === 'P') {
      return Math.min(PHI_INV, rawConfidence);
    }
    return this.transformConfidence(rawConfidence, bestMeasure, context);
  }

  /**
   * Get statistics about measure performance
   */
  getStats() {
    return {
      totalObservations: this.totalObservations,
      accuracyRanking: this.accuracyRanking,
      measures: {
        P: {
          brierScore: Math.round(this.measures.P.brierScore * 1000) / 1000,
          observations: this.measures.P.history.length,
        },
        Q_risk: {
          brierScore: Math.round(this.measures.Q_risk.brierScore * 1000) / 1000,
          observations: this.measures.Q_risk.history.length,
        },
        Q_opt: {
          brierScore: Math.round(this.measures.Q_opt.brierScore * 1000) / 1000,
          observations: this.measures.Q_opt.history.length,
        },
      },
      bestMeasure: this.getBestMeasure(),
      insight: this._generateInsight(),
    };
  }

  /**
   * Generate insight about measure performance
   * @private
   */
  _generateInsight() {
    const best = this.getBestMeasure();
    const bestBrier = this.measures[best].brierScore;

    if (bestBrier < 0.1) {
      return `*tail wag* ${this.measures[best].name} measure is well-calibrated (Brier: ${bestBrier.toFixed(3)})`;
    } else if (bestBrier < 0.2) {
      return `*sniff* ${this.measures[best].name} measure is adequately calibrated`;
    } else {
      return `*ears perk* All measures need more calibration data`;
    }
  }

  /**
   * Export state for persistence
   */
  exportState() {
    return {
      measures: {
        P: { history: this.measures.P.history.slice(-50), brierScore: this.measures.P.brierScore },
        Q_risk: { history: this.measures.Q_risk.history.slice(-50), brierScore: this.measures.Q_risk.brierScore },
        Q_opt: { history: this.measures.Q_opt.history.slice(-50), brierScore: this.measures.Q_opt.brierScore },
      },
      accuracyRanking: this.accuracyRanking,
      totalObservations: this.totalObservations,
    };
  }

  /**
   * Import state
   */
  importState(data) {
    if (data.measures) {
      for (const measureName of ['P', 'Q_risk', 'Q_opt']) {
        if (data.measures[measureName]) {
          this.measures[measureName].history = data.measures[measureName].history || [];
          this.measures[measureName].brierScore = data.measures[measureName].brierScore || 0.125;
        }
      }
    }
    if (data.accuracyRanking) this.accuracyRanking = data.accuracyRanking;
    if (data.totalObservations) this.totalObservations = data.totalObservations;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// ANTIFRAGILITY INDEX (Task #24)
// "Un système qui survit au hasard peut survivre à n'importe quoi"
//
// Antifragile systems benefit from volatility (convex response to stress)
// Fragile systems are hurt by volatility (concave response)
// Robust systems are unchanged by volatility (linear response)
//
// Antifragility Index = second derivative of performance vs volatility curve
//   > 0 = antifragile (gains from stress)
//   = 0 = robust (unchanged by stress)
//   < 0 = fragile (hurt by stress)
// ═══════════════════════════════════════════════════════════════════════════

const ANTIFRAGILITY_CONFIG = Object.freeze({
  // Volatility buckets
  BUCKET_COUNT: 5,                  // Low, Medium-Low, Medium, Medium-High, High
  MIN_SAMPLES_PER_BUCKET: 5,        // Minimum observations per bucket

  // Volatility calculation weights
  ERROR_WEIGHT: 0.4,                // Errors contribute 40% to volatility
  COMPLEXITY_WEIGHT: 0.3,           // Tool complexity contributes 30%
  UNCERTAINTY_WEIGHT: 0.3,          // Model uncertainty contributes 30%

  // Interpretation thresholds
  ANTIFRAGILE_THRESHOLD: 0.1,       // Index > 0.1 = clearly antifragile
  FRAGILE_THRESHOLD: -0.1,          // Index < -0.1 = clearly fragile

  // φ-aligned bounds
  MAX_ANTIFRAGILITY: PHI_INV,       // Can't be infinitely antifragile
  MIN_ANTIFRAGILITY: -PHI_INV,      // Can't be infinitely fragile
});

/**
 * AntifragilityTracker - Measures system response to volatility
 *
 * Based on Taleb's Antifragile: Things That Gain from Disorder
 * "Le chien devient plus fort par l'adversité"
 */
class AntifragilityTracker {
  constructor() {
    // Volatility buckets (0-20%, 20-40%, 40-60%, 60-80%, 80-100%)
    this.buckets = Array(ANTIFRAGILITY_CONFIG.BUCKET_COUNT).fill(null).map((_, i) => ({
      label: this._getBucketLabel(i),
      volatilityRange: [i * 0.2, (i + 1) * 0.2],
      performances: [],
      avgPerformance: null,
      count: 0,
    }));

    // Antifragility metrics
    this.metrics = {
      index: 0,                       // Current antifragility index
      trend: 'neutral',               // 'antifragile', 'fragile', 'robust'
      convexity: 0,                   // Second derivative (curvature)
      stressResponse: 0,              // How much performance changes with stress
      recoveryRate: 0,                // How fast system recovers from high volatility
    };

    // History for trend analysis
    this.history = [];
    this.totalObservations = 0;
  }

  /**
   * Get label for volatility bucket
   * @private
   */
  _getBucketLabel(index) {
    const labels = ['Low', 'Medium-Low', 'Medium', 'Medium-High', 'High'];
    return labels[index] || `Bucket ${index}`;
  }

  /**
   * Calculate volatility from context signals
   *
   * @param {Object} signals - Volatility signals
   * @param {number} signals.errorRate - Recent error rate (0-1)
   * @param {number} signals.complexity - Task complexity (0-1)
   * @param {number} signals.uncertainty - Model uncertainty (0-1)
   * @returns {number} Composite volatility (0-1)
   */
  calculateVolatility(signals) {
    const { errorRate = 0, complexity = 0.5, uncertainty = 0.5 } = signals;

    // Weighted average of volatility components
    const volatility =
      errorRate * ANTIFRAGILITY_CONFIG.ERROR_WEIGHT +
      complexity * ANTIFRAGILITY_CONFIG.COMPLEXITY_WEIGHT +
      uncertainty * ANTIFRAGILITY_CONFIG.UNCERTAINTY_WEIGHT;

    return Math.max(0, Math.min(1, volatility));
  }

  /**
   * Get bucket index for a volatility level
   * @private
   */
  _getBucketIndex(volatility) {
    const index = Math.floor(volatility * ANTIFRAGILITY_CONFIG.BUCKET_COUNT);
    return Math.max(0, Math.min(ANTIFRAGILITY_CONFIG.BUCKET_COUNT - 1, index));
  }

  /**
   * Record a performance observation under specific volatility
   *
   * @param {number} performance - Performance score (0-1)
   * @param {Object} volatilitySignals - Signals for volatility calculation
   */
  record(performance, volatilitySignals) {
    const volatility = this.calculateVolatility(volatilitySignals);
    const bucketIndex = this._getBucketIndex(volatility);
    const bucket = this.buckets[bucketIndex];

    // Record performance in bucket
    bucket.performances.push({
      performance,
      volatility,
      timestamp: Date.now(),
    });
    bucket.count += 1;

    // Keep bounded
    if (bucket.performances.length > 50) {
      bucket.performances.shift();
    }

    // Update bucket average
    bucket.avgPerformance = bucket.performances.reduce((s, p) => s + p.performance, 0) / bucket.performances.length;

    // Record to history
    this.history.push({
      volatility,
      performance,
      bucketIndex,
      timestamp: Date.now(),
    });

    // Keep bounded history
    if (this.history.length > 200) {
      this.history.shift();
    }

    this.totalObservations++;

    // Update metrics
    this._updateMetrics();
  }

  /**
   * Update antifragility metrics
   * @private
   */
  _updateMetrics() {
    // Need data in at least 3 buckets for meaningful analysis
    const bucketsWithData = this.buckets.filter(b =>
      b.count >= ANTIFRAGILITY_CONFIG.MIN_SAMPLES_PER_BUCKET
    );

    if (bucketsWithData.length < 3) {
      this.metrics.index = 0;
      this.metrics.trend = 'insufficient_data';
      return;
    }

    // Calculate convexity (second derivative approximation)
    this.metrics.convexity = this._calculateConvexity();

    // Calculate stress response (slope at high volatility)
    this.metrics.stressResponse = this._calculateStressResponse();

    // Calculate recovery rate
    this.metrics.recoveryRate = this._calculateRecoveryRate();

    // Calculate overall antifragility index
    // Combines convexity and stress response
    this.metrics.index = this._calculateAntifragilityIndex();

    // Determine trend
    if (this.metrics.index > ANTIFRAGILITY_CONFIG.ANTIFRAGILE_THRESHOLD) {
      this.metrics.trend = 'antifragile';
    } else if (this.metrics.index < ANTIFRAGILITY_CONFIG.FRAGILE_THRESHOLD) {
      this.metrics.trend = 'fragile';
    } else {
      this.metrics.trend = 'robust';
    }
  }

  /**
   * Calculate convexity (curvature) of performance vs volatility curve
   * Positive convexity = antifragile (smile curve)
   * Negative convexity = fragile (frown curve)
   * @private
   */
  _calculateConvexity() {
    // Get bucket averages
    const points = this.buckets
      .filter(b => b.avgPerformance !== null && b.count >= ANTIFRAGILITY_CONFIG.MIN_SAMPLES_PER_BUCKET)
      .map((b, i) => ({
        x: (b.volatilityRange[0] + b.volatilityRange[1]) / 2,  // Bucket midpoint
        y: b.avgPerformance,
      }));

    if (points.length < 3) return 0;

    // Calculate second derivative using finite differences
    // f''(x) ≈ (f(x+h) - 2f(x) + f(x-h)) / h²
    let convexitySum = 0;
    let convexityCount = 0;

    for (let i = 1; i < points.length - 1; i++) {
      const h = (points[i + 1].x - points[i - 1].x) / 2;
      if (h === 0) continue;

      const secondDerivative = (points[i + 1].y - 2 * points[i].y + points[i - 1].y) / (h * h);
      convexitySum += secondDerivative;
      convexityCount++;
    }

    const convexity = convexityCount > 0 ? convexitySum / convexityCount : 0;

    // Normalize to [-1, 1]
    return Math.max(-1, Math.min(1, convexity));
  }

  /**
   * Calculate stress response (how performance changes at high volatility)
   * @private
   */
  _calculateStressResponse() {
    const lowBucket = this.buckets[0];
    const highBucket = this.buckets[ANTIFRAGILITY_CONFIG.BUCKET_COUNT - 1];

    if (!lowBucket.avgPerformance || !highBucket.avgPerformance) return 0;

    // Positive = performance increases under stress
    // Negative = performance decreases under stress
    return highBucket.avgPerformance - lowBucket.avgPerformance;
  }

  /**
   * Calculate recovery rate from high volatility
   * @private
   */
  _calculateRecoveryRate() {
    // Look at transitions from high to lower volatility
    const transitions = [];

    for (let i = 1; i < this.history.length; i++) {
      const prev = this.history[i - 1];
      const curr = this.history[i];

      // If volatility decreased
      if (curr.volatility < prev.volatility - 0.2) {
        const performanceRecovery = curr.performance - prev.performance;
        transitions.push(performanceRecovery);
      }
    }

    if (transitions.length === 0) return 0;

    // Average recovery (positive = good recovery)
    const avgRecovery = transitions.reduce((s, t) => s + t, 0) / transitions.length;
    return Math.max(-1, Math.min(1, avgRecovery));
  }

  /**
   * Calculate overall antifragility index
   * @private
   */
  _calculateAntifragilityIndex() {
    // Combine convexity (60%), stress response (25%), recovery (15%)
    const raw =
      this.metrics.convexity * 0.6 +
      this.metrics.stressResponse * 0.25 +
      this.metrics.recoveryRate * 0.15;

    // Bound by φ limits
    return Math.max(
      ANTIFRAGILITY_CONFIG.MIN_ANTIFRAGILITY,
      Math.min(ANTIFRAGILITY_CONFIG.MAX_ANTIFRAGILITY, raw)
    );
  }

  /**
   * Get the performance curve data for visualization
   * @returns {Object[]} Points for volatility vs performance curve
   */
  getPerformanceCurve() {
    return this.buckets.map((b, i) => ({
      label: b.label,
      volatilityMidpoint: (b.volatilityRange[0] + b.volatilityRange[1]) / 2,
      avgPerformance: b.avgPerformance,
      sampleCount: b.count,
      hasSufficientData: b.count >= ANTIFRAGILITY_CONFIG.MIN_SAMPLES_PER_BUCKET,
    }));
  }

  /**
   * Get human-readable interpretation
   * @returns {Object} Interpretation
   */
  getInterpretation() {
    const { index, trend, convexity, stressResponse, recoveryRate } = this.metrics;

    let message, emoji, advice;

    if (trend === 'insufficient_data') {
      message = 'Pas assez de données pour évaluer l\'antifragilité';
      emoji = '*head tilt*';
      advice = 'Continuer à collecter des observations';
    } else if (trend === 'antifragile') {
      message = `CYNIC devient plus fort sous le stress (index: ${index.toFixed(3)})`;
      emoji = '*tail wag*';
      advice = 'Le système s\'améliore avec l\'adversité - excellent!';
    } else if (trend === 'fragile') {
      message = `CYNIC souffre sous le stress (index: ${index.toFixed(3)})`;
      emoji = '*GROWL*';
      advice = 'Considérer ajouter des mécanismes de résilience';
    } else {
      message = `CYNIC est robuste mais pas antifragile (index: ${index.toFixed(3)})`;
      emoji = '*sniff*';
      advice = 'Stable mais pourrait bénéficier de plus d\'apprentissage de l\'adversité';
    }

    return {
      trend,
      index,
      message: `${emoji} ${message}`,
      advice,
      details: {
        convexity: `Convexité: ${convexity.toFixed(3)} (${convexity > 0 ? 'smile' : convexity < 0 ? 'frown' : 'flat'})`,
        stressResponse: `Réponse au stress: ${stressResponse.toFixed(3)} (${stressResponse > 0 ? 'positive' : 'negative'})`,
        recoveryRate: `Récupération: ${recoveryRate.toFixed(3)}`,
      },
    };
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      totalObservations: this.totalObservations,
      bucketsWithData: this.buckets.filter(b => b.count >= ANTIFRAGILITY_CONFIG.MIN_SAMPLES_PER_BUCKET).length,
      metrics: this.metrics,
      curve: this.getPerformanceCurve(),
      interpretation: this.getInterpretation(),
    };
  }

  /**
   * Export state for persistence
   */
  exportState() {
    return {
      buckets: this.buckets.map(b => ({
        performances: b.performances.slice(-30),
        avgPerformance: b.avgPerformance,
        count: Math.min(b.count, 30),
      })),
      metrics: this.metrics,
      totalObservations: this.totalObservations,
    };
  }

  /**
   * Import state
   */
  importState(data) {
    if (data.buckets) {
      for (let i = 0; i < Math.min(data.buckets.length, ANTIFRAGILITY_CONFIG.BUCKET_COUNT); i++) {
        if (data.buckets[i]) {
          this.buckets[i].performances = data.buckets[i].performances || [];
          this.buckets[i].avgPerformance = data.buckets[i].avgPerformance;
          this.buckets[i].count = data.buckets[i].count || 0;
        }
      }
    }
    if (data.metrics) this.metrics = { ...this.metrics, ...data.metrics };
    if (data.totalObservations) this.totalObservations = data.totalObservations;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// NON-COMMUTATIVE EVALUATION (Task #25)
// "L'ordre d'évaluation change le résultat"
//
// Like quantum operators: [A,B] = AB - BA ≠ 0
// Order of dimension evaluation affects judgment outcomes
// Tracks "commutator strength" between dimension pairs
// ═══════════════════════════════════════════════════════════════════════════

const NONCOMMUTATIVE_CONFIG = Object.freeze({
  // Minimum observations to calculate commutator
  MIN_OBSERVATIONS: 10,

  // Commutator strength thresholds
  STRONG_NONCOMMUTATIVE: 0.2,     // |[A,B]| > 0.2 = strongly non-commutative
  WEAK_NONCOMMUTATIVE: 0.05,      // |[A,B]| > 0.05 = weakly non-commutative

  // Maximum tracked dimension pairs
  MAX_PAIRS: 50,

  // φ-derived decay for old observations
  OBSERVATION_DECAY: PHI_INV_3,
});

/**
 * NonCommutativeEvaluator - Tracks order-dependent evaluation effects
 *
 * In CYNIC's 25-dimension judgment system, the order of evaluation
 * can affect outcomes. This is like quantum mechanics where [A,B] ≠ 0.
 *
 * "Le chien sait que l'ordre importe"
 */
class NonCommutativeEvaluator {
  constructor() {
    // Commutator matrix: stores [A,B] = effect(A→B) - effect(B→A)
    // Key format: "dimA:dimB" (always alphabetically ordered)
    this.commutators = new Map();

    // Evaluation order history
    this.evaluationHistory = [];

    // Optimal evaluation order (learned)
    this.optimalOrder = [];

    // Metrics
    this.metrics = {
      avgNonCommutativity: 0,       // Average |[A,B]| across all pairs
      stronglyNonCommutative: 0,    // Count of strongly non-commutative pairs
      mostNonCommutativePair: null, // Pair with highest |[A,B]|
      orderSensitivity: 0,          // How much order affects overall judgments
    };

    this.totalEvaluations = 0;
  }

  /**
   * Get canonical key for dimension pair (alphabetically ordered)
   * @private
   */
  _getPairKey(dimA, dimB) {
    return dimA < dimB ? `${dimA}:${dimB}` : `${dimB}:${dimA}`;
  }

  /**
   * Record an evaluation with its order and outcome
   *
   * @param {Object} evaluation
   * @param {string[]} evaluation.order - Order of dimensions evaluated
   * @param {Object} evaluation.scores - Score per dimension
   * @param {number} evaluation.finalScore - Final judgment score
   * @param {string} evaluation.context - Context identifier
   */
  recordEvaluation(evaluation) {
    const { order, scores, finalScore, context } = evaluation;
    const timestamp = Date.now();

    // Store in history
    this.evaluationHistory.push({
      order: [...order],
      scores: { ...scores },
      finalScore,
      context,
      timestamp,
    });

    // Keep bounded
    if (this.evaluationHistory.length > 200) {
      this.evaluationHistory.shift();
    }

    // Update pairwise commutator estimates
    this._updateCommutators(order, scores, finalScore);

    this.totalEvaluations++;
    this._updateMetrics();
  }

  /**
   * Update commutator estimates from evaluation
   * @private
   */
  _updateCommutators(order, scores, finalScore) {
    // For each adjacent pair in the order, track the effect
    for (let i = 0; i < order.length - 1; i++) {
      const dimA = order[i];
      const dimB = order[i + 1];
      const key = this._getPairKey(dimA, dimB);

      if (!this.commutators.has(key)) {
        this.commutators.set(key, {
          dimA: dimA < dimB ? dimA : dimB,
          dimB: dimA < dimB ? dimB : dimA,
          abFirst: [],  // A evaluated before B
          baFirst: [],  // B evaluated before A
          commutator: 0,
        });
      }

      const data = this.commutators.get(key);

      // Calculate local effect (contribution to final score)
      const scoreA = scores[dimA] || 0;
      const scoreB = scores[dimB] || 0;
      const localEffect = (scoreA + scoreB) / 2;  // Simple average

      // Record based on which came first
      if (dimA === order[i]) {
        // A came before B
        data.abFirst.push({ effect: localEffect, finalScore, timestamp: Date.now() });
        if (data.abFirst.length > 30) data.abFirst.shift();
      } else {
        // B came before A
        data.baFirst.push({ effect: localEffect, finalScore, timestamp: Date.now() });
        if (data.baFirst.length > 30) data.baFirst.shift();
      }

      // Recalculate commutator
      this._recalculateCommutator(key);
    }

    // Limit total pairs
    if (this.commutators.size > NONCOMMUTATIVE_CONFIG.MAX_PAIRS) {
      // Remove least observed pairs
      const pairs = Array.from(this.commutators.entries())
        .sort((a, b) => (a[1].abFirst.length + a[1].baFirst.length) -
                       (b[1].abFirst.length + b[1].baFirst.length));
      const toRemove = pairs.slice(0, 10);
      for (const [key] of toRemove) {
        this.commutators.delete(key);
      }
    }
  }

  /**
   * Recalculate commutator [A,B] for a pair
   * @private
   */
  _recalculateCommutator(key) {
    const data = this.commutators.get(key);
    if (!data) return;

    // Need observations in both orders
    if (data.abFirst.length < 3 || data.baFirst.length < 3) {
      data.commutator = 0;
      return;
    }

    // Calculate weighted average effect for each order
    // Weight by recency
    const now = Date.now();
    const decay = NONCOMMUTATIVE_CONFIG.OBSERVATION_DECAY;

    const weightedAvg = (observations) => {
      let sumWeighted = 0;
      let sumWeights = 0;
      for (const obs of observations) {
        const age = (now - obs.timestamp) / (60 * 60 * 1000);  // Age in hours
        const weight = Math.exp(-decay * age);
        sumWeighted += obs.effect * weight;
        sumWeights += weight;
      }
      return sumWeights > 0 ? sumWeighted / sumWeights : 0;
    };

    const avgAB = weightedAvg(data.abFirst);
    const avgBA = weightedAvg(data.baFirst);

    // Commutator [A,B] = effect(A→B) - effect(B→A)
    data.commutator = avgAB - avgBA;
  }

  /**
   * Update overall metrics
   * @private
   */
  _updateMetrics() {
    const pairs = Array.from(this.commutators.values());

    if (pairs.length === 0) {
      this.metrics = {
        avgNonCommutativity: 0,
        stronglyNonCommutative: 0,
        mostNonCommutativePair: null,
        orderSensitivity: 0,
      };
      return;
    }

    // Calculate average |[A,B]|
    const absCommutators = pairs.map(p => Math.abs(p.commutator));
    this.metrics.avgNonCommutativity = absCommutators.reduce((s, c) => s + c, 0) / pairs.length;

    // Count strongly non-commutative pairs
    this.metrics.stronglyNonCommutative = pairs.filter(
      p => Math.abs(p.commutator) > NONCOMMUTATIVE_CONFIG.STRONG_NONCOMMUTATIVE
    ).length;

    // Find most non-commutative pair
    const maxPair = pairs.reduce((max, p) =>
      Math.abs(p.commutator) > Math.abs(max.commutator) ? p : max
    , pairs[0]);

    if (Math.abs(maxPair.commutator) > NONCOMMUTATIVE_CONFIG.WEAK_NONCOMMUTATIVE) {
      this.metrics.mostNonCommutativePair = {
        dimensions: [maxPair.dimA, maxPair.dimB],
        commutator: maxPair.commutator,
        interpretation: maxPair.commutator > 0
          ? `${maxPair.dimA} before ${maxPair.dimB} tends to increase score`
          : `${maxPair.dimB} before ${maxPair.dimA} tends to increase score`,
      };
    } else {
      this.metrics.mostNonCommutativePair = null;
    }

    // Overall order sensitivity
    this.metrics.orderSensitivity = Math.min(
      PHI_INV,
      this.metrics.avgNonCommutativity * 2  // Scale up for visibility
    );
  }

  /**
   * Get recommended evaluation order
   * Based on learned commutators, optimize order to maximize scores
   *
   * @param {string[]} dimensions - Dimensions to evaluate
   * @returns {string[]} Recommended order
   */
  getRecommendedOrder(dimensions) {
    if (dimensions.length <= 1) return [...dimensions];

    // Simple greedy: for each position, pick dimension that has positive
    // commutator when coming before remaining dimensions
    const result = [];
    const remaining = new Set(dimensions);

    while (remaining.size > 0) {
      let bestDim = null;
      let bestScore = -Infinity;

      for (const dim of remaining) {
        // Calculate expected benefit from putting this dimension next
        let score = 0;
        for (const other of remaining) {
          if (dim === other) continue;
          const key = this._getPairKey(dim, other);
          const data = this.commutators.get(key);
          if (data) {
            // Positive commutator means dim before other is better
            if (data.dimA === dim) {
              score += data.commutator;
            } else {
              score -= data.commutator;
            }
          }
        }
        if (score > bestScore) {
          bestScore = score;
          bestDim = dim;
        }
      }

      if (bestDim) {
        result.push(bestDim);
        remaining.delete(bestDim);
      } else {
        // Fallback: just take first remaining
        const first = remaining.values().next().value;
        result.push(first);
        remaining.delete(first);
      }
    }

    this.optimalOrder = result;
    return result;
  }

  /**
   * Get commutator between two dimensions
   * @param {string} dimA - First dimension
   * @param {string} dimB - Second dimension
   * @returns {number} Commutator [A,B]
   */
  getCommutator(dimA, dimB) {
    const key = this._getPairKey(dimA, dimB);
    const data = this.commutators.get(key);
    if (!data) return 0;

    // Return signed commutator (depends on query order)
    if (dimA === data.dimA) {
      return data.commutator;
    } else {
      return -data.commutator;
    }
  }

  /**
   * Get all dimension pairs that strongly don't commute
   * @returns {Object[]} Non-commutative pairs
   */
  getStronglyNonCommutativePairs() {
    return Array.from(this.commutators.values())
      .filter(p => Math.abs(p.commutator) > NONCOMMUTATIVE_CONFIG.STRONG_NONCOMMUTATIVE)
      .map(p => ({
        dimensions: [p.dimA, p.dimB],
        commutator: Math.round(p.commutator * 1000) / 1000,
        observations: p.abFirst.length + p.baFirst.length,
      }))
      .sort((a, b) => Math.abs(b.commutator) - Math.abs(a.commutator));
  }

  /**
   * Get human-readable interpretation
   * @returns {Object} Interpretation
   */
  getInterpretation() {
    const { avgNonCommutativity, stronglyNonCommutative, mostNonCommutativePair, orderSensitivity } = this.metrics;

    let message, emoji;

    if (this.totalEvaluations < NONCOMMUTATIVE_CONFIG.MIN_OBSERVATIONS) {
      message = 'Pas assez d\'évaluations pour détecter les effets d\'ordre';
      emoji = '*head tilt*';
    } else if (stronglyNonCommutative > 0) {
      message = `${stronglyNonCommutative} paire(s) de dimensions fortement non-commutative(s) détectée(s)`;
      emoji = '*ears perk*';
    } else if (avgNonCommutativity > NONCOMMUTATIVE_CONFIG.WEAK_NONCOMMUTATIVE) {
      message = 'L\'ordre d\'évaluation a un effet faible mais détectable';
      emoji = '*sniff*';
    } else {
      message = 'L\'ordre d\'évaluation n\'affecte pas significativement les jugements';
      emoji = '*yawn*';
    }

    return {
      message: `${emoji} ${message}`,
      orderSensitivity: `${Math.round(orderSensitivity * 100)}%`,
      strongPairs: this.getStronglyNonCommutativePairs().slice(0, 3),
      recommendation: mostNonCommutativePair
        ? `Considérer l'ordre: ${mostNonCommutativePair.interpretation}`
        : 'Pas de recommandation d\'ordre spécifique',
    };
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      totalEvaluations: this.totalEvaluations,
      trackedPairs: this.commutators.size,
      metrics: this.metrics,
      interpretation: this.getInterpretation(),
      optimalOrder: this.optimalOrder,
    };
  }

  /**
   * Export state for persistence
   */
  exportState() {
    const commutatorData = {};
    for (const [key, data] of this.commutators) {
      commutatorData[key] = {
        dimA: data.dimA,
        dimB: data.dimB,
        abFirst: data.abFirst.slice(-20),
        baFirst: data.baFirst.slice(-20),
        commutator: data.commutator,
      };
    }

    return {
      commutators: commutatorData,
      metrics: this.metrics,
      totalEvaluations: this.totalEvaluations,
      optimalOrder: this.optimalOrder,
    };
  }

  /**
   * Import state
   */
  importState(data) {
    if (data.commutators) {
      for (const [key, value] of Object.entries(data.commutators)) {
        this.commutators.set(key, {
          dimA: value.dimA,
          dimB: value.dimB,
          abFirst: value.abFirst || [],
          baFirst: value.baFirst || [],
          commutator: value.commutator || 0,
        });
      }
    }
    if (data.metrics) this.metrics = { ...this.metrics, ...data.metrics };
    if (data.totalEvaluations) this.totalEvaluations = data.totalEvaluations;
    if (data.optimalOrder) this.optimalOrder = data.optimalOrder;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIFIED HARMONIC FEEDBACK SYSTEM
// ═══════════════════════════════════════════════════════════════════════════

// Promotion thresholds (φ-derived)
const PROMOTION_CONFIG = Object.freeze({
  MIN_PULLS: 10,                    // Minimum observations before promotion
  MIN_EXPECTED_VALUE: PHI_INV_2,    // 38.2% - must exceed this
  MAX_UNCERTAINTY: PHI_INV_3,       // 23.6% - uncertainty must be below this
  MIN_CONFIDENCE: PHI_INV,          // 61.8% - promotion confidence threshold
  DEMOTION_THRESHOLD: PHI_INV_3,    // Below this = demote
  REVIEW_INTERVAL_MS: 5 * 60 * 1000, // Review every 5 minutes
});

class HarmonicFeedbackSystem {
  constructor() {
    // ═══════════════════════════════════════════════════════════════════════
    // FIL 2 (Task #84): Learning service callback
    // Allows wiring to external learning service without tight coupling
    // ═══════════════════════════════════════════════════════════════════════
    this.learningCallback = null;

    // Kabbalistic tracking
    this.sefirotSignals = new Map();
    for (const [key, config] of Object.entries(SEFIROT_CHANNELS)) {
      this.sefirotSignals.set(key, { signals: [], config });
    }

    // Cybernetic feedback
    this.secondOrderFeedback = new SecondOrderFeedback();

    // Thompson Sampling for suggestion selection
    this.thompsonSampler = new ThompsonSampler();

    // Confidence Calibration (Task #71)
    this.confidenceCalibrator = new ConfidenceCalibrator();

    // Girsanov Multi-Measure System (Task #23)
    // Three parallel probability measures for multi-scenario confidence
    this.girsanovTransformer = new GirsanovMeasureTransformer();

    // Antifragility Tracker (Task #24)
    // Measures system response to volatility
    this.antifragilityTracker = new AntifragilityTracker();

    // Non-Commutative Evaluator (Task #25)
    // Tracks order-dependent effects in dimension evaluation
    this.nonCommutativeEvaluator = new NonCommutativeEvaluator();

    // Gateway resonance tracking
    this.resonanceHistory = [];
    this.coherenceWindow = [];

    // Pattern-to-Heuristic Promotion System
    this.heuristics = new Map();     // Promoted patterns → rules
    this.promotionHistory = [];       // Track promotions/demotions
    this.lastReviewTime = Date.now();

    // ═══════════════════════════════════════════════════════════════════════
    // TEMPORAL PATTERN ANALYZER (Task #22)
    // FFT analysis of user behavior cycles
    // "Le chien détecte les rythmes cachés"
    // ═══════════════════════════════════════════════════════════════════════
    this.temporalAnalyzer = new TemporalPatternAnalyzer();
    this.lastTemporalUpdate = Date.now();
    this.temporalUpdateInterval = 5 * 60 * 1000; // Update every 5 minutes

    // Overall state
    this.state = {
      overallCoherence: 0.5,
      overallResonance: 0.5,
      entrainment: 0.5,
      tikkunProgress: 0,  // How much "repair" has been done
      shevirahCount: 0,   // How many "breakings" (failures)
      ohrFlow: 0.5,       // Current light flow intensity
      promotedHeuristics: 0, // Count of active heuristics
    };
  }

  /**
   * Set callback for learning service integration (Task #84: Fil 2)
   * @param {Function} callback - async (feedback) => void
   */
  setLearningCallback(callback) {
    this.learningCallback = callback;
  }

  /**
   * Record a suggestion (Ohr flowing down)
   * @param {Object} suggestion
   * @param {string} suggestion.type - Type of suggestion
   * @param {string} suggestion.sefirah - Which Sefirah channel
   * @param {number} suggestion.confidence
   * @param {Object} suggestion.context
   * @returns {string} Suggestion ID
   */
  recordSuggestion(suggestion) {
    const id = `sug_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`;

    const sefirah = suggestion.sefirah || 'MALKHUT';
    const channelData = this.sefirotSignals.get(sefirah);

    if (channelData) {
      channelData.signals.push({
        id,
        ...suggestion,
        timestamp: Date.now(),
        state: FeedbackState.OHR_YASHAR,
        resolved: false,
      });

      // Keep bounded
      if (channelData.signals.length > 50) {
        channelData.signals.shift();
      }
    }

    // Initialize Thompson arm
    this.thompsonSampler.initArm(suggestion.type);

    // Update Ohr flow
    this.state.ohrFlow = Math.min(PHI_INV, this.state.ohrFlow + 0.1);

    return id;
  }

  /**
   * Record user action (Kelim receiving or rejecting)
   * @param {Object} action
   * @param {string} action.suggestionId - Which suggestion this responds to
   * @param {boolean} action.followed - Was suggestion followed
   * @param {string} action.timing - 'immediate' | 'delayed' | 'ignored'
   * @param {Object} action.context
   */
  recordAction(action) {
    // Find the suggestion
    let suggestion = null;
    let sefirah = null;

    for (const [key, channelData] of this.sefirotSignals) {
      const found = channelData.signals.find(s => s.id === action.suggestionId);
      if (found) {
        suggestion = found;
        sefirah = key;
        break;
      }
    }

    if (!suggestion) {
      // No matching suggestion - record as standalone observation
      this._recordStandaloneAction(action);
      return;
    }

    // Determine feedback state
    const newState = action.followed ?
      FeedbackState.TIKKUN_OLAM :  // Repair - suggestion worked
      (action.timing === 'ignored' ?
        FeedbackState.KELIM_DOCHEH :  // Rejected
        FeedbackState.SHEVIRAH);      // Breaking - opposite action

    suggestion.state = newState;
    suggestion.resolved = true;
    suggestion.resolvedAt = Date.now();
    suggestion.action = action;

    // Update Thompson Sampling
    this.thompsonSampler.update(suggestion.type, action.followed);

    // Update Confidence Calibration
    this.confidenceCalibrator.record(
      suggestion.confidence || 0.5,
      action.followed,
      { type: suggestion.type, sefirah }
    );

    // Update Girsanov Multi-Measure (Task #23)
    // Determine context for measure selection
    const girsanovContext = {
      isDangerous: sefirah === 'GEVURAH',  // Protection channel = dangerous
      isRoutine: suggestion.type?.includes('routine') || false,
      isNovel: suggestion.type?.includes('novel') || false,
    };
    this.girsanovTransformer.recordOutcome(
      suggestion.confidence || 0.5,
      action.followed,
      girsanovContext
    );

    // Update Antifragility Tracker (Task #24)
    // Calculate volatility signals from current context
    const volatilitySignals = {
      errorRate: action.followed ? 0 : 1,  // This action's outcome
      complexity: suggestion.complexity || 0.5,
      uncertainty: this.thompsonSampler.getUncertainty(suggestion.type) * 4 || 0.5,  // Scale up
    };
    const performance = action.followed ? 1 : 0;
    this.antifragilityTracker.record(performance, volatilitySignals);

    // Update second-order feedback
    const cybResult = this.secondOrderFeedback.observe(
      { followed: action.followed, timing: action.timing },
      { followed: true, probability: suggestion.confidence },
      { sefirah, suggestionType: suggestion.type }
    );

    // Calculate resonance
    const resonance = calculateResonance(suggestion, action);
    this.resonanceHistory.push({
      suggestionId: action.suggestionId,
      resonance,
      timestamp: Date.now(),
      followed: action.followed,
      suggestionTime: suggestion.timestamp,
      actionTime: Date.now(),
    });

    // Keep bounded
    if (this.resonanceHistory.length > 100) {
      this.resonanceHistory.shift();
    }

    // Update coherence window
    this.coherenceWindow.push(action.followed ? 1 : 0);
    if (this.coherenceWindow.length > 20) {
      this.coherenceWindow.shift();
    }

    // Update state
    this._updateState(action, cybResult, resonance, sefirah);

    // ═══════════════════════════════════════════════════════════════════════
    // TEMPORAL PATTERN COLLECTION (Task #22)
    // Record time series data for FFT analysis
    // ═══════════════════════════════════════════════════════════════════════
    this.temporalAnalyzer.record('activity', 1.0);  // Activity event
    this.temporalAnalyzer.record('coherence', this.state.overallCoherence);
    this.temporalAnalyzer.record('resonance', Math.max(0, Math.min(1, (resonance + 1) / 2)));  // Normalize -1..1 to 0..1
    if (!action.followed) {
      this.temporalAnalyzer.record('errors', 1.0);  // Error/rejection event
    }

    // Periodic FFT analysis
    const now = Date.now();
    if (now - this.lastTemporalUpdate > this.temporalUpdateInterval) {
      this.lastTemporalUpdate = now;
      this.temporalAnalyzer.analyzeAll();
    }

    // Return feedback summary
    return this._generateFeedbackSummary(suggestion, action, resonance, cybResult);
  }

  _recordStandaloneAction(action) {
    // Action without a matching suggestion
    // Still valuable for learning general patterns
    this.secondOrderFeedback.observe(
      { followed: true, timing: action.timing },
      { followed: true, probability: 0.5 },
      { standalone: true }
    );

    // Record to temporal analyzer (Task #22)
    this.temporalAnalyzer.record('activity', 0.7);  // Standalone = slightly lower activity signal
  }

  _updateState(action, cybResult, resonance, sefirah) {
    const channelConfig = SEFIROT_CHANNELS[sefirah];
    const weight = channelConfig?.signalWeight || 1.0;

    // Update coherence (moving average)
    const recentCoherence = this.coherenceWindow.reduce((s, v) => s + v, 0) /
                           Math.max(1, this.coherenceWindow.length);
    this.state.overallCoherence = this.state.overallCoherence * (1 - PHI_INV_3) +
                                  recentCoherence * PHI_INV_3;

    // Update resonance (weighted by channel)
    this.state.overallResonance = this.state.overallResonance * (1 - PHI_INV_3 * weight) +
                                  resonance * PHI_INV_3 * weight;

    // Update entrainment
    this.state.entrainment = calculateEntrainment(this.resonanceHistory);

    // Update Tikkun/Shevirah counters
    if (action.followed) {
      this.state.tikkunProgress += weight * PHI_INV_3;
    } else {
      this.state.shevirahCount += 1;
    }

    // Decay Ohr flow
    this.state.ohrFlow = Math.max(PHI_INV_3, this.state.ohrFlow * (1 - PHI_INV_3));

    // Cap all values
    this.state.overallCoherence = Math.min(PHI_INV, this.state.overallCoherence);
    this.state.overallResonance = Math.min(PHI_INV, Math.max(-PHI_INV, this.state.overallResonance));
    this.state.entrainment = Math.min(PHI_INV, this.state.entrainment);
    this.state.tikkunProgress = Math.min(PHI, this.state.tikkunProgress);
  }

  _generateFeedbackSummary(suggestion, action, resonance, cybResult) {
    return {
      suggestionId: suggestion.id,
      type: suggestion.type,
      sefirah: suggestion.sefirah,
      followed: action.followed,
      resonance,
      calibration: cybResult.metaAdjustment.calibrationFactor,
      expectedValue: this.thompsonSampler.getExpectedValue(suggestion.type),
      uncertainty: this.thompsonSampler.getUncertainty(suggestion.type),
      state: this.state,
    };
  }

  /**
   * Get best suggestion type to use
   * Uses Thompson Sampling for exploration/exploitation
   */
  getBestSuggestionType(availableTypes = null) {
    return this.thompsonSampler.selectArm(availableTypes);
  }

  /**
   * Get system insights for meta-learning
   */
  getInsights() {
    const cyberInsights = this.secondOrderFeedback.getInsights();

    // Calculate per-sefirah statistics
    const sefirahStats = {};
    for (const [key, channelData] of this.sefirotSignals) {
      const signals = channelData.signals;
      const resolved = signals.filter(s => s.resolved);
      const followed = resolved.filter(s => s.action?.followed);

      sefirahStats[key] = {
        total: signals.length,
        resolved: resolved.length,
        followRate: resolved.length > 0 ? followed.length / resolved.length : 0,
        avgResonance: this._avgResonanceForSefirah(key),
      };
    }

    return {
      state: this.state,
      cybernetic: cyberInsights,
      sefirahStats,
      thompsonStats: this.thompsonSampler.exportState(),
      resonanceHistory: this.resonanceHistory.slice(-10),
      // Temporal analysis (Task #22)
      temporal: {
        lastAnalysis: this.temporalAnalyzer.lastAnalysis,
        optimalTiming: this.temporalAnalyzer.getOptimalTiming(),
        analysisCount: this.temporalAnalyzer.analysisCount,
      },
    };
  }

  /**
   * Get temporal pattern analysis
   * @returns {Object} FFT analysis of user behavior cycles
   */
  getTemporalAnalysis() {
    return this.temporalAnalyzer.analyzeAll();
  }

  /**
   * Get optimal timing recommendation based on detected cycles
   * @returns {Object} Timing recommendation
   */
  getOptimalTiming() {
    return this.temporalAnalyzer.getOptimalTiming();
  }

  _avgResonanceForSefirah(sefirah) {
    const relevant = this.resonanceHistory.filter(r => {
      const suggestion = this._findSuggestionById(r.suggestionId);
      return suggestion?.sefirah === sefirah;
    });

    if (relevant.length === 0) return 0;
    return relevant.reduce((s, r) => s + r.resonance, 0) / relevant.length;
  }

  _findSuggestionById(id) {
    for (const [, channelData] of this.sefirotSignals) {
      const found = channelData.signals.find(s => s.id === id);
      if (found) return found;
    }
    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PATTERN-TO-HEURISTIC PROMOTION SYSTEM
  // "Le chien apprend → Le chien sait" - Pattern becomes rule
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check if a pattern should be promoted to a heuristic
   * @param {string} patternId - Pattern/arm identifier
   * @returns {Object} Promotion assessment
   */
  assessPromotion(patternId) {
    const arm = this.thompsonSampler.arms.get(patternId);
    if (!arm) {
      return { eligible: false, reason: 'Pattern not found' };
    }

    const expectedValue = this.thompsonSampler.getExpectedValue(patternId);
    const uncertainty = this.thompsonSampler.getUncertainty(patternId);

    // Check eligibility criteria
    const checks = {
      minPulls: arm.pulls >= PROMOTION_CONFIG.MIN_PULLS,
      minExpected: expectedValue >= PROMOTION_CONFIG.MIN_EXPECTED_VALUE,
      maxUncertainty: uncertainty <= PROMOTION_CONFIG.MAX_UNCERTAINTY,
    };

    const eligible = checks.minPulls && checks.minExpected && checks.maxUncertainty;

    // Calculate promotion confidence
    const confidence = eligible ?
      Math.min(PHI_INV, expectedValue * (1 - uncertainty)) :
      0;

    return {
      eligible,
      checks,
      expectedValue,
      uncertainty,
      confidence,
      pulls: arm.pulls,
      reason: !eligible ?
        (!checks.minPulls ? 'Insufficient observations' :
         !checks.minExpected ? 'Success rate too low' :
         'Uncertainty too high') :
        'Eligible for promotion',
    };
  }

  /**
   * Promote a pattern to a heuristic
   * @param {string} patternId - Pattern identifier
   * @param {Object} rule - Rule definition
   * @param {string} rule.trigger - When to apply this heuristic
   * @param {string} rule.action - What to do
   * @param {string} rule.sefirah - Which Sefirot channel
   * @returns {Object} Promotion result
   */
  promoteToHeuristic(patternId, rule) {
    const assessment = this.assessPromotion(patternId);

    if (!assessment.eligible) {
      return { promoted: false, ...assessment };
    }

    // Create heuristic
    const heuristic = {
      id: `heur_${patternId}_${Date.now()}`,
      patternId,
      rule,
      promotedAt: Date.now(),
      promotionConfidence: assessment.confidence,
      expectedValue: assessment.expectedValue,
      uncertainty: assessment.uncertainty,
      applications: 0,
      successes: 0,
      active: true,
    };

    this.heuristics.set(patternId, heuristic);

    // Record promotion
    this.promotionHistory.push({
      type: 'promotion',
      patternId,
      heuristicId: heuristic.id,
      timestamp: Date.now(),
      confidence: assessment.confidence,
    });

    // Keep bounded history
    if (this.promotionHistory.length > 100) {
      this.promotionHistory.shift();
    }

    // Update state
    this.state.promotedHeuristics = this.heuristics.size;

    return { promoted: true, heuristic, ...assessment };
  }

  /**
   * Demote a heuristic back to pattern
   * Called when a heuristic's performance degrades
   * @param {string} patternId - Pattern identifier
   * @param {string} reason - Why demoting
   */
  demoteHeuristic(patternId, reason = 'Performance degraded') {
    const heuristic = this.heuristics.get(patternId);
    if (!heuristic) return { demoted: false, reason: 'Heuristic not found' };

    // Record demotion
    this.promotionHistory.push({
      type: 'demotion',
      patternId,
      heuristicId: heuristic.id,
      timestamp: Date.now(),
      reason,
      finalStats: {
        applications: heuristic.applications,
        successes: heuristic.successes,
        successRate: heuristic.applications > 0 ?
          heuristic.successes / heuristic.applications : 0,
      },
    });

    // Remove heuristic
    this.heuristics.delete(patternId);
    this.state.promotedHeuristics = this.heuristics.size;

    return { demoted: true, reason, heuristic };
  }

  /**
   * Apply a heuristic and record outcome
   * @param {string} patternId - Pattern/heuristic identifier
   * @param {boolean} success - Whether application was successful
   */
  recordHeuristicApplication(patternId, success) {
    const heuristic = this.heuristics.get(patternId);
    if (!heuristic) return;

    heuristic.applications += 1;
    if (success) {
      heuristic.successes += 1;
    }

    // Also update Thompson Sampler (for re-evaluation)
    this.thompsonSampler.update(patternId, success);

    // Check if should demote
    if (heuristic.applications >= 5) {
      const currentRate = heuristic.successes / heuristic.applications;
      if (currentRate < PROMOTION_CONFIG.DEMOTION_THRESHOLD) {
        this.demoteHeuristic(patternId, `Success rate dropped to ${Math.round(currentRate * 100)}%`);
      }
    }
  }

  /**
   * Get all active heuristics that match a context
   * @param {Object} context - Current context
   * @param {string} context.tool - Current tool
   * @param {string} context.errorType - Error type if applicable
   * @param {string} context.file - Current file
   * @returns {Object[]} Matching heuristics with their rules
   */
  getActiveHeuristics(context = {}) {
    const matching = [];

    for (const [patternId, heuristic] of this.heuristics) {
      if (!heuristic.active) continue;

      // Check if trigger matches context
      const rule = heuristic.rule;
      let matches = false;

      if (rule.trigger === 'always') {
        matches = true;
      } else if (rule.trigger === 'on_error' && context.errorType) {
        matches = true;
      } else if (rule.trigger === 'on_tool' && rule.tool === context.tool) {
        matches = true;
      } else if (rule.trigger === 'on_file' && context.file?.includes(rule.filePattern)) {
        matches = true;
      }

      if (matches) {
        matching.push({
          patternId,
          heuristic,
          expectedValue: this.thompsonSampler.getExpectedValue(patternId),
          confidence: heuristic.promotionConfidence,
        });
      }
    }

    // Sort by expected value (best first)
    matching.sort((a, b) => b.expectedValue - a.expectedValue);

    return matching;
  }

  /**
   * Review all patterns and auto-promote eligible ones
   * Called periodically
   */
  reviewPatterns() {
    const now = Date.now();

    // Rate limit reviews
    if (now - this.lastReviewTime < PROMOTION_CONFIG.REVIEW_INTERVAL_MS) {
      return { reviewed: false, reason: 'Too soon' };
    }

    this.lastReviewTime = now;
    const results = { promoted: [], demoted: [], reviewed: 0 };

    // Review all Thompson arms
    for (const [patternId, arm] of this.thompsonSampler.arms) {
      results.reviewed += 1;

      // Skip already promoted
      if (this.heuristics.has(patternId)) {
        // Check if should demote
        const heuristic = this.heuristics.get(patternId);
        const expectedValue = this.thompsonSampler.getExpectedValue(patternId);

        if (expectedValue < PROMOTION_CONFIG.DEMOTION_THRESHOLD) {
          const demotion = this.demoteHeuristic(patternId, 'Expected value dropped below threshold');
          if (demotion.demoted) results.demoted.push(patternId);
        }
        continue;
      }

      // Check for promotion
      const assessment = this.assessPromotion(patternId);
      if (assessment.eligible) {
        // Auto-generate rule based on pattern characteristics
        const rule = this._inferRule(patternId);
        const promotion = this.promoteToHeuristic(patternId, rule);
        if (promotion.promoted) {
          results.promoted.push(patternId);
        }
      }
    }

    return results;
  }

  /**
   * Infer a rule from pattern ID and history
   * @private
   */
  _inferRule(patternId) {
    // Parse pattern ID for context
    const isErrorPattern = patternId.includes('error_') || patternId.includes('_error');
    const isToolPattern = patternId.startsWith('tool_');

    return {
      trigger: isErrorPattern ? 'on_error' : (isToolPattern ? 'on_tool' : 'always'),
      tool: isToolPattern ? patternId.replace('tool_', '') : null,
      action: 'suggest',
      sefirah: isErrorPattern ? 'GEVURAH' : 'MALKHUT',
      description: `Auto-promoted pattern: ${patternId}`,
    };
  }

  /**
   * Get promotion statistics
   */
  getPromotionStats() {
    const recentPromotions = this.promotionHistory.filter(p =>
      Date.now() - p.timestamp < 24 * 60 * 60 * 1000 // Last 24h
    );

    return {
      activeHeuristics: this.heuristics.size,
      totalPromotions: this.promotionHistory.filter(p => p.type === 'promotion').length,
      totalDemotions: this.promotionHistory.filter(p => p.type === 'demotion').length,
      recentActivity: recentPromotions.length,
      heuristicDetails: Array.from(this.heuristics.values()).map(h => ({
        id: h.id,
        patternId: h.patternId,
        applications: h.applications,
        successRate: h.applications > 0 ? h.successes / h.applications : null,
        promotionConfidence: h.promotionConfidence,
      })),
    };
  }

  /**
   * Export state for persistence
   */
  exportState() {
    const sefirotData = {};
    for (const [key, data] of this.sefirotSignals) {
      sefirotData[key] = data.signals;
    }

    // Also export heuristics
    const heuristicsData = {};
    for (const [key, data] of this.heuristics) {
      heuristicsData[key] = data;
    }

    return {
      sefirot: sefirotData,
      resonanceHistory: this.resonanceHistory,
      coherenceWindow: this.coherenceWindow,
      state: this.state,
      thompson: this.thompsonSampler.exportState(),
      heuristics: heuristicsData,
      promotionHistory: this.promotionHistory,
      calibration: this.confidenceCalibrator.exportState(),
      // Temporal analyzer (Task #22)
      temporal: this.temporalAnalyzer.export(),
      // Girsanov multi-measure (Task #23)
      girsanov: this.girsanovTransformer.exportState(),
      // Antifragility tracker (Task #24)
      antifragility: this.antifragilityTracker.exportState(),
      // Non-commutative evaluator (Task #25)
      nonCommutative: this.nonCommutativeEvaluator.exportState(),
    };
  }

  /**
   * Import state from persistence
   */
  importState(data) {
    if (data.sefirot) {
      for (const [key, signals] of Object.entries(data.sefirot)) {
        const channelData = this.sefirotSignals.get(key);
        if (channelData) {
          channelData.signals = signals;
        }
      }
    }
    if (data.resonanceHistory) this.resonanceHistory = data.resonanceHistory;
    if (data.coherenceWindow) this.coherenceWindow = data.coherenceWindow;
    if (data.state) this.state = { ...this.state, ...data.state };
    if (data.thompson) this.thompsonSampler.importState(data.thompson);

    // Import heuristics
    if (data.heuristics) {
      for (const [key, heuristic] of Object.entries(data.heuristics)) {
        this.heuristics.set(key, heuristic);
      }
      this.state.promotedHeuristics = this.heuristics.size;
    }
    if (data.promotionHistory) this.promotionHistory = data.promotionHistory;

    // Import calibration
    if (data.calibration) this.confidenceCalibrator.importState(data.calibration);

    // Import temporal analyzer (Task #22)
    if (data.temporal) this.temporalAnalyzer.import(data.temporal);

    // Import Girsanov multi-measure (Task #23)
    if (data.girsanov) this.girsanovTransformer.importState(data.girsanov);

    // Import Antifragility tracker (Task #24)
    if (data.antifragility) this.antifragilityTracker.importState(data.antifragility);

    // Import Non-commutative evaluator (Task #25)
    if (data.nonCommutative) this.nonCommutativeEvaluator.importState(data.nonCommutative);
  }

  /**
   * Process feedback (called from observe.js)
   * Unified entry point for feedback processing
   * @param {Object} feedback
   * @param {string} feedback.type - Feedback type
   * @param {string} feedback.sentiment - positive/negative/neutral
   * @param {string} feedback.suggestionId - Related suggestion if any
   * @param {number} feedback.confidence - Feedback confidence
   * @param {Object} feedback.action - User action details
   * @param {string} feedback.source - 'implicit' or 'explicit'
   */
  processFeedback(feedback) {
    const { type, sentiment, suggestionId, confidence, action, source } = feedback;

    // 1. Update Thompson Sampler
    const success = sentiment === 'positive';
    this.thompsonSampler.update(type, success);

    // 2. If we have a suggestion ID, record the action
    if (suggestionId) {
      this.recordAction({
        suggestionId,
        followed: success,
        timing: action?.timing || 'unknown',
        context: { source, confidence },
      });
    }

    // 3. Check if any heuristic was applied
    const matchingHeuristic = this.heuristics.get(type);
    if (matchingHeuristic) {
      this.recordHeuristicApplication(type, success);
    }

    // 4. Periodic review for promotions
    this.reviewPatterns();

    // ═══════════════════════════════════════════════════════════════════════
    // FIL 2 (Task #84): Forward to learning service via callback
    // This connects Thompson Sampling updates to weight adjustments
    // ═══════════════════════════════════════════════════════════════════════
    if (this.learningCallback) {
      // Fire-and-forget async call - don't block processFeedback
      this.learningCallback({
        outcome: success ? 'correct' : 'incorrect',
        source: source || 'harmonic',
        sourceContext: {
          type,
          sentiment,
          confidence: confidence || 0.5,
          heuristicApplied: !!matchingHeuristic,
        },
      }).catch(() => {
        // Silently fail - learning is enhancement, not critical path
      });
    }

    return {
      processed: true,
      type,
      sentiment,
      success,
      heuristicApplied: !!matchingHeuristic,
      state: this.getState(),
    };
  }

  /**
   * Get current state (for observation output)
   */
  getState() {
    const calibrationMetrics = this.confidenceCalibrator.metrics;
    return {
      coherence: this.state.overallCoherence,
      resonance: this.state.overallResonance,
      entrainment: this.state.entrainment,
      ohrFlow: this.state.ohrFlow,
      tikkunProgress: this.state.tikkunProgress,
      promotedHeuristics: this.state.promotedHeuristics,
      calibration: this.secondOrderFeedback.calibration,
      // Confidence calibration metrics
      brierScore: calibrationMetrics.brierScore,
      reliability: calibrationMetrics.reliability,
      calibrationFactor: this.confidenceCalibrator.calibrationFactor,
    };
  }

  /**
   * Calibrate a raw confidence value
   * Uses historical accuracy to adjust confidence
   * @param {number} rawConfidence - Original confidence estimate
   * @returns {number} Calibrated confidence (never > φ⁻¹)
   */
  calibrateConfidence(rawConfidence) {
    return this.confidenceCalibrator.calibrate(rawConfidence);
  }

  /**
   * Get calibration recommendations
   * @returns {Object} Calibration analysis
   */
  getCalibrationAnalysis() {
    return {
      recommendations: this.confidenceCalibrator.getRecommendations(),
      reliabilityDiagram: this.confidenceCalibrator.getReliabilityDiagram(),
      metrics: this.confidenceCalibrator.metrics,
      factor: this.confidenceCalibrator.calibrationFactor,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════
  // GIRSANOV MULTI-MEASURE METHODS (Task #23)
  // "Le chien voit trois futurs possibles"
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get confidence under all three probability measures
   * @param {number} rawConfidence - Raw confidence from Thompson Sampling
   * @param {Object} context - Context for measure selection
   * @returns {Object} Multi-measure confidence with range and recommendation
   */
  getMultiMeasureConfidence(rawConfidence, context = {}) {
    return this.girsanovTransformer.getMultiMeasureConfidence(rawConfidence, context);
  }

  /**
   * Get adaptive confidence using best measure for context
   * Automatically selects risk-averse for dangerous contexts
   * @param {number} rawConfidence - Raw confidence
   * @param {Object} context - Context
   * @returns {number} Best confidence estimate
   */
  getAdaptiveConfidence(rawConfidence, context = {}) {
    return this.girsanovTransformer.getAdaptiveConfidence(rawConfidence, context);
  }

  /**
   * Get Girsanov multi-measure statistics
   * @returns {Object} Stats about measure performance
   */
  getGirsanovStats() {
    return this.girsanovTransformer.getStats();
  }

  // ═══════════════════════════════════════════════════════════════════════
  // ANTIFRAGILITY METHODS (Task #24)
  // "Le chien devient plus fort par l'adversité"
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get antifragility statistics
   * @returns {Object} Antifragility metrics and interpretation
   */
  getAntifragilityStats() {
    return this.antifragilityTracker.getStats();
  }

  /**
   * Get antifragility index
   * @returns {number} Current antifragility index (-0.618 to 0.618)
   */
  getAntifragilityIndex() {
    return this.antifragilityTracker.metrics.index;
  }

  /**
   * Get performance curve for visualization
   * @returns {Object[]} Volatility vs performance data points
   */
  getPerformanceCurve() {
    return this.antifragilityTracker.getPerformanceCurve();
  }

  // ═══════════════════════════════════════════════════════════════════════
  // NON-COMMUTATIVE EVALUATION METHODS (Task #25)
  // "L'ordre d'évaluation change le résultat"
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Record a dimension evaluation for non-commutativity tracking
   * @param {Object} evaluation - Evaluation data
   * @param {string[]} evaluation.order - Order of dimensions evaluated
   * @param {Object} evaluation.scores - Score per dimension
   * @param {number} evaluation.finalScore - Final judgment score
   * @param {string} evaluation.context - Context identifier
   */
  recordDimensionEvaluation(evaluation) {
    this.nonCommutativeEvaluator.recordEvaluation(evaluation);
  }

  /**
   * Get recommended order for evaluating dimensions
   * @param {string[]} dimensions - Dimensions to evaluate
   * @returns {string[]} Recommended evaluation order
   */
  getRecommendedEvaluationOrder(dimensions) {
    return this.nonCommutativeEvaluator.getRecommendedOrder(dimensions);
  }

  /**
   * Get commutator between two dimensions
   * @param {string} dimA - First dimension
   * @param {string} dimB - Second dimension
   * @returns {number} Commutator [A,B]
   */
  getDimensionCommutator(dimA, dimB) {
    return this.nonCommutativeEvaluator.getCommutator(dimA, dimB);
  }

  /**
   * Get non-commutative evaluation statistics
   * @returns {Object} Non-commutativity stats
   */
  getNonCommutativeStats() {
    return this.nonCommutativeEvaluator.getStats();
  }

  /**
   * Introspect: CYNIC views its own subconscious state
   * Task #74: Allow self-reflection on learning state
   * @returns {Object} Full subconscious introspection
   */
  introspect() {
    const thompsonStats = this.thompsonSampler.getStats();
    const calibrationAnalysis = this.getCalibrationAnalysis();
    const promotionStats = this.getPromotionStats();

    // Calculate subconscious confidence (what CYNIC "really" believes)
    const armEntries = Object.entries(thompsonStats.arms || {});
    const avgSubconsciousConfidence = armEntries.length > 0
      ? armEntries.reduce((sum, [, arm]) => sum + arm.alpha / (arm.alpha + arm.beta), 0) / armEntries.length
      : 0.5;

    // Detect over/under confidence asymmetry
    const consciousMax = PHI_INV; // 61.8%
    const confidenceGap = avgSubconsciousConfidence - consciousMax;

    return {
      // Conscious state (what CYNIC admits)
      conscious: {
        maxConfidence: consciousMax,
        coherence: this.state.coherence,
        resonance: this.state.resonance,
        tikkunProgress: this.state.tikkunProgress,
      },

      // Subconscious state (what CYNIC actually learned)
      subconscious: {
        thompsonArms: thompsonStats.armCount,
        avgConfidence: Math.round(avgSubconsciousConfidence * 100) / 100,
        totalLearningEvents: thompsonStats.totalPulls,
        patterns: {
          successful: armEntries.filter(([, a]) => a.alpha / (a.alpha + a.beta) > PHI_INV_2).length,
          uncertain: armEntries.filter(([, a]) => {
            const rate = a.alpha / (a.alpha + a.beta);
            return rate >= PHI_INV_3 && rate <= PHI_INV_2;
          }).length,
          failing: armEntries.filter(([, a]) => a.alpha / (a.alpha + a.beta) < PHI_INV_3).length,
        },
      },

      // Heuristics (promoted patterns)
      heuristics: {
        active: promotionStats.activeHeuristics,
        details: promotionStats.heuristicDetails.map(h => ({
          id: h.id,
          successRate: Math.round(h.successRate * 100),
          applications: h.applications,
        })),
      },

      // Calibration (self-assessment accuracy)
      calibration: {
        brierScore: calibrationAnalysis.metrics?.brierScore,
        reliability: calibrationAnalysis.metrics?.reliability,
        factor: calibrationAnalysis.factor,
        overconfident: calibrationAnalysis.recommendations?.isOverconfident,
        underconfident: calibrationAnalysis.recommendations?.isUnderconfident,
      },

      // Meta-awareness
      meta: {
        confidenceGap: Math.round(confidenceGap * 100) / 100,
        selfAssessment: confidenceGap > 0.1
          ? 'Subconscious more confident than conscious admits - φ is working'
          : confidenceGap < -0.1
            ? 'Subconscious less confident - learning is struggling'
            : 'Balanced awareness - healthy φ state',
        philosophy: 'φ⁻¹ caps conscious confidence at 61.8%, but subconscious learns from everything.',
      },
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// TEMPORAL PATTERN ANALYZER (FFT)
// "Un système qui survit au hasard peut survivre à n'importe quoi"
// Fourier analysis detects cyclical patterns in user behavior
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Known human cognitive cycles (in milliseconds)
 * These are the frequencies we look for in user behavior
 */
const COGNITIVE_CYCLES = Object.freeze({
  // Ultradian rhythms (within-day cycles)
  FOCUS_CYCLE: {
    name: 'Focus/Break Cycle',
    periodMs: 90 * 60 * 1000,      // 90 minutes (BRAC cycle)
    description: 'Basic Rest-Activity Cycle - natural focus/recovery rhythm',
  },
  POMODORO: {
    name: 'Pomodoro-like',
    periodMs: 25 * 60 * 1000,      // 25 minutes
    description: 'Short focus bursts common in developers',
  },
  HOURLY: {
    name: 'Hourly Check-in',
    periodMs: 60 * 60 * 1000,      // 1 hour
    description: 'Hourly rhythm - often meetings/standup driven',
  },

  // Circadian rhythms (daily cycles)
  CIRCADIAN: {
    name: 'Circadian',
    periodMs: 24 * 60 * 60 * 1000, // 24 hours
    description: 'Daily rhythm - energy peaks/troughs',
  },

  // Infradian rhythms (longer than a day)
  WEEKLY: {
    name: 'Weekly',
    periodMs: 7 * 24 * 60 * 60 * 1000, // 1 week
    description: 'Weekly patterns - Monday blues, Friday rushes',
  },
});

/**
 * TemporalPatternAnalyzer - Fourier analysis of user behavior
 *
 * Detects cyclical patterns using Discrete Fourier Transform.
 * "Le chien détecte les rythmes cachés"
 */
class TemporalPatternAnalyzer {
  constructor(options = {}) {
    this.windowSize = options.windowSize || 256;  // Number of samples for FFT
    this.sampleIntervalMs = options.sampleIntervalMs || 5 * 60 * 1000;  // 5 min default
    this.minSignificance = options.minSignificance || PHI_INV_2;  // 38.2% threshold

    // Time series storage (circular buffer)
    this.timeSeries = {
      activity: [],       // Overall activity level
      coherence: [],      // Coherence scores
      resonance: [],      // Resonance scores
      errors: [],         // Error frequency
    };

    // Detected patterns
    this.detectedCycles = new Map();
    this.lastAnalysis = null;
    this.analysisCount = 0;
  }

  /**
   * Record a data point in the time series
   * @param {string} series - Which series to record to
   * @param {number} value - Value to record (0-1 normalized)
   * @param {number} [timestamp] - Optional timestamp (default: now)
   */
  record(series, value, timestamp = Date.now()) {
    if (!this.timeSeries[series]) {
      this.timeSeries[series] = [];
    }

    this.timeSeries[series].push({ value, timestamp });

    // Keep bounded to window size
    if (this.timeSeries[series].length > this.windowSize) {
      this.timeSeries[series].shift();
    }
  }

  /**
   * Discrete Fourier Transform (DFT)
   * For small datasets, this is simpler than FFT and sufficient
   *
   * @param {number[]} signal - Real-valued time series
   * @returns {Object[]} Array of { frequency, magnitude, phase }
   */
  dft(signal) {
    const N = signal.length;
    const spectrum = [];

    for (let k = 0; k < N / 2; k++) {  // Only need half (Nyquist)
      let real = 0;
      let imag = 0;

      for (let n = 0; n < N; n++) {
        const angle = (2 * Math.PI * k * n) / N;
        real += signal[n] * Math.cos(angle);
        imag -= signal[n] * Math.sin(angle);
      }

      const magnitude = Math.sqrt(real * real + imag * imag) / N;
      const phase = Math.atan2(imag, real);

      spectrum.push({
        k,                                      // Frequency bin index
        frequency: k / N,                       // Normalized frequency (cycles per sample)
        magnitude,
        phase,
        periodSamples: k > 0 ? N / k : Infinity, // Period in number of samples
      });
    }

    return spectrum;
  }

  /**
   * Analyze a time series for dominant frequencies
   * @param {string} seriesName - Name of the series to analyze
   * @returns {Object} Analysis results
   */
  analyzeTimeSeries(seriesName) {
    const series = this.timeSeries[seriesName];
    if (!series || series.length < 16) {  // Need at least 16 samples
      return { error: 'Insufficient data', samples: series?.length || 0 };
    }

    // Extract values and normalize (remove DC component / mean)
    const values = series.map(s => s.value);
    const mean = values.reduce((a, b) => a + b, 0) / values.length;
    const normalized = values.map(v => v - mean);

    // Compute DFT
    const spectrum = this.dft(normalized);

    // Find dominant frequencies (excluding DC component at k=0)
    const significant = spectrum
      .filter(s => s.k > 0 && s.magnitude > this.minSignificance * Math.max(...spectrum.map(x => x.magnitude)))
      .sort((a, b) => b.magnitude - a.magnitude)
      .slice(0, 5);  // Top 5 frequencies

    // Calculate time span of data
    const timeSpanMs = series[series.length - 1].timestamp - series[0].timestamp;
    const avgIntervalMs = timeSpanMs / (series.length - 1);

    // Convert to real-world periods
    const cycles = significant.map(s => {
      const periodMs = s.periodSamples * avgIntervalMs;
      const matchedCycle = this._matchKnownCycle(periodMs);

      return {
        periodMs,
        periodHuman: this._formatPeriod(periodMs),
        magnitude: Math.round(s.magnitude * 1000) / 1000,
        relativeStrength: s.magnitude / Math.max(...spectrum.map(x => x.magnitude)),
        matchedCycle: matchedCycle?.name || null,
        description: matchedCycle?.description || 'Unknown pattern',
      };
    });

    return {
      series: seriesName,
      samples: series.length,
      timeSpanMs,
      avgIntervalMs,
      dominantCycles: cycles,
      analysisTimestamp: Date.now(),
    };
  }

  /**
   * Match detected period to known cognitive cycles
   * @param {number} periodMs - Detected period in milliseconds
   * @returns {Object|null} Matched cycle or null
   */
  _matchKnownCycle(periodMs) {
    // Allow 20% tolerance
    const tolerance = 0.2;

    for (const [key, cycle] of Object.entries(COGNITIVE_CYCLES)) {
      const ratio = periodMs / cycle.periodMs;
      if (ratio > (1 - tolerance) && ratio < (1 + tolerance)) {
        return cycle;
      }
    }
    return null;
  }

  /**
   * Format period in human-readable form
   * @param {number} periodMs - Period in milliseconds
   * @returns {string} Human readable string
   */
  _formatPeriod(periodMs) {
    const minutes = periodMs / (60 * 1000);
    const hours = periodMs / (60 * 60 * 1000);
    const days = periodMs / (24 * 60 * 60 * 1000);

    if (days >= 1) return `${Math.round(days * 10) / 10} days`;
    if (hours >= 1) return `${Math.round(hours * 10) / 10} hours`;
    return `${Math.round(minutes)} minutes`;
  }

  /**
   * Run full analysis on all time series
   * @returns {Object} Complete analysis report
   */
  analyzeAll() {
    const results = {};

    for (const seriesName of Object.keys(this.timeSeries)) {
      results[seriesName] = this.analyzeTimeSeries(seriesName);
    }

    this.lastAnalysis = {
      timestamp: Date.now(),
      results,
      recommendation: this._generateRecommendation(results),
    };

    this.analysisCount++;
    return this.lastAnalysis;
  }

  /**
   * Generate recommendations based on detected patterns
   * @param {Object} results - Analysis results
   * @returns {Object} Recommendations
   */
  _generateRecommendation(results) {
    const recommendations = [];

    // Check activity patterns
    if (results.activity?.dominantCycles?.length > 0) {
      const topCycle = results.activity.dominantCycles[0];

      if (topCycle.matchedCycle === 'Focus/Break Cycle') {
        recommendations.push({
          type: 'rhythm_detected',
          message: `*sniff* Ton cycle focus/repos est de ~${topCycle.periodHuman}. CYNIC va s'adapter.`,
          action: 'adjust_timing',
          periodMs: topCycle.periodMs,
        });
      }

      if (topCycle.matchedCycle === 'Circadian') {
        recommendations.push({
          type: 'circadian_detected',
          message: '*ears perk* Cycle circadien détecté. Pic d\'énergie prévisible.',
          action: 'predict_energy',
        });
      }
    }

    // Check error patterns
    if (results.errors?.dominantCycles?.length > 0) {
      const errorCycle = results.errors.dominantCycles[0];
      recommendations.push({
        type: 'error_pattern',
        message: `*GROWL* Les erreurs suivent un pattern de ${errorCycle.periodHuman}. Fatigue cyclique?`,
        action: 'warn_fatigue',
        periodMs: errorCycle.periodMs,
      });
    }

    // Coherence trends
    if (results.coherence?.dominantCycles?.length > 0) {
      recommendations.push({
        type: 'coherence_cycle',
        message: 'Fluctuations de cohérence détectées - flow state cyclique.',
        action: 'optimize_sessions',
      });
    }

    return {
      count: recommendations.length,
      recommendations,
      summary: recommendations.length > 0
        ? `${recommendations.length} pattern(s) temporel(s) détecté(s)`
        : 'Pas assez de données pour détecter des patterns',
    };
  }

  /**
   * Get timing recommendation for next action
   * Based on detected cycles, when is the best time?
   * @returns {Object} Timing recommendation
   */
  getOptimalTiming() {
    if (!this.lastAnalysis) {
      return { recommendation: 'analyze_first', message: 'Pas d\'analyse disponible' };
    }

    const activityCycles = this.lastAnalysis.results.activity?.dominantCycles || [];
    if (activityCycles.length === 0) {
      return { recommendation: 'no_pattern', message: 'Pas de cycle détecté' };
    }

    // Use the strongest detected cycle
    const mainCycle = activityCycles[0];
    const now = Date.now();
    const phaseInCycle = (now % mainCycle.periodMs) / mainCycle.periodMs;

    // Activity typically peaks at phase 0.25 (quarter into cycle)
    // and troughs at phase 0.75 (three quarters in)
    const peakPhase = 0.25;
    const distanceFromPeak = Math.abs(phaseInCycle - peakPhase);

    return {
      currentPhase: Math.round(phaseInCycle * 100) + '%',
      cyclePeriod: mainCycle.periodHuman,
      energyLevel: distanceFromPeak < 0.25 ? 'HIGH' : distanceFromPeak > 0.5 ? 'LOW' : 'MEDIUM',
      recommendation: distanceFromPeak < 0.25
        ? 'optimal'
        : distanceFromPeak > 0.5
          ? 'rest_recommended'
          : 'normal',
      message: distanceFromPeak < 0.25
        ? '*tail wag* Période optimale pour les tâches complexes!'
        : distanceFromPeak > 0.5
          ? '*yawn* Énergie basse - tâches simples recommandées'
          : 'Période normale - continuer comme prévu',
    };
  }

  /**
   * Export analysis state for persistence
   */
  export() {
    return {
      timeSeries: this.timeSeries,
      detectedCycles: Object.fromEntries(this.detectedCycles),
      lastAnalysis: this.lastAnalysis,
      analysisCount: this.analysisCount,
    };
  }

  /**
   * Import from persisted state
   */
  import(state) {
    if (state.timeSeries) this.timeSeries = state.timeSeries;
    if (state.detectedCycles) this.detectedCycles = new Map(Object.entries(state.detectedCycles));
    if (state.lastAnalysis) this.lastAnalysis = state.lastAnalysis;
    if (state.analysisCount) this.analysisCount = state.analysisCount;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON & EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;
let _temporalAnalyzer = null;

/**
 * Get temporal pattern analyzer singleton
 */
function getTemporalPatternAnalyzer() {
  if (!_temporalAnalyzer) {
    _temporalAnalyzer = new TemporalPatternAnalyzer();
  }
  return _temporalAnalyzer;
}

function getHarmonicFeedback() {
  if (!_instance) {
    _instance = new HarmonicFeedbackSystem();
  }
  return _instance;
}

function resetHarmonicFeedback() {
  _instance = null;
}

export {
  // Main system
  HarmonicFeedbackSystem,
  getHarmonicFeedback,
  resetHarmonicFeedback,

  // Components
  ThompsonSampler,
  FirstOrderFeedback,
  SecondOrderFeedback,
  ConfidenceCalibrator,

  // Girsanov Multi-Measure (Task #23)
  GirsanovMeasureTransformer,
  GIRSANOV_CONFIG,

  // Antifragility Tracker (Task #24)
  AntifragilityTracker,
  ANTIFRAGILITY_CONFIG,

  // Non-Commutative Evaluator (Task #25)
  NonCommutativeEvaluator,
  NONCOMMUTATIVE_CONFIG,

  // Temporal Analysis (FFT) - Task #22
  TemporalPatternAnalyzer,
  getTemporalPatternAnalyzer,
  COGNITIVE_CYCLES,

  // Utilities
  calculateCoherence,
  calculateResonance,
  calculateEntrainment,
  temporalDecay,
  eligibilityTrace,

  // Constants
  SEFIROT_CHANNELS,
  FeedbackState,
  PROMOTION_CONFIG,
  CALIBRATION_CONFIG,
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,
};

export default HarmonicFeedbackSystem;

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

    // Gateway resonance tracking
    this.resonanceHistory = [];
    this.coherenceWindow = [];

    // Pattern-to-Heuristic Promotion System
    this.heuristics = new Map();     // Promoted patterns → rules
    this.promotionHistory = [];       // Track promotions/demotions
    this.lastReviewTime = Date.now();

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
    };
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
// SINGLETON & EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

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

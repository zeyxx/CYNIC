/**
 * CYNIC Judge
 *
 * The judgment engine - κυνικός (like a dog)
 *
 * "φ distrusts φ" - Max confidence 61.8%, min doubt 38.2%
 *
 * Verdicts (dog metaphors):
 * - HOWL: Exceptional (≥80)
 * - WAG: Passes (≥50)
 * - GROWL: Needs work (≥38.2)
 * - BARK: Critical (<38.2)
 *
 * @module @cynic/node/judge/judge
 */

'use strict';

import {
  PHI_INV,
  PHI_INV_2,
  THRESHOLDS,
  calculateQScoreFromAxioms,
  calculateFinalScore,
  getVerdict,
  analyzeWeaknesses,
} from '@cynic/core';
import {
  createJudgment,
  validateJudgment,
  calculateResidual,
  isAnomalous,
  Verdict,
} from '@cynic/protocol';
import {
  Dimensions,
  getAllDimensions,
  getDimensionsForAxiom,
  getTotalWeight,
  getAxiomTotalWeight,
  dimensionRegistry,
} from './dimensions.js';
import { createRealScorer } from './scorers.js';

/**
 * CYNIC Judge - The judgment engine
 */
export class CYNICJudge {
  /**
   * Create judge instance
   * @param {Object} [options] - Judge options
   * @param {Object} [options.customDimensions] - Custom dimensions to include
   * @param {Function} [options.scorer] - Custom scoring function
   * @param {import('./learning-service.js').LearningService} [options.learningService] - RLHF learning service
   * @param {import('./self-skeptic.js').SelfSkeptic} [options.selfSkeptic] - φ distrusts φ skepticism service
   * @param {import('./residual.js').ResidualDetector} [options.residualDetector] - THE_UNNAMEABLE dimension discovery
   * @param {boolean} [options.applySkepticism=true] - Whether to automatically apply skepticism to judgments
   * @param {boolean} [options.includeUnnameable=true] - Whether to include THE_UNNAMEABLE in dimensions
   * @param {Object} [options.eScoreProvider] - E-Score provider for vote weighting
   */
  constructor(options = {}) {
    this.customDimensions = options.customDimensions || {};
    // Use real scorer by default, allow custom override
    this.customScorer = options.scorer !== undefined ? options.scorer : createRealScorer();
    this.learningService = options.learningService || null;
    this.selfSkeptic = options.selfSkeptic || null;
    this.residualDetector = options.residualDetector || null;
    this.applySkepticism = options.applySkepticism !== false; // Default true
    this.includeUnnameable = options.includeUnnameable !== false; // Default true

    // E-Score provider for vote weighting (Burns → E-Score → Vote Weight)
    this.eScoreProvider = options.eScoreProvider || null;

    // Stats tracking
    this.stats = {
      totalJudgments: 0,
      verdicts: { HOWL: 0, WAG: 0, GROWL: 0, BARK: 0 },
      anomaliesDetected: 0,
      avgScore: 0,
    };

    // Anomaly buffer for ResidualDetector
    this.anomalyBuffer = [];
  }

  /**
   * Set learning service (for post-construction injection)
   * @param {import('./learning-service.js').LearningService} learningService
   */
  setLearningService(learningService) {
    this.learningService = learningService;
  }

  /**
   * Set self-skeptic service (for post-construction injection)
   * "φ distrusts φ" - active self-doubt mechanism
   * @param {import('./self-skeptic.js').SelfSkeptic} selfSkeptic
   */
  setSelfSkeptic(selfSkeptic) {
    this.selfSkeptic = selfSkeptic;
  }

  /**
   * Set residual detector (for post-construction injection)
   * THE_UNNAMEABLE - discovers new dimensions from unexplained variance
   * @param {import('./residual.js').ResidualDetector} residualDetector
   */
  setResidualDetector(residualDetector) {
    this.residualDetector = residualDetector;
  }

  /**
   * Set E-Score provider (for post-construction injection)
   * Burns → E-Score → Vote Weight
   * @param {Object} eScoreProvider - Object with getScore() and getState() methods
   */
  setEScoreProvider(eScoreProvider) {
    this.eScoreProvider = eScoreProvider;
  }

  /**
   * Judge an item
   * @param {Object} item - Item to judge
   * @param {Object} [context] - Judgment context
   * @returns {Object} Judgment result with Q-Score
   */
  judge(item, context = {}) {
    // Score each dimension
    const dimensionScores = this._scoreDimensions(item, context);

    // Calculate global score (weighted average - legacy)
    const globalScore = this._calculateGlobalScore(dimensionScores);

    // Calculate axiom scores (aggregated by axiom)
    const axiomScores = this._calculateAxiomScores(dimensionScores);

    // Calculate Q-Score (geometric mean of axioms)
    const qScoreResult = this._calculateQScore(axiomScores);

    // Calculate confidence (φ-bounded)
    const confidence = this._calculateConfidence(dimensionScores, context);

    // If K-Score is provided, calculate Final score
    let finalScore = null;
    if (typeof context.kScore === 'number') {
      finalScore = calculateFinalScore(context.kScore, qScoreResult.Q);
    }

    // Create judgment
    const judgment = createJudgment({
      item,
      globalScore,
      dimensions: dimensionScores,
      confidence,
      metadata: {
        context: context.type || 'general',
        judgedAt: Date.now(),
      },
    });

    // Enhance judgment with Q-Score data
    judgment.axiomScores = axiomScores;
    judgment.qScore = qScoreResult.Q;
    judgment.qVerdict = qScoreResult.verdict;
    judgment.weaknesses = qScoreResult.weaknesses;

    // Include Final score if K-Score was provided
    if (finalScore) {
      judgment.kScore = context.kScore;
      judgment.finalScore = finalScore.Final;
      judgment.finalVerdict = finalScore.verdict;
      judgment.limiting = finalScore.limiting;
    }

    // Check for anomalies
    if (isAnomalous(judgment)) {
      this._recordAnomaly(judgment, item);
    }

    // Apply self-skepticism: "φ distrusts φ"
    // Active doubt mechanism that questions the judgment
    if (this.selfSkeptic && this.applySkepticism) {
      const skepticism = this.selfSkeptic.doubt(judgment, context);

      // Enhance judgment with skepticism metadata
      judgment.skepticism = {
        originalConfidence: judgment.confidence,
        adjustedConfidence: skepticism.adjustedConfidence,
        doubt: skepticism.doubt,
        biases: skepticism.biases,
        counterHypotheses: skepticism.counterHypotheses,
        recommendation: skepticism.recommendation,
        meta: skepticism.meta,
      };

      // Update confidence to the skepticism-adjusted value
      judgment.confidence = skepticism.adjustedConfidence;
    }

    // Calculate vote weight from E-Score (Burns → E-Score → Vote Weight)
    // Formula: voteWeight = eScore × log(burnedAmount + 1) / log(φ)
    judgment.voteWeight = this._calculateVoteWeight(context);

    // Update stats
    this._updateStats(judgment);

    return judgment;
  }

  /**
   * Score all dimensions for item
   * @private
   * @param {Object} item - Item to score
   * @param {Object} context - Context
   * @returns {Object} Dimension scores
   */
  _scoreDimensions(item, context) {
    const scores = {};

    // Score base dimensions (skip META - calculated after)
    for (const [axiom, dims] of Object.entries(Dimensions)) {
      if (axiom === 'META') continue; // THE_UNNAMEABLE calculated below

      for (const [dimName, config] of Object.entries(dims)) {
        scores[dimName] = this._scoreDimension(dimName, config, item, context);
      }
    }

    // Score custom dimensions
    for (const [dimName, config] of Object.entries(this.customDimensions)) {
      scores[dimName] = this._scoreDimension(dimName, config, item, context);
    }

    // Score discovered dimensions from registry
    for (const [dimName, config] of Object.entries(dimensionRegistry.getAll())) {
      scores[dimName] = this._scoreDimension(dimName, config, item, context);
    }

    // Calculate THE_UNNAMEABLE (25th dimension) based on score variance
    // High variance = high unexplained variance = low UNNAMEABLE score
    // Low variance = dimensions capture well = high UNNAMEABLE score
    scores.THE_UNNAMEABLE = this._scoreTheUnnameable(scores);

    return scores;
  }

  /**
   * Calculate THE_UNNAMEABLE dimension score
   * Measures "explained variance" - how well 24 dimensions capture the item
   * High score = low residual = well understood
   * @private
   * @param {Object} scores - All other dimension scores
   * @returns {number} THE_UNNAMEABLE score (0-100)
   */
  _scoreTheUnnameable(scores) {
    const values = Object.values(scores);
    if (values.length === 0) return 50; // Neutral if no data

    // Calculate variance - proxy for unexplained variance
    const mean = values.reduce((a, b) => a + b, 0) / values.length;
    const variance = values.reduce((sum, v) => sum + Math.pow(v - mean, 2), 0) / values.length;
    const stdDev = Math.sqrt(variance);

    // Normalize: stdDev of 0 = perfect (100), stdDev of 50 = poor (0)
    // φ⁻² (38.2%) of max stdDev is the anomaly threshold
    const maxStdDev = 50; // Max possible standard deviation (0-100 range)
    const normalizedStdDev = Math.min(stdDev / maxStdDev, 1);

    // Invert: low variance = high score
    const score = 100 * (1 - normalizedStdDev);

    return Math.round(score * 10) / 10;
  }

  /**
   * Score single dimension
   * @private
   */
  _scoreDimension(name, config, item, context) {
    // Use custom scorer if provided
    if (this.customScorer) {
      const score = this.customScorer(name, item, context);
      if (score !== null && score !== undefined) {
        return Math.min(Math.max(score, 0), 100);
      }
    }

    // Default scoring based on item properties
    return this._defaultScore(name, item, context);
  }

  /**
   * Default scoring heuristics
   * @private
   */
  _defaultScore(name, item, context) {
    // Base score - can be overridden by specific item properties
    let score = 50; // Neutral default

    // Check if item has explicit scores
    if (item.scores && typeof item.scores[name] === 'number') {
      return item.scores[name];
    }

    // Heuristics based on dimension type
    switch (name) {
      case 'COHERENCE':
        score = this._scoreCoherence(item);
        break;
      case 'ACCURACY':
        score = this._scoreAccuracy(item);
        break;
      case 'NOVELTY':
        score = this._scoreNovelty(item);
        break;
      case 'UTILITY':
        score = this._scoreUtility(item);
        break;
      case 'VERIFIABILITY':
        score = this._scoreVerifiability(item);
        break;
      default:
        // Use item's overall quality indicators if available
        if (typeof item.quality === 'number') {
          score = item.quality;
        }
    }

    return Math.min(Math.max(score, 0), 100);
  }

  /**
   * Score coherence
   * @private
   */
  _scoreCoherence(item) {
    let score = 50;

    // Structured data is more coherent
    if (typeof item === 'object' && item !== null) {
      score += 10;
    }

    // Has required fields
    if (item.id && item.type) {
      score += 10;
    }

    // Has content
    if (item.content || item.data || item.body) {
      score += 10;
    }

    return Math.min(score, 100);
  }

  /**
   * Score accuracy
   * @private
   */
  _scoreAccuracy(item) {
    let score = 50;

    // Has sources
    if (item.sources && item.sources.length > 0) {
      score += item.sources.length * 5;
    }

    // Has verification
    if (item.verified) {
      score += 20;
    }

    // Has hash/signature
    if (item.hash || item.signature) {
      score += 10;
    }

    return Math.min(score, 100);
  }

  /**
   * Score novelty
   * @private
   */
  _scoreNovelty(item) {
    let score = 40; // Slightly lower default - novelty should be earned

    // New item
    if (item.createdAt && Date.now() - item.createdAt < 86400000) {
      score += 20;
    }

    // Marked as original
    if (item.original || item.isNew) {
      score += 20;
    }

    return Math.min(score, 100);
  }

  /**
   * Score utility
   * @private
   */
  _scoreUtility(item) {
    let score = 50;

    // Has clear purpose
    if (item.purpose || item.goal) {
      score += 15;
    }

    // Has usage count
    if (item.usageCount && item.usageCount > 0) {
      score += Math.min(item.usageCount * 2, 30);
    }

    return Math.min(score, 100);
  }

  /**
   * Score verifiability
   * @private
   */
  _scoreVerifiability(item) {
    let score = 40;

    // Has proof
    if (item.proof) {
      score += 30;
    }

    // Has signature
    if (item.signature) {
      score += 15;
    }

    // Has hash
    if (item.hash) {
      score += 15;
    }

    return Math.min(score, 100);
  }

  /**
   * Calculate global score from dimensions
   * Applies learned weight modifiers from RLHF feedback
   * @private
   */
  _calculateGlobalScore(dimensionScores) {
    const allDims = getAllDimensions();
    let weightedSum = 0;
    let totalWeight = 0;

    for (const [name, score] of Object.entries(dimensionScores)) {
      const config = allDims[name] ||
        this.customDimensions[name] ||
        dimensionRegistry.get(name);

      let weight = config?.weight || 1.0;

      // Apply learned weight modifier from RLHF feedback
      if (this.learningService) {
        const modifier = this.learningService.getWeightModifier(name);
        weight = weight * modifier;
      }

      weightedSum += score * weight;
      totalWeight += weight;
    }

    if (totalWeight === 0) return 50;
    return Math.round(weightedSum / totalWeight * 10) / 10;
  }

  /**
   * Calculate scores aggregated by axiom
   * Applies learned weight modifiers from RLHF feedback
   * @private
   * @param {Object} dimensionScores - All dimension scores
   * @returns {Object} Axiom scores {PHI, VERIFY, CULTURE, BURN}
   */
  _calculateAxiomScores(dimensionScores) {
    const axiomScores = {};

    for (const [axiom, dims] of Object.entries(Dimensions)) {
      // Skip META - THE_UNNAMEABLE is not part of axiom Q-Score calculation
      if (axiom === 'META') continue;

      let weightedSum = 0;
      let totalWeight = 0;

      for (const [dimName, config] of Object.entries(dims)) {
        const score = dimensionScores[dimName];
        if (typeof score === 'number') {
          let weight = config.weight;

          // Apply learned weight modifier from RLHF feedback
          if (this.learningService) {
            const modifier = this.learningService.getWeightModifier(dimName);
            weight = weight * modifier;
          }

          weightedSum += score * weight;
          totalWeight += weight;
        }
      }

      // Calculate weighted average for this axiom
      axiomScores[axiom] = totalWeight > 0
        ? Math.round(weightedSum / totalWeight * 10) / 10
        : 50; // Neutral if no data
    }

    // Store THE_UNNAMEABLE separately (not in Q-Score formula)
    if (typeof dimensionScores.THE_UNNAMEABLE === 'number') {
      axiomScores.META = dimensionScores.THE_UNNAMEABLE;
    }

    return axiomScores;
  }

  /**
   * Calculate Q-Score using geometric mean of axiom scores
   * Q = 100 × ∜(φ × V × C × B / 100^4)
   * @private
   * @param {Object} axiomScores - Scores by axiom
   * @returns {Object} Q-Score result
   */
  _calculateQScore(axiomScores) {
    const qResult = calculateQScoreFromAxioms(axiomScores);
    const weaknesses = analyzeWeaknesses(qResult);

    return {
      Q: qResult.Q,
      verdict: qResult.verdict,
      axiomBreakdown: axiomScores,
      weaknesses,
    };
  }

  /**
   * Calculate confidence (φ-bounded)
   * @private
   */
  _calculateConfidence(dimensionScores, context) {
    const scores = Object.values(dimensionScores);
    if (scores.length === 0) return PHI_INV_2;

    // Base confidence on score consistency
    const mean = scores.reduce((a, b) => a + b, 0) / scores.length;
    const variance = scores.reduce((sum, s) => sum + Math.pow(s - mean, 2), 0) / scores.length;
    const stdDev = Math.sqrt(variance);

    // Lower variance = higher confidence
    // But NEVER exceed φ⁻¹ (61.8%)
    const rawConfidence = 1 - (stdDev / 100);
    return Math.min(rawConfidence * PHI_INV, PHI_INV);
  }

  /**
   * Record anomaly for ResidualDetector
   * THE_UNNAMEABLE - tracks unexplained variance for dimension discovery
   * @private
   */
  _recordAnomaly(judgment, item) {
    this.stats.anomaliesDetected++;

    const residual = calculateResidual(judgment);

    // Add THE_UNNAMEABLE score to judgment if not present
    if (judgment.dimensions && !judgment.dimensions.THE_UNNAMEABLE) {
      judgment.dimensions.THE_UNNAMEABLE = Math.round((1 - residual) * 100);
    }

    // Feed to ResidualDetector for dimension discovery
    if (this.residualDetector) {
      const analysis = this.residualDetector.analyze(judgment, {
        item: { type: item.type, id: item.id },
        residual,
      });

      // If anomaly detected, add to judgment metadata
      if (analysis.isAnomaly) {
        judgment.anomaly = {
          detected: true,
          residual: analysis.residual,
          potentialNewDimension: true,
        };
      }
    }

    // Also maintain local buffer for quick access
    this.anomalyBuffer.push({
      judgmentId: judgment.id,
      residual,
      timestamp: Date.now(),
      item: { type: item.type, id: item.id },
    });

    // Keep buffer bounded
    if (this.anomalyBuffer.length > 100) {
      this.anomalyBuffer.shift();
    }
  }

  /**
   * Calculate vote weight from E-Score and burns
   * Formula: voteWeight = (eScore / 100) × log(burnedAmount + 1) / log(φ)
   *
   * E-Score reflects commitment (burns, uptime, quality)
   * Burns amplify weight logarithmically (diminishing returns)
   *
   * @private
   * @param {Object} context - Judgment context
   * @returns {number} Vote weight (0+, uncapped but φ-scaled)
   */
  _calculateVoteWeight(context = {}) {
    // Default vote weight is 1.0 (equal voting)
    const DEFAULT_WEIGHT = 1.0;

    // Check for explicit vote weight in context
    if (typeof context.voteWeight === 'number') {
      return context.voteWeight;
    }

    // Check for E-Score provider
    if (!this.eScoreProvider) {
      return DEFAULT_WEIGHT;
    }

    // Get E-Score (0-100)
    let eScore = 50; // Neutral default
    try {
      if (typeof this.eScoreProvider.getScore === 'function') {
        eScore = this.eScoreProvider.getScore();
      } else if (typeof this.eScoreProvider.score === 'number') {
        eScore = this.eScoreProvider.score;
      }
    } catch (e) {
      // Use default
    }

    // Get burned amount
    let burnedAmount = 0;
    try {
      const state = typeof this.eScoreProvider.getState === 'function'
        ? this.eScoreProvider.getState()
        : this.eScoreProvider;

      burnedAmount = state?.raw?.burnedTotal ||
                     state?.totalBurns ||
                     context.burnedAmount ||
                     0;
    } catch (e) {
      // Use default
    }

    // φ-based logarithm: log_φ(x) = ln(x) / ln(φ)
    const PHI = 1.618033988749895;
    const logPhi = Math.log(PHI);

    // Calculate: (eScore/100) × log_φ(burnedAmount + 1)
    // - eScore/100 normalizes to 0-1
    // - log_φ provides diminishing returns on burns
    // - +1 ensures log(0) is avoided
    const normalizedEScore = Math.max(0, Math.min(100, eScore)) / 100;
    const burnMultiplier = Math.log(burnedAmount + 1) / logPhi;

    // Final weight: at least DEFAULT_WEIGHT, scaled by e-score and burns
    // If no burns (burnMultiplier ~= 0), weight ≈ DEFAULT_WEIGHT × normalizedEScore
    // With burns, weight increases logarithmically
    const voteWeight = DEFAULT_WEIGHT * (normalizedEScore + burnMultiplier * normalizedEScore);

    // Ensure minimum weight of 0.1 (even low e-score gets some vote)
    return Math.max(0.1, voteWeight);
  }

  /**
   * Update stats
   * @private
   */
  _updateStats(judgment) {
    this.stats.totalJudgments++;
    this.stats.verdicts[judgment.verdict]++;

    // Rolling average
    const n = this.stats.totalJudgments;
    this.stats.avgScore =
      (this.stats.avgScore * (n - 1) + judgment.global_score) / n;
  }

  /**
   * Judge an item without applying automatic skepticism
   * Useful when you want to analyze skepticism separately
   * @param {Object} item - Item to judge
   * @param {Object} [context] - Judgment context
   * @returns {Object} Judgment result (without skepticism applied)
   */
  judgeRaw(item, context = {}) {
    const originalSetting = this.applySkepticism;
    this.applySkepticism = false;
    const judgment = this.judge(item, context);
    this.applySkepticism = originalSetting;
    return judgment;
  }

  /**
   * Analyze skepticism for an existing judgment
   * "φ distrusts φ" - explicit doubt analysis
   * @param {Object} judgment - Previously created judgment
   * @param {Object} [context] - Additional context
   * @returns {Object|null} Skepticism analysis or null if no skeptic
   */
  analyzeSkepticism(judgment, context = {}) {
    if (!this.selfSkeptic) {
      return null;
    }
    return this.selfSkeptic.doubt(judgment, context);
  }

  /**
   * Get anomaly candidates for dimension discovery
   * @returns {Object[]} Anomaly candidates
   */
  getAnomalyCandidates() {
    return this.anomalyBuffer.filter((a) => a.residual > PHI_INV_2);
  }

  /**
   * Get candidate dimensions discovered by ResidualDetector
   * THE_UNNAMEABLE reveals patterns that current dimensions don't capture
   * @returns {Object[]} Candidate dimensions (empty if no detector)
   */
  getCandidateDimensions() {
    if (!this.residualDetector) {
      return [];
    }
    return this.residualDetector.getCandidates();
  }

  /**
   * Accept a candidate dimension (promotes it to actual dimension)
   * Requires governance - humans must approve new dimensions
   * @param {string} candidateKey - Candidate key from getCandidateDimensions()
   * @param {Object} params - Acceptance parameters
   * @param {string} params.name - Final dimension name (e.g., 'SUSTAINABILITY')
   * @param {string} params.axiom - Axiom to add to (PHI, VERIFY, CULTURE, BURN)
   * @param {number} [params.weight] - Dimension weight (default: 1.0)
   * @param {number} [params.threshold] - Score threshold (default: 50)
   * @returns {Object|null} Accepted dimension or null if no detector
   */
  acceptCandidateDimension(candidateKey, params) {
    if (!this.residualDetector) {
      return null;
    }
    return this.residualDetector.acceptCandidate(candidateKey, params);
  }

  /**
   * Reject a candidate dimension
   * @param {string} candidateKey - Candidate key to reject
   */
  rejectCandidateDimension(candidateKey) {
    if (this.residualDetector) {
      this.residualDetector.rejectCandidate(candidateKey);
    }
  }

  /**
   * Get discovered dimensions history
   * @returns {Object[]} Previously accepted dimensions
   */
  getDiscoveredDimensions() {
    if (!this.residualDetector) {
      return [];
    }
    return this.residualDetector.getDiscoveries();
  }

  /**
   * Get ResidualDetector statistics
   * @returns {Object|null} Stats or null if no detector
   */
  getResidualStats() {
    if (!this.residualDetector) {
      return null;
    }
    return this.residualDetector.getStats();
  }

  /**
   * Get judge statistics
   * @returns {Object} Statistics
   */
  getStats() {
    return {
      ...this.stats,
      avgScore: Math.round(this.stats.avgScore * 10) / 10,
      anomalyRate: this.stats.totalJudgments > 0
        ? Math.round(this.stats.anomaliesDetected / this.stats.totalJudgments * 1000) / 1000
        : 0,
    };
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      totalJudgments: 0,
      verdicts: { HOWL: 0, WAG: 0, GROWL: 0, BARK: 0 },
      anomaliesDetected: 0,
      avgScore: 0,
    };
    this.anomalyBuffer = [];
  }
}

export default { CYNICJudge };

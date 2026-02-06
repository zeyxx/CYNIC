/**
 * @cynic/agent - Decider Module
 *
 * Judges opportunities and decides actions using CYNIC 25-dimension system.
 * Delegates to the REAL CYNICJudge brain when available.
 * "Le doute est la sagesse du chien" - κυνικός
 *
 * @module @cynic/agent/decider
 */

'use strict';

import { EventEmitter } from 'eventemitter3';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus, EventType } from '@cynic/core';

const log = createLogger('Decider');

// Lazy-load CYNICJudge to avoid hard dependency
let _judgeModules = null;
async function loadJudgeModules() {
  if (_judgeModules) return _judgeModules;
  try {
    const { CYNICJudge, SelfSkeptic, ResidualDetector } = await import('@cynic/node/judge');
    _judgeModules = { CYNICJudge, SelfSkeptic, ResidualDetector };
    return _judgeModules;
  } catch (e) {
    log.debug('CYNICJudge modules not available', { error: e.message });
    return null;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Decision Types
// ═══════════════════════════════════════════════════════════════════════════════

export const Action = {
  BUY: 'BUY',
  SELL: 'SELL',
  HOLD: 'HOLD',
};

export const Verdict = {
  STRONG_BUY: 'STRONG_BUY',   // High confidence long
  BUY: 'BUY',                 // Moderate confidence long
  HOLD: 'HOLD',               // No action
  SELL: 'SELL',               // Moderate confidence short/exit
  STRONG_SELL: 'STRONG_SELL', // High confidence short/exit
};

// ═══════════════════════════════════════════════════════════════════════════════
// 25 Judgment Dimensions (from CYNIC ontology)
// ═══════════════════════════════════════════════════════════════════════════════

export const Dimensions = {
  // Reality Perception
  AUTHENTICITY: 'authenticity',       // Is the signal real or manipulated?
  TIMING: 'timing',                   // Is timing favorable?
  LIQUIDITY: 'liquidity',             // Can we enter/exit without slippage?
  VOLATILITY: 'volatility',           // Risk from price swings?

  // Token Quality
  TOKEN_QUALITY: 'token_quality',     // Overall token fundamentals
  TEAM: 'team',                       // Team credibility
  CONTRACT: 'contract',               // Contract safety (rug risk)
  COMMUNITY: 'community',             // Community strength

  // Market Context
  TREND: 'trend',                     // Market trend alignment
  SENTIMENT: 'sentiment',             // Social sentiment
  MOMENTUM: 'momentum',               // Price momentum
  VOLUME: 'volume',                   // Volume confirmation

  // Risk Assessment
  RISK_REWARD: 'risk_reward',         // R:R ratio
  POSITION_SIZE: 'position_size',     // Appropriate size?
  CORRELATION: 'correlation',         // Portfolio correlation
  DRAWDOWN: 'drawdown',               // Max drawdown risk

  // Technical Signals
  SUPPORT_RESISTANCE: 'support_resistance',
  BREAKOUT: 'breakout',
  DIVERGENCE: 'divergence',
  PATTERN: 'pattern',

  // Meta
  CONFIDENCE: 'confidence',           // Self-assessed confidence
  NOVELTY: 'novelty',                 // How new is this pattern?
  HISTORY: 'history',                 // Past performance on similar
  ALIGNMENT: 'alignment',             // Strategy alignment

  // The Unnameable (residual)
  THE_UNNAMEABLE: 'the_unnameable',   // Unknown unknowns
};

// ═══════════════════════════════════════════════════════════════════════════════
// Decider Class
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Decider - Judges opportunities and decides actions
 *
 * ADAPT not REPLACE: Decider stays as agent-facing interface.
 * When CYNICJudge is available, delegates scoring to the real 25D brain.
 * Falls back to basic scoring when CYNICJudge is unavailable.
 */
export class Decider extends EventEmitter {
  constructor(options = {}) {
    super();

    this.options = options;

    // Dimension weights (φ-aligned)
    this.weights = this._initWeights();

    // Weight adjustments from Learner feedback
    this._weightAdjustments = {};

    // Metrics
    this.metrics = {
      judgments: 0,
      decisions: 0,
      buys: 0,
      sells: 0,
      holds: 0,
      cynicJudgeUsed: 0,
      fallbackUsed: 0,
    };

    // History for learning
    this.history = [];
    this.maxHistory = 100;

    // CYNICJudge (wired lazily on first judge() call)
    this.cynicJudge = null;
    this._judgeInitialized = false;
  }

  /**
   * Wire the REAL CYNICJudge brain (lazy init on first use)
   * @private
   */
  async _ensureCynicJudge() {
    if (this._judgeInitialized) return;
    this._judgeInitialized = true;

    try {
      const modules = await loadJudgeModules();
      if (!modules) return;

      const { CYNICJudge, SelfSkeptic, ResidualDetector } = modules;

      const selfSkeptic = new SelfSkeptic();
      const residualDetector = new ResidualDetector();

      this.cynicJudge = new CYNICJudge({
        selfSkeptic,
        residualDetector,
        applySkepticism: true,
        includeUnnameable: true,
      });

      this._registerTradingDimensions();
      log.info('CYNICJudge wired successfully');
    } catch (e) {
      log.warn('CYNICJudge unavailable, using basic scoring', { error: e.message });
      this.cynicJudge = null;
    }
  }

  /**
   * Register trading-specific dimension scorers via plugin API
   * @private
   */
  _registerTradingDimensions() {
    if (!this.cynicJudge?.registerPlugin) return;

    try {
      this.cynicJudge.registerPlugin({
        name: 'trading-agent',
        dimensions: [
          {
            name: 'LIQUIDITY',
            axiom: 'VERIFY',
            scorer: (item, context) => {
              const liquidity = context?.marketData?.liquidity || item?.marketData?.liquidity;
              if (!liquidity) return 0.5;
              if (liquidity > 100000) return 0.8;
              if (liquidity > 10000) return 0.6;
              return 0.3;
            },
          },
          {
            name: 'VOLATILITY',
            axiom: 'PHI',
            scorer: (item, context) => {
              const magnitude = context?.magnitude || Math.abs(item?.marketData?.changePercent || 0);
              // High volatility = more risk = lower score
              if (magnitude > 0.3) return 0.3;
              if (magnitude > 0.15) return 0.5;
              return 0.7;
            },
          },
          {
            name: 'VOLUME_CONFIRMATION',
            axiom: 'VERIFY',
            scorer: (item, context) => {
              const current = context?.marketData?.currentVolume || item?.marketData?.currentVolume;
              const previous = context?.marketData?.previousVolume || item?.marketData?.previousVolume;
              if (!current || !previous || previous === 0) return 0.5;
              const ratio = current / previous;
              if (ratio > 2.0) return 0.85;
              if (ratio > 1.5) return 0.7;
              if (ratio > 1.0) return 0.55;
              return 0.3;
            },
          },
          {
            name: 'TIMING',
            axiom: 'CULTURE',
            scorer: () => {
              const hour = new Date().getUTCHours();
              // Highest activity: 13-21 UTC (US market hours)
              if (hour >= 13 && hour <= 21) return 0.7;
              // Secondary: 7-13 UTC (EU hours)
              if (hour >= 7 && hour < 13) return 0.55;
              return 0.4;
            },
          },
          {
            name: 'RISK_REWARD',
            axiom: 'BURN',
            scorer: (item, context) => {
              const magnitude = context?.magnitude || Math.abs(item?.marketData?.changePercent || 0);
              // Good magnitude = good R:R potential
              if (magnitude > 0.1 && magnitude < 0.3) return 0.7;
              if (magnitude > 0.05) return 0.55;
              return 0.35;
            },
          },
        ],
      });
      log.debug('Trading dimensions registered');
    } catch (e) {
      log.debug('Trading dimension registration failed', { error: e.message });
    }
  }

  /**
   * Initialize dimension weights
   * @private
   */
  _initWeights() {
    const weights = {};

    // Critical dimensions get φ⁻¹ weight
    const critical = [
      Dimensions.CONTRACT,
      Dimensions.LIQUIDITY,
      Dimensions.RISK_REWARD,
    ];

    // Important dimensions get φ⁻² weight
    const important = [
      Dimensions.AUTHENTICITY,
      Dimensions.TOKEN_QUALITY,
      Dimensions.TREND,
      Dimensions.VOLUME,
    ];

    // All others get φ⁻³ weight
    for (const dim of Object.values(Dimensions)) {
      if (critical.includes(dim)) {
        weights[dim] = PHI_INV;
      } else if (important.includes(dim)) {
        weights[dim] = PHI_INV_2;
      } else {
        weights[dim] = PHI_INV_3;
      }
    }

    return weights;
  }

  /**
   * Convert opportunity format to CYNICJudge item format
   * @private
   */
  _opportunityToItem(opportunity) {
    const { signal, magnitude, direction, token, mint } = opportunity;
    const data = signal?.data || {};

    return {
      type: 'trading_opportunity',
      itemType: 'market_signal',
      content: `${direction} ${token}: ${signal?.type || 'signal'} magnitude=${(magnitude * 100).toFixed(1)}%`,
      identifier: mint || token,
      name: token,
      scores: {},
      quality: magnitude > 0.1 ? 'high' : magnitude > 0.05 ? 'medium' : 'low',
      marketData: {
        changePercent: data.changePercent || magnitude * (direction === 'SHORT' ? -1 : 1),
        currentVolume: data.currentVolume,
        previousVolume: data.previousVolume,
        liquidity: data.liquidity || 0,
        priceUsd: data.priceUsd || data.currentPrice,
        direction,
        magnitude,
        signalType: signal?.type,
      },
    };
  }

  /**
   * Map CYNICJudge verdict (HOWL/WAG/GROWL/BARK) to agent verdict
   * @private
   */
  _mapVerdict(cynicVerdict, qScore, opportunity) {
    const direction = opportunity?.direction || 'LONG';

    switch (cynicVerdict) {
      case 'HOWL':
        return direction === 'LONG' ? Verdict.STRONG_BUY : Verdict.STRONG_SELL;
      case 'WAG':
        return direction === 'LONG' ? Verdict.BUY : Verdict.SELL;
      case 'GROWL':
      case 'BARK':
        return Verdict.HOLD;
      default:
        // Fallback to Q-Score based
        return this._determineVerdict(qScore, {});
    }
  }

  /**
   * Judge an opportunity across 25 dimensions
   *
   * @param {Object} opportunity - The opportunity to judge
   * @returns {Object} Judgment with scores and verdict
   */
  async judge(opportunity) {
    this.metrics.judgments++;

    // Ensure CYNICJudge is loaded
    await this._ensureCynicJudge();

    // Try CYNICJudge first
    if (this.cynicJudge) {
      try {
        return await this._judgeWithCynicJudge(opportunity);
      } catch (e) {
        log.warn('CYNICJudge failed, falling back to basic scoring', { error: e.message });
        this.metrics.fallbackUsed++;
      }
    }

    // Fallback: basic scoring
    return this._judgeBasic(opportunity);
  }

  /**
   * Judge using the real CYNICJudge brain
   * @private
   */
  async _judgeWithCynicJudge(opportunity) {
    this.metrics.cynicJudgeUsed++;

    const item = this._opportunityToItem(opportunity);
    const context = {
      source: 'cynic-agent',
      marketData: item.marketData,
      magnitude: opportunity.magnitude,
      direction: opportunity.direction,
      weightAdjustments: this._weightAdjustments,
    };

    const cynicResult = await this.cynicJudge.judge(item, context);

    // Extract scores - map axiomScores to our dimension space
    const scores = {};
    const axiomScores = cynicResult.axiomScores || cynicResult.axiom_scores || {};

    // Map CYNICJudge axiom scores to agent dimensions
    for (const dim of Object.values(Dimensions)) {
      // Check if CYNICJudge scored this dimension directly
      if (cynicResult.dimensions?.[dim] !== undefined) {
        scores[dim] = cynicResult.dimensions[dim];
      } else {
        // Map from axiom categories
        scores[dim] = this._mapAxiomToDimension(dim, axiomScores, item.marketData);
      }
    }

    // Apply weight adjustments from learning
    for (const [dim, adj] of Object.entries(this._weightAdjustments)) {
      if (scores[dim] !== undefined) {
        scores[dim] = Math.max(0, Math.min(1, scores[dim] + adj));
      }
    }

    const qScore = cynicResult.q_score || cynicResult.qScore || cynicResult.global_score || 50;
    const rawConfidence = cynicResult.confidence || this._calculateConfidence(scores);
    const confidence = Math.min(rawConfidence, PHI_INV);

    // Map HOWL/WAG/GROWL/BARK → agent verdicts
    const cynicVerdict = cynicResult.qVerdict || cynicResult.verdict || cynicResult.q_verdict;
    const verdict = this._mapVerdict(cynicVerdict, qScore, opportunity);

    const judgment = {
      id: `jdg_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      opportunityId: opportunity.id,
      timestamp: Date.now(),
      qScore,
      verdict,
      confidence,
      scores,
      opportunity,
      // Rich data from CYNICJudge
      entropy: cynicResult.entropy || null,
      skepticism: cynicResult.skepticism || null,
      axiomScores,
      residual: cynicResult.residual || null,
      markov: cynicResult.markov || null,
      source: 'CYNICJudge',
    };

    this._recordJudgment(judgment);
    this.emit('judgment', judgment);

    log.info('CYNICJudge judgment', {
      qScore,
      verdict,
      confidence: confidence.toFixed(3),
      entropy: cynicResult.entropy?.category,
      skepticism: !!cynicResult.skepticism,
    });

    return judgment;
  }

  /**
   * Map axiom scores to agent dimensions
   * @private
   */
  _mapAxiomToDimension(dimension, axiomScores, marketData) {
    const phi = (axiomScores.PHI || axiomScores.phi || 50) / 100;
    const verify = (axiomScores.VERIFY || axiomScores.verify || 50) / 100;
    const culture = (axiomScores.CULTURE || axiomScores.culture || 50) / 100;
    const burn = (axiomScores.BURN || axiomScores.burn || 50) / 100;

    switch (dimension) {
      // PHI-aligned dimensions
      case Dimensions.CONFIDENCE:
      case Dimensions.VOLATILITY:
      case Dimensions.POSITION_SIZE:
      case Dimensions.DRAWDOWN:
        return phi;

      // VERIFY-aligned dimensions
      case Dimensions.AUTHENTICITY:
      case Dimensions.CONTRACT:
      case Dimensions.LIQUIDITY:
      case Dimensions.VOLUME:
        return verify;

      // CULTURE-aligned dimensions
      case Dimensions.TIMING:
      case Dimensions.COMMUNITY:
      case Dimensions.SENTIMENT:
      case Dimensions.HISTORY:
        return culture;

      // BURN-aligned dimensions
      case Dimensions.RISK_REWARD:
      case Dimensions.TOKEN_QUALITY:
      case Dimensions.TREND:
      case Dimensions.MOMENTUM:
        return burn;

      // Average for remaining
      default:
        return (phi + verify + culture + burn) / 4;
    }
  }

  /**
   * Basic scoring fallback (original logic)
   * @private
   */
  async _judgeBasic(opportunity) {
    this.metrics.fallbackUsed++;

    const scores = {};
    let totalWeight = 0;
    let weightedSum = 0;

    // Score each dimension
    for (const [dim, weight] of Object.entries(this.weights)) {
      const score = await this._scoreDimension(dim, opportunity);
      scores[dim] = score;

      weightedSum += score * weight;
      totalWeight += weight;
    }

    // Calculate Q-Score (0-100)
    const qScore = Math.round((weightedSum / totalWeight) * 100);

    // Determine verdict
    const verdict = this._determineVerdict(qScore, scores);

    // Calculate confidence (capped at φ⁻¹)
    const rawConfidence = this._calculateConfidence(scores);
    const confidence = Math.min(rawConfidence, PHI_INV);

    const judgment = {
      id: `jdg_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      opportunityId: opportunity.id,
      timestamp: Date.now(),
      qScore,
      verdict,
      confidence,
      scores,
      opportunity,
      source: 'basic',
    };

    // Record for learning
    this._recordJudgment(judgment);

    this.emit('judgment', judgment);
    log.info('Basic judgment', { qScore, verdict, confidence: confidence.toFixed(3) });

    return judgment;
  }

  /**
   * Score a single dimension (basic fallback)
   * @private
   */
  async _scoreDimension(dimension, opportunity) {
    const { signal, magnitude, direction } = opportunity;

    switch (dimension) {
      case Dimensions.AUTHENTICITY:
        return magnitude > 0.3 ? 0.4 : 0.7;

      case Dimensions.TIMING: {
        const hour = new Date().getUTCHours();
        return (hour >= 13 && hour <= 21) ? 0.7 : 0.5;
      }

      case Dimensions.LIQUIDITY:
        return 0.6;

      case Dimensions.VOLATILITY:
        return magnitude > 0.2 ? 0.4 : 0.7;

      case Dimensions.TOKEN_QUALITY:
        return 0.5;

      case Dimensions.CONTRACT:
        return 0.5;

      case Dimensions.TREND:
        return direction === 'LONG' ? 0.6 : 0.4;

      case Dimensions.VOLUME:
        return signal?.data?.currentVolume > signal?.data?.previousVolume ? 0.7 : 0.4;

      case Dimensions.RISK_REWARD:
        return magnitude > 0.1 ? 0.6 : 0.4;

      case Dimensions.THE_UNNAMEABLE:
        return 0.5;

      default:
        return 0.5;
    }
  }

  /**
   * Determine verdict from Q-Score and dimensions
   * @private
   */
  _determineVerdict(qScore, scores) {
    // Red flags that override Q-Score
    if (scores[Dimensions.CONTRACT] < 0.3) return Verdict.STRONG_SELL;
    if (scores[Dimensions.LIQUIDITY] < 0.2) return Verdict.HOLD;

    // Q-Score based verdict
    if (qScore >= 75) return Verdict.STRONG_BUY;
    if (qScore >= 60) return Verdict.BUY;
    if (qScore >= 40) return Verdict.HOLD;
    if (qScore >= 25) return Verdict.SELL;
    return Verdict.STRONG_SELL;
  }

  /**
   * Calculate confidence from score variance
   * @private
   */
  _calculateConfidence(scores) {
    const values = Object.values(scores);
    const mean = values.reduce((a, b) => a + b, 0) / values.length;

    // Lower variance = higher confidence
    const variance = values.reduce((sum, v) => sum + Math.pow(v - mean, 2), 0) / values.length;
    const stdDev = Math.sqrt(variance);

    // High stdDev = low confidence
    const confidence = 1 - stdDev;
    return Math.max(0.1, Math.min(PHI_INV, confidence));
  }

  /**
   * Decide action from judgment
   *
   * @param {Object} judgment - The judgment
   * @param {Object} options - Decision options
   * @returns {Object} Decision with action and parameters
   */
  async decide(judgment, options = {}) {
    const {
      maxConfidence = PHI_INV,
      minConfidence = PHI_INV_2,
    } = options;

    this.metrics.decisions++;

    // Cap confidence
    const confidence = Math.min(judgment.confidence, maxConfidence);

    // Determine action
    let action = Action.HOLD;
    let size = 0;

    if (confidence >= minConfidence) {
      switch (judgment.verdict) {
        case Verdict.STRONG_BUY:
        case Verdict.BUY:
          action = Action.BUY;
          size = this._calculatePositionSize(confidence, judgment.qScore);
          this.metrics.buys++;
          break;

        case Verdict.STRONG_SELL:
        case Verdict.SELL:
          action = Action.SELL;
          size = this._calculatePositionSize(confidence, judgment.qScore);
          this.metrics.sells++;
          break;

        default:
          this.metrics.holds++;
      }
    } else {
      this.metrics.holds++;
    }

    const decision = {
      id: `dec_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      judgmentId: judgment.id,
      timestamp: Date.now(),
      action,
      confidence,
      verdict: judgment.verdict,
      qScore: judgment.qScore,
      size,
      token: judgment.opportunity.token,
      mint: judgment.opportunity.mint,
      reason: this._generateReason(judgment, action),
    };

    this.emit('decision', decision);
    log.info('Decision made', { action, confidence: confidence.toFixed(3), size });

    return decision;
  }

  /**
   * Apply weight adjustments from Learner feedback
   * Closes the learning loop: Learner → Decider
   *
   * @param {Object} adjustments - Map of dimension → adjustment value
   */
  applyWeightAdjustments(adjustments) {
    this._weightAdjustments = { ...adjustments };
    log.debug('Weight adjustments applied', {
      dimensions: Object.keys(adjustments).length,
    });
  }

  /**
   * Calculate position size from confidence
   * @private
   */
  _calculatePositionSize(confidence, qScore) {
    const maxPosition = 0.10;
    const minPosition = 0.01;

    const scaleFactor = (confidence / PHI_INV) * (qScore / 100);
    const size = minPosition + (maxPosition - minPosition) * scaleFactor;

    return Math.round(size * 1000) / 1000;
  }

  /**
   * Generate human-readable reason
   * @private
   */
  _generateReason(judgment, action) {
    const { qScore, confidence, scores } = judgment;

    if (action === Action.HOLD) {
      if (confidence < PHI_INV_2) {
        return `Confidence too low (${(confidence * 100).toFixed(1)}% < ${(PHI_INV_2 * 100).toFixed(1)}%)`;
      }
      return `Q-Score neutral (${qScore}/100)`;
    }

    // Find strongest contributing dimensions
    const sorted = Object.entries(scores)
      .filter(([k]) => k !== Dimensions.THE_UNNAMEABLE)
      .sort((a, b) => b[1] - a[1]);

    const top3 = sorted.slice(0, 3).map(([k]) => k);

    return `${action} signal: Q=${qScore}, conf=${(confidence * 100).toFixed(1)}%. Top factors: ${top3.join(', ')}`;
  }

  /**
   * Record judgment for history
   * @private
   */
  _recordJudgment(judgment) {
    this.history.push({
      id: judgment.id,
      timestamp: judgment.timestamp,
      qScore: judgment.qScore,
      verdict: judgment.verdict,
      confidence: judgment.confidence,
    });

    while (this.history.length > this.maxHistory) {
      this.history.shift();
    }
  }

  /**
   * Get status
   */
  getStatus() {
    return {
      metrics: { ...this.metrics },
      historySize: this.history.length,
      weights: Object.keys(this.weights).length,
      cynicJudge: !!this.cynicJudge,
      weightAdjustments: Object.keys(this._weightAdjustments).length,
    };
  }
}

export default Decider;

/**
 * Thompson Sampling for suggestion selection
 *
 * Uses Beta distribution for binary outcomes (followed/not followed).
 * Bayesian exploration/exploitation with φ-bounded confidence.
 *
 * Extracted from harmonic-feedback.js for reuse.
 *
 * "Le chien explore avec sagesse" - κυνικός
 *
 * @module @cynic/node/learning/thompson-sampler
 */

'use strict';

// ═══════════════════════════════════════════════════════════════════════════
// φ CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

import { PHI_INV } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// THOMPSON SAMPLER
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

export { ThompsonSampler };
export default ThompsonSampler;

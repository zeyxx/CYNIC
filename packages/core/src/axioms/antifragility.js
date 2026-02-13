/**
 * ANTIFRAGILITY Axiom — "Gains from Disorder"
 *
 * The 9th Axiom of CYNIC
 *
 * ## Principle
 *
 * **Chaos makes the organism STRONGER, not weaker.**
 *
 * Antifragility (Nassim Taleb): Beyond robustness. Robust systems resist stress.
 * Antifragile systems IMPROVE under stress. Volatility, randomness, and errors
 * are FUEL for growth, not obstacles.
 *
 * ## Violations
 *
 * ❌ Avoiding all errors/failures
 * ❌ Seeking perfect stability
 * ❌ "Zero downtime" as goal
 * ❌ Suppressing variation
 * ❌ Single point of failure
 *
 * ## Compliance
 *
 * ✅ Learn MORE from failures than successes
 * ✅ Chaos testing reveals weaknesses
 * ✅ Redundancy + diversity (11 Dogs, not 1)
 * ✅ Small frequent failures > catastrophic single failure
 * ✅ Volatility as signal (market swings → trading opportunities)
 *
 * ## Why Critical
 *
 * Without ANTIFRAGILITY:
 * - Organism fragile to unexpected events
 * - Errors seen as pure cost (not learning opportunities)
 * - Optimization for current environment (brittle when environment shifts)
 * - Fear of experimentation (might break things!)
 *
 * ## Relationship to Other Axioms
 *
 * - **VERIFY**: Failures captured on blockchain (learn from mistakes)
 * - **FIDELITY**: Doubt enables antifragility (always testing assumptions)
 * - **BURN**: Simplicity = antifragile (fewer failure modes)
 * - **AUTONOMY**: Required to act on chaos signals
 * - **EMERGENCE**: Chaos reveals emergent patterns
 *
 * ## Examples in CYNIC
 *
 * 1. **Chaos Generator**: Inject faults to discover weaknesses
 * 2. **Circuit Breakers**: Fail fast, recover fast (graceful degradation)
 * 3. **Multi-tier LLM**: Fallback from Opus → Sonnet → Haiku → Ollama
 * 4. **Thompson Sampling**: Explore suboptimal arms (volatility reveals information)
 * 5. **11 Dogs**: Redundancy means single dog failure doesn't kill organism
 * 6. **Residual variance**: What doesn't fit model = signal for growth
 *
 * ## Barbell Strategy
 *
 * Taleb's barbell: 90% safe + 10% high-risk (asymmetric payoff)
 * CYNIC: 90% proven patterns + 10% experiments (φ⁻³ falsification rate)
 *
 * ## Implementation
 *
 * See:
 * - packages/node/src/orchestration/chaos-generator.js (chaos engineering)
 * - packages/core/src/circuit-breaker.js (fail-fast, recover-fast)
 * - packages/node/src/learning/model-intelligence.js (multi-tier fallback)
 * - packages/node/src/learning/thompson-sampler.js (exploration under uncertainty)
 *
 * @module @cynic/core/axioms/antifragility
 */

'use strict';

import { PHI, PHI_INV, PHI_INV_3 } from './constants.js';

/**
 * ANTIFRAGILITY configuration
 */
export const ANTIFRAGILITY = Object.freeze({
  name: 'ANTIFRAGILITY',
  principle: 'Gains from disorder',
  chaosInjectionRate: PHI_INV_3, // φ⁻³ = 3.8% of operations get chaos injected
  failureLearnWeight: PHI,       // φ = 1.618× more learning from failures than successes
  barbellSafeRatio: 0.90,       // 90% safe, 10% experimental
  enableChaosTest: true,
  enableGracefulDegradation: true,
  enableRedundancy: true,
});

/**
 * Calculate antifragility score
 *
 * Measures how much organism benefits from volatility/errors
 *
 * @param {Object} metrics
 * @param {number} metrics.learningFromFailures - Learning events from failures
 * @param {number} metrics.learningFromSuccesses - Learning events from successes
 * @param {number} metrics.performanceAfterChaos - Performance after chaos injection
 * @param {number} metrics.performanceBeforeChaos - Performance before chaos
 * @returns {Object} { score: number, antifragile: boolean, reason: string }
 */
export function antifragilityScore(metrics) {
  // Check if organism learns more from failures than successes
  const totalLearning = metrics.learningFromFailures + metrics.learningFromSuccesses;
  if (totalLearning === 0) {
    return {
      score: 0,
      antifragile: false,
      reason: 'No learning events yet',
    };
  }

  const failureLearnRatio = metrics.learningFromFailures / totalLearning;

  // Check if performance improved after chaos
  const chaosImprovement = metrics.performanceBeforeChaos === 0
    ? 0
    : (metrics.performanceAfterChaos - metrics.performanceBeforeChaos) / metrics.performanceBeforeChaos;

  // Antifragile if:
  // 1. Learn more from failures (>50%)
  // 2. Chaos improves performance (positive improvement)
  const antifragile = failureLearnRatio > 0.5 && chaosImprovement > 0;

  // Score = weighted sum
  // - 61.8% weight on failure learning
  // - 38.2% weight on chaos improvement (capped at φ)
  const score = PHI_INV * failureLearnRatio + PHI_INV_2 * Math.min(chaosImprovement, PHI);

  return {
    score,
    antifragile,
    reason: antifragile
      ? `Learns from failures (${(failureLearnRatio * 100).toFixed(1)}%) + chaos improves performance (${(chaosImprovement * 100).toFixed(1)}%)`
      : failureLearnRatio <= 0.5
        ? 'Learns mostly from successes (fragile)'
        : 'Chaos degrades performance (robust but not antifragile)',
  };
}

/**
 * Check if failure mode is antifragile (small, frequent, recoverable)
 *
 * @param {Object} failure
 * @param {string} failure.type - Failure type
 * @param {number} failure.cost - Cost of failure (0-1)
 * @param {number} failure.frequency - Failures per day
 * @param {boolean} failure.recoverable - Can system recover automatically?
 * @returns {Object} { antifragile: boolean, reason: string }
 */
export function isAntifragileFailure(failure) {
  // Catastrophic failure = NOT antifragile
  if (failure.cost > PHI_INV) {
    return {
      antifragile: false,
      reason: `Cost too high (${(failure.cost * 100).toFixed(1)}% > φ⁻¹)`,
    };
  }

  // Can't recover = NOT antifragile
  if (!failure.recoverable) {
    return {
      antifragile: false,
      reason: 'Not recoverable (permanent damage)',
    };
  }

  // Rare failures = fragile (no opportunity to learn)
  if (failure.frequency < 0.1) {
    return {
      antifragile: false,
      reason: 'Too rare to learn from (< 0.1/day)',
    };
  }

  // Small, frequent, recoverable = ANTIFRAGILE!
  return {
    antifragile: true,
    reason: `Small (${(failure.cost * 100).toFixed(1)}%), frequent (${failure.frequency.toFixed(1)}/day), recoverable`,
  };
}

/**
 * Calculate barbell allocation
 *
 * 90% safe + 10% experimental (asymmetric upside)
 *
 * @param {number} totalBudget - Total budget available
 * @param {Object} [options] - Options
 * @param {number} [options.safeRatio=0.90] - Ratio allocated to safe operations
 * @returns {Object} { safe: number, experimental: number }
 */
export function barbellAllocation(totalBudget, options = {}) {
  const safeRatio = options.safeRatio ?? ANTIFRAGILITY.barbellSafeRatio;
  const experimentalRatio = 1 - safeRatio;

  return {
    safe: totalBudget * safeRatio,
    experimental: totalBudget * experimentalRatio,
    safeRatio,
    experimentalRatio,
  };
}

/**
 * Check if redundancy is sufficient for antifragility
 *
 * @param {Object} system
 * @param {number} system.components - Total components
 * @param {number} system.minRequired - Minimum components needed for function
 * @param {number} system.diversity - How diverse are components (0-1)
 * @returns {Object} { antifragile: boolean, redundancy: number, reason: string }
 */
export function checkRedundancy(system) {
  if (system.components === 0 || system.minRequired === 0) {
    return {
      antifragile: false,
      redundancy: 0,
      reason: 'No components',
    };
  }

  const redundancy = (system.components - system.minRequired) / system.minRequired;

  // Need at least φ redundancy (61.8% extra capacity)
  if (redundancy < PHI_INV) {
    return {
      antifragile: false,
      redundancy,
      reason: `Insufficient redundancy (${(redundancy * 100).toFixed(1)}% < φ⁻¹)`,
    };
  }

  // Redundancy without diversity = monoculture (fragile to common-mode failure)
  if (system.diversity < 0.5) {
    return {
      antifragile: false,
      redundancy,
      reason: 'Low diversity (monoculture = fragile)',
    };
  }

  return {
    antifragile: true,
    redundancy,
    reason: `${(redundancy * 100).toFixed(1)}% redundancy + ${(system.diversity * 100).toFixed(1)}% diversity`,
  };
}

/**
 * Validate axiom implementation
 *
 * @returns {Object} { compliant: boolean, issues: string[] }
 */
export function validateAntifragility() {
  const issues = [];

  // Check if chaos generator exists
  let hasChaos = false;
  try {
    require.resolve('../../node/src/orchestration/chaos-generator.js');
    hasChaos = true;
  } catch {
    issues.push('ChaosGenerator not found');
  }

  // Check if circuit breaker exists
  let hasCircuitBreaker = false;
  try {
    require.resolve('../circuit-breaker.js');
    hasCircuitBreaker = true;
  } catch {
    issues.push('Circuit breaker not found');
  }

  // Check if multi-tier LLM exists
  let hasMultiTier = false;
  try {
    require.resolve('../../node/src/learning/model-intelligence.js');
    hasMultiTier = true;
  } catch {
    issues.push('Multi-tier LLM (antifragile fallback) not found');
  }

  return {
    compliant: hasChaos && hasCircuitBreaker && hasMultiTier && issues.length === 0,
    issues,
  };
}

/**
 * Axiom metadata
 */
export const AntifragilityAxiom = Object.freeze({
  id: 9,
  name: 'ANTIFRAGILITY',
  symbol: '💪',
  principle: 'Gains from disorder',
  tradeName: 'Chaos as Fuel',
  established: '2026-02-13',
  criticality: 'CRITICAL',
  dependencies: ['VERIFY', 'FIDELITY', 'AUTONOMY'],
  enables: ['RESILIENCE', 'EVOLUTION'],
  references: [
    'Nassim Taleb - Antifragile: Things That Gain from Disorder',
    'Small frequent failures > catastrophic single failure',
    'Barbell strategy: 90% safe + 10% experimental',
  ],
});

export default ANTIFRAGILITY;

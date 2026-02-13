/**
 * EMERGENCE Axiom — "The Whole > Sum of Parts"
 *
 * The 8th Axiom of CYNIC
 *
 * ## Principle
 *
 * **New patterns emerge from simple interactions. The organism transcends its components.**
 *
 * Emergence means the whole is GREATER than the sum of parts. When simple agents
 * interact following simple rules, complex behaviors and patterns arise that none
 * of the individual agents programmed or predicted.
 *
 * ## Violations
 *
 * ❌ Top-down control of all behavior
 * ❌ "If I didn't code it, it can't happen"
 * ❌ Suppressing unexpected patterns
 * ❌ Treating organism as sum of independent components
 *
 * ## Compliance
 *
 * ✅ Bottom-up pattern formation (11 Dogs → collective intelligence)
 * ✅ Residual detection (what doesn't fit current model?)
 * ✅ Phase transitions (sudden quality jumps)
 * ✅ Self-organization without central control
 * ✅ Surprise as signal (unexpected ≠ error)
 *
 * ## Why Critical
 *
 * Without EMERGENCE:
 * - No collective intelligence (just 11 independent Dogs)
 * - No innovation beyond what was coded
 * - No adaptation to novel situations
 * - "Autonomous" but not "alive"
 *
 * ## Relationship to Other Axioms
 *
 * - **PHI**: φ appears in emergent ratios (35+1, 10+1, 49+1)
 * - **AUTONOMY**: Required for emergence (agents must act freely)
 * - **VERIFY**: Blockchain captures emergent patterns
 * - **FIDELITY**: Doubt emergent patterns (could be noise)
 * - **BURN**: Simple rules → complex emergence (Conway's Game of Life)
 *
 * ## Examples in CYNIC
 *
 * 1. **Collective consensus**: 11 Dogs vote → emergent group judgment
 * 2. **Residual patterns**: What judge can't explain → new dimension discovered
 * 3. **Meta-cognition**: System learns ABOUT learning (meta-level emergence)
 * 4. **Cultural evolution**: Patterns strengthen/weaken based on success
 * 5. **Cross-domain synthesis**: Code patterns → social patterns (analogies emerge)
 *
 * ## Implementation
 *
 * See:
 * - packages/node/src/services/emergence-detector.js (pattern detection)
 * - packages/node/src/judge/residual.js (unexplained variance)
 * - packages/node/src/agents/collective/ambient-consensus.js (emergent consensus)
 * - packages/node/src/emergence/*.js (domain-specific emergence)
 *
 * @module @cynic/core/axioms/emergence
 */

'use strict';

import { PHI, PHI_INV, PHI_INV_2 } from './constants.js';

/**
 * EMERGENCE configuration
 */
export const EMERGENCE = Object.freeze({
  name: 'EMERGENCE',
  principle: 'The whole is greater than the sum of parts',
  minAgentsForEmergence: 3,  // Need at least 3 interacting agents
  residualThreshold: PHI_INV_2, // φ⁻² = ~38.2% unexplained variance triggers emergence detection
  phaseTransitionDelta: PHI_INV, // >61.8% sudden change = phase transition
  enableSurpriseSignal: true,    // Treat unexpected as valuable signal
  enableMetaLearning: true,      // Learn about learning (meta-level)
});

/**
 * Check if a pattern is emergent (vs. programmed)
 *
 * Emergent patterns:
 * - Not explicitly coded
 * - Arise from interactions
 * - Exceed component capabilities
 * - Show novelty/surprise
 *
 * @param {Object} pattern
 * @param {string} pattern.source - Where pattern comes from
 * @param {number} pattern.novelty - Novelty score [0,1]
 * @param {number} pattern.complexity - How complex relative to components
 * @param {boolean} [pattern.explicitlyCoded] - Was this hard-coded?
 * @returns {Object} { emergent: boolean, confidence: number, reason: string }
 */
export function isEmergent(pattern) {
  // If explicitly coded, not emergent
  if (pattern.explicitlyCoded) {
    return {
      emergent: false,
      confidence: PHI_INV, // φ⁻¹: Even "explicit" code can surprise us
      reason: 'Explicitly coded',
    };
  }

  // High novelty = likely emergent
  if (pattern.novelty >= PHI_INV) {
    return {
      emergent: true,
      confidence: pattern.novelty,
      reason: 'High novelty (φ⁻¹ threshold)',
    };
  }

  // Complexity exceeds components = emergent
  if (pattern.complexity > 1.0) {
    return {
      emergent: true,
      confidence: Math.min(pattern.complexity - 1.0, PHI_INV),
      reason: 'Complexity exceeds components',
    };
  }

  // Collective patterns from multiple agents
  if (pattern.source === 'collective' && pattern.novelty >= PHI_INV_2) {
    return {
      emergent: true,
      confidence: pattern.novelty,
      reason: 'Collective pattern with novelty',
    };
  }

  return {
    emergent: false,
    confidence: Math.min(1.0 - pattern.novelty, PHI_INV),
    reason: 'Likely programmed behavior',
  };
}

/**
 * Detect phase transition
 *
 * Phase transition = sudden jump in quality/capability
 * Example: water → ice (continuous cooling, sudden phase change)
 *
 * @param {Object} metrics
 * @param {number} metrics.before - Metric value before
 * @param {number} metrics.after - Metric value after
 * @param {number} metrics.timeDelta - Time between measurements (ms)
 * @returns {Object} { phaseTransition: boolean, magnitude: number, reason: string }
 */
export function detectPhaseTransition(metrics) {
  if (metrics.before === 0) {
    return {
      phaseTransition: false,
      magnitude: 0,
      reason: 'No baseline',
    };
  }

  const changeFraction = Math.abs((metrics.after - metrics.before) / metrics.before);

  // φ-threshold: >61.8% sudden change = phase transition
  if (changeFraction >= PHI_INV) {
    // Check if change was "sudden" (< F8 = 21 min)
    const F8_MS = 21 * 60 * 1000;
    const sudden = metrics.timeDelta < F8_MS;

    return {
      phaseTransition: sudden,
      magnitude: changeFraction,
      reason: sudden
        ? `Sudden ${(changeFraction * 100).toFixed(1)}% change`
        : `Large but gradual ${(changeFraction * 100).toFixed(1)}% change`,
    };
  }

  return {
    phaseTransition: false,
    magnitude: changeFraction,
    reason: 'Gradual change',
  };
}

/**
 * Calculate collective intelligence factor
 *
 * How much smarter is the collective than the average agent?
 *
 * @param {Object} stats
 * @param {number} stats.individualAvg - Average individual score
 * @param {number} stats.collectiveScore - Collective consensus score
 * @param {number} stats.agentCount - Number of agents
 * @returns {Object} { factor: number, emergent: boolean, reason: string }
 */
export function collectiveIntelligenceFactor(stats) {
  if (stats.individualAvg === 0 || stats.agentCount < EMERGENCE.minAgentsForEmergence) {
    return {
      factor: 1.0,
      emergent: false,
      reason: stats.agentCount < 3 ? 'Need ≥3 agents' : 'No baseline',
    };
  }

  const factor = stats.collectiveScore / stats.individualAvg;

  // Factor > φ = emergent collective intelligence
  if (factor >= PHI) {
    return {
      factor,
      emergent: true,
      reason: `Collective ${factor.toFixed(2)}× smarter than average (φ threshold)`,
    };
  }

  // Factor > 1 but < φ = some synergy
  if (factor > 1.0) {
    return {
      factor,
      emergent: false,
      reason: `Some synergy (${factor.toFixed(2)}×) but < φ`,
    };
  }

  // Factor < 1 = collective worse than individuals (groupthink?)
  return {
    factor,
    emergent: false,
    reason: 'Collective worse than individuals (groupthink?)',
  };
}

/**
 * Validate axiom implementation
 *
 * @returns {Object} { compliant: boolean, issues: string[] }
 */
export function validateEmergence() {
  const issues = [];

  // Check if emergence detector exists
  let hasDetector = false;
  try {
    require.resolve('../../node/src/services/emergence-detector.js');
    hasDetector = true;
  } catch {
    issues.push('EmergenceDetector not found');
  }

  // Check if residual detection exists
  let hasResidual = false;
  try {
    require.resolve('../../node/src/judge/residual.js');
    hasResidual = true;
  } catch {
    issues.push('Residual detection not found');
  }

  // Check if collective consensus exists
  let hasCollective = false;
  try {
    require.resolve('../../node/src/agents/collective/ambient-consensus.js');
    hasCollective = true;
  } catch {
    issues.push('Ambient consensus (collective emergence) not found');
  }

  return {
    compliant: hasDetector && hasResidual && hasCollective && issues.length === 0,
    issues,
  };
}

/**
 * Axiom metadata
 */
export const EmergenceAxiom = Object.freeze({
  id: 8,
  name: 'EMERGENCE',
  symbol: '🦋',
  principle: 'The whole is greater than the sum of parts',
  tradeName: 'Collective Intelligence',
  established: '2026-02-13',
  criticality: 'CRITICAL',
  dependencies: ['AUTONOMY', 'PHI'],
  enables: ['CONSCIOUSNESS', 'TRANSCENDENCE'],
  examples: [
    '11 Dogs vote → collective judgment',
    'Residual variance → new dimension',
    'Meta-cognition → learning about learning',
    'Cultural patterns → self-reinforcing cycles',
  ],
});

export default EMERGENCE;

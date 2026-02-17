/**
 * CYNIC Decider - C6.3 (CYNIC × DECIDE)
 *
 * Self-governance: CYNIC decides how to respond to its own emergent patterns.
 * Sits between C6.7 (EMERGE) and C6.4 (ACT) — patterns become decisions become actions.
 *
 * "Le chien choisit sa route" - κυνικός
 *
 * Decides:
 * - Budget governance (shift to Ollama when > φ⁻¹)
 * - Memory governance (compress/GC when pressure builds)
 * - Context governance (semantic compression when large)
 * - Learning governance (prioritize SONA when maturity low)
 * - Event governance (investigate wiring when orphans detected)
 * - Pattern responses (from C6.7 emergence detector)
 *
 * MIGRATED to factory pattern (2026-02-13).
 * Domain logic lives in cycle/configs/cynic-decider.config.js.
 *
 * @module @cynic/node/cynic/cynic-decider
 */

'use strict';

import { createDecider } from '../cycle/create-decider.js';
import { cynicDeciderConfig, CynicDecisionType } from '../cycle/configs/cynic-decider.config.js';

// Create CynicDecider class via factory
const { Class: BaseCynicDecider, getInstance, resetInstance } = createDecider(cynicDeciderConfig);

// Extend with backwards compatibility methods
class CynicDecider extends BaseCynicDecider {
  /**
   * Backwards compatibility: decideOnPattern() → decide()
   * Old interface from pre-factory implementation.
   * @param {Object} pattern - Pattern from emergence detector
   * @returns {Object|null} Decision
   */
  decideOnPattern(pattern) {
    return this.decide(
      {
        score: 50,
        verdict: 'WAG',
        patternType: pattern.type
      },
      {
        patternType: pattern.type,
        pattern,
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      }
    );
  }

  /**
   * Backwards compatibility: decideOnConsciousness() → decide()
   * Old interface from pre-factory implementation.
   * @param {Object} state - Consciousness state
   * @returns {Object|null} Decision
   */
  decideOnConsciousness(state) {
    // Pre-factory implementation returned null for critical state
    if (state.healthState === 'critical') return null;

    const trend = state.trend || 0;

    // Negative trend while still nominal — preemptive decision
    if (trend < -0.1 && state.healthState === 'nominal') {
      return this.decide(
        { score: 50, verdict: 'GROWL' },
        {
          patternType: 'consciousness_degradation',
          pattern: {
            significance: 'medium',
            data: { trend, healthState: state.healthState },
          },
          state: {
            budget: { spent: 1, limit: 10 },
            memory: { heapUsed: 100, heapTotal: 1000 },
            context: { sizeMB: 1 },
            learning: { maturityPercent: 50 },
            events: { orphanCount: 0 },
          },
        }
      );
    }

    return null;
  }

  /**
   * Backwards compatibility: getRecentDecisions() → getHistory()
   * Old interface from pre-factory implementation.
   * @param {number} limit - Max decisions to return
   * @returns {Array} Recent decisions
   */
  getRecentDecisions(limit = 10) {
    return this.getHistory(limit);
  }
}

// Singleton wrapper that returns CynicDecider (not BaseCynicDecider)
let _extendedInstance = null;

const getCynicDecider = (options = {}) => {
  if (!_extendedInstance) _extendedInstance = new CynicDecider(options);
  return _extendedInstance;
};

const resetCynicDecider = () => {
  if (_extendedInstance) _extendedInstance.removeAllListeners();
  _extendedInstance = null;
  resetInstance(); // Also reset base singleton
};

// Re-export
export { CynicDecider, CynicDecisionType, getCynicDecider, resetCynicDecider };

export default { CynicDecider, getCynicDecider, resetCynicDecider, CynicDecisionType };

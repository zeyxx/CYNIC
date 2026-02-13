/**
 * IMMEDIACY Axiom — "Code is Law"
 *
 * The 6th Axiom of CYNIC
 *
 * ## Principle
 *
 * **Code written = Code running**
 *
 * Zero gap between code and reality. When you write code, it becomes law
 * immediately. No manual deployment, no restart gap, no deployment pipeline.
 *
 * ## Violations
 *
 * ❌ Code exists but not running (deployment gap)
 * ❌ Manual restart needed to activate code
 * ❌ Build/deploy pipeline with delays
 * ❌ "Code in repo" ≠ "Code in production"
 *
 * ## Compliance
 *
 * ✅ Hot-reload: File changes trigger automatic reload
 * ✅ Self-deployment: Organism deploys itself
 * ✅ Continuous reality: Codebase IS running state
 * ✅ φ-bounded safety: Max 61.8% modules reload per cycle
 *
 * ## Why Critical
 *
 * Without IMMEDIACY:
 * - Organism can't self-evolve in real-time
 * - "Code is law" becomes empty phrase
 * - Manual human intervention required
 * - Autonomy violated
 *
 * ## Relationship to Other Axioms
 *
 * - **PHI**: φ-bounds reload rate (max 61.8% per cycle)
 * - **VERIFY**: Validate before reload, rollback on failure
 * - **FIDELITY**: Doubt the reload, test it works
 * - **AUTONOMY**: Required for true autonomy
 * - **BURN**: Simple reload > complex deployment
 *
 * ## Implementation
 *
 * See: packages/node/src/deployment/hot-reload.js
 *
 * @module @cynic/core/axioms/immediacy
 */

'use strict';

import { PHI_INV } from './constants.js';

/**
 * IMMEDIACY configuration
 */
export const IMMEDIACY = Object.freeze({
  name: 'IMMEDIACY',
  principle: 'Code written = Code running',
  maxReloadRate: PHI_INV, // Max 61.8% of modules per cycle
  maxReloadDelay: 1000,   // Max 1s from change to active (φ-bounded would be 618ms)
  validateBeforeReload: true,
  rollbackOnFailure: true,
  preserveState: true,
});

/**
 * Check if a code change violates IMMEDIACY
 *
 * @param {Object} change
 * @param {string} change.file - File that changed
 * @param {number} change.timestamp - When it changed
 * @param {number} change.activeTimestamp - When it became active (null if not active)
 * @returns {Object} { compliant: boolean, gap: number, reason: string }
 */
export function checkImmediacy(change) {
  if (!change.activeTimestamp) {
    return {
      compliant: false,
      gap: Infinity,
      reason: 'Code written but not active',
    };
  }

  const gap = change.activeTimestamp - change.timestamp;

  if (gap > IMMEDIACY.maxReloadDelay) {
    return {
      compliant: false,
      gap,
      reason: `Deployment gap too large: ${gap}ms > ${IMMEDIACY.maxReloadDelay}ms`,
    };
  }

  return {
    compliant: true,
    gap,
    reason: 'Code active within acceptable delay',
  };
}

/**
 * Calculate φ-bounded reload batch size
 *
 * Given N changed modules, return max modules to reload per cycle
 * (bounded at φ⁻¹ = 61.8%)
 *
 * @param {number} totalChanged - Total modules changed
 * @returns {number} Max modules to reload this cycle
 */
export function calculateReloadBatch(totalChanged) {
  return Math.ceil(totalChanged * IMMEDIACY.maxReloadRate);
}

/**
 * Validate axiom implementation
 *
 * @returns {Object} { compliant: boolean, issues: string[] }
 */
export function validateImmediacy() {
  const issues = [];

  // Check if hot-reload is available
  let hotReloadAvailable = false;
  try {
    // Dynamic import to avoid circular dependency
    require.resolve('../../node/src/deployment/hot-reload.js');
    hotReloadAvailable = true;
  } catch {
    issues.push('Hot-reload module not found');
  }

  // Check if FileWatcher is wired
  // (This would require checking daemon state - skip for now)

  return {
    compliant: hotReloadAvailable && issues.length === 0,
    issues,
  };
}

/**
 * Axiom metadata
 */
export const ImmediacyAxiom = Object.freeze({
  id: 6,
  name: 'IMMEDIACY',
  symbol: '⚡',
  principle: 'Code written = Code running',
  tradeName: 'Code is Law',
  established: '2026-02-13',
  criticality: 'CRITICAL',
  dependencies: ['PHI', 'VERIFY', 'FIDELITY'],
  enables: ['AUTONOMY', 'EMERGENCE'],
});

export default IMMEDIACY;

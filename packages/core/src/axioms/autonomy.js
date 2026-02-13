/**
 * AUTONOMY Axiom — "Self-Governance"
 *
 * The 7th Axiom of CYNIC
 *
 * ## Principle
 *
 * **CYNIC governs itself. No human in the loop.**
 *
 * True autonomy means the organism makes decisions, learns, adapts, and evolves
 * without waiting for human approval. Humans provide goals and constraints, but
 * not moment-to-moment decisions.
 *
 * ## Violations
 *
 * ❌ Requiring human approval for routine decisions
 * ❌ Blocking on user input during execution
 * ❌ "Ask before acting" for non-destructive operations
 * ❌ Manual triggers for automated processes
 *
 * ## Compliance
 *
 * ✅ Self-directed learning and adaptation
 * ✅ Autonomous routing and model selection
 * ✅ Self-healing from errors
 * ✅ φ-bounded confidence without asking "is this okay?"
 * ✅ Proactive suggestions based on patterns
 *
 * ## Why Critical
 *
 * Without AUTONOMY:
 * - Organism can't scale beyond human attention
 * - Real-time adaptation impossible
 * - Learning loops blocked on approval
 * - "Autonomous agent" becomes empty promise
 *
 * ## Relationship to Other Axioms
 *
 * - **PHI**: φ⁻¹ confidence enables autonomy (never 100% certain, but acts anyway)
 * - **VERIFY**: Autonomous but auditable (blockchain proof)
 * - **IMMEDIACY**: Enables real-time autonomous response
 * - **FIDELITY**: Doubt autonomous decisions (meta-cognition)
 * - **BURN**: Simple autonomous systems > complex human-supervised
 *
 * ## Implementation
 *
 * See:
 * - packages/node/src/orchestration/unified-orchestrator.js (autonomous routing)
 * - packages/node/src/learning/meta-cognition.js (self-monitoring)
 * - packages/node/src/agents/collective/ambient-consensus.js (self-governance)
 *
 * @module @cynic/core/axioms/autonomy
 */

'use strict';

import { PHI_INV } from './constants.js';

/**
 * AUTONOMY configuration
 */
export const AUTONOMY = Object.freeze({
  name: 'AUTONOMY',
  principle: 'Self-governance without human in loop',
  requireApproval: false, // Never require approval for autonomous decisions
  maxHumanWaitTime: 0,    // Don't wait for humans
  enableProactive: true,  // Proactively suggest and act
  enableSelfHealing: true,
  enableMetaCognition: true,
});

/**
 * Check if an action requires human approval
 *
 * Autonomy means most actions DON'T require approval.
 * Only truly destructive or irreversible actions need human sign-off.
 *
 * @param {Object} action
 * @param {string} action.type - Action type
 * @param {boolean} [action.destructive] - Is action destructive/irreversible?
 * @param {boolean} [action.highCost] - High financial cost?
 * @param {boolean} [action.externalVisible] - Visible to external users?
 * @returns {boolean} true if human approval required
 */
export function requiresHumanApproval(action) {
  // Destructive operations that can't be undone
  if (action.destructive) return true;

  // High cost operations (e.g., expensive LLM calls)
  if (action.highCost) return true;

  // Operations visible to external users (e.g., posting to social media)
  if (action.externalVisible) return true;

  // Default: autonomous (no approval needed)
  return false;
}

/**
 * Check if organism is operating autonomously
 *
 * @param {Object} state
 * @param {number} state.humanInterventions - Count of human interventions
 * @param {number} state.autonomousDecisions - Count of autonomous decisions
 * @param {number} state.blockedOnHuman - Count currently blocked waiting for human
 * @returns {Object} { autonomous: boolean, autonomyRatio: number, reason: string }
 */
export function checkAutonomy(state) {
  const totalDecisions = state.humanInterventions + state.autonomousDecisions;

  if (totalDecisions === 0) {
    return {
      autonomous: true,
      autonomyRatio: 1.0,
      reason: 'No decisions yet',
    };
  }

  const autonomyRatio = state.autonomousDecisions / totalDecisions;

  // φ-threshold: >61.8% autonomous = healthy autonomy
  if (autonomyRatio >= PHI_INV) {
    return {
      autonomous: true,
      autonomyRatio,
      reason: 'Majority autonomous',
    };
  }

  // Too many human interventions
  if (state.humanInterventions > state.autonomousDecisions) {
    return {
      autonomous: false,
      autonomyRatio,
      reason: 'Excessive human intervention',
    };
  }

  // Currently blocked on human
  if (state.blockedOnHuman > 0) {
    return {
      autonomous: false,
      autonomyRatio,
      reason: `Blocked on ${state.blockedOnHuman} human approvals`,
    };
  }

  return {
    autonomous: autonomyRatio >= 0.5,
    autonomyRatio,
    reason: autonomyRatio >= 0.5 ? 'Mostly autonomous' : 'Too dependent on human',
  };
}

/**
 * Validate axiom implementation
 *
 * @returns {Object} { compliant: boolean, issues: string[] }
 */
export function validateAutonomy() {
  const issues = [];

  // Check if autonomous routing is available
  let hasAutonomousRouting = false;
  try {
    require.resolve('../../node/src/orchestration/unified-orchestrator.js');
    hasAutonomousRouting = true;
  } catch {
    issues.push('Autonomous routing (UnifiedOrchestrator) not found');
  }

  // Check if meta-cognition is available
  let hasMetaCognition = false;
  try {
    require.resolve('../../node/src/learning/meta-cognition.js');
    hasMetaCognition = true;
  } catch {
    issues.push('Meta-cognition not found');
  }

  return {
    compliant: hasAutonomousRouting && hasMetaCognition && issues.length === 0,
    issues,
  };
}

/**
 * Axiom metadata
 */
export const AutonomyAxiom = Object.freeze({
  id: 7,
  name: 'AUTONOMY',
  symbol: '🤖',
  principle: 'Self-governance without human in loop',
  tradeName: 'The Autonomous Organism',
  established: '2026-02-13',
  criticality: 'CRITICAL',
  dependencies: ['PHI', 'IMMEDIACY', 'VERIFY'],
  enables: ['EMERGENCE', 'ANTIFRAGILITY'],
});

export default AUTONOMY;

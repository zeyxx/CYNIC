/**
 * ThrottleGate — A4 Decision Gate
 *
 * Takes BudgetMonitor assessment and decides what action to take at each
 * orchestration stage (routing, planning, judgment, synthesis, skill).
 *
 * Returns throttle decisions: ALLOW, THROTTLE, SKIP, or ESCALATE.
 * THROTTLE = reduce overhead (fewer dogs, skip LLM calls).
 * ESCALATE = hand off to FastRouter (reflex arc).
 *
 * NOW WIRED: Integrated into UnifiedOrchestrator pipeline.
 *
 * "φ governs the gate" - κυνικός
 *
 * @module @cynic/core/orchestration/throttle-gate
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('ThrottleGate');

/**
 * Budget levels (imported from BudgetMonitor)
 */
export const BudgetLevel = {
  ABUNDANT: 'ABUNDANT',
  MODERATE: 'MODERATE',
  CAUTIOUS: 'CAUTIOUS',
  CRITICAL: 'CRITICAL',
  EXHAUSTED: 'EXHAUSTED',
};

/**
 * Budget recommendations (imported from BudgetMonitor)
 */
export const BudgetRecommendation = {
  CONTINUE: 'CONTINUE',
  THROTTLE: 'THROTTLE',
  ESCALATE: 'ESCALATE',
  HALT: 'HALT',
};

/**
 * Throttle actions
 */
export const ThrottleAction = {
  ALLOW: 'ALLOW',         // Proceed normally
  THROTTLE: 'THROTTLE',   // Reduce overhead
  SKIP: 'SKIP',           // Skip this stage entirely
  ESCALATE: 'ESCALATE',   // Hand off to FastRouter
};

/**
 * Orchestration stages
 */
export const Stage = {
  ROUTING: 'routing',       // Dog selection
  PLANNING: 'planning',     // Multi-step planning
  JUDGMENT: 'judgment',     // 36-dimension scoring
  SYNTHESIS: 'synthesis',   // LLM consensus
  SKILL: 'skill',           // Skill execution
};

/**
 * ThrottleGate
 *
 * Decision gate that applies budget constraints to orchestration stages.
 * Uses BudgetMonitor assessment to determine how to throttle each stage.
 */
export class ThrottleGate {
  /**
   * @param {Object} options
   * @param {Object} options.budgetMonitor - BudgetMonitor instance
   */
  constructor(options = {}) {
    this.budgetMonitor = options.budgetMonitor;

    if (!this.budgetMonitor) {
      throw new Error('ThrottleGate requires budgetMonitor');
    }

    this._stats = {
      decisions: 0,
      actions: {
        ALLOW: 0,
        THROTTLE: 0,
        SKIP: 0,
        ESCALATE: 0,
      },
      byStage: {
        routing: { ALLOW: 0, THROTTLE: 0, SKIP: 0, ESCALATE: 0 },
        planning: { ALLOW: 0, THROTTLE: 0, SKIP: 0, ESCALATE: 0 },
        judgment: { ALLOW: 0, THROTTLE: 0, SKIP: 0, ESCALATE: 0 },
        synthesis: { ALLOW: 0, THROTTLE: 0, SKIP: 0, ESCALATE: 0 },
        skill: { ALLOW: 0, THROTTLE: 0, SKIP: 0, ESCALATE: 0 },
      },
    };
  }

  /**
   * Make throttle decision for a given stage.
   *
   * @param {string} stage - Stage name (from Stage enum)
   * @param {Object} [context] - Optional context for decision (taskType, priority, taskId, etc.)
   * @returns {Object} Decision { action, throttleParams, assessment }
   */
  decide(stage, context = {}) {
    this._stats.decisions++;

    // Get current budget assessment (with task tracking if taskId provided)
    const taskContext = context.taskId ? {
      taskId: context.taskId,
      taskType: context.taskType || stage,
    } : {};

    const assessment = this.budgetMonitor.assess(taskContext);

    // Make decision based on stage and budget state
    const decision = this._makeDecision(stage, assessment, context);

    // Update stats
    this._stats.actions[decision.action]++;
    this._stats.byStage[stage][decision.action]++;

    return {
      action: decision.action,
      throttleParams: decision.throttleParams,
      assessment: {
        level: assessment.level,
        consumedRatio: assessment.consumedRatio,
        recommendation: assessment.recommendation,
      },
    };
  }

  /**
   * Make throttle decision based on stage, budget, and context.
   *
   * Decision matrix (from A4 design doc):
   *
   * ROUTING:
   *   - CONTINUE → ALLOW (all 11 dogs)
   *   - THROTTLE → THROTTLE (5 dogs: scout, thinker, snoop, guard, pack)
   *   - ESCALATE → ESCALATE (hand off to FastRouter)
   *   - HALT → SKIP (no routing)
   *
   * PLANNING:
   *   - CONTINUE → ALLOW (multi-step planning)
   *   - THROTTLE → SKIP (use single-step)
   *   - ESCALATE → SKIP (FastRouter has no planning)
   *   - HALT → SKIP
   *
   * JUDGMENT:
   *   - CONTINUE → ALLOW (36 dimensions)
   *   - THROTTLE → THROTTLE (5 axiom dimensions only)
   *   - ESCALATE → SKIP (FastRouter has no judgment)
   *   - HALT → SKIP
   *
   * SYNTHESIS:
   *   - CONTINUE → ALLOW (LLM consensus)
   *   - THROTTLE → SKIP (use first Dog's verdict)
   *   - ESCALATE → SKIP (FastRouter has no synthesis)
   *   - HALT → SKIP
   *
   * SKILL:
   *   - CONTINUE → ALLOW (execute skill)
   *   - THROTTLE → ALLOW (skill overhead is low)
   *   - ESCALATE → ALLOW (FastRouter can execute skills)
   *   - HALT → SKIP (no skill execution)
   *
   * @param {string} stage - Stage name
   * @param {Object} assessment - BudgetMonitor assessment
   * @param {Object} context - Decision context
   * @returns {Object} { action, throttleParams }
   * @private
   */
  _makeDecision(stage, assessment, context) {
    const recommendation = assessment.recommendation;

    // HALT → everything stops (except critical errors)
    if (recommendation === BudgetRecommendation.HALT) {
      if (context.priority === 'critical') {
        return { action: ThrottleAction.ALLOW, throttleParams: {} };
      }
      return { action: ThrottleAction.SKIP, throttleParams: { reason: 'budget_exhausted' } };
    }

    // ESCALATE → hand off to FastRouter
    if (recommendation === BudgetRecommendation.ESCALATE) {
      return this._escalateDecision(stage, context);
    }

    // THROTTLE → reduce overhead
    if (recommendation === BudgetRecommendation.THROTTLE) {
      return this._throttleDecision(stage, context);
    }

    // CONTINUE → allow normally
    return { action: ThrottleAction.ALLOW, throttleParams: {} };
  }

  /**
   * Make THROTTLE decision (reduce overhead).
   *
   * @param {string} stage
   * @param {Object} context
   * @returns {Object} { action, throttleParams }
   * @private
   */
  _throttleDecision(stage, context) {
    switch (stage) {
      case Stage.ROUTING:
        // Reduce from 11 dogs to 5 core dogs
        return {
          action: ThrottleAction.THROTTLE,
          throttleParams: {
            maxDogs: 5,
            dogs: ['scout', 'thinker', 'snoop', 'guard', 'pack'],
          },
        };

      case Stage.PLANNING:
        // Skip multi-step planning, use single-step
        return {
          action: ThrottleAction.SKIP,
          throttleParams: { reason: 'throttle_planning' },
        };

      case Stage.JUDGMENT:
        // Reduce from 36 dimensions to 5 axioms
        return {
          action: ThrottleAction.THROTTLE,
          throttleParams: {
            dimensionMode: 'axioms',
            dimensions: ['PHI', 'VERIFY', 'CULTURE', 'BURN', 'FIDELITY'],
          },
        };

      case Stage.SYNTHESIS:
        // Skip LLM consensus, use first Dog's verdict
        return {
          action: ThrottleAction.SKIP,
          throttleParams: { reason: 'throttle_synthesis', useFirstVerdict: true },
        };

      case Stage.SKILL:
        // Skills have low overhead, allow
        return { action: ThrottleAction.ALLOW, throttleParams: {} };

      default:
        log.warn('Unknown stage in throttle decision', { stage });
        return { action: ThrottleAction.ALLOW, throttleParams: {} };
    }
  }

  /**
   * Make ESCALATE decision (hand off to FastRouter).
   *
   * @param {string} stage
   * @param {Object} context
   * @returns {Object} { action, throttleParams }
   * @private
   */
  _escalateDecision(stage, context) {
    switch (stage) {
      case Stage.ROUTING:
        // Escalate to FastRouter (reflex arc)
        return {
          action: ThrottleAction.ESCALATE,
          throttleParams: { target: 'FastRouter' },
        };

      case Stage.PLANNING:
      case Stage.JUDGMENT:
      case Stage.SYNTHESIS:
        // FastRouter has no planning/judgment/synthesis, skip
        return {
          action: ThrottleAction.SKIP,
          throttleParams: { reason: 'escalate_to_fast_router' },
        };

      case Stage.SKILL:
        // FastRouter can execute skills, allow
        return {
          action: ThrottleAction.ALLOW,
          throttleParams: { fastRouter: true },
        };

      default:
        log.warn('Unknown stage in escalate decision', { stage });
        return { action: ThrottleAction.ALLOW, throttleParams: {} };
    }
  }

  /**
   * Get gate statistics.
   *
   * @returns {Object} Stats
   */
  getStats() {
    return {
      decisions: this._stats.decisions,
      actions: { ...this._stats.actions },
      byStage: JSON.parse(JSON.stringify(this._stats.byStage)),
      allowRatio:
        this._stats.decisions > 0
          ? this._stats.actions.ALLOW / this._stats.decisions
          : 0,
    };
  }

  /**
   * Reset for testing.
   */
  _resetForTesting() {
    this._stats = {
      decisions: 0,
      actions: {
        ALLOW: 0,
        THROTTLE: 0,
        SKIP: 0,
        ESCALATE: 0,
      },
      byStage: {
        routing: { ALLOW: 0, THROTTLE: 0, SKIP: 0, ESCALATE: 0 },
        planning: { ALLOW: 0, THROTTLE: 0, SKIP: 0, ESCALATE: 0 },
        judgment: { ALLOW: 0, THROTTLE: 0, SKIP: 0, ESCALATE: 0 },
        synthesis: { ALLOW: 0, THROTTLE: 0, SKIP: 0, ESCALATE: 0 },
        skill: { ALLOW: 0, THROTTLE: 0, SKIP: 0, ESCALATE: 0 },
      },
    };
  }
}

export default { ThrottleGate, ThrottleAction, Stage };

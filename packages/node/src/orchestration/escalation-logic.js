/**
 * EscalationLogic — A4 FastRouter Handoff
 *
 * Handles escalation from slow path (UnifiedOrchestrator) to fast path
 * (FastRouter) when budget is critical. Formats prompt, calls FastRouter,
 * handles response.
 *
 * Escalation scenarios:
 * - Budget CRITICAL (80%+) → hand off to reflex arc
 * - ThrottleGate returns ESCALATE
 * - User sets priority=high
 *
 * "When budget burns, reflexes take over" - κυνικός
 *
 * @module @cynic/node/orchestration/escalation-logic
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('EscalationLogic');

/**
 * Escalation reasons
 */
export const EscalationReason = {
  BUDGET_CRITICAL: 'budget_critical',     // Budget >= 80%
  THROTTLE_GATE: 'throttle_gate',         // ThrottleGate decided to escalate
  USER_PRIORITY: 'user_priority',         // User set priority=high
  SLOW_PATH_TIMEOUT: 'slow_path_timeout', // Slow path took too long
};

/**
 * EscalationLogic
 *
 * Coordinates handoff from UnifiedOrchestrator to FastRouter when budget
 * constraints require switching to reflex arc.
 */
export class EscalationLogic {
  /**
   * @param {Object} options
   * @param {Object} options.fastRouter - FastRouter instance
   * @param {Object} [options.budgetMonitor] - BudgetMonitor instance (optional)
   */
  constructor(options = {}) {
    this.fastRouter = options.fastRouter;
    this.budgetMonitor = options.budgetMonitor || null;

    if (!this.fastRouter) {
      throw new Error('EscalationLogic requires fastRouter');
    }

    this._stats = {
      escalations: 0,
      byReason: {
        budget_critical: 0,
        throttle_gate: 0,
        user_priority: 0,
        slow_path_timeout: 0,
      },
      successes: 0,
      fallbacks: 0,
      totalLatencyMs: 0,
    };
  }

  /**
   * Check if escalation is needed based on budget state.
   *
   * @param {Object} [context] - Optional context (priority, taskType)
   * @returns {Object} { shouldEscalate, reason }
   */
  shouldEscalate(context = {}) {
    // Priority override: user explicitly requested fast path
    if (context.priority === 'high' || context.priority === 'critical') {
      return { shouldEscalate: true, reason: EscalationReason.USER_PRIORITY };
    }

    // Budget check (if monitor available)
    if (this.budgetMonitor) {
      const assessment = this.budgetMonitor.assess();

      // CRITICAL budget → escalate
      if (assessment.level === 'CRITICAL') {
        return { shouldEscalate: true, reason: EscalationReason.BUDGET_CRITICAL };
      }

      // EXHAUSTED budget → escalate (no choice)
      if (assessment.level === 'EXHAUSTED') {
        return { shouldEscalate: true, reason: EscalationReason.BUDGET_CRITICAL };
      }
    }

    // No escalation needed
    return { shouldEscalate: false, reason: null };
  }

  /**
   * Escalate to FastRouter.
   *
   * Simplifies the prompt, removes heavy context, and calls FastRouter
   * with minimal overhead.
   *
   * @param {Object} input - Escalation input
   * @param {string} input.prompt - Original user prompt
   * @param {string} [input.reason] - Escalation reason (from EscalationReason)
   * @param {Object} [input.context] - Optional context (taskType, priority)
   * @returns {Promise<Object>} Escalation result
   */
  async escalate(input) {
    this._stats.escalations++;

    const reason = input.reason || EscalationReason.THROTTLE_GATE;
    this._stats.byReason[reason]++;

    const startTime = Date.now();

    try {
      // Simplify prompt for fast path
      const simplifiedPrompt = this._simplifyPrompt(input.prompt, input.context);

      log.info('Escalating to FastRouter', {
        reason,
        originalLength: input.prompt.length,
        simplifiedLength: simplifiedPrompt.length,
      });

      // Call FastRouter (reflex arc)
      const fastResponse = await this.fastRouter.route({
        prompt: simplifiedPrompt,
        context: {
          ...input.context,
          escalated: true,
          escalationReason: reason,
        },
      });

      const latencyMs = Date.now() - startTime;
      this._stats.totalLatencyMs += latencyMs;
      this._stats.successes++;

      log.info('FastRouter escalation succeeded', {
        reason,
        latencyMs,
        action: fastResponse.action,
      });

      return {
        success: true,
        response: fastResponse,
        reason,
        latencyMs,
      };
    } catch (err) {
      const latencyMs = Date.now() - startTime;
      this._stats.totalLatencyMs += latencyMs;
      this._stats.fallbacks++;

      log.error('FastRouter escalation failed', {
        reason,
        latencyMs,
        error: err.message,
      });

      // Fallback: return minimal response
      return {
        success: false,
        reason,
        latencyMs,
        error: err.message,
        fallback: this._createFallbackResponse(input),
      };
    }
  }

  /**
   * Simplify prompt for FastRouter.
   *
   * FastRouter is a reflex arc — it doesn't need full context.
   * Remove verbosity, keep core intent.
   *
   * @param {string} prompt - Original prompt
   * @param {Object} context - Context
   * @returns {string} Simplified prompt
   * @private
   */
  _simplifyPrompt(prompt, context = {}) {
    // If prompt is already short, return as-is
    if (prompt.length < 200) {
      return prompt;
    }

    // Extract core intent (first sentence + task type if available)
    const firstSentence = prompt.split(/[.!?]\s+/)[0] || prompt.substring(0, 200);

    let simplified = firstSentence;

    // Add task type if present
    if (context.taskType) {
      simplified = `[${context.taskType}] ${simplified}`;
    }

    // Truncate to max 300 chars
    if (simplified.length > 300) {
      simplified = simplified.substring(0, 297) + '...';
    }

    return simplified;
  }

  /**
   * Create fallback response when FastRouter fails.
   *
   * @param {Object} input - Original input
   * @returns {Object} Fallback response
   * @private
   */
  _createFallbackResponse(input) {
    return {
      action: 'skip',
      verdict: 'BARK',
      message: 'Budget critical and FastRouter unavailable. Skipping operation.',
      qScore: 19, // BARK threshold
      metadata: {
        fallback: true,
        originalPrompt: input.prompt.substring(0, 100),
      },
    };
  }

  /**
   * Get escalation statistics.
   *
   * @returns {Object} Stats
   */
  getStats() {
    return {
      escalations: this._stats.escalations,
      byReason: { ...this._stats.byReason },
      successes: this._stats.successes,
      fallbacks: this._stats.fallbacks,
      successRate:
        this._stats.escalations > 0
          ? this._stats.successes / this._stats.escalations
          : 0,
      avgLatencyMs:
        this._stats.escalations > 0
          ? this._stats.totalLatencyMs / this._stats.escalations
          : 0,
    };
  }

  /**
   * Reset for testing.
   */
  _resetForTesting() {
    this._stats = {
      escalations: 0,
      byReason: {
        budget_critical: 0,
        throttle_gate: 0,
        user_priority: 0,
        slow_path_timeout: 0,
      },
      successes: 0,
      fallbacks: 0,
      totalLatencyMs: 0,
    };
  }
}

export default { EscalationLogic, EscalationReason };

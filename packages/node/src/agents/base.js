/**
 * CYNIC Base Agent
 *
 * Abstract base class for all CYNIC agents
 *
 * The Four Dogs:
 * - Observer: Silent watcher (PostToolUse, non-blocking)
 * - Digester: Archivist (PostConversation, non-blocking)
 * - Guardian: Watchdog (PreToolUse, blocking)
 * - Mentor: Wise elder (context-aware, non-blocking)
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * Agent trigger types
 */
export const AgentTrigger = {
  PRE_TOOL_USE: 'PreToolUse',
  POST_TOOL_USE: 'PostToolUse',
  POST_CONVERSATION: 'PostConversation',
  SESSION_START: 'SessionStart',
  SESSION_END: 'SessionEnd',
  CONTEXT_AWARE: 'ContextAware',
  PERIODIC: 'Periodic',
};

/**
 * Agent behavior modes
 */
export const AgentBehavior = {
  BLOCKING: 'blocking',      // Requires confirmation
  NON_BLOCKING: 'non-blocking', // Runs in background
  SILENT: 'silent',          // No output unless critical
};

/**
 * Agent response types
 */
export const AgentResponse = {
  ALLOW: 'allow',            // Proceed with action
  BLOCK: 'block',            // Stop action
  WARN: 'warn',              // Proceed with warning
  SUGGEST: 'suggest',        // Offer alternative
  LOG: 'log',                // Just record
};

/**
 * Base Agent class
 */
export class BaseAgent {
  /**
   * Create agent
   * @param {Object} options - Agent options
   * @param {string} options.name - Agent name
   * @param {string} options.trigger - Trigger type
   * @param {string} options.behavior - Behavior mode
   * @param {Object} [options.judge] - CYNIC judge instance
   * @param {Object} [options.state] - State manager instance
   */
  constructor(options = {}) {
    if (this.constructor === BaseAgent) {
      throw new Error('BaseAgent is abstract - use a specific agent class');
    }

    this.name = options.name || this.constructor.name;
    this.trigger = options.trigger || AgentTrigger.POST_TOOL_USE;
    this.behavior = options.behavior || AgentBehavior.NON_BLOCKING;
    this.judge = options.judge || null;
    this.state = options.state || null;

    // Agent stats
    this.stats = {
      invocations: 0,
      actions: 0,
      blocks: 0,
      warnings: 0,
      lastInvocation: null,
      patterns: [],
    };

    // Confidence threshold for action
    this.confidenceThreshold = options.confidenceThreshold || PHI_INV_2; // 38.2%

    // Max confidence (φ⁻¹)
    this.maxConfidence = PHI_INV;
  }

  /**
   * Check if agent should trigger for event
   * @param {Object} event - Event data
   * @returns {boolean} True if should trigger
   */
  shouldTrigger(event) {
    // Override in subclass
    return event.type === this.trigger;
  }

  /**
   * Process event and return response
   * @param {Object} event - Event data
   * @param {Object} context - Event context
   * @returns {Promise<Object>} Agent response
   */
  async process(event, context = {}) {
    this.stats.invocations++;
    this.stats.lastInvocation = Date.now();

    try {
      // Check trigger condition
      if (!this.shouldTrigger(event)) {
        return { response: AgentResponse.ALLOW, reason: 'No trigger match' };
      }

      // Analyze event
      const analysis = await this.analyze(event, context);

      // Decide action
      const decision = await this.decide(analysis, context);

      // Record stats
      if (decision.response === AgentResponse.BLOCK) {
        this.stats.blocks++;
      } else if (decision.response === AgentResponse.WARN) {
        this.stats.warnings++;
      }
      if (decision.action) {
        this.stats.actions++;
      }

      return decision;
    } catch (err) {
      console.error(`[${this.name}] Error:`, err.message);
      return {
        response: AgentResponse.ALLOW,
        error: err.message,
        reason: 'Agent error - allowing by default',
      };
    }
  }

  /**
   * Analyze event (override in subclass)
   * @param {Object} event - Event data
   * @param {Object} context - Event context
   * @returns {Promise<Object>} Analysis result
   */
  async analyze(event, _context) {
    // Default: use judge if available
    if (this.judge) {
      return this.judge.judge(event);
    }
    return { score: 50, verdict: 'WAG', confidence: PHI_INV_2 };
  }

  /**
   * Decide action based on analysis (override in subclass)
   * @param {Object} analysis - Analysis result
   * @param {Object} context - Event context
   * @returns {Promise<Object>} Decision
   */
  async decide(analysis, _context) {
    // Default: allow if score >= 50
    if (analysis.score >= 50) {
      return { response: AgentResponse.ALLOW };
    }
    return { response: AgentResponse.WARN, reason: `Low score: ${analysis.score}` };
  }

  /**
   * Record pattern for learning
   * @param {Object} pattern - Pattern data
   */
  recordPattern(pattern) {
    // Enforce bounds before pushing (FIFO eviction)
    while (this.stats.patterns.length >= 100) {
      this.stats.patterns.shift();
    }
    this.stats.patterns.push({
      ...pattern,
      timestamp: Date.now(),
      agent: this.name,
    });
  }

  /**
   * Get agent stats
   * @returns {Object} Stats
   */
  getStats() {
    return {
      name: this.name,
      trigger: this.trigger,
      behavior: this.behavior,
      ...this.stats,
    };
  }

  /**
   * Get agent info for display
   * @returns {Object} Display info
   */
  getInfo() {
    return {
      name: this.name,
      trigger: this.trigger,
      behavior: this.behavior,
      confidenceThreshold: this.confidenceThreshold,
      invocations: this.stats.invocations,
    };
  }
}

export default BaseAgent;

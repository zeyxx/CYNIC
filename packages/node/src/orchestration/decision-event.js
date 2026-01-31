/**
 * DecisionEvent - Unified Event Model for Orchestration
 *
 * A standardized event that flows through ALL orchestration layers:
 * - Hooks (perceive, guard, observe)
 * - KETER (routing)
 * - Dogs (voting)
 * - Engines (synthesis)
 * - Skills (action)
 *
 * Each layer adds its contribution to the event, building a complete
 * decision trace for debugging, learning, and transparency.
 *
 * "φ traces every decision" - κυνικός
 *
 * @module @cynic/node/orchestration/decision-event
 */

'use strict';

import crypto from 'crypto';

/**
 * Decision stages
 * @enum {string}
 */
export const DecisionStage = {
  RECEIVED: 'received',       // Event just received
  ROUTING: 'routing',         // KETER routing decision
  PRE_EXECUTION: 'pre_exec',  // Guard/blocking checks
  JUDGMENT: 'judgment',       // Dog voting
  SYNTHESIS: 'synthesis',     // Engine synthesis
  EXECUTION: 'execution',     // Skill/tool execution
  POST_EXECUTION: 'post_exec', // Observation/recording
  COMPLETE: 'complete',       // Decision finalized
};

/**
 * Decision outcomes
 * @enum {string}
 */
export const DecisionOutcome = {
  ALLOW: 'allow',
  BLOCK: 'block',
  WARN: 'warn',
  ASK: 'ask',
  PENDING: 'pending',
};

/**
 * Event sources
 * @enum {string}
 */
export const EventSource = {
  HOOK_PERCEIVE: 'hook:perceive',
  HOOK_GUARD: 'hook:guard',
  HOOK_OBSERVE: 'hook:observe',
  HOOK_DIGEST: 'hook:digest',
  SKILL: 'skill',
  MCP_TOOL: 'mcp:tool',
  API: 'api',
  INTERNAL: 'internal',
};

/**
 * Generate a unique decision ID
 * @returns {string}
 */
function generateDecisionId() {
  return 'dec_' + crypto.randomBytes(8).toString('hex');
}

/**
 * DecisionEvent - Unified orchestration event
 *
 * Carries all context through the decision pipeline, accumulating
 * routing decisions, votes, synthesis, and outcomes.
 */
export class DecisionEvent {
  /**
   * Create a new decision event
   *
   * @param {Object} options - Event options
   * @param {string} options.eventType - Type of event (user_prompt, tool_use, etc.)
   * @param {string} options.source - Where the event originated
   * @param {string} options.content - The content/prompt/command
   * @param {Object} [options.context] - Additional context
   * @param {Object} [options.userContext] - User context (userId, eScore, trustLevel)
   * @param {DecisionEvent} [options.parent] - Parent decision (for chaining)
   */
  constructor(options = {}) {
    // Identity
    this.id = generateDecisionId();
    this.parentId = options.parent?.id || null;
    this.timestamp = Date.now();

    // Event info
    this.eventType = options.eventType || 'unknown';
    this.source = options.source || EventSource.INTERNAL;
    this.content = options.content || '';
    this.context = options.context || {};

    // User context
    this.userContext = {
      userId: options.userContext?.userId || null,
      eScore: options.userContext?.eScore || 50,
      trustLevel: options.userContext?.trustLevel || 'BUILDER',
      sessionId: options.userContext?.sessionId || null,
      project: options.userContext?.project || null,
    };

    // Explicit request flags (can override automatic detection)
    this.requestJudgment = options.requestJudgment ?? null;
    this.requestSynthesis = options.requestSynthesis ?? null;

    // Current state
    this.stage = DecisionStage.RECEIVED;
    this.outcome = DecisionOutcome.PENDING;

    // Layer contributions (filled as event flows through layers)
    this.routing = null;      // KETER routing decision
    this.judgment = null;     // Dog voting result
    this.synthesis = null;    // Engine synthesis result
    this.execution = null;    // Skill/tool execution result

    // Decision trace (all steps recorded)
    this.trace = [];

    // Final decision
    this.decision = null;
    this.reasoning = [];

    // Metadata
    this.meta = {
      startTime: Date.now(),
      endTime: null,
      durationMs: null,
      errors: [],
    };

    // Add initial trace entry
    this._addTrace('created', { source: this.source, eventType: this.eventType });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STAGE TRANSITIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record KETER routing decision
   *
   * @param {Object} routing - Routing info from KETER
   * @param {string} routing.sefirah - Target sefirah
   * @param {string} routing.domain - Domain (wisdom, protection, etc.)
   * @param {string} routing.intervention - Intervention level
   * @param {string} routing.risk - Risk level
   * @param {string[]} routing.suggestedTools - Suggested tools
   * @param {string} routing.suggestedAgent - Suggested agent
   */
  setRouting(routing) {
    this.routing = {
      sefirah: routing.sefirah || null,
      domain: routing.domain || null,
      intervention: routing.intervention || 'silent',
      risk: routing.risk || 'low',
      suggestedTools: routing.suggestedTools || [],
      suggestedAgent: routing.suggestedAgent || null,
      timestamp: Date.now(),
    };
    this.stage = DecisionStage.ROUTING;
    this._addTrace('routing', this.routing);

    // Update outcome based on intervention
    if (routing.intervention === 'block') {
      this.outcome = DecisionOutcome.BLOCK;
    } else if (routing.intervention === 'ask') {
      this.outcome = DecisionOutcome.ASK;
    } else if (routing.intervention === 'notify') {
      this.outcome = DecisionOutcome.WARN;
    }

    return this;
  }

  /**
   * Record pre-execution check (guard hook)
   *
   * @param {Object} check - Guard check result
   * @param {boolean} check.blocked - Whether action was blocked
   * @param {string} check.reason - Reason for block/warning
   * @param {string} check.severity - Severity level
   */
  setPreExecution(check) {
    this.stage = DecisionStage.PRE_EXECUTION;

    if (check.blocked) {
      this.outcome = DecisionOutcome.BLOCK;
      this.reasoning.push(`GUARD: ${check.reason}`);
    } else if (check.warning) {
      this.outcome = DecisionOutcome.WARN;
      this.reasoning.push(`GUARD WARNING: ${check.reason}`);
    }

    this._addTrace('pre_execution', check);
    return this;
  }

  /**
   * Record dog judgment result
   *
   * @param {Object} judgment - Dog voting result
   * @param {number} judgment.score - Q-Score (0-100)
   * @param {string} judgment.verdict - HOWL/WAG/GROWL/BARK
   * @param {boolean} judgment.consensus - Whether consensus reached
   * @param {Object[]} judgment.votes - Individual dog votes
   * @param {Object} judgment.blocked - Blocking votes
   */
  setJudgment(judgment) {
    this.judgment = {
      score: judgment.score || 0,
      verdict: judgment.verdict || 'UNKNOWN',
      consensus: judgment.consensus || false,
      consensusRatio: judgment.consensusRatio || 0,
      votes: judgment.votes || [],
      blocked: judgment.blocked || null,
      dimensions: judgment.dimensions || {},
      timestamp: Date.now(),
    };
    this.stage = DecisionStage.JUDGMENT;
    this._addTrace('judgment', {
      score: this.judgment.score,
      verdict: this.judgment.verdict,
      consensus: this.judgment.consensus,
      blocked: !!this.judgment.blocked,
    });

    // Update outcome based on judgment
    if (this.judgment.blocked) {
      this.outcome = DecisionOutcome.BLOCK;
      this.reasoning.push(`DOG BLOCK: ${this.judgment.blocked.agent} - ${this.judgment.blocked.reason}`);
    } else if (this.judgment.verdict === 'HOWL' || this.judgment.verdict === 'WAG') {
      this.outcome = DecisionOutcome.ALLOW;
    } else if (this.judgment.verdict === 'GROWL') {
      this.outcome = DecisionOutcome.WARN;
    } else if (this.judgment.verdict === 'BARK') {
      this.outcome = DecisionOutcome.BLOCK;
    }

    return this;
  }

  /**
   * Record engine synthesis result
   *
   * @param {Object} synthesis - Engine synthesis
   * @param {string} synthesis.insight - Synthesized insight
   * @param {number} synthesis.confidence - Confidence (0-1)
   * @param {string} synthesis.strategy - Synthesis strategy used
   * @param {Object[]} synthesis.consultations - Individual engine consultations
   */
  setSynthesis(synthesis) {
    this.synthesis = {
      insight: synthesis.insight || null,
      confidence: synthesis.confidence || 0,
      strategy: synthesis.strategy || 'unknown',
      consultations: synthesis.consultations || [],
      timestamp: Date.now(),
    };
    this.stage = DecisionStage.SYNTHESIS;
    this._addTrace('synthesis', {
      confidence: this.synthesis.confidence,
      strategy: this.synthesis.strategy,
      enginesConsulted: this.synthesis.consultations.length,
    });

    return this;
  }

  /**
   * Record execution result
   *
   * @param {Object} execution - Execution result
   * @param {string} execution.tool - Tool/skill executed
   * @param {boolean} execution.success - Whether execution succeeded
   * @param {*} execution.result - Execution result
   * @param {string} execution.error - Error if failed
   */
  setExecution(execution) {
    this.execution = {
      tool: execution.tool || null,
      skill: execution.skill || null,
      success: execution.success !== false,
      result: execution.result || null,
      error: execution.error || null,
      timestamp: Date.now(),
    };
    this.stage = DecisionStage.EXECUTION;
    this._addTrace('execution', {
      tool: this.execution.tool,
      skill: this.execution.skill,
      success: this.execution.success,
    });

    return this;
  }

  /**
   * Record post-execution observation
   *
   * @param {Object} observation - Post-execution observation
   */
  setPostExecution(observation) {
    this.stage = DecisionStage.POST_EXECUTION;
    this._addTrace('post_execution', observation);
    return this;
  }

  /**
   * Finalize the decision
   *
   * @param {DecisionOutcome} outcome - Final outcome
   * @param {string[]} reasoning - Final reasoning
   */
  finalize(outcome, reasoning = []) {
    this.outcome = outcome || this.outcome;
    this.reasoning = [...this.reasoning, ...reasoning];
    this.stage = DecisionStage.COMPLETE;

    this.decision = {
      outcome: this.outcome,
      reasoning: this.reasoning,
      routing: this.routing,
      judgment: this.judgment ? {
        score: this.judgment.score,
        verdict: this.judgment.verdict,
        consensus: this.judgment.consensus,
      } : null,
    };

    this.meta.endTime = Date.now();
    this.meta.durationMs = this.meta.endTime - this.meta.startTime;

    this._addTrace('finalized', {
      outcome: this.outcome,
      durationMs: this.meta.durationMs,
    });

    return this;
  }

  /**
   * Record an error
   *
   * @param {string} stage - Stage where error occurred
   * @param {Error|string} error - The error
   */
  recordError(stage, error) {
    const errorInfo = {
      stage,
      message: error.message || String(error),
      timestamp: Date.now(),
    };
    this.meta.errors.push(errorInfo);
    this._addTrace('error', errorInfo);
    return this;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TRACE
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Add entry to decision trace
   * @private
   */
  _addTrace(action, data) {
    this.trace.push({
      action,
      data,
      timestamp: Date.now(),
      elapsed: Date.now() - this.meta.startTime,
    });
  }

  /**
   * Get formatted trace for display
   *
   * @returns {string} Human-readable trace
   */
  getFormattedTrace() {
    const lines = [];
    lines.push(`┌─ DECISION TRACE [${this.id}]`);
    lines.push(`│ Event: ${this.eventType} | Source: ${this.source}`);
    lines.push(`│ User: ${this.userContext.userId || 'anonymous'} (Trust: ${this.userContext.trustLevel})`);
    lines.push('│');

    for (const entry of this.trace) {
      const icon = this._getTraceIcon(entry.action);
      const elapsed = `+${entry.elapsed}ms`;
      lines.push(`├─ ${icon} ${entry.action.toUpperCase()} ${elapsed}`);

      // Add relevant details
      if (entry.action === 'routing' && entry.data.sefirah) {
        lines.push(`│   → Sefirah: ${entry.data.sefirah} | Risk: ${entry.data.risk}`);
      }
      if (entry.action === 'judgment') {
        lines.push(`│   → Score: ${entry.data.score} | Verdict: ${entry.data.verdict}`);
      }
      if (entry.action === 'error') {
        lines.push(`│   ❌ ${entry.data.message}`);
      }
    }

    lines.push('│');
    lines.push(`└─ OUTCOME: ${this.outcome} (${this.meta.durationMs || '?'}ms)`);

    return lines.join('\n');
  }

  /**
   * Get icon for trace action
   * @private
   */
  _getTraceIcon(action) {
    const icons = {
      created: '🆕',
      routing: '🔀',
      pre_execution: '🛡️',
      judgment: '⚖️',
      synthesis: '🧠',
      execution: '⚡',
      post_execution: '👁️',
      finalized: '✅',
      error: '❌',
    };
    return icons[action] || '•';
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SERIALIZATION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Convert to JSON-serializable object
   */
  toJSON() {
    return {
      id: this.id,
      parentId: this.parentId,
      timestamp: this.timestamp,
      eventType: this.eventType,
      source: this.source,
      content: this.content,
      context: this.context,
      userContext: this.userContext,
      stage: this.stage,
      outcome: this.outcome,
      routing: this.routing,
      judgment: this.judgment,
      synthesis: this.synthesis,
      execution: this.execution,
      decision: this.decision,
      reasoning: this.reasoning,
      trace: this.trace,
      meta: this.meta,
    };
  }

  /**
   * Create from JSON
   * @param {Object} json - JSON data
   * @returns {DecisionEvent}
   */
  static fromJSON(json) {
    const event = new DecisionEvent({
      eventType: json.eventType,
      source: json.source,
      content: json.content,
      context: json.context,
      userContext: json.userContext,
    });

    // Restore state
    event.id = json.id;
    event.parentId = json.parentId;
    event.timestamp = json.timestamp;
    event.stage = json.stage;
    event.outcome = json.outcome;
    event.routing = json.routing;
    event.judgment = json.judgment;
    event.synthesis = json.synthesis;
    event.execution = json.execution;
    event.decision = json.decision;
    event.reasoning = json.reasoning;
    event.trace = json.trace;
    event.meta = json.meta;

    return event;
  }
}

/**
 * Create a decision event from a hook context
 *
 * @param {string} hookName - Hook name (perceive, guard, observe, etc.)
 * @param {Object} hookContext - Context from the hook
 * @returns {DecisionEvent}
 */
export function createFromHook(hookName, hookContext) {
  const sourceMap = {
    perceive: EventSource.HOOK_PERCEIVE,
    guard: EventSource.HOOK_GUARD,
    observe: EventSource.HOOK_OBSERVE,
    digest: EventSource.HOOK_DIGEST,
  };

  return new DecisionEvent({
    eventType: hookContext.eventType || hookContext.event || 'hook_event',
    source: sourceMap[hookName] || EventSource.INTERNAL,
    content: hookContext.content || hookContext.prompt || hookContext.command || '',
    context: hookContext,
    userContext: {
      userId: hookContext.userId || hookContext.user?.userId,
      sessionId: hookContext.sessionId,
      project: hookContext.project || hookContext.cwd,
    },
  });
}

/**
 * Create a decision event from an MCP tool call
 *
 * @param {string} toolName - Tool name
 * @param {Object} params - Tool parameters
 * @returns {DecisionEvent}
 */
export function createFromTool(toolName, params) {
  return new DecisionEvent({
    eventType: 'tool_use',
    source: EventSource.MCP_TOOL,
    content: toolName,
    context: { toolName, params },
    userContext: {
      userId: params.userId,
      sessionId: params.sessionId,
    },
  });
}

export default DecisionEvent;

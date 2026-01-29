/**
 * Orchestration Client - Wrapper for MCP orchestration calls
 *
 * Provides a consistent interface for hooks to request decisions from
 * the UnifiedOrchestrator, with automatic SessionState context injection.
 *
 * "Le cerveau collectif décide" - The collective brain decides
 *
 * @module scripts/hooks/lib/orchestration-client
 */

'use strict';

import { getSessionState } from './session-state.js';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/** Default timeout for orchestration calls (ms) */
const DEFAULT_TIMEOUT_MS = 5000;

/** Timeout for high-risk decisions (ms) */
const HIGH_RISK_TIMEOUT_MS = 8000;

// ═══════════════════════════════════════════════════════════════════════════
// ORCHESTRATION CLIENT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * OrchestrationClient - Unified interface for orchestration decisions
 *
 * Wraps the MCP orchestrateFull() call with:
 * - Automatic SessionState context injection
 * - Timeout handling with fallback
 * - Normalized response format
 */
class OrchestrationClient {
  constructor() {
    this._orchestrateFull = null;
    this._initialized = false;
  }

  /**
   * Initialize client with orchestration function
   * @param {Function} orchestrateFull - The orchestrateFull function from lib
   */
  init(orchestrateFull) {
    this._orchestrateFull = orchestrateFull;
    this._initialized = true;
  }

  /**
   * Check if initialized
   * @returns {boolean}
   */
  isInitialized() {
    return this._initialized && this._orchestrateFull !== null;
  }

  /**
   * Request a decision from the orchestrator
   *
   * @param {Object} params - Decision parameters
   * @param {string} params.content - Content to evaluate (command, prompt, etc.)
   * @param {string} params.eventType - Event type (tool_use, user_prompt, etc.)
   * @param {boolean} [params.requestJudgment=false] - Whether to get 11 Dogs judgment
   * @param {Object} [params.metadata] - Additional metadata
   * @param {Object} [params.sessionContext] - Override session context (auto-injected if not provided)
   * @param {number} [params.timeout] - Custom timeout in ms
   * @returns {Promise<OrchestrationDecision>} Normalized decision
   */
  async decide(params) {
    const {
      content,
      eventType,
      requestJudgment = false,
      metadata = {},
      sessionContext = null,
      timeout = requestJudgment ? HIGH_RISK_TIMEOUT_MS : DEFAULT_TIMEOUT_MS,
    } = params;

    // Build session context from SessionState if not provided
    const context = sessionContext || this._buildSessionContext();

    // Merge session context into metadata
    const enrichedMetadata = {
      ...metadata,
      sessionContext: context,
      escalationLevel: context.escalationLevel,
      consecutiveErrors: context.consecutiveErrors,
    };

    // If not initialized, return allow decision
    if (!this.isInitialized()) {
      return this._createFallbackDecision('allow', 'Orchestration not initialized');
    }

    try {
      // Call orchestrateFull with timeout
      const result = await this._withTimeout(
        this._orchestrateFull(content, {
          eventType,
          requestJudgment,
          metadata: enrichedMetadata,
        }),
        timeout
      );

      // Normalize the response
      return this._normalizeDecision(result);
    } catch (error) {
      // On timeout or error, return safe fallback
      const isTimeout = error.message?.includes('timed out');
      return this._createFallbackDecision(
        'allow',
        isTimeout ? 'Orchestration timed out' : `Orchestration error: ${error.message}`
      );
    }
  }

  /**
   * Quick decision for low-risk operations (no judgment, shorter timeout)
   *
   * @param {string} content - Content to evaluate
   * @param {string} eventType - Event type
   * @param {Object} [metadata] - Additional metadata
   * @returns {Promise<OrchestrationDecision>}
   */
  async quickDecide(content, eventType, metadata = {}) {
    return this.decide({
      content,
      eventType,
      requestJudgment: false,
      metadata,
      timeout: 2000, // Very short timeout for quick checks
    });
  }

  /**
   * Full decision with 11 Dogs judgment (for high-risk operations)
   *
   * @param {string} content - Content to evaluate
   * @param {string} eventType - Event type
   * @param {Object} [metadata] - Additional metadata
   * @returns {Promise<OrchestrationDecision>}
   */
  async fullDecide(content, eventType, metadata = {}) {
    return this.decide({
      content,
      eventType,
      requestJudgment: true,
      metadata,
      timeout: HIGH_RISK_TIMEOUT_MS,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PRIVATE HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Build session context from SessionState
   * @private
   */
  _buildSessionContext() {
    const sessionState = getSessionState();

    if (!sessionState.isInitialized()) {
      return {
        escalationLevel: 'normal',
        consecutiveErrors: 0,
        recentPrompts: [],
        recentTools: [],
        activeWarnings: [],
      };
    }

    return {
      escalationLevel: sessionState.getEscalationLevel(),
      consecutiveErrors: sessionState.getConsecutiveErrors(),
      recentPrompts: sessionState.getPromptHistory(3).map(p => p.content?.slice(0, 100)),
      recentTools: sessionState.getToolHistory(5).map(t => ({
        tool: t.tool,
        isError: t.isError,
      })),
      recentErrors: sessionState.getRecentErrors(3).map(e => e.errorType),
      activeWarnings: sessionState.getActiveWarnings().map(w => w.message),
    };
  }

  /**
   * Normalize orchestration response to consistent format
   * @private
   */
  _normalizeDecision(result) {
    if (!result) {
      return this._createFallbackDecision('allow', 'Empty result');
    }

    // Handle different response formats from orchestrateFull
    return {
      // Core decision
      outcome: result.outcome || 'allow',
      reasoning: this._extractReasoning(result),

      // Judgment details (if requested)
      judgment: result.judgment ? {
        qScore: result.judgment.qScore,
        verdict: result.judgment.verdict,
        confidence: result.judgment.confidence,
        axiomScores: result.judgment.axiomScores || result.judgment.breakdown,
      } : null,

      // Tracing
      decisionId: result.decisionId || null,
      success: result.success !== false,

      // Synthesis (for perceive hook)
      synthesis: result.synthesis || null,

      // Original result for debugging
      _raw: process.env.CYNIC_DEBUG ? result : undefined,
    };
  }

  /**
   * Extract reasoning from various result formats
   * @private
   */
  _extractReasoning(result) {
    if (Array.isArray(result.reasoning)) {
      return result.reasoning;
    }
    if (typeof result.reasoning === 'string') {
      return [result.reasoning];
    }
    if (result.judgment?.reasoning) {
      return [result.judgment.reasoning];
    }
    return [];
  }

  /**
   * Create fallback decision when orchestration fails
   * @private
   */
  _createFallbackDecision(outcome, reason) {
    return {
      outcome,
      reasoning: [reason],
      judgment: null,
      decisionId: null,
      success: false,
      synthesis: null,
      _fallback: true,
    };
  }

  /**
   * Wrap promise with timeout
   * @private
   */
  async _withTimeout(promise, timeoutMs) {
    let timeoutId;
    const timeoutPromise = new Promise((_, reject) => {
      timeoutId = setTimeout(() => {
        reject(new Error(`Operation timed out after ${timeoutMs}ms`));
      }, timeoutMs);
    });

    try {
      const result = await Promise.race([promise, timeoutPromise]);
      clearTimeout(timeoutId);
      return result;
    } catch (error) {
      clearTimeout(timeoutId);
      throw error;
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON & EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

/** Singleton instance */
let _instance = null;

/**
 * Get OrchestrationClient singleton instance
 * @returns {OrchestrationClient}
 */
export function getOrchestrationClient() {
  if (!_instance) {
    _instance = new OrchestrationClient();
  }
  return _instance;
}

/**
 * Initialize OrchestrationClient with orchestrateFull function
 * Call this once during hook startup
 *
 * @param {Function} orchestrateFull - The orchestrateFull function from lib/index.js
 */
export function initOrchestrationClient(orchestrateFull) {
  const client = getOrchestrationClient();
  client.init(orchestrateFull);
}

/**
 * @typedef {Object} OrchestrationDecision
 * @property {'allow'|'block'|'warn'} outcome - Decision outcome
 * @property {string[]} reasoning - Reasons for decision
 * @property {Object|null} judgment - 11 Dogs judgment details
 * @property {number|null} judgment.qScore - Quality score (0-100)
 * @property {string|null} judgment.verdict - HOWL/WAG/GROWL/BARK
 * @property {number|null} judgment.confidence - Confidence (0-1)
 * @property {string|null} decisionId - Unique decision ID for tracing
 * @property {boolean} success - Whether orchestration succeeded
 * @property {Object|null} synthesis - Synthesis for prompt injection
 */

export { OrchestrationClient };
export default OrchestrationClient;

/**
 * UnifiedOrchestrator - Central Coordination Facade
 *
 * Single entry point for ALL orchestration in CYNIC:
 * - Receives events from hooks, skills, MCP tools, API
 * - Routes through KETER for initial decision
 * - Spawns dogs for judgment if needed
 * - Consults engines for synthesis if needed
 * - Auto-invokes skills based on routing
 * - Records outcomes for learning
 *
 * "φ coordinates all" - κυνικός
 *
 * @module @cynic/node/orchestration/unified-orchestrator
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, getCircuitBreakerRegistry } from '@cynic/core';
import {
  DecisionEvent,
  DecisionStage,
  DecisionOutcome,
  EventSource,
  createFromHook,
  createFromTool,
} from './decision-event.js';
import { getEventBus, EventType } from '../services/event-bus.js';
import {
  SEFIROT_ROUTING,
  TRUST_THRESHOLDS,
  RISK_INTERVENTIONS,
  DANGER_PATTERNS,
  calculateTrustLevel,
  determineIntervention,
  detectRisk,
  findRouting,
} from './routing-config.js';

const log = createLogger('UnifiedOrchestrator');

/**
 * Unified Orchestrator
 *
 * Coordinates all CYNIC components through a single interface.
 */
export class UnifiedOrchestrator extends EventEmitter {
  /**
   * Create the orchestrator
   *
   * @param {Object} options - Options
   * @param {Object} [options.dogOrchestrator] - DogOrchestrator instance
   * @param {Object} [options.engineOrchestrator] - EngineOrchestrator instance
   * @param {Object} [options.skillRegistry] - SkillRegistry instance
   * @param {Object} [options.persistence] - PersistenceManager instance
   * @param {Object} [options.eventBus] - EventBus instance
   * @param {Object} [options.decisionTracer] - DecisionTracer instance
   */
  constructor(options = {}) {
    super();

    this.dogOrchestrator = options.dogOrchestrator || null;
    this.engineOrchestrator = options.engineOrchestrator || null;
    this.skillRegistry = options.skillRegistry || null;
    this.persistence = options.persistence || null;
    this.eventBus = options.eventBus || getEventBus();
    this.decisionTracer = options.decisionTracer || null;

    // Circuit breakers for external calls
    const cbRegistry = getCircuitBreakerRegistry();
    this._circuitBreakers = {
      judgment: cbRegistry.get('orchestrator:judgment', { timeout: 8000 }),
      synthesis: cbRegistry.get('orchestrator:synthesis', { timeout: 8000 }),
      skill: cbRegistry.get('orchestrator:skill', { timeout: 13000 }),
    };

    // User profile cache (per-session)
    this._profileCache = new Map();
    this._profileCacheTTL = 5 * 60 * 1000; // 5 minutes

    // Decision history (for learning)
    this._recentDecisions = [];
    this._maxRecentDecisions = 100;

    // Statistics
    this.stats = {
      eventsProcessed: 0,
      decisionsRouted: 0,
      judgmentsRequested: 0,
      synthesisRequested: 0,
      skillsInvoked: 0,
      blocked: 0,
      errors: 0,
    };

    log.debug('UnifiedOrchestrator created');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MAIN ENTRY POINT
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Process an event through the orchestration pipeline
   *
   * This is the main entry point for ALL orchestration requests.
   *
   * @param {DecisionEvent|Object} eventOrOptions - DecisionEvent or options to create one
   * @returns {Promise<DecisionEvent>} The processed decision event
   */
  async process(eventOrOptions) {
    let event;

    // Create DecisionEvent if not already one
    if (eventOrOptions instanceof DecisionEvent) {
      event = eventOrOptions;
    } else {
      event = new DecisionEvent(eventOrOptions);
    }

    this.stats.eventsProcessed++;

    // Emit orchestration start event for visibility
    this.eventBus.publish('orchestration:start', {
      decisionId: event.id,
      eventType: event.eventType,
      content: event.content?.substring(0, 100),
    }, { source: 'UnifiedOrchestrator' });

    try {
      // 1. Load/cache user profile
      await this._loadUserProfile(event);

      // 2. Route through KETER
      await this._routeEvent(event);

      // 3. Pre-execution check (if needed)
      if (this._needsPreCheck(event)) {
        await this._preExecutionCheck(event);
      }

      // 4. If blocked, finalize early
      if (event.outcome === DecisionOutcome.BLOCK) {
        event.finalize(DecisionOutcome.BLOCK);
        this.stats.blocked++;
        this._recordDecision(event);
        return event;
      }

      // 5. Request judgment if needed
      if (this._needsJudgment(event)) {
        await this._requestJudgment(event);
      }

      // 6. Request synthesis if needed
      if (this._needsSynthesis(event)) {
        await this._requestSynthesis(event);
      }

      // 7. Auto-invoke skill if routing suggests one
      if (this._shouldInvokeSkill(event)) {
        await this._invokeSkill(event);
      }

      // 8. Finalize decision
      event.finalize(event.outcome || DecisionOutcome.ALLOW);

      // 9. Record for learning
      this._recordDecision(event);

      // 10. Emit event
      this.eventBus.publish('orchestration:complete', {
        decisionId: event.id,
        outcome: event.outcome,
        routing: event.routing,
      }, { source: 'UnifiedOrchestrator' });

      this.emit('decision', event);

    } catch (err) {
      event.recordError('process', err);
      event.finalize(DecisionOutcome.ALLOW, [`Error during orchestration: ${err.message}`]);
      this.stats.errors++;
      log.error('Orchestration error', { eventId: event.id, error: err.message });
    }

    return event;
  }

  /**
   * Process a hook event
   *
   * @param {string} hookName - Hook name (perceive, guard, observe, digest)
   * @param {Object} hookContext - Context from the hook
   * @returns {Promise<DecisionEvent>}
   */
  async processHook(hookName, hookContext) {
    const event = createFromHook(hookName, hookContext);
    return this.process(event);
  }

  /**
   * Process an MCP tool call
   *
   * @param {string} toolName - Tool name
   * @param {Object} params - Tool parameters
   * @returns {Promise<DecisionEvent>}
   */
  async processTool(toolName, params) {
    const event = createFromTool(toolName, params);
    return this.process(event);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PIPELINE STAGES
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Load user profile (with caching)
   * @private
   */
  async _loadUserProfile(event) {
    const userId = event.userContext.userId;
    if (!userId) return;

    // Check cache
    const cached = this._profileCache.get(userId);
    if (cached && Date.now() - cached.timestamp < this._profileCacheTTL) {
      event.userContext.eScore = cached.eScore;
      event.userContext.trustLevel = cached.trustLevel;
      return;
    }

    // Load from persistence
    if (this.persistence?.userLearningProfiles) {
      try {
        const profile = await this.persistence.userLearningProfiles.getSummary(userId);
        if (profile) {
          const eScore = profile.eScore ?? 50;
          const trustLevel = calculateTrustLevel(eScore);

          event.userContext.eScore = eScore;
          event.userContext.trustLevel = trustLevel;

          // Cache it
          this._profileCache.set(userId, {
            eScore,
            trustLevel,
            timestamp: Date.now(),
          });
        }
      } catch (err) {
        log.warn('Failed to load user profile', { userId, error: err.message });
      }
    }
  }

  /**
   * Route event through KETER logic
   * @private
   */
  async _routeEvent(event) {
    const content = event.content.toLowerCase();

    // Find matching sefirah
    let matchedDomain = null;
    let matchedRouting = null;

    for (const [domain, routing] of Object.entries(SEFIROT_ROUTING)) {
      for (const trigger of routing.triggers) {
        if (content.includes(trigger)) {
          matchedDomain = domain;
          matchedRouting = routing;
          break;
        }
      }
      if (matchedDomain) break;
    }

    // Determine risk level
    const risk = detectRisk(event.content);

    // Determine intervention based on trust level and risk
    const intervention = determineIntervention(event.userContext.trustLevel, risk);

    // Set routing on event
    event.setRouting({
      sefirah: matchedRouting?.sefirah || null,
      domain: matchedDomain || 'general',
      intervention,
      risk,
      suggestedAgent: matchedRouting?.agent || null,
      suggestedTools: matchedRouting?.tools || [],
    });

    this.stats.decisionsRouted++;

    return event;
  }

  /**
   * Pre-execution check (guard-like)
   * @private
   */
  async _preExecutionCheck(event) {
    // Check for dangerous patterns using shared config
    const content = event.content;

    for (const pattern of DANGER_PATTERNS) {
      if (pattern.test(content)) {
        event.setPreExecution({
          blocked: true,
          reason: 'Dangerous command pattern detected',
          severity: 'critical',
        });
        return event;
      }
    }

    event.setPreExecution({
      blocked: false,
      warning: false,
    });

    return event;
  }

  /**
   * Request judgment from dog orchestrator
   * @private
   */
  async _requestJudgment(event) {
    if (!this.dogOrchestrator) {
      log.debug('No dog orchestrator available, skipping judgment');
      return event;
    }

    // Circuit breaker protection for judgment requests
    const cb = this._circuitBreakers.judgment;
    if (!cb.isAllowed()) {
      log.warn('Judgment circuit breaker open, skipping');
      event.recordError('judgment', new Error('Circuit breaker open'));
      return event;
    }

    try {
      const item = {
        content: event.content,
        itemType: event.eventType,
        context: event.context,
      };

      const result = await this.dogOrchestrator.judge(item);

      event.setJudgment({
        score: result.score,
        verdict: result.verdict,
        consensus: result.consensus,
        consensusRatio: result.consensusRatio,
        votes: result.votes,
        blocked: result.blocked,
        dimensions: result.dimensions,
      });

      cb.recordSuccess();
      this.stats.judgmentsRequested++;
    } catch (err) {
      cb.recordFailure(err);
      event.recordError('judgment', err);
      log.warn('Judgment request failed', { error: err.message });
    }

    return event;
  }

  /**
   * Request synthesis from engine orchestrator
   * @private
   */
  async _requestSynthesis(event) {
    if (!this.engineOrchestrator) {
      log.debug('No engine orchestrator available, skipping synthesis');
      return event;
    }

    // Circuit breaker protection for synthesis requests
    const cb = this._circuitBreakers.synthesis;
    if (!cb.isAllowed()) {
      log.warn('Synthesis circuit breaker open, skipping');
      event.recordError('synthesis', new Error('Circuit breaker open'));
      return event;
    }

    try {
      const result = await this.engineOrchestrator.consult({
        query: event.content,
        domain: event.routing?.domain,
        context: event.context,
      });

      event.setSynthesis({
        insight: result.synthesis?.insight,
        confidence: result.confidence,
        strategy: result.strategy,
        consultations: result.consultations,
      });

      cb.recordSuccess();
      this.stats.synthesisRequested++;
    } catch (err) {
      cb.recordFailure(err);
      event.recordError('synthesis', err);
      log.warn('Synthesis request failed', { error: err.message });
    }

    return event;
  }

  /**
   * Auto-invoke skill based on routing
   * @private
   */
  async _invokeSkill(event) {
    if (!this.skillRegistry) {
      return event;
    }

    const domain = event.routing?.domain;
    if (!domain) return event;

    // Circuit breaker protection for skill invocations
    const cb = this._circuitBreakers.skill;
    if (!cb.isAllowed()) {
      log.warn('Skill circuit breaker open, skipping', { domain });
      event.recordError('skill_invoke', new Error('Circuit breaker open'));
      return event;
    }

    try {
      const skill = this.skillRegistry.getSkillForDomain(domain);
      if (skill) {
        const result = await this.skillRegistry.invoke(skill.name, {
          content: event.content,
          context: event.context,
          userContext: event.userContext,
        });

        event.setExecution({
          skill: skill.name,
          success: result.success,
          result: result.data,
          error: result.error,
        });

        cb.recordSuccess();
        this.stats.skillsInvoked++;
      }
    } catch (err) {
      cb.recordFailure(err);
      event.recordError('skill_invoke', err);
      log.warn('Skill invocation failed', { domain, error: err.message });
    }

    return event;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  // Helper methods now imported from routing-config.js
  // _calculateTrustLevel → calculateTrustLevel
  // _detectRisk → detectRisk
  // _determineIntervention → determineIntervention

  /**
   * Check if event needs pre-execution check
   * @private
   */
  _needsPreCheck(event) {
    // Always check tool_use events
    return event.eventType === 'tool_use' ||
           event.source === EventSource.HOOK_GUARD ||
           event.routing?.risk === 'critical' ||
           event.routing?.risk === 'high';
  }

  /**
   * Check if event needs judgment
   * @private
   */
  _needsJudgment(event) {
    // Explicit request overrides automatic detection
    if (event.requestJudgment === true) return true;
    if (event.requestJudgment === false) return false;

    // Request judgment for:
    // - High/critical risk events
    // - Protection domain
    // - Explicit judgment requests
    return event.eventType === 'judgment_request' ||
           event.routing?.domain === 'protection' ||
           event.routing?.risk === 'critical' ||
           event.routing?.intervention === 'ask';
  }

  /**
   * Check if event needs synthesis
   * @private
   */
  _needsSynthesis(event) {
    // Explicit request overrides automatic detection
    if (event.requestSynthesis === true) return true;
    if (event.requestSynthesis === false) return false;

    // Request synthesis for wisdom/analysis domains
    return event.routing?.domain === 'wisdom' ||
           event.routing?.domain === 'analysis' ||
           event.routing?.domain === 'design';
  }

  /**
   * Check if should auto-invoke skill
   * @private
   */
  _shouldInvokeSkill(event) {
    // Only auto-invoke if:
    // - We have a skill registry
    // - Routing suggests a domain
    // - Not blocked
    // - Outcome is ALLOW
    return this.skillRegistry &&
           event.routing?.domain &&
           event.outcome !== DecisionOutcome.BLOCK &&
           event.routing?.intervention !== 'block';
  }

  /**
   * Record decision for learning
   * @private
   */
  _recordDecision(event) {
    // Add to recent decisions
    this._recentDecisions.push({
      id: event.id,
      timestamp: event.timestamp,
      outcome: event.outcome,
      routing: event.routing,
      judgment: event.judgment ? {
        score: event.judgment.score,
        verdict: event.judgment.verdict,
      } : null,
    });

    // Trim if too many
    if (this._recentDecisions.length > this._maxRecentDecisions) {
      this._recentDecisions.shift();
    }

    // Record in tracer if available
    if (this.decisionTracer) {
      this.decisionTracer.record(event);
    }

    // Emit feedback for learning
    this.eventBus.publish(EventType.FEEDBACK_RECEIVED, {
      source: 'orchestration',
      decisionId: event.id,
      outcome: event.outcome,
      judgment: event.judgment,
    }, { source: 'UnifiedOrchestrator' });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PUBLIC API
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get recent decisions
   * @returns {Object[]}
   */
  getRecentDecisions(limit = 10) {
    return this._recentDecisions.slice(-limit);
  }

  /**
   * Get statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      circuitBreakers: {
        judgment: this._circuitBreakers.judgment.getStats(),
        synthesis: this._circuitBreakers.synthesis.getStats(),
        skill: this._circuitBreakers.skill.getStats(),
      },
    };
  }

  /**
   * Get circuit breaker health status
   * @returns {Object}
   */
  getCircuitBreakerHealth() {
    return {
      judgment: this._circuitBreakers.judgment.getHealth(),
      synthesis: this._circuitBreakers.synthesis.getHealth(),
      skill: this._circuitBreakers.skill.getHealth(),
    };
  }

  /**
   * Reset all circuit breakers
   */
  resetCircuitBreakers() {
    this._circuitBreakers.judgment.reset();
    this._circuitBreakers.synthesis.reset();
    this._circuitBreakers.skill.reset();
    log.debug('All circuit breakers reset');
  }

  /**
   * Clear profile cache
   */
  clearProfileCache() {
    this._profileCache.clear();
  }

  /**
   * Update user profile in cache
   *
   * @param {string} userId - User ID
   * @param {Object} update - Profile update (eScore, trustLevel)
   */
  updateProfileCache(userId, update) {
    const existing = this._profileCache.get(userId) || {};
    this._profileCache.set(userId, {
      ...existing,
      ...update,
      timestamp: Date.now(),
    });
  }
}

/**
 * Create a UnifiedOrchestrator instance
 *
 * @param {Object} options - Options
 * @returns {UnifiedOrchestrator}
 */
export function createUnifiedOrchestrator(options) {
  return new UnifiedOrchestrator(options);
}

// Singleton instance
let _globalOrchestrator = null;

/**
 * Get the global orchestrator instance
 *
 * @param {Object} [options] - Options for creation if not exists
 * @returns {UnifiedOrchestrator}
 */
export function getOrchestrator(options) {
  if (!_globalOrchestrator) {
    _globalOrchestrator = new UnifiedOrchestrator(options);
  }
  return _globalOrchestrator;
}

export default UnifiedOrchestrator;

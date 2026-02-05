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
import { createLogger, PHI_INV, PHI_INV_2, getCircuitBreakerRegistry } from '@cynic/core';
import {
  DecisionEvent,
  DecisionStage,
  DecisionOutcome,
  EventSource,
  createFromHook,
  createFromTool,
} from './decision-event.js';
import { PlanningGate, PlanningDecision } from './planning-gate.js';
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
   * @param {Object} [options.kabbalisticRouter] - KabbalisticRouter instance (for low-confidence escalation)
   * @param {Object} [options.learningService] - QLearningService instance (for Q-learning routing)
   * @param {Object} [options.costOptimizer] - CostOptimizer instance (for tier selection)
   * @param {Object} [options.skillRegistry] - SkillRegistry instance
   * @param {Object} [options.persistence] - PersistenceManager instance
   * @param {Object} [options.eventBus] - EventBus instance
   * @param {Object} [options.decisionTracer] - DecisionTracer instance
   * @param {Object} [options.planningGate] - PlanningGate instance for meta-cognition
   * @param {Object} [options.llmRouter] - LLMRouter instance for multi-model routing
   * @param {Object} [options.perceptionRouter] - PerceptionRouter for data source routing
   * @param {Object} [options.psychologyProvider] - Psychology state provider (D11: calibration wiring)
   */
  constructor(options = {}) {
    super();

    this.dogOrchestrator = options.dogOrchestrator || null;
    this.engineOrchestrator = options.engineOrchestrator || null;
    this.kabbalisticRouter = options.kabbalisticRouter || null;
    this.learningService = options.learningService || null;
    this.costOptimizer = options.costOptimizer || null;
    this.skillRegistry = options.skillRegistry || null;
    this.persistence = options.persistence || null;
    this.eventBus = options.eventBus || getEventBus();
    this.decisionTracer = options.decisionTracer || null;
    this.planningGate = options.planningGate || null;
    this.llmRouter = options.llmRouter || null;
    this.perceptionRouter = options.perceptionRouter || null;
    this.memoryRetriever = options.memoryRetriever || null;
    this.psychologyProvider = options.psychologyProvider || null;

    // Wire learning and cost services to kabbalistic router if available
    if (this.kabbalisticRouter) {
      if (this.learningService) {
        this.kabbalisticRouter.setLearningService(this.learningService);
      }
      if (this.costOptimizer) {
        this.kabbalisticRouter.setCostOptimizer(this.costOptimizer);
      }
    }

    // Circuit breakers for external calls
    const cbRegistry = getCircuitBreakerRegistry();
    this._circuitBreakers = {
      judgment: cbRegistry.get('orchestrator:judgment', { timeout: 8000 }),
      synthesis: cbRegistry.get('orchestrator:synthesis', { timeout: 8000 }),
      skill: cbRegistry.get('orchestrator:skill', { timeout: 13000 }),
      escalation: cbRegistry.get('orchestrator:escalation', { timeout: 13000 }),
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
      planningTriggered: 0,
      planningPaused: 0,
      judgmentsRequested: 0,
      synthesisRequested: 0,
      skillsInvoked: 0,
      escalationsTriggered: 0,
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

      // 2.5. Planning gate check (meta-cognition)
      if (this._needsPlanning(event)) {
        const planningResult = await this._requestPlanning(event);
        if (planningResult?.decision === PlanningDecision.PAUSE) {
          // Pause for human approval - emit event and finalize as ASK
          event.finalize(DecisionOutcome.ASK, ['Paused for planning approval']);
          this.stats.planningPaused++;
          this._recordDecision(event);
          this.emit('planning:pause', { event, planning: planningResult });
          return event;
        }
      }

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

    // D11: Enrich with psychology state if provider available
    if (this.psychologyProvider && userId) {
      try {
        // Supports both: PsychologyRepository.loadPsychology(userId) and in-memory getSummary()
        const psy = typeof this.psychologyProvider.loadPsychology === 'function'
          ? await this.psychologyProvider.loadPsychology(userId)
          : (this.psychologyProvider.getSummary?.() || this.psychologyProvider.getState?.() || null);
        if (psy) {
          const dims = psy.dimensions || {};
          event.userContext.psychology = {
            energy: dims.energy?.value ?? dims.energy ?? null,
            focus: dims.focus?.value ?? dims.focus ?? null,
            frustration: dims.frustration?.value ?? dims.frustration ?? null,
            burnoutRisk: psy.composites?.burnoutRisk ?? 0,
            flow: psy.composites?.flow ?? 0,
            overallState: psy.overallState || null,
          };
        }
      } catch (e) {
        log.debug(`Psychology enrichment skipped: ${e.message}`);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // FIX #2: Inject relevant memory facts into context
    // "Le chien se souvient" - Memory shapes perception
    // ═══════════════════════════════════════════════════════════════════════════
    if (this.memoryRetriever && event.content && userId) {
      try {
        // Retrieve top-K relevant facts based on event content
        // API: search(userId, query, options) - returns { sources: { facts, memories, ... } }
        const query = event.content.substring(0, 200); // Limit query length
        const searchResult = await this.memoryRetriever.search(userId, query, {
          sources: ['facts', 'lessons'],  // Focus on facts and lessons learned
          limit: 5,
          useVector: !!this.memoryRetriever.embedder,  // Use vector if available
        });

        // Extract facts from search results
        const allFacts = [
          ...(searchResult?.sources?.facts || []),
          ...(searchResult?.sources?.lessons || []),
        ];

        if (allFacts.length > 0) {
          event.userContext.relevantFacts = allFacts.slice(0, 5).map(f => ({
            content: f.content || f.fact || f.text || f.description,
            confidence: f.confidence || f.score || f.similarity || PHI_INV,
            source: f.source || f.type || f.factType || 'memory',
          }));
          log.debug('Memory facts injected', { userId, factCount: event.userContext.relevantFacts.length });
        }
      } catch (e) {
        log.debug(`Memory injection skipped: ${e.message}`);
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
    let intervention = determineIntervention(event.userContext.trustLevel, risk);

    // D11: Psychology-aware routing — reduce friction during flow, add caution during burnout
    const psy = event.userContext.psychology;
    if (psy) {
      if (psy.burnoutRisk > PHI_INV) {
        // High burnout risk → escalate caution (user making mistakes)
        if (intervention === 'observe') intervention = 'warn';
        log.debug('Psychology: burnout risk elevated, increased caution');
      } else if (psy.flow > PHI_INV && risk.level === 'low') {
        // Flow state + low risk → reduce interruptions
        if (intervention === 'warn') intervention = 'observe';
        log.debug('Psychology: flow state detected, reducing friction');
      }
    }

    // Consult PerceptionRouter for data source routing (D4: close dormant loop)
    let perception = null;
    if (this.perceptionRouter) {
      try {
        perception = this.perceptionRouter.route({
          target: event.content,
          intent: 'read',
          preferStructured: true,
        });
      } catch (e) {
        log.debug(`Perception routing skipped: ${e.message}`);
      }
    }

    // Set routing on event (enriched with perception if available)
    event.setRouting({
      sefirah: matchedRouting?.sefirah || null,
      domain: matchedDomain || 'general',
      intervention,
      risk,
      suggestedAgent: matchedRouting?.agent || null,
      suggestedTools: perception?.tools?.length
        ? [...(matchedRouting?.tools || []), ...perception.tools]
        : matchedRouting?.tools || [],
      perception: perception ? {
        layer: perception.layer,
        confidence: perception.confidence,
        tools: perception.tools,
      } : null,
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

    // D10: Check lessons_learned for prevention guidance
    let lessonWarning = null;
    if (this.memoryRetriever?.checkForMistakes) {
      try {
        const userId = event.userContext?.userId || 'unknown';
        const check = await this.memoryRetriever.checkForMistakes(userId, content, { limit: 2 });
        if (check.warning) {
          lessonWarning = {
            message: check.message,
            severity: check.lessons?.[0]?.severity || 'medium',
            prevention: check.lessons?.[0]?.prevention || check.lessons?.[0]?.correction,
          };
          log.debug('Lesson matched', { tool: content.slice(0, 50), severity: lessonWarning.severity });
        }
      } catch (e) {
        log.debug(`Lesson check skipped: ${e.message}`);
      }
    }

    event.setPreExecution({
      blocked: false,
      warning: !!lessonWarning,
      lesson: lessonWarning,
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
   * Check if event needs planning gate
   * @private
   */
  _needsPlanning(event) {
    // Skip if no planning gate configured
    if (!this.planningGate) return false;

    // Explicit skip
    if (event.context?.skipPlanning === true) return false;

    // Always plan for design/architecture domains
    if (event.routing?.domain === 'design' ||
        event.routing?.domain === 'architecture') {
      return true;
    }

    // Plan for high complexity or uncertainty
    if (event.routing?.intervention === 'ask') return true;

    // Let the planning gate decide for edge cases
    return true;
  }

  /**
   * Request planning from planning gate
   * @private
   */
  async _requestPlanning(event) {
    if (!this.planningGate) return null;

    try {
      // Check if planning is needed
      const planningResult = this.planningGate.shouldPlan(event, {
        complexity: event.context?.complexity,
        confidence: event.judgment?.consensusRatio,
        entropy: event.context?.entropy,
        consensusRatio: event.judgment?.consensusRatio,
      });

      // Record planning result on event
      event.setPlanning(planningResult);

      // If planning needed, generate plan
      if (planningResult.needed) {
        this.stats.planningTriggered++;
        await this.planningGate.generatePlan(event, planningResult);
      }

      return planningResult;
    } catch (err) {
      // DEFENSIVE: Planning gate failure should NOT block execution
      log.warn('Planning gate error (non-blocking)', { error: err.message });
      event.recordError('planning', err);
      // Return continue to allow execution to proceed
      return { decision: PlanningDecision.CONTINUE, needed: false };
    }
  }

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
  // LLM ROUTING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Route a request to the appropriate LLM
   *
   * Routing logic:
   *   code/logic → Claude (primary, always available)
   *   design/UI  → Gemini API (if GEMINI_API_KEY)
   *   simple     → Ollama/local (if OLLAMA_ENDPOINT)
   *   fallback   → Claude
   *
   * @param {Object} request - Request to route
   * @param {string} request.content - Request content
   * @param {string} [request.domain] - Task domain hint
   * @param {Object} [request.context] - Additional context
   * @returns {Promise<Object>} Routed LLM response
   */
  async routeToLLM(request) {
    if (!this.llmRouter) {
      return { error: 'LLM Router not configured', tier: 'none' };
    }

    try {
      return await this.llmRouter.route(request);
    } catch (err) {
      log.error('LLM routing failed', { error: err.message });
      return { error: err.message, tier: 'error' };
    }
  }

  /**
   * Set LLM Router at runtime
   *
   * @param {Object} llmRouter - LLMRouter instance
   */
  setLLMRouter(llmRouter) {
    this.llmRouter = llmRouter;
    log.debug('LLM Router set');
  }

  /**
   * Set the PerceptionRouter for data source routing
   * @param {Object} perceptionRouter - PerceptionRouter instance
   */
  setPerceptionRouter(perceptionRouter) {
    this.perceptionRouter = perceptionRouter;
    log.debug('Perception Router set');
  }

  /**
   * Consult PerceptionRouter for optimal data source
   *
   * @param {string} target - URL, path, keyword, or intent description
   * @param {Object} [options] - Routing options
   * @param {string} [options.intent='read'] - read|write|monitor
   * @param {boolean} [options.preferStructured=true] - Prefer API/MCP over browser
   * @param {boolean} [options.preferFast=false] - Prefer speed over accuracy
   * @returns {Object|null} Routing decision { layer, tools, plan, confidence }
   */
  requestPerception(target, options = {}) {
    if (!this.perceptionRouter) return null;
    try {
      const result = this.perceptionRouter.route({
        target,
        intent: options.intent || 'read',
        preferStructured: options.preferStructured !== false,
        preferFast: options.preferFast || false,
      });
      this.stats.perceptionRouted = (this.stats.perceptionRouted || 0) + 1;
      return result;
    } catch (e) {
      log.debug(`Perception routing failed for "${target}": ${e.message}`);
      return null;
    }
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
    const stats = {
      ...this.stats,
      circuitBreakers: {
        judgment: this._circuitBreakers.judgment.getStats(),
        synthesis: this._circuitBreakers.synthesis.getStats(),
        skill: this._circuitBreakers.skill.getStats(),
      },
    };

    // Add learning stats if available
    if (this.learningService) {
      stats.learning = this.learningService.getStats();
    }

    // Add cost stats if available
    if (this.costOptimizer) {
      stats.cost = this.costOptimizer.getStats();
    }

    // Add kabbalistic router stats if available
    if (this.kabbalisticRouter) {
      stats.kabbalistic = this.kabbalisticRouter.getStats();
    }

    // Add planning gate stats if available
    if (this.planningGate) {
      stats.planning = this.planningGate.getStats();
    }

    // Add LLM router stats if available
    if (this.llmRouter) {
      stats.llmRouter = this.llmRouter.getStatus();
    }

    return stats;
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

  /**
   * Set learning service at runtime
   *
   * @param {Object} learningService - LearningService instance
   */
  setLearningService(learningService) {
    this.learningService = learningService;
    if (this.kabbalisticRouter) {
      this.kabbalisticRouter.setLearningService(learningService);
    }
  }

  /**
   * Set cost optimizer at runtime
   *
   * @param {Object} costOptimizer - CostOptimizer instance
   */
  setCostOptimizer(costOptimizer) {
    this.costOptimizer = costOptimizer;
    if (this.kabbalisticRouter) {
      this.kabbalisticRouter.setCostOptimizer(costOptimizer);
    }
  }

  /**
   * Get learned weights from learning service (via kabbalistic router)
   *
   * @returns {Object|null} Learned weights or null
   */
  getLearnedWeights() {
    return this.kabbalisticRouter?.getLearnedWeights() || null;
  }

  /**
   * Apply learned weights to relationship graph
   *
   * @returns {boolean} True if weights were applied
   */
  applyLearnedWeights() {
    return this.kabbalisticRouter?.applyLearnedWeights() || false;
  }

  /**
   * Set planning gate at runtime
   *
   * @param {Object} planningGate - PlanningGate instance
   */
  setPlanningGate(planningGate) {
    this.planningGate = planningGate;
    log.debug('Planning gate set');
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

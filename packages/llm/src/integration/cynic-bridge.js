/**
 * CYNIC Bridge - Integration Layer
 * 
 * Connecte @cynic/llm a l'ecosysteme existant:
 * - IntelligentSwitch → Judge (LLM pour raisonnement)
 * - PageIndex → Judge (retrieval contextuel)
 * - LearningEngine → Judge (RLHF feedback)
 * - Prometheus → Agent (planning)
 * 
 * "Le chien ne confiance pas seulement a ses yeux - il utilise son nez, ses oreilles, et son intelligence"
 *
 * @module @cynic/llm/integration/cynic-bridge
 */

'use strict';

import { createLogger, PHI_INV, globalEventBus, EventType } from '@cynic/core';

// Import our LLM components
import { IntelligentSwitch } from '../adapters/intelligent-switch.js';
import { LearningEngine } from '../learning/index.js';
import { PageIndex } from '../retrieval/page-index.js';
import { Prometheus, Atlas } from '../orchestration/index.js';
import { LearningPersistence, createLearningPersistence } from '../persistence-integration.js';

// Lazy imports to avoid circular dependencies
let CYNICJudge = null;
let SelfSkeptic = null;
let ResidualDetector = null;

async function loadJudgeModules() {
  if (CYNICJudge) return { CYNICJudge, SelfSkeptic, ResidualDetector };
  try {
    const judge = await import('@cynic/node/judge');
    CYNICJudge = judge.CYNICJudge;
    SelfSkeptic = judge.SelfSkeptic;
    ResidualDetector = judge.ResidualDetector;
    return { CYNICJudge, SelfSkeptic, ResidualDetector };
  } catch (e) {
    return null;
  }
}

const log = createLogger('CYNICBridge');

/**
 * CYNIC Bridge - Unifie la couche LLM avec le Judge existant
 * 
 * Cette classe connecte:
 * 1. Notre IntelligentSwitch (selection LLM) -> au Judge pour les tasks de raisonnement
 * 2. PageIndex (RAG) -> au Judge pour le contexte
 * 3. LearningEngine -> au Judge pour les feedback loops RLHF
 * 4. Prometheus -> a l'agent pour le planning
 */
export class CYNICBridge {
  constructor(options = {}) {
    this.config = {
      // LLM options
      enableLLMSelection: options.enableLLMSelection !== false,
      enableContextRetrieval: options.enableContextRetrieval !== false,
      enableLearning: options.enableLearning !== false,
      enablePlanning: options.enablePlanning !== false,
      ...options,
    };

    // Core components
    this.intelligentSwitch = null;
    this.pageIndex = null;
    this.learningEngine = null;
    this.learningPersistence = null;
    this.prometheus = null;
    this.atlas = null;

    // Judge connection
    this.judge = null;
    this.judgeInitialized = false;

    // Agent connection
    this.agent = null;

    // Metrics
    this.stats = {
      judgmentsWithLLM: 0,
      judgmentsWithContext: 0,
      learningFeedbacks: 0,
      plansExecuted: 0,
    };
  }

  /**
   * Initialize the bridge and all components
   */
  async initialize(options = {}) {
    log.info('Initializing CYNIC Bridge...');

    // Initialize Learning Persistence first (dependencies)
    if (this.config.enableLearning) {
      try {
        this.learningPersistence = await createLearningPersistence();
        log.info('LearningPersistence initialized');
      } catch (e) {
        log.warn('LearningPersistence init failed', { error: e.message });
      }
    }

    // Initialize IntelligentSwitch for LLM selection
    if (this.config.enableLLMSelection) {
      this.intelligentSwitch = new IntelligentSwitch({
        ...options.intelligentSwitch,
        learningEnabled: this.config.enableLearning,
        persistence: this.learningPersistence,
      });
      log.info('IntelligentSwitch initialized');
    }

    // Initialize PageIndex for retrieval
    if (this.config.enableContextRetrieval) {
      this.pageIndex = new PageIndex({
        ...options.pageIndex,
        useVectorSearch: options.useVectorSearch !== false,
      });
      
      // If documents provided, build index
      if (options.documents?.length > 0) {
        await this.pageIndex.buildFromDocuments(options.documents);
      }
      log.info('PageIndex initialized');
    }

    // Initialize Prometheus + Atlas for planning
    if (this.config.enablePlanning) {
      this.prometheus = new Prometheus(options.prometheus);
      this.atlas = new Atlas(options.atlas);
      log.info('Prometheus + Atlas initialized');
    }

    // Initialize Learning Engine
    if (this.config.enableLearning && this.learningPersistence) {
      this.learningEngine = new LearningEngine({
        ...options.learningEngine,
        persistence: this.learningPersistence,
      });
      log.info('LearningEngine initialized');
    }

    // Initialize Judge connection
    await this._initializeJudge(options.judge);

    // Subscribe to events
    this._subscribeToEvents();

    log.info('CYNIC Bridge fully initialized');
    return this;
  }

  /**
   * Initialize connection to CYNICJudge
   * @private
   */
  async _initializeJudge(options = {}) {
    const modules = await loadJudgeModules();
    
    if (!modules) {
      log.warn('CYNICJudge not available - running in LLM-only mode');
      return;
    }

    const { CYNICJudge, SelfSkeptic, ResidualDetector } = modules;

    // Create Judge with our LLM integration
    this.judge = new CYNICJudge({
      // Wire our components into the Judge
      ...options,
      // Our LLM selection as the scorer
      scorer: options.scorer || this._createLLMScorer(),
      // Learning service for RLHF
      learningService: this.learningEngine || null,
      // Self-skepticism
      selfSkeptic: options.selfSkeptic || new SelfSkeptic(),
      // Residual detector for THE_UNNAMEABLE
      residualDetector: options.residualDetector || new ResidualDetector(),
    });

    // Register our LLM dimensions as plugins
    this._registerLLMDimensions();

    this.judgeInitialized = true;
    log.info('CYNICJudge connected to Bridge');
  }

  /**
   * Create LLM-aware scorer for Judge
   * @private
   */
  _createLLMScorer() {
    return async (dimension, item, context) => {
      // Use LLM for reasoning dimensions
      const reasoningDimensions = [
        'COHERENCE', 'ACCURACY', 'NOVELTY', 'UTILITY', 'VERIFIABILITY',
        'AUTHENTICITY', 'TIMING', 'MOMENTUM', 'SENTIMENT'
      ];

      if (!reasoningDimensions.includes(dimension)) {
        return null; // Let default scorer handle
      }

      // If IntelligentSwitch available, use it for scoring
      if (this.intelligentSwitch && context.query) {
        try {
          const result = await this.intelligentSwitch.select(context.query, {
            taskType: 'reasoning',
            dimension,
          });
          
          // Map LLM quality to score
          if (result.adapter) {
            return result.quality * 100;
          }
        } catch (e) {
          log.debug('LLM scorer failed for dimension', { dimension, error: e.message });
        }
      }

      return null; // Fallback to default
    };
  }

  /**
   * Register LLM-specific dimensions as plugins
   * @private
   */
  _registerLLMDimensions() {
    if (!this.judge?.registerPlugin) return;

    this.judge.registerPlugin({
      name: 'llm-bridge',
      version: '1.0.0',
      dimensions: [
        {
          axiom: 'PHI',
          name: 'LLM_REASONING_QUALITY',
          config: {
            weight: 1.2,
            threshold: 70,
            description: 'Quality of LLM reasoning for this judgment',
            scorer: async (item, context) => {
              if (!this.intelligentSwitch) return 50;
              try {
                const result = await this.intelligentSwitch.select(
                  item.content || item.name || '',
                  { taskType: 'reasoning' }
                );
                return result.quality * 100;
              } catch {
                return 50;
              }
            },
          },
        },
        {
          axiom: 'VERIFY',
          name: 'RETRIEVAL_RELEVANCE',
          config: {
            weight: 1.0,
            threshold: 60,
            description: 'Relevance of retrieved context',
            scorer: async (item, context) => {
              if (!this.pageIndex || !context.query) return 50;
              try {
                const retrieval = await this.pageIndex.retrieve(context.query, {
                  maxNodes: 3,
                });
                const avgRelevance = retrieval.context.reduce(
                  (sum, c) => sum + (c.relevance || 0), 0
                ) / retrieval.context.length;
                return avgRelevance * 100;
              } catch {
                return 50;
              }
            },
          },
        },
        {
          axiom: 'CULTURE',
          name: 'LEARNING_ADAPTATION',
          config: {
            weight: 1.1,
            threshold: 65,
            description: 'How well the system learned from similar past cases',
            scorer: async (item, context) => {
              if (!this.learningEngine) return 50;
              try {
                // Get adaptation score from learning engine
                const adaptationScore = this.learningEngine.getAdapterStats()?.adaptationScore || 0.5;
                return adaptationScore * 100;
              } catch {
                return 50;
              }
            },
          },
        },
        {
          axiom: 'BURN',
          name: 'PLAN_EFFICIENCY',
          config: {
            weight: 0.9,
            threshold: 55,
            description: 'Efficiency of the execution plan',
            scorer: async (item, context) => {
              if (!this.prometheus || !context.task) return 50;
              try {
                const plan = await this.prometheus.plan(context.task);
                // Score based on plan efficiency
                const stepCount = plan.steps?.length || 1;
                const efficiency = Math.max(0, 1 - (stepCount / 10)); // Fewer steps = higher score
                return efficiency * 100;
              } catch {
                return 50;
              }
            },
          },
        },
      ],
    });

    log.info('LLM dimensions registered with Judge');
  }

  /**
   * Subscribe to global events for reactivity
   * @private
   */
  _subscribeToEvents() {
    // Listen for judgment feedback to update learning
    globalEventBus.on(EventType.USER_FEEDBACK, async (event) => {
      if (this.config.enableLearning && this.learningEngine) {
        await this._handleFeedback(event);
      }
    });

    // Listen for new patterns to update retrieval
    globalEventBus.on(EventType.PATTERN_DETECTED, async (event) => {
      if (this.config.enableContextRetrieval && this.pageIndex) {
        await this._handlePatternDetected(event);
      }
    });

    log.debug('Event subscriptions active');
  }

  /**
   * Handle user feedback for learning
   * @private
   */
  async _handleFeedback(event) {
    if (!this.learningEngine) return;

    try {
      await this.learningEngine.record({
        type: 'judgment_feedback',
        data: {
          judgmentId: event.payload?.judgmentId,
          correct: event.payload?.correct,
          score: event.payload?.score,
          feedback: event.payload?.feedback,
        },
      });

      // Also save to persistence
      if (this.learningPersistence) {
        await this.learningPersistence.saveEvent({
          id: `fb_${Date.now()}`,
          adapter: 'bridge',
          data: event.payload,
          timestamp: Date.now(),
        });
      }

      this.stats.learningFeedbacks++;
      log.debug('Feedback processed', { correct: event.payload?.correct });
    } catch (e) {
      log.warn('Feedback handling failed', { error: e.message });
    }
  }

  /**
   * Handle new patterns for retrieval update
   * @private
   */
  async _handlePatternDetected(event) {
    if (!this.pageIndex) return;

    try {
      // Add new pattern to PageIndex for future retrieval
      await this.pageIndex.buildFromDocuments([{
        id: `pattern_${Date.now()}`,
        content: event.payload?.description || JSON.stringify(event.payload),
        metadata: {
          type: 'detected_pattern',
          source: event.source,
          timestamp: event.timestamp,
        },
      }]);

      log.debug('Pattern added to retrieval index');
    } catch (e) {
      log.warn('Pattern handling failed', { error: e.message });
    }
  }

  /**
   * Judge an item with full LLM integration
   * 
   * This is the main entry point - replaces direct Judge calls
   * when you want LLM-enhanced judgment
   *
   * @param {Object} item - Item to judge
   * @param {Object} context - Judgment context
   * @returns {Promise<Object>} Judgment result
   */
  async judge(item, context = {}) {
    const startTime = Date.now();

    // Step 1: Retrieve relevant context if enabled
    let enrichedContext = { ...context };
    
    if (this.config.enableContextRetrieval && this.pageIndex && context.query) {
      const retrieval = await this.pageIndex.retrieve(context.query, {
        maxNodes: 5,
        useVector: true,
      });

      enrichedContext.retrievedContext = retrieval.context;
      enrichedContext.retrievalMetadata = retrieval.metadata;
      this.stats.judgmentsWithContext++;
    }

    // Step 2: Use IntelligentSwitch for LLM reasoning if enabled
    if (this.config.enableLLMSelection && this.intelligentSwitch && context.query) {
      const llmSelection = await this.intelligentSwitch.select(context.query, {
        taskType: context.taskType || 'reasoning',
      });

      enrichedContext.llmSelection = {
        adapter: llmSelection.adapter,
        model: llmSelection.model,
        quality: llmSelection.quality,
        reasoning: llmSelection.reasoning,
      };

      this.stats.judgmentsWithLLM++;
    }

    // Step 3: Execute judgment with Judge
    let judgment;
    if (this.judge && this.judgeInitialized) {
      judgment = await this.judge.judge(item, enrichedContext);
    } else {
      // Fallback to basic judgment if Judge not available
      judgment = this._basicJudgment(item, enrichedContext);
    }

    // Step 4: Learn from this judgment if enabled
    if (this.config.enableLearning && this.learningEngine) {
      await this.learningEngine.record({
        type: 'judgment',
        data: {
          itemId: item.id,
          qScore: judgment.qScore,
          verdict: judgment.verdict,
        },
      });
    }

    // Add bridge metadata
    judgment.bridgeMetadata = {
      llmUsed: !!enrichedContext.llmSelection,
      contextRetrieved: !!enrichedContext.retrievedContext,
      latencyMs: Date.now() - startTime,
    };

    return judgment;
  }

  /**
   * Basic judgment fallback when Judge not available
   * @private
   */
  _basicJudgment(item, context) {
    const content = item.content || item.name || '';
    const wordCount = content.split(/\s+/).length;
    
    // Simple heuristic scoring
    const baseScore = Math.min(80, 40 + (wordCount / 10));
    const contextBonus = context.retrievedContext?.length ? 10 : 0;
    const llmBonus = context.llmSelection?.quality ? 15 : 0;
    
    const qScore = Math.min(95, baseScore + contextBonus + llmBonus);
    const verdict = qScore >= 80 ? 'HOWL' : qScore >= 60 ? 'WAG' : qScore >= 40 ? 'GROWL' : 'BARK';

    return {
      id: `bridge_${Date.now()}`,
      qScore,
      verdict,
      confidence: PHI_INV,
      dimensions: {},
      axiomScores: {},
      bridgeMetadata: {
        fallback: true,
        llmUsed: false,
        contextRetrieved: false,
      },
    };
  }

  /**
   * Plan a task using Prometheus
   * 
   * @param {Object} task - Task to plan
   * @param {Object} options - Planning options
   * @returns {Promise<Object>} Execution plan
   */
  async plan(task, options = {}) {
    if (!this.prometheus) {
      throw new Error('Prometheus not initialized');
    }

    // Get context if available
    let context = options.context || {};
    
    if (this.config.enableContextRetrieval && this.pageIndex && task.query) {
      const retrieval = await this.pageIndex.retrieve(task.query, { maxNodes: 3 });
      context.retrievedContext = retrieval.context;
    }

    // Create plan
    const plan = await this.prometheus.plan(task, context);

    return plan;
  }

  /**
   * Execute a plan using Atlas
   * 
   * @param {Object} plan - Plan to execute
   * @param {Object} options - Execution options
   * @returns {Promise<Object>} Execution result
   */
  async execute(plan, options = {}) {
    if (!this.atlas) {
      throw new Error('Atlas not initialized');
    }

    // Execute with LLM selection for each step
    const executeStep = async (step) => {
      if (this.config.enableLLMSelection && this.intelligentSwitch) {
        const selection = await this.intelligentSwitch.select(
          step.description || step.action,
          { taskType: 'execution' }
        );
        step.selectedAdapter = selection.adapter;
        step.selectedModel = selection.model;
      }
      return step;
    };

    // Apply LLM selection to plan steps
    if (plan.steps) {
      for (const step of plan.steps) {
        await executeStep(step);
      }
    }

    // Execute plan
    const result = await this.atlas.execute(plan, options);
    
    this.stats.plansExecuted++;

    // Learn from execution
    if (this.config.enableLearning && this.learningEngine) {
      await this.learningEngine.record({
        type: 'execution',
        data: {
          planId: plan.id,
          success: result.status === 'completed',
          duration: result.duration,
        },
      });
    }

    return result;
  }

  /**
   * Get comprehensive stats
   */
  getStats() {
    return {
      ...this.stats,
      judgeConnected: this.judgeInitialized,
      llmSelection: !!this.intelligentSwitch,
      contextRetrieval: !!this.pageIndex,
      learning: !!this.learningEngine,
      planning: !!this.prometheus,
    };
  }

  /**
   * Add documents to retrieval index
   */
  async addDocuments(documents) {
    if (!this.pageIndex) {
      throw new Error('PageIndex not initialized');
    }
    await this.pageIndex.buildFromDocuments(documents);
    log.info('Documents added to index', { count: documents.length });
  }

  /**
   * Search using retrieval
   */
  async search(query, options = {}) {
    if (!this.pageIndex) {
      return [];
    }
    return this.pageIndex.retrieve(query, options);
  }

  /**
   * Close all connections
   */
  async close() {
    if (this.learningPersistence) {
      await this.learningPersistence.close();
    }
    if (this.judge?.cleanup) {
      await this.judge.cleanup();
    }
    log.info('CYNIC Bridge closed');
  }
}

/**
 * Create a CYNIC Bridge instance
 */
export async function createCYNICBridge(options = {}) {
  const bridge = new CYNICBridge(options);
  await bridge.initialize(options);
  return bridge;
}

export default CYNICBridge;

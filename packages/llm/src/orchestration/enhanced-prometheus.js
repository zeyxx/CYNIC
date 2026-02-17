/**
 * CYNIC Enhanced Prometheus - With PageIndex Retrieval
 * 
 * Flow: Query → Prometheus → PageIndex (retrieve context) → Plan with context → Atlas
 *
 * @module @cynic/llm/orchestration/enhanced-prometheus
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { Prometheus as BasePrometheus, TaskType } from './prometheus.js';
import { PageIndex } from '../retrieval/page-index.js';

const log = createLogger('EnhancedPrometheus');

/**
 * Enhanced Prometheus with retrieval capabilities
 */
export class EnhancedPrometheus extends BasePrometheus {
  constructor(options = {}) {
    super(options);
    
    // PageIndex for context retrieval
    this.pageIndex = options.pageIndex || null;
    this.retrievalEnabled = options.retrievalEnabled !== false;
    this.maxRetrievalResults = options.maxRetrievalResults || 3;
    
    // Track retrieval stats
    this.retrievalStats = {
      queriesWithRetrieval: 0,
      avgRetrievalTime: 0,
      avgResultsRetrieved: 0,
    };
  }

  /**
   * Set PageIndex for retrieval
   */
  setPageIndex(index) {
    this.pageIndex = index;
    log.info('PageIndex set for retrieval');
  }

  /**
   * Analyze task and create plan with context
   */
  async analyze(task, context = {}) {
    log.info('Analyzing task with retrieval', { task: task.slice(0, 50) });
    
    // Step 1: Retrieve relevant context if PageIndex is available
    let retrievedContext = [];
    
    if (this.retrievalEnabled && this.pageIndex) {
      try {
        const startTime = Date.now();
        
        const retrievalResult = await this.pageIndex.retrieve(task, {
          maxNodes: this.maxRetrievalResults,
          llm: this.llm, // Use LLM for guided retrieval
        });
        
        retrievedContext = retrievalResult.context;
        
        // Update stats
        const retrievalTime = Date.now() - startTime;
        this.retrievalStats.queriesWithRetrieval++;
        this.retrievalStats.avgRetrievalTime = 
          (this.retrievalStats.avgRetrievalTime * (this.retrievalStats.queriesWithRetrieval - 1) + retrievalTime)
          / this.retrievalStats.queriesWithRetrieval;
        this.retrievalStats.avgResultsRetrieved = 
          (this.retrievalStats.avgResultsRetrieved * (this.retrievalStats.queriesWithRetrieval - 1) + retrievedContext.length)
          / this.retrievalStats.queriesWithRetrieval;
        
        log.info('Context retrieved', { 
          results: retrievedContext.length, 
          time: retrievalTime 
        });
        
      } catch (e) {
        log.warn('Retrieval failed', { error: e.message });
      }
    }
    
    // Step 2: Create plan with context
    const plan = await this._createPlanWithContext(task, retrievedContext, context);
    
    return plan;
  }

  /**
   * Create plan including retrieved context
   * @private
   */
  async _createPlanWithContext(task, retrievedContext, context) {
    const taskType = this._classifyTask(task);
    
    // Build context prompt from retrieved content
    let contextPrompt = '';
    if (retrievedContext.length > 0) {
      contextPrompt = '\n\n--- Relevant Context ---\n';
      for (const ctx of retrievedContext) {
        contextPrompt += `${ctx.content}\n\n`;
      }
      contextPrompt += '--- End Context ---\n\n';
    }
    
    // For simple tasks, create single-step plan with context
    if (taskType === TaskType.SINGLE || taskType === TaskType.CHAT) {
      const plan = new (await import('./prometheus.js')).ExecutionPlan({
        task,
        taskType,
        context: { ...context, retrievedContext },
        steps: [
          new (await import('./prometheus.js')).PlanStep({
            action: 'respond_with_context',
            tool: 'llm',
            input: { 
              prompt: task,
              context: contextPrompt,
              retrievedContext,
            },
          }),
        ],
      });
      
      this.stats.plansCreated++;
      return plan;
    }
    
    // For complex tasks, use LLM to break down
    if (this.llm && taskType === TaskType.MULTI) {
      return this._createMultiStepPlanWithContext(task, retrievedContext, context);
    }
    
    // Default
    const plan = new (await import('./prometheus.js')).ExecutionPlan({
      task,
      taskType,
      context: { ...context, retrievedContext },
      steps: [
        new (await import('./prometheus.js')).PlanStep({
          action: 'execute',
          tool: 'llm',
          input: { prompt: task, context: contextPrompt },
        }),
      ],
    });
    
    this.stats.plansCreated++;
    return plan;
  }

  /**
   * Create multi-step plan with context
   * @private
   */
  async _createMultiStepPlanWithContext(task, retrievedContext, context) {
    const prompt = `Analyze this task and break it down into steps:
    
Task: ${task}

Context from knowledge base:
${retrievedContext.map((c, i) => `${i + 1}. ${c.summary}`).join('\n')}

Available tools: ${this.tools.map(t => t.name).join(', ')}

Respond with a JSON array of steps.`;

    const response = await this.llm.complete(prompt);
    
    try {
      const stepsData = JSON.parse(response.content);
      const plan = new (await import('./prometheus.js')).ExecutionPlan({
        task,
        taskType: TaskType.MULTI,
        context: { ...context, retrievedContext },
      });
      
      for (const stepData of stepsData.slice(this.maxSteps)) {
        plan.addStep(new (await import('./prometheus.js')).PlanStep(stepData));
      }
      
      this.stats.plansCreated++;
      return plan;
    } catch (e) {
      log.error('Failed to parse plan', { error: e.message });
      return this._createPlanWithContext(task, [], context);
    }
  }

  /**
   * Get retrieval stats
   */
  getRetrievalStats() {
    return { ...this.retrievalStats };
  }
}

export default EnhancedPrometheus;

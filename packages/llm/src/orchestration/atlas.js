/**
 * CYNIC Atlas - Execution Layer
 *
 * Inspired by oh-my-opencode:
 * - Atlas: executes plans created by Prometheus
 * - Handles tool execution, error handling, retry logic
 *
 * Flow:
 * 1. Receives execution plan from Prometheus
 * 2. Executes steps in dependency order
 * 3. Handles errors with retry logic
 * 4. Returns results
 *
 * @module @cynic/llm/orchestration/atlas
 */

'use strict';

import { createLogger } from '@cynic/core';
import { ExecutionPlan, PlanStep } from './prometheus.js';

const log = createLogger('Atlas');

/**
 * Execution status
 */
export const ExecutionStatus = {
  PENDING: 'pending',
  RUNNING: 'running',
  COMPLETED: 'completed',
  FAILED: 'failed',
  CANCELLED: 'cancelled',
};

/**
 * Atlas - Execution Engine
 */
export class Atlas {
  constructor(options = {}) {
    this.name = options.name || 'atlas';
    this.tools = new Map(); // name -> tool
    this.maxConcurrent = options.maxConcurrent || 3;
    this.retryConfig = {
      maxRetries: options.maxRetries || 2,
      backoffMs: options.backoffMs || 1000,
    };
    
    // Execution state
    this.currentPlan = null;
    this.executionId = null;
    
    // Stats
    this.stats = {
      plansExecuted: 0,
      stepsCompleted: 0,
      stepsFailed: 0,
      totalDuration: 0,
    };
  }

  /**
   * Register a tool
   */
  registerTool(tool) {
    this.tools.set(tool.name, tool);
    log.info('Tool registered', { name: tool.name });
  }

  /**
   * Register multiple tools
   */
  registerTools(tools) {
    for (const tool of tools) {
      this.registerTool(tool);
    }
  }

  /**
   * Execute a plan
   */
  async execute(plan) {
    if (!(plan instanceof ExecutionPlan)) {
      throw new Error('Invalid plan - must be ExecutionPlan instance');
    }

    this.currentPlan = plan;
    this.executionId = `exec-${Date.now()}`;
    
    log.info('Starting execution', { planId: plan.id, steps: plan.steps.length });
    
    const startTime = Date.now();
    
    // Execute until complete or failed
    while (!plan.isComplete() && !plan.hasFailed()) {
      const readySteps = plan.getReadySteps();
      
      if (readySteps.length === 0) {
        // Deadlock - no steps can run
        log.error('Execution deadlock - no ready steps');
        break;
      }
      
      // Execute ready steps (up to maxConcurrent)
      const batch = readySteps.slice(0, this.maxConcurrent);
      await Promise.all(batch.map(step => this._executeStep(step, plan)));
    }
    
    const duration = Date.now() - startTime;
    this.stats.totalDuration += duration;
    this.stats.plansExecuted++;
    
    log.info('Execution complete', {
      planId: plan.id,
      success: plan.isComplete(),
      duration,
      completed: plan.steps.filter(s => s.status === 'completed').length,
      failed: plan.steps.filter(s => s.status === 'failed').length,
    });
    
    return {
      success: plan.isComplete(),
      plan,
      executionId: this.executionId,
      duration,
      results: plan.steps.map(s => ({
        id: s.id,
        action: s.action,
        status: s.status,
        result: s.result,
        error: s.error,
      })),
    };
  }

  /**
   * Execute a single step
   * @private
   */
  async _executeStep(step, plan) {
    if (step.status !== 'pending') return;
    
    step.status = ExecutionStatus.RUNNING;
    log.info('Executing step', { stepId: step.id, action: step.action });
    
    let lastError = null;
    
    // Retry loop
    for (let attempt = 0; attempt <= this.retryConfig.maxRetries; attempt++) {
      try {
        // Execute the step
        const result = await this._runTool(step.tool, step.input, step);
        
        step.status = ExecutionStatus.COMPLETED;
        step.result = result;
        this.stats.stepsCompleted++;
        
        log.info('Step completed', { stepId: step.id });
        return result;
        
      } catch (error) {
        lastError = error;
        log.warn('Step failed, retrying', {
          stepId: step.id,
          attempt: attempt + 1,
          error: error.message,
        });
        
        // Exponential backoff
        if (attempt < this.retryConfig.maxRetries) {
          await this._sleep(this.retryConfig.backoffMs * Math.pow(2, attempt));
        }
      }
    }
    
    // All retries exhausted
    step.status = ExecutionStatus.FAILED;
    step.error = lastError.message;
    this.stats.stepsFailed++;
    
    log.error('Step failed permanently', { stepId: step.id, error: lastError.message });
    
    return null;
  }

  /**
   * Run a tool
   * @private
   */
  async _runTool(toolName, input, step) {
    const tool = this.tools.get(toolName);
    
    if (!tool) {
      throw new Error(`Unknown tool: ${toolName}`);
    }
    
    // Execute tool
    if (typeof tool.execute === 'function') {
      return await tool.execute(input);
    } else if (typeof tool === 'function') {
      return await tool(input);
    } else {
      throw new Error(`Tool ${toolName} has no execute method`);
    }
  }

  /**
   * Sleep utility
   * @private
   */
  _sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * Cancel current execution
   */
  async cancel() {
    if (!this.currentPlan) {
      log.warn('No execution to cancel');
      return false;
    }
    
    log.info('Cancelling execution', { executionId: this.executionId });
    
    for (const step of this.currentPlan.steps) {
      if (step.status === ExecutionStatus.PENDING) {
        step.status = ExecutionStatus.CANCELLED;
      }
    }
    
    return true;
  }

  /**
   * Get execution status
   */
  getStatus() {
    if (!this.currentPlan) {
      return { status: 'idle' };
    }
    
    return {
      executionId: this.executionId,
      planId: this.currentPlan.id,
      status: this.currentPlan.isComplete() ? 'completed' : 
             this.currentPlan.hasFailed() ? 'failed' : 'running',
      steps: {
        pending: this.currentPlan.steps.filter(s => s.status === 'pending').length,
        running: this.currentPlan.steps.filter(s => s.status === 'running').length,
        completed: this.currentPlan.steps.filter(s => s.status === 'completed').length,
        failed: this.currentPlan.steps.filter(s => s.status === 'failed').length,
      },
    };
  }

  /**
   * Get stats
   */
  getStats() {
    return {
      ...this.stats,
      avgDuration: this.stats.plansExecuted > 0 
        ? this.stats.totalDuration / this.stats.plansExecuted 
        : 0,
    };
  }
}

/**
 * Create Atlas instance
 */
export function createAtlas(options) {
  return new Atlas(options);
}

export default Atlas;

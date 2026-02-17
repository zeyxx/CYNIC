/**
 * CYNIC Prometheus - Planning Layer
 *
 * Inspired by oh-my-opencode:
 * - Prometheus: analyzes task, creates execution plan
 * - Atlas: executes plan with tools
 *
 * Flow:
 * 1. Prometheus receives task
 * 2. Analyzes requirements, available tools, constraints
 * 3. Creates execution plan (ordered steps)
 * 4. Passes to Atlas for execution
 *
 * @module @cynic/llm/orchestration/prometheus
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('Prometheus');

/**
 * Task types
 */
export const TaskType = {
  /** Simple one-shot request */
  SINGLE: 'single',
  /** Multi-step task requiring planning */
  MULTI: 'multi',
  /** Interactive conversation */
  CHAT: 'chat',
  /** Code/analysis task */
  CODE: 'code',
  /** Research task */
  RESEARCH: 'research',
};

/**
 * Plan step
 */
export class PlanStep {
  constructor(options = {}) {
    this.id = options.id || `step-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    this.action = options.action || '';
    this.tool = options.tool || null;
    this.input = options.input || {};
    this.dependencies = options.dependencies || []; // Step IDs this depends on
    this.expectedOutput = options.expectedOutput || null;
    this.status = 'pending';
    this.result = null;
    this.error = null;
  }
}

/**
 * Execution plan
 */
export class ExecutionPlan {
  constructor(options = {}) {
    this.id = options.id || `plan-${Date.now()}`;
    this.task = options.task || '';
    this.taskType = options.taskType || TaskType.SINGLE;
    this.steps = options.steps || [];
    this.context = options.context || {};
    this.metadata = {
      createdAt: Date.now(),
      estimatedDuration: 0,
      complexity: 0,
    };
  }

  addStep(step) {
    this.steps.push(step);
    return this;
  }

  getStep(stepId) {
    return this.steps.find(s => s.id === stepId);
  }

  getReadySteps() {
    return this.steps.filter(step => {
      if (step.status !== 'pending') return false;
      // Check if all dependencies are completed
      return step.dependencies.every(depId => {
        const dep = this.getStep(depId);
        return dep && dep.status === 'completed';
      });
    });
  }

  isComplete() {
    return this.steps.every(s => s.status === 'completed');
  }

  hasFailed() {
    return this.steps.some(s => s.status === 'failed');
  }
}

/**
 * Prometheus - Planning Engine
 */
export class Prometheus {
  constructor(options = {}) {
    this.name = options.name || 'prometheus';
    this.llm = options.llm || null;
    this.tools = options.tools || [];
    this.maxSteps = options.maxSteps || 10;
    
    // Planning config
    this.config = {
      thinkStepByStep: options.thinkStepByStep !== false,
      includeReasoning: options.includeReasoning !== false,
      maxRetries: options.maxRetries || 2,
    };
    
    // Stats
    this.stats = {
      plansCreated: 0,
      tasksCompleted: 0,
      tasksFailed: 0,
    };
  }

  /**
   * Register tools available for execution
   */
  registerTools(tools) {
    this.tools = tools;
    log.info('Tools registered', { count: tools.length });
  }

  /**
   * Analyze task and create execution plan
   */
  async analyze(task, context = {}) {
    log.info('Analyzing task', { task: task.slice(0, 100) });
    
    // Determine task type
    const taskType = this._classifyTask(task);
    
    // If simple task, create single-step plan
    if (taskType === TaskType.SINGLE || taskType === TaskType.CHAT) {
      const plan = new ExecutionPlan({
        task,
        taskType,
        steps: [
          new PlanStep({
            action: 'respond',
            tool: 'llm',
            input: { prompt: task, context },
          }),
        ],
      });
      this.stats.plansCreated++;
      return plan;
    }
    
    // For complex tasks, use LLM to break down
    if (this.llm && taskType === TaskType.MULTI) {
      return this._createMultiStepPlan(task, context);
    }
    
    // Default: single step
    const plan = new ExecutionPlan({
      task,
      taskType,
      steps: [
        new PlanStep({
          action: 'execute',
          tool: 'llm',
          input: { prompt: task },
        }),
      ],
    });
    
    this.stats.plansCreated++;
    return plan;
  }

  /**
   * Create multi-step plan using LLM
   * @private
   */
  async _createMultiStepPlan(task, context) {
    const prompt = `Analyze this task and break it down into steps:
    
Task: ${task}

Available tools: ${this.tools.map(t => t.name).join(', ')}

Respond with a JSON array of steps, each with:
- action: what to do
- tool: which tool to use
- input: what to pass to the tool
- dependencies: array of step IDs this depends on

Example:
[
  {"action": "search docs", "tool": "search", "input": {"query": "..."}, "dependencies": []},
  {"action": "analyze results", "tool": "llm", "input": {"prompt": "..."}, "dependencies": ["step1"]}
]

Respond ONLY with valid JSON, no other text.`;

    const response = await this.llm.complete(prompt);
    
    try {
      const stepsData = JSON.parse(response.content);
      const plan = new ExecutionPlan({
        task,
        taskType: TaskType.MULTI,
        context,
      });
      
      for (const stepData of stepsData.slice(0, this.maxSteps)) {
        plan.addStep(new PlanStep(stepData));
      }
      
      this.stats.plansCreated++;
      return plan;
    } catch (e) {
      log.error('Failed to parse plan', { error: e.message });
      // Fallback to single step
      return new ExecutionPlan({
        task,
        taskType: TaskType.SINGLE,
        steps: [new PlanStep({ action: 'execute', tool: 'llm', input: { prompt: task } })],
      });
    }
  }

  /**
   * Classify task type
   * @private
   */
  _classifyTask(task) {
    const taskLower = task.toLowerCase();
    
    // Code/analysis keywords
    if (taskLower.includes('analyze') || taskLower.includes('debug') || 
        taskLower.includes('review') || taskLower.includes('explain')) {
      return TaskType.CODE;
    }
    
    // Research keywords
    if (taskLower.includes('research') || taskLower.includes('find') || 
        taskLower.includes('search') || taskLower.includes('compare')) {
      return TaskType.RESEARCH;
    }
    
    // Multi-step indicators
    if (taskLower.includes('then') || taskLower.includes(' and then') ||
        taskLower.includes('first') || taskLower.includes('next') ||
        taskLower.includes('steps') || taskLower.includes('process')) {
      return TaskType.MULTI;
    }
    
    // Chat indicators
    if (taskLower.includes('?') && taskLower.length < 200) {
      return TaskType.CHAT;
    }
    
    // Default
    return TaskType.SINGLE;
  }

  /**
   * Estimate plan complexity
   */
  estimateComplexity(plan) {
    const factors = {
      stepCount: plan.steps.length,
      hasDependencies: plan.steps.some(s => s.dependencies.length > 0),
      hasLoops: false, // Could detect from dependency cycles
    };
    
    // Simple complexity score
    let score = factors.stepCount * 10;
    if (factors.hasDependencies) score += 20;
    
    return {
      score,
      level: score < 20 ? 'simple' : score < 50 ? 'moderate' : 'complex',
      factors,
    };
  }

  /**
   * Get stats
   */
  getStats() {
    return { ...this.stats };
  }
}

/**
 * Create Prometheus instance
 */
export function createPrometheus(options) {
  return new Prometheus(options);
}

export default Prometheus;

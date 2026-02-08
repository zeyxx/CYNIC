/**
 * Dog Pipeline - Stream Chaining Between Dogs
 *
 * Enables sequential processing where one dog's output flows
 * to the next dog with full context preservation.
 *
 * Like Unix pipes: Scout | Analyst | Architect
 *
 * "La meute chasse ensemble" - κυνικός
 *
 * @module @cynic/node/routing/dog-pipeline
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { DogId, DOG_CAPABILITIES } from './dog-capabilities.js';
import { createTaskDescriptor } from './task-descriptor.js';

/**
 * Stream context that flows between dogs
 * Accumulates results and preserves full history
 */
export class StreamContext {
  /**
   * @param {Object} options
   * @param {string} options.pipelineId - Pipeline identifier
   * @param {string} options.originalInput - Original task input
   * @param {Object} [options.metadata] - Additional metadata
   */
  constructor(options) {
    this.pipelineId = options.pipelineId;
    this.originalInput = options.originalInput;
    this.metadata = options.metadata || {};

    // Current state
    this.currentInput = options.originalInput;
    this.currentStage = 0;

    // Accumulated history
    this.stages = [];
    this.outputs = [];
    this.errors = [];

    // Timing
    this.startedAt = Date.now();
    this.completedAt = null;

    // Control flags
    this.aborted = false;
    this.abortReason = null;
  }

  /**
   * Record a stage completion
   *
   * @param {Object} result
   * @param {string} result.dogId - Dog that processed this stage
   * @param {*} result.output - Stage output
   * @param {number} result.latency - Processing time in ms
   * @param {boolean} [result.success=true] - Whether stage succeeded
   * @param {string} [result.error] - Error message if failed
   */
  recordStage(result) {
    const stage = {
      index: this.currentStage,
      dogId: result.dogId,
      dogName: DOG_CAPABILITIES[result.dogId]?.name,
      input: this.currentInput,
      output: result.output,
      latency: result.latency,
      success: result.success !== false,
      error: result.error || null,
      timestamp: Date.now(),
    };

    this.stages.push(stage);
    this.outputs.push(result.output);

    if (!stage.success) {
      this.errors.push({
        stage: this.currentStage,
        dogId: result.dogId,
        error: result.error,
      });
    }

    // Update current input for next stage
    this.currentInput = result.output;
    this.currentStage++;

    return stage;
  }

  /**
   * Abort the pipeline
   * @param {string} reason
   */
  abort(reason) {
    this.aborted = true;
    this.abortReason = reason;
    this.completedAt = Date.now();
  }

  /**
   * Mark pipeline as complete
   */
  complete() {
    this.completedAt = Date.now();
  }

  /**
   * Get final output (last stage's output)
   * @returns {*}
   */
  getFinalOutput() {
    if (this.outputs.length === 0) return null;
    return this.outputs[this.outputs.length - 1];
  }

  /**
   * Get total latency
   * @returns {number} ms
   */
  getTotalLatency() {
    return this.stages.reduce((sum, s) => sum + (s.latency || 0), 0);
  }

  /**
   * Check if all stages succeeded
   * @returns {boolean}
   */
  isSuccessful() {
    return !this.aborted && this.errors.length === 0;
  }

  /**
   * Get stage by index
   * @param {number} index
   * @returns {Object|null}
   */
  getStage(index) {
    return this.stages[index] || null;
  }

  /**
   * Get output from specific dog
   * @param {string} dogId
   * @returns {*}
   */
  getOutputFrom(dogId) {
    const stage = this.stages.find(s => s.dogId === dogId);
    return stage?.output || null;
  }

  /**
   * Serialize for logging/storage
   * @returns {Object}
   */
  toJSON() {
    return {
      pipelineId: this.pipelineId,
      originalInput: typeof this.originalInput === 'string'
        ? this.originalInput.slice(0, 200)
        : this.originalInput,
      stageCount: this.stages.length,
      stages: this.stages.map(s => ({
        dogId: s.dogId,
        dogName: s.dogName,
        success: s.success,
        latency: s.latency,
      })),
      totalLatency: this.getTotalLatency(),
      successful: this.isSuccessful(),
      aborted: this.aborted,
      abortReason: this.abortReason,
      errorCount: this.errors.length,
      startedAt: this.startedAt,
      completedAt: this.completedAt,
    };
  }
}

/**
 * Pipeline stage definition
 */
export class PipelineStage {
  /**
   * @param {Object} options
   * @param {string} options.dogId - Dog to execute this stage
   * @param {Function} [options.handler] - Custom handler (optional)
   * @param {Function} [options.transform] - Transform input before processing
   * @param {Function} [options.validate] - Validate output before continuing
   * @param {boolean} [options.optional=false] - Skip if handler unavailable
   * @param {number} [options.timeout] - Stage timeout in ms
   */
  constructor(options) {
    this.dogId = options.dogId;
    this.handler = options.handler || null;
    this.transform = options.transform || null;
    this.validate = options.validate || null;
    this.optional = options.optional || false;
    this.timeout = options.timeout || 30000;
  }

  /**
   * Transform input for this stage
   * @param {StreamContext} context
   * @returns {*} Transformed input
   */
  transformInput(context) {
    if (this.transform) {
      return this.transform(context.currentInput, context);
    }
    return context.currentInput;
  }

  /**
   * Validate output from this stage
   * @param {*} output
   * @param {StreamContext} context
   * @returns {{valid: boolean, reason?: string}}
   */
  validateOutput(output, context) {
    if (this.validate) {
      return this.validate(output, context);
    }
    return { valid: true };
  }
}

/**
 * Predefined pipeline templates
 */
export const PipelineTemplates = Object.freeze({
  // Exploration → Analysis → Architecture
  EXPLORE_ANALYZE_BUILD: [
    { dogId: DogId.SCOUT, description: 'Explore and gather information' },
    { dogId: DogId.ANALYST, description: 'Analyze findings' },
    { dogId: DogId.ARCHITECT, description: 'Design solution' },
  ],

  // Research → Document
  RESEARCH_DOCUMENT: [
    { dogId: DogId.SCHOLAR, description: 'Research topic' },
    { dogId: DogId.CARTOGRAPHER, description: 'Map and document' },
  ],

  // Security audit → Fix → Verify
  SECURITY_AUDIT: [
    { dogId: DogId.GUARDIAN, description: 'Security audit' },
    { dogId: DogId.ARCHITECT, description: 'Design fix' },
    { dogId: DogId.GUARDIAN, description: 'Verify fix' },
  ],

  // Cleanup → Verify → Deploy
  CLEANUP_DEPLOY: [
    { dogId: DogId.JANITOR, description: 'Clean up code' },
    { dogId: DogId.GUARDIAN, description: 'Verify changes' },
    { dogId: DogId.DEPLOYER, description: 'Deploy' },
  ],

  // Explore → Analyze → Sage wisdom
  DEEP_ANALYSIS: [
    { dogId: DogId.SCOUT, description: 'Explore codebase' },
    { dogId: DogId.ANALYST, description: 'Deep analysis' },
    { dogId: DogId.SAGE, description: 'Strategic insight' },
  ],

  // Oracle synthesis (for complex decisions)
  SYNTHESIS: [
    { dogId: DogId.SCOUT, description: 'Gather context' },
    { dogId: DogId.ANALYST, description: 'Analyze options' },
    { dogId: DogId.ORACLE, description: 'Synthesize decision' },
  ],
});

/**
 * Dog Pipeline - Orchestrates sequential dog processing
 */
export class DogPipeline extends EventEmitter {
  /**
   * @param {Object} [options]
   * @param {Map<string, Function>} [options.handlers] - Dog handlers
   * @param {boolean} [options.continueOnError=false] - Continue after stage failure
   * @param {number} [options.maxStages=10] - Maximum stages in a pipeline
   */
  constructor(options = {}) {
    super();

    this.handlers = options.handlers || new Map();
    this.continueOnError = options.continueOnError || false;
    this.maxStages = options.maxStages || 10;

    // Statistics
    this.stats = {
      pipelinesRun: 0,
      stagesExecuted: 0,
      successfulPipelines: 0,
      failedPipelines: 0,
      abortedPipelines: 0,
      totalLatency: 0,
      byTemplate: {},
    };

    // Active pipelines
    this._activePipelines = new Map();
  }

  /**
   * Register a handler for a dog
   *
   * @param {string} dogId
   * @param {Function} handler - async (input, context, stage) => output
   */
  registerHandler(dogId, handler) {
    this.handlers.set(dogId, handler);
    this.emit('handler:registered', { dogId });
  }

  /**
   * Create a pipeline from template
   *
   * @param {string} templateName - Template name from PipelineTemplates
   * @returns {Array<PipelineStage>}
   */
  fromTemplate(templateName) {
    const template = PipelineTemplates[templateName];
    if (!template) {
      throw new Error(`Unknown template: ${templateName}`);
    }
    return template.map(t => new PipelineStage({
      dogId: t.dogId,
      optional: t.optional || false,
    }));
  }

  /**
   * Create a pipeline from dog IDs
   *
   * @param {...string} dogIds - Dog IDs in order
   * @returns {Array<PipelineStage>}
   */
  chain(...dogIds) {
    return dogIds.map(dogId => new PipelineStage({ dogId }));
  }

  /**
   * Execute a pipeline
   *
   * @param {Array<PipelineStage>} stages - Pipeline stages
   * @param {string} input - Initial input
   * @param {Object} [options] - Execution options
   * @param {Object} [options.metadata] - Additional metadata
   * @param {string} [options.templateName] - Template name for stats
   * @returns {Promise<StreamContext>}
   */
  async execute(stages, input, options = {}) {
    if (stages.length > this.maxStages) {
      throw new Error(`Pipeline exceeds maximum stages (${this.maxStages})`);
    }

    const pipelineId = `pipe_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
    const context = new StreamContext({
      pipelineId,
      originalInput: input,
      metadata: options.metadata,
    });

    this._activePipelines.set(pipelineId, context);
    this.stats.pipelinesRun++;

    this.emit('pipeline:start', {
      pipelineId,
      stageCount: stages.length,
      dogs: stages.map(s => s.dogId),
    });

    try {
      for (let i = 0; i < stages.length; i++) {
        const stage = stages[i];

        // Check for abort
        if (context.aborted) {
          break;
        }

        // Get handler
        const handler = stage.handler || this.handlers.get(stage.dogId);
        if (!handler) {
          if (stage.optional) {
            this.emit('stage:skipped', { pipelineId, stage: i, dogId: stage.dogId });
            continue;
          }
          context.abort(`No handler for dog: ${stage.dogId}`);
          break;
        }

        // Transform input
        const stageInput = stage.transformInput(context);

        this.emit('stage:start', {
          pipelineId,
          stage: i,
          dogId: stage.dogId,
          dogName: DOG_CAPABILITIES[stage.dogId]?.name,
        });

        const startTime = performance.now();
        let output;
        let success = true;
        let error = null;

        try {
          // Execute with timeout (clear timer on resolve to prevent process hang)
          let timeoutId;
          output = await Promise.race([
            handler(stageInput, context, stage).then(r => { clearTimeout(timeoutId); return r; }),
            new Promise((_, reject) => {
              timeoutId = setTimeout(() => reject(new Error('Stage timeout')), stage.timeout);
              timeoutId.unref?.(); // Don't prevent process exit
            }),
          ]);

          // Validate output
          const validation = stage.validateOutput(output, context);
          if (!validation.valid) {
            throw new Error(validation.reason || 'Output validation failed');
          }
        } catch (err) {
          success = false;
          error = err.message;

          this.emit('stage:error', {
            pipelineId,
            stage: i,
            dogId: stage.dogId,
            error: err.message,
          });

          if (!this.continueOnError) {
            context.abort(`Stage ${i} (${stage.dogId}) failed: ${err.message}`);
          }
        }

        const latency = performance.now() - startTime;

        // Record stage result
        context.recordStage({
          dogId: stage.dogId,
          output,
          latency,
          success,
          error,
        });

        this.stats.stagesExecuted++;

        this.emit('stage:complete', {
          pipelineId,
          stage: i,
          dogId: stage.dogId,
          success,
          latency,
        });

        // Stop if aborted (from validation or continue check)
        if (context.aborted) {
          break;
        }
      }
    } finally {
      context.complete();
      this._activePipelines.delete(pipelineId);

      // Update stats
      if (context.isSuccessful()) {
        this.stats.successfulPipelines++;
      } else if (context.aborted) {
        this.stats.abortedPipelines++;
      } else {
        this.stats.failedPipelines++;
      }
      this.stats.totalLatency += context.getTotalLatency();

      // Track by template
      if (options.templateName) {
        if (!this.stats.byTemplate[options.templateName]) {
          this.stats.byTemplate[options.templateName] = { run: 0, success: 0 };
        }
        this.stats.byTemplate[options.templateName].run++;
        if (context.isSuccessful()) {
          this.stats.byTemplate[options.templateName].success++;
        }
      }

      this.emit('pipeline:complete', {
        pipelineId,
        successful: context.isSuccessful(),
        stagesCompleted: context.stages.length,
        totalLatency: context.getTotalLatency(),
      });
    }

    return context;
  }

  /**
   * Execute a template pipeline
   *
   * @param {string} templateName - Template name
   * @param {string} input - Initial input
   * @param {Object} [options] - Options
   * @returns {Promise<StreamContext>}
   */
  async executeTemplate(templateName, input, options = {}) {
    const stages = this.fromTemplate(templateName);
    return this.execute(stages, input, { ...options, templateName });
  }

  /**
   * Execute a simple chain of dogs
   *
   * @param {string} input - Initial input
   * @param {...string} dogIds - Dog IDs in order
   * @returns {Promise<StreamContext>}
   */
  async executeChain(input, ...dogIds) {
    const stages = this.chain(...dogIds);
    return this.execute(stages, input);
  }

  /**
   * Abort an active pipeline
   *
   * @param {string} pipelineId
   * @param {string} [reason='Manual abort']
   * @returns {boolean} Whether pipeline was found and aborted
   */
  abort(pipelineId, reason = 'Manual abort') {
    const context = this._activePipelines.get(pipelineId);
    if (context) {
      context.abort(reason);
      this.emit('pipeline:aborted', { pipelineId, reason });
      return true;
    }
    return false;
  }

  /**
   * Get active pipelines
   * @returns {Array<{pipelineId: string, context: Object}>}
   */
  getActivePipelines() {
    const active = [];
    for (const [pipelineId, context] of this._activePipelines) {
      active.push({
        pipelineId,
        currentStage: context.currentStage,
        startedAt: context.startedAt,
      });
    }
    return active;
  }

  /**
   * Get statistics
   * @returns {Object}
   */
  getStats() {
    const successRate = this.stats.pipelinesRun > 0
      ? this.stats.successfulPipelines / this.stats.pipelinesRun
      : 0;

    return {
      ...this.stats,
      successRate: Math.round(successRate * 1000) / 1000,
      avgLatency: this.stats.pipelinesRun > 0
        ? Math.round(this.stats.totalLatency / this.stats.pipelinesRun)
        : 0,
      avgStagesPerPipeline: this.stats.pipelinesRun > 0
        ? Math.round((this.stats.stagesExecuted / this.stats.pipelinesRun) * 10) / 10
        : 0,
      activePipelines: this._activePipelines.size,
      handlersRegistered: this.handlers.size,
    };
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      pipelinesRun: 0,
      stagesExecuted: 0,
      successfulPipelines: 0,
      failedPipelines: 0,
      abortedPipelines: 0,
      totalLatency: 0,
      byTemplate: {},
    };
  }

  /**
   * Format pipeline visualization
   *
   * @param {Array<PipelineStage>} stages
   * @returns {string}
   */
  static formatPipeline(stages) {
    const parts = stages.map(s => {
      const cap = DOG_CAPABILITIES[s.dogId];
      return `${cap?.emoji || '🐕'} ${cap?.name || s.dogId}`;
    });
    return parts.join(' → ');
  }

  /**
   * Format completed context visualization
   *
   * @param {StreamContext} context
   * @returns {string}
   */
  static formatResult(context) {
    const lines = [];
    lines.push(`Pipeline: ${context.pipelineId}`);
    lines.push(`Status: ${context.isSuccessful() ? '✅ Success' : context.aborted ? '⛔ Aborted' : '❌ Failed'}`);
    lines.push(`Stages: ${context.stages.length}`);
    lines.push('');

    for (const stage of context.stages) {
      const cap = DOG_CAPABILITIES[stage.dogId];
      const status = stage.success ? '✅' : '❌';
      lines.push(`  ${status} ${cap?.emoji || '🐕'} ${stage.dogName}: ${stage.latency.toFixed(0)}ms`);
      if (stage.error) {
        lines.push(`      └─ ${stage.error}`);
      }
    }

    lines.push('');
    lines.push(`Total: ${context.getTotalLatency().toFixed(0)}ms`);

    return lines.join('\n');
  }
}

/**
 * Create a dog pipeline
 * @param {Object} [options]
 * @returns {DogPipeline}
 */
export function createDogPipeline(options = {}) {
  return new DogPipeline(options);
}

// Singleton
let _instance = null;

/**
 * Get singleton pipeline
 * @returns {DogPipeline}
 */
export function getDogPipeline() {
  if (!_instance) {
    _instance = createDogPipeline();
  }
  return _instance;
}

export default DogPipeline;

/**
 * CYNIC Learning-Enabled Intelligent Switch
 * 
 * IntelligentSwitch with Learning integration:
 * - Uses LearningEngine stats for adapter selection
 * - Thompson Sampling for exploration/exploitation
 * - Real-time cost tracking
 *
 * @module @cynic/llm/adapters/learning-switch
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { IntelligentSwitch } from './intelligent-switch.js';
import { getOracle, calculateCost } from '../pricing/oracle.js';

const log = createLogger('LearningSwitch');

/**
 * Learning-Enabled Intelligent Switch
 * 
 * Extends IntelligentSwitch with Learning capabilities:
 * - Records every selection to LearningEngine
 * - Uses Learning stats for better selection
 * - Thompson Sampling for exploration
 */
export class LearningSwitch extends IntelligentSwitch {
  constructor(options = {}) {
    super(options);
    
    // Learning integration
    this.learningEngine = options.learningEngine || null;
    this.useLearning = options.useLearning !== false;
    
    // Thompson Sampling config
    this.explorationRate = options.explorationRate || 0.1;
  }

  /**
   * Set learning engine
   */
  setLearningEngine(engine) {
    this.learningEngine = engine;
    log.info('Learning engine set');
  }

  /**
   * Complete with learning integration
   */
  async complete(prompt, options = {}) {
    this.stats.selections++;
    
    // Get candidates
    const candidates = await this._detectAndSelect(prompt, options);
    
    if (!candidates || candidates.length === 0) {
      throw new Error('No adapters available');
    }

    // Try candidates in order
    let lastError = null;
    let selectedAdapter = null;
    let selectedCandidate = null;
    
    for (let i = 0; i < candidates.length; i++) {
      const candidate = candidates[i];
      
      if (options.stream && !candidate.streaming) {
        continue;
      }

      try {
        // Learning-guided selection
        if (this.useLearning && this.learningEngine) {
          const adapter = await this._learnedSelection(candidates, options);
          selectedAdapter = adapter;
          selectedCandidate = candidates.find(c => c.adapter === adapter);
          
          if (selectedCandidate) {
            log.info('Learning-guided selection', { adapter });
          }
        } else {
          // Use default ranking
          selectedCandidate = candidate;
          selectedAdapter = candidate.adapter;
        }

        log.info('Selected adapter', {
          adapter: selectedAdapter,
          strategy: this.strategy,
        });

        // Track stats
        if (!this.stats.byAdapter[selectedAdapter]) {
          this.stats.byAdapter[selectedAdapter] = 0;
        }
        this.stats.byAdapter[selectedAdapter]++;

        // Execute
        const response = await selectedCandidate.adapterRef.complete(prompt, {
          ...options,
          model: options.model || undefined,
        });

        // Record to learning
        if (this.learningEngine) {
          this._recordLearningEvent(selectedAdapter, response, options);
        }

        // Add metadata
        response.metadata.selectedAdapter = selectedAdapter;
        response.metadata.selectionScore = selectedCandidate.score;
        response.metadata.strategy = this.strategy;

        return response;

      } catch (error) {
        lastError = error;
        log.warn('Adapter failed, trying fallback', {
          adapter: selectedAdapter,
          error: error.message,
        });

        if (i > 0) {
          this.stats.fallbacks++;
        }
      }
    }

    this.stats.failures++;
    throw lastError || new Error('All adapters failed');
  }

  /**
   * Learned adapter selection using Thompson Sampling
   * @private
   */
  async _learnedSelection(candidates, options = {}) {
    const adapters = candidates.map(c => c.adapter);
    
    // Use Learning engine's Thompson Sampling
    if (this.learningEngine.getBestAdapter) {
      return this.learningEngine.getBestAdapter(adapters);
    }
    
    // Fallback to highest scored
    return candidates[0].adapter;
  }

  /**
   * Record learning event
   * @private
   */
  _recordLearningEvent(adapter, response, options) {
    if (!this.learningEngine) return;
    
    // Estimate cost
    const inputTokens = options.estimatedInputTokens || 1000;
    const outputTokens = response.content?.length || 500;
    const costResult = calculateCost(adapter, 'default', inputTokens, outputTokens);
    
    // Determine quality (could be from response metadata)
    const quality = response.metadata?.quality || PHI_INV;
    
    this.learningEngine.record({
      type: 'completion_success',
      adapter,
      data: {
        prompt: options.prompt || '',
        response: response.content,
        latency: response.metadata?.duration || 0,
        cost: costResult.cost,
        quality,
      },
    });
    
    log.debug('Learning event recorded', { adapter, cost: costResult.cost });
  }

  /**
   * Get learning stats
   */
  getLearningStats() {
    if (!this.learningEngine) {
      return { error: 'No learning engine set' };
    }
    
    return {
      ...this.getStats(),
      learning: this.learningEngine.getAllStats(),
    };
  }
}

export default LearningSwitch;

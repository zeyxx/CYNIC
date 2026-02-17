/**
 * CYNIC Learning Module
 *
 * Phase 4: Learning Layer
 * - Learning events collector
 * - Adapter performance tracking
 * - Thompson Sampling for exploration/exploitation
 * - LoRA fine-tuning preparation
 *
 * @module @cynic/llm/learning
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('Learning');

/**
 * Learning event types
 */
export const EventType = {
  ADAPTER_SELECTED: 'adapter_selected',
  COMPLETION_SUCCESS: 'completion_success',
  COMPLETION_FAILURE: 'completion_failure',
  LATENCY_MEASURED: 'latency_measured',
  COST_CALCULATED: 'cost_calculated',
  QUALITY_ESTIMATED: 'quality_estimated',
  USER_FEEDBACK: 'user_feedback',
};

/**
 * Learning event
 */
export class LearningEvent {
  constructor(options = {}) {
    this.id = options.id || `event-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    this.type = options.type;
    this.adapter = options.adapter || null;
    this.model = options.model || null;
    this.timestamp = options.timestamp || Date.now();
    this.data = options.data || {};
    this.reward = options.reward || 0;
  }
}

/**
 * Adapter performance tracker
 */
export class AdapterTracker {
  constructor(options = {}) {
    this.adapter = options.adapter;
    this.totalUses = 0;
    this.successes = 0;
    this.failures = 0;
    this.totalLatency = 0;
    this.totalCost = 0;
    this.qualityScores = [];
    
    // Bandit state (Thompson Sampling)
    this.alpha = 1; // successes + 1
    this.beta = 1;  // failures + 1
  }

  recordSuccess(latency, cost, quality) {
    this.totalUses++;
    this.successes++;
    this.totalLatency += latency;
    this.totalCost += cost;
    if (quality) this.qualityScores.push(quality);
    
    // Update bandit params
    this.alpha++;
  }

  recordFailure(latency, cost) {
    this.totalUses++;
    this.failures++;
    this.totalLatency += latency;
    this.totalCost += cost;
    
    // Update bandit params
    this.beta++;
  }

  getStats() {
    return {
      adapter: this.adapter,
      totalUses: this.totalUses,
      successRate: this.totalUses > 0 ? this.successes / this.totalUses : 0,
      avgLatency: this.totalUses > 0 ? this.totalLatency / this.totalUses : 0,
      totalCost: this.totalCost,
      avgQuality: this.qualityScores.length > 0 
        ? this.qualityScores.reduce((a, b) => a + b, 0) / this.qualityScores.length 
        : 0,
      // Thompson Sampling params
      alpha: this.alpha,
      beta: this.beta,
      expectedReward: this.alpha / (this.alpha + this.beta),
    };
  }

  sample() {
    // Thompson Sampling: sample from Beta distribution
    // Simplified: use Math.random() weighted by alpha/beta
    return Math.random() * (this.alpha / (this.alpha + this.beta));
  }
}

/**
 * Learning Engine - collects events and updates adapter weights
 */
export class LearningEngine {
  constructor(options = {}) {
    this.events = [];
    this.maxEvents = options.maxEvents || 10000;
    this.trackers = new Map(); // adapter -> AdapterTracker
    
    // Exploration vs exploitation
    this.explorationRate = options.explorationRate || 0.1; // 10% exploration
    
    // Stats
    this.stats = {
      eventsCollected: 0,
      updatesPerformed: 0,
    };
  }

  /**
   * Record a learning event
   */
  record(event) {
    if (!(event instanceof LearningEvent)) {
      event = new LearningEvent(event);
    }
    
    this.events.push(event);
    this.stats.eventsCollected++;
    
    // Trim old events
    if (this.events.length > this.maxEvents) {
      this.events = this.events.slice(-this.maxEvents);
    }
    
    // Update tracker if adapter event
    if (event.adapter) {
      this._updateTracker(event);
    }
    
    return event;
  }

  /**
   * Update tracker based on event
   * @private
   */
  _updateTracker(event) {
    if (!this.trackers.has(event.adapter)) {
      this.trackers.set(event.adapter, new AdapterTracker({ adapter: event.adapter }));
    }
    
    const tracker = this.trackers.get(event.adapter);
    
    if (event.type === EventType.COMPLETION_SUCCESS) {
      tracker.recordSuccess(
        event.data.latency || 0,
        event.data.cost || 0,
        event.data.quality
      );
    } else if (event.type === EventType.COMPLETION_FAILURE) {
      tracker.recordFailure(
        event.data.latency || 0,
        event.data.cost || 0
      );
    }
  }

  /**
   * Get best adapter using Thompson Sampling
   */
  getBestAdapter(adapters) {
    // Update trackers for all adapters
    for (const adapter of adapters) {
      if (!this.trackers.has(adapter)) {
        this.trackers.set(adapter, new AdapterTracker({ adapter }));
      }
    }
    
    // Exploration: random choice
    if (Math.random() < this.explorationRate) {
      const randomAdapter = adapters[Math.floor(Math.random() * adapters.length)];
      log.info('Exploration: random adapter selected', { adapter: randomAdapter });
      return randomAdapter;
    }
    
    // Exploitation: choose best according to Thompson Sampling
    let bestAdapter = adapters[0];
    let bestScore = 0;
    
    for (const adapter of adapters) {
      const tracker = this.trackers.get(adapter);
      const score = tracker.sample();
      
      if (score > bestScore) {
        bestScore = score;
        bestAdapter = adapter;
      }
    }
    
    log.info('Exploitation: best adapter selected', { adapter: bestAdapter, score: bestScore });
    return bestAdapter;
  }

  /**
   * Get all adapter stats
   */
  getAllStats() {
    const stats = {};
    for (const [adapter, tracker] of this.trackers) {
      stats[adapter] = tracker.getStats();
    }
    return stats;
  }

  /**
   * Export learning data for LoRA fine-tuning
   */
  exportForFineTuning() {
    // Aggregate successful completions
    const completions = this.events
      .filter(e => e.type === EventType.COMPLETION_SUCCESS)
      .map(e => ({
        prompt: e.data.prompt,
        response: e.data.response,
        adapter: e.adapter,
        quality: e.data.quality || 0,
        timestamp: e.timestamp,
      }));
    
    return {
      totalExamples: completions.length,
      byAdapter: this.getAllStats(),
      examples: completions.slice(-1000), // Last 1000 examples
    };
  }

  /**
   * Get recent events
   */
  getRecentEvents(count = 100) {
    return this.events.slice(-count);
  }

  /**
   * Get stats
   */
  getStats() {
    return {
      ...this.stats,
      totalEvents: this.events.length,
      adaptersTracked: this.trackers.size,
    };
  }
}

/**
 * Create LearningEngine instance
 */
export function createLearningEngine(options) {
  return new LearningEngine(options);
}

export default LearningEngine;

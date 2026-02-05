/**
 * Dog State Emitter - C6.1 (CYNIC × PERCEIVE)
 *
 * Real-time emission of Dog states for self-perception.
 * Part of the 7×7 Fractal Matrix perception layer.
 *
 * "Le chien se voit lui-même" - κυνικός
 *
 * Emits:
 * - Individual dog states (invocations, actions, warnings, blocks)
 * - Collective state (which dogs are active, consensus status)
 * - Memory state (patterns, load, freshness)
 * - Energy levels (per dog and collective)
 *
 * @module @cynic/node/perception/dog-state-emitter
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, globalEventBus, EventType } from '@cynic/core';

/**
 * Dog state snapshot schema
 */
export const DogStateType = {
  INDIVIDUAL: 'individual',   // Single dog state
  COLLECTIVE: 'collective',   // All dogs aggregate
  MEMORY: 'memory',           // Memory subsystem
  CONSENSUS: 'consensus',     // Consensus tracking
};

/**
 * Health thresholds (φ-aligned)
 */
const HEALTH_THRESHOLDS = {
  EXCELLENT: PHI_INV,       // > 61.8%
  GOOD: PHI_INV_2,          // > 38.2%
  WARNING: PHI_INV_3,       // > 23.6%
  CRITICAL: 0,              // <= 23.6%
};

/**
 * Dog State Emitter - Real-time self-perception
 */
export class DogStateEmitter extends EventEmitter {
  /**
   * @param {Object} options
   * @param {Object} [options.collectivePack] - Reference to CollectivePack
   * @param {Object} [options.sharedMemory] - Reference to SharedMemory
   * @param {number} [options.emitIntervalMs] - Interval for periodic emissions (default: 5000)
   * @param {boolean} [options.autoStart] - Start periodic emission immediately (default: false)
   */
  constructor(options = {}) {
    super();

    this._pack = options.collectivePack || null;
    this._memory = options.sharedMemory || null;
    this._emitIntervalMs = options.emitIntervalMs || 5000;
    this._autoStart = options.autoStart || false;

    // Periodic emission timer
    this._intervalId = null;

    // State cache (reduce redundant emissions)
    this._lastState = null;
    this._lastEmitTime = 0;

    // Subscribers count
    this._subscriberCount = 0;

    // Stats
    this._stats = {
      emissions: 0,
      suppressedDuplicates: 0,
      started: null,
      lastEmission: null,
    };

    // Dog names for iteration
    this._dogNames = [
      'guardian', 'analyst', 'scholar', 'architect', 'sage',
      'cynic', 'janitor', 'scout', 'cartographer', 'oracle', 'deployer'
    ];

    if (this._autoStart) {
      this.start();
    }
  }

  /**
   * Set CollectivePack reference
   */
  setCollectivePack(pack) {
    this._pack = pack;
  }

  /**
   * Set SharedMemory reference
   */
  setSharedMemory(memory) {
    this._memory = memory;
  }

  /**
   * Start periodic state emission
   */
  start() {
    if (this._intervalId) return;

    this._stats.started = Date.now();
    this._intervalId = setInterval(() => {
      this._emitPeriodicState();
    }, this._emitIntervalMs);

    // Emit initial state
    this._emitPeriodicState();
  }

  /**
   * Stop periodic emission
   */
  stop() {
    if (this._intervalId) {
      clearInterval(this._intervalId);
      this._intervalId = null;
    }
  }

  /**
   * Emit state immediately
   */
  emitNow() {
    return this._emitPeriodicState(true);
  }

  /**
   * Get individual dog state
   *
   * @param {string} dogName - Name of the dog
   * @returns {Object|null} Dog state or null
   */
  getDogState(dogName) {
    if (!this._pack) return null;

    const dog = this._pack[dogName];
    if (!dog) return null;

    const stats = dog.getStats?.() || dog.stats || {};
    const summary = dog.getSummary?.() || {};

    // Calculate health based on success rate
    const successRate = stats.invocations > 0
      ? (stats.invocations - (stats.errors || 0)) / stats.invocations
      : 1;

    const health = this._calculateHealth(successRate);

    return {
      name: dogName,
      stats: {
        invocations: stats.invocations || 0,
        actions: stats.actions || 0,
        blocks: stats.blocks || 0,
        warnings: stats.warnings || 0,
        errors: stats.errors || 0,
        lastInvocation: stats.lastInvocation,
      },
      health,
      successRate,
      isActive: stats.lastInvocation && (Date.now() - stats.lastInvocation < 60000),
      patterns: summary.patterns?.length || 0,
      timestamp: Date.now(),
    };
  }

  /**
   * Get collective state (all dogs)
   */
  getCollectiveState() {
    if (!this._pack) {
      return { available: false, reason: 'No pack reference' };
    }

    const dogStates = {};
    let totalInvocations = 0;
    let totalActions = 0;
    let totalBlocks = 0;
    let activeDogs = 0;
    let healthSum = 0;

    for (const name of this._dogNames) {
      const state = this.getDogState(name);
      if (state) {
        dogStates[name] = state;
        totalInvocations += state.stats.invocations;
        totalActions += state.stats.actions;
        totalBlocks += state.stats.blocks;
        healthSum += state.successRate;
        if (state.isActive) activeDogs++;
      }
    }

    const avgHealth = this._dogNames.length > 0
      ? healthSum / this._dogNames.length
      : 0;

    return {
      type: DogStateType.COLLECTIVE,
      timestamp: Date.now(),
      dogCount: this._dogNames.length,
      activeDogs,
      dogs: dogStates,
      aggregate: {
        totalInvocations,
        totalActions,
        totalBlocks,
        averageHealth: avgHealth,
        healthRating: this._calculateHealth(avgHealth),
      },
      eventBusStats: this._pack.eventBus?.getStats?.() || null,
      profileLevel: this._pack.profileLevel,
    };
  }

  /**
   * Get memory state
   */
  getMemoryState() {
    if (!this._memory) {
      return { available: false, reason: 'No memory reference' };
    }

    const stats = this._memory.getStats?.() || {};
    const patterns = this._memory.patterns || [];

    // Calculate memory load (based on pattern count vs limit)
    const maxPatterns = 987; // Fib(16) default
    const load = patterns.length / maxPatterns;

    // Calculate freshness (average age of patterns)
    const now = Date.now();
    let totalAge = 0;
    for (const pattern of patterns) {
      totalAge += now - (pattern.createdAt || now);
    }
    const avgAge = patterns.length > 0 ? totalAge / patterns.length : 0;
    const freshness = Math.max(0, 1 - avgAge / (24 * 60 * 60 * 1000)); // 24h window

    return {
      type: DogStateType.MEMORY,
      timestamp: Date.now(),
      patternCount: patterns.length,
      load,
      loadRating: this._calculateHealth(1 - load), // Inverse - low load is healthy
      freshness,
      freshnessRating: this._calculateHealth(freshness),
      contextCount: stats.contextCount || 0,
      fisherScores: stats.fisherScores || 0,
      ewcEnabled: !!this._memory.ewc,
    };
  }

  /**
   * Calculate health rating from value (φ-aligned)
   * @private
   */
  _calculateHealth(value) {
    if (value >= HEALTH_THRESHOLDS.EXCELLENT) return 'excellent';
    if (value >= HEALTH_THRESHOLDS.GOOD) return 'good';
    if (value >= HEALTH_THRESHOLDS.WARNING) return 'warning';
    return 'critical';
  }

  /**
   * Emit periodic state
   * @private
   */
  _emitPeriodicState(force = false) {
    const state = {
      collective: this.getCollectiveState(),
      memory: this.getMemoryState(),
      timestamp: Date.now(),
    };

    // Check if state changed (to avoid spam)
    const stateHash = this._hashState(state);
    if (!force && stateHash === this._lastState) {
      this._stats.suppressedDuplicates++;
      return null;
    }

    this._lastState = stateHash;
    this._lastEmitTime = Date.now();
    this._stats.emissions++;
    this._stats.lastEmission = Date.now();

    // Emit locally
    this.emit('state', state);

    // Emit to globalEventBus for cross-system integration
    try {
      globalEventBus.publish(EventType.CYNIC_STATE, {
        source: 'DogStateEmitter',
        stateType: 'periodic',
        collective: {
          activeDogs: state.collective.activeDogs,
          dogCount: state.collective.dogCount,
          averageHealth: state.collective.aggregate?.averageHealth,
          healthRating: state.collective.aggregate?.healthRating,
        },
        memory: {
          patternCount: state.memory.patternCount,
          load: state.memory.load,
          freshness: state.memory.freshness,
        },
        timestamp: state.timestamp,
      }, { source: 'DogStateEmitter' });
    } catch (e) {
      // Non-critical
    }

    return state;
  }

  /**
   * Hash state for change detection
   * @private
   */
  _hashState(state) {
    // Simple hash based on key metrics
    const c = state.collective?.aggregate || {};
    const m = state.memory || {};
    return `${c.totalInvocations}:${c.activeDogs}:${m.patternCount}:${m.load?.toFixed(2)}`;
  }

  /**
   * Emit a specific dog's state change (for real-time updates)
   *
   * @param {string} dogName - Dog that changed
   * @param {string} eventType - Type of change (invocation, block, warning)
   * @param {Object} [details] - Additional details
   */
  emitDogEvent(dogName, eventType, details = {}) {
    const dogState = this.getDogState(dogName);
    if (!dogState) return;

    const event = {
      type: DogStateType.INDIVIDUAL,
      dog: dogName,
      eventType,
      state: dogState,
      details,
      timestamp: Date.now(),
    };

    this.emit('dog_event', event);
    this._stats.emissions++;

    // Also emit to global bus
    try {
      globalEventBus.publish(EventType.DOG_EVENT, {
        dog: dogName,
        eventType,
        stats: dogState.stats,
        health: dogState.health,
        details,
        timestamp: Date.now(),
      }, { source: 'DogStateEmitter' });
    } catch (e) {
      // Non-critical
    }
  }

  /**
   * Get emitter statistics
   */
  getStats() {
    return {
      ...this._stats,
      isRunning: !!this._intervalId,
      emitIntervalMs: this._emitIntervalMs,
      subscriberCount: this.listenerCount('state') + this.listenerCount('dog_event'),
    };
  }

  /**
   * Get summary
   */
  getSummary() {
    return {
      running: !!this._intervalId,
      stats: this.getStats(),
      collective: this.getCollectiveState(),
      memory: this.getMemoryState(),
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create DogStateEmitter singleton
 */
export function getDogStateEmitter(options = {}) {
  if (!_instance) {
    _instance = new DogStateEmitter(options);
  } else {
    // Update references if provided
    if (options.collectivePack) {
      _instance.setCollectivePack(options.collectivePack);
    }
    if (options.sharedMemory) {
      _instance.setSharedMemory(options.sharedMemory);
    }
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetDogStateEmitter() {
  if (_instance) {
    _instance.stop();
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  DogStateEmitter,
  DogStateType,
  getDogStateEmitter,
  resetDogStateEmitter,
};

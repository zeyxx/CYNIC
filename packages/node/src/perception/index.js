/**
 * CYNIC Perception Layer - Multi-Dimensional Awareness
 *
 * Watches filesystem and blockchain events, feeding the EventBus.
 * Part of the sensory system that enables CYNIC to perceive its environment.
 *
 * S3.2: Concurrent sensor polling for optimal latency.
 *
 * "The dog smells before it sees" - cynic
 *
 * @module @cynic/node/perception
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('PerceptionLayer');

// Filesystem Watcher
export {
  FilesystemWatcher,
  createFilesystemWatcher,
  FilesystemEventType,
} from './filesystem-watcher.js';

// Solana Watcher - C2.1 (SOLANA × PERCEIVE)
export {
  SolanaWatcher,
  createSolanaWatcher,
  SolanaEventType,
  getSolanaWatcher,
  resetSolanaWatcher,
} from './solana-watcher.js';

// C6.1: Dog State Emitter (CYNIC × PERCEIVE)
export {
  DogStateEmitter,
  DogStateType,
  getDogStateEmitter,
  resetDogStateEmitter,
} from './dog-state-emitter.js';

// Machine Health Watcher - C5.1 (HUMAN × PERCEIVE)
export {
  MachineHealthWatcher,
} from './machine-health-watcher.js';

// Market Watcher - C3.1 (MARKET × PERCEIVE)
export {
  MarketWatcher,
  MarketEventType,
} from './market-watcher.js';

/**
 * Create a unified perception layer with concurrent sensor polling
 *
 * @param {Object} [options] - Configuration
 * @param {Object} [options.filesystem] - Filesystem watcher options
 * @param {Object} [options.solana] - Solana watcher options
 * @param {Object} [options.health] - Machine health watcher options
 * @param {Object} [options.dogState] - Dog state emitter options
 * @param {Object} [options.market] - Market watcher options
 * @param {EventBus} [options.eventBus] - Shared EventBus
 * @param {boolean} [options.enableConcurrentPolling=true] - Enable concurrent sensor polling
 * @returns {Object} Perception layer with start/stop/poll methods
 */
export function createPerceptionLayer(options = {}) {
  const { FilesystemWatcher } = require('./filesystem-watcher.js');
  const { SolanaWatcher } = require('./solana-watcher.js');
  const { MachineHealthWatcher } = require('./machine-health-watcher.js');
  const { DogStateEmitter } = require('./dog-state-emitter.js');
  const { MarketWatcher } = require('./market-watcher.js');

  const enableConcurrentPolling = options.enableConcurrentPolling !== false;

  // Initialize all sensors
  const fsWatcher = new FilesystemWatcher({
    ...options.filesystem,
    eventBus: options.eventBus,
  });

  const solanaWatcher = new SolanaWatcher({
    ...options.solana,
    eventBus: options.eventBus,
  });

  const healthWatcher = new MachineHealthWatcher({
    ...options.health,
    eventBus: options.eventBus,
  });

  const dogStateEmitter = new DogStateEmitter({
    ...options.dogState,
  });

  const marketWatcher = new MarketWatcher({
    ...options.market,
    eventBus: options.eventBus,
  });

  return {
    filesystem: fsWatcher,
    solana: solanaWatcher,
    health: healthWatcher,
    dogState: dogStateEmitter,
    market: marketWatcher,

    /**
     * Start all watchers
     */
    async start() {
      const startTime = Date.now();

      if (enableConcurrentPolling) {
        // Concurrent startup (faster)
        const results = await Promise.allSettled([
          Promise.resolve(fsWatcher.start()),
          solanaWatcher.start(),
          healthWatcher.start(),
          Promise.resolve(dogStateEmitter.start()),
          marketWatcher.start(),
        ]);

        const failures = results.filter(r => r.status === 'rejected');
        if (failures.length > 0) {
          log.warn('Some sensors failed to start', {
            failures: failures.map(f => f.reason?.message || 'unknown'),
            elapsed: Date.now() - startTime,
          });
        } else {
          log.info('All sensors started', { elapsed: Date.now() - startTime });
        }
      } else {
        // Sequential startup (legacy)
        fsWatcher.start();
        await solanaWatcher.start();
        await healthWatcher.start();
        dogStateEmitter.start();
        await marketWatcher.start();
      }

      return this;
    },

    /**
     * Stop all watchers
     */
    async stop() {
      const stopTime = Date.now();

      // Always stop concurrently
      const results = await Promise.allSettled([
        fsWatcher.stop(),
        solanaWatcher.stop(),
        healthWatcher.stop(),
        Promise.resolve(dogStateEmitter.stop()),
        marketWatcher.stop(),
      ]);

      const failures = results.filter(r => r.status === 'rejected');
      if (failures.length > 0) {
        log.warn('Some sensors failed to stop', {
          failures: failures.map(f => f.reason?.message || 'unknown'),
        });
      }

      log.info('Perception layer stopped', { elapsed: Date.now() - stopTime });
    },

    /**
     * Poll all sensors concurrently (S3.2 optimization)
     *
     * Returns partial results even if some sensors fail.
     * Each sensor's state is captured independently.
     *
     * @returns {Promise<Object>} Perception snapshot
     */
    async poll() {
      const pollStart = Date.now();

      // Concurrent poll (Promise.allSettled for resilience)
      const [
        solanaResult,
        healthResult,
        dogStateResult,
        marketResult,
        filesystemResult,
      ] = await Promise.allSettled([
        // Solana: Get current health + subscription state
        Promise.resolve().then(() => ({
          health: solanaWatcher.getHealth(),
          subscriptions: solanaWatcher.getSubscriptions(),
          isRunning: solanaWatcher.isRunning(),
        })),

        // Machine Health: Get current metrics
        Promise.resolve().then(() => healthWatcher.getHealth()),

        // Dog State: Get collective + memory state
        Promise.resolve().then(() => ({
          collective: dogStateEmitter.getCollectiveState(),
          memory: dogStateEmitter.getMemoryState(),
        })),

        // Market: Get current state
        Promise.resolve().then(() => marketWatcher.getState()),

        // Filesystem: Get current stats
        Promise.resolve().then(() => fsWatcher.getStats()),
      ]);

      // Extract results (null if failed)
      const snapshot = {
        solana: solanaResult.status === 'fulfilled' ? solanaResult.value : { error: solanaResult.reason?.message },
        health: healthResult.status === 'fulfilled' ? healthResult.value : { error: healthResult.reason?.message },
        dogState: dogStateResult.status === 'fulfilled' ? dogStateResult.value : { error: dogStateResult.reason?.message },
        market: marketResult.status === 'fulfilled' ? marketResult.value : { error: marketResult.reason?.message },
        filesystem: filesystemResult.status === 'fulfilled' ? filesystemResult.value : { error: filesystemResult.reason?.message },
        timestamp: Date.now(),
        latency: Date.now() - pollStart,
      };

      // Log failures
      const failures = [solanaResult, healthResult, dogStateResult, marketResult, filesystemResult]
        .filter(r => r.status === 'rejected');

      if (failures.length > 0) {
        log.debug('Sensor poll partial failure', {
          failedCount: failures.length,
          latency: snapshot.latency,
        });
      }

      return snapshot;
    },

    /**
     * Check if perception layer is active
     */
    isRunning() {
      return fsWatcher.isRunning()
        || solanaWatcher.isRunning()
        || !!healthWatcher._isRunning
        || !!dogStateEmitter._intervalId
        || marketWatcher._isRunning;
    },

    /**
     * Get combined statistics
     */
    getStats() {
      return {
        filesystem: fsWatcher.getStats(),
        solana: solanaWatcher.getStats(),
        health: healthWatcher.getHealth(),
        dogState: dogStateEmitter.getStats(),
        market: marketWatcher.stats,
      };
    },
  };
}

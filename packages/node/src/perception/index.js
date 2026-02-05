/**
 * CYNIC Perception Layer - Multi-Dimensional Awareness
 *
 * Watches filesystem and blockchain events, feeding the EventBus.
 * Part of the sensory system that enables CYNIC to perceive its environment.
 *
 * "The dog smells before it sees" - cynic
 *
 * @module @cynic/node/perception
 */

'use strict';

// Filesystem Watcher
export {
  FilesystemWatcher,
  createFilesystemWatcher,
  FilesystemEventType,
} from './filesystem-watcher.js';

// Solana Watcher
export {
  SolanaWatcher,
  createSolanaWatcher,
  SolanaEventType,
} from './solana-watcher.js';

// C6.1: Dog State Emitter (CYNIC × PERCEIVE)
export {
  DogStateEmitter,
  DogStateType,
  getDogStateEmitter,
  resetDogStateEmitter,
} from './dog-state-emitter.js';

/**
 * Create a unified perception layer
 *
 * @param {Object} [options] - Configuration
 * @param {Object} [options.filesystem] - Filesystem watcher options
 * @param {Object} [options.solana] - Solana watcher options
 * @param {EventBus} [options.eventBus] - Shared EventBus
 * @returns {Object} Perception layer with start/stop methods
 */
export function createPerceptionLayer(options = {}) {
  const { FilesystemWatcher } = require('./filesystem-watcher.js');
  const { SolanaWatcher } = require('./solana-watcher.js');

  const fsWatcher = new FilesystemWatcher({
    ...options.filesystem,
    eventBus: options.eventBus,
  });

  const solanaWatcher = new SolanaWatcher({
    ...options.solana,
    eventBus: options.eventBus,
  });

  return {
    filesystem: fsWatcher,
    solana: solanaWatcher,

    /**
     * Start all watchers
     */
    async start() {
      fsWatcher.start();
      await solanaWatcher.start();
      return this;
    },

    /**
     * Stop all watchers
     */
    async stop() {
      await Promise.all([
        fsWatcher.stop(),
        solanaWatcher.stop(),
      ]);
    },

    /**
     * Check if perception layer is active
     */
    isRunning() {
      return fsWatcher.isRunning() || solanaWatcher.isRunning();
    },

    /**
     * Get combined statistics
     */
    getStats() {
      return {
        filesystem: fsWatcher.getStats(),
        solana: solanaWatcher.getStats(),
      };
    },
  };
}

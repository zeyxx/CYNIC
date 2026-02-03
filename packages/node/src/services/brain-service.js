/**
 * CYNIC Brain Service
 *
 * Pre-configured Brain with all orchestrators wired.
 * This is the "alive" version of CYNIC - ready to think.
 *
 * "Le cerveau vivant" - κυνικός
 *
 * Usage:
 *   import { getBrainService } from '@cynic/node';
 *   const brain = await getBrainService();
 *   const thought = await brain.think({ content: 'my prompt' });
 *
 * @module @cynic/node/services/brain-service
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { EngineOrchestrator, globalEngineRegistry, loadPhilosophyEngines, areEnginesLoaded } from '@cynic/core/engines';
import { Brain, createBrain } from '../orchestration/brain.js';
import { DogOrchestrator, DogMode } from '../agents/orchestrator.js';
import SharedMemory from '../memory/shared-memory.js';
import { getEventBus } from './event-bus.js';

// Singleton for shared memory
let _sharedMemory = null;
function getSharedMemory() {
  if (!_sharedMemory) {
    _sharedMemory = new SharedMemory();
  }
  return _sharedMemory;
}

const log = createLogger('BrainService');

/**
 * Brain Service configuration
 */
export const BRAIN_CONFIG = {
  // Dog orchestrator settings
  dogs: {
    mode: DogMode.PARALLEL,
    consensusThreshold: PHI_INV,
    useSwarmConsensus: true,
  },

  // Engine orchestrator settings
  engines: {
    timeout: 5000,
    defaultStrategy: 'weighted-average',
  },

  // Brain settings
  brain: {
    maxThoughtHistory: 100,
  },
};

/**
 * Brain Service - Fully configured Brain singleton
 */
class BrainService {
  constructor() {
    this._brain = null;
    this._dogOrchestrator = null;
    this._engineOrchestrator = null;
    this._memoryStore = null;
    this._initialized = false;
    this._initializing = false;
  }

  /**
   * Initialize the brain service
   * @param {Object} [options] - Override options
   * @returns {Promise<Brain>}
   */
  async initialize(options = {}) {
    if (this._initialized) {
      return this._brain;
    }

    if (this._initializing) {
      // Wait for existing initialization
      while (this._initializing) {
        await new Promise(resolve => setTimeout(resolve, 10));
      }
      return this._brain;
    }

    this._initializing = true;

    try {
      log.info('Initializing BrainService...');

      // 1. Create Brain
      this._brain = createBrain({
        ...BRAIN_CONFIG.brain,
        ...options.brain,
      });

      // 2. Create/get shared memory
      try {
        this._memoryStore = getSharedMemory();
        this._brain.setMemoryStore(this._memoryStore);
        log.debug('Memory store wired');
      } catch (e) {
        log.warn('Memory store not available', { error: e.message });
      }

      // 3. Create DogOrchestrator (if dependencies available)
      try {
        this._dogOrchestrator = await this._createDogOrchestrator(options);
        if (this._dogOrchestrator) {
          this._brain.setDogOrchestrator(this._dogOrchestrator);
          log.debug('DogOrchestrator wired');
        }
      } catch (e) {
        log.warn('DogOrchestrator not available', { error: e.message });
      }

      // 4. Create EngineOrchestrator
      try {
        this._engineOrchestrator = this._createEngineOrchestrator(options);
        if (this._engineOrchestrator) {
          this._brain.setEngineOrchestrator(this._engineOrchestrator);
          log.debug('EngineOrchestrator wired');
        }
      } catch (e) {
        log.warn('EngineOrchestrator not available', { error: e.message });
      }

      this._initialized = true;
      log.info('BrainService initialized', {
        hasDogs: !!this._dogOrchestrator,
        hasEngines: !!this._engineOrchestrator,
        hasMemory: !!this._memoryStore,
      });

      return this._brain;

    } finally {
      this._initializing = false;
    }
  }

  /**
   * Create DogOrchestrator with dependencies
   * @private
   */
  async _createDogOrchestrator(options = {}) {
    const config = { ...BRAIN_CONFIG.dogs, ...options.dogs };

    // DogOrchestrator needs collectivePack and sharedMemory
    // Try to get them from existing services
    let collectivePack = null;
    let sharedMemory = null;

    try {
      sharedMemory = getSharedMemory();
    } catch (e) {
      log.debug('SharedMemory not available for DogOrchestrator');
    }

    // CollectivePack is optional - dogs can work without it
    try {
      const { getCollectivePack } = await import('../agents/collective/index.js');
      collectivePack = getCollectivePack?.();
    } catch (e) {
      log.debug('CollectivePack not available for DogOrchestrator');
    }

    // Create orchestrator even without all deps
    const orchestrator = new DogOrchestrator({
      collectivePack,
      sharedMemory,
      mode: config.mode,
      consensusThreshold: config.consensusThreshold,
      useSwarmConsensus: config.useSwarmConsensus,
      eventBus: getEventBus(),
    });

    return orchestrator;
  }

  /**
   * Create EngineOrchestrator
   * @private
   */
  _createEngineOrchestrator(options = {}) {
    const config = { ...BRAIN_CONFIG.engines, ...options.engines };

    // Load philosophy engines if not already loaded
    if (!areEnginesLoaded()) {
      const loadResult = loadPhilosophyEngines({ silent: true });
      log.debug('Philosophy engines loaded', {
        registered: loadResult.registered,
        failed: loadResult.failed,
      });
    }

    // Use global engine registry (now populated)
    const orchestrator = new EngineOrchestrator(globalEngineRegistry, {
      timeout: config.timeout,
      defaultStrategy: config.defaultStrategy,
    });

    return orchestrator;
  }

  /**
   * Get the brain (initializing if needed)
   * @returns {Promise<Brain>}
   */
  async getBrain() {
    if (!this._initialized) {
      await this.initialize();
    }
    return this._brain;
  }

  /**
   * Think using the brain (convenience method)
   * @param {Object} input - What to think about
   * @param {Object} [options] - Thinking options
   * @returns {Promise<import('../orchestration/brain.js').Thought>}
   */
  async think(input, options = {}) {
    const brain = await this.getBrain();
    return brain.think(input, options);
  }

  /**
   * Quick judgment
   * @param {Object} input
   * @returns {Promise<import('../orchestration/brain.js').Thought>}
   */
  async judge(input) {
    const brain = await this.getBrain();
    return brain.judge(input);
  }

  /**
   * Deep synthesis
   * @param {Object} input
   * @returns {Promise<import('../orchestration/brain.js').Thought>}
   */
  async synthesize(input) {
    const brain = await this.getBrain();
    return brain.synthesize(input);
  }

  /**
   * Get service status
   */
  getStatus() {
    return {
      initialized: this._initialized,
      hasBrain: !!this._brain,
      hasDogOrchestrator: !!this._dogOrchestrator,
      hasEngineOrchestrator: !!this._engineOrchestrator,
      hasMemoryStore: !!this._memoryStore,
      brainStats: this._brain?.getStats() || null,
    };
  }

  /**
   * Reset the service (for testing)
   */
  reset() {
    this._brain = null;
    this._dogOrchestrator = null;
    this._engineOrchestrator = null;
    this._memoryStore = null;
    this._initialized = false;
    this._initializing = false;
  }
}

// Singleton instance
let _brainService = null;

/**
 * Get the brain service singleton
 * @returns {BrainService}
 */
export function getBrainService() {
  if (!_brainService) {
    _brainService = new BrainService();
  }
  return _brainService;
}

/**
 * Get an initialized brain (async convenience)
 * @param {Object} [options] - Override options
 * @returns {Promise<Brain>}
 */
export async function getConfiguredBrain(options = {}) {
  const service = getBrainService();
  return service.initialize(options);
}

/**
 * Think using the configured brain (async convenience)
 * @param {Object} input
 * @param {Object} [options]
 * @returns {Promise<import('../orchestration/brain.js').Thought>}
 */
export async function thinkWithBrain(input, options = {}) {
  const service = getBrainService();
  return service.think(input, options);
}

/**
 * Reset the brain service (for testing)
 */
export function _resetBrainServiceForTesting() {
  if (_brainService) {
    _brainService.reset();
  }
  _brainService = null;
}

export { BrainService };
export default getBrainService;

/**
 * EmergenceComponent - Emergence & Consciousness Domain
 *
 * Encapsulates EmergenceLayer, collective pack, shared memory, and consciousness.
 * Part of CYNICNode decomposition (Phase 2, #32).
 *
 * "The crown observes all" - κυνικός
 *
 * @module @cynic/node/components/emergence-component
 */

'use strict';

import { EventEmitter } from 'events';
import { createEmergenceLayer } from '../emergence/layer.js';
import { SharedMemory } from '../memory/shared-memory.js';
import { LabManager } from '../memory/user-lab.js';
import { DogOrchestrator, DogMode } from '../agents/orchestrator.js';
import {
  AgentEventBus,
  EventPriority,
  createCollectivePack,
} from '../agents/index.js';

/**
 * Emergence Component - manages consciousness and collective awareness
 *
 * Single Responsibility: Pattern emergence, consciousness tracking, collective memory
 */
export class EmergenceComponent extends EventEmitter {
  /**
   * Create emergence component
   *
   * @param {Object} options - Component options
   * @param {string} options.nodeId - Node ID
   * @param {number} options.eScore - Initial E-Score
   * @param {string} [options.dataDir] - Data directory for labs
   * @param {Object} [options.storage] - Storage instance for SharedMemory
   * @param {string} [options.orchestratorMode] - DogOrchestrator mode
   */
  constructor(options = {}) {
    super();

    this._nodeId = options.nodeId;

    // Initialize emergence layer (Layer 7)
    this._emergence = createEmergenceLayer({
      nodeId: options.nodeId,
      eScore: options.eScore,
    });

    // Initialize 6-Layer Memory Architecture

    // Layer 2-3: SharedMemory (Collective + Procedural)
    this._sharedMemory = new SharedMemory({
      storage: options.storage,
    });

    // Layer 4: User Labs
    this._labManager = new LabManager({
      dataDir: options.dataDir ? `${options.dataDir}/labs` : null,
    });

    // Dog Orchestrator
    this._orchestrator = new DogOrchestrator({
      sharedMemory: this._sharedMemory,
      mode: options.orchestratorMode || DogMode.PARALLEL,
    });

    // Event bus for inter-agent communication
    this._eventBus = new AgentEventBus({
      nodeId: options.nodeId,
    });

    // The collective pack (11 dogs)
    this._collectivePack = createCollectivePack({
      eventBus: this._eventBus,
      sharedMemory: this._sharedMemory,
      nodeId: options.nodeId,
    });

    // Track initialization
    this._initialized = false;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Lifecycle
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Initialize emergence systems
   * @returns {Promise<void>}
   */
  async initialize() {
    // Initialize emergence layer
    this._emergence.initialize();

    // Initialize shared memory
    await this._sharedMemory.initialize();

    this._initialized = true;
    console.log('[EmergenceComponent] Emergence layer initialized');
  }

  /**
   * Save memory state
   * @returns {Promise<void>}
   */
  async save() {
    await this._sharedMemory.save();
    await this._labManager.saveAll();
    console.log('[EmergenceComponent] Memory saved');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Observation & Patterns
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Observe a judgment for pattern detection
   * @param {Object} judgment - Judgment to observe
   */
  observeJudgment(judgment) {
    this._emergence.observeJudgment(judgment);
  }

  /**
   * Update E-Score in emergence layer
   * @param {number} eScore - New E-Score
   */
  updateEScore(eScore) {
    this._emergence.updateEScore(eScore);
  }

  /**
   * Report to collective
   */
  reportToCollective() {
    this._emergence.reportToCollective();
  }

  /**
   * Sync significant emergence patterns to shared memory
   */
  syncPatternsToMemory() {
    const topPatterns = this._emergence.patterns.getTopPatterns(10);

    for (const pattern of topPatterns) {
      if (pattern.significance >= 0.5) {
        const existingId = `emrg_${pattern.id}`;
        const existing = this._sharedMemory._patterns?.get(existingId);

        if (!existing) {
          this._sharedMemory.addPattern({
            id: existingId,
            type: pattern.type,
            rule: pattern.summary || `Detected pattern: ${pattern.type}`,
            applicableTo: ['*'],
            confidence: Math.min(pattern.significance, 0.618), // Cap at φ⁻¹
            source: 'emergence',
            verified: pattern.verified || false,
            occurrences: pattern.occurrences,
          });
        }
      }
    }
  }

  /**
   * Called on each epoch - syncs patterns and reports
   */
  onEpoch() {
    this.syncPatternsToMemory();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Collective Consciousness
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Wire collective events to external handlers
   *
   * @param {Object} handlers - Event handlers
   * @param {Function} [handlers.onConsensusReached] - (event) => void
   * @param {Function} [handlers.onInsightShared] - (event) => void
   */
  wireCollectiveEvents(handlers = {}) {
    // Collective consensus
    this._eventBus.on('consensus:reached', (event) => {
      this.emit('collective:consensus', event);
      handlers.onConsensusReached?.(event);
    });

    // Individual dog insights
    this._eventBus.on('insight:shared', (event) => {
      const { insight, agentId, priority } = event;

      // High-priority insights go to shared memory
      if (priority >= EventPriority.HIGH) {
        this._sharedMemory.addPattern({
          type: 'dog_insight',
          source: agentId,
          content: insight,
          priority,
          timestamp: Date.now(),
        });
      }

      this.emit('collective:insight', event);
      handlers.onInsightShared?.(event);
    });
  }

  /**
   * Request collective review of a judgment
   *
   * @param {Object} judgment - Judgment to review
   * @param {Object} item - Original item
   * @returns {Promise<void>}
   */
  async reviewJudgment(judgment, item) {
    if (!this._collectivePack?.reviewJudgment) return;

    try {
      await this._collectivePack.reviewJudgment(judgment, item);
    } catch (err) {
      console.debug(`[EmergenceComponent] Review skipped: ${err.message}`);
    }
  }

  /**
   * Publish event to collective
   *
   * @param {string} eventType - Event type
   * @param {Object} data - Event data
   */
  publishEvent(eventType, data) {
    this._eventBus.emit(eventType, data);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Shared Memory Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Add pattern to shared memory
   * @param {Object} pattern - Pattern to add
   */
  addPattern(pattern) {
    this._sharedMemory.addPattern(pattern);
  }

  /**
   * Adjust weight in shared memory
   *
   * @param {string} dimension - Dimension name
   * @param {number} delta - Weight change
   * @param {string} source - Change source
   */
  adjustWeight(dimension, delta, source) {
    this._sharedMemory.adjustWeight(dimension, delta, source);
  }

  /**
   * Record feedback in shared memory
   * @param {Object} feedback - Feedback to record
   */
  recordFeedback(feedback) {
    this._sharedMemory.recordFeedback(feedback);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // User Labs
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get or create a user lab
   *
   * @param {string} userId - User ID
   * @returns {Object} User lab
   */
  getOrCreateLab(userId) {
    return this._labManager.getOrCreate(userId);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // State & Stats
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {string} Current consciousness state */
  get consciousnessState() {
    return this._emergence.consciousness.state;
  }

  /** @returns {string} Current collective phase */
  get collectivePhase() {
    return this._emergence.collective.phase;
  }

  /** @returns {boolean} Whether initialized */
  get initialized() {
    return this._initialized;
  }

  /**
   * Get emergence state
   * @returns {Object} Emergence state
   */
  getState() {
    return this._emergence.getState();
  }

  /**
   * Get component info
   * @returns {Object} Component info
   */
  getInfo() {
    return {
      initialized: this._initialized,
      emergence: this.getState(),
      consciousnessState: this.consciousnessState,
      collectivePhase: this.collectivePhase,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Export/Import
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Export component state
   * @returns {Object} Exportable state
   */
  export() {
    return {
      emergenceState: this._emergence.export(),
    };
  }

  /**
   * Import state
   * @param {Object} state - Saved state
   */
  import(state) {
    if (state.emergenceState) {
      this._emergence.import(state.emergenceState);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Backward Compatibility
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get raw emergence layer
   * @returns {Object} Emergence layer
   * @deprecated Use component methods instead
   */
  get emergence() {
    return this._emergence;
  }

  /**
   * Get raw shared memory
   * @returns {SharedMemory} Shared memory
   * @deprecated Use component methods instead
   */
  get sharedMemory() {
    return this._sharedMemory;
  }

  /**
   * Get raw event bus
   * @returns {AgentEventBus} Event bus
   * @deprecated Use component methods instead
   */
  get eventBus() {
    return this._eventBus;
  }

  /**
   * Get raw collective pack
   * @returns {Object} Collective pack
   * @deprecated Use component methods instead
   */
  get collectivePack() {
    return this._collectivePack;
  }
}

export default EmergenceComponent;

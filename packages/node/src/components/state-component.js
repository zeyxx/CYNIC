/**
 * StateComponent - State Management Domain
 *
 * Encapsulates StateManager, persistence, chain, and knowledge tree.
 * Part of CYNICNode decomposition (Phase 2, #29).
 *
 * "State is memory, memory is wisdom" - κυνικός
 *
 * @module @cynic/node/components/state-component
 */

'use strict';

import { EventEmitter } from 'events';
import { StateManager } from '../state/manager.js';

/**
 * State Component - manages node state and persistence
 *
 * Single Responsibility: State persistence, chain operations, knowledge management
 */
export class StateComponent extends EventEmitter {
  /**
   * Create state component
   *
   * @param {Object} options - Component options
   * @param {string} [options.dataDir] - Data directory for persistence
   * @param {Object} options.operator - Operator instance (for StateManager)
   */
  constructor(options = {}) {
    super();

    this._dataDir = options.dataDir;

    // Initialize state manager
    this._state = new StateManager({
      dataDir: options.dataDir,
      operator: options.operator,
    });

    // Track initialization
    this._initialized = false;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Lifecycle
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Initialize state (load from disk)
   * @returns {Promise<void>}
   */
  async initialize() {
    await this._state.initialize();
    this._initialized = true;
    console.log('[StateComponent] State initialized');
  }

  /**
   * Save state to disk
   * @returns {Promise<void>}
   */
  async save() {
    await this._state.save();
    console.log('[StateComponent] State saved');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Chain Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get the PoJ chain
   * @returns {Object} Chain instance
   */
  get chain() {
    return this._state.chain;
  }

  /**
   * Get latest block hash
   * @returns {string} Latest hash
   */
  get latestHash() {
    return this._state.chain.latestHash;
  }

  /**
   * Add a judgment block to chain
   * @param {Array} judgments - Judgments to include
   * @returns {Object} Created block
   */
  addJudgmentBlock(judgments) {
    return this._state.chain.addJudgmentBlock(judgments);
  }

  /**
   * Import a block from external source
   * @param {Object} block - Block to import
   * @returns {Object} Import result
   */
  importBlock(block) {
    return this._state.chain.importBlock(block);
  }

  /**
   * Persist a finalized block
   * @param {Object} block - Block with hash and status
   * @returns {Promise<boolean>} Success
   */
  async persistBlock(block) {
    return this._state.persistBlock(block);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Judgment Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Add a judgment to state
   * @param {Object} judgment - Judgment to add
   */
  addJudgment(judgment) {
    this._state.addJudgment(judgment);
  }

  /**
   * Get recent judgments
   * @param {number} [limit=100] - Max judgments to return
   * @returns {Array} Recent judgments
   */
  getRecentJudgments(limit = 100) {
    return this._state.getRecentJudgments(limit);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Knowledge Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get the knowledge tree
   * @returns {Object} Knowledge tree instance
   */
  get knowledge() {
    return this._state.knowledge;
  }

  /**
   * Add a pattern to knowledge
   * @param {Object} pattern - Pattern to add
   */
  addPattern(pattern) {
    this._state.knowledge.addPattern(pattern);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Peer State
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Add peer to state
   * @param {Object} peerInfo - Peer info
   */
  addPeer(peerInfo) {
    this._state.addPeer(peerInfo);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Storage Access
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get storage instance
   * @returns {Object} Storage
   */
  get storage() {
    return this._state.storage;
  }

  /**
   * Get data directory
   * @returns {string|null} Data directory path
   */
  get dataDir() {
    return this._dataDir;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Stats & Info
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {boolean} Whether initialized */
  get initialized() {
    return this._initialized;
  }

  /**
   * Get state summary
   * @returns {Object} Summary
   */
  getSummary() {
    return this._state.getSummary();
  }

  /**
   * Get component info
   * @returns {Object} Component info
   */
  getInfo() {
    return {
      initialized: this._initialized,
      dataDir: this._dataDir,
      summary: this.getSummary(),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Backward Compatibility
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get raw state manager
   * @returns {StateManager} State manager
   * @deprecated Use component methods instead
   */
  get manager() {
    return this._state;
  }
}

export default StateComponent;

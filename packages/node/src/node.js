/**
 * CYNIC Node
 *
 * Main node implementation - combines all protocol layers
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node
 */

'use strict';

import { EPOCH_MS, CYCLE_MS, PHI_INV } from '@cynic/core';
import {
  GossipProtocol,
  MessageType,
  createPattern,
  checkPatternFormation,
  calculateConsensus,
  ConsensusType,
} from '@cynic/protocol';
import { Operator } from './operator/operator.js';
import { CYNICJudge } from './judge/judge.js';
import { ResidualDetector } from './judge/residual.js';
import { StateManager } from './state/manager.js';

/**
 * Node status
 */
export const NodeStatus = {
  STOPPED: 'STOPPED',
  STARTING: 'STARTING',
  RUNNING: 'RUNNING',
  SYNCING: 'SYNCING',
  STOPPING: 'STOPPING',
};

/**
 * CYNIC Node - Decentralized Collective Consciousness Node
 */
export class CYNICNode {
  /**
   * Create CYNIC node
   * @param {Object} options - Node options
   * @param {string} [options.dataDir] - Data directory for persistence
   * @param {string} [options.name] - Node name
   * @param {Object} [options.identity] - Existing identity to use
   * @param {Function} [options.sendFn] - Network send function
   */
  constructor(options = {}) {
    // Initialize operator
    this.operator = new Operator({
      name: options.name,
      identity: options.identity,
    });

    // Initialize state manager
    this.state = new StateManager({
      dataDir: options.dataDir,
      operator: this.operator,
    });

    // Initialize judge
    this.judge = new CYNICJudge();

    // Initialize residual detector
    this.residualDetector = new ResidualDetector();

    // Initialize gossip protocol
    this.gossip = new GossipProtocol({
      publicKey: this.operator.publicKey,
      privateKey: this.operator.privateKey,
      address: options.address || process.env.CYNIC_ADDRESS || 'localhost',
      onMessage: this._handleMessage.bind(this),
      sendFn: options.sendFn || (async () => {}),
    });

    // Node status
    this.status = NodeStatus.STOPPED;
    this.startedAt = null;

    // Timers
    this._epochTimer = null;
    this._cycleTimer = null;

    // Pending patterns (observations before they become patterns)
    this._pendingObservations = new Map();
    this._maxPendingItems = options.maxPendingItems || 1000;
    this._maxObservationsPerItem = options.maxObservationsPerItem || 50;
  }

  /**
   * Start the node
   */
  async start() {
    if (this.status !== NodeStatus.STOPPED) {
      throw new Error(`Cannot start node in status: ${this.status}`);
    }

    this.status = NodeStatus.STARTING;

    try {
      // Initialize state
      await this.state.initialize();

      // Start timers
      this._startTimers();

      this.status = NodeStatus.RUNNING;
      this.startedAt = Date.now();

      console.log(`🐕 CYNIC Node started: ${this.operator.identity.name}`);
      console.log(`   ID: ${this.operator.id.slice(0, 16)}...`);
      console.log(`   E-Score: ${this.operator.getEScore()}`);

      return {
        success: true,
        nodeId: this.operator.id,
        name: this.operator.identity.name,
      };
    } catch (err) {
      this.status = NodeStatus.STOPPED;
      throw err;
    }
  }

  /**
   * Stop the node
   */
  async stop() {
    if (this.status !== NodeStatus.RUNNING && this.status !== NodeStatus.SYNCING) {
      return;
    }

    this.status = NodeStatus.STOPPING;

    // Stop timers
    this._stopTimers();

    // Save state
    await this.state.save();

    this.status = NodeStatus.STOPPED;
    console.log(`🐕 CYNIC Node stopped: ${this.operator.identity.name}`);
  }

  /**
   * Start epoch and cycle timers
   * @private
   */
  _startTimers() {
    // Epoch timer - checkpoint and state snapshot
    this._epochTimer = setInterval(() => {
      this._onEpoch();
    }, EPOCH_MS * 100); // Scale up for practical use

    // Cycle timer - governance check
    this._cycleTimer = setInterval(() => {
      this._onCycle();
    }, CYCLE_MS * 100); // Scale up for practical use
  }

  /**
   * Stop timers
   * @private
   */
  _stopTimers() {
    if (this._epochTimer) {
      clearInterval(this._epochTimer);
      this._epochTimer = null;
    }
    if (this._cycleTimer) {
      clearInterval(this._cycleTimer);
      this._cycleTimer = null;
    }
  }

  /**
   * Epoch handler
   * @private
   */
  _onEpoch() {
    // Update operator uptime
    const uptime = Date.now() - this.startedAt;
    this.operator.updateUptime(uptime);

    // Check for pattern emergence
    this._checkPatternEmergence();
  }

  /**
   * Cycle handler
   * @private
   */
  _onCycle() {
    // Governance and maintenance tasks
    // (Placeholder for future implementation)
  }

  /**
   * Handle incoming gossip message
   * @private
   */
  async _handleMessage(message) {
    switch (message.type) {
      case MessageType.BLOCK:
        await this._handleBlock(message.payload);
        break;
      case MessageType.JUDGMENT:
        await this._handleJudgment(message.payload);
        break;
      case MessageType.PATTERN:
        await this._handlePattern(message.payload);
        break;
    }
  }

  /**
   * Handle incoming block
   * @private
   */
  async _handleBlock(block) {
    const result = this.state.chain.importBlock(block);
    if (!result.success) {
      console.warn('Failed to import block:', result.errors);
    }
  }

  /**
   * Handle incoming judgment
   * @private
   */
  async _handleJudgment(judgment) {
    // Record observation
    const itemHash = judgment.item_hash;

    // Evict LRU entry if at capacity (before adding new key)
    if (!this._pendingObservations.has(itemHash) &&
        this._pendingObservations.size >= this._maxPendingItems) {
      let oldestKey = null;
      let oldestTime = Infinity;
      for (const [key, entry] of this._pendingObservations) {
        if (entry.lastUpdated < oldestTime) {
          oldestTime = entry.lastUpdated;
          oldestKey = key;
        }
      }
      if (oldestKey) this._pendingObservations.delete(oldestKey);
    }

    if (!this._pendingObservations.has(itemHash)) {
      this._pendingObservations.set(itemHash, { observations: [], lastUpdated: Date.now() });
    }

    const entry = this._pendingObservations.get(itemHash);
    entry.lastUpdated = Date.now();

    // Enforce per-item observation limit (FIFO eviction)
    while (entry.observations.length >= this._maxObservationsPerItem) {
      entry.observations.shift();
    }

    entry.observations.push({
      ...judgment,
      source: judgment.source || 'external',
    });
  }

  /**
   * Handle incoming pattern
   * @private
   */
  async _handlePattern(pattern) {
    // Validate and add to knowledge tree
    this.state.knowledge.addPattern(pattern);
  }

  /**
   * Check for pattern emergence from pending observations
   * @private
   */
  _checkPatternEmergence() {
    for (const [itemHash, entry] of this._pendingObservations) {
      const observations = entry.observations;
      const result = checkPatternFormation(observations);

      if (result.isPattern) {
        // Create and broadcast pattern
        const pattern = createPattern({
          content: { itemHash, observations: observations.length },
          axiom: 'VERIFY', // Default axiom
          strength: result.strength,
          sources: result.sources,
        });

        this.state.knowledge.addPattern(pattern);
        this.gossip.broadcastPattern(pattern);

        // Clear pending
        this._pendingObservations.delete(itemHash);
      }
    }
  }

  /**
   * Judge an item
   * @param {Object} item - Item to judge
   * @param {Object} [context] - Judgment context
   * @returns {Object} Judgment result
   */
  async judge(item, context = {}) {
    // Create judgment
    const judgment = this.judge.judge(item, context);

    // Analyze for residuals
    this.residualDetector.analyze(judgment, context);

    // Add to state
    this.state.addJudgment(judgment);

    // Add to chain
    const block = this.state.chain.addJudgmentBlock([judgment]);

    // Broadcast
    await this.gossip.broadcastJudgment(judgment);
    await this.gossip.broadcastBlock(block);

    // Update operator stats
    this.operator.recordJudgment();

    return judgment;
  }

  /**
   * Get node info
   * @returns {Object} Node info
   */
  getInfo() {
    return {
      id: this.operator.id,
      name: this.operator.identity.name,
      status: this.status,
      startedAt: this.startedAt,
      uptime: this.startedAt ? Date.now() - this.startedAt : 0,
      operator: this.operator.getPublicInfo(),
      state: this.state.getSummary(),
      judge: this.judge.getStats(),
      residual: this.residualDetector.getStats(),
      gossip: this.gossip.getStats(),
    };
  }

  /**
   * Add peer
   * @param {Object} peerInfo - Peer info
   */
  addPeer(peerInfo) {
    this.gossip.addPeer(peerInfo);
    this.state.addPeer(peerInfo);
  }

  /**
   * Get chain
   * @returns {PoJChain} Chain
   */
  get chain() {
    return this.state.chain;
  }

  /**
   * Get knowledge tree
   * @returns {KnowledgeTree} Knowledge tree
   */
  get knowledge() {
    return this.state.knowledge;
  }

  /**
   * Export node state
   * @returns {Object} Exportable state
   */
  async export() {
    await this.state.save();
    return {
      operator: this.operator.export(),
      stateDir: this.state.dataDir,
    };
  }

  /**
   * Create node from saved state
   * @param {Object} savedState - Saved state
   * @param {Object} [options] - Additional options
   * @returns {CYNICNode} Restored node
   */
  static async restore(savedState, options = {}) {
    const node = new CYNICNode({
      ...options,
      identity: savedState.operator?.identity,
      dataDir: savedState.stateDir || options.dataDir,
    });

    // Restore operator state
    if (savedState.operator) {
      node.operator = Operator.import(savedState.operator);
    }

    return node;
  }
}

export default { CYNICNode, NodeStatus };

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
import {
  createAnchorer,
  createAnchorQueue,
  SolanaCluster,
  AnchorStatus,
  ANCHOR_CONSTANTS,
} from '@cynic/anchor';
import { createBurnVerifier, BurnStatus } from '@cynic/burns';
import { Operator } from './operator/operator.js';
import { CYNICJudge } from './judge/judge.js';
import { ResidualDetector } from './judge/residual.js';
import { StateManager } from './state/manager.js';
import { createEmergenceLayer } from './emergence/layer.js';

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
   * @param {Object} [options.anchor] - Anchoring options
   * @param {string} [options.anchor.cluster] - Solana cluster (devnet/mainnet)
   * @param {Object} [options.anchor.wallet] - CynicWallet for signing
   * @param {boolean} [options.anchor.enabled=false] - Enable anchoring
   * @param {boolean} [options.anchor.autoAnchor=true] - Auto-anchor blocks
   * @param {Object} [options.burns] - Burns verification options
   * @param {boolean} [options.burns.enabled=false] - Require burn verification
   * @param {number} [options.burns.minAmount] - Minimum burn amount (lamports)
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

    // Initialize judge (underscore to avoid shadowing judge() method)
    this._judge = new CYNICJudge();

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

    // ═══════════════════════════════════════════════════════════════════════
    // Solana Anchoring - "Onchain is truth"
    // ═══════════════════════════════════════════════════════════════════════

    this._anchorConfig = {
      enabled: options.anchor?.enabled || false,
      autoAnchor: options.anchor?.autoAnchor ?? true,
      cluster: options.anchor?.cluster || SolanaCluster.DEVNET,
    };

    // Create anchorer (simulation mode if no wallet)
    this._anchorer = createAnchorer({
      cluster: this._anchorConfig.cluster,
      wallet: options.anchor?.wallet || null,
      onAnchor: (record) => {
        console.log(`⚓ Anchored: ${record.signature?.slice(0, 20)}...`);
      },
      onError: (record, error) => {
        console.error(`⚓ Anchor failed: ${error.message}`);
      },
    });

    // Create anchor queue
    this._anchorQueue = createAnchorQueue({
      anchorer: this._anchorer,
      batchSize: ANCHOR_CONSTANTS.ANCHOR_BATCH_SIZE, // φ-aligned: 38
      intervalMs: ANCHOR_CONSTANTS.ANCHOR_INTERVAL_MS, // φ-aligned: 61,803ms
      autoStart: false,
      onBatchReady: (batch) => {
        console.log(`⚓ Batch ready: ${batch.items.length} items`);
      },
      onAnchorComplete: (batch, result) => {
        console.log(`⚓ Batch anchored: ${result.signature?.slice(0, 20)}...`);
      },
    });

    // ═══════════════════════════════════════════════════════════════════════
    // Burns Verification - "Don't extract, burn"
    // ═══════════════════════════════════════════════════════════════════════

    this._burnsConfig = {
      enabled: options.burns?.enabled || false,
      minAmount: options.burns?.minAmount || Math.floor(PHI_INV * 1_000_000_000), // φ⁻¹ SOL default
      // Use same cluster as anchor, or allow override
      cluster: options.burns?.cluster || options.anchor?.cluster || SolanaCluster.MAINNET,
    };

    this._burnVerifier = createBurnVerifier({
      // Enable on-chain verification (preferred over external API)
      solanaCluster: this._burnsConfig.cluster,
      onVerify: (result) => {
        if (result.verified) {
          console.log(`🔥 Burn verified on-chain: ${result.amount / 1e9} SOL (${result.burnType})`);
        }
      },
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

    // ═══════════════════════════════════════════════════════════════════════
    // Emergence Layer (Layer 7) - "The crown observes all"
    // ═══════════════════════════════════════════════════════════════════════

    this._emergence = createEmergenceLayer({
      nodeId: this.operator.id,
      eScore: this.operator.getEScore(),
    });
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

      // Start anchor queue if enabled
      if (this._anchorConfig.enabled && this._anchorConfig.autoAnchor) {
        this._anchorQueue.startTimer();
        console.log(`⚓ Anchoring enabled (${this._anchorConfig.cluster})`);
      }

      // Initialize emergence layer
      this._emergence.initialize();

      this.status = NodeStatus.RUNNING;
      this.startedAt = Date.now();

      console.log(`🐕 CYNIC Node started: ${this.operator.identity.name}`);
      console.log(`   Consciousness: ${this._emergence.consciousness.state}`);
      console.log(`   ID: ${this.operator.id.slice(0, 16)}...`);
      console.log(`   E-Score: ${this.operator.getEScore()}`);
      if (this._burnsConfig.enabled) {
        console.log(`🔥 Burns verification enabled (min: ${this._burnsConfig.minAmount / 1e9} SOL)`);
      }

      return {
        success: true,
        nodeId: this.operator.id,
        name: this.operator.identity.name,
        anchoring: this._anchorConfig.enabled,
        burns: this._burnsConfig.enabled,
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

    // Stop anchor queue and flush pending
    if (this._anchorConfig.enabled) {
      this._anchorQueue.stopTimer();

      // Flush any pending anchors
      const pendingCount = this._anchorQueue.getQueueLength();
      if (pendingCount > 0) {
        console.log(`⚓ Flushing ${pendingCount} pending anchors...`);
        await this._anchorQueue.flush();
      }
    }

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

    // Update emergence layer E-Score and report to collective
    this._emergence.updateEScore(this.operator.getEScore());
    this._emergence.reportToCollective();
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
   * @param {string} [context.burnTx] - Burn transaction for verification
   * @param {string} [context.expectedBurner] - Expected burner address
   * @returns {Object} Judgment result
   */
  async judge(item, context = {}) {
    // ═══════════════════════════════════════════════════════════════════════
    // Burns Verification - "Don't extract, burn"
    // ═══════════════════════════════════════════════════════════════════════

    if (this._burnsConfig.enabled) {
      if (!context.burnTx) {
        return {
          success: false,
          error: 'Burns verification enabled - burnTx required in context',
          code: 'BURN_REQUIRED',
        };
      }

      const burnVerification = await this._burnVerifier.verify(context.burnTx, {
        minAmount: this._burnsConfig.minAmount,
        expectedBurner: context.expectedBurner,
      });

      if (!burnVerification.verified) {
        return {
          success: false,
          error: `Burn verification failed: ${burnVerification.error}`,
          code: 'BURN_INVALID',
          burnVerification,
        };
      }

      // Attach burn info to context
      context.burnVerified = true;
      context.burnAmount = burnVerification.amount;
      context.burner = burnVerification.burner;
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Create Judgment
    // ═══════════════════════════════════════════════════════════════════════

    // Create judgment
    const judgment = this._judge.judge(item, context);

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

    // ═══════════════════════════════════════════════════════════════════════
    // Emergence Layer - "The crown observes all"
    // ═══════════════════════════════════════════════════════════════════════

    this._emergence.observeJudgment(judgment);

    // ═══════════════════════════════════════════════════════════════════════
    // Solana Anchoring - "Onchain is truth"
    // ═══════════════════════════════════════════════════════════════════════

    if (this._anchorConfig.enabled) {
      // Queue judgment for anchoring
      this._anchorQueue.enqueue(judgment.id, {
        judgmentId: judgment.id,
        itemHash: judgment.item_hash,
        verdict: judgment.verdict,
        qScore: judgment.q_score,
        timestamp: judgment.timestamp,
        operator: this.operator.id,
        burnTx: context.burnTx,
      });

      judgment.anchorStatus = AnchorStatus.QUEUED;
    }

    return {
      success: true,
      judgment,
      anchored: this._anchorConfig.enabled,
      burnVerified: context.burnVerified || false,
    };
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
      judge: this._judge.getStats(),
      residual: this.residualDetector.getStats(),
      gossip: this.gossip.getStats(),
      // Solana anchoring stats
      anchor: {
        enabled: this._anchorConfig.enabled,
        cluster: this._anchorConfig.cluster,
        stats: this._anchorer.getStats(),
        queue: {
          length: this._anchorQueue.getQueueLength(),
          stats: this._anchorQueue.getStats(),
        },
      },
      // Burns verification stats
      burns: {
        enabled: this._burnsConfig.enabled,
        minAmount: this._burnsConfig.minAmount,
        stats: this._burnVerifier.getStats(),
      },
      // Emergence layer (Layer 7)
      emergence: this._emergence.getState(),
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
      // Anchor/Burns state for persistence
      anchorConfig: this._anchorConfig,
      anchorStats: this._anchorer.getStats(),
      anchorQueueState: this._anchorQueue.export?.() || null,
      burnsConfig: this._burnsConfig,
      burnsState: this._burnVerifier.export(),
      // Emergence state
      emergenceState: this._emergence.export(),
    };
  }

  /**
   * Create node from saved state
   * @param {Object} savedState - Saved state
   * @param {Object} [options] - Additional options
   * @returns {CYNICNode} Restored node
   */
  static async restore(savedState, options = {}) {
    // Merge saved anchor/burns config with options (options take precedence)
    const anchorOptions = {
      ...(savedState.anchorConfig || {}),
      ...(options.anchor || {}),
    };

    const burnsOptions = {
      ...(savedState.burnsConfig || {}),
      ...(options.burns || {}),
    };

    const node = new CYNICNode({
      ...options,
      identity: savedState.operator?.identity,
      dataDir: savedState.stateDir || options.dataDir,
      anchor: anchorOptions,
      burns: burnsOptions,
    });

    // Restore operator state
    if (savedState.operator) {
      node.operator = Operator.import(savedState.operator);
    }

    // Restore burn verifier cache (if available)
    if (savedState.burnsState) {
      node._burnVerifier.import(savedState.burnsState);
    }

    // Restore emergence state
    if (savedState.emergenceState) {
      node._emergence.import(savedState.emergenceState);
    }

    return node;
  }

  /**
   * Get anchor queue for external access
   * @returns {Object} Anchor queue
   */
  get anchorQueue() {
    return this._anchorQueue;
  }

  /**
   * Get anchorer for external access
   * @returns {Object} Anchorer
   */
  get anchorer() {
    return this._anchorer;
  }

  /**
   * Get burn verifier for external access
   * @returns {Object} Burn verifier
   */
  get burnVerifier() {
    return this._burnVerifier;
  }

  /**
   * Check if a burn has been verified
   * @param {string} burnTx - Burn transaction signature
   * @returns {boolean} Whether burn is verified
   */
  isBurnVerified(burnTx) {
    return this._burnVerifier.isVerified(burnTx);
  }

  /**
   * Get emergence layer for external access
   * @returns {EmergenceLayer} Emergence layer
   */
  get emergence() {
    return this._emergence;
  }

  /**
   * Get consciousness state
   * @returns {string} Current consciousness state
   */
  get consciousnessState() {
    return this._emergence.consciousness.state;
  }

  /**
   * Get collective phase
   * @returns {string} Current collective phase
   */
  get collectivePhase() {
    return this._emergence.collective.phase;
  }
}

export default { CYNICNode, NodeStatus };

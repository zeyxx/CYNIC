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

import { EPOCH_MS, CYCLE_MS, PHI_INV, createLogger } from '@cynic/core';
import {
  GossipProtocol,
  MessageType,
  createPattern,
  checkPatternFormation,
  calculateConsensus,
  ConsensusType,
  // Layer 4: φ-BFT Consensus
  ConsensusEngine,
  ConsensusState,
  BlockStatus,
  ConsensusGossip,
  // Block creation
  createJudgmentBlock,
} from '@cynic/protocol';
import {
  createAnchorer,
  createAnchorQueue,
  SolanaCluster,
  AnchorStatus,
  ANCHOR_CONSTANTS,
} from '@cynic/anchor';
import { createBurnVerifier, BurnStatus } from '@cynic/burns';
import { EScoreHistoryRepository, getPool } from '@cynic/persistence';
import { Operator } from './operator/operator.js';
import { CYNICJudge } from './judge/judge.js';
import { ResidualDetector } from './judge/residual.js';
import { LearningService } from './judge/learning-service.js';
import { StateManager } from './state/manager.js';
import { createEmergenceLayer } from './emergence/layer.js';
import { SharedMemory } from './memory/shared-memory.js';
import { UserLab, LabManager } from './memory/user-lab.js';
import { DogOrchestrator, DogMode } from './agents/orchestrator.js';
import { WebSocketTransport, ConnectionState } from './transport/websocket.js';
// Collective consciousness - The 11 Dogs
import {
  AgentEventBus,
  AgentEvent,
  EventPriority,
  CollectivePack,
  createCollectivePack,
} from './agents/index.js';

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
   * @param {Object} [options.transport] - P2P transport options
   * @param {boolean} [options.transport.enabled=true] - Enable P2P transport
   * @param {number} [options.transport.port=8618] - WebSocket port (φ-aligned)
   * @param {string} [options.transport.host='0.0.0.0'] - Listen host
   * @param {Object} [options.transport.ssl] - SSL config for WSS
   */
  constructor(options = {}) {
    // Structured logger
    this._log = createLogger('CYNICNode');

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

    // ═══════════════════════════════════════════════════════════════════════
    // Persistence Layer - "Memory makes wisdom possible"
    // ═══════════════════════════════════════════════════════════════════════

    this._persistence = null;
    if (options.persistence?.enabled !== false) {
      try {
        const pool = getPool();
        this._persistence = {
          escoreHistory: new EScoreHistoryRepository(pool),
          // Add other repositories as needed
        };
      } catch (e) {
        // Database not available - persistence disabled
        this._log.warn('Persistence unavailable - running in-memory only');
      }
    }

    // Initialize judge (underscore to avoid shadowing judge() method)
    this._judge = new CYNICJudge();

    // Initialize residual detector
    this.residualDetector = new ResidualDetector();

    // ═══════════════════════════════════════════════════════════════════════
    // P2P Transport Layer - "Nodes talk to nodes"
    // ═══════════════════════════════════════════════════════════════════════

    this._transportConfig = {
      enabled: options.transport?.enabled ?? true, // Enabled by default
      port: options.transport?.port || 8618, // φ-aligned: 8×φ×1000
      host: options.transport?.host || '0.0.0.0',
      ssl: options.transport?.ssl || null,
    };

    // Initialize WebSocket transport (if enabled)
    if (this._transportConfig.enabled) {
      this._transport = new WebSocketTransport({
        port: this._transportConfig.port,
        host: this._transportConfig.host,
        publicKey: this.operator.publicKey,
        privateKey: this.operator.privateKey,
        ssl: this._transportConfig.ssl,
      });

      // Wire transport events
      this._setupTransportHandlers();
    } else {
      this._transport = null;
    }

    // Initialize gossip protocol with real transport sendFn
    this.gossip = new GossipProtocol({
      publicKey: this.operator.publicKey,
      privateKey: this.operator.privateKey,
      address: options.address || process.env.CYNIC_ADDRESS || `${this._transportConfig.host}:${this._transportConfig.port}`,
      onMessage: this._handleMessage.bind(this),
      sendFn: this._transport ? this._transport.getSendFn() : (options.sendFn || (async () => {})),
    });

    // ═══════════════════════════════════════════════════════════════════════
    // Layer 4: φ-BFT Consensus - "61.8% supermajority"
    // ═══════════════════════════════════════════════════════════════════════

    this._consensusConfig = {
      enabled: options.consensus?.enabled ?? true, // Enabled by default
      confirmationsForFinality: options.consensus?.confirmations || 32,
    };

    // Initialize consensus engine
    this._consensus = new ConsensusEngine({
      publicKey: this.operator.publicKey,
      privateKey: this.operator.privateKey,
      eScore: this.operator.getEScore(),
      burned: this.operator.burned || 0,
      confirmationsForFinality: this._consensusConfig.confirmationsForFinality,
    });

    // Initialize consensus-gossip bridge
    this._consensusGossip = new ConsensusGossip({
      consensus: this._consensus,
      gossip: this.gossip,
      autoSync: true,
    });

    // Wire consensus events to state updates
    this._setupConsensusHandlers();

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
        this._log.info('Anchored', { signature: record.signature?.slice(0, 20) });
      },
      onError: (record, error) => {
        this._log.error('Anchor failed', { error: error.message, record: record?.id });
      },
    });

    // Create anchor queue
    this._anchorQueue = createAnchorQueue({
      anchorer: this._anchorer,
      batchSize: ANCHOR_CONSTANTS.ANCHOR_BATCH_SIZE, // φ-aligned: 38
      intervalMs: ANCHOR_CONSTANTS.ANCHOR_INTERVAL_MS, // φ-aligned: 61,803ms
      autoStart: false,
      onBatchReady: (batch) => {
        this._log.debug('Anchor batch ready', { items: batch.items.length });
      },
      onAnchorComplete: (batch, result) => {
        this._log.info('Batch anchored', { signature: result.signature?.slice(0, 20), items: batch.items?.length });
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
      onVerify: async (result) => {
        if (result.verified && result.amount > 0) {
          this._log.info('Burn verified on-chain', { amount: result.amount / 1e9, burnType: result.burnType });

          // ═══════════════════════════════════════════════════════════════════
          // Burns → E-Score (automatic wiring)
          // "Don't extract, burn" - verified burns automatically update E-Score
          // ═══════════════════════════════════════════════════════════════════
          this.operator.recordBurn(result.amount, result.burnType || 'verified_onchain');

          // ═══════════════════════════════════════════════════════════════════
          // Burns → E-Score → Persistence (automatic recording)
          // "Memory makes wisdom possible" - persist E-Score snapshots
          // ═══════════════════════════════════════════════════════════════════
          await this._recordEScoreSnapshot(`burn_${result.burnType || 'verified'}`);
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

    // ═══════════════════════════════════════════════════════════════════════
    // 6-Layer Memory Architecture - "Fresh execution, collective wisdom"
    // ═══════════════════════════════════════════════════════════════════════

    // Layer 2-3: SharedMemory (Collective + Procedural)
    this._sharedMemory = new SharedMemory({
      storage: this.state.storage,
    });

    // Layer 4: User Labs
    this._labManager = new LabManager({
      dataDir: options.dataDir ? `${options.dataDir}/labs` : null,
    });

    // RLHF Learning Service
    this._learningService = new LearningService({
      persistence: this._persistence,
      learningRate: 0.236, // φ⁻³
    });

    // Set learning service on judge
    this._judge.setLearningService(this._learningService);

    // Dog Orchestrator (parallel subagents with hybrid context)
    this._orchestrator = new DogOrchestrator({
      sharedMemory: this._sharedMemory,
      mode: options.orchestratorMode || DogMode.PARALLEL,
    });

    // ═══════════════════════════════════════════════════════════════════════
    // Collective Consciousness - The 11 Dogs (Gap #1 Fix)
    // "The pack sees what the individual misses"
    // ═══════════════════════════════════════════════════════════════════════

    // Event bus for inter-agent communication
    this._eventBus = new AgentEventBus({
      nodeId: this.operator.id,
    });

    // The collective pack (11 dogs)
    this._collectivePack = createCollectivePack({
      eventBus: this._eventBus,
      sharedMemory: this._sharedMemory,
      nodeId: this.operator.id,
    });

    // Wire event bus to learning service (collective feedback loop)
    this._wireCollectiveToLearning();
  }

  /**
   * Wire collective consciousness to learning service (Gap #1)
   *
   * When the collective reaches consensus on a judgment, that feedback
   * is sent to the learning service to improve future judgments.
   *
   * @private
   */
  _wireCollectiveToLearning() {
    // Listen for collective consensus on judgments
    this._eventBus.on('consensus:reached', async (event) => {
      const { vote, confidence, voters, context } = event;

      // Only process if it's about a judgment
      if (context?.type !== 'judgment') return;

      this._log.info('Collective consensus', { vote, confidence: Math.round(confidence * 100), voters: voters?.length || 0 });

      // Convert collective vote to feedback
      const isCorrect = vote === 'APPROVE' || vote === 'WAG';
      const feedbackType = isCorrect ? 'positive' : 'negative';

      // Process as collective feedback
      this._learningService.processFeedback({
        judgmentId: context.judgmentId,
        feedbackType,
        source: 'collective_consensus',
        confidence,
        details: {
          vote,
          voters,
          dogCount: voters?.length || 0,
        },
      });
    });

    // Listen for individual dog insights
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
    });
  }

  /**
   * Setup consensus event handlers
   * @private
   */
  _setupConsensusHandlers() {
    // When a block is finalized by consensus, add it to state chain and persist
    this._consensus.on('block:finalized', async (event) => {
      const { blockHash, slot, block } = event;
      this._log.info('Block finalized', { slot, hash: blockHash.slice(0, 16) });

      // Import finalized block to state chain (in-memory)
      if (block && block.judgments) {
        const result = this.state.chain.importBlock(block);
        if (!result.success) {
          this._log.warn('Failed to import finalized block', { errors: result.errors });
        }

        // Persist block to storage (Gap #3 - persistent PoJ)
        const persisted = await this.state.persistBlock({
          ...block,
          hash: blockHash,
          status: 'FINALIZED',
        });
        if (persisted) {
          this._log.debug('Block persisted to storage', { slot });
        }

        // Queue for Solana anchoring if enabled (Gap #5)
        if (this._anchorConfig?.enabled && this._anchorQueue) {
          this._anchorQueue.enqueue(`poj_block_${slot}`, {
            type: 'poj_block',
            slot,
            hash: blockHash,
            merkleRoot: block.merkleRoot || block.stateRoot,
            judgmentCount: block.judgments?.length || 0,
            timestamp: block.timestamp,
          });
          this._log.debug('Block queued for Solana anchoring', { slot });
        }
      }

      // Emit for external listeners
      this.emit?.('block:finalized', event);
    });

    // When a block is confirmed (but not yet finalized)
    this._consensus.on('block:confirmed', (event) => {
      this._log.debug('Block confirmed', { slot: event.slot, ratio: (event.ratio * 100).toFixed(1) });
    });

    // When consensus starts
    this._consensus.on('consensus:started', (event) => {
      this._log.info('Consensus started', { slot: event.slot });
    });

    // When a slot changes
    this._consensus.on('slot:change', (event) => {
      // Periodic status update every 100 slots
      if (event.currentSlot % 100 === 0) {
        const stats = this._consensus.getStats();
        this._log.debug('Slot status', { slot: event.currentSlot, finalized: stats.blocksFinalized, pending: stats.pendingBlocks });
      }
    });
  }

  /**
   * Setup P2P transport event handlers
   * @private
   */
  _setupTransportHandlers() {
    if (!this._transport) return;

    // When a peer connects
    this._transport.on('peer:connected', ({ peerId, publicKey, address }) => {
      this._log.info('Peer connected', { peerId: peerId.slice(0, 16), address });

      // Add to gossip peer manager
      this.gossip.addPeer({
        id: peerId,
        publicKey: publicKey || peerId,
        address,
      });

      // Register as validator for consensus (with default E-Score)
      // Real E-Score can be updated later via registerValidator()
      if (this._consensusConfig.enabled) {
        this._consensus.registerValidator({
          publicKey: publicKey || peerId,
          eScore: 50, // Default E-Score for new peers
          burned: 0,
          uptime: 1.0,
        });
      }

      this.emit?.('peer:connected', { peerId, publicKey, address });
    });

    // When a peer disconnects
    this._transport.on('peer:disconnected', ({ peerId, code, reason }) => {
      this._log.info('Peer disconnected', { peerId: peerId.slice(0, 16), reason: reason || code });

      // Remove from consensus validators
      if (this._consensusConfig.enabled) {
        this._consensus.removeValidator(peerId);
      }

      this.emit?.('peer:disconnected', { peerId, code, reason });
    });

    // When a message is received from transport
    this._transport.on('message', ({ message, peerId }) => {
      // Route to gossip protocol for processing
      this.gossip.handleMessage(message, peerId);
    });

    // When server starts listening
    this._transport.on('server:listening', ({ port, host, secure }) => {
      this._log.info('P2P listening', { protocol: secure ? 'wss' : 'ws', host, port });
    });

    // When peer identity is verified
    this._transport.on('peer:identified', ({ peerId, publicKey, address }) => {
      this._log.debug('Peer identified', { peerId: peerId.slice(0, 16), address });
    });

    // Handle transport errors
    this._transport.on('peer:error', ({ peerId, error }) => {
      this._log.warn('Peer error', { peerId: peerId?.slice(0, 16), error: error.message });
    });

    this._transport.on('server:error', (error) => {
      this._log.error('Transport server error', { error: error.message });
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

      // ═══════════════════════════════════════════════════════════════════════
      // Start P2P Transport Server
      // ═══════════════════════════════════════════════════════════════════════

      if (this._transportConfig.enabled && this._transport) {
        await this._transport.startServer();
        this._log.info('P2P transport enabled', { port: this._transportConfig.port });
      }

      // ═══════════════════════════════════════════════════════════════════════
      // Start Layer 4: φ-BFT Consensus
      // ═══════════════════════════════════════════════════════════════════════

      if (this._consensusConfig.enabled) {
        // Register self as validator
        this._consensus.registerValidator({
          publicKey: this.operator.publicKey,
          eScore: this.operator.getEScore(),
          burned: this.operator.burned || 0,
          uptime: 1.0,
        });

        // Start consensus engine
        this._consensus.start();

        // Start consensus-gossip bridge
        this._consensusGossip.start();

        this._log.info('Consensus enabled', { supermajority: '61.8%', confirmations: this._consensusConfig.confirmationsForFinality });
      }

      // Start anchor queue if enabled
      if (this._anchorConfig.enabled && this._anchorConfig.autoAnchor) {
        this._anchorQueue.startTimer();
        this._log.info('Anchoring enabled', { cluster: this._anchorConfig.cluster });
      }

      // Initialize emergence layer
      this._emergence.initialize();

      // ═══════════════════════════════════════════════════════════════════════
      // Initialize 6-Layer Memory Architecture
      // ═══════════════════════════════════════════════════════════════════════

      await this._sharedMemory.initialize();
      await this._learningService.init();

      // Wire emergence patterns → SharedMemory (Gap #4 feedback loop)
      this._wireEmergenceFeedback();

      this._log.debug('Memory layers initialized', { architecture: '6-layer hybrid' });

      this.status = NodeStatus.RUNNING;
      this.startedAt = Date.now();

      this._log.info('CYNIC Node started', {
        name: this.operator.identity.name,
        id: this.operator.id.slice(0, 16),
        consciousness: this._emergence.consciousness.state,
        eScore: this.operator.getEScore(),
        burns: this._burnsConfig.enabled ? { minAmount: this._burnsConfig.minAmount / 1e9 } : false,
      });

      // ═══════════════════════════════════════════════════════════════════════
      // Judgment Sync on Startup (Gap #10 fix)
      // Request missed judgments from peers after connection stabilizes
      // ═══════════════════════════════════════════════════════════════════════
      if (this._transportConfig.enabled) {
        // Delay sync to allow peer connections to establish (φ-aligned: 8s ≈ F₆)
        setTimeout(async () => {
          try {
            await this.syncJudgmentsFromPeers();
            this._log.debug('Judgment sync requested from peers');
          } catch (err) {
            this._log.warn('Judgment sync failed', { error: err.message });
          }
        }, 8000);
      }

      return {
        success: true,
        nodeId: this.operator.id,
        name: this.operator.identity.name,
        anchoring: this._anchorConfig.enabled,
        burns: this._burnsConfig.enabled,
        consensus: this._consensusConfig.enabled,
        transport: this._transportConfig.enabled ? {
          port: this._transportConfig.port,
          host: this._transportConfig.host,
          secure: !!this._transportConfig.ssl,
        } : null,
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

    // Stop consensus
    if (this._consensusConfig.enabled) {
      this._consensusGossip.stop();
      this._consensus.stop();
      this._log.info('Consensus stopped');
    }

    // Stop P2P transport
    if (this._transportConfig.enabled && this._transport) {
      await this._transport.stopServer();
      this._log.info('P2P transport stopped');
    }

    // Stop anchor queue and flush pending
    if (this._anchorConfig.enabled) {
      this._anchorQueue.stopTimer();

      // Flush any pending anchors
      const pendingCount = this._anchorQueue.getQueueLength();
      if (pendingCount > 0) {
        this._log.debug('Flushing pending anchors', { count: pendingCount });
        await this._anchorQueue.flush();
      }
    }

    // Save 6-layer memory state
    if (this._sharedMemory) {
      await this._sharedMemory.save();
      this._log.debug('SharedMemory saved');
    }
    if (this._labManager) {
      await this._labManager.saveAll();
    }

    // Save state
    await this.state.save();

    this.status = NodeStatus.STOPPED;
    this._log.info('CYNIC Node stopped', { name: this.operator.identity.name });
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
   * Wire emergence layer feedback to SharedMemory and LearningService
   * This closes the feedback loop: Emergence → SharedMemory → Judge → Better Judgments
   * @private
   */
  _wireEmergenceFeedback() {
    // 1. When emergence detects a significant pattern, add it to SharedMemory
    // Check patterns on each epoch and sync significant ones
    const originalOnEpoch = this._onEpoch.bind(this);
    this._onEpoch = () => {
      originalOnEpoch();
      this._syncEmergencePatternsToMemory();
    };

    // 2. When feedback is processed, sync to SharedMemory
    this._learningService.on('feedback-processed', (result) => {
      // Note: result contains { scoreDelta, queueSize, shouldLearn }
      // We'll sync after learning completes for weight adjustments
    });

    // 3. When learning completes, sync weight changes to SharedMemory
    this._learningService.on('learning-complete', (result) => {
      // Sync weight modifiers to SharedMemory
      if (result.weightChanges) {
        for (const [dimension, change] of Object.entries(result.weightChanges)) {
          this._sharedMemory.adjustWeight(dimension, change.delta || 0, 'rlhf');
        }
      }

      // Record as feedback
      this._sharedMemory.recordFeedback({
        type: 'learning_cycle',
        iteration: result.iteration,
        source: 'learning_service',
      });
    });

    // 4. When a new learning entry is added, consider adding as pattern
    this._learningService.on('learning-added', (entry) => {
      // Add significant learnings as patterns
      if (entry.type === 'bias_correction' || entry.type === 'pattern') {
        this._sharedMemory.addPattern({
          id: `learn_${entry.id || Date.now().toString(36)}`,
          rule: entry.insight || entry.description || `Learning: ${entry.type}`,
          applicableTo: entry.applicableTo || ['*'],
          confidence: Math.min(entry.confidence || 0.6, 0.618),
          source: 'rlhf',
          verified: false,
        });
      }
    });
  }

  /**
   * Sync significant emergence patterns to SharedMemory
   * @private
   */
  _syncEmergencePatternsToMemory() {
    if (!this._emergence || !this._sharedMemory) return;

    const topPatterns = this._emergence.patterns.getTopPatterns(10);

    for (const pattern of topPatterns) {
      // Only sync significant patterns not already in SharedMemory
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

    // ═══════════════════════════════════════════════════════════════════════
    // Periodic E-Score snapshot (every hour ~ 36 epochs at 100s scaled)
    // "φ-aligned retention: hourly for 24h, daily for 30d"
    // ═══════════════════════════════════════════════════════════════════════
    this._epochCount = (this._epochCount || 0) + 1;
    if (this._persistence?.escoreHistory && this._epochCount % 36 === 0) {
      this._recordEScoreSnapshot('epoch_periodic').catch(() => {});
    }
  }

  /**
   * Record E-Score snapshot to persistence
   * @param {string} trigger - What triggered the snapshot
   * @private
   */
  async _recordEScoreSnapshot(trigger = 'manual') {
    if (!this._persistence?.escoreHistory) return;

    try {
      const breakdown = this.operator.getEScoreBreakdown();
      await this._persistence.escoreHistory.recordSnapshot(
        this.operator.id,
        this.operator.getEScore(),
        {
          hold: breakdown.dimensions?.find(d => d.dimension === 'HOLD')?.score || 0,
          burn: breakdown.dimensions?.find(d => d.dimension === 'BURN')?.score || 0,
          use: breakdown.dimensions?.find(d => d.dimension === 'USE')?.score || 0,
          build: breakdown.dimensions?.find(d => d.dimension === 'BUILD')?.score || 0,
          run: breakdown.dimensions?.find(d => d.dimension === 'RUN')?.score || 0,
          refer: breakdown.dimensions?.find(d => d.dimension === 'REFER')?.score || 0,
          time: breakdown.dimensions?.find(d => d.dimension === 'TIME')?.score || 0,
        },
        trigger
      );
    } catch (e) {
      // Non-fatal
      this._log.warn('E-Score history persistence failed', { error: e.message });
    }
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
      case MessageType.JUDGMENT_SYNC_REQUEST:
        await this._handleJudgmentSyncRequest(message);
        break;
      case MessageType.JUDGMENT_SYNC_RESPONSE:
        await this._handleJudgmentSyncResponse(message.payload);
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
      this._log.warn('Failed to import block', { errors: result.errors });
    }
  }

  /**
   * Handle incoming judgment from peer
   * @private
   */
  async _handleJudgment(judgment) {
    // ═══════════════════════════════════════════════════════════════════════
    // CRITICAL FIX: Store received judgments (Gap #9)
    // Previously only buffered for pattern detection, now persisted
    // ═══════════════════════════════════════════════════════════════════════

    // Check for duplicate (avoid storing same judgment twice)
    const existingJudgments = this.state.getRecentJudgments(1000);
    const isDuplicate = existingJudgments.some(j => j.id === judgment.id);

    if (!isDuplicate) {
      // Store in local state (same as self-created judgments)
      this.state.addJudgment(judgment);

      // Observe in emergence layer
      this._emergence?.observeJudgment?.(judgment);

      // Emit event for collective awareness
      if (this._eventBus) {
        this._eventBus.emit('judgment:received', {
          judgmentId: judgment.id,
          verdict: judgment.verdict,
          qScore: judgment.q_score,
          source: 'peer',
          timestamp: Date.now(),
        });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Pattern detection (existing logic)
    // ═══════════════════════════════════════════════════════════════════════

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

  // ═══════════════════════════════════════════════════════════════════════════
  // Judgment Sync Handlers (Gap #10 fix)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Handle judgment sync request from peer
   * @private
   */
  async _handleJudgmentSyncRequest(message) {
    const { since_timestamp, limit } = message.payload;
    const requestId = message.id;
    const requesterKey = message.sender;

    // Get judgments since timestamp
    const allJudgments = this.state.getRecentJudgments(1000);
    const filteredJudgments = allJudgments
      .filter(j => (j.timestamp || 0) > (since_timestamp || 0))
      .slice(0, limit || 100);

    const hasMore = filteredJudgments.length >= (limit || 100);

    // Create and send response
    const { createJudgmentSyncResponse } = await import('@cynic/protocol/gossip/message.js');
    const response = createJudgmentSyncResponse(
      filteredJudgments,
      requestId,
      hasMore,
      this.operator.publicKey,
      this.operator.privateKey
    );

    // Send directly to requester
    await this.gossip.sendTo(requesterKey, response);
  }

  /**
   * Handle judgment sync response from peer
   * @private
   */
  async _handleJudgmentSyncResponse(payload) {
    const { judgments, request_id, has_more, count } = payload;

    if (!judgments || !Array.isArray(judgments)) return;

    let imported = 0;
    for (const judgment of judgments) {
      // Use existing handler to store (handles dedup)
      await this._handleJudgment(judgment);
      imported++;
    }

    // Emit sync progress event
    if (this._eventBus) {
      this._eventBus.emit('judgment:sync:progress', {
        requestId: request_id,
        received: count,
        imported,
        hasMore: has_more,
        timestamp: Date.now(),
      });
    }

    // Store last sync state for continuation
    if (judgments.length > 0) {
      const lastTimestamp = Math.max(...judgments.map(j => j.timestamp || 0));
      this._lastJudgmentSyncTimestamp = lastTimestamp;
    }
  }

  /**
   * Request judgment sync from a peer
   * @param {string} peerId - Peer to request from
   * @param {number} [sinceTimestamp=0] - Request judgments since this timestamp
   * @param {number} [limit=100] - Max judgments to request
   * @returns {Promise<void>}
   */
  async requestJudgmentSync(peerId, sinceTimestamp = 0, limit = 100) {
    const { createJudgmentSyncRequest } = await import('@cynic/protocol/gossip/message.js');
    const request = createJudgmentSyncRequest(
      sinceTimestamp,
      limit,
      this.operator.publicKey,
      this.operator.privateKey
    );

    await this.gossip.sendTo(peerId, request);
  }

  /**
   * Sync judgments from all connected peers
   * Called on node startup to catch up with network
   * @returns {Promise<void>}
   */
  async syncJudgmentsFromPeers() {
    const peers = this.gossip.getActivePeers?.() || [];
    if (peers.length === 0) return;

    // Get oldest judgment timestamp we have (or 0 if none)
    const existingJudgments = this.state.getRecentJudgments(1);
    const sinceTimestamp = existingJudgments.length > 0
      ? Math.min(...existingJudgments.map(j => j.timestamp || 0)) - 1
      : 0;

    // Request from first 3 peers (φ-aligned redundancy)
    const syncPeers = peers.slice(0, 3);
    for (const peer of syncPeers) {
      try {
        await this.requestJudgmentSync(peer.id || peer.publicKey, sinceTimestamp, 100);
      } catch (err) {
        this._log.warn('Judgment sync request failed', { peerId: peer.id, error: err.message });
      }
    }
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
    const residualAnalysis = this.residualDetector.analyze(judgment, context);

    // ═══════════════════════════════════════════════════════════════════════
    // Gap #2 Fix: Anomalies → Learning Service
    // "When the dog notices something strange, it learns from it"
    // ═══════════════════════════════════════════════════════════════════════

    if (residualAnalysis.isAnomaly) {
      // Feed anomaly signal to learning service
      this._learningService.processAnomalySignal?.({
        judgmentId: judgment.id,
        residual: residualAnalysis.residual,
        threshold: this.residualDetector.threshold,
        dimensions: judgment.dimensions,
        verdict: judgment.verdict,
        qScore: judgment.q_score || judgment.global_score,
        source: 'residual_detector',
        timestamp: Date.now(),
      });

      // Also add to shared memory as a pattern to watch
      this._sharedMemory?.addPattern?.({
        type: 'anomaly_detected',
        judgmentId: judgment.id,
        residual: residualAnalysis.residual,
        itemType: item?.type,
        timestamp: Date.now(),
      });
    }

    // Add to local state (optimistic - will be confirmed by consensus)
    this.state.addJudgment(judgment);

    // ═══════════════════════════════════════════════════════════════════════
    // Layer 4: φ-BFT Consensus Block Production
    // ═══════════════════════════════════════════════════════════════════════

    let block;
    let consensusRecord = null;

    if (this._consensusConfig.enabled && this._consensus.state === ConsensusState.PARTICIPATING) {
      // Create block for consensus proposal
      const currentSlot = this._consensus.getCurrentSlot();
      block = createJudgmentBlock({
        judgments: [judgment],
        previousHash: this.state.chain.latestHash,
        timestamp: Date.now(),
        slot: currentSlot,
        proposer: this.operator.publicKey,
      });

      // Propose block to consensus (will be broadcast via ConsensusGossip bridge)
      try {
        consensusRecord = this._consensus.proposeBlock(block);

        judgment.consensusStatus = 'PROPOSED';
        judgment.consensusSlot = consensusRecord.slot;
        judgment.consensusBlockHash = consensusRecord.hash;
      } catch (err) {
        // Consensus might not be ready, fall back to direct broadcast
        this._log.warn('Consensus proposal failed, falling back to direct broadcast', { error: err.message });
        block = this.state.chain.addJudgmentBlock([judgment]);
        await this.gossip.broadcastJudgment(judgment);
        await this.gossip.broadcastBlock(block);
      }
    } else {
      // Consensus disabled or not participating - direct add to chain
      block = this.state.chain.addJudgmentBlock([judgment]);

      // Broadcast directly
      await this.gossip.broadcastJudgment(judgment);
      await this.gossip.broadcastBlock(block);
    }

    // Update operator stats
    this.operator.recordJudgment();

    // ═══════════════════════════════════════════════════════════════════════
    // Emergence Layer - "The crown observes all"
    // ═══════════════════════════════════════════════════════════════════════

    this._emergence.observeJudgment(judgment);

    // ═══════════════════════════════════════════════════════════════════════
    // Collective Consciousness - "The pack reviews the judgment" (Gap #1)
    // ═══════════════════════════════════════════════════════════════════════

    // Publish judgment to event bus for collective review
    if (this._eventBus && this._collectivePack) {
      this._eventBus.emit('judgment:created', {
        type: 'judgment',
        judgmentId: judgment.id,
        verdict: judgment.verdict,
        qScore: judgment.q_score || judgment.global_score,
        confidence: judgment.confidence,
        item: {
          type: item?.type,
          identifier: item?.identifier || item?.name,
        },
        timestamp: Date.now(),
      });

      // Request collective consensus (async, non-blocking)
      this._collectivePack.reviewJudgment?.(judgment, item).catch((err) => {
        // Non-critical - log but don't fail the judgment
        this._log.trace('Collective review skipped', { error: err.message });
      });
    }

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
      consensus: consensusRecord ? {
        status: consensusRecord.status,
        slot: consensusRecord.slot,
        blockHash: consensusRecord.hash,
      } : null,
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
      // Layer 4: φ-BFT Consensus stats
      consensus: {
        enabled: this._consensusConfig.enabled,
        state: this._consensus.state,
        stats: this._consensus.getStats(),
        validators: this._consensus.validators.size,
      },
      // P2P Transport stats
      transport: {
        enabled: this._transportConfig.enabled,
        port: this._transportConfig.port,
        secure: !!this._transportConfig.ssl,
        stats: this._transport?.getStats() || null,
        connectedPeers: this._transport?.getConnectedPeers()?.length || 0,
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
   * Connect to a remote peer via P2P transport
   * @param {Object} peer - Peer info
   * @param {string} peer.address - Peer address (ws://host:port or host:port)
   * @param {string} [peer.id] - Peer ID (optional, will be discovered via identity exchange)
   * @returns {Promise<void>}
   */
  async connectToPeer(peer) {
    if (!this._transport) {
      throw new Error('P2P transport not enabled');
    }

    if (!peer.address) {
      throw new Error('Peer address required');
    }

    // Connect via WebSocket transport
    // Identity exchange will happen automatically
    // Peer will be added to gossip/consensus via transport event handlers
    await this._transport.connect({
      id: peer.id || peer.address,
      address: peer.address,
    });
  }

  /**
   * Get list of connected peers
   * @returns {string[]} Connected peer IDs
   */
  getConnectedPeers() {
    if (!this._transport) {
      return [];
    }
    return this._transport.getConnectedPeers();
  }

  /**
   * Check if connected to a specific peer
   * @param {string} peerId - Peer ID
   * @returns {boolean} True if connected
   */
  isConnectedToPeer(peerId) {
    if (!this._transport) {
      return false;
    }
    return this._transport.isConnected(peerId);
  }

  /**
   * Disconnect from a peer
   * @param {string} peerId - Peer ID to disconnect
   */
  disconnectPeer(peerId) {
    if (this._transport) {
      this._transport.disconnect(peerId);
    }
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

  /**
   * Get consensus engine for external access
   * @returns {ConsensusEngine} Consensus engine
   */
  get consensus() {
    return this._consensus;
  }

  /**
   * Get consensus-gossip bridge for external access
   * @returns {ConsensusGossip} Consensus-gossip bridge
   */
  get consensusGossip() {
    return this._consensusGossip;
  }

  /**
   * Get P2P transport for external access
   * @returns {WebSocketTransport|null} Transport or null if disabled
   */
  get transport() {
    return this._transport;
  }

  /**
   * Get current consensus slot
   * @returns {number} Current slot
   */
  get currentSlot() {
    return this._consensus.getCurrentSlot();
  }

  /**
   * Get last finalized slot
   * @returns {number} Last finalized slot
   */
  get lastFinalizedSlot() {
    return this._consensus.lastFinalizedSlot;
  }

  /**
   * Check if a block is finalized
   * @param {string} blockHash - Block hash
   * @returns {boolean} True if finalized
   */
  isBlockFinalized(blockHash) {
    return this._consensus.isFinalized(blockHash);
  }

  /**
   * Register a peer as validator for consensus
   * @param {Object} peerInfo - Peer info
   * @param {string} peerInfo.publicKey - Peer public key
   * @param {number} [peerInfo.eScore=50] - Peer E-Score
   * @param {number} [peerInfo.burned=0] - Peer burned amount
   */
  registerValidator(peerInfo) {
    this._consensus.registerValidator({
      publicKey: peerInfo.publicKey,
      eScore: peerInfo.eScore || 50,
      burned: peerInfo.burned || 0,
      uptime: peerInfo.uptime || 1.0,
    });
  }
}

export default { CYNICNode, NodeStatus };

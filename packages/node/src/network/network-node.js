/**
 * CYNICNetworkNode - Multi-Node Orchestrator
 *
 * PHASE 2: DECENTRALIZE
 *
 * Wires together all P2P components:
 * - TransportComponent: WebSocket/gossip networking
 * - ConsensusComponent: φ-BFT consensus (61.8% supermajority)
 * - PeerDiscovery: Finding and connecting to peers
 * - StateSync: Synchronizing state between nodes
 *
 * "The pack hunts together" - κυνικός
 *
 * @module @cynic/node/network/network-node
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, globalEventBus, EventType } from '@cynic/core';
import { TransportComponent } from '../components/transport-component.js';
import { ConsensusComponent } from '../components/consensus-component.js';
import { PeerDiscovery, DiscoveryState } from '../transport/discovery.js';

const log = createLogger('CYNICNetworkNode');

/**
 * Network node states
 */
export const NetworkState = {
  OFFLINE: 'OFFLINE',
  BOOTSTRAPPING: 'BOOTSTRAPPING',
  SYNCING: 'SYNCING',
  ONLINE: 'ONLINE',
  PARTICIPATING: 'PARTICIPATING',
  ERROR: 'ERROR',
};

/**
 * φ-aligned network intervals (Fibonacci-based)
 */
const NETWORK_INTERVALS = {
  HEARTBEAT_MS: 8000,          // F(6) seconds
  STATE_SYNC_MS: 13000,        // F(7) seconds
  VALIDATOR_CHECK_MS: 21000,   // F(8) seconds
  METRICS_REPORT_MS: 34000,    // F(9) seconds
};

/**
 * CYNICNetworkNode - Orchestrates all P2P components
 *
 * Single entry point for multi-node operations.
 */
export class CYNICNetworkNode extends EventEmitter {
  /**
   * Create a network node
   *
   * @param {Object} options - Node options
   * @param {string} options.publicKey - Node public key
   * @param {string} options.privateKey - Node private key
   * @param {number} [options.eScore=50] - Node E-Score
   * @param {number} [options.port=8618] - P2P port (φ-aligned)
   * @param {string[]} [options.seedNodes=[]] - Bootstrap seed nodes
   * @param {boolean} [options.enabled=true] - Enable networking
   * @param {Object} [options.ssl] - SSL config for WSS
   */
  constructor(options = {}) {
    super();

    if (!options.publicKey || !options.privateKey) {
      throw new Error('publicKey and privateKey required');
    }

    this._publicKey = options.publicKey;
    this._privateKey = options.privateKey;
    this._eScore = options.eScore || 50;
    this._port = options.port || 8618;
    this._seedNodes = options.seedNodes || [];
    this._enabled = options.enabled ?? true;

    this._state = NetworkState.OFFLINE;
    this._startedAt = null;

    // Initialize components (lazy - created but not started)
    this._transport = null;
    this._consensus = null;
    this._discovery = null;

    // Timers
    this._heartbeatTimer = null;
    this._syncTimer = null;
    this._validatorTimer = null;
    this._metricsTimer = null;

    // State sync tracking
    this._syncState = {
      lastSyncSlot: 0,
      syncing: false,
      behindBy: 0,
      syncInProgress: false,
      lastSyncAttempt: 0,
    };

    // Track peer finalized slots (from heartbeats)
    this._peerSlots = new Map(); // peerId → { finalizedSlot, lastSeen }

    // Fork detection state
    this._forkState = {
      detected: false,
      forkSlot: null,           // Slot where fork was detected
      forkHashes: new Map(),    // slot → Map<hash, Set<peerId>>
      ourBranch: null,          // Which branch we're on
      resolutionInProgress: false,
      lastCheck: 0,
    };

    // Track block hashes per slot (for fork detection)
    this._slotHashes = new Map(); // slot → { hash, confirmedBy: Set<peerId> }

    // Stats
    this._stats = {
      uptime: 0,
      messagesReceived: 0,
      messagesSent: 0,
      blocksProposed: 0,
      blocksFinalized: 0,
      validatorsKnown: 0,
      peersConnected: 0,
      errors: 0,
      forksDetected: 0,
      forksResolved: 0,
    };

    if (this._enabled) {
      this._initializeComponents(options);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Initialization
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Initialize all P2P components
   * @private
   */
  _initializeComponents(options) {
    // Transport (WebSocket + Gossip)
    this._transport = new TransportComponent({
      port: this._port,
      host: options.host || '0.0.0.0',
      publicKey: this._publicKey,
      privateKey: this._privateKey,
      ssl: options.ssl,
      onMessage: this._handleMessage.bind(this),
    });

    // Consensus (φ-BFT)
    this._consensus = new ConsensusComponent({
      publicKey: this._publicKey,
      privateKey: this._privateKey,
      eScore: this._eScore,
      burned: options.burned || 0,
      confirmations: options.confirmations || 32,
      gossip: this._transport.gossip,
    });

    // Discovery
    this._discovery = new PeerDiscovery({
      transport: this._transport.transport,
      seedNodes: this._seedNodes,
      minPeers: options.minPeers || 3,
      maxPeers: options.maxPeers || 50,
    });

    log.info('Components initialized', {
      port: this._port,
      seedNodes: this._seedNodes.length,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Lifecycle
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Start the network node
   * @returns {Promise<void>}
   */
  async start() {
    if (this._state !== NetworkState.OFFLINE) {
      log.warn('Already started', { state: this._state });
      return;
    }

    if (!this._enabled) {
      log.info('Networking disabled');
      return;
    }

    this._state = NetworkState.BOOTSTRAPPING;
    this._startedAt = Date.now();

    try {
      // 1. Start transport
      await this._transport.start();
      this._wireTransportEvents();

      // 2. Wire consensus events
      this._wireConsensusEvents();

      // 3. Start discovery
      this._discovery.start();
      this._wireDiscoveryEvents();

      // 4. Start consensus (registers self as validator)
      this._consensus.start({ eScore: this._eScore });

      // 5. Start periodic tasks
      this._startPeriodicTasks();

      // 6. Emit startup event
      globalEventBus.publish(EventType.NODE_STARTED, {
        nodeId: this._publicKey.slice(0, 16),
        port: this._port,
        eScore: this._eScore,
        timestamp: Date.now(),
      });

      this._state = NetworkState.ONLINE;
      this.emit('started', { nodeId: this._publicKey.slice(0, 16), port: this._port });

      log.info('🌐 Network node started', {
        port: this._port,
        eScore: this._eScore,
        seedNodes: this._seedNodes.length,
      });

    } catch (error) {
      this._state = NetworkState.ERROR;
      this._stats.errors++;
      log.error('Start failed', { error: error.message });
      throw error;
    }
  }

  /**
   * Stop the network node
   * @returns {Promise<void>}
   */
  async stop() {
    if (this._state === NetworkState.OFFLINE) return;

    log.info('Stopping network node...');

    // Stop periodic tasks
    this._stopPeriodicTasks();

    // Stop components
    this._discovery?.stop();
    this._consensus?.stop();
    await this._transport?.stop();

    // Emit shutdown event
    globalEventBus.publish(EventType.NODE_STOPPED, {
      nodeId: this._publicKey.slice(0, 16),
      uptime: Date.now() - this._startedAt,
      stats: this._stats,
    });

    this._state = NetworkState.OFFLINE;
    this.emit('stopped', { uptime: Date.now() - this._startedAt });

    log.info('Network node stopped', { uptime: Math.round((Date.now() - this._startedAt) / 1000) + 's' });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Event Wiring
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Wire transport events
   * @private
   */
  _wireTransportEvents() {
    this._transport.wireEvents({
      onPeerConnected: (peerId, publicKey, address) => {
        this._stats.peersConnected = this._transport.getConnectedPeers().length;

        // Register peer as potential validator
        this._consensus.registerValidator({
          publicKey: publicKey || peerId,
          eScore: 50, // Default, will be updated when we receive their E-Score
        });

        this.emit('peer:connected', { peerId, publicKey, address });
      },

      onPeerDisconnected: (peerId, code, reason) => {
        this._stats.peersConnected = this._transport.getConnectedPeers().length;

        // If we drop below minimum peers, go back to syncing
        if (this._stats.peersConnected < 3 && this._state === NetworkState.PARTICIPATING) {
          this._state = NetworkState.SYNCING;
        }

        this.emit('peer:disconnected', { peerId, code, reason });
      },

      onPeerError: (peerId, error) => {
        this._stats.errors++;
        this.emit('peer:error', { peerId, error });
      },
    });
  }

  /**
   * Wire consensus events
   * @private
   */
  _wireConsensusEvents() {
    this._consensus.wireEvents({
      onBlockFinalized: async (blockHash, slot, block) => {
        this._stats.blocksFinalized++;

        // Publish to global event bus
        globalEventBus.publish(EventType.BLOCK_FINALIZED, {
          blockHash,
          slot,
          block,
          timestamp: Date.now(),
        });

        this.emit('block:finalized', { blockHash, slot, block });
      },

      onBlockConfirmed: (slot, ratio) => {
        // Update sync state
        this._syncState.lastSyncSlot = slot;
        this.emit('block:confirmed', { slot, ratio });
      },

      onConsensusStarted: (slot) => {
        // We're now participating in consensus
        if (this._state === NetworkState.ONLINE || this._state === NetworkState.SYNCING) {
          this._state = NetworkState.PARTICIPATING;
        }
        this.emit('consensus:started', { slot });
      },
    });
  }

  /**
   * Wire discovery events
   * @private
   */
  _wireDiscoveryEvents() {
    this._discovery.on('peer:discovered', (event) => {
      this.emit('peer:discovered', event);
    });

    this._discovery.on('started', () => {
      log.info('Peer discovery started');
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Periodic Tasks
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Start periodic tasks
   * @private
   */
  _startPeriodicTasks() {
    // Heartbeat - publish node status
    this._heartbeatTimer = setInterval(() => {
      this._publishHeartbeat();
    }, NETWORK_INTERVALS.HEARTBEAT_MS);

    // State sync - check if we're behind
    this._syncTimer = setInterval(() => {
      this._checkStateSync();
    }, NETWORK_INTERVALS.STATE_SYNC_MS);

    // Validator check - update validator set
    this._validatorTimer = setInterval(() => {
      this._updateValidatorSet();
    }, NETWORK_INTERVALS.VALIDATOR_CHECK_MS);

    // Metrics report
    this._metricsTimer = setInterval(() => {
      this._reportMetrics();
    }, NETWORK_INTERVALS.METRICS_REPORT_MS);
  }

  /**
   * Stop periodic tasks
   * @private
   */
  _stopPeriodicTasks() {
    if (this._heartbeatTimer) {
      clearInterval(this._heartbeatTimer);
      this._heartbeatTimer = null;
    }
    if (this._syncTimer) {
      clearInterval(this._syncTimer);
      this._syncTimer = null;
    }
    if (this._validatorTimer) {
      clearInterval(this._validatorTimer);
      this._validatorTimer = null;
    }
    if (this._metricsTimer) {
      clearInterval(this._metricsTimer);
      this._metricsTimer = null;
    }
  }

  /**
   * Publish heartbeat to network
   * @private
   */
  _publishHeartbeat() {
    // Get recent finalized block hashes for fork detection (last 5 slots)
    const recentHashes = this._getRecentBlockHashes(5);

    this._transport.gossip?.broadcastMessage?.({
      type: 'HEARTBEAT',
      nodeId: this._publicKey.slice(0, 32),
      eScore: this._eScore,
      slot: this._consensus?.currentSlot || 0,
      finalizedSlot: this._consensus?.lastFinalizedSlot || 0,
      finalizedHash: this._consensus?.lastFinalizedHash || null,
      recentHashes, // [{slot, hash}, ...] for fork detection
      state: this._state,
      timestamp: Date.now(),
    });
  }

  /**
   * Get recent block hashes for fork detection
   * @private
   * @param {number} count - Number of recent slots to include
   * @returns {Array<{slot: number, hash: string}>}
   */
  _getRecentBlockHashes(count) {
    const hashes = [];
    const currentSlot = this._consensus?.lastFinalizedSlot || 0;

    for (let i = 0; i < count && currentSlot - i >= 0; i++) {
      const slot = currentSlot - i;
      const slotInfo = this._slotHashes.get(slot);
      if (slotInfo?.hash) {
        hashes.push({ slot, hash: slotInfo.hash });
      }
    }

    return hashes;
  }

  /**
   * Check if we need to sync state
   * @private
   */
  _checkStateSync() {
    const ourSlot = this._consensus?.lastFinalizedSlot || 0;
    this._syncState.lastSyncSlot = ourSlot;

    // Find the highest finalized slot among peers
    let highestPeerSlot = ourSlot;
    let bestPeer = null;

    for (const [peerId, peerInfo] of this._peerSlots) {
      // Only consider recent heartbeats (within 60s)
      if (Date.now() - peerInfo.lastSeen < 60000) {
        if (peerInfo.finalizedSlot > highestPeerSlot) {
          highestPeerSlot = peerInfo.finalizedSlot;
          bestPeer = peerId;
        }
      }
    }

    // Calculate how far behind we are
    this._syncState.behindBy = highestPeerSlot - ourSlot;

    // If we're significantly behind (>10 slots), start syncing
    if (this._syncState.behindBy > 10 && !this._syncState.syncInProgress) {
      if (this._state !== NetworkState.SYNCING) {
        this._state = NetworkState.SYNCING;
        log.info('Node is behind network', { ourSlot, highestPeerSlot, behindBy: this._syncState.behindBy });
      }

      // Request state sync from the best peer
      if (bestPeer) {
        this._requestStateSync(bestPeer, ourSlot);
      }

      this.emit('sync:needed', { behindBy: this._syncState.behindBy, bestPeer });
    } else if (this._syncState.behindBy <= 10 && this._state === NetworkState.SYNCING) {
      // We've caught up
      this._state = NetworkState.ONLINE;
      this._syncState.syncInProgress = false;
      log.info('Sync complete - caught up with network', { slot: ourSlot });
      this.emit('sync:complete', { slot: ourSlot });
    }
  }

  /**
   * Request state sync from a peer
   * @private
   * @param {string} peerId - Peer to request from
   * @param {number} fromSlot - Slot to start sync from
   */
  async _requestStateSync(peerId, fromSlot) {
    // Debounce sync requests (min 5s between attempts)
    if (Date.now() - this._syncState.lastSyncAttempt < 5000) {
      return;
    }

    this._syncState.syncInProgress = true;
    this._syncState.lastSyncAttempt = Date.now();

    log.info('Requesting state sync', { peerId: peerId.slice(0, 16), fromSlot });

    try {
      await this._transport.sendTo(peerId, {
        type: 'STATE_REQUEST',
        fromSlot,
        nodeId: this._publicKey.slice(0, 32),
        timestamp: Date.now(),
      });
      this._stats.messagesSent++;
    } catch (error) {
      log.warn('State sync request failed', { peerId: peerId.slice(0, 16), error: error.message });
      this._syncState.syncInProgress = false;
    }
  }

  /**
   * Update validator set from connected peers
   * @private
   */
  _updateValidatorSet() {
    const connectedPeers = this._transport.getConnectedPeers();
    this._stats.validatorsKnown = this._consensus.validatorCount;

    // Clean up validators that haven't been seen
    // (In a real implementation, this would track last-seen times)
  }

  /**
   * Report metrics
   * @private
   */
  _reportMetrics() {
    this._stats.uptime = Date.now() - this._startedAt;

    globalEventBus.publish(EventType.METRICS_REPORTED, {
      nodeId: this._publicKey.slice(0, 16),
      state: this._state,
      stats: this._stats,
      consensus: this._consensus.getStats(),
      transport: this._transport.getInfo(),
      discovery: this._discovery.getStats(),
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Message Handling
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Handle incoming message
   * @private
   */
  async _handleMessage(message, peerId) {
    this._stats.messagesReceived++;

    switch (message.type) {
      case 'HEARTBEAT':
        await this._handleHeartbeat(message, peerId);
        break;

      case 'STATE_REQUEST':
        await this._handleStateRequest(message, peerId);
        break;

      case 'STATE_RESPONSE':
        await this._handleStateResponse(message, peerId);
        break;

      case 'BLOCK_REQUEST':
        await this._handleBlockRequest(message, peerId);
        break;

      case 'VALIDATOR_UPDATE':
        await this._handleValidatorUpdate(message, peerId);
        break;

      // Consensus messages are handled by ConsensusGossip
      // Judgment/Block messages are handled by GossipProtocol
      default:
        // Unknown message type - let components handle
        break;
    }
  }

  /**
   * Handle heartbeat from peer
   * @private
   */
  async _handleHeartbeat(message, peerId) {
    const { eScore, slot, finalizedSlot, state, nodeId } = message;

    // Track peer's finalized slot
    this._peerSlots.set(peerId, {
      finalizedSlot: finalizedSlot || 0,
      slot: slot || 0,
      state: state || 'UNKNOWN',
      eScore: eScore || 50,
      lastSeen: Date.now(),
    });

    // Update peer's E-Score in consensus
    if (eScore && this._consensus) {
      this._consensus.registerValidator({
        publicKey: peerId,
        eScore,
      });
    }

    // Emit for monitoring
    this.emit('heartbeat:received', {
      peerId,
      nodeId,
      finalizedSlot,
      eScore,
    });
  }

  /**
   * Handle state request
   * @private
   */
  async _handleStateRequest(message, peerId) {
    const { fromSlot } = message;

    // Send our state from requested slot
    await this._transport.sendTo(peerId, {
      type: 'STATE_RESPONSE',
      fromSlot,
      currentSlot: this._consensus.currentSlot,
      finalizedSlot: this._consensus.lastFinalizedSlot,
      // Would include block hashes, state root, etc.
    });
  }

  /**
   * Handle state response (for syncing)
   * @private
   */
  async _handleStateResponse(message, peerId) {
    const { finalizedSlot, blocks, stateRoot } = message;
    const ourSlot = this._consensus.lastFinalizedSlot;

    this._syncState.syncInProgress = false;

    // If they have blocks we need
    if (finalizedSlot > ourSlot && blocks && blocks.length > 0) {
      log.info('Received state sync response', {
        ourSlot,
        theirSlot: finalizedSlot,
        blocksReceived: blocks.length,
      });

      // Process received blocks
      let processedCount = 0;
      for (const block of blocks) {
        try {
          // Verify and apply block
          if (block.slot > ourSlot) {
            // In a real implementation, we'd validate and apply each block
            // For now, just count them
            processedCount++;
          }
        } catch (error) {
          log.warn('Failed to process sync block', { slot: block.slot, error: error.message });
        }
      }

      this.emit('sync:blocks_received', {
        fromPeer: peerId,
        blocksReceived: blocks.length,
        blocksProcessed: processedCount,
      });
    } else if (finalizedSlot > ourSlot) {
      // They're ahead but didn't send blocks - request them
      log.info('Peer is ahead but no blocks received, requesting blocks', {
        ourSlot,
        theirSlot: finalizedSlot,
      });

      await this._transport.sendTo(peerId, {
        type: 'BLOCK_REQUEST',
        fromSlot: ourSlot + 1,
        toSlot: finalizedSlot,
        nodeId: this._publicKey.slice(0, 32),
        timestamp: Date.now(),
      });
      this._stats.messagesSent++;
    } else {
      log.debug('State response - we are caught up', { ourSlot, theirSlot: finalizedSlot });
    }
  }

  /**
   * Handle validator update
   * @private
   */
  async _handleValidatorUpdate(message, peerId) {
    const { validator, action } = message;

    if (action === 'ADD' && validator) {
      this._consensus.registerValidator(validator);
    } else if (action === 'REMOVE' && validator?.publicKey) {
      this._consensus.removeValidator(validator.publicKey);
    }
  }

  /**
   * Handle block request from peer (for sync)
   * @private
   */
  async _handleBlockRequest(message, peerId) {
    const { fromSlot, toSlot } = message;

    log.info('Block request received', { fromPeer: peerId.slice(0, 16), fromSlot, toSlot });

    // TODO: In a real implementation, we'd retrieve blocks from storage
    // For now, send an empty response acknowledging the request
    const blocks = [];

    // Would be something like:
    // const blocks = await this._storage.getBlocks(fromSlot, toSlot);

    await this._transport.sendTo(peerId, {
      type: 'STATE_RESPONSE',
      fromSlot,
      finalizedSlot: this._consensus.lastFinalizedSlot,
      blocks,
      stateRoot: null, // Would be computed from state
      timestamp: Date.now(),
    });
    this._stats.messagesSent++;

    this.emit('sync:blocks_sent', {
      toPeer: peerId,
      fromSlot,
      toSlot,
      blocksSent: blocks.length,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Public API
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Connect to a peer
   * @param {Object} peer - Peer info
   * @param {string} peer.address - Peer address (ws://host:port)
   * @returns {Promise<void>}
   */
  async connectToPeer(peer) {
    await this._transport.connectToPeer(peer);
  }

  /**
   * Add a seed node
   * @param {string} address - Seed node address
   */
  addSeedNode(address) {
    this._seedNodes.push(address);
    this._discovery?.addSeedNode(address);
  }

  /**
   * Propose a block to the network
   * @param {Object} block - Block to propose
   * @returns {Object|null} Consensus record
   */
  proposeBlock(block) {
    if (this._state !== NetworkState.PARTICIPATING) {
      log.warn('Cannot propose - not participating', { state: this._state });
      return null;
    }

    const record = this._consensus.proposeBlock(block);
    if (record) {
      this._stats.blocksProposed++;
      this._transport.broadcastBlock(block);
    }
    return record;
  }

  /**
   * Broadcast a judgment to the network
   * @param {Object} judgment - Judgment to broadcast
   * @returns {Promise<void>}
   */
  async broadcastJudgment(judgment) {
    if (this._state === NetworkState.OFFLINE) return;
    await this._transport.broadcastJudgment(judgment);
    this._stats.messagesSent++;
  }

  /**
   * Broadcast a pattern to the network
   * @param {Object} pattern - Pattern to broadcast
   * @returns {Promise<void>}
   */
  async broadcastPattern(pattern) {
    if (this._state === NetworkState.OFFLINE) return;
    await this._transport.broadcastPattern(pattern);
    this._stats.messagesSent++;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // State & Stats
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {string} Current state */
  get state() {
    return this._state;
  }

  /** @returns {boolean} Whether node is online */
  get isOnline() {
    return this._state === NetworkState.ONLINE || this._state === NetworkState.PARTICIPATING;
  }

  /** @returns {boolean} Whether participating in consensus */
  get isParticipating() {
    return this._state === NetworkState.PARTICIPATING;
  }

  /** @returns {string} Node public key */
  get publicKey() {
    return this._publicKey;
  }

  /** @returns {number} Node E-Score */
  get eScore() {
    return this._eScore;
  }

  /**
   * Update E-Score
   * @param {number} score - New E-Score
   */
  setEScore(score) {
    this._eScore = Math.min(Math.max(score, 0), 100);
    // Update in consensus
    this._consensus?.registerValidator({
      publicKey: this._publicKey,
      eScore: this._eScore,
    });
  }

  /**
   * Get node info
   * @returns {Object} Node info
   */
  getInfo() {
    return {
      publicKey: this._publicKey.slice(0, 32) + '...',
      port: this._port,
      eScore: this._eScore,
      state: this._state,
      uptime: this._startedAt ? Date.now() - this._startedAt : 0,
      stats: this._stats,
    };
  }

  /**
   * Get detailed status
   * @returns {Object} Full status
   */
  getStatus() {
    return {
      node: this.getInfo(),
      transport: this._transport?.getInfo() || null,
      consensus: this._consensus?.getInfo() || null,
      discovery: this._discovery?.getStats() || null,
      sync: this._syncState,
    };
  }

  /**
   * Get connected peers
   * @returns {string[]} Peer IDs
   */
  getConnectedPeers() {
    return this._transport?.getConnectedPeers() || [];
  }

  /**
   * Get validator count
   * @returns {number} Validator count
   */
  getValidatorCount() {
    return this._consensus?.validatorCount || 0;
  }
}

export default CYNICNetworkNode;

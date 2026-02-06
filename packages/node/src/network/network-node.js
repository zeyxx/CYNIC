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

    // Validator set management
    this._validators = new Map(); // publicKey → ValidatorInfo
    this._validatorConfig = {
      minEScore: 20,              // Minimum E-Score to be validator
      maxValidators: 100,         // Maximum validators in set
      inactivityTimeout: 120000,  // 2 minutes without heartbeat = inactive
      penaltyDecay: 0.95,         // Penalty decay per epoch (φ²)
      rewardMultiplier: 1.02,     // Reward multiplier for good behavior
    };

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
      validatorsAdded: 0,
      validatorsRemoved: 0,
      validatorsPenalized: 0,
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
    // Check for inactive validators
    this._checkInactiveValidators();

    // Sync connected peers as potential validators
    const connectedPeers = this._transport?.getConnectedPeers() || [];

    for (const peerId of connectedPeers) {
      const peerInfo = this._peerSlots.get(peerId);
      if (peerInfo && !this._validators.has(peerId)) {
        // Add connected peer as validator if they have sufficient E-Score
        if (peerInfo.eScore >= this._validatorConfig.minEScore) {
          this.addValidator({
            publicKey: peerId,
            eScore: peerInfo.eScore,
          });
        }
      }
    }

    // Update stats
    this._stats.validatorsKnown = this._validators.size;

    // Apply penalty decay (φ²) - penalties slowly decrease over time
    for (const validator of this._validators.values()) {
      if (validator.penalties > 0) {
        validator.penalties *= this._validatorConfig.penaltyDecay;
        if (validator.penalties < 0.1) {
          validator.penalties = 0;
        }
      }
    }
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

      case 'FORK_RESOLUTION_REQUEST':
        await this._handleForkResolutionRequest(message, peerId);
        break;

      case 'FORK_RESOLUTION_RESPONSE':
        await this._handleForkResolutionResponse(message, peerId);
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
    const { eScore, slot, finalizedSlot, finalizedHash, recentHashes, state, nodeId } = message;

    // Track peer's finalized slot
    this._peerSlots.set(peerId, {
      finalizedSlot: finalizedSlot || 0,
      finalizedHash: finalizedHash || null,
      slot: slot || 0,
      state: state || 'UNKNOWN',
      eScore: eScore || 50,
      lastSeen: Date.now(),
    });

    // Update validator activity (marks as active)
    this.updateValidatorActivity(peerId);

    // Add or update validator if they meet minimum E-Score
    if (eScore && eScore >= this._validatorConfig.minEScore) {
      this.addValidator({
        publicKey: peerId,
        eScore,
      });
    }

    // Check for forks using recent hashes
    if (recentHashes && recentHashes.length > 0) {
      this._checkForForks(peerId, recentHashes, eScore || 50);
    }

    // Emit for monitoring
    this.emit('heartbeat:received', {
      peerId,
      nodeId,
      finalizedSlot,
      finalizedHash,
      eScore,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Fork Detection
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check for chain forks based on peer's recent block hashes
   * @private
   * @param {string} peerId - Peer ID
   * @param {Array<{slot: number, hash: string}>} peerHashes - Peer's recent block hashes
   * @param {number} peerEScore - Peer's E-Score for weighting
   */
  _checkForForks(peerId, peerHashes, peerEScore) {
    for (const { slot, hash } of peerHashes) {
      if (!hash) continue;

      // Get or create fork tracking for this slot
      if (!this._forkState.forkHashes.has(slot)) {
        this._forkState.forkHashes.set(slot, new Map());
      }

      const slotForks = this._forkState.forkHashes.get(slot);

      // Track which peers have which hash for this slot
      if (!slotForks.has(hash)) {
        slotForks.set(hash, { peers: new Set(), totalEScore: 0 });
      }

      const hashInfo = slotForks.get(hash);
      if (!hashInfo.peers.has(peerId)) {
        hashInfo.peers.add(peerId);
        hashInfo.totalEScore += peerEScore;
      }

      // Check if we have multiple hashes for the same slot (FORK!)
      if (slotForks.size > 1 && !this._forkState.detected) {
        this._onForkDetected(slot, slotForks);
      }
    }

    // Cleanup old fork data (keep last 100 slots)
    this._cleanupForkData();
  }

  /**
   * Handle fork detection
   * @private
   * @param {number} slot - Slot where fork was detected
   * @param {Map<string, {peers: Set, totalEScore: number}>} forks - Fork information
   */
  _onForkDetected(slot, forks) {
    this._forkState.detected = true;
    this._forkState.forkSlot = slot;
    this._stats.forksDetected++;

    // Calculate which branch has more weight (E-Score weighted)
    let heaviestHash = null;
    let heaviestWeight = 0;
    const forkDetails = [];

    for (const [hash, info] of forks) {
      forkDetails.push({
        hash: hash.slice(0, 16) + '...',
        peers: info.peers.size,
        totalEScore: info.totalEScore,
      });

      if (info.totalEScore > heaviestWeight) {
        heaviestWeight = info.totalEScore;
        heaviestHash = hash;
      }
    }

    log.warn('🔀 FORK DETECTED', {
      slot,
      branches: forks.size,
      forks: forkDetails,
      heaviestBranch: heaviestHash?.slice(0, 16),
    });

    // Determine which branch we're on
    const ourHash = this._slotHashes.get(slot)?.hash;
    this._forkState.ourBranch = ourHash;

    const onHeaviestBranch = ourHash === heaviestHash;

    this.emit('fork:detected', {
      slot,
      branches: forks.size,
      forks: forkDetails,
      ourBranch: ourHash?.slice(0, 16),
      heaviestBranch: heaviestHash?.slice(0, 16),
      onHeaviestBranch,
      recommendation: onHeaviestBranch ? 'STAY' : 'REORG_NEEDED',
    });

    // If we're NOT on the heaviest branch, trigger resolution
    if (!onHeaviestBranch && !this._forkState.resolutionInProgress) {
      this._resolveFork(slot, heaviestHash);
    }
  }

  /**
   * Attempt to resolve a fork by switching to the heaviest branch
   * @private
   * @param {number} forkSlot - Slot where fork occurred
   * @param {string} targetHash - Hash of the branch to switch to
   */
  async _resolveFork(forkSlot, targetHash) {
    this._forkState.resolutionInProgress = true;

    log.info('Attempting fork resolution', {
      forkSlot,
      targetBranch: targetHash?.slice(0, 16),
    });

    // Find a peer on the heaviest branch
    const forkInfo = this._forkState.forkHashes.get(forkSlot)?.get(targetHash);
    if (!forkInfo || forkInfo.peers.size === 0) {
      log.warn('No peers found on target branch');
      this._forkState.resolutionInProgress = false;
      return;
    }

    // Get the peer with highest E-Score on that branch
    let bestPeer = null;
    let bestScore = 0;

    for (const peerId of forkInfo.peers) {
      const peerInfo = this._peerSlots.get(peerId);
      if (peerInfo && peerInfo.eScore > bestScore) {
        bestScore = peerInfo.eScore;
        bestPeer = peerId;
      }
    }

    if (!bestPeer) {
      log.warn('No suitable peer for fork resolution');
      this._forkState.resolutionInProgress = false;
      return;
    }

    // Request blocks from fork point to sync to correct branch
    try {
      await this._transport?.sendTo(bestPeer, {
        type: 'FORK_RESOLUTION_REQUEST',
        forkSlot,
        targetHash,
        nodeId: this._publicKey.slice(0, 32),
        timestamp: Date.now(),
      });
      this._stats.messagesSent++;

      log.info('Fork resolution request sent', {
        toPeer: bestPeer.slice(0, 16),
        forkSlot,
      });

      this.emit('fork:resolution_started', {
        forkSlot,
        targetBranch: targetHash?.slice(0, 16),
        resolvingWith: bestPeer.slice(0, 16),
      });
    } catch (error) {
      log.error('Fork resolution request failed', { error: error.message });
      this._forkState.resolutionInProgress = false;
    }
  }

  /**
   * Mark fork as resolved
   * @private
   */
  _markForkResolved() {
    if (this._forkState.detected) {
      this._stats.forksResolved++;
      log.info('Fork resolved', { forkSlot: this._forkState.forkSlot });

      this.emit('fork:resolved', {
        forkSlot: this._forkState.forkSlot,
      });
    }

    this._forkState.detected = false;
    this._forkState.forkSlot = null;
    this._forkState.ourBranch = null;
    this._forkState.resolutionInProgress = false;
  }

  /**
   * Cleanup old fork tracking data
   * @private
   */
  _cleanupForkData() {
    const currentSlot = this._consensus?.lastFinalizedSlot || 0;
    const keepFrom = currentSlot - 100; // Keep last 100 slots

    for (const slot of this._forkState.forkHashes.keys()) {
      if (slot < keepFrom) {
        this._forkState.forkHashes.delete(slot);
      }
    }

    for (const slot of this._slotHashes.keys()) {
      if (slot < keepFrom) {
        this._slotHashes.delete(slot);
      }
    }
  }

  /**
   * Record a block hash for a slot (called when block is finalized)
   * @param {number} slot - Slot number
   * @param {string} hash - Block hash
   */
  recordBlockHash(slot, hash) {
    this._slotHashes.set(slot, {
      hash,
      confirmedAt: Date.now(),
    });
  }

  /**
   * Get fork status
   * @returns {Object} Fork detection status
   */
  getForkStatus() {
    const forkDetails = [];
    if (this._forkState.forkSlot !== null) {
      const forks = this._forkState.forkHashes.get(this._forkState.forkSlot);
      if (forks) {
        for (const [hash, info] of forks) {
          forkDetails.push({
            hash: hash.slice(0, 16) + '...',
            peers: info.peers.size,
            totalEScore: info.totalEScore,
          });
        }
      }
    }

    return {
      detected: this._forkState.detected,
      forkSlot: this._forkState.forkSlot,
      ourBranch: this._forkState.ourBranch?.slice(0, 16),
      resolutionInProgress: this._forkState.resolutionInProgress,
      branches: forkDetails,
      stats: {
        forksDetected: this._stats.forksDetected,
        forksResolved: this._stats.forksResolved,
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Validator Set Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * ValidatorInfo structure
   * @typedef {Object} ValidatorInfo
   * @property {string} publicKey - Validator public key
   * @property {number} eScore - Current E-Score (0-100)
   * @property {number} burned - Total burned tokens
   * @property {number} uptime - Uptime ratio (0-1)
   * @property {number} lastSeen - Last heartbeat timestamp
   * @property {number} blocksProposed - Blocks proposed count
   * @property {number} blocksFinalized - Blocks finalized count
   * @property {number} penalties - Accumulated penalties
   * @property {string} status - 'active' | 'inactive' | 'penalized' | 'removed'
   * @property {number} joinedAt - When validator joined
   */

  /**
   * Add or update a validator in the set
   * @param {Object} validator - Validator info
   * @param {string} validator.publicKey - Validator public key
   * @param {number} [validator.eScore=50] - E-Score
   * @param {number} [validator.burned=0] - Burned tokens
   * @returns {boolean} True if added, false if updated
   */
  addValidator(validator) {
    const { publicKey, eScore = 50, burned = 0 } = validator;

    if (!publicKey) {
      log.warn('Cannot add validator without publicKey');
      return false;
    }

    // Check minimum E-Score
    if (eScore < this._validatorConfig.minEScore) {
      log.info('Validator E-Score below minimum', {
        publicKey: publicKey.slice(0, 16),
        eScore,
        minRequired: this._validatorConfig.minEScore,
      });
      return false;
    }

    const existing = this._validators.get(publicKey);
    const isNew = !existing;

    if (isNew) {
      // Check max validators limit
      if (this._validators.size >= this._validatorConfig.maxValidators) {
        // Try to remove lowest E-Score validator
        if (!this._evictLowestValidator(eScore)) {
          log.info('Validator set full, cannot add', { publicKey: publicKey.slice(0, 16) });
          return false;
        }
      }
    }

    const validatorInfo = {
      publicKey,
      eScore,
      burned,
      uptime: existing?.uptime || 1.0,
      lastSeen: Date.now(),
      blocksProposed: existing?.blocksProposed || 0,
      blocksFinalized: existing?.blocksFinalized || 0,
      penalties: existing?.penalties || 0,
      status: 'active',
      joinedAt: existing?.joinedAt || Date.now(),
    };

    this._validators.set(publicKey, validatorInfo);

    // Also register in consensus component
    if (this._consensus) {
      this._consensus.registerValidator({
        publicKey,
        eScore,
        burned,
        uptime: validatorInfo.uptime,
      });
    }

    if (isNew) {
      this._stats.validatorsAdded++;
      this._stats.validatorsKnown = this._validators.size;

      log.info('Validator added', {
        publicKey: publicKey.slice(0, 16),
        eScore,
        totalValidators: this._validators.size,
      });

      this.emit('validator:added', { publicKey, eScore, burned });
    } else {
      this.emit('validator:updated', { publicKey, eScore, burned });
    }

    return isNew;
  }

  /**
   * Remove a validator from the set
   * @param {string} publicKey - Validator public key
   * @param {string} [reason='manual'] - Removal reason
   * @returns {boolean} True if removed
   */
  removeValidator(publicKey, reason = 'manual') {
    const validator = this._validators.get(publicKey);
    if (!validator) {
      return false;
    }

    // Mark as removed
    validator.status = 'removed';
    this._validators.delete(publicKey);

    // Remove from consensus
    if (this._consensus) {
      this._consensus.removeValidator(publicKey);
    }

    this._stats.validatorsRemoved++;
    this._stats.validatorsKnown = this._validators.size;

    log.info('Validator removed', {
      publicKey: publicKey.slice(0, 16),
      reason,
      remainingValidators: this._validators.size,
    });

    this.emit('validator:removed', { publicKey, reason });

    return true;
  }

  /**
   * Penalize a validator for bad behavior
   * @param {string} publicKey - Validator public key
   * @param {number} penalty - Penalty amount (0-100)
   * @param {string} reason - Penalty reason
   * @returns {boolean} True if penalized
   */
  penalizeValidator(publicKey, penalty, reason) {
    const validator = this._validators.get(publicKey);
    if (!validator) {
      return false;
    }

    validator.penalties += penalty;
    validator.eScore = Math.max(0, validator.eScore - penalty);

    // If E-Score drops too low, remove validator
    if (validator.eScore < this._validatorConfig.minEScore) {
      validator.status = 'penalized';
      this.removeValidator(publicKey, `penalty_threshold_${reason}`);
      this._stats.validatorsPenalized++;
      return true;
    }

    // Update in consensus
    if (this._consensus) {
      this._consensus.registerValidator({
        publicKey,
        eScore: validator.eScore,
        burned: validator.burned,
        uptime: validator.uptime,
      });
    }

    this._stats.validatorsPenalized++;

    log.warn('Validator penalized', {
      publicKey: publicKey.slice(0, 16),
      penalty,
      reason,
      newEScore: validator.eScore,
    });

    this.emit('validator:penalized', {
      publicKey,
      penalty,
      reason,
      newEScore: validator.eScore,
    });

    return true;
  }

  /**
   * Reward a validator for good behavior
   * @param {string} publicKey - Validator public key
   * @param {string} action - Action that earned reward ('block_proposed', 'block_finalized')
   */
  rewardValidator(publicKey, action) {
    const validator = this._validators.get(publicKey);
    if (!validator) return;

    switch (action) {
      case 'block_proposed':
        validator.blocksProposed++;
        break;
      case 'block_finalized':
        validator.blocksFinalized++;
        // Small E-Score boost for successful blocks (capped at 100)
        validator.eScore = Math.min(100, validator.eScore * this._validatorConfig.rewardMultiplier);
        break;
    }

    this.emit('validator:rewarded', { publicKey, action });
  }

  /**
   * Update validator activity (called on heartbeat)
   * @param {string} publicKey - Validator public key
   */
  updateValidatorActivity(publicKey) {
    const validator = this._validators.get(publicKey);
    if (!validator) return;

    validator.lastSeen = Date.now();
    validator.status = 'active';
  }

  /**
   * Check for inactive validators and update their status
   * Called periodically by _updateValidatorSet
   * @private
   */
  _checkInactiveValidators() {
    const now = Date.now();
    const timeout = this._validatorConfig.inactivityTimeout;

    for (const [publicKey, validator] of this._validators) {
      if (validator.status === 'active' && now - validator.lastSeen > timeout) {
        validator.status = 'inactive';
        validator.uptime *= 0.9; // Reduce uptime on inactivity

        log.info('Validator marked inactive', {
          publicKey: publicKey.slice(0, 16),
          lastSeen: Math.round((now - validator.lastSeen) / 1000) + 's ago',
        });

        this.emit('validator:inactive', { publicKey });

        // Penalize for extended inactivity
        if (now - validator.lastSeen > timeout * 3) {
          this.penalizeValidator(publicKey, 5, 'extended_inactivity');
        }
      }
    }
  }

  /**
   * Evict the lowest E-Score validator to make room
   * @private
   * @param {number} newEScore - E-Score of incoming validator
   * @returns {boolean} True if evicted someone
   */
  _evictLowestValidator(newEScore) {
    let lowest = null;
    let lowestScore = Infinity;

    for (const [publicKey, validator] of this._validators) {
      // Don't evict self
      if (publicKey === this._publicKey) continue;

      // Calculate effective score (E-Score + uptime bonus)
      const effectiveScore = validator.eScore * (0.5 + validator.uptime * 0.5);

      if (effectiveScore < lowestScore) {
        lowestScore = effectiveScore;
        lowest = publicKey;
      }
    }

    // Only evict if new validator has higher score
    if (lowest && newEScore > lowestScore) {
      this.removeValidator(lowest, 'evicted_for_higher_escore');
      return true;
    }

    return false;
  }

  /**
   * Get validator info
   * @param {string} publicKey - Validator public key
   * @returns {ValidatorInfo|null} Validator info or null
   */
  getValidator(publicKey) {
    return this._validators.get(publicKey) || null;
  }

  /**
   * Get all validators
   * @param {Object} [filter] - Filter options
   * @param {string} [filter.status] - Filter by status
   * @param {number} [filter.minEScore] - Minimum E-Score
   * @returns {Array<ValidatorInfo>} Validators
   */
  getValidators(filter = {}) {
    let validators = Array.from(this._validators.values());

    if (filter.status) {
      validators = validators.filter(v => v.status === filter.status);
    }

    if (filter.minEScore !== undefined) {
      validators = validators.filter(v => v.eScore >= filter.minEScore);
    }

    // Sort by effective score (E-Score * uptime)
    validators.sort((a, b) => {
      const scoreA = a.eScore * (0.5 + a.uptime * 0.5);
      const scoreB = b.eScore * (0.5 + b.uptime * 0.5);
      return scoreB - scoreA;
    });

    return validators;
  }

  /**
   * Get validator set summary
   * @returns {Object} Validator set summary
   */
  getValidatorSetStatus() {
    const validators = Array.from(this._validators.values());
    const active = validators.filter(v => v.status === 'active');
    const inactive = validators.filter(v => v.status === 'inactive');

    const totalEScore = active.reduce((sum, v) => sum + v.eScore, 0);
    const avgEScore = active.length > 0 ? totalEScore / active.length : 0;

    return {
      total: validators.length,
      active: active.length,
      inactive: inactive.length,
      maxValidators: this._validatorConfig.maxValidators,
      minEScore: this._validatorConfig.minEScore,
      totalEScore,
      avgEScore: Math.round(avgEScore * 10) / 10,
      selfIncluded: this._validators.has(this._publicKey),
      stats: {
        added: this._stats.validatorsAdded,
        removed: this._stats.validatorsRemoved,
        penalized: this._stats.validatorsPenalized,
      },
    };
  }

  /**
   * Calculate total voting weight for φ-BFT consensus
   * @returns {number} Total voting weight
   */
  getTotalVotingWeight() {
    let total = 0;
    for (const validator of this._validators.values()) {
      if (validator.status === 'active') {
        // Weight = E-Score * sqrt(burned + 1) * uptime
        const weight = validator.eScore * Math.sqrt(validator.burned + 1) * validator.uptime;
        total += weight;
      }
    }
    return total;
  }

  /**
   * Check if we have supermajority (61.8% φ-BFT)
   * @param {number} votingWeight - Weight of votes received
   * @returns {boolean} True if supermajority reached
   */
  hasSupermajority(votingWeight) {
    const total = this.getTotalVotingWeight();
    if (total === 0) return false;

    const PHI_THRESHOLD = 0.618; // φ⁻¹
    return votingWeight / total >= PHI_THRESHOLD;
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

  /**
   * Handle fork resolution request from a peer
   * @private
   */
  async _handleForkResolutionRequest(message, peerId) {
    const { forkSlot, targetHash } = message;

    log.info('Fork resolution request received', {
      fromPeer: peerId.slice(0, 16),
      forkSlot,
      targetHash: targetHash?.slice(0, 16),
    });

    // Check if we have the requested branch
    const ourHash = this._slotHashes.get(forkSlot)?.hash;
    const haveTargetBranch = ourHash === targetHash;

    if (!haveTargetBranch) {
      // We don't have the requested branch
      await this._transport?.sendTo(peerId, {
        type: 'FORK_RESOLUTION_RESPONSE',
        forkSlot,
        success: false,
        reason: 'BRANCH_NOT_AVAILABLE',
        timestamp: Date.now(),
      });
      this._stats.messagesSent++;
      return;
    }

    // Send blocks from fork slot to current finalized slot
    // TODO: In real implementation, retrieve actual blocks
    const blocks = [];
    const currentSlot = this._consensus?.lastFinalizedSlot || 0;

    await this._transport?.sendTo(peerId, {
      type: 'FORK_RESOLUTION_RESPONSE',
      forkSlot,
      success: true,
      blocks,
      fromSlot: forkSlot,
      toSlot: currentSlot,
      targetHash,
      timestamp: Date.now(),
    });
    this._stats.messagesSent++;

    this.emit('fork:resolution_provided', {
      toPeer: peerId.slice(0, 16),
      forkSlot,
      blocksProvided: blocks.length,
    });
  }

  /**
   * Handle fork resolution response
   * @private
   */
  async _handleForkResolutionResponse(message, peerId) {
    const { forkSlot, success, blocks, reason } = message;

    if (!success) {
      log.warn('Fork resolution failed', { forkSlot, reason });
      this._forkState.resolutionInProgress = false;

      this.emit('fork:resolution_failed', {
        forkSlot,
        reason,
      });
      return;
    }

    log.info('Fork resolution response received', {
      fromPeer: peerId.slice(0, 16),
      forkSlot,
      blocksReceived: blocks?.length || 0,
    });

    // TODO: In real implementation, we would:
    // 1. Validate the received blocks
    // 2. Reorg our chain to the new branch
    // 3. Update our state

    // For now, just mark fork as resolved
    this._markForkResolved();

    this.emit('fork:reorg_complete', {
      forkSlot,
      blocksApplied: blocks?.length || 0,
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

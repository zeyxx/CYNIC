/**
 * CYNICNetworkNode - Multi-Node Orchestrator
 *
 * PHASE 2: DECENTRALIZE
 *
 * Thin orchestrator that wires together extracted SRP components:
 * - ForkDetector: Chain fork detection and resolution
 * - ValidatorManager: Validator set management
 * - SolanaAnchoringManager: Onchain truth anchoring
 * - StateSyncManager: State synchronization between nodes
 * - TransportComponent: WebSocket/gossip networking
 * - ConsensusComponent: φ-BFT consensus (61.8% supermajority)
 * - PeerDiscovery: Finding and connecting to peers
 *
 * "The pack hunts together" - κυνικός
 *
 * @module @cynic/node/network/network-node
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, globalEventBus, EventType } from '@cynic/core';
import { TransportComponent } from '../components/transport-component.js';
import { ConsensusComponent } from '../components/consensus-component.js';
import { PeerDiscovery } from '../transport/discovery.js';
import { ForkDetector } from './fork-detector.js';
import { ValidatorManager } from './validator-manager.js';
import { SolanaAnchoringManager } from './solana-anchoring.js';
import { StateSyncManager } from './state-sync-manager.js';

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
   * @param {Object} options
   * @param {string} options.publicKey - Node public key
   * @param {string} options.privateKey - Node private key
   * @param {number} [options.eScore=50]
   * @param {number} [options.port=8618] - P2P port (φ-aligned)
   * @param {string[]} [options.seedNodes=[]]
   * @param {boolean} [options.enabled=true]
   * @param {Object} [options.ssl]
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

    // Core P2P components (lazy)
    this._transport = null;
    this._consensus = null;
    this._discovery = null;

    // Timers
    this._heartbeatTimer = null;
    this._syncTimer = null;
    this._validatorTimer = null;
    this._metricsTimer = null;

    // Extracted SRP components (always created)
    this._forkDetector = new ForkDetector();
    this._validatorManager = new ValidatorManager({ selfPublicKey: this._publicKey });
    this._anchoringManager = new SolanaAnchoringManager({
      enabled: options.anchoringEnabled ?? false,
      cluster: options.solanaCluster || 'devnet',
      wallet: options.wallet || null,
      dryRun: options.dryRun ?? false,
      anchorInterval: options.anchorInterval || 100,
    });
    this._stateSyncManager = new StateSyncManager();

    // Stats (node-level only; components own their own stats)
    this._stats = {
      uptime: 0,
      messagesReceived: 0,
      messagesSent: 0,
      blocksProposed: 0,
      blocksFinalized: 0,
      peersConnected: 0,
      errors: 0,
    };

    // Bubble component events
    this._wireComponentEvents();

    if (this._enabled) {
      this._initializeComponents(options);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Initialization
  // ═══════════════════════════════════════════════════════════════════════════

  /** @private */
  _initializeComponents(options) {
    this._transport = new TransportComponent({
      port: this._port,
      host: options.host || '0.0.0.0',
      publicKey: this._publicKey,
      privateKey: this._privateKey,
      ssl: options.ssl,
      onMessage: this._handleMessage.bind(this),
    });

    this._consensus = new ConsensusComponent({
      publicKey: this._publicKey,
      privateKey: this._privateKey,
      eScore: this._eScore,
      burned: options.burned || 0,
      confirmations: options.confirmations || 32,
      gossip: this._transport.gossip,
    });

    this._discovery = new PeerDiscovery({
      transport: this._transport.transport,
      seedNodes: this._seedNodes,
      minPeers: options.minPeers || 3,
      maxPeers: options.maxPeers || 50,
    });

    // Wire dependencies into extracted components
    const sendTo = (peerId, msg) => {
      this._stats.messagesSent++;
      return this._transport.sendTo(peerId, msg);
    };
    const getLastFinalizedSlot = () => this._consensus?.lastFinalizedSlot || 0;
    const getCurrentSlot = () => this._consensus?.currentSlot || 0;

    this._forkDetector.wire({
      getLastFinalizedSlot,
      sendTo,
      getPeerSlots: () => this._stateSyncManager.peerSlots,
      publicKey: this._publicKey,
    });

    this._validatorManager.wire({
      syncToConsensus: (v) => this._consensus?.registerValidator(v),
      removeFromConsensus: (pk) => this._consensus?.removeValidator(pk),
    });

    this._stateSyncManager.wire({
      getLastFinalizedSlot,
      getCurrentSlot,
      sendTo,
      publicKey: this._publicKey,
    });

    log.info('Components initialized', {
      port: this._port,
      seedNodes: this._seedNodes.length,
    });
  }

  /** @private - Bubble events from extracted components */
  _wireComponentEvents() {
    // ForkDetector events
    for (const event of ['fork:detected', 'fork:resolved', 'fork:resolution_started']) {
      this._forkDetector.on(event, (data) => this.emit(event, data));
    }

    // ValidatorManager events
    for (const event of ['validator:added', 'validator:updated', 'validator:removed',
      'validator:penalized', 'validator:rewarded', 'validator:inactive']) {
      this._validatorManager.on(event, (data) => this.emit(event, data));
    }

    // Anchoring events
    for (const event of ['block:anchored', 'anchor:failed', 'anchoring:enabled', 'anchoring:disabled']) {
      this._anchoringManager.on(event, (data) => this.emit(event, data));
    }

    // StateSync events
    for (const event of ['sync:needed', 'sync:complete', 'sync:blocks_received',
      'sync:blocks_sent', 'fork:resolution_provided', 'fork:resolution_failed', 'fork:reorg_complete']) {
      this._stateSyncManager.on(event, (data) => this.emit(event, data));
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Lifecycle
  // ═══════════════════════════════════════════════════════════════════════════

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
      await this._transport.start();
      this._wireTransportEvents();
      this._wireConsensusEvents();

      this._discovery.start();
      this._wireDiscoveryEvents();

      this._consensus.start({ eScore: this._eScore });
      this._startPeriodicTasks();

      globalEventBus.publish(EventType.NODE_STARTED, {
        nodeId: this._publicKey.slice(0, 16),
        port: this._port,
        eScore: this._eScore,
        timestamp: Date.now(),
      });

      this._state = NetworkState.ONLINE;
      this.emit('started', { nodeId: this._publicKey.slice(0, 16), port: this._port });

      log.info('Network node started', {
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

  async stop() {
    this._anchoringManager.cleanup();

    if (this._state === NetworkState.OFFLINE) return;

    log.info('Stopping network node...');

    this._stopPeriodicTasks();
    this._discovery?.stop();
    this._consensus?.stop();
    await this._transport?.stop();

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
  // Event Wiring (Transport / Consensus / Discovery)
  // ═══════════════════════════════════════════════════════════════════════════

  /** @private */
  _wireTransportEvents() {
    this._transport.wireEvents({
      onPeerConnected: (peerId, publicKey, address) => {
        this._stats.peersConnected = this._transport.getConnectedPeers().length;
        this._consensus.registerValidator({
          publicKey: publicKey || peerId,
          eScore: 50,
        });
        this.emit('peer:connected', { peerId, publicKey, address });
      },
      onPeerDisconnected: (peerId, code, reason) => {
        this._stats.peersConnected = this._transport.getConnectedPeers().length;
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

  /** @private */
  _wireConsensusEvents() {
    this._consensus.wireEvents({
      onBlockFinalized: async (blockHash, slot, block) => {
        this._stats.blocksFinalized++;

        globalEventBus.publish(EventType.BLOCK_FINALIZED, {
          blockHash, slot, block, timestamp: Date.now(),
        });

        this.emit('block:finalized', { blockHash, slot, block });

        await this._anchoringManager.onBlockFinalized(
          { slot, hash: blockHash, ...block },
          (s, h) => this._forkDetector.recordBlockHash(s, h),
        );
      },
      onBlockConfirmed: (slot, ratio) => {
        this.emit('block:confirmed', { slot, ratio });
      },
      onConsensusStarted: (slot) => {
        if (this._state === NetworkState.ONLINE || this._state === NetworkState.SYNCING) {
          this._state = NetworkState.PARTICIPATING;
        }
        this.emit('consensus:started', { slot });
      },
    });
  }

  /** @private */
  _wireDiscoveryEvents() {
    this._discovery.on('peer:discovered', (event) => this.emit('peer:discovered', event));
    this._discovery.on('started', () => log.info('Peer discovery started'));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Periodic Tasks
  // ═══════════════════════════════════════════════════════════════════════════

  /** @private */
  _startPeriodicTasks() {
    this._heartbeatTimer = setInterval(() => this._publishHeartbeat(), NETWORK_INTERVALS.HEARTBEAT_MS);
    this._syncTimer = setInterval(() => this._checkStateSync(), NETWORK_INTERVALS.STATE_SYNC_MS);
    this._validatorTimer = setInterval(() => this._updateValidatorSet(), NETWORK_INTERVALS.VALIDATOR_CHECK_MS);
    this._metricsTimer = setInterval(() => this._reportMetrics(), NETWORK_INTERVALS.METRICS_REPORT_MS);
  }

  /** @private */
  _stopPeriodicTasks() {
    for (const timer of [this._heartbeatTimer, this._syncTimer, this._validatorTimer, this._metricsTimer]) {
      if (timer) clearInterval(timer);
    }
    this._heartbeatTimer = this._syncTimer = this._validatorTimer = this._metricsTimer = null;
  }

  /** @private */
  _publishHeartbeat() {
    const recentHashes = this._forkDetector.getRecentBlockHashes(5);

    this._transport.gossip?.broadcastMessage?.({
      type: 'HEARTBEAT',
      nodeId: this._publicKey.slice(0, 32),
      eScore: this._eScore,
      slot: this._consensus?.currentSlot || 0,
      finalizedSlot: this._consensus?.lastFinalizedSlot || 0,
      finalizedHash: this._consensus?.lastFinalizedHash || null,
      recentHashes,
      state: this._state,
      timestamp: Date.now(),
    });
  }

  /** @private */
  _checkStateSync() {
    const result = this._stateSyncManager.checkStateSync();

    if (result.needsSync && this._state !== NetworkState.SYNCING) {
      this._state = NetworkState.SYNCING;
      log.info('Node is behind network', { behindBy: result.behindBy });
    } else if (result.justCompleted && this._state === NetworkState.SYNCING) {
      this._state = NetworkState.ONLINE;
    }
  }

  /** @private */
  _updateValidatorSet() {
    this._validatorManager.checkInactiveValidators();

    const connectedPeers = this._transport?.getConnectedPeers() || [];
    for (const peerId of connectedPeers) {
      const peerInfo = this._stateSyncManager.peerSlots.get(peerId);
      if (peerInfo && !this._validatorManager.getValidator(peerId)) {
        if (peerInfo.eScore >= 20) {
          this._validatorManager.addValidator({
            publicKey: peerId,
            eScore: peerInfo.eScore,
          });
        }
      }
    }

    this._validatorManager.applyPenaltyDecay();
  }

  /** @private */
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
  // Message Routing
  // ═══════════════════════════════════════════════════════════════════════════

  /** @private */
  async _handleMessage(message, peerId) {
    this._stats.messagesReceived++;

    switch (message.type) {
      case 'HEARTBEAT':
        return this._handleHeartbeat(message, peerId);
      case 'STATE_REQUEST':
        return this._stateSyncManager.handleStateRequest(message, peerId);
      case 'STATE_RESPONSE':
        return this._stateSyncManager.handleStateResponse(message, peerId);
      case 'BLOCK_REQUEST':
        return this._stateSyncManager.handleBlockRequest(message, peerId);
      case 'VALIDATOR_UPDATE':
        return this._stateSyncManager.handleValidatorUpdate(
          message,
          (v) => this._consensus?.registerValidator(v),
          (pk) => this._consensus?.removeValidator(pk),
        );
      case 'FORK_RESOLUTION_REQUEST':
        return this._stateSyncManager.handleForkResolutionRequest(
          message, peerId,
          (slot) => this._forkDetector._slotHashes.get(slot)?.hash,
        );
      case 'FORK_RESOLUTION_RESPONSE':
        return this._stateSyncManager.handleForkResolutionResponse(
          message, peerId,
          () => this._forkDetector.markForkResolved(),
        );
      default:
        break;
    }
  }

  /** @private */
  async _handleHeartbeat(message, peerId) {
    const { eScore, finalizedSlot, finalizedHash, slot, state, recentHashes, nodeId } = message;

    this._stateSyncManager.updatePeer(peerId, { finalizedSlot, finalizedHash, slot, state, eScore });
    this._validatorManager.updateValidatorActivity(peerId);

    if (eScore && eScore >= 20) {
      this._validatorManager.addValidator({ publicKey: peerId, eScore });
    }

    if (recentHashes && recentHashes.length > 0) {
      this._forkDetector.checkForForks(peerId, recentHashes, eScore || 50);
    }

    this.emit('heartbeat:received', {
      peerId, nodeId, finalizedSlot, finalizedHash, eScore,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Public API - Delegated to components
  // ═══════════════════════════════════════════════════════════════════════════

  async connectToPeer(peer) { await this._transport.connectToPeer(peer); }

  addSeedNode(address) {
    this._seedNodes.push(address);
    this._discovery?.addSeedNode(address);
  }

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

  async broadcastJudgment(judgment) {
    if (this._state === NetworkState.OFFLINE) return;
    await this._transport.broadcastJudgment(judgment);
    this._stats.messagesSent++;
  }

  async broadcastPattern(pattern) {
    if (this._state === NetworkState.OFFLINE) return;
    await this._transport.broadcastPattern(pattern);
    this._stats.messagesSent++;
  }

  // Validator delegation
  addValidator(v) { return this._validatorManager.addValidator(v); }
  removeValidator(pk, reason) { return this._validatorManager.removeValidator(pk, reason); }
  penalizeValidator(pk, penalty, reason) { return this._validatorManager.penalizeValidator(pk, penalty, reason); }
  rewardValidator(pk, action) { this._validatorManager.rewardValidator(pk, action); }
  updateValidatorActivity(pk) { this._validatorManager.updateValidatorActivity(pk); }
  getValidator(pk) { return this._validatorManager.getValidator(pk); }
  getValidators(filter) { return this._validatorManager.getValidators(filter); }
  getValidatorSetStatus() { return this._validatorManager.getValidatorSetStatus(); }
  getTotalVotingWeight() { return this._validatorManager.getTotalVotingWeight(); }
  hasSupermajority(w) { return this._validatorManager.hasSupermajority(w); }

  // Block store wiring (injected by external code, e.g. unified-orchestrator)
  wireBlockStore({ getBlocks, storeBlock }) {
    this._stateSyncManager.wire({ getBlocks, storeBlock });
  }

  // Fork detection delegation
  recordBlockHash(slot, hash) { this._forkDetector.recordBlockHash(slot, hash); }
  getForkStatus() { return this._forkDetector.getForkStatus(); }

  // Anchoring delegation
  async enableAnchoring(opts) { return this._anchoringManager.enable(opts); }
  disableAnchoring() { this._anchoringManager.disable(); }
  async anchorBlock(block) { return this._anchoringManager.anchorBlock(block); }
  shouldAnchor(slot) { return this._anchoringManager.shouldAnchor(slot); }
  getAnchorStatus(hash) { return this._anchoringManager.getAnchorStatus(hash); }
  getAnchoringStatus() { return this._anchoringManager.getAnchoringStatus(); }
  async verifyAnchor(sig) { return this._anchoringManager.verifyAnchor(sig); }

  async onBlockFinalized(block) {
    await this._anchoringManager.onBlockFinalized(
      block,
      (s, h) => this._forkDetector.recordBlockHash(s, h),
    );
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // State & Stats
  // ═══════════════════════════════════════════════════════════════════════════

  get state() { return this._state; }
  get isOnline() { return this._state === NetworkState.ONLINE || this._state === NetworkState.PARTICIPATING; }
  get isParticipating() { return this._state === NetworkState.PARTICIPATING; }
  get publicKey() { return this._publicKey; }
  get eScore() { return this._eScore; }

  setEScore(score) {
    this._eScore = Math.min(Math.max(score, 0), 100);
    this._consensus?.registerValidator({
      publicKey: this._publicKey,
      eScore: this._eScore,
    });
  }

  getInfo() {
    return {
      publicKey: this._publicKey.slice(0, 32) + '...',
      port: this._port,
      eScore: this._eScore,
      state: this._state,
      uptime: this._startedAt ? Date.now() - this._startedAt : 0,
      stats: {
        ...this._stats,
        ...this._forkDetector.stats,
        ...this._validatorManager.stats,
        ...this._anchoringManager.stats,
      },
    };
  }

  getStatus() {
    return {
      node: this.getInfo(),
      transport: this._transport?.getInfo() || null,
      consensus: this._consensus?.getInfo() || null,
      discovery: this._discovery?.getStats() || null,
      sync: this._stateSyncManager.syncState,
    };
  }

  getConnectedPeers() { return this._transport?.getConnectedPeers() || []; }
  getValidatorCount() { return this._consensus?.validatorCount || 0; }
}

export default CYNICNetworkNode;

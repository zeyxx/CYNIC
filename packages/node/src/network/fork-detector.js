/**
 * ForkDetector - Chain fork detection and resolution
 *
 * Extracted from CYNICNetworkNode monolith (BURN)
 *
 * Detects when peers report different block hashes for the same slot,
 * calculates heaviest branch by E-Score weighting, and coordinates
 * fork resolution via transport.
 *
 * "Two dogs, two bones - someone's wrong" - κυνικός
 *
 * @module @cynic/node/network/fork-detector
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger } from '@cynic/core';

const log = createLogger('ForkDetector');

export class ForkDetector extends EventEmitter {
  constructor() {
    super();

    this._forkState = {
      detected: false,
      forkSlot: null,
      forkHashes: new Map(),  // slot -> Map<hash, {peers: Set, totalEScore: number}>
      ourBranch: null,
      resolutionInProgress: false,
      lastCheck: 0,
    };

    this._slotHashes = new Map(); // slot -> { hash, confirmedAt }

    this._stats = {
      forksDetected: 0,
      forksResolved: 0,
    };

    // Injected dependencies (set via wire())
    this._getLastFinalizedSlot = () => 0;
    this._sendTo = null;
    this._getPeerSlots = () => new Map();
    this._publicKey = null;
  }

  /**
   * Wire external dependencies
   * @param {Object} deps
   * @param {Function} deps.getLastFinalizedSlot - Returns last finalized slot
   * @param {Function} deps.sendTo - Send message to peer (peerId, msg)
   * @param {Function} deps.getPeerSlots - Returns peer slots Map
   * @param {string} deps.publicKey - Our node public key
   */
  wire({ getLastFinalizedSlot, sendTo, getPeerSlots, publicKey }) {
    if (getLastFinalizedSlot) this._getLastFinalizedSlot = getLastFinalizedSlot;
    if (sendTo) this._sendTo = sendTo;
    if (getPeerSlots) this._getPeerSlots = getPeerSlots;
    if (publicKey) this._publicKey = publicKey;
  }

  /**
   * Check for chain forks based on peer's recent block hashes
   * @param {string} peerId - Peer ID
   * @param {Array<{slot: number, hash: string}>} peerHashes - Peer's recent block hashes
   * @param {number} peerEScore - Peer's E-Score for weighting
   */
  checkForForks(peerId, peerHashes, peerEScore) {
    for (const { slot, hash } of peerHashes) {
      if (!hash) continue;

      if (!this._forkState.forkHashes.has(slot)) {
        this._forkState.forkHashes.set(slot, new Map());
      }

      const slotForks = this._forkState.forkHashes.get(slot);

      if (!slotForks.has(hash)) {
        slotForks.set(hash, { peers: new Set(), totalEScore: 0 });
      }

      const hashInfo = slotForks.get(hash);
      if (!hashInfo.peers.has(peerId)) {
        hashInfo.peers.add(peerId);
        hashInfo.totalEScore += peerEScore;
      }

      // Multiple hashes for same slot = FORK
      if (slotForks.size > 1 && !this._forkState.detected) {
        this._onForkDetected(slot, slotForks);
      }
    }

    this._cleanupForkData();
  }

  /**
   * Handle fork detection
   * @private
   */
  _onForkDetected(slot, forks) {
    this._forkState.detected = true;
    this._forkState.forkSlot = slot;
    this._stats.forksDetected++;

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

    log.warn('FORK DETECTED', {
      slot,
      branches: forks.size,
      forks: forkDetails,
      heaviestBranch: heaviestHash?.slice(0, 16),
    });

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

    if (!onHeaviestBranch && !this._forkState.resolutionInProgress) {
      this._resolveFork(slot, heaviestHash);
    }
  }

  /**
   * Attempt to resolve a fork by switching to the heaviest branch
   * @private
   */
  async _resolveFork(forkSlot, targetHash) {
    this._forkState.resolutionInProgress = true;

    log.info('Attempting fork resolution', {
      forkSlot,
      targetBranch: targetHash?.slice(0, 16),
    });

    const forkInfo = this._forkState.forkHashes.get(forkSlot)?.get(targetHash);
    if (!forkInfo || forkInfo.peers.size === 0) {
      log.warn('No peers found on target branch');
      this._forkState.resolutionInProgress = false;
      return;
    }

    // Find peer with highest E-Score on target branch
    let bestPeer = null;
    let bestScore = 0;
    const peerSlots = this._getPeerSlots();

    for (const peerId of forkInfo.peers) {
      const peerInfo = peerSlots.get(peerId);
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

    try {
      await this._sendTo?.(bestPeer, {
        type: 'FORK_RESOLUTION_REQUEST',
        forkSlot,
        targetHash,
        nodeId: this._publicKey?.slice(0, 32),
        timestamp: Date.now(),
      });

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
   */
  markForkResolved() {
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
   * Cleanup old fork tracking data (keep last 100 slots)
   * @private
   */
  _cleanupForkData() {
    const currentSlot = this._getLastFinalizedSlot();
    const keepFrom = currentSlot - 100;

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
   * Record a block hash for a slot
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
   * Get recent block hashes for heartbeat
   * @param {number} count - Number of recent slots
   * @returns {Array<{slot: number, hash: string}>}
   */
  getRecentBlockHashes(count) {
    const hashes = [];
    const currentSlot = this._getLastFinalizedSlot();

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
      stats: { ...this._stats },
    };
  }

  /** @returns {Object} Stats */
  get stats() {
    return { ...this._stats };
  }
}

export default ForkDetector;

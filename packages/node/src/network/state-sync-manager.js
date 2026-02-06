/**
 * StateSyncManager - State synchronization between nodes
 *
 * Extracted from CYNICNetworkNode monolith (BURN)
 *
 * Tracks peer slots, detects when we're behind, requests state sync,
 * handles state/block request/response messages.
 *
 * "The pack stays together" - κυνικός
 *
 * @module @cynic/node/network/state-sync-manager
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger } from '@cynic/core';

const log = createLogger('StateSyncManager');

export class StateSyncManager extends EventEmitter {
  constructor() {
    super();

    this._syncState = {
      lastSyncSlot: 0,
      syncing: false,
      behindBy: 0,
      syncInProgress: false,
      lastSyncAttempt: 0,
    };

    this._peerSlots = new Map(); // peerId -> { finalizedSlot, finalizedHash, slot, state, eScore, lastSeen }

    // Injected dependencies (set via wire())
    this._getLastFinalizedSlot = () => 0;
    this._getCurrentSlot = () => 0;
    this._sendTo = null;
    this._publicKey = null;
    this._getBlocks = async () => [];     // (fromSlot, toSlot) => Promise<Block[]>
    this._storeBlock = async () => null;  // (block) => Promise<Object|null>
  }

  /**
   * Wire external dependencies
   * @param {Object} deps
   * @param {Function} deps.getLastFinalizedSlot
   * @param {Function} deps.getCurrentSlot
   * @param {Function} deps.sendTo - (peerId, message) => Promise
   * @param {string} deps.publicKey
   * @param {Function} [deps.getBlocks] - (fromSlot, toSlot) => Promise<Block[]>
   * @param {Function} [deps.storeBlock] - (block) => Promise<Object|null>
   */
  wire({ getLastFinalizedSlot, getCurrentSlot, sendTo, publicKey, getBlocks, storeBlock }) {
    if (getLastFinalizedSlot) this._getLastFinalizedSlot = getLastFinalizedSlot;
    if (getCurrentSlot) this._getCurrentSlot = getCurrentSlot;
    if (sendTo) this._sendTo = sendTo;
    if (publicKey) this._publicKey = publicKey;
    if (getBlocks) this._getBlocks = getBlocks;
    if (storeBlock) this._storeBlock = storeBlock;
  }

  /**
   * Update peer info from heartbeat
   * @param {string} peerId
   * @param {Object} heartbeat - { finalizedSlot, finalizedHash, slot, state, eScore }
   */
  updatePeer(peerId, heartbeat) {
    this._peerSlots.set(peerId, {
      finalizedSlot: heartbeat.finalizedSlot || 0,
      finalizedHash: heartbeat.finalizedHash || null,
      slot: heartbeat.slot || 0,
      state: heartbeat.state || 'UNKNOWN',
      eScore: heartbeat.eScore || 50,
      lastSeen: Date.now(),
    });
  }

  /**
   * Check if we need to sync state
   * @returns {Object|null} { behindBy, bestPeer } if sync needed, null otherwise
   */
  checkStateSync() {
    const ourSlot = this._getLastFinalizedSlot();
    this._syncState.lastSyncSlot = ourSlot;

    let highestPeerSlot = ourSlot;
    let bestPeer = null;

    for (const [peerId, peerInfo] of this._peerSlots) {
      if (Date.now() - peerInfo.lastSeen < 60000) {
        if (peerInfo.finalizedSlot > highestPeerSlot) {
          highestPeerSlot = peerInfo.finalizedSlot;
          bestPeer = peerId;
        }
      }
    }

    this._syncState.behindBy = highestPeerSlot - ourSlot;

    if (this._syncState.behindBy > 10 && !this._syncState.syncInProgress) {
      if (bestPeer) {
        this._requestStateSync(bestPeer, ourSlot);
      }

      this.emit('sync:needed', { behindBy: this._syncState.behindBy, bestPeer });
      return { behindBy: this._syncState.behindBy, bestPeer, needsSync: true };
    } else if (this._syncState.behindBy <= 10 && this._syncState.syncInProgress) {
      this._syncState.syncInProgress = false;
      log.info('Sync complete - caught up with network', { slot: ourSlot });
      this.emit('sync:complete', { slot: ourSlot });
      return { behindBy: this._syncState.behindBy, bestPeer: null, needsSync: false, justCompleted: true };
    }

    return { behindBy: this._syncState.behindBy, bestPeer: null, needsSync: false };
  }

  /**
   * Request state sync from a peer
   * @private
   */
  async _requestStateSync(peerId, fromSlot) {
    if (Date.now() - this._syncState.lastSyncAttempt < 5000) return;

    this._syncState.syncInProgress = true;
    this._syncState.lastSyncAttempt = Date.now();

    log.info('Requesting state sync', { peerId: peerId.slice(0, 16), fromSlot });

    try {
      await this._sendTo?.(peerId, {
        type: 'STATE_REQUEST',
        fromSlot,
        nodeId: this._publicKey?.slice(0, 32),
        timestamp: Date.now(),
      });
    } catch (error) {
      log.warn('State sync request failed', { peerId: peerId.slice(0, 16), error: error.message });
      this._syncState.syncInProgress = false;
    }
  }

  /**
   * Handle STATE_REQUEST from peer
   * @param {Object} message - { fromSlot }
   * @param {string} peerId
   */
  async handleStateRequest(message, peerId) {
    const { fromSlot } = message;
    const finalizedSlot = this._getLastFinalizedSlot();

    const blocks = await this._getBlocks(fromSlot, finalizedSlot);

    await this._sendTo?.(peerId, {
      type: 'STATE_RESPONSE',
      fromSlot,
      currentSlot: this._getCurrentSlot(),
      finalizedSlot,
      blocks,
    });
  }

  /**
   * Handle STATE_RESPONSE from peer
   * @param {Object} message - { finalizedSlot, blocks, stateRoot }
   * @param {string} peerId
   */
  async handleStateResponse(message, peerId) {
    const { finalizedSlot, blocks } = message;
    const ourSlot = this._getLastFinalizedSlot();

    this._syncState.syncInProgress = false;

    if (finalizedSlot > ourSlot && blocks && blocks.length > 0) {
      log.info('Received state sync response', {
        ourSlot,
        theirSlot: finalizedSlot,
        blocksReceived: blocks.length,
      });

      let processedCount = 0;
      for (const block of blocks) {
        try {
          if (block.slot > ourSlot) {
            await this._storeBlock(block);
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
      log.info('Peer is ahead but no blocks received, requesting blocks', {
        ourSlot,
        theirSlot: finalizedSlot,
      });

      await this._sendTo?.(peerId, {
        type: 'BLOCK_REQUEST',
        fromSlot: ourSlot + 1,
        toSlot: finalizedSlot,
        nodeId: this._publicKey?.slice(0, 32),
        timestamp: Date.now(),
      });
    } else {
      log.debug?.('State response - we are caught up', { ourSlot, theirSlot: finalizedSlot });
    }
  }

  /**
   * Handle BLOCK_REQUEST from peer
   * @param {Object} message - { fromSlot, toSlot }
   * @param {string} peerId
   */
  async handleBlockRequest(message, peerId) {
    const { fromSlot, toSlot } = message;

    log.info('Block request received', { fromPeer: peerId.slice(0, 16), fromSlot, toSlot });

    const blocks = await this._getBlocks(fromSlot, toSlot);

    await this._sendTo?.(peerId, {
      type: 'STATE_RESPONSE',
      fromSlot,
      finalizedSlot: this._getLastFinalizedSlot(),
      blocks,
      stateRoot: null,
      timestamp: Date.now(),
    });

    this.emit('sync:blocks_sent', {
      toPeer: peerId,
      fromSlot,
      toSlot,
      blocksSent: blocks.length,
    });
  }

  /**
   * Handle FORK_RESOLUTION_REQUEST from peer
   * @param {Object} message - { forkSlot, targetHash }
   * @param {string} peerId
   * @param {Function} getSlotHash - (slot) => hash or null
   */
  async handleForkResolutionRequest(message, peerId, getSlotHash) {
    const { forkSlot, targetHash } = message;

    log.info('Fork resolution request received', {
      fromPeer: peerId.slice(0, 16),
      forkSlot,
      targetHash: targetHash?.slice(0, 16),
    });

    const ourHash = getSlotHash(forkSlot);
    const haveTargetBranch = ourHash === targetHash;

    if (!haveTargetBranch) {
      await this._sendTo?.(peerId, {
        type: 'FORK_RESOLUTION_RESPONSE',
        forkSlot,
        success: false,
        reason: 'BRANCH_NOT_AVAILABLE',
        timestamp: Date.now(),
      });
      return;
    }

    const currentSlot = this._getLastFinalizedSlot();
    const blocks = await this._getBlocks(forkSlot, currentSlot);

    await this._sendTo?.(peerId, {
      type: 'FORK_RESOLUTION_RESPONSE',
      forkSlot,
      success: true,
      blocks,
      fromSlot: forkSlot,
      toSlot: currentSlot,
      targetHash,
      timestamp: Date.now(),
    });

    this.emit('fork:resolution_provided', {
      toPeer: peerId.slice(0, 16),
      forkSlot,
      blocksProvided: blocks.length,
    });
  }

  /**
   * Handle FORK_RESOLUTION_RESPONSE from peer
   * @param {Object} message - { forkSlot, success, blocks, reason }
   * @param {string} peerId
   * @param {Function} markForkResolved - ForkDetector's markForkResolved
   */
  async handleForkResolutionResponse(message, peerId, markForkResolved) {
    const { forkSlot, success, blocks, reason } = message;

    if (!success) {
      log.warn('Fork resolution failed', { forkSlot, reason });
      this.emit('fork:resolution_failed', { forkSlot, reason });
      return;
    }

    log.info('Fork resolution response received', {
      fromPeer: peerId.slice(0, 16),
      forkSlot,
      blocksReceived: blocks?.length || 0,
    });

    // Validate chain integrity: blocks must be ordered and hash-linked
    let valid = true;
    if (blocks && blocks.length > 0) {
      for (let i = 1; i < blocks.length; i++) {
        if (blocks[i].prev_hash && blocks[i - 1].hash &&
            blocks[i].prev_hash !== blocks[i - 1].hash) {
          log.warn('Fork resolution blocks have broken chain', {
            slot: blocks[i].slot,
            expected: blocks[i - 1].hash?.slice(0, 16),
            actual: blocks[i].prev_hash?.slice(0, 16),
          });
          valid = false;
          break;
        }
      }
    }

    if (!valid) {
      this.emit('fork:resolution_failed', { forkSlot, reason: 'INVALID_CHAIN' });
      return;
    }

    // Store validated blocks
    let appliedCount = 0;
    if (blocks && blocks.length > 0) {
      for (const block of blocks) {
        try {
          await this._storeBlock(block);
          appliedCount++;
        } catch (error) {
          log.warn('Failed to store fork resolution block', { slot: block.slot, error: error.message });
        }
      }
    }

    markForkResolved?.();

    this.emit('fork:reorg_complete', {
      forkSlot,
      blocksApplied: appliedCount,
    });
  }

  /**
   * Handle VALIDATOR_UPDATE from peer
   * @param {Object} message - { validator, action }
   * @param {Function} registerValidator
   * @param {Function} removeValidator
   */
  async handleValidatorUpdate(message, registerValidator, removeValidator) {
    const { validator, action } = message;

    if (action === 'ADD' && validator) {
      registerValidator?.(validator);
    } else if (action === 'REMOVE' && validator?.publicKey) {
      removeValidator?.(validator.publicKey);
    }
  }

  /** @returns {Object} Sync state snapshot */
  get syncState() {
    return { ...this._syncState };
  }

  /** @returns {Map} Peer slots */
  get peerSlots() {
    return this._peerSlots;
  }
}

export default StateSyncManager;

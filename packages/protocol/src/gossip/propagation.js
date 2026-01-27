/**
 * Gossip Propagation
 *
 * Push-pull hybrid propagation with Fibonacci fanout
 *
 * @module @cynic/protocol/gossip/propagation
 */

'use strict';

import { GOSSIP_FANOUT, createLogger } from '@cynic/core';

const log = createLogger('GossipPropagation');
import { PeerManager, createPeerInfo } from './peer.js';
import {
  MessageType,
  createMessage,
  createBlockMessage,
  createSyncRequest,
  createSyncResponse,
  verifyMessage,
  shouldRelay,
  prepareRelay,
  createConsensusBlockProposal,
  createConsensusVote,
  createConsensusVoteAggregate,
  createConsensusFinality,
  createConsensusSlotStatus,
  createConsensusStateRequest,
  createConsensusStateResponse,
  isConsensusGossipMessage,
} from './message.js';

/**
 * Gossip Protocol - Handles message propagation
 */
export class GossipProtocol {
  /**
   * @param {Object} params - Protocol parameters
   * @param {string} params.publicKey - Node public key
   * @param {string} params.privateKey - Node private key
   * @param {string} params.address - Node network address
   * @param {Function} [params.onMessage] - Message handler callback
   * @param {Function} [params.sendFn] - Function to send messages to peers
   * @param {number} [params.maxPendingRequests=1000] - Max concurrent pending requests
   * @param {number} [params.requestMaxAgeMs=60000] - Max age for pending requests before cleanup
   */
  constructor({ publicKey, privateKey, address, onMessage, sendFn, maxPendingRequests, requestMaxAgeMs }) {
    this.publicKey = publicKey;
    this.privateKey = privateKey;
    this.address = address;
    this.onMessage = onMessage || (() => {});
    this.sendFn = sendFn || (() => Promise.resolve());
    this.maxPendingRequests = maxPendingRequests || 1000;
    this.requestMaxAgeMs = requestMaxAgeMs || 60000; // 1 minute

    this.peerManager = new PeerManager();
    this.messageQueue = [];
    this.pendingRequests = new Map(); // requestId -> { resolve, reject, createdAt }
  }

  /**
   * Handle incoming message
   * @param {Object} message - Received message
   * @param {string} fromPeerId - Sender peer ID
   * @returns {Promise<void>}
   */
  async handleMessage(message, fromPeerId) {
    // Check if already seen
    if (this.peerManager.hasSeenMessage(message.id)) {
      return;
    }
    this.peerManager.markMessageSeen(message.id);

    // Update peer activity
    this.peerManager.updateActivity(fromPeerId, 'received');

    // Verify signature for important messages
    if (message.type !== MessageType.HEARTBEAT) {
      if (!verifyMessage(message)) {
        log.warn('Invalid message signature', { fromPeerId });
        this.peerManager.recordFailure(fromPeerId);
        return;
      }
    }

    // Handle by type
    switch (message.type) {
      case MessageType.BLOCK:
        await this._handleBlock(message);
        break;
      case MessageType.JUDGMENT:
        await this._handleJudgment(message);
        break;
      case MessageType.PATTERN:
        await this._handlePattern(message);
        break;
      case MessageType.SYNC_REQUEST:
        await this._handleSyncRequest(message, fromPeerId);
        break;
      case MessageType.SYNC_RESPONSE:
        await this._handleSyncResponse(message);
        break;
      case MessageType.HEARTBEAT:
        this._handleHeartbeat(message, fromPeerId);
        break;
      case MessageType.PEER_ANNOUNCE:
        this._handlePeerAnnounce(message);
        break;
      // Consensus messages (Layer 4)
      case MessageType.CONSENSUS_BLOCK_PROPOSAL:
      case MessageType.CONSENSUS_VOTE:
      case MessageType.CONSENSUS_VOTE_AGGREGATE:
      case MessageType.CONSENSUS_FINALITY:
      case MessageType.CONSENSUS_SLOT_STATUS:
        // Consensus messages are delegated to onMessage callback
        // The ConsensusGossip bridge will handle them
        break;
      default:
        // Check if it's a consensus message we don't explicitly handle
        if (!isConsensusGossipMessage(message.type)) {
          log.warn('Unknown message type', { type: message.type });
        }
    }

    // Relay if appropriate
    if (shouldRelay(message)) {
      await this._relay(message, new Set([fromPeerId]));
    }

    // Notify handler
    await this.onMessage(message);
  }

  /**
   * Handle block message
   * @private
   */
  async _handleBlock(message) {
    // Block handling is delegated to onMessage callback
    // This method can add validation logic
  }

  /**
   * Handle judgment message
   * @private
   */
  async _handleJudgment(message) {
    // Judgment handling is delegated to onMessage callback
  }

  /**
   * Handle pattern message
   * @private
   */
  async _handlePattern(message) {
    // Pattern handling is delegated to onMessage callback
  }

  /**
   * Handle sync request
   * @private
   */
  async _handleSyncRequest(message, fromPeerId) {
    // Emit event for chain to respond
    // Response is sent via sendSyncResponse
  }

  /**
   * Handle sync response
   * @private
   */
  async _handleSyncResponse(message) {
    const requestId = message.payload?.request_id;
    const resolver = this.pendingRequests.get(requestId);
    if (resolver) {
      resolver.resolve(message.payload.blocks);
      this.pendingRequests.delete(requestId);
    }
  }

  /**
   * Handle heartbeat
   * @private
   */
  _handleHeartbeat(message, fromPeerId) {
    const peer = this.peerManager.getPeer(fromPeerId);
    if (peer) {
      peer.lastSeen = Date.now();
      if (message.payload?.eScore) {
        peer.eScore = message.payload.eScore;
      }
    }
  }

  /**
   * Handle peer announcement
   * @private
   */
  _handlePeerAnnounce(message) {
    const peerInfo = message.payload;
    if (peerInfo && peerInfo.publicKey !== this.publicKey) {
      this.peerManager.addPeer(createPeerInfo(peerInfo));
    }
  }

  /**
   * Broadcast message to gossip network
   * @param {Object} message - Message to broadcast
   * @param {Set<string>} [exclude] - Peers to exclude
   * @returns {Promise<number>} Number of peers sent to
   */
  async broadcast(message, exclude = new Set()) {
    this.peerManager.markMessageSeen(message.id);

    const peers = this.peerManager.selectGossipPeers(GOSSIP_FANOUT, exclude);
    let sent = 0;

    const errors = [];
    for (const peer of peers) {
      try {
        await this.sendFn(peer, message);
        this.peerManager.updateActivity(peer.id, 'sent');
        sent++;
      } catch (err) {
        this.peerManager.recordFailure(peer.id);
        errors.push({ peerId: peer.id, error: err.message || String(err) });
      }
    }

    // Log errors if any occurred (don't throw - broadcast should be best-effort)
    if (errors.length > 0) {
      log.warn('Broadcast failures', { failedCount: errors.length, totalPeers: peers.length, peerIds: errors.map(e => e.peerId) });
    }

    return sent;
  }

  /**
   * Relay message to other peers
   * @private
   */
  async _relay(message, exclude) {
    const relayMessage = prepareRelay(message);
    await this.broadcast(relayMessage, exclude);
  }

  /**
   * Broadcast a block
   * @param {Object} block - Block to broadcast
   * @returns {Promise<number>} Number of peers sent to
   */
  async broadcastBlock(block) {
    const message = createBlockMessage(block, this.publicKey, this.privateKey);
    return this.broadcast(message);
  }

  /**
   * Broadcast a judgment
   * @param {Object} judgment - Judgment to broadcast
   * @returns {Promise<number>} Number of peers sent to
   */
  async broadcastJudgment(judgment) {
    const message = createMessage({
      type: MessageType.JUDGMENT,
      payload: judgment,
      sender: this.publicKey,
      privateKey: this.privateKey,
    });
    return this.broadcast(message);
  }

  /**
   * Broadcast a pattern
   * @param {Object} pattern - Pattern to broadcast
   * @returns {Promise<number>} Number of peers sent to
   */
  async broadcastPattern(pattern) {
    const message = createMessage({
      type: MessageType.PATTERN,
      payload: pattern,
      sender: this.publicKey,
      privateKey: this.privateKey,
    });
    return this.broadcast(message);
  }

  /**
   * Request sync from peer
   * @param {string} peerId - Peer to request from
   * @param {number} sinceSlot - Request blocks since slot
   * @param {number} [timeoutMs=5000] - Request timeout
   * @returns {Promise<Object[]>} Blocks received
   */
  async requestSync(peerId, sinceSlot, timeoutMs = 5000) {
    const peer = this.peerManager.getPeer(peerId);
    if (!peer) {
      throw new Error(`Unknown peer: ${peerId}`);
    }

    const request = createSyncRequest(sinceSlot, this.publicKey, this.privateKey);

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(request.id);
        reject(new Error('Sync request timeout'));
      }, timeoutMs);

      this._addPendingRequest(request.id, {
        resolve: (blocks) => {
          clearTimeout(timeout);
          resolve(blocks);
        },
        reject: (err) => {
          clearTimeout(timeout);
          reject(err);
        },
      });

      this.sendFn(peer, request).catch((err) => {
        clearTimeout(timeout);
        this.pendingRequests.delete(request.id);
        reject(err);
      });
    });
  }

  /**
   * Send sync response
   * @param {string} peerId - Peer to respond to
   * @param {string} requestId - Original request ID
   * @param {Object[]} blocks - Blocks to send
   */
  async sendSyncResponse(peerId, requestId, blocks) {
    const peer = this.peerManager.getPeer(peerId);
    if (!peer) return;

    const response = createSyncResponse(blocks, requestId, this.publicKey, this.privateKey);
    try {
      await this.sendFn(peer, response);
    } catch (err) {
      log.warn('Failed to send sync response', { peerId, error: err.message || String(err) });
      this.peerManager.recordFailure(peerId);
    }
  }

  /**
   * Add peer to network
   * @param {Object} peerInfo - Peer info
   */
  addPeer(peerInfo) {
    this.peerManager.addPeer(createPeerInfo(peerInfo));
  }

  /**
   * Remove peer from network
   * @param {string} peerId - Peer ID
   */
  removePeer(peerId) {
    this.peerManager.removePeer(peerId);
  }

  /**
   * Get network statistics
   * @returns {Object} Network stats
   */
  getStats() {
    const peerStats = this.peerManager.getStats();
    return {
      ...peerStats,
      fanout: GOSSIP_FANOUT,
      estimatedReachTime: PeerManager.calculatePropagationTime(peerStats.total),
      pendingRequests: this.pendingRequests.size,
    };
  }

  /**
   * Add a pending request with bounds enforcement
   * @private
   * @param {string} requestId - Request ID
   * @param {Object} resolver - { resolve, reject } functions
   */
  _addPendingRequest(requestId, resolver) {
    // Clean up stale requests first
    const now = Date.now();
    for (const [id, req] of this.pendingRequests) {
      if (now - req.createdAt > this.requestMaxAgeMs) {
        req.reject(new Error('Request expired (cleanup)'));
        this.pendingRequests.delete(id);
      }
    }

    // Evict oldest if at capacity
    if (this.pendingRequests.size >= this.maxPendingRequests) {
      const oldest = this.pendingRequests.keys().next().value;
      if (oldest) {
        const req = this.pendingRequests.get(oldest);
        if (req) req.reject(new Error('Request evicted (queue full)'));
        this.pendingRequests.delete(oldest);
      }
    }

    this.pendingRequests.set(requestId, {
      ...resolver,
      createdAt: now,
    });
  }

  // ===========================================
  // Consensus Message Broadcasting (Layer 4)
  // ===========================================

  /**
   * Broadcast a consensus block proposal
   * @param {Object} proposal - Block proposal data
   * @returns {Promise<number>} Number of peers sent to
   */
  async broadcastBlockProposal(proposal) {
    const message = createConsensusBlockProposal(proposal, this.publicKey, this.privateKey);
    return this.broadcast(message);
  }

  /**
   * Broadcast a consensus vote
   * @param {Object} vote - Vote data
   * @returns {Promise<number>} Number of peers sent to
   */
  async broadcastVote(vote) {
    const message = createConsensusVote(vote, this.publicKey, this.privateKey);
    return this.broadcast(message);
  }

  /**
   * Broadcast a vote aggregate
   * @param {Object} aggregate - Vote aggregate data
   * @returns {Promise<number>} Number of peers sent to
   */
  async broadcastVoteAggregate(aggregate) {
    const message = createConsensusVoteAggregate(aggregate, this.publicKey, this.privateKey);
    return this.broadcast(message);
  }

  /**
   * Broadcast a finality notification
   * @param {Object} finality - Finality data
   * @returns {Promise<number>} Number of peers sent to
   */
  async broadcastFinality(finality) {
    const message = createConsensusFinality(finality, this.publicKey, this.privateKey);
    return this.broadcast(message);
  }

  /**
   * Broadcast slot status to all peers or send to a specific peer
   * Used for sync handshake coordination
   * @param {Object} status - Slot status data
   * @param {string} [toPeerPublicKey] - Optional: send only to this peer
   * @returns {Promise<number>} Number of peers sent to
   */
  async broadcastSlotStatus(status, toPeerPublicKey = null) {
    const message = createConsensusSlotStatus(status, this.publicKey, this.privateKey);

    if (toPeerPublicKey) {
      // Send to specific peer
      let peer = this.peerManager.getPeer(toPeerPublicKey);
      if (!peer) {
        peer = this.peerManager.getPeerByPublicKey(toPeerPublicKey);
      }
      if (!peer) {
        log.warn('Cannot send slot status: peer not found', { peer: toPeerPublicKey });
        return 0;
      }
      try {
        await this.sendFn(peer, message);
        this.peerManager.updateActivity(peer.id, 'sent');
        return 1;
      } catch (err) {
        this.peerManager.recordFailure(peer.id);
        log.warn('Failed to send slot status', { peer: toPeerPublicKey, error: err.message });
        return 0;
      }
    }

    // Broadcast to all peers
    return this.broadcast(message);
  }

  /**
   * Request state sync from a peer (for late joiners)
   * @param {string} peerIdOrPublicKey - Peer ID or public key
   * @param {number} [sinceSlot=0] - Request state since this slot
   * @param {number} [timeoutMs=10000] - Request timeout
   * @returns {Promise<Object>} State response with finalized blocks
   */
  async requestStateSync(peerIdOrPublicKey, sinceSlot = 0, timeoutMs = 10000) {
    // Try both peer ID and public key lookup
    let peer = this.peerManager.getPeer(peerIdOrPublicKey);
    if (!peer) {
      peer = this.peerManager.getPeerByPublicKey(peerIdOrPublicKey);
    }
    if (!peer) {
      throw new Error(`Unknown peer: ${peerIdOrPublicKey}`);
    }

    const request = createConsensusStateRequest(
      { sinceSlot, maxBlocks: 50 },
      this.publicKey,
      this.privateKey
    );

    const requestId = request.payload.requestId;

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(requestId);
        reject(new Error('State sync request timeout'));
      }, timeoutMs);

      this._addPendingRequest(requestId, {
        resolve: (response) => {
          clearTimeout(timeout);
          resolve(response);
        },
        reject: (err) => {
          clearTimeout(timeout);
          reject(err);
        },
      });

      this.sendFn(peer, request).catch((err) => {
        clearTimeout(timeout);
        this.pendingRequests.delete(requestId);
        reject(err);
      });
    });
  }

  /**
   * Send state sync response to a peer
   * @param {string} peerIdOrPublicKey - Peer ID or public key
   * @param {string} requestId - Original request ID
   * @param {Object} state - State to send
   * @param {Object[]} state.blocks - Finalized blocks
   * @param {number} state.latestSlot - Latest finalized slot
   * @param {number} state.validatorCount - Number of validators
   */
  async sendStateResponse(peerIdOrPublicKey, requestId, state) {
    // Try both peer ID and public key lookup
    let peer = this.peerManager.getPeer(peerIdOrPublicKey);
    if (!peer) {
      peer = this.peerManager.getPeerByPublicKey(peerIdOrPublicKey);
    }
    if (!peer) return;

    const response = createConsensusStateResponse(
      {
        requestId,
        blocks: state.blocks || [],
        latestSlot: state.latestSlot || 0,
        validatorCount: state.validatorCount || 0,
      },
      this.publicKey,
      this.privateKey
    );

    try {
      await this.sendFn(peer, response);
    } catch (err) {
      log.warn('Failed to send state response', { peer: peerIdOrPublicKey, error: err.message || String(err) });
      this.peerManager.recordFailure(peer.id);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Judgment Sync Support (Gap #10 fix)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get active peers
   * @returns {Array} Active peers
   */
  getActivePeers() {
    return this.peerManager.getActivePeers();
  }

  /**
   * Send a message directly to a specific peer
   * @param {string} peerIdOrPublicKey - Peer ID or public key
   * @param {Object} message - Message to send
   * @returns {Promise<void>}
   */
  async sendTo(peerIdOrPublicKey, message) {
    let peer = this.peerManager.getPeer(peerIdOrPublicKey);
    if (!peer) {
      peer = this.peerManager.getPeerByPublicKey(peerIdOrPublicKey);
    }
    if (!peer) {
      throw new Error(`Peer not found: ${peerIdOrPublicKey}`);
    }

    try {
      await this.sendFn(peer, message);
      this.peerManager.updateActivity(peer.id, 'sent');
    } catch (err) {
      this.peerManager.recordFailure(peer.id);
      throw err;
    }
  }
}

export default { GossipProtocol };

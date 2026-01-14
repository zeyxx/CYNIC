/**
 * Gossip Propagation
 *
 * Push-pull hybrid propagation with Fibonacci fanout
 *
 * @module @cynic/protocol/gossip/propagation
 */

'use strict';

import { GOSSIP_FANOUT } from '@cynic/core';
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
   */
  constructor({ publicKey, privateKey, address, onMessage, sendFn }) {
    this.publicKey = publicKey;
    this.privateKey = privateKey;
    this.address = address;
    this.onMessage = onMessage || (() => {});
    this.sendFn = sendFn || (() => Promise.resolve());

    this.peerManager = new PeerManager();
    this.messageQueue = [];
    this.pendingRequests = new Map();
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
        console.warn(`Invalid message signature from ${fromPeerId}`);
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
      default:
        console.warn(`Unknown message type: ${message.type}`);
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

    for (const peer of peers) {
      try {
        await this.sendFn(peer, message);
        this.peerManager.updateActivity(peer.id, 'sent');
        sent++;
      } catch (err) {
        this.peerManager.recordFailure(peer.id);
      }
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

      this.pendingRequests.set(request.id, {
        resolve: (blocks) => {
          clearTimeout(timeout);
          resolve(blocks);
        },
        reject,
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
    await this.sendFn(peer, response);
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
    };
  }
}

export default { GossipProtocol };

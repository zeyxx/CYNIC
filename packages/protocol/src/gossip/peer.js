/**
 * Peer Management
 *
 * Track and manage gossip peers
 *
 * @module @cynic/protocol/gossip/peer
 */

'use strict';

import { GOSSIP_FANOUT, PHI_INV } from '@cynic/core';
import { phiSaltedHash } from '../crypto/hash.js';

/**
 * Peer status
 */
export const PeerStatus = {
  ACTIVE: 'ACTIVE',
  INACTIVE: 'INACTIVE',
  BANNED: 'BANNED',
};

/**
 * Create peer info
 * @param {Object} params - Peer parameters
 * @param {string} params.publicKey - Peer public key
 * @param {string} params.address - Peer network address
 * @param {number} [params.eScore=50] - Peer E-Score
 * @returns {Object} Peer info
 */
export function createPeerInfo({ publicKey, address, eScore = 50 }) {
  return {
    id: phiSaltedHash(publicKey),
    publicKey,
    address,
    eScore,
    status: PeerStatus.ACTIVE,
    lastSeen: Date.now(),
    connectedAt: Date.now(),
    messagesSent: 0,
    messagesReceived: 0,
    failedAttempts: 0,
  };
}

/**
 * Peer Manager - Track and select peers for gossip
 */
export class PeerManager {
  /**
   * @param {Object} [options] - Manager options
   * @param {number} [options.maxPeers=100] - Maximum peers to track
   * @param {number} [options.inactiveThresholdMs=300000] - Inactive threshold (5 min)
   * @param {number} [options.maxSeenMessages=100000] - Max messages to track for dedup
   * @param {number} [options.messageExpireMs=60000] - Message expiration time
   */
  constructor(options = {}) {
    this.maxPeers = options.maxPeers || 100;
    this.inactiveThresholdMs = options.inactiveThresholdMs || 300000;
    this.maxSeenMessages = options.maxSeenMessages || 100000;
    this.messageExpireMs = options.messageExpireMs || 60000; // 1 minute
    this.peers = new Map();
    this.bannedPeers = new Set();
    // Use Map with timestamps for bounded dedup with LRU eviction
    this.seenMessages = new Map(); // messageId -> timestamp
  }

  /**
   * Add or update peer
   * @param {Object} peerInfo - Peer info
   * @returns {boolean} True if added/updated
   */
  addPeer(peerInfo) {
    if (this.bannedPeers.has(peerInfo.id)) {
      return false;
    }

    const existing = this.peers.get(peerInfo.id);
    if (existing) {
      // Update existing peer
      existing.lastSeen = Date.now();
      existing.address = peerInfo.address || existing.address;
      existing.eScore = peerInfo.eScore || existing.eScore;
      existing.status = PeerStatus.ACTIVE;
      return true;
    }

    // Add new peer (evict if at capacity)
    if (this.peers.size >= this.maxPeers) {
      this._evictWorstPeer();
    }

    this.peers.set(peerInfo.id, peerInfo);
    return true;
  }

  /**
   * Remove peer
   * @param {string} peerId - Peer ID
   */
  removePeer(peerId) {
    this.peers.delete(peerId);
  }

  /**
   * Ban peer
   * @param {string} peerId - Peer ID
   * @param {string} [reason] - Ban reason
   */
  banPeer(peerId, reason = 'unspecified') {
    const peer = this.peers.get(peerId);
    if (peer) {
      peer.status = PeerStatus.BANNED;
      peer.banReason = reason;
    }
    this.bannedPeers.add(peerId);
    this.peers.delete(peerId);
  }

  /**
   * Update peer activity
   * @param {string} peerId - Peer ID
   * @param {string} direction - 'sent' or 'received'
   */
  updateActivity(peerId, direction) {
    const peer = this.peers.get(peerId);
    if (peer) {
      peer.lastSeen = Date.now();
      if (direction === 'sent') {
        peer.messagesSent++;
      } else {
        peer.messagesReceived++;
      }
    }
  }

  /**
   * Get peer by public key
   * @param {string} publicKey - Peer public key
   * @returns {Object|null} Peer info or null
   */
  getPeerByPublicKey(publicKey) {
    for (const peer of this.peers.values()) {
      if (peer.publicKey === publicKey) {
        return peer;
      }
    }
    return null;
  }

  /**
   * Record failed attempt
   * @param {string} peerId - Peer ID
   * @param {number} [maxFailures=5] - Max failures before marking inactive
   */
  recordFailure(peerId, maxFailures = 5) {
    const peer = this.peers.get(peerId);
    if (peer) {
      peer.failedAttempts++;
      if (peer.failedAttempts >= maxFailures) {
        peer.status = PeerStatus.INACTIVE;
      }
    }
  }

  /**
   * Get peer by ID
   * @param {string} peerId - Peer ID
   * @returns {Object|null} Peer info or null
   */
  getPeer(peerId) {
    return this.peers.get(peerId) || null;
  }

  /**
   * Get all active peers
   * @returns {Object[]} Active peers
   */
  getActivePeers() {
    const now = Date.now();
    return Array.from(this.peers.values()).filter((p) => {
      if (p.status !== PeerStatus.ACTIVE) return false;
      if (now - p.lastSeen > this.inactiveThresholdMs) {
        p.status = PeerStatus.INACTIVE;
        return false;
      }
      return true;
    });
  }

  /**
   * Select peers for gossip (fanout selection)
   * @param {number} [count] - Number of peers (default: GOSSIP_FANOUT)
   * @param {Set<string>} [exclude] - Peer IDs to exclude
   * @returns {Object[]} Selected peers
   */
  selectGossipPeers(count = GOSSIP_FANOUT, exclude = new Set()) {
    const active = this.getActivePeers().filter((p) => !exclude.has(p.id));

    if (active.length <= count) {
      return active;
    }

    // Weight by E-Score for selection
    const weighted = active.map((p) => ({
      peer: p,
      weight: p.eScore || 50,
    }));

    const totalWeight = weighted.reduce((sum, w) => sum + w.weight, 0);
    const selected = [];
    const usedIds = new Set();

    while (selected.length < count && selected.length < active.length) {
      let random = Math.random() * totalWeight;
      for (const { peer, weight } of weighted) {
        if (usedIds.has(peer.id)) continue;
        random -= weight;
        if (random <= 0) {
          selected.push(peer);
          usedIds.add(peer.id);
          break;
        }
      }
    }

    return selected;
  }

  /**
   * Check if message has been seen
   * @param {string} messageId - Message ID
   * @returns {boolean} True if seen
   */
  hasSeenMessage(messageId) {
    const timestamp = this.seenMessages.get(messageId);
    if (!timestamp) return false;

    // Check if expired (wall-clock based, not setTimeout)
    if (Date.now() - timestamp > this.messageExpireMs) {
      this.seenMessages.delete(messageId);
      return false;
    }
    return true;
  }

  /**
   * Mark message as seen
   * @param {string} messageId - Message ID
   */
  markMessageSeen(messageId) {
    // Evict oldest entries if at capacity (FIFO eviction)
    if (this.seenMessages.size >= this.maxSeenMessages) {
      // Delete oldest entries (first 10% to amortize eviction cost)
      const evictCount = Math.max(1, Math.floor(this.maxSeenMessages * 0.1));
      const iterator = this.seenMessages.keys();
      for (let i = 0; i < evictCount; i++) {
        const key = iterator.next().value;
        if (key) this.seenMessages.delete(key);
      }
    }

    this.seenMessages.set(messageId, Date.now());
  }

  /**
   * Evict worst peer (lowest E-Score and activity)
   * @private
   */
  _evictWorstPeer() {
    let worst = null;
    let worstScore = Infinity;

    for (const peer of this.peers.values()) {
      // Score based on E-Score and activity
      const activityScore = peer.messagesReceived + peer.messagesSent;
      const score = peer.eScore + activityScore * 0.1;

      if (score < worstScore) {
        worst = peer;
        worstScore = score;
      }
    }

    if (worst) {
      this.peers.delete(worst.id);
    }
  }

  /**
   * Get peer statistics
   * @returns {Object} Statistics
   */
  getStats() {
    const peers = Array.from(this.peers.values());
    return {
      total: peers.length,
      active: peers.filter((p) => p.status === PeerStatus.ACTIVE).length,
      inactive: peers.filter((p) => p.status === PeerStatus.INACTIVE).length,
      banned: this.bannedPeers.size,
      avgEScore:
        peers.length > 0
          ? peers.reduce((sum, p) => sum + p.eScore, 0) / peers.length
          : 0,
    };
  }

  /**
   * Calculate hops needed to reach N nodes
   * O(log₁₃ n) calculation
   * @param {number} n - Target nodes
   * @returns {number} Hops needed
   */
  static calculateHops(n) {
    if (n <= 1) return 0;
    return Math.ceil(Math.log(n) / Math.log(GOSSIP_FANOUT));
  }

  /**
   * Calculate propagation time
   * @param {number} n - Target nodes
   * @param {number} [hopLatencyMs=50] - Latency per hop
   * @returns {number} Total propagation time ms
   */
  static calculatePropagationTime(n, hopLatencyMs = 50) {
    const hops = PeerManager.calculateHops(n);
    return hops * hopLatencyMs;
  }
}

export default {
  PeerStatus,
  createPeerInfo,
  PeerManager,
};

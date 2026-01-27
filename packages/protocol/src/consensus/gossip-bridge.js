/**
 * Consensus-Gossip Bridge
 *
 * Integrates the ConsensusEngine with the GossipProtocol
 * for distributed block proposal, voting, and finality.
 *
 * Flow:
 * 1. ConsensusEngine emits events (block:proposed, vote:cast, block:finalized)
 * 2. Bridge broadcasts via GossipProtocol
 * 3. GossipProtocol receives messages
 * 4. Bridge routes to ConsensusEngine
 *
 * @module @cynic/protocol/consensus/gossip-bridge
 */

'use strict';

import { EventEmitter } from 'events';
import { MessageType } from '../gossip/message.js';

/**
 * ConsensusGossip Bridge
 *
 * Connects ConsensusEngine with GossipProtocol for distributed consensus
 */
export class ConsensusGossip extends EventEmitter {
  /**
   * @param {Object} options - Bridge options
   * @param {ConsensusEngine} options.consensus - ConsensusEngine instance
   * @param {GossipProtocol} options.gossip - GossipProtocol instance
   * @param {boolean} [options.autoSync=true] - Auto-sync when joining network
   * @param {number} [options.syncDelayMs=500] - Delay before auto-sync (allows peers to connect)
   * @param {number} [options.handshakeTimeoutMs=5000] - Timeout for sync handshake
   */
  constructor({ consensus, gossip, autoSync = true, syncDelayMs = 500, handshakeTimeoutMs = 5000 }) {
    super();

    this.consensus = consensus;
    this.gossip = gossip;
    this.autoSync = autoSync;
    this.syncDelayMs = syncDelayMs;
    this.handshakeTimeoutMs = handshakeTimeoutMs;

    this.started = false;
    this.synced = false;
    this.syncTimer = null;

    // Track handshake state per peer to coordinate who syncs
    this.peerHandshakes = new Map(); // peerPublicKey -> { ourSlot, theirSlot, resolved }

    this.stats = {
      proposalsBroadcast: 0,
      votesBroadcast: 0,
      finalityBroadcast: 0,
      proposalsReceived: 0,
      votesReceived: 0,
      finalityReceived: 0,
      handshakesSent: 0,
      handshakesReceived: 0,
      syncDecisions: { weSync: 0, theySync: 0, noSync: 0 },
    };

    // Bind methods
    this._onConsensusEvent = this._onConsensusEvent.bind(this);
    this._onGossipMessage = this._onGossipMessage.bind(this);
    this._onPeerAdded = this._onPeerAdded.bind(this);
  }

  /**
   * Start the bridge
   *
   * Connects consensus events to gossip broadcasts and
   * routes incoming gossip messages to consensus.
   */
  start() {
    if (this.started) return;

    // Listen to consensus events
    this.consensus.on('block:proposed', this._onConsensusEvent);
    this.consensus.on('vote:cast', this._onConsensusEvent);
    this.consensus.on('vote:received', this._onConsensusEvent);
    this.consensus.on('block:finalized', this._onConsensusEvent);
    this.consensus.on('block:confirmed', this._onConsensusEvent);

    // Store original onMessage handler
    this._originalOnMessage = this.gossip.onMessage;

    // Wrap gossip onMessage to intercept consensus messages
    this.gossip.onMessage = async (message) => {
      // Handle consensus messages
      if (this._isConsensusMessage(message.type)) {
        await this._onGossipMessage(message);
      }

      // Call original handler for all messages
      if (this._originalOnMessage) {
        await this._originalOnMessage(message);
      }
    };

    // Hook into peer manager for auto-sync on join
    if (this.autoSync) {
      this._originalAddPeer = this.gossip.peerManager.addPeer.bind(this.gossip.peerManager);
      this.gossip.peerManager.addPeer = (peerInfo) => {
        const result = this._originalAddPeer(peerInfo);
        if (result) {
          this._onPeerAdded(peerInfo);
        }
        return result;
      };
    }

    this.started = true;
    this.emit('started');
  }

  /**
   * Stop the bridge
   */
  stop() {
    if (!this.started) return;

    // Clear sync timer
    if (this.syncTimer) {
      clearTimeout(this.syncTimer);
      this.syncTimer = null;
    }

    // Remove consensus event listeners
    this.consensus.off('block:proposed', this._onConsensusEvent);
    this.consensus.off('vote:cast', this._onConsensusEvent);
    this.consensus.off('vote:received', this._onConsensusEvent);
    this.consensus.off('block:finalized', this._onConsensusEvent);
    this.consensus.off('block:confirmed', this._onConsensusEvent);

    // Restore original onMessage handler
    if (this._originalOnMessage) {
      this.gossip.onMessage = this._originalOnMessage;
    }

    // Restore original addPeer
    if (this._originalAddPeer) {
      this.gossip.peerManager.addPeer = this._originalAddPeer;
    }

    this.started = false;
    this.emit('stopped');
  }

  /**
   * Handle consensus engine events
   * @private
   */
  async _onConsensusEvent(event) {
    try {
      switch (event.event || event.type) {
        case 'block:proposed':
          await this._broadcastBlockProposal(event);
          break;
        case 'vote:cast':
          await this._broadcastVote(event);
          break;
        case 'block:finalized':
          await this._broadcastFinality(event);
          break;
        case 'block:confirmed':
          // Optionally broadcast confirmed status
          // await this._broadcastConfirmation(event);
          break;
      }
    } catch (err) {
      this.emit('error', { source: 'consensus-event', error: err.message });
    }
  }

  /**
   * Handle incoming gossip messages
   * @private
   */
  async _onGossipMessage(message) {
    try {
      switch (message.type) {
        case MessageType.CONSENSUS_BLOCK_PROPOSAL:
          await this._handleBlockProposal(message);
          break;
        case MessageType.CONSENSUS_VOTE:
          await this._handleVote(message);
          break;
        case MessageType.CONSENSUS_VOTE_AGGREGATE:
          await this._handleVoteAggregate(message);
          break;
        case MessageType.CONSENSUS_FINALITY:
          await this._handleFinality(message);
          break;
        case MessageType.CONSENSUS_SLOT_STATUS:
          await this._handleSlotStatus(message);
          break;
        case MessageType.CONSENSUS_STATE_REQUEST:
          await this._handleStateRequest(message);
          break;
        case MessageType.CONSENSUS_STATE_RESPONSE:
          await this._handleStateResponse(message);
          break;
      }
    } catch (err) {
      this.emit('error', { source: 'gossip-message', error: err.message });
    }
  }

  /**
   * Broadcast block proposal
   * @private
   */
  async _broadcastBlockProposal(event) {
    const proposal = {
      blockHash: event.blockHash,
      block: event.block,
      slot: event.slot,
      proposer: this.consensus.publicKey,
    };

    const sent = await this.gossip.broadcastBlockProposal(proposal);
    this.stats.proposalsBroadcast++;

    this.emit('proposal:broadcast', { blockHash: event.blockHash, peers: sent });
    return sent;
  }

  /**
   * Broadcast vote
   * @private
   */
  async _broadcastVote(event) {
    // Include the full vote object for signature verification
    const votePayload = {
      blockHash: event.blockHash,
      slot: event.slot,
      decision: event.decision,
      voter: this.consensus.publicKey,
      weight: event.weight,
      // Include full vote for verification (has proposal_id, vote, timestamp, signature)
      vote: event.vote,
    };

    const sent = await this.gossip.broadcastVote(votePayload);
    this.stats.votesBroadcast++;

    this.emit('vote:broadcast', { blockHash: event.blockHash, peers: sent });
    return sent;
  }

  /**
   * Broadcast finality notification
   * @private
   */
  async _broadcastFinality(event) {
    const finality = {
      blockHash: event.blockHash,
      slot: event.slot,
      status: event.status,
      probability: event.probability,
      confirmations: event.confirmations,
    };

    const sent = await this.gossip.broadcastFinality(finality);
    this.stats.finalityBroadcast++;

    this.emit('finality:broadcast', { blockHash: event.blockHash, peers: sent });
    return sent;
  }

  /**
   * Handle received block proposal
   * @private
   */
  async _handleBlockProposal(message) {
    const { payload, sender } = message;

    // Don't process our own proposals
    if (sender === this.consensus.publicKey) return;

    this.stats.proposalsReceived++;

    // Forward to consensus engine
    const block = {
      hash: payload.blockHash,
      ...payload.block,
      slot: payload.slot,
      proposer: payload.proposer,
    };

    try {
      this.consensus.receiveBlock(block, sender);
      this.emit('proposal:received', { blockHash: payload.blockHash, from: sender });
    } catch (err) {
      this.emit('error', { source: 'proposal-receive', error: err.message });
    }
  }

  /**
   * Handle received vote
   * @private
   */
  async _handleVote(message) {
    const { payload, sender } = message;

    // Don't process our own votes
    if (sender === this.consensus.publicKey) return;

    this.stats.votesReceived++;

    // Use full vote object if available (for signature verification)
    // Fall back to constructing from payload fields
    const vote = payload.vote || {
      proposal_id: payload.blockHash,
      block_hash: payload.blockHash,
      vote: payload.decision,
      voter: payload.voter,
      weight: payload.weight,
      signature: payload.signature,
      timestamp: payload.timestamp || Date.now(),
    };

    // Ensure block_hash is set for engine lookup
    vote.block_hash = vote.block_hash || payload.blockHash;

    try {
      this.consensus.receiveVote(vote, sender);
      this.emit('vote:received', { blockHash: payload.blockHash, from: sender });
    } catch (err) {
      this.emit('error', { source: 'vote-receive', error: err.message });
    }
  }

  /**
   * Handle received vote aggregate
   * @private
   */
  async _handleVoteAggregate(message) {
    const { payload, sender } = message;

    // Skip if from self
    if (sender === this.consensus.publicKey) return;

    // Process aggregate votes
    if (payload.votes && Array.isArray(payload.votes)) {
      for (const vote of payload.votes) {
        try {
          this.consensus.receiveVote(vote, sender);
        } catch (err) {
          // Continue with other votes
        }
      }
    }

    this.emit('aggregate:received', { blockHash: payload.blockHash, from: sender });
  }

  /**
   * Handle received finality notification
   * @private
   */
  async _handleFinality(message) {
    const { payload, sender } = message;

    this.stats.finalityReceived++;

    // Emit for external handlers (e.g., chain sync)
    this.emit('finality:received', {
      blockHash: payload.blockHash,
      status: payload.status,
      from: sender,
    });
  }

  /**
   * Handle received slot status - implements sync handshake
   *
   * When a peer sends their slot status, we compare slots to decide who syncs:
   * - If our slot < their slot: we sync from them
   * - If our slot > their slot: they sync from us (we wait)
   * - If equal: use public key comparison to break tie deterministically
   *
   * @private
   */
  async _handleSlotStatus(message) {
    const { payload, sender } = message;

    // Emit for external listeners
    this.emit('slot-status:received', {
      slot: payload.slot,
      leader: payload.leader,
      from: sender,
    });

    // Skip if auto-sync disabled
    if (!this.autoSync) return;

    // Get or create handshake state for this peer
    let handshake = this.peerHandshakes.get(sender);
    if (!handshake) {
      handshake = { ourSlot: null, theirSlot: null, resolved: false };
      this.peerHandshakes.set(sender, handshake);
    }

    // Record their slot
    handshake.theirSlot = payload.slot || 0;
    this.stats.handshakesReceived++;

    // If we haven't sent our status yet, send it now
    if (handshake.ourSlot === null) {
      await this._sendSlotStatus(sender);
    }

    // If handshake already resolved, skip
    if (handshake.resolved) return;

    // Both sides have exchanged - make sync decision
    if (handshake.ourSlot !== null && handshake.theirSlot !== null) {
      await this._resolveSyncHandshake(sender, handshake);
    }
  }

  /**
   * Send our slot status to a specific peer
   * @private
   */
  async _sendSlotStatus(peerPublicKey) {
    const ourSlot = this.consensus.getCurrentSlot?.() || 0;
    const ourPublicKey = this.consensus.publicKey;

    // Update handshake state
    let handshake = this.peerHandshakes.get(peerPublicKey);
    if (!handshake) {
      handshake = { ourSlot: null, theirSlot: null, resolved: false };
      this.peerHandshakes.set(peerPublicKey, handshake);
    }
    handshake.ourSlot = ourSlot;

    // Send slot status to peer
    try {
      const status = {
        slot: ourSlot,
        publicKey: ourPublicKey,
        isHandshake: true, // Flag to indicate this is for sync coordination
      };

      // Use gossip to send directly to specific peer
      if (this.gossip.broadcastSlotStatus) {
        await this.gossip.broadcastSlotStatus(status, peerPublicKey);
      } else {
        await this._broadcastSlotStatusToPeer(peerPublicKey, status);
      }

      this.stats.handshakesSent++;
      this.emit('handshake:sent', { to: peerPublicKey, slot: ourSlot });
    } catch (err) {
      this.emit('error', { source: 'handshake-send', error: err.message });
    }
  }

  /**
   * Broadcast slot status to a specific peer
   * @private
   */
  async _broadcastSlotStatusToPeer(peerPublicKey, status) {
    // Find peer and send directly
    const peer = this.gossip.peerManager.getPeerByPublicKey?.(peerPublicKey);
    if (peer && this.gossip.sendFn) {
      const message = {
        id: `slot_${Date.now()}_${Math.random().toString(36).slice(2)}`,
        type: MessageType.CONSENSUS_SLOT_STATUS,
        payload: status,
        sender: this.consensus.publicKey,
        timestamp: Date.now(),
        ttl: 1,
        hops: 0,
      };
      await this.gossip.sendFn(peer, message);
    }
  }

  /**
   * Resolve sync handshake - decide who syncs from whom
   * @private
   */
  async _resolveSyncHandshake(peerPublicKey, handshake) {
    handshake.resolved = true;

    const ourSlot = handshake.ourSlot;
    const theirSlot = handshake.theirSlot;
    const ourPublicKey = this.consensus.publicKey;

    this.emit('handshake:resolving', {
      peer: peerPublicKey,
      ourSlot,
      theirSlot,
    });

    // Decision logic:
    // 1. Lower slot syncs from higher slot
    // 2. If equal, use public key comparison (lower key syncs) for determinism
    let shouldWeSync = false;

    if (ourSlot < theirSlot) {
      shouldWeSync = true;
      this.stats.syncDecisions.weSync++;
    } else if (ourSlot > theirSlot) {
      shouldWeSync = false;
      this.stats.syncDecisions.theySync++;
    } else {
      // Equal slots - use public key to break tie deterministically
      // Lower public key syncs (arbitrary but consistent)
      shouldWeSync = ourPublicKey < peerPublicKey;
      if (shouldWeSync) {
        this.stats.syncDecisions.weSync++;
      } else {
        this.stats.syncDecisions.theySync++;
      }
    }

    this.emit('handshake:resolved', {
      peer: peerPublicKey,
      ourSlot,
      theirSlot,
      decision: shouldWeSync ? 'we-sync' : 'they-sync',
    });

    // If we should sync, request state from this peer
    if (shouldWeSync && !this.synced) {
      try {
        this.emit('sync:starting', { peer: peerPublicKey, reason: 'handshake' });

        const result = await this.requestSync(peerPublicKey, 0);

        if (result && (result.imported > 0 || result.latestSlot >= 0)) {
          this.synced = true;
          this.emit('sync:completed', {
            from: peerPublicKey,
            imported: result.imported || 0,
            latestSlot: result.latestSlot || 0,
          });
        }
      } catch (err) {
        // Sync failed - not critical, we can still participate
        this.emit('sync:failed', {
          peer: peerPublicKey,
          error: err.message,
        });
      }
    }
  }

  /**
   * Handle peer added - initiates sync handshake
   *
   * Instead of directly syncing, we now send our slot status to coordinate.
   * The peer will respond with their slot status, and we'll decide who syncs.
   *
   * @private
   */
  _onPeerAdded(peerInfo) {
    // Skip if auto-sync disabled
    if (!this.autoSync) return;

    const peerPublicKey = peerInfo.publicKey || peerInfo.id;
    if (!peerPublicKey) return;

    // Debounce to allow connection to stabilize
    setTimeout(async () => {
      // Skip if we already have a resolved handshake with this peer
      const existing = this.peerHandshakes.get(peerPublicKey);
      if (existing?.resolved) return;

      // Initiate handshake by sending our slot status
      await this._sendSlotStatus(peerPublicKey);

      // Set timeout for handshake completion
      setTimeout(() => {
        const handshake = this.peerHandshakes.get(peerPublicKey);
        if (handshake && !handshake.resolved) {
          // Handshake timed out - peer didn't respond
          // This is OK, they might be an older version or busy
          handshake.resolved = true;
          this.emit('handshake:timeout', { peer: peerPublicKey });
        }
      }, this.handshakeTimeoutMs);
    }, this.syncDelayMs);
  }

  /**
   * Handle state sync request from a peer
   * @private
   */
  async _handleStateRequest(message) {
    const { payload, sender } = message;

    this.stats.syncRequestsReceived = (this.stats.syncRequestsReceived || 0) + 1;

    // Export our finalized state
    const state = this.consensus.exportState(payload.sinceSlot || 0, payload.maxBlocks || 50);

    // Send response back to requesting peer
    try {
      await this.gossip.sendStateResponse(sender, payload.requestId, state);
      this.stats.syncResponsesSent = (this.stats.syncResponsesSent || 0) + 1;

      this.emit('sync:response-sent', {
        to: sender,
        blocksCount: state.blocks.length,
        latestSlot: state.latestSlot,
      });
    } catch (err) {
      this.emit('error', { source: 'sync-response', error: err.message });
    }
  }

  /**
   * Handle state sync response from a peer
   * @private
   */
  async _handleStateResponse(message) {
    const { payload, sender } = message;

    this.stats.syncResponsesReceived = (this.stats.syncResponsesReceived || 0) + 1;

    // Import the synced state
    const result = this.consensus.importState({
      blocks: payload.blocks,
      latestSlot: payload.latestSlot,
      validatorCount: payload.validatorCount,
    });

    this.emit('sync:state-imported', {
      from: sender,
      imported: result.imported,
      total: result.total,
      latestSlot: result.latestSlot,
      errors: result.errors,
    });

    // Resolve pending request if there is one
    const pending = this.gossip.pendingRequests?.get(payload.requestId);
    if (pending) {
      this.gossip.pendingRequests.delete(payload.requestId);
      pending.resolve(result);
    }
  }

  /**
   * Check if message type is consensus-related
   * @private
   */
  _isConsensusMessage(type) {
    return type && type.startsWith('CONSENSUS_');
  }

  /**
   * Get bridge statistics
   * @returns {Object} Statistics
   */
  getStats() {
    return {
      ...this.stats,
      started: this.started,
      consensusStats: this.consensus.getStats(),
      gossipStats: this.gossip.getStats(),
    };
  }

  /**
   * Manually broadcast a block proposal
   * (Used when proposing blocks outside of events)
   *
   * @param {Object} block - Block to propose
   * @returns {Promise<number>} Number of peers sent to
   */
  async proposeBlock(block) {
    return this._broadcastBlockProposal({
      blockHash: block.hash,
      block,
      slot: block.slot,
    });
  }

  /**
   * Manually broadcast a vote
   *
   * @param {Object} vote - Vote to broadcast
   * @returns {Promise<number>} Number of peers sent to
   */
  async broadcastVote(vote) {
    return this._broadcastVote(vote);
  }

  /**
   * Request state sync from a specific peer
   * Used by late-joining nodes to catch up on finalized history
   *
   * @param {string} peerId - Peer to request from
   * @param {number} [sinceSlot=0] - Request state since this slot
   * @returns {Promise<Object>} Sync result with imported blocks
   */
  async requestSync(peerId, sinceSlot = 0) {
    this.stats.syncRequestsSent = (this.stats.syncRequestsSent || 0) + 1;

    try {
      const result = await this.gossip.requestStateSync(peerId, sinceSlot);
      return result;
    } catch (err) {
      this.emit('error', { source: 'sync-request', error: err.message });
      throw err;
    }
  }

  /**
   * Sync state from all connected peers
   * Requests state from each peer and imports the most complete response
   *
   * @param {number} [sinceSlot=0] - Request state since this slot
   * @returns {Promise<Object>} Best sync result
   */
  async syncFromPeers(sinceSlot = 0) {
    const peers = this.gossip.peerManager.getActivePeers();
    if (peers.length === 0) {
      return { imported: 0, success: false, error: 'No peers available' };
    }

    let bestResult = { imported: 0, latestSlot: 0 };
    let anySuccess = false;

    // Request sync from each peer
    const results = await Promise.allSettled(
      peers.map(peer => this.requestSync(peer.publicKey, sinceSlot))
    );

    // Find the best result (most blocks imported or highest slot)
    for (const result of results) {
      if (result.status === 'fulfilled' && result.value) {
        anySuccess = true;
        if (result.value.latestSlot > bestResult.latestSlot) {
          bestResult = result.value;
        }
      }
    }

    this.emit('sync:completed', {
      peersQueried: peers.length,
      imported: bestResult.imported,
      latestSlot: bestResult.latestSlot,
      success: anySuccess,
    });

    return { ...bestResult, success: anySuccess };
  }
}

export default { ConsensusGossip };

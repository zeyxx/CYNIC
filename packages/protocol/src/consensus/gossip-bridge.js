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
import { ConsensusStateTree } from './merkle-state.js';

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

    // Merkle state tree for O(log n) diff sync
    this.stateTree = new ConsensusStateTree();
    this.peerRoots = new Map(); // peerPublicKey -> { root, slot, blockCount }

    // Track handshake state per peer to coordinate who syncs
    this.peerHandshakes = new Map(); // peerPublicKey -> { ourSlot, theirSlot, ourRoot, theirRoot, resolved }

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
      // Merkle diff stats
      merkleRootsSent: 0,
      merkleRootsReceived: 0,
      merkleDiffRequests: 0,
      merkleDiffResponses: 0,
      merkleDiffBlocksSynced: 0,
      fullSyncFallbacks: 0, // When Merkle diff not available
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
        // Merkle diff sync handlers
        case MessageType.CONSENSUS_MERKLE_ROOT:
          await this._handleMerkleRoot(message);
          break;
        case MessageType.CONSENSUS_MERKLE_DIFF_REQUEST:
          await this._handleMerkleDiffRequest(message);
          break;
        case MessageType.CONSENSUS_MERKLE_DIFF_RESPONSE:
          await this._handleMerkleDiffResponse(message);
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
      handshake = { ourSlot: null, theirSlot: null, ourRoot: null, theirRoot: null, resolved: false };
      this.peerHandshakes.set(sender, handshake);
    }

    // Record their slot and Merkle root
    handshake.theirSlot = payload.slot || 0;
    handshake.theirRoot = payload.merkleRoot || null;
    this.stats.handshakesReceived++;

    // Store peer root for future reference
    if (payload.merkleRoot) {
      this.peerRoots.set(sender, {
        root: payload.merkleRoot,
        slot: payload.slot || 0,
        blockCount: payload.blockCount || 0,
      });
    }

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
   * Includes Merkle root for efficient diff sync
   * @private
   */
  async _sendSlotStatus(peerPublicKey) {
    const ourSlot = this.consensus.getCurrentSlot?.() || 0;
    const ourPublicKey = this.consensus.publicKey;

    // Update state tree from consensus
    this._updateStateTree();
    const ourRoot = this.stateTree.getRoot();

    // Update handshake state
    let handshake = this.peerHandshakes.get(peerPublicKey);
    if (!handshake) {
      handshake = { ourSlot: null, theirSlot: null, ourRoot: null, theirRoot: null, resolved: false };
      this.peerHandshakes.set(peerPublicKey, handshake);
    }
    handshake.ourSlot = ourSlot;
    handshake.ourRoot = ourRoot;

    // Send slot status to peer with Merkle root
    try {
      const status = {
        slot: ourSlot,
        publicKey: ourPublicKey,
        isHandshake: true, // Flag to indicate this is for sync coordination
        // Merkle diff support
        merkleRoot: ourRoot,
        blockCount: this.stateTree.blocks.size,
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
   * Uses Merkle root comparison for O(log n) efficiency when possible
   * @private
   */
  async _resolveSyncHandshake(peerPublicKey, handshake) {
    handshake.resolved = true;

    const ourSlot = handshake.ourSlot;
    const theirSlot = handshake.theirSlot;
    const ourRoot = handshake.ourRoot;
    const theirRoot = handshake.theirRoot;
    const ourPublicKey = this.consensus.publicKey;

    this.emit('handshake:resolving', {
      peer: peerPublicKey,
      ourSlot,
      theirSlot,
      ourRoot,
      theirRoot,
    });

    // Quick check: if roots match, no sync needed (O(1))
    if (ourRoot && theirRoot && ourRoot === theirRoot) {
      this.stats.syncDecisions.noSync++;
      this.synced = true;
      this.emit('handshake:resolved', {
        peer: peerPublicKey,
        decision: 'no-sync',
        reason: 'merkle-roots-match',
      });
      return;
    }

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
      // Equal slots but different roots - use public key to break tie
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
      useMerkleDiff: Boolean(theirRoot),
    });

    // If we should sync, use Merkle diff if possible, else fall back to full sync
    if (shouldWeSync && !this.synced) {
      try {
        this.emit('sync:starting', { peer: peerPublicKey, reason: 'handshake' });

        let result;

        // Try Merkle diff first (O(log n)), fall back to full sync (O(n))
        if (theirRoot && ourRoot) {
          this.emit('sync:using-merkle-diff', { peer: peerPublicKey });
          result = await this.requestMerkleDiff(peerPublicKey, ourRoot, theirRoot, ourSlot);
        }

        // Fall back to full sync if Merkle diff not available or failed
        if (!result || result.error) {
          this.stats.fullSyncFallbacks++;
          this.emit('sync:fallback-full', { peer: peerPublicKey, reason: result?.error || 'no-merkle-support' });
          result = await this.requestSync(peerPublicKey, 0);
        }

        if (result && (result.imported > 0 || result.latestSlot >= 0)) {
          this.synced = true;
          this.emit('sync:completed', {
            from: peerPublicKey,
            imported: result.imported || 0,
            latestSlot: result.latestSlot || 0,
            usedMerkleDiff: result.usedMerkleDiff || false,
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

  // ═══════════════════════════════════════════════════════════════════════════
  // Merkle Diff Sync Methods (O(log n) efficiency)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Update state tree from consensus engine's finalized blocks
   * @private
   */
  _updateStateTree() {
    try {
      const state = this.consensus.exportState?.(0, 1000) || { blocks: [] };
      for (const block of state.blocks || []) {
        const slot = block.slot ?? block.height;
        if (!this.stateTree.blocks.has(slot)) {
          this.stateTree.addBlock(block);
        }
      }
    } catch (err) {
      // Consensus may not support exportState yet
      this.emit('error', { source: 'state-tree-update', error: err.message });
    }
  }

  /**
   * Handle received Merkle root announcement
   * @private
   */
  async _handleMerkleRoot(message) {
    const { payload, sender } = message;

    this.stats.merkleRootsReceived++;

    // Store peer's root
    this.peerRoots.set(sender, {
      root: payload.root,
      slot: payload.slot,
      blockCount: payload.blockCount,
      receivedAt: Date.now(),
    });

    this.emit('merkle-root:received', {
      from: sender,
      root: payload.root,
      slot: payload.slot,
    });

    // Update our state tree
    this._updateStateTree();

    // Check if we need to sync (roots differ)
    const ourRoot = this.stateTree.getRoot();
    if (ourRoot !== payload.root && payload.slot > this.stateTree.getLatestSlot()) {
      // They have more data - we may want to request diff
      this.emit('merkle-root:diff-detected', {
        peer: sender,
        ourRoot,
        theirRoot: payload.root,
        theirSlot: payload.slot,
      });
    }
  }

  /**
   * Handle Merkle diff request from peer
   * @private
   */
  async _handleMerkleDiffRequest(message) {
    const { payload, sender } = message;

    this.stats.merkleDiffRequests++;

    // Update our state tree
    this._updateStateTree();

    // Get diff based on their slot
    const diff = this.stateTree.getDiff(payload.sinceSlot || 0);

    // Send diff response
    try {
      await this.gossip.sendMerkleDiffResponse?.(sender, payload.requestId, diff) ||
        await this._sendMerkleDiffResponse(sender, payload.requestId, diff);

      this.stats.merkleDiffResponses++;

      this.emit('merkle-diff:response-sent', {
        to: sender,
        blocksCount: diff.blocks.length,
        newRoot: diff.newRoot,
      });
    } catch (err) {
      this.emit('error', { source: 'merkle-diff-response', error: err.message });
    }
  }

  /**
   * Send Merkle diff response to peer
   * @private
   */
  async _sendMerkleDiffResponse(peerPublicKey, requestId, diff) {
    const peer = this.gossip.peerManager.getPeerByPublicKey?.(peerPublicKey);
    if (peer && this.gossip.sendFn) {
      const message = {
        id: `mdiff_${Date.now()}_${Math.random().toString(36).slice(2)}`,
        type: MessageType.CONSENSUS_MERKLE_DIFF_RESPONSE,
        payload: {
          requestId,
          blocks: diff.blocks,
          newRoot: diff.newRoot,
          latestSlot: diff.latestSlot,
          diffSize: diff.blocks.length,
        },
        sender: this.consensus.publicKey,
        timestamp: Date.now(),
        ttl: 1,
        hops: 0,
      };
      await this.gossip.sendFn(peer, message);
    }
  }

  /**
   * Handle Merkle diff response from peer
   * @private
   */
  async _handleMerkleDiffResponse(message) {
    const { payload, sender } = message;

    this.stats.merkleDiffBlocksSynced += payload.blocks?.length || 0;

    // Import the diff
    const result = this.stateTree.importDiff(payload.blocks || []);

    // Also import to consensus engine
    if (result.imported > 0 && this.consensus.importState) {
      this.consensus.importState({
        blocks: payload.blocks,
        latestSlot: payload.latestSlot,
      });
    }

    this.emit('merkle-diff:imported', {
      from: sender,
      imported: result.imported,
      skipped: result.skipped,
      newRoot: result.newRoot,
      latestSlot: result.latestSlot,
    });

    // Resolve pending request if there is one
    const pending = this.gossip.pendingRequests?.get(payload.requestId);
    if (pending) {
      this.gossip.pendingRequests.delete(payload.requestId);
      pending.resolve({
        ...result,
        usedMerkleDiff: true,
      });
    }
  }

  /**
   * Request Merkle diff from a peer
   * Only syncs blocks that differ, achieving O(log n) efficiency
   *
   * @param {string} peerId - Peer to request from
   * @param {string} ourRoot - Our Merkle root
   * @param {string} theirRoot - Their Merkle root
   * @param {number} sinceSlot - Our latest slot
   * @returns {Promise<Object>} Diff result
   */
  async requestMerkleDiff(peerId, ourRoot, theirRoot, sinceSlot) {
    this.stats.merkleRootsSent++;

    const requestId = `mdiff_${Date.now()}_${Math.random().toString(36).slice(2)}`;

    // Create promise for response
    const responsePromise = new Promise((resolve, reject) => {
      // Store pending request
      if (!this.gossip.pendingRequests) {
        this.gossip.pendingRequests = new Map();
      }
      this.gossip.pendingRequests.set(requestId, { resolve, reject });

      // Timeout after 30 seconds
      setTimeout(() => {
        if (this.gossip.pendingRequests?.has(requestId)) {
          this.gossip.pendingRequests.delete(requestId);
          reject(new Error('Merkle diff request timeout'));
        }
      }, 30000);
    });

    // Send diff request
    try {
      const peer = this.gossip.peerManager.getPeerByPublicKey?.(peerId);
      if (peer && this.gossip.sendFn) {
        const message = {
          id: requestId,
          type: MessageType.CONSENSUS_MERKLE_DIFF_REQUEST,
          payload: {
            theirRoot,
            ourRoot,
            sinceSlot,
            requestId,
          },
          sender: this.consensus.publicKey,
          timestamp: Date.now(),
          ttl: 1,
          hops: 0,
        };
        await this.gossip.sendFn(peer, message);
      } else {
        return { error: 'peer-not-found' };
      }
    } catch (err) {
      return { error: err.message };
    }

    return responsePromise;
  }

  /**
   * Get Merkle state tree statistics
   * @returns {Object} State tree stats
   */
  getMerkleStats() {
    return {
      stateTree: this.stateTree.getStats(),
      peerRoots: Object.fromEntries(this.peerRoots),
      stats: {
        merkleRootsSent: this.stats.merkleRootsSent,
        merkleRootsReceived: this.stats.merkleRootsReceived,
        merkleDiffRequests: this.stats.merkleDiffRequests,
        merkleDiffResponses: this.stats.merkleDiffResponses,
        merkleDiffBlocksSynced: this.stats.merkleDiffBlocksSynced,
        fullSyncFallbacks: this.stats.fullSyncFallbacks,
      },
    };
  }
}

export default { ConsensusGossip };

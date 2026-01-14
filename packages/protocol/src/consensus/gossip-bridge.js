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
   */
  constructor({ consensus, gossip }) {
    super();

    this.consensus = consensus;
    this.gossip = gossip;

    this.started = false;
    this.stats = {
      proposalsBroadcast: 0,
      votesBroadcast: 0,
      finalityBroadcast: 0,
      proposalsReceived: 0,
      votesReceived: 0,
      finalityReceived: 0,
    };

    // Bind methods
    this._onConsensusEvent = this._onConsensusEvent.bind(this);
    this._onGossipMessage = this._onGossipMessage.bind(this);
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

    this.started = true;
    this.emit('started');
  }

  /**
   * Stop the bridge
   */
  stop() {
    if (!this.started) return;

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
    const vote = {
      blockHash: event.blockHash,
      slot: event.slot,
      decision: event.decision,
      voter: this.consensus.publicKey,
      weight: event.weight,
    };

    const sent = await this.gossip.broadcastVote(vote);
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

    // Forward to consensus engine
    const vote = {
      blockHash: payload.blockHash,
      slot: payload.slot,
      decision: payload.decision,
      voter: payload.voter,
      weight: payload.weight,
      signature: payload.signature,
    };

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
   * Handle received slot status
   * @private
   */
  async _handleSlotStatus(message) {
    const { payload, sender } = message;

    // Emit for sync purposes
    this.emit('slot-status:received', {
      slot: payload.slot,
      leader: payload.leader,
      from: sender,
    });
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
}

export default { ConsensusGossip };

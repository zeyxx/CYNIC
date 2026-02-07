/**
 * φ-BFT Consensus Engine
 *
 * Main orchestrator for CYNIC consensus:
 * - Slot-based block production
 * - Vote collection and verification
 * - φ exponential lockout
 * - Block finality (61.8% supermajority)
 *
 * @module @cynic/protocol/consensus/engine
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI, PHI_INV, CONSENSUS_THRESHOLD, SLOT_MS } from '@cynic/core';
import { hashObject } from '../crypto/hash.js';
import {
  VoteType,
  ConsensusType,
  createVote,
  verifyVote,
  calculateConsensus,
  calculateVoteWeight,
} from './voting.js';
import { LockoutManager, calculateTotalLockout } from './lockout.js';

/**
 * Verify consensus block integrity
 * Note: Consensus blocks don't require cryptographic signatures because
 * they are validated through the voting process itself. This function
 * checks basic structural integrity.
 *
 * @param {Object} block - Block to verify
 * @param {string} expectedHash - Expected hash (optional)
 * @returns {boolean} True if valid
 */
function verifyConsensusBlock(block, expectedHash = null) {
  // Required fields
  if (!block) return false;
  if (typeof block.slot !== 'number') return false;
  if (!block.proposer) return false;

  // Verify hash if provided
  if (expectedHash && block.hash && block.hash !== expectedHash) {
    return false;
  }

  return true;
}

/**
 * Consensus state
 */
export const ConsensusState = {
  INITIALIZING: 'INITIALIZING',
  SYNCING: 'SYNCING',
  PARTICIPATING: 'PARTICIPATING',
  LEADER: 'LEADER',
  STOPPED: 'STOPPED',
};

/**
 * Block status in consensus
 */
export const BlockStatus = {
  PROPOSED: 'PROPOSED',
  VOTING: 'VOTING',
  CONFIRMED: 'CONFIRMED',
  FINALIZED: 'FINALIZED',
  REJECTED: 'REJECTED',
  ORPHANED: 'ORPHANED',
};

/**
 * φ-BFT Consensus Engine
 *
 * Implements CYNIC's 4th layer consensus with:
 * - φ⁻¹ (61.8%) supermajority threshold
 * - E-Score weighted voting
 * - Exponential lockout (φⁿ slots)
 * - Probabilistic finality after sufficient confirmations
 */
export class ConsensusEngine extends EventEmitter {
  /**
   * @param {Object} options - Engine options
   * @param {string} options.publicKey - Node public key
   * @param {string} options.privateKey - Node private key
   * @param {number} [options.eScore=50] - Node E-Score
   * @param {number} [options.burned=0] - Node burned tokens
   * @param {number} [options.slotDuration=SLOT_MS] - Slot duration in ms
   * @param {number} [options.confirmationsForFinality=32] - Confirmations needed
   * @param {number} [options.maxPendingVotes=10000] - Max queued votes for unknown blocks
   * @param {number} [options.pendingVoteMaxAge=100] - Max slots a pending vote can wait
   * @param {number} [options.maxBlockHistory=1000] - Max slots of block history to retain
   */
  constructor(options) {
    super();

    this.publicKey = options.publicKey;
    this.privateKey = options.privateKey;
    this.eScore = options.eScore || 50;
    this.burned = options.burned || 0;
    this.slotDuration = options.slotDuration || SLOT_MS || 400;
    this.confirmationsForFinality = options.confirmationsForFinality || 32;
    this.maxPendingVotes = options.maxPendingVotes || 10000;
    this.pendingVoteMaxAge = options.pendingVoteMaxAge || 100;
    this.maxBlockHistory = options.maxBlockHistory || 1000;

    // State
    this.state = ConsensusState.INITIALIZING;
    this.currentSlot = 0;
    this.genesisTime = null;

    // Block tracking
    this.blocks = new Map(); // blockHash -> BlockRecord
    this.slotBlocks = new Map(); // slot -> Set<blockHash>
    this.slotProposals = new Map(); // slot -> Map<proposer, blockHash> (equivocation detection)
    this.lastFinalizedSlot = 0;
    this.lastFinalizedBlock = null;

    // Vote tracking
    this.votes = new Map(); // blockHash -> Map<voter, vote>
    this.pendingVotes = []; // Votes waiting to be processed

    // Lockout
    this.lockoutManager = new LockoutManager();

    // Leader selection (simplified round-robin for now)
    this.validators = new Map(); // publicKey -> { eScore, weight, ... }

    // Timers
    this.slotTimer = null;

    // Stats
    this.stats = {
      blocksProposed: 0,
      blocksFinalized: 0,
      votesSubmitted: 0,
      votesReceived: 0,
      slotsProcessed: 0,
    };
  }

  /**
   * Start consensus engine
   * @param {number} [genesisTime] - Network genesis time
   */
  start(genesisTime = Date.now()) {
    if (this.state !== ConsensusState.INITIALIZING && this.state !== ConsensusState.STOPPED) {
      return;
    }

    this.genesisTime = genesisTime;
    this.currentSlot = this._calculateSlot(Date.now());
    this.state = ConsensusState.PARTICIPATING;

    // Start slot timer
    this._startSlotTimer();

    this.emit('consensus:started', {
      slot: this.currentSlot,
      genesisTime: this.genesisTime,
    });
  }

  /**
   * Stop consensus engine
   */
  stop() {
    this.state = ConsensusState.STOPPED;

    if (this.slotTimer) {
      clearTimeout(this.slotTimer);
      clearInterval(this.slotTimer);
      this.slotTimer = null;
    }

    this.emit('consensus:stopped');
    this.removeAllListeners();
  }

  /**
   * Register a validator
   * @param {Object} validator - Validator info
   * @param {string} validator.publicKey - Validator public key
   * @param {number} validator.eScore - Validator E-Score
   * @param {number} [validator.burned=0] - Burned tokens
   * @param {number} [validator.uptime=1] - Uptime ratio
   */
  registerValidator(validator) {
    const weight = calculateVoteWeight({
      eScore: validator.eScore,
      burned: validator.burned || 0,
      uptime: validator.uptime || 1,
    });

    this.validators.set(validator.publicKey, {
      ...validator,
      weight,
      registeredAt: Date.now(),
    });

    this.emit('validator:registered', { publicKey: validator.publicKey, weight });
  }

  /**
   * Remove a validator
   * @param {string} publicKey - Validator public key
   */
  removeValidator(publicKey) {
    this.validators.delete(publicKey);
    this.emit('validator:removed', { publicKey });
  }

  /**
   * Propose a block
   * @param {Object} block - Block to propose
   * @returns {Object} Block record
   */
  proposeBlock(block) {
    if (this.state !== ConsensusState.PARTICIPATING && this.state !== ConsensusState.LEADER) {
      throw new Error('Not participating in consensus');
    }

    const blockHash = block.hash || hashObject(block);
    const slot = block.slot || this.currentSlot;

    // Create block record
    const record = {
      hash: blockHash,
      block,
      slot,
      proposer: block.proposer || this.publicKey,
      status: BlockStatus.PROPOSED,
      proposedAt: Date.now(),
      votes: new Map(),
      confirmations: 0,
      totalWeight: 0,
      approveWeight: 0,
      rejectWeight: 0,
    };

    this.blocks.set(blockHash, record);

    // Track by slot
    if (!this.slotBlocks.has(slot)) {
      this.slotBlocks.set(slot, new Set());
    }
    this.slotBlocks.get(slot).add(blockHash);

    // Track proposal for equivocation detection
    if (!this.slotProposals.has(slot)) {
      this.slotProposals.set(slot, new Map());
    }
    this.slotProposals.get(slot).set(record.proposer, blockHash);

    // Initialize vote tracking
    this.votes.set(blockHash, new Map());

    this.stats.blocksProposed++;
    this.emit('block:proposed', {
      event: 'block:proposed',
      blockHash,
      slot,
      block,
      proposer: record.proposer,
    });

    // Self-vote (approve own block)
    this._submitVote(blockHash, VoteType.APPROVE);

    return record;
  }

  /**
   * Receive a proposed block from network
   * @param {Object} block - Block received
   * @param {string} fromPeer - Peer who sent it
   */
  receiveBlock(block, fromPeer) {
    const blockHash = block.hash || hashObject(block);

    // Already have this block
    if (this.blocks.has(blockHash)) {
      return;
    }

    // SECURITY: Verify block integrity before any processing
    // Note: Cryptographic signatures are not required for consensus blocks
    // because validation happens through the voting process itself
    if (!verifyConsensusBlock(block, blockHash)) {
      this.emit('block:invalid', {
        blockHash,
        reason: 'invalid_structure',
        fromPeer,
      });
      return;
    }

    // Validate block slot
    const slot = block.slot;
    if (slot < this.lastFinalizedSlot) {
      // Too old, ignore
      return;
    }

    // Equivocation detection: check if proposer already proposed for this slot
    const proposer = block.proposer;
    const slotProposals = this.slotProposals.get(slot);
    if (slotProposals && slotProposals.has(proposer)) {
      const existingBlockHash = slotProposals.get(proposer);
      if (existingBlockHash !== blockHash) {
        // EQUIVOCATION DETECTED: same proposer, same slot, different block
        this.emit('block:equivocation', {
          slot,
          proposer,
          existingBlock: existingBlockHash,
          conflictingBlock: blockHash,
          fromPeer,
        });
        this.emit('block:rejected', {
          blockHash,
          reason: `Equivocation: proposer already proposed block ${existingBlockHash.slice(0, 16)}... for slot ${slot}`,
        });
        return;
      }
    }

    // Create record
    const record = {
      hash: blockHash,
      block,
      slot,
      proposer: block.proposer,
      status: BlockStatus.VOTING,
      receivedAt: Date.now(),
      receivedFrom: fromPeer,
      votes: new Map(),
      confirmations: 0,
      totalWeight: 0,
      approveWeight: 0,
      rejectWeight: 0,
    };

    this.blocks.set(blockHash, record);

    if (!this.slotBlocks.has(slot)) {
      this.slotBlocks.set(slot, new Set());
    }
    this.slotBlocks.get(slot).add(blockHash);

    // Track proposal for equivocation detection
    if (!this.slotProposals.has(slot)) {
      this.slotProposals.set(slot, new Map());
    }
    this.slotProposals.get(slot).set(proposer, blockHash);

    this.votes.set(blockHash, new Map());

    this.emit('block:received', { blockHash, slot, fromPeer });

    // Validate and vote
    this._validateAndVote(record);
  }

  /**
   * Receive a vote from network
   * @param {Object} vote - Vote received
   * @param {string} fromPeer - Peer who sent it
   */
  receiveVote(vote, fromPeer) {
    // Verify vote signature
    if (!verifyVote(vote)) {
      this.emit('vote:invalid', { vote, reason: 'signature' });
      return;
    }

    const blockHash = vote.block_hash || vote.proposal_id;
    const record = this.blocks.get(blockHash);

    if (!record) {
      // Don't have the block yet, queue vote (with bounds)
      if (this.pendingVotes.length >= this.maxPendingVotes) {
        // Evict oldest vote to make room
        this.pendingVotes.shift();
      }
      this.pendingVotes.push({
        vote,
        receivedSlot: this.currentSlot,
      });
      return;
    }

    // Check if voter is locked out
    if (vote.vote === VoteType.REJECT) {
      if (this.lockoutManager.isLockedOut(vote.voter, blockHash, this.currentSlot)) {
        this.emit('vote:rejected', { vote, reason: 'lockout' });
        return;
      }
    }

    // Add vote
    this._addVote(record, vote);
    this.stats.votesReceived++;
  }

  /**
   * Submit own vote for a block
   * @param {string} blockHash - Block hash
   * @param {string} voteType - Vote type
   * @returns {Object} Vote object
   */
  vote(blockHash, voteType) {
    return this._submitVote(blockHash, voteType);
  }

  /**
   * Get block status
   * @param {string} blockHash - Block hash
   * @returns {Object|null} Block record or null
   */
  getBlock(blockHash) {
    return this.blocks.get(blockHash) || null;
  }

  /**
   * Get blocks at slot
   * @param {number} slot - Slot number
   * @returns {Object[]} Block records
   */
  getBlocksAtSlot(slot) {
    const hashes = this.slotBlocks.get(slot);
    if (!hashes) return [];
    return Array.from(hashes).map((h) => this.blocks.get(h)).filter(Boolean);
  }

  /**
   * Check if block is finalized
   * @param {string} blockHash - Block hash
   * @returns {boolean} True if finalized
   */
  isFinalized(blockHash) {
    const record = this.blocks.get(blockHash);
    return record?.status === BlockStatus.FINALIZED;
  }

  /**
   * Get current slot
   * @returns {number} Current slot
   */
  getCurrentSlot() {
    return this.currentSlot;
  }

  /**
   * Get consensus state summary
   * @returns {Object} State summary
   */
  getState() {
    return {
      state: this.state,
      currentSlot: this.currentSlot,
      latestSlot: this.currentSlot,
      finalizedSlot: this.lastFinalizedSlot,
      pendingBlocks: Array.from(this.blocks.values()).filter(
        (b) => b.status === BlockStatus.VOTING || b.status === BlockStatus.PROPOSED
      ).length,
      confirmedBlocks: Array.from(this.blocks.values()).filter(
        (b) => b.status === BlockStatus.CONFIRMED
      ).length,
      validators: this.validators.size,
    };
  }

  /**
   * Get consensus statistics
   * @returns {Object} Statistics
   */
  getStats() {
    const lockoutStats = this.lockoutManager.getStats(this.currentSlot);

    return {
      ...this.stats,
      state: this.state,
      currentSlot: this.currentSlot,
      lastFinalizedSlot: this.lastFinalizedSlot,
      pendingBlocks: Array.from(this.blocks.values()).filter(
        (b) => b.status === BlockStatus.VOTING
      ).length,
      validators: this.validators.size,
      totalWeight: this._getTotalValidatorWeight(),
      ...lockoutStats,
    };
  }

  // ─────────────────────────────────────────────────────────────────
  // State Sync Methods (for late joiners)
  // ─────────────────────────────────────────────────────────────────

  /**
   * Export finalized blocks for state sync
   * Returns blocks that have reached FINALIZED status
   *
   * @param {number} [sinceSlot=0] - Export blocks since this slot
   * @param {number} [maxBlocks=50] - Maximum number of blocks to export
   * @returns {Object} Exported state
   */
  exportState(sinceSlot = 0, maxBlocks = 50) {
    const finalizedBlocks = [];

    for (const [blockHash, record] of this.blocks) {
      if (record.status === BlockStatus.FINALIZED && record.slot >= sinceSlot) {
        finalizedBlocks.push({
          hash: blockHash,
          slot: record.slot,
          proposer: record.proposer,
          block: record.block,
          status: record.status,
          approveWeight: record.approveWeight,
          confirmations: record.confirmations,
          finalizedAt: record.finalizedAt,
        });
      }
    }

    // Sort by slot ascending
    finalizedBlocks.sort((a, b) => a.slot - b.slot);

    // Limit results
    const blocks = finalizedBlocks.slice(0, maxBlocks);

    return {
      blocks,
      latestSlot: this.lastFinalizedSlot,
      validatorCount: this.validators.size,
      genesisTime: this.genesisTime,
    };
  }

  /**
   * Import synced state from a peer
   * Used by late-joining nodes to catch up on finalized history
   *
   * @param {Object} state - State to import
   * @param {Object[]} state.blocks - Finalized blocks
   * @param {number} state.latestSlot - Latest finalized slot from peer
   * @param {number} [state.genesisTime] - Network genesis time
   * @returns {Object} Import result
   */
  importState(state) {
    const { blocks, latestSlot, genesisTime } = state;

    if (!blocks || !Array.isArray(blocks)) {
      return { imported: 0, error: 'Invalid state: blocks must be an array' };
    }

    let imported = 0;
    const errors = [];

    // Set genesis time if not already set
    if (genesisTime && !this.genesisTime) {
      this.genesisTime = genesisTime;
    }

    for (const blockData of blocks) {
      try {
        // Skip if we already have this block
        if (this.blocks.has(blockData.hash)) {
          continue;
        }

        // Create block record
        const record = {
          hash: blockData.hash,
          slot: blockData.slot,
          proposer: blockData.proposer,
          block: blockData.block,
          status: BlockStatus.FINALIZED, // Trust peer's finalized state
          approveWeight: blockData.approveWeight || 0,
          totalWeight: blockData.approveWeight || 0,
          confirmations: blockData.confirmations || this.confirmationsForFinality,
          votes: new Map(),
          receivedAt: Date.now(),
          finalizedAt: blockData.finalizedAt || Date.now(),
        };

        // Store block
        this.blocks.set(blockData.hash, record);

        // Track in slot index
        if (!this.slotBlocks.has(record.slot)) {
          this.slotBlocks.set(record.slot, new Set());
        }
        this.slotBlocks.get(record.slot).add(blockData.hash);

        imported++;

        this.emit('block:synced', {
          event: 'block:synced',
          blockHash: blockData.hash,
          slot: record.slot,
        });
      } catch (err) {
        errors.push({ hash: blockData.hash, error: err.message });
      }
    }

    // Update our finalized slot if peer is ahead.
    // Guard: calculate a fresh slot from current time (not this.currentSlot
    // which may lag behind if the slot timer hasn't fired yet). Reject any
    // latestSlot that's ahead of what the current time says — this is stale
    // data from a previous deployment epoch.
    if (latestSlot > this.lastFinalizedSlot) {
      const freshSlot = this._calculateSlot(Date.now());
      if (freshSlot > 0 && latestSlot <= freshSlot) {
        this.lastFinalizedSlot = latestSlot;
      }
    }

    return {
      imported,
      total: blocks.length,
      latestSlot: this.lastFinalizedSlot,
      errors: errors.length > 0 ? errors : undefined,
    };
  }

  /**
   * Check if we need to sync (are we behind the network?)
   * @param {number} networkLatestSlot - Latest slot reported by network
   * @returns {boolean} True if sync needed
   */
  needsSync(networkLatestSlot) {
    return networkLatestSlot > this.lastFinalizedSlot;
  }

  // ─────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────

  /**
   * Calculate slot from timestamp
   * @private
   */
  _calculateSlot(timestamp) {
    if (!this.genesisTime) return 0;
    return Math.floor((timestamp - this.genesisTime) / this.slotDuration);
  }

  /**
   * Start slot timer
   * @private
   */
  _startSlotTimer() {
    // Calculate time until next slot
    const now = Date.now();
    const currentSlotStart = this.genesisTime + this.currentSlot * this.slotDuration;
    const nextSlotStart = currentSlotStart + this.slotDuration;
    const delay = nextSlotStart - now;

    this.slotTimer = setTimeout(() => {
      if (this.state === ConsensusState.STOPPED) return;
      this._onSlotChange();
      this.slotTimer = setInterval(() => {
        if (this.state === ConsensusState.STOPPED) return;
        this._onSlotChange();
      }, this.slotDuration);
    }, delay > 0 ? delay : 0);
  }

  /**
   * Handle slot change
   * @private
   */
  _onSlotChange() {
    const previousSlot = this.currentSlot;
    this.currentSlot = this._calculateSlot(Date.now());

    if (this.currentSlot <= previousSlot) return;

    // Detect stale lastFinalizedSlot (e.g. from syncing with a previous deployment epoch)
    // If lastFinalizedSlot is ahead of currentSlot, it's stale data — reset to 0
    // so that receiveBlock() stops rejecting valid peer blocks and pruning works correctly.
    if (this.lastFinalizedSlot > this.currentSlot && this.currentSlot > 0) {
      console.warn(`[Consensus] Stale lastFinalizedSlot detected: ${this.lastFinalizedSlot} > currentSlot ${this.currentSlot}, resetting`);
      this.lastFinalizedSlot = 0;
      this.lastFinalizedBlock = null;
    }

    this.stats.slotsProcessed++;

    // Process pending votes
    this._processPendingVotes();

    // Check for block confirmations
    this._processConfirmations();

    // Check for finality
    this._checkFinality();

    // Prune old lockouts
    if (this.currentSlot % 100 === 0) {
      this.lockoutManager.prune(this.currentSlot);
    }

    // Periodic pruning of stale blocks (bounds memory even without finalization)
    if (this.currentSlot % 50 === 0) {
      this._pruneStaleBlocks();
    }

    this.emit('slot:change', {
      previousSlot,
      currentSlot: this.currentSlot,
    });
  }

  /**
   * Submit own vote
   * @private
   */
  _submitVote(blockHash, voteType) {
    const record = this.blocks.get(blockHash);
    if (!record) {
      throw new Error(`Unknown block: ${blockHash}`);
    }

    // Check lockout if rejecting
    if (voteType === VoteType.REJECT) {
      const lockout = this.lockoutManager.canSwitchVote(this.publicKey, this.currentSlot);
      if (!lockout.canSwitch && lockout.locked.some((l) => l.blockHash === blockHash)) {
        throw new Error('Locked out from voting against this block');
      }
    }

    // Create vote
    const vote = createVote({
      proposalId: blockHash,
      vote: voteType,
      voterPublicKey: this.publicKey,
      voterPrivateKey: this.privateKey,
      eScore: this.eScore,
      burned: this.burned,
      uptime: 1,
    });

    vote.block_hash = blockHash;
    vote.slot = this.currentSlot;

    // Add to record
    this._addVote(record, vote);

    // Record for lockout if approving
    if (voteType === VoteType.APPROVE) {
      this.lockoutManager.recordVote(this.publicKey, blockHash, this.currentSlot);
    }

    this.stats.votesSubmitted++;

    // Emit vote:cast for gossip broadcast
    this.emit('vote:cast', {
      event: 'vote:cast',
      blockHash,
      slot: this.currentSlot,
      decision: voteType,
      weight: vote.weight,
      vote,
    });

    return vote;
  }

  /**
   * Add vote to block record
   * @private
   */
  _addVote(record, vote) {
    const blockVotes = this.votes.get(record.hash);
    if (!blockVotes) return;

    // Reject duplicate votes (vote immutability for Byzantine safety)
    const existing = blockVotes.get(vote.voter);
    if (existing) {
      // Already voted - reject the new vote
      // This prevents double-voting attacks
      if (existing.vote !== vote.vote) {
        this.emit('vote:double-vote-attempt', {
          blockHash: record.hash,
          voter: vote.voter,
          existingVote: existing.vote,
          attemptedVote: vote.vote,
        });
      }
      return; // Ignore duplicate vote
    }

    // Add new vote (first vote only)
    blockVotes.set(vote.voter, vote);
    record.votes.set(vote.voter, vote);

    // Update weights
    const weight = vote.weight || 1;
    if (vote.vote === VoteType.APPROVE) {
      record.approveWeight += weight;
      // Record lockout
      this.lockoutManager.recordVote(vote.voter, record.hash, this.currentSlot);
    } else if (vote.vote === VoteType.REJECT) {
      record.rejectWeight += weight;
    }
    record.totalWeight = record.approveWeight + record.rejectWeight;

    // Check for consensus
    this._checkBlockConsensus(record);

    this.emit('vote:added', {
      blockHash: record.hash,
      voter: vote.voter,
      voteType: vote.vote,
      weight,
    });
  }

  /**
   * Check if block has reached consensus
   * @private
   */
  _checkBlockConsensus(record) {
    if (record.status === BlockStatus.FINALIZED || record.status === BlockStatus.REJECTED) {
      return;
    }

    const totalWeight = this._getTotalValidatorWeight();
    if (totalWeight === 0) return;

    const ratio = record.approveWeight / totalWeight;

    // Check for supermajority approval
    if (ratio >= CONSENSUS_THRESHOLD) {
      record.status = BlockStatus.CONFIRMED;
      record.confirmations++;
      record.confirmedAt = Date.now();

      this.emit('block:confirmed', {
        blockHash: record.hash,
        slot: record.slot,
        ratio,
        confirmations: record.confirmations,
      });
    }

    // Check for rejection
    const rejectRatio = record.rejectWeight / totalWeight;
    if (rejectRatio > 1 - CONSENSUS_THRESHOLD) {
      record.status = BlockStatus.REJECTED;
      record.rejectedAt = Date.now();

      this.emit('block:rejected', {
        blockHash: record.hash,
        slot: record.slot,
        ratio: rejectRatio,
      });
    }
  }

  /**
   * Validate block and submit vote
   * @private
   */
  _validateAndVote(record) {
    // Basic validation (can be extended)
    const isValid = this._validateBlock(record.block);

    // Check if we should vote
    const voteType = isValid ? VoteType.APPROVE : VoteType.REJECT;

    try {
      this._submitVote(record.hash, voteType);
    } catch (err) {
      // Likely locked out, which is fine
      this.emit('vote:skipped', { blockHash: record.hash, reason: err.message });
    }
  }

  /**
   * Validate a block
   * @private
   */
  _validateBlock(block) {
    // Basic validation
    if (!block) return false;
    if (!block.slot && block.slot !== 0) return false;
    if (!block.proposer) return false;

    // Check proposer is valid validator
    if (!this.validators.has(block.proposer)) {
      // Unknown proposer - could be valid in decentralized network
      // For now, accept blocks from unknown proposers
    }

    return true;
  }

  /**
   * Process pending votes
   * @private
   */
  _processPendingVotes() {
    const stillPending = [];
    let expired = 0;

    for (const pending of this.pendingVotes) {
      const { vote, receivedSlot } = pending;

      // Evict votes that are too old (block never arrived)
      if (this.currentSlot - receivedSlot > this.pendingVoteMaxAge) {
        expired++;
        continue;
      }

      const blockHash = vote.block_hash || vote.proposal_id;
      const record = this.blocks.get(blockHash);

      if (record) {
        this._addVote(record, vote);
      } else {
        // Still don't have block, keep pending
        stillPending.push(pending);
      }
    }

    if (expired > 0) {
      console.warn(`[Consensus] Evicted ${expired} pending votes (block never received)`);
    }

    this.pendingVotes = stillPending;
  }

  /**
   * Process block confirmations
   * @private
   */
  _processConfirmations() {
    for (const record of this.blocks.values()) {
      if (record.status === BlockStatus.CONFIRMED) {
        // Increment confirmations if still receiving votes
        const totalWeight = this._getTotalValidatorWeight();
        const ratio = record.approveWeight / totalWeight;

        if (ratio >= CONSENSUS_THRESHOLD) {
          record.confirmations++;
        }
      }
    }
  }

  /**
   * Check for block finality
   * @private
   */
  _checkFinality() {
    let newlyFinalized = false;

    for (const record of this.blocks.values()) {
      if (record.status === BlockStatus.CONFIRMED) {
        // Check if enough confirmations for finality
        if (record.confirmations >= this.confirmationsForFinality) {
          record.status = BlockStatus.FINALIZED;
          record.finalizedAt = Date.now();

          // Update finalized slot
          if (record.slot > this.lastFinalizedSlot) {
            this.lastFinalizedSlot = record.slot;
            this.lastFinalizedBlock = record.hash;
          }

          // Mark conflicting blocks as orphaned
          this._orphanConflictingBlocks(record);

          this.stats.blocksFinalized++;
          newlyFinalized = true;
          this.emit('block:finalized', {
            event: 'block:finalized',
            blockHash: record.hash,
            slot: record.slot,
            block: record.block,
            confirmations: record.confirmations,
            status: 'FINALIZED',
            probability: 1.0,
          });
        }
      }
    }

    // Prune old data after finalization to bound memory usage
    if (newlyFinalized) {
      this._pruneOldData();
    }
  }

  /**
   * Mark conflicting blocks as orphaned
   * @private
   */
  _orphanConflictingBlocks(finalizedRecord) {
    const slotBlocks = this.slotBlocks.get(finalizedRecord.slot);
    if (!slotBlocks) return;

    for (const blockHash of slotBlocks) {
      if (blockHash !== finalizedRecord.hash) {
        const record = this.blocks.get(blockHash);
        if (record && record.status !== BlockStatus.FINALIZED) {
          record.status = BlockStatus.ORPHANED;
          this.emit('block:orphaned', { blockHash, slot: record.slot });
        }
      }
    }
  }

  /**
   * Prune old block data to prevent unbounded memory growth
   * Removes blocks, votes, and slot indices older than maxBlockHistory
   * @private
   */
  _pruneOldData() {
    const cutoffSlot = this.lastFinalizedSlot - this.maxBlockHistory;
    if (cutoffSlot <= 0) return;

    let pruned = 0;

    // Collect block hashes to remove
    const blocksToRemove = [];
    for (const [blockHash, record] of this.blocks) {
      if (record.slot < cutoffSlot) {
        blocksToRemove.push(blockHash);
      }
    }

    // Remove blocks and their votes
    for (const blockHash of blocksToRemove) {
      this.blocks.delete(blockHash);
      this.votes.delete(blockHash);
      pruned++;
    }

    // Remove old slot indices
    for (const slot of this.slotBlocks.keys()) {
      if (slot < cutoffSlot) {
        this.slotBlocks.delete(slot);
      }
    }

    // Remove old slot proposals
    for (const slot of this.slotProposals.keys()) {
      if (slot < cutoffSlot) {
        this.slotProposals.delete(slot);
      }
    }

    if (pruned > 0) {
      this.emit('consensus:pruned', {
        event: 'consensus:pruned',
        prunedBlocks: pruned,
        cutoffSlot,
        remainingBlocks: this.blocks.size,
      });
    }
  }

  /**
   * Prune stale non-finalized blocks to bound memory.
   * Uses currentSlot as reference (not lastFinalizedSlot which may be stale).
   * Called periodically from _onSlotChange regardless of finalization progress.
   * @private
   */
  _pruneStaleBlocks() {
    const cutoff = this.currentSlot - this.maxBlockHistory;
    if (cutoff <= 0) return;

    let pruned = 0;

    for (const [hash, record] of this.blocks) {
      if (record.slot < cutoff && record.status !== BlockStatus.FINALIZED) {
        this.blocks.delete(hash);
        this.votes.delete(hash);
        pruned++;
      }
    }

    // Clean slot indices
    for (const slot of this.slotBlocks.keys()) {
      if (slot < cutoff) this.slotBlocks.delete(slot);
    }
    for (const slot of this.slotProposals.keys()) {
      if (slot < cutoff) this.slotProposals.delete(slot);
    }

    if (pruned > 0) {
      this.emit('consensus:pruned', {
        event: 'consensus:pruned',
        prunedBlocks: pruned,
        cutoffSlot: cutoff,
        remainingBlocks: this.blocks.size,
        source: 'periodic',
      });
    }
  }

  /**
   * Get total validator weight
   * @private
   */
  _getTotalValidatorWeight() {
    let total = 0;
    for (const validator of this.validators.values()) {
      total += validator.weight || 1;
    }
    return total || 1; // Prevent division by zero
  }
}

/**
 * Create consensus vote message for gossip
 * @param {Object} vote - Vote object
 * @param {string} senderPublicKey - Sender public key
 * @returns {Object} Gossip message
 */
export function createConsensusVoteMessage(vote, senderPublicKey) {
  return {
    type: 'CONSENSUS_VOTE',
    payload: vote,
    sender: senderPublicKey,
    timestamp: Date.now(),
  };
}

/**
 * Create block proposal message for gossip
 * @param {Object} block - Block to propose
 * @param {string} senderPublicKey - Sender public key
 * @returns {Object} Gossip message
 */
export function createBlockProposalMessage(block, senderPublicKey) {
  return {
    type: 'BLOCK_PROPOSAL',
    payload: block,
    sender: senderPublicKey,
    timestamp: Date.now(),
  };
}

export default {
  ConsensusState,
  BlockStatus,
  ConsensusEngine,
  createConsensusVoteMessage,
  createBlockProposalMessage,
};

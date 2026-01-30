/**
 * CYNIC Blockchain Bridge
 *
 * Connects the full loop: Judgment → Block → Anchor → E-Score → Collective
 *
 * Phase 5: "Onchain is truth"
 *
 * This bridge subscribes to blockchain events and:
 * 1. Updates E-Score 7D when blocks are anchored
 * 2. Creates patterns for collective memory
 * 3. Notifies collective dogs of on-chain truth
 *
 * @module @cynic/mcp/blockchain-bridge
 */

'use strict';

import { globalEventBus, createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('BlockchainBridge');

// ═══════════════════════════════════════════════════════════════════════════
// BLOCKCHAIN EVENTS (must match poj-chain-manager.js)
// ═══════════════════════════════════════════════════════════════════════════
const BlockchainEvent = {
  BLOCK_CREATED: 'poj:block:created',
  BLOCK_ANCHORED: 'poj:block:anchored',
  ANCHOR_FAILED: 'poj:anchor:failed',
};

// ═══════════════════════════════════════════════════════════════════════════
// BLOCKCHAIN BRIDGE CLASS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * BlockchainBridge - Wires blockchain events to E-Score and collective
 *
 * @example
 * const bridge = new BlockchainBridge({
 *   eScore: eScore7DCalculator,
 *   collective: collectivePack,
 *   persistence: persistenceManager,
 * });
 * bridge.start();
 */
export class BlockchainBridge {
  /**
   * @param {Object} options
   * @param {Object} [options.eScore] - EScore7DCalculator instance
   * @param {Object} [options.collective] - CollectivePack instance
   * @param {Object} [options.persistence] - PersistenceManager instance
   * @param {Object} [options.burnVerifier] - BurnVerifier instance
   */
  constructor(options = {}) {
    this.eScore = options.eScore || null;
    this.collective = options.collective || null;
    this.persistence = options.persistence || null;
    this.burnVerifier = options.burnVerifier || null;

    // Subscription IDs for cleanup
    this._subscriptions = [];

    // Stats
    this.stats = {
      blocksCreated: 0,
      blocksAnchored: 0,
      anchorsFailed: 0,
      eScoreUpdates: 0,
      patternsCreated: 0,
    };

    // Track judgment consensus for E-Score JUDGE dimension
    this._judgmentConsensus = new Map(); // blockHash → consensus data
  }

  // ===========================================================================
  // LIFECYCLE
  // ===========================================================================

  /**
   * Start listening to blockchain events
   */
  start() {
    log.info('Blockchain bridge starting...');

    // Subscribe to block created
    const blockCreatedSub = globalEventBus.subscribe(
      BlockchainEvent.BLOCK_CREATED,
      this._onBlockCreated.bind(this)
    );
    this._subscriptions.push(blockCreatedSub);

    // Subscribe to block anchored
    const blockAnchoredSub = globalEventBus.subscribe(
      BlockchainEvent.BLOCK_ANCHORED,
      this._onBlockAnchored.bind(this)
    );
    this._subscriptions.push(blockAnchoredSub);

    // Subscribe to anchor failed
    const anchorFailedSub = globalEventBus.subscribe(
      BlockchainEvent.ANCHOR_FAILED,
      this._onAnchorFailed.bind(this)
    );
    this._subscriptions.push(anchorFailedSub);

    log.info('Blockchain bridge started', {
      subscriptions: this._subscriptions.length,
      eScore: !!this.eScore,
      collective: !!this.collective,
    });
  }

  /**
   * Stop listening to blockchain events
   */
  stop() {
    for (const sub of this._subscriptions) {
      try {
        globalEventBus.unsubscribe(sub);
      } catch (e) { /* ignore */ }
    }
    this._subscriptions = [];
    log.info('Blockchain bridge stopped');
  }

  // ===========================================================================
  // EVENT HANDLERS
  // ===========================================================================

  /**
   * Handle PoJ block creation
   * @param {Object} event - Block created event
   * @private
   */
  async _onBlockCreated(event) {
    this.stats.blocksCreated++;

    const {
      slot,
      hash,
      judgmentCount,
      judgmentIds,
      timestamp,
    } = event.data || event;

    log.debug('Block created', { slot, hash: hash?.slice(0, 16), judgmentCount });

    // Track consensus for later E-Score update when anchored
    this._judgmentConsensus.set(hash, {
      slot,
      judgmentCount,
      judgmentIds,
      timestamp,
      consensusMatched: 0, // Will be updated with actual consensus data
    });

    // Notify collective oracle of pending block
    if (this.collective?.oracle) {
      try {
        await this.collective.oracle.observeBlock?.({
          type: 'block_pending',
          slot,
          hash,
          judgmentCount,
        });
      } catch (e) { /* non-blocking */ }
    }
  }

  /**
   * Handle PoJ block anchored to Solana
   *
   * This is the critical moment: block is now ON-CHAIN TRUTH
   * Update E-Score JUDGE dimension and create patterns
   *
   * @param {Object} event - Block anchored event
   * @private
   */
  async _onBlockAnchored(event) {
    this.stats.blocksAnchored++;

    const {
      slot,
      hash,
      signature,
      solanaSlot,
      merkleRoot,
      anchoredAt,
      judgmentCount,
    } = event.data || event;

    log.info('Block anchored - ON-CHAIN TRUTH', {
      slot,
      signature: signature?.slice(0, 16),
      judgmentCount,
    });

    // ═══════════════════════════════════════════════════════════════════════
    // 1. UPDATE E-SCORE 7D - JUDGE DIMENSION
    // ═══════════════════════════════════════════════════════════════════════
    if (this.eScore?.recordJudgment) {
      try {
        // Get consensus data from tracking
        const consensusData = this._judgmentConsensus.get(hash);
        const count = consensusData?.judgmentCount || judgmentCount || 1;

        // Record each judgment as matching consensus (anchored = verified)
        for (let i = 0; i < count; i++) {
          this.eScore.recordJudgment(true); // matchedConsensus = true for anchored
        }

        this.stats.eScoreUpdates++;
        log.debug('E-Score JUDGE updated', { judgmentCount: count });
      } catch (e) {
        log.warn('E-Score update failed', { error: e.message });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 2. CREATE PATTERN FOR COLLECTIVE MEMORY
    // ═══════════════════════════════════════════════════════════════════════
    if (this.persistence?.storePattern) {
      try {
        await this.persistence.storePattern({
          type: 'block_anchored',
          pattern: 'onchain_truth_verified',
          slot,
          hash,
          signature,
          solanaSlot,
          merkleRoot,
          judgmentCount,
          timestamp: anchoredAt || Date.now(),
          confidence: PHI_INV, // 61.8% - on-chain is high confidence
        });
        this.stats.patternsCreated++;
      } catch (e) {
        log.warn('Pattern creation failed', { error: e.message });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 3. NOTIFY COLLECTIVE DOGS
    // ═══════════════════════════════════════════════════════════════════════
    if (this.collective) {
      try {
        // Oracle: update prophecy/visualization
        if (this.collective.oracle?.observeBlock) {
          await this.collective.oracle.observeBlock({
            type: 'block_finalized',
            slot,
            hash,
            signature,
            isOnChain: true,
          });
        }

        // Analyst: record reputation data
        if (this.collective.analyst?.recordSignal) {
          await this.collective.analyst.recordSignal({
            type: 'block_anchored',
            confidence: PHI_INV,
            slot,
          });
        }

        // Guardian: log security event
        if (this.collective.guardian?.logEvent) {
          await this.collective.guardian.logEvent({
            type: 'block_verified',
            slot,
            signature: signature?.slice(0, 16),
          });
        }
      } catch (e) {
        log.warn('Collective notification failed', { error: e.message });
      }
    }

    // Clean up tracking
    this._judgmentConsensus.delete(hash);
  }

  /**
   * Handle anchor failure
   * @param {Object} event - Anchor failed event
   * @private
   */
  async _onAnchorFailed(event) {
    this.stats.anchorsFailed++;

    const { batchId, itemCount, error } = event.data || event;

    log.warn('Anchor failed', { batchId, itemCount, error });

    // Notify Guardian of security/integrity event
    if (this.collective?.guardian?.logEvent) {
      try {
        await this.collective.guardian.logEvent({
          type: 'anchor_failed',
          batchId,
          error,
          severity: 'medium',
        });
      } catch (e) { /* non-blocking */ }
    }
  }

  // ===========================================================================
  // BURN INTEGRATION
  // ===========================================================================

  /**
   * Wire burn verification to E-Score BURN dimension
   *
   * @param {Object} burnVerifier - BurnVerifier instance
   */
  wireBurnVerifier(burnVerifier) {
    if (!burnVerifier) return;

    this.burnVerifier = burnVerifier;

    // Subscribe to burn verification
    if (burnVerifier.onVerify) {
      burnVerifier.onVerify = (result) => {
        if (result.verified && result.amount && this.eScore?.recordBurn) {
          this.eScore.recordBurn(result.amount, result.signature);
          log.debug('Burn recorded to E-Score', { amount: result.amount });
        }
      };
    }

    log.info('Burn verifier wired to E-Score');
  }

  // ===========================================================================
  // STATS
  // ===========================================================================

  /**
   * Get bridge statistics
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      isRunning: this._subscriptions.length > 0,
      hasEScore: !!this.eScore,
      hasCollective: !!this.collective,
      hasPersistence: !!this.persistence,
      pendingBlocks: this._judgmentConsensus.size,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create and start a blockchain bridge
 *
 * @param {Object} options - Bridge options
 * @returns {BlockchainBridge} Running bridge instance
 */
export function createBlockchainBridge(options) {
  const bridge = new BlockchainBridge(options);
  bridge.start();
  return bridge;
}

export default BlockchainBridge;

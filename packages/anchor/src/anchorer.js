/**
 * @cynic/anchor - Solana Anchorer
 *
 * Anchors merkle roots to Solana for immutable truth.
 *
 * "Onchain is truth" - κυνικός
 *
 * @module @cynic/anchor/anchorer
 */

'use strict';

import { createHash } from 'crypto';
import {
  AnchorStatus,
  ANCHOR_CONSTANTS,
  DEFAULT_CONFIG,
  CYNIC_PROGRAM,
} from './constants.js';
import { CynicWallet } from './wallet.js';
import { CynicProgramClient } from './program-client.js';

/**
 * Result of an anchor operation
 * @typedef {Object} AnchorResult
 * @property {boolean} success - Whether anchor succeeded
 * @property {string} [signature] - Solana transaction signature
 * @property {string} [slot] - Solana slot number
 * @property {string} merkleRoot - The anchored merkle root
 * @property {number} timestamp - Unix timestamp
 * @property {string} [error] - Error message if failed
 */

/**
 * Anchor record for tracking
 * @typedef {Object} AnchorRecord
 * @property {string} id - Unique anchor ID
 * @property {string} merkleRoot - 32-byte hex merkle root
 * @property {AnchorStatus} status - Current status
 * @property {string} [signature] - Solana tx signature
 * @property {number} [slot] - Solana slot
 * @property {number} createdAt - Creation timestamp
 * @property {number} [anchoredAt] - Anchor timestamp
 * @property {number} retryCount - Number of retries
 * @property {string[]} itemIds - IDs of items in this anchor
 */

/**
 * Solana Anchorer
 *
 * Handles anchoring merkle roots to Solana blockchain.
 * Uses memo program for storing roots with minimal cost.
 */
export class SolanaAnchorer {
  /**
   * @param {Object} config - Configuration
   * @param {string} [config.cluster] - Solana cluster URL
   * @param {Object} [config.wallet] - Wallet for signing (keypair or adapter)
   * @param {Function} [config.onAnchor] - Callback when anchor completes
   * @param {Function} [config.onError] - Callback on anchor error
   * @param {boolean} [config.useAnchorProgram] - Use real Anchor program (default: true)
   * @param {string} [config.programId] - CYNIC program ID
   */
  constructor(config = {}) {
    this.cluster = config.cluster || DEFAULT_CONFIG.cluster;
    this.wallet = config.wallet;
    this.onAnchor = config.onAnchor;
    this.onError = config.onError;
    this.useAnchorProgram =
      config.useAnchorProgram !== undefined
        ? config.useAnchorProgram
        : DEFAULT_CONFIG.useAnchorProgram;
    this.programId = config.programId || CYNIC_PROGRAM.PROGRAM_ID;

    // Program client (lazy initialized)
    this._programClient = null;

    // Track anchors
    this.anchors = new Map();
    this.pendingCount = 0;

    // Block height tracking
    this.blockHeight = 0;

    // Stats
    this.stats = {
      totalAnchored: 0,
      totalFailed: 0,
      totalItems: 0,
      lastAnchorTime: null,
      lastSignature: null,
    };
  }

  /**
   * Get or create the program client
   * @returns {CynicProgramClient}
   * @private
   */
  _getProgramClient() {
    if (!this._programClient && this.wallet) {
      this._programClient = new CynicProgramClient({
        cluster: this.cluster,
        wallet: this.wallet,
        programId: this.programId,
      });
    }
    return this._programClient;
  }

  /**
   * Set the current PoJ block height
   * @param {number} height - Block height
   */
  setBlockHeight(height) {
    this.blockHeight = height;
  }

  /**
   * Generate anchor ID
   * @param {string} merkleRoot - Merkle root to anchor
   * @returns {string} Unique anchor ID
   */
  generateAnchorId(merkleRoot) {
    const timestamp = Date.now();
    const hash = createHash('sha256')
      .update(`${merkleRoot}:${timestamp}`)
      .digest('hex')
      .slice(0, 16);
    return `anc_${hash}`;
  }

  /**
   * Create anchor record
   * @param {string} merkleRoot - 32-byte hex merkle root
   * @param {string[]} itemIds - IDs of items included
   * @returns {AnchorRecord}
   */
  createAnchorRecord(merkleRoot, itemIds = []) {
    const id = this.generateAnchorId(merkleRoot);
    const record = {
      id,
      merkleRoot,
      status: AnchorStatus.QUEUED,
      signature: null,
      slot: null,
      createdAt: Date.now(),
      anchoredAt: null,
      retryCount: 0,
      itemIds,
    };

    this.anchors.set(id, record);
    this.pendingCount++;

    return record;
  }

  /**
   * Anchor a merkle root to Solana
   *
   * Uses the Memo program to store the root on-chain.
   * This is the most cost-effective way to anchor data.
   *
   * @param {string} merkleRoot - 32-byte hex merkle root
   * @param {string[]} [itemIds] - Optional IDs of items included
   * @returns {Promise<AnchorResult>}
   */
  async anchor(merkleRoot, itemIds = []) {
    // Validate merkle root format
    if (!/^[a-f0-9]{64}$/i.test(merkleRoot)) {
      return {
        success: false,
        merkleRoot,
        timestamp: Date.now(),
        error: 'Invalid merkle root format (expected 64 hex chars)',
      };
    }

    // Create record
    const record = this.createAnchorRecord(merkleRoot, itemIds);

    // Check wallet
    if (!this.wallet) {
      // No wallet - simulate for testing/development
      return this._simulateAnchor(record);
    }

    try {
      // Real Solana anchoring
      const result = await this._sendAnchorTransaction(record);

      // Update record
      record.status = AnchorStatus.ANCHORED;
      record.signature = result.signature;
      record.slot = result.slot;
      record.anchoredAt = Date.now();

      // Update stats
      this.stats.totalAnchored++;
      this.stats.totalItems += itemIds.length;
      this.stats.lastAnchorTime = record.anchoredAt;
      this.stats.lastSignature = result.signature;
      this.pendingCount--;

      // Callback
      if (this.onAnchor) {
        this.onAnchor(record);
      }

      return {
        success: true,
        signature: result.signature,
        slot: result.slot,
        merkleRoot,
        timestamp: record.anchoredAt,
      };
    } catch (error) {
      // Handle failure
      record.status = AnchorStatus.FAILED;
      record.retryCount++;
      this.stats.totalFailed++;
      this.pendingCount--;

      if (this.onError) {
        this.onError(record, error);
      }

      return {
        success: false,
        merkleRoot,
        timestamp: Date.now(),
        error: error.message,
      };
    }
  }

  /**
   * Send anchor transaction to Solana
   * @param {AnchorRecord} record - Anchor record
   * @returns {Promise<{signature: string, slot: number}>}
   * @private
   */
  async _sendAnchorTransaction(record) {
    // Use Anchor program if enabled
    if (this.useAnchorProgram) {
      return this._sendAnchorProgramTransaction(record);
    }

    // Fallback to Memo program (legacy)
    return this._sendMemoTransaction(record);
  }

  /**
   * Send anchor via CYNIC Anchor program
   * @param {AnchorRecord} record - Anchor record
   * @returns {Promise<{signature: string, slot: number, rootPda: string}>}
   * @private
   */
  async _sendAnchorProgramTransaction(record) {
    const client = this._getProgramClient();
    if (!client) {
      throw new Error('Wallet required for Anchor program anchoring');
    }

    // Anchor the root via the program
    const result = await client.anchorRoot(
      record.merkleRoot,
      record.itemIds.length,
      this.blockHeight
    );

    return {
      signature: result.signature,
      slot: result.slot,
      rootPda: result.rootPda,
    };
  }

  /**
   * Send anchor via Memo program (legacy fallback)
   * @param {AnchorRecord} record - Anchor record
   * @returns {Promise<{signature: string, slot: number}>}
   * @private
   */
  async _sendMemoTransaction(record) {
    // Lazy import @solana/web3.js (may not be installed)
    let Connection, Transaction, TransactionInstruction, PublicKey, Keypair;
    try {
      const solanaWeb3 = await import('@solana/web3.js');
      Connection = solanaWeb3.Connection;
      Transaction = solanaWeb3.Transaction;
      TransactionInstruction = solanaWeb3.TransactionInstruction;
      PublicKey = solanaWeb3.PublicKey;
      Keypair = solanaWeb3.Keypair;
    } catch (error) {
      throw new Error(
        '@solana/web3.js is required for real anchoring. ' +
          'Install with: npm install @solana/web3.js'
      );
    }

    // Build memo data
    const memo = `${ANCHOR_CONSTANTS.MEMO_PREFIX}${record.merkleRoot}`;
    const memoData = Buffer.from(memo, 'utf-8');

    // Memo program ID
    const MEMO_PROGRAM_ID = new PublicKey(
      'MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr'
    );

    // Create connection
    const connection = new Connection(this.cluster, 'confirmed');

    // Get payer keypair from wallet
    let payer;
    if (this.wallet instanceof CynicWallet) {
      if (this.wallet._secretKey) {
        payer = Keypair.fromSecretKey(this.wallet._secretKey);
      } else if (this.wallet._keypair) {
        payer = this.wallet._keypair;
      } else {
        throw new Error('Wallet does not have signing capability');
      }
    } else if (this.wallet && this.wallet.secretKey) {
      // Raw keypair object
      payer = Keypair.fromSecretKey(this.wallet.secretKey);
    } else if (this.wallet && typeof this.wallet.signTransaction === 'function') {
      // Wallet adapter - handle differently
      payer = null;
    } else {
      throw new Error('Invalid wallet configuration');
    }

    // Create memo instruction
    const memoInstruction = new TransactionInstruction({
      keys: [],
      programId: MEMO_PROGRAM_ID,
      data: memoData,
    });

    // Create and sign transaction
    const transaction = new Transaction().add(memoInstruction);

    // Get recent blockhash
    const { blockhash, lastValidBlockHeight } =
      await connection.getLatestBlockhash('confirmed');
    transaction.recentBlockhash = blockhash;
    transaction.lastValidBlockHeight = lastValidBlockHeight;

    let signature;
    let slot;

    if (payer) {
      // Sign with keypair
      transaction.feePayer = payer.publicKey;
      transaction.sign(payer);

      // Send and confirm
      signature = await connection.sendRawTransaction(transaction.serialize(), {
        skipPreflight: false,
        preflightCommitment: 'confirmed',
      });

      // Wait for confirmation
      const confirmation = await connection.confirmTransaction(
        {
          signature,
          blockhash,
          lastValidBlockHeight,
        },
        'confirmed'
      );

      if (confirmation.value.err) {
        throw new Error(`Transaction failed: ${JSON.stringify(confirmation.value.err)}`);
      }

      // Get slot from transaction
      const txInfo = await connection.getTransaction(signature, {
        commitment: 'confirmed',
      });
      slot = txInfo?.slot || 0;
    } else {
      // Use wallet adapter
      transaction.feePayer = new PublicKey(this.wallet.publicKey);
      const signedTx = await this.wallet.signTransaction(transaction);

      signature = await connection.sendRawTransaction(signedTx.serialize(), {
        skipPreflight: false,
        preflightCommitment: 'confirmed',
      });

      const confirmation = await connection.confirmTransaction(
        {
          signature,
          blockhash,
          lastValidBlockHeight,
        },
        'confirmed'
      );

      if (confirmation.value.err) {
        throw new Error(`Transaction failed: ${JSON.stringify(confirmation.value.err)}`);
      }

      const txInfo = await connection.getTransaction(signature, {
        commitment: 'confirmed',
      });
      slot = txInfo?.slot || 0;
    }

    return { signature, slot };
  }

  /**
   * Simulate anchor for testing/development
   * @param {AnchorRecord} record - Anchor record
   * @returns {AnchorResult}
   * @private
   */
  _simulateAnchor(record) {
    // Generate fake signature (base58-like)
    const fakeSignature = createHash('sha256')
      .update(`sim:${record.merkleRoot}:${Date.now()}`)
      .digest('base64')
      .replace(/[+/=]/g, '')
      .slice(0, 88);

    const fakeSlot = Math.floor(Date.now() / 400); // ~Solana slot rate

    // Update record
    record.status = AnchorStatus.ANCHORED;
    record.signature = `sim_${fakeSignature}`;
    record.slot = fakeSlot;
    record.anchoredAt = Date.now();

    // Update stats
    this.stats.totalAnchored++;
    this.stats.totalItems += record.itemIds.length;
    this.stats.lastAnchorTime = record.anchoredAt;
    this.stats.lastSignature = record.signature;
    this.pendingCount--;

    // Callback
    if (this.onAnchor) {
      this.onAnchor(record);
    }

    return {
      success: true,
      signature: record.signature,
      slot: record.slot,
      merkleRoot: record.merkleRoot,
      timestamp: record.anchoredAt,
      simulated: true,
    };
  }

  /**
   * Retry a failed anchor
   * @param {string} anchorId - Anchor ID to retry
   * @returns {Promise<AnchorResult>}
   */
  async retry(anchorId) {
    const record = this.anchors.get(anchorId);
    if (!record) {
      throw new Error(`Anchor not found: ${anchorId}`);
    }

    if (record.status !== AnchorStatus.FAILED) {
      throw new Error(`Cannot retry anchor in status: ${record.status}`);
    }

    if (record.retryCount >= ANCHOR_CONSTANTS.MAX_RETRY_ATTEMPTS) {
      throw new Error(
        `Max retries (${ANCHOR_CONSTANTS.MAX_RETRY_ATTEMPTS}) exceeded`
      );
    }

    // Reset and retry
    record.status = AnchorStatus.QUEUED;
    this.pendingCount++;

    return this.anchor(record.merkleRoot, record.itemIds);
  }

  /**
   * Get anchor by ID
   * @param {string} anchorId - Anchor ID
   * @returns {AnchorRecord|undefined}
   */
  getAnchor(anchorId) {
    return this.anchors.get(anchorId);
  }

  /**
   * Get anchor by signature
   * @param {string} signature - Solana tx signature
   * @returns {AnchorRecord|undefined}
   */
  getAnchorBySignature(signature) {
    for (const record of this.anchors.values()) {
      if (record.signature === signature) {
        return record;
      }
    }
    return undefined;
  }

  /**
   * Get all pending anchors
   * @returns {AnchorRecord[]}
   */
  getPendingAnchors() {
    return Array.from(this.anchors.values()).filter(
      (r) => r.status === AnchorStatus.QUEUED
    );
  }

  /**
   * Get all failed anchors
   * @returns {AnchorRecord[]}
   */
  getFailedAnchors() {
    return Array.from(this.anchors.values()).filter(
      (r) => r.status === AnchorStatus.FAILED
    );
  }

  /**
   * Get anchorer statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      pendingCount: this.pendingCount,
      totalRecords: this.anchors.size,
      cluster: this.cluster,
      hasWallet: !!this.wallet,
    };
  }

  /**
   * Verify an anchor exists on Solana
   * @param {string} signatureOrMerkleRoot - Solana tx signature or merkle root
   * @param {string} [expectedMerkleRoot] - Expected merkle root (if first arg is signature)
   * @returns {Promise<{verified: boolean, slot?: number, memo?: string, entry?: Object, error?: string}>}
   */
  async verifyAnchor(signatureOrMerkleRoot, expectedMerkleRoot = null) {
    // Skip simulation signatures
    if (signatureOrMerkleRoot.startsWith('sim_')) {
      return { verified: true, simulated: true };
    }

    // If using Anchor program and we have a 64-char hex string, verify via program
    if (this.useAnchorProgram && /^[a-f0-9]{64}$/i.test(signatureOrMerkleRoot)) {
      return this._verifyViaProgram(signatureOrMerkleRoot);
    }

    // Verify via transaction signature (legacy/memo method)
    return this._verifyViaTransaction(signatureOrMerkleRoot, expectedMerkleRoot);
  }

  /**
   * Verify anchor via CYNIC Anchor program (on-chain account lookup)
   * @param {string} merkleRoot - Merkle root to verify
   * @returns {Promise<{verified: boolean, entry?: Object, error?: string}>}
   * @private
   */
  async _verifyViaProgram(merkleRoot) {
    const client = this._getProgramClient();
    if (!client) {
      // Fallback to creating a readonly client
      try {
        const { Connection, PublicKey } = await import('@solana/web3.js');
        const connection = new Connection(this.cluster, 'confirmed');

        // Calculate root PDA
        const rootBytes = Buffer.from(merkleRoot, 'hex');
        const [rootPda] = PublicKey.findProgramAddressSync(
          [Buffer.from(CYNIC_PROGRAM.ROOT_SEED), rootBytes],
          new PublicKey(this.programId)
        );

        // Fetch account data
        const accountInfo = await connection.getAccountInfo(rootPda);
        if (!accountInfo) {
          return { verified: false, error: 'Root not found on-chain' };
        }

        return {
          verified: true,
          slot: null,
          message: 'Root exists on-chain (account found)',
        };
      } catch (error) {
        return { verified: false, error: error.message };
      }
    }

    // Use program client for full verification
    const result = await client.verifyRoot(merkleRoot);
    return result;
  }

  /**
   * Verify anchor via transaction signature (legacy/memo)
   * @param {string} signature - Solana tx signature
   * @param {string} [expectedMerkleRoot] - Expected merkle root
   * @returns {Promise<{verified: boolean, slot?: number, memo?: string, error?: string}>}
   * @private
   */
  async _verifyViaTransaction(signature, expectedMerkleRoot = null) {
    // Lazy import @solana/web3.js
    let Connection;
    try {
      const solanaWeb3 = await import('@solana/web3.js');
      Connection = solanaWeb3.Connection;
    } catch (error) {
      return {
        verified: false,
        error: '@solana/web3.js is required for verification',
      };
    }

    try {
      const connection = new Connection(this.cluster, 'confirmed');

      // Get transaction
      const tx = await connection.getTransaction(signature, {
        commitment: 'confirmed',
        maxSupportedTransactionVersion: 0,
      });

      if (!tx) {
        return { verified: false, error: 'Transaction not found' };
      }

      // Extract memo from transaction logs
      let memoContent = null;
      if (tx.meta?.logMessages) {
        for (const log of tx.meta.logMessages) {
          // Memo program logs the memo content
          if (log.includes('Program log: Memo')) {
            const match = log.match(/Memo \(len \d+\): "(.+)"/);
            if (match) {
              memoContent = match[1];
            }
          }
          // Also check for raw memo
          if (log.includes(ANCHOR_CONSTANTS.MEMO_PREFIX)) {
            memoContent = log;
          }
          // Check for CYNIC program anchor event
          if (log.includes('Root anchored')) {
            memoContent = log;
          }
        }
      }

      // Verify merkle root if provided
      if (expectedMerkleRoot && memoContent) {
        if (!memoContent.includes(expectedMerkleRoot)) {
          return {
            verified: false,
            slot: tx.slot,
            memo: memoContent,
            error: 'Merkle root mismatch',
          };
        }
      }

      return {
        verified: true,
        slot: tx.slot,
        memo: memoContent,
        blockTime: tx.blockTime,
      };
    } catch (error) {
      return {
        verified: false,
        error: error.message,
      };
    }
  }

  /**
   * Get the program client for direct program interaction
   * @returns {CynicProgramClient|null}
   */
  getProgramClient() {
    return this._getProgramClient();
  }

  /**
   * Export anchorer state
   * @returns {Object}
   */
  export() {
    return {
      anchors: Array.from(this.anchors.entries()),
      stats: { ...this.stats },
      cluster: this.cluster,
    };
  }

  /**
   * Import anchorer state
   * @param {Object} state - Exported state
   */
  import(state) {
    if (state.anchors) {
      this.anchors = new Map(state.anchors);
      this.pendingCount = Array.from(this.anchors.values()).filter(
        (r) => r.status === AnchorStatus.QUEUED
      ).length;
    }
    if (state.stats) {
      this.stats = { ...this.stats, ...state.stats };
    }
    if (state.cluster) {
      this.cluster = state.cluster;
    }
  }
}

/**
 * Create a Solana anchorer instance
 * @param {Object} [config] - Configuration
 * @returns {SolanaAnchorer}
 */
export function createAnchorer(config = {}) {
  return new SolanaAnchorer(config);
}

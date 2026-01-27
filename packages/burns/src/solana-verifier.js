/**
 * @cynic/burns - Solana On-Chain Verifier
 *
 * Direct Solana blockchain verification of burn transactions.
 *
 * "Onchain is truth" - κυνικός
 *
 * @module @cynic/burns/solana-verifier
 */

'use strict';

import { Connection } from '@solana/web3.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Known burn addresses
 */
export const BURN_ADDRESSES = {
  // System burn address (all 1s)
  SYSTEM: '1111111111111111111111111111111111',
  // Common burn address (all 1s, 32 bytes base58)
  COMMON: '1nc1nerator11111111111111111111111111111111',
  // Another common burn pattern
  DEAD: 'DeaDDeaDDeaDDeaDDeaDDeaDDeaDDeaDDeaDDeaDDe',
};

/**
 * SPL Token Program ID
 */
const TOKEN_PROGRAM_ID = 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';

/**
 * Token 2022 Program ID
 */
const TOKEN_2022_PROGRAM_ID = 'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb';

/**
 * Solana cluster URLs
 */
export const SolanaCluster = {
  MAINNET: 'https://api.mainnet-beta.solana.com',
  DEVNET: 'https://api.devnet.solana.com',
  TESTNET: 'https://api.testnet.solana.com',
  // Helius RPC (faster, no rate-limits)
  HELIUS_DEVNET: process.env.HELIUS_API_KEY
    ? `https://devnet.helius-rpc.com/?api-key=${process.env.HELIUS_API_KEY}`
    : null,
  HELIUS_MAINNET: process.env.HELIUS_API_KEY
    ? `https://mainnet.helius-rpc.com/?api-key=${process.env.HELIUS_API_KEY}`
    : null,
};

// ═══════════════════════════════════════════════════════════════════════════════
// Solana Burn Verifier
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Solana on-chain burn verifier
 *
 * Verifies burn transactions directly on Solana blockchain.
 */
export class SolanaBurnVerifier {
  /**
   * @param {Object} options
   * @param {string} [options.cluster] - Solana cluster URL
   * @param {string} [options.commitment] - Commitment level
   */
  constructor(options = {}) {
    this.cluster = options.cluster || SolanaCluster.MAINNET;
    this.commitment = options.commitment || 'confirmed';
    this.connection = new Connection(this.cluster, this.commitment);

    // Cache verified burns (tx signature -> result)
    this.cache = new Map();
    this.cacheTtl = 24 * 60 * 60 * 1000; // 24 hours

    // Stats
    this.stats = {
      totalVerified: 0,
      totalFailed: 0,
      totalRequests: 0,
      cacheHits: 0,
    };
  }

  /**
   * Verify a burn transaction on-chain
   *
   * @param {string} signature - Transaction signature
   * @param {Object} [options] - Verification options
   * @param {number} [options.minAmount] - Minimum burn amount
   * @param {string} [options.expectedBurner] - Expected burner address
   * @param {boolean} [options.skipCache] - Skip cache lookup
   * @returns {Promise<Object>} Verification result
   */
  async verify(signature, options = {}) {
    this.stats.totalRequests++;

    // Check cache
    if (!options.skipCache) {
      const cached = this._getCached(signature);
      if (cached) {
        this.stats.cacheHits++;
        return { ...cached, cached: true };
      }
    }

    try {
      // Fetch transaction from Solana
      const tx = await this.connection.getParsedTransaction(signature, {
        maxSupportedTransactionVersion: 0,
        commitment: this.commitment,
      });

      if (!tx) {
        this.stats.totalFailed++;
        return {
          verified: false,
          txSignature: signature,
          error: 'Transaction not found on Solana',
          cached: false,
        };
      }

      // Check if transaction was successful
      if (tx.meta?.err) {
        this.stats.totalFailed++;
        return {
          verified: false,
          txSignature: signature,
          error: `Transaction failed: ${JSON.stringify(tx.meta.err)}`,
          cached: false,
        };
      }

      // Analyze transaction for burns
      const burnInfo = this._analyzeBurn(tx);

      if (!burnInfo.isBurn) {
        this.stats.totalFailed++;
        return {
          verified: false,
          txSignature: signature,
          error: burnInfo.reason || 'Not a burn transaction',
          cached: false,
        };
      }

      // Validate against options
      if (options.minAmount && burnInfo.amount < options.minAmount) {
        this.stats.totalFailed++;
        return {
          verified: false,
          txSignature: signature,
          amount: burnInfo.amount,
          error: `Burn amount ${burnInfo.amount} below minimum ${options.minAmount}`,
          cached: false,
        };
      }

      if (options.expectedBurner && burnInfo.burner !== options.expectedBurner) {
        this.stats.totalFailed++;
        return {
          verified: false,
          txSignature: signature,
          error: `Burner mismatch: expected ${options.expectedBurner}, got ${burnInfo.burner}`,
          cached: false,
        };
      }

      // Build result
      const result = {
        verified: true,
        txSignature: signature,
        amount: burnInfo.amount,
        token: burnInfo.token || null,
        burner: burnInfo.burner,
        burnAddress: burnInfo.burnAddress,
        burnType: burnInfo.type,
        timestamp: tx.blockTime ? tx.blockTime * 1000 : Date.now(),
        slot: tx.slot,
        cached: false,
      };

      // Cache result
      this._setCached(signature, result);
      this.stats.totalVerified++;

      return result;
    } catch (error) {
      this.stats.totalFailed++;
      return {
        verified: false,
        txSignature: signature,
        error: error.message,
        cached: false,
      };
    }
  }

  /**
   * Analyze transaction for burn patterns
   *
   * @param {Object} tx - Parsed transaction
   * @returns {Object} Burn analysis result
   * @private
   */
  _analyzeBurn(tx) {
    const instructions = tx.transaction?.message?.instructions || [];
    const innerInstructions = tx.meta?.innerInstructions || [];

    // Collect all instructions (including inner)
    const allInstructions = [...instructions];
    for (const inner of innerInstructions) {
      allInstructions.push(...(inner.instructions || []));
    }

    // Check for SOL burn (transfer to burn address)
    for (const ix of allInstructions) {
      if (ix.program === 'system' && ix.parsed?.type === 'transfer') {
        const destination = ix.parsed.info?.destination;
        const source = ix.parsed.info?.source;
        const amount = ix.parsed.info?.lamports;

        if (this._isBurnAddress(destination)) {
          return {
            isBurn: true,
            type: 'SOL_BURN',
            amount: amount,
            burner: source,
            burnAddress: destination,
          };
        }
      }
    }

    // Check for SPL Token burn
    for (const ix of allInstructions) {
      const programId = ix.programId?.toString?.() || ix.program;

      if (programId === TOKEN_PROGRAM_ID || programId === TOKEN_2022_PROGRAM_ID) {
        const parsed = ix.parsed;

        // Direct burn instruction
        if (parsed?.type === 'burn' || parsed?.type === 'burnChecked') {
          return {
            isBurn: true,
            type: 'TOKEN_BURN',
            amount: parseInt(parsed.info?.amount || parsed.info?.tokenAmount?.amount || 0),
            token: parsed.info?.mint,
            burner: parsed.info?.authority || parsed.info?.owner,
            burnAddress: null,
          };
        }

        // Transfer to burn address
        if (parsed?.type === 'transfer' || parsed?.type === 'transferChecked') {
          const destination = parsed.info?.destination;

          if (this._isBurnAddress(destination)) {
            return {
              isBurn: true,
              type: 'TOKEN_TRANSFER_BURN',
              amount: parseInt(parsed.info?.amount || parsed.info?.tokenAmount?.amount || 0),
              token: parsed.info?.mint,
              burner: parsed.info?.authority || parsed.info?.source,
              burnAddress: destination,
            };
          }
        }
      }
    }

    // Check for close account (burns remaining tokens)
    for (const ix of allInstructions) {
      const programId = ix.programId?.toString?.() || ix.program;

      if (programId === TOKEN_PROGRAM_ID || programId === TOKEN_2022_PROGRAM_ID) {
        if (ix.parsed?.type === 'closeAccount') {
          return {
            isBurn: true,
            type: 'ACCOUNT_CLOSE',
            amount: 0,
            token: null,
            burner: ix.parsed.info?.owner,
            burnAddress: null,
            reason: 'Account closed (remaining balance burned)',
          };
        }
      }
    }

    // Check pre/post balances for burn patterns
    const preBalances = tx.meta?.preBalances || [];
    const postBalances = tx.meta?.postBalances || [];
    const accountKeys = tx.transaction?.message?.accountKeys || [];

    for (let i = 0; i < accountKeys.length; i++) {
      const account = accountKeys[i]?.pubkey?.toString?.() || accountKeys[i];
      const preBal = preBalances[i] || 0;
      const postBal = postBalances[i] || 0;

      // Check if balance increased to a burn address
      if (this._isBurnAddress(account) && postBal > preBal) {
        const burnAmount = postBal - preBal;
        // Find the sender
        for (let j = 0; j < accountKeys.length; j++) {
          const senderBal = (preBalances[j] || 0) - (postBalances[j] || 0);
          if (senderBal >= burnAmount && j !== i) {
            return {
              isBurn: true,
              type: 'SOL_BURN_INFERRED',
              amount: burnAmount,
              burner: accountKeys[j]?.pubkey?.toString?.() || accountKeys[j],
              burnAddress: account,
            };
          }
        }
      }
    }

    return {
      isBurn: false,
      reason: 'No burn pattern detected',
    };
  }

  /**
   * Check if address is a known burn address
   *
   * @param {string} address - Address to check
   * @returns {boolean}
   * @private
   */
  _isBurnAddress(address) {
    if (!address) return false;

    const addr = address.toString?.() || address;

    // Check known burn addresses
    if (Object.values(BURN_ADDRESSES).includes(addr)) {
      return true;
    }

    // Check for all-ones pattern (system burn)
    if (/^1{32,44}$/.test(addr)) {
      return true;
    }

    // Check for common burn patterns
    const lowerAddr = addr.toLowerCase();
    if (
      lowerAddr.includes('burn') ||
      lowerAddr.includes('dead') ||
      lowerAddr.includes('null') ||
      lowerAddr.startsWith('1111111111')
    ) {
      return true;
    }

    return false;
  }

  /**
   * Get cached result
   * @private
   */
  _getCached(signature) {
    const entry = this.cache.get(signature);
    if (!entry) return null;

    if (Date.now() - entry.timestamp > this.cacheTtl) {
      this.cache.delete(signature);
      return null;
    }

    return entry.data;
  }

  /**
   * Set cached result
   * @private
   */
  _setCached(signature, data) {
    this.cache.set(signature, {
      data,
      timestamp: Date.now(),
    });
  }

  /**
   * Get service stats
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      cacheSize: this.cache.size,
      cluster: this.cluster,
    };
  }

  /**
   * Clear cache
   */
  clearCache() {
    this.cache.clear();
  }

  /**
   * Export state
   */
  export() {
    return {
      cache: Array.from(this.cache.entries()),
      stats: { ...this.stats },
    };
  }

  /**
   * Import state
   */
  import(state) {
    if (state.cache) {
      this.cache = new Map(state.cache);
    }
    if (state.stats) {
      this.stats = { ...this.stats, ...state.stats };
    }
  }
}

/**
 * Create a Solana burn verifier
 * @param {Object} [options] - Options
 * @returns {SolanaBurnVerifier}
 */
export function createSolanaBurnVerifier(options = {}) {
  return new SolanaBurnVerifier(options);
}

export default {
  SolanaBurnVerifier,
  createSolanaBurnVerifier,
  SolanaCluster,
  BURN_ADDRESSES,
};

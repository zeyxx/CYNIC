/**
 * CYNIC Burns API
 *
 * Verifies burn transactions on Solana blockchain.
 *
 * "Don't extract, burn" - κυνικός
 *
 * @module @cynic/node/api/burns
 */

'use strict';

import { Connection, PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { SolanaCluster } from '@cynic/anchor';
import { PHI_INV } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Known burn addresses
 */
const BURN_ADDRESSES = {
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

// ═══════════════════════════════════════════════════════════════════════════════
// Burns API Service
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Burns verification service
 */
export class BurnsAPI {
  /**
   * @param {Object} options
   * @param {string} [options.cluster] - Solana cluster URL
   * @param {string} [options.commitment] - Commitment level
   */
  constructor(options = {}) {
    this.cluster = options.cluster || SolanaCluster.MAINNET;
    this.commitment = options.commitment || 'confirmed';
    this.connection = new Connection(this.cluster, this.commitment);

    // Cache verified burns
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
   * Verify a burn transaction
   *
   * @param {string} signature - Transaction signature
   * @returns {Promise<Object>} Verification result
   */
  async verify(signature) {
    this.stats.totalRequests++;

    // Check cache
    const cached = this._getCached(signature);
    if (cached) {
      this.stats.cacheHits++;
      return { ...cached, cached: true };
    }

    try {
      // Fetch transaction
      const tx = await this.connection.getParsedTransaction(signature, {
        maxSupportedTransactionVersion: 0,
        commitment: this.commitment,
      });

      if (!tx) {
        this.stats.totalFailed++;
        return {
          verified: false,
          txSignature: signature,
          error: 'Transaction not found',
        };
      }

      // Check if transaction was successful
      if (tx.meta?.err) {
        this.stats.totalFailed++;
        return {
          verified: false,
          txSignature: signature,
          error: `Transaction failed: ${JSON.stringify(tx.meta.err)}`,
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
      };

      // Cache result
      this._setCached(signature, result);
      this.stats.totalVerified++;

      return { ...result, cached: false };
    } catch (error) {
      this.stats.totalFailed++;
      return {
        verified: false,
        txSignature: signature,
        error: error.message,
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
          // Close account burns any remaining dust
          return {
            isBurn: true,
            type: 'ACCOUNT_CLOSE',
            amount: 0, // Remaining balance unknown without pre-state
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

      // Check if balance decreased significantly to a burn address
      if (this._isBurnAddress(account) && postBal > preBal) {
        const burnAmount = postBal - preBal;
        // Find the sender (account with decreased balance)
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
   * @param {string} signature
   * @returns {Object|null}
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
   * @param {string} signature
   * @param {Object} data
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
}

// ═══════════════════════════════════════════════════════════════════════════════
// Express Router
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Create burns API router
 *
 * @param {Object} options
 * @param {string} [options.cluster] - Solana cluster
 * @returns {Promise<Object>} Express router and service
 */
export async function createBurnsRouter(options = {}) {
  const express = await import('express');
  const router = express.default.Router();
  const service = new BurnsAPI(options);

  // GET /burns/verify/:signature
  router.get('/verify/:signature', async (req, res) => {
    try {
      const { signature } = req.params;

      // Validate signature format
      if (!signature || !/^[1-9A-HJ-NP-Za-km-z]{64,88}$/.test(signature)) {
        return res.status(400).json({
          verified: false,
          error: 'Invalid signature format',
        });
      }

      const result = await service.verify(signature);

      if (result.verified) {
        res.json(result);
      } else {
        res.status(404).json(result);
      }
    } catch (error) {
      res.status(500).json({
        verified: false,
        error: error.message,
      });
    }
  });

  // GET /burns/stats
  router.get('/stats', (req, res) => {
    res.json(service.getStats());
  });

  // POST /burns/verify (batch)
  router.post('/verify', async (req, res) => {
    try {
      const { signatures } = req.body;

      if (!Array.isArray(signatures) || signatures.length === 0) {
        return res.status(400).json({
          error: 'signatures array required',
        });
      }

      if (signatures.length > 10) {
        return res.status(400).json({
          error: 'Maximum 10 signatures per batch',
        });
      }

      const results = {};
      for (const sig of signatures) {
        results[sig] = await service.verify(sig);
      }

      res.json({ results });
    } catch (error) {
      res.status(500).json({
        error: error.message,
      });
    }
  });

  return { router, service };
}

/**
 * Setup burns routes on an Express app
 *
 * @param {Object} app - Express app
 * @param {Object} [options] - Options
 * @returns {BurnsAPI} Burns service
 */
export function setupBurnsRoutes(app, options = {}) {
  const service = new BurnsAPI(options);

  // GET /burns/verify/:signature
  app.get('/burns/verify/:signature', async (req, res) => {
    try {
      const { signature } = req.params;

      // Validate signature format
      if (!signature || !/^[1-9A-HJ-NP-Za-km-z]{64,88}$/.test(signature)) {
        return res.status(400).json({
          verified: false,
          error: 'Invalid signature format',
        });
      }

      const result = await service.verify(signature);

      if (result.verified) {
        res.json(result);
      } else {
        res.status(404).json(result);
      }
    } catch (error) {
      res.status(500).json({
        verified: false,
        error: error.message,
      });
    }
  });

  // GET /burns/stats
  app.get('/burns/stats', (req, res) => {
    res.json(service.getStats());
  });

  // POST /burns/verify (batch)
  app.post('/burns/verify', async (req, res) => {
    try {
      const { signatures } = req.body;

      if (!Array.isArray(signatures) || signatures.length === 0) {
        return res.status(400).json({
          error: 'signatures array required',
        });
      }

      if (signatures.length > 10) {
        return res.status(400).json({
          error: 'Maximum 10 signatures per batch',
        });
      }

      const results = {};
      for (const sig of signatures) {
        results[sig] = await service.verify(sig);
      }

      res.json({ results });
    } catch (error) {
      res.status(500).json({
        error: error.message,
      });
    }
  });

  return service;
}

export default BurnsAPI;

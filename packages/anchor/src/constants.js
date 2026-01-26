/**
 * @cynic/anchor - Constants
 *
 * φ-aligned constants for Solana anchoring
 *
 * "Onchain is truth" - κυνικός
 *
 * @module @cynic/anchor/constants
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * Anchor status enum
 */
export const AnchorStatus = {
  /** Not yet processed */
  EPHEMERAL: 'EPHEMERAL',
  /** Proof stored in DAG */
  PROVED: 'PROVED',
  /** Queued for anchoring */
  QUEUED: 'QUEUED',
  /** Anchored to Solana */
  ANCHORED: 'ANCHORED',
  /** Indexed in PostgreSQL */
  INDEXED: 'INDEXED',
  /** Anchor failed */
  FAILED: 'FAILED',
};

/**
 * φ-aligned anchoring constants
 */
export const ANCHOR_CONSTANTS = {
  /**
   * Anchor every φ minutes (61.8 seconds)
   * Batches transactions for efficiency
   */
  ANCHOR_INTERVAL_MS: Math.round(PHI_INV * 100 * 1000), // 61,800ms

  /**
   * Or when batch has φ² * 100 items (38 items)
   * Whichever comes first triggers anchor
   */
  ANCHOR_BATCH_SIZE: Math.floor(PHI_INV_2 * 100), // 38 items

  /**
   * Max confidence in any anchor operation
   * φ⁻¹ = 61.8% - we never claim certainty
   */
  ANCHOR_CONFIDENCE_CAP: PHI_INV, // 0.618

  /**
   * Min Solana confirmations for FINALIZED status
   * Solana finalized = 32 confirmations
   */
  MIN_CONFIRMATIONS_FINALIZED: 32,

  /**
   * Confirmations for CONFIRMED status
   */
  MIN_CONFIRMATIONS_CONFIRMED: 1,

  /**
   * Retry attempts for failed anchors
   * φ² * 10 = ~3.8 attempts
   */
  MAX_RETRY_ATTEMPTS: Math.round(PHI_INV_2 * 10), // 4

  /**
   * Retry delay base (exponential backoff)
   * φ seconds between retries
   */
  RETRY_DELAY_MS: Math.round(PHI_INV * 1000), // 618ms

  /**
   * Max items per anchor transaction
   * Keep transactions small for reliability
   */
  MAX_ITEMS_PER_TX: 100,

  /**
   * Anchor memo prefix for identification
   */
  MEMO_PREFIX: 'CYNIC:POJ:',
};

/**
 * Solana cluster endpoints
 */
export const SolanaCluster = {
  MAINNET: 'https://api.mainnet-beta.solana.com',
  DEVNET: 'https://api.devnet.solana.com',
  TESTNET: 'https://api.testnet.solana.com',
  LOCALNET: 'http://localhost:8899',
  // Helius RPC (faster, no rate-limits)
  HELIUS_DEVNET: process.env.HELIUS_API_KEY
    ? `https://devnet.helius-rpc.com/?api-key=${process.env.HELIUS_API_KEY}`
    : null,
  HELIUS_MAINNET: process.env.HELIUS_API_KEY
    ? `https://mainnet.helius-rpc.com/?api-key=${process.env.HELIUS_API_KEY}`
    : null,
};

/**
 * CYNIC Anchor Program - Deployed on Solana
 */
export const CYNIC_PROGRAM = {
  /** Program ID on devnet/mainnet */
  PROGRAM_ID: 'D2vprzEbzha6pRDs3EfFToGUvPocFoZTQG1uFkf2boRn',

  /** State PDA seed */
  STATE_SEED: 'cynic_state',

  /** Root entry PDA seed */
  ROOT_SEED: 'root',

  /** Max validators (F(8)) */
  MAX_VALIDATORS: 21,

  /** Max roots to store (F(14)) */
  MAX_ROOTS: 377,
};

/**
 * Default configuration
 */
export const DEFAULT_CONFIG = {
  cluster: SolanaCluster.DEVNET,
  autoAnchor: true,
  batchSize: ANCHOR_CONSTANTS.ANCHOR_BATCH_SIZE,
  intervalMs: ANCHOR_CONSTANTS.ANCHOR_INTERVAL_MS,
  /** Use real Anchor program instead of Memo */
  useAnchorProgram: true,
  programId: CYNIC_PROGRAM.PROGRAM_ID,
};

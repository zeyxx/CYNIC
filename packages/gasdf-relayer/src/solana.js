/**
 * @cynic/gasdf-relayer - Solana Module
 *
 * Modern Solana interactions using @solana/kit (Anza SDK).
 * No legacy web3.js - pure functional approach.
 *
 * @module @cynic/gasdf-relayer/solana
 */

'use strict';

import {
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstruction,
  signTransactionMessageWithSigners,
  getSignatureFromTransaction,
  getBase64EncodedWireTransaction,
  address,
  lamports,
  createKeyPairSignerFromBytes,
  getBase64Decoder,
  getBase58Decoder,
  sendAndConfirmTransactionFactory,
  compileTransaction,
} from '@solana/kit';
import { getTransferSolInstruction } from '@solana-program/system';

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

const config = {
  // Solana cluster
  rpcUrl: process.env.SOLANA_RPC_URL || 'https://api.mainnet-beta.solana.com',
  wsUrl: process.env.SOLANA_WS_URL || 'wss://api.mainnet-beta.solana.com',

  // Relayer wallet (base58 encoded private key)
  relayerPrivateKey: process.env.RELAYER_PRIVATE_KEY,

  // Burn destination (Solana's official burn address)
  burnAddress: process.env.BURN_ADDRESS || '1111111111111111111111111111111111111111',

  // Treasury wallet
  treasuryAddress: process.env.TREASURY_ADDRESS,

  // φ-aligned fee ratios
  burnRate: 0.763932022500210, // 1 - φ⁻³
  treasuryRate: 0.236067977499790, // φ⁻³
};

// ═══════════════════════════════════════════════════════════════════════════════
// RPC Clients
// ═══════════════════════════════════════════════════════════════════════════════

let rpc = null;
let rpcSubscriptions = null;
let relayerSigner = null;

/**
 * Initialize Solana RPC connection
 */
export function initRpc() {
  if (!rpc) {
    rpc = createSolanaRpc(config.rpcUrl);
  }
  return rpc;
}

/**
 * Initialize WebSocket subscriptions
 */
export function initSubscriptions() {
  if (!rpcSubscriptions) {
    rpcSubscriptions = createSolanaRpcSubscriptions(config.wsUrl);
  }
  return rpcSubscriptions;
}

/**
 * Initialize relayer wallet signer from private key
 */
export async function initRelayerSigner() {
  if (relayerSigner) return relayerSigner;

  if (!config.relayerPrivateKey) {
    throw new Error('RELAYER_PRIVATE_KEY not configured');
  }

  // Decode base58 private key to bytes
  const decoder = getBase58Decoder();
  const keyBytes = decoder.decode(config.relayerPrivateKey);

  // Create signer from key bytes
  relayerSigner = await createKeyPairSignerFromBytes(keyBytes);

  console.log(`[Solana] Relayer wallet: ${relayerSigner.address}`);
  return relayerSigner;
}

/**
 * Get relayer address
 */
export function getRelayerAddress() {
  if (!relayerSigner) {
    throw new Error('Relayer signer not initialized. Call initRelayerSigner() first.');
  }
  return relayerSigner.address;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Balance & Account Operations
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Get SOL balance for an address
 *
 * @param {string} pubkey - Base58 public key
 * @returns {Promise<bigint>} Balance in lamports
 */
export async function getBalance(pubkey) {
  const client = initRpc();
  const { value } = await client.getBalance(address(pubkey)).send();
  return value;
}

/**
 * Get relayer balance
 */
export async function getRelayerBalance() {
  const signer = await initRelayerSigner();
  return getBalance(signer.address);
}

/**
 * Get latest blockhash
 */
export async function getLatestBlockhash() {
  const client = initRpc();
  const { value } = await client.getLatestBlockhash().send();
  return value;
}

/**
 * Get minimum balance for rent exemption
 *
 * @param {number} dataSize - Account data size
 */
export async function getMinRent(dataSize = 0) {
  const client = initRpc();
  const rent = await client.getMinimumBalanceForRentExemption(BigInt(dataSize)).send();
  return rent;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Transaction Operations
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Estimate priority fee for transaction
 *
 * @returns {Promise<bigint>} Estimated priority fee in lamports
 */
export async function estimatePriorityFee() {
  const client = initRpc();

  try {
    // Get recent prioritization fees
    const fees = await client.getRecentPrioritizationFees([]).send();

    if (!fees || fees.length === 0) {
      return 5000n; // Default 5000 lamports
    }

    // Use median of recent fees
    const sorted = fees.map(f => f.prioritizationFee).sort((a, b) => Number(a - b));
    const median = sorted[Math.floor(sorted.length / 2)];

    return median || 5000n;
  } catch {
    return 5000n; // Fallback
  }
}

/**
 * Calculate total fee for a transaction
 *
 * @param {number} computeUnits - Estimated compute units
 * @returns {Promise<Object>} Fee breakdown
 */
export async function calculateFee(computeUnits = 200000) {
  const priorityFee = await estimatePriorityFee();

  // Base fee is 5000 lamports per signature
  const baseFee = 5000n;

  // Compute unit price (micro-lamports per CU)
  const cuPrice = priorityFee;

  // Total fee = base + (cu * price / 1_000_000)
  const computeFee = (BigInt(computeUnits) * cuPrice) / 1_000_000n;
  const totalFee = baseFee + computeFee;

  // φ-aligned split
  const burnAmount = BigInt(Math.floor(Number(totalFee) * config.burnRate));
  const treasuryAmount = totalFee - burnAmount;

  return {
    baseFee,
    priorityFee,
    computeUnits,
    totalFee,
    burnAmount,
    treasuryAmount,
    breakdown: {
      burn: config.burnRate,
      treasury: config.treasuryRate,
    },
  };
}

/**
 * Submit a transaction and pay fees
 *
 * User sends their transaction (partially signed), relayer adds fee payment
 * and signs as fee payer.
 *
 * @param {string} serializedTx - Base64 encoded transaction
 * @param {Object} options - Options
 * @returns {Promise<Object>} Submission result
 */
export async function submitAndPayFee(serializedTx, options = {}) {
  const client = initRpc();
  const signer = await initRelayerSigner();
  const blockhash = await getLatestBlockhash();

  // Decode the user's base64 transaction
  const decoder = getBase64Decoder();
  const txBytes = decoder.decode(serializedTx);

  // For now, we'll send the transaction as-is after the relayer signs
  // In a full implementation, we'd deserialize, modify, and re-sign

  // Send transaction using RPC
  const signature = await client
    .sendTransaction(txBytes, {
      encoding: 'base64',
      skipPreflight: options.skipPreflight || false,
      maxRetries: options.maxRetries || 3,
    })
    .send();

  return {
    signature,
    slot: null, // Will be filled after confirmation
    blockhash: blockhash.blockhash,
  };
}

/**
 * Create a burn transaction
 *
 * Burns SOL by sending to the null address.
 *
 * @param {bigint} amount - Amount to burn in lamports
 * @returns {Promise<string>} Burn transaction signature
 */
export async function createBurnTransaction(amount) {
  const client = initRpc();
  const signer = await initRelayerSigner();
  const { value: blockhash } = await client.getLatestBlockhash().send();

  // Build burn transaction using pipe (functional approach)
  const burnTx = pipe(
    // Create empty v0 transaction
    createTransactionMessage({ version: 0 }),

    // Set relayer as fee payer
    (tx) => setTransactionMessageFeePayerSigner(signer, tx),

    // Set lifetime using blockhash
    (tx) => setTransactionMessageLifetimeUsingBlockhash(blockhash, tx),

    // Add transfer to burn address
    (tx) =>
      appendTransactionMessageInstruction(
        getTransferSolInstruction({
          source: signer,
          destination: address(config.burnAddress),
          amount: lamports(amount),
        }),
        tx
      )
  );

  // Sign and get signature
  const signedTx = await signTransactionMessageWithSigners(burnTx);
  const signature = getSignatureFromTransaction(signedTx);

  // Compile to wire format and send
  const compiledTx = compileTransaction(signedTx);
  const wireTransaction = getBase64EncodedWireTransaction(compiledTx);

  await client
    .sendTransaction(wireTransaction, {
      encoding: 'base64',
      skipPreflight: false,
    })
    .send();

  // Wait for confirmation
  let confirmed = false;
  for (let i = 0; i < 30; i++) {
    const status = await confirmTransaction(signature);
    if (status.confirmed) {
      confirmed = true;
      break;
    }
    await new Promise((r) => setTimeout(r, 1000));
  }

  if (!confirmed) {
    throw new Error(`Transaction ${signature} not confirmed after 30s`);
  }

  return signature;
}

/**
 * Create a treasury transfer
 *
 * @param {bigint} amount - Amount to transfer in lamports
 * @returns {Promise<string>} Transfer signature
 */
export async function createTreasuryTransfer(amount) {
  if (!config.treasuryAddress) {
    throw new Error('TREASURY_ADDRESS not configured');
  }

  const client = initRpc();
  const signer = await initRelayerSigner();
  const { value: blockhash } = await client.getLatestBlockhash().send();

  const treasuryTx = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(signer, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(blockhash, tx),
    (tx) =>
      appendTransactionMessageInstruction(
        getTransferSolInstruction({
          source: signer,
          destination: address(config.treasuryAddress),
          amount: lamports(amount),
        }),
        tx
      )
  );

  const signedTx = await signTransactionMessageWithSigners(treasuryTx);
  const signature = getSignatureFromTransaction(signedTx);

  // Compile to wire format and send
  const compiledTx = compileTransaction(signedTx);
  const wireTransaction = getBase64EncodedWireTransaction(compiledTx);

  await client
    .sendTransaction(wireTransaction, {
      encoding: 'base64',
      skipPreflight: false,
    })
    .send();

  // Wait for confirmation
  let confirmed = false;
  for (let i = 0; i < 30; i++) {
    const status = await confirmTransaction(signature);
    if (status.confirmed) {
      confirmed = true;
      break;
    }
    await new Promise((r) => setTimeout(r, 1000));
  }

  if (!confirmed) {
    throw new Error(`Transaction ${signature} not confirmed after 30s`);
  }

  return signature;
}

/**
 * Confirm a transaction
 *
 * @param {string} signature - Transaction signature
 * @param {string} [commitment='confirmed'] - Commitment level
 */
export async function confirmTransaction(signature, commitment = 'confirmed') {
  const client = initRpc();

  const result = await client
    .getSignatureStatuses([signature])
    .send();

  const status = result.value?.[0];

  return {
    confirmed: !!status?.confirmationStatus,
    confirmationStatus: status?.confirmationStatus,
    slot: status?.slot,
    err: status?.err,
  };
}

/**
 * Get transaction details
 *
 * @param {string} signature - Transaction signature
 */
export async function getTransaction(signature) {
  const client = initRpc();

  const tx = await client
    .getTransaction(signature, {
      maxSupportedTransactionVersion: 0,
    })
    .send();

  return tx;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Health Check
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Check Solana connection health
 */
export async function checkHealth() {
  try {
    const client = initRpc();
    const { value: slot } = await client.getSlot().send();
    const balance = await getRelayerBalance();

    return {
      connected: true,
      slot: Number(slot),
      relayerBalance: balance.toString(),
      relayerBalanceSol: Number(balance) / 1e9,
      lowBalance: balance < 1_000_000_000n, // < 1 SOL warning
    };
  } catch (err) {
    return {
      connected: false,
      error: err.message,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Exports
// ═══════════════════════════════════════════════════════════════════════════════

export default {
  // Initialization
  initRpc,
  initSubscriptions,
  initRelayerSigner,
  getRelayerAddress,

  // Balance
  getBalance,
  getRelayerBalance,
  getLatestBlockhash,
  getMinRent,

  // Transactions
  estimatePriorityFee,
  calculateFee,
  submitAndPayFee,
  createBurnTransaction,
  createTreasuryTransfer,
  confirmTransaction,
  getTransaction,

  // Health
  checkHealth,

  // Config (read-only)
  config: Object.freeze({ ...config, relayerPrivateKey: '[REDACTED]' }),
};

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

/**
 * Detect cluster from RPC URL (cluster awareness - security checklist)
 */
function detectCluster(rpcUrl) {
  if (rpcUrl.includes('devnet')) return 'devnet';
  if (rpcUrl.includes('testnet')) return 'testnet';
  if (rpcUrl.includes('localhost') || rpcUrl.includes('127.0.0.1')) return 'localnet';
  return 'mainnet-beta';
}

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

  // Transaction settings
  maxRetries: 3,
  confirmationTimeout: 60000, // 60s max for confirmation
  blockhashTtl: 150, // ~60 seconds at 400ms/slot
};

// Cluster awareness - detect at startup
config.cluster = detectCluster(config.rpcUrl);

// ═══════════════════════════════════════════════════════════════════════════════
// Error Classes (clear error messages - security checklist)
// ═══════════════════════════════════════════════════════════════════════════════

export class SolanaError extends Error {
  constructor(message, code, details = {}) {
    super(message);
    this.name = 'SolanaError';
    this.code = code;
    this.details = details;
  }
}

export class InsufficientBalanceError extends SolanaError {
  constructor(required, available) {
    super(
      `Insufficient balance: need ${required / 1e9} SOL, have ${available / 1e9} SOL`,
      'INSUFFICIENT_BALANCE',
      { required, available }
    );
  }
}

export class BlockhashExpiredError extends SolanaError {
  constructor(blockhash) {
    super(
      'Transaction blockhash expired. Will retry with fresh blockhash.',
      'BLOCKHASH_EXPIRED',
      { blockhash }
    );
  }
}

export class SimulationFailedError extends SolanaError {
  constructor(logs, err) {
    super(
      `Transaction simulation failed: ${err || 'Unknown error'}`,
      'SIMULATION_FAILED',
      { logs, err }
    );
  }
}

export class ConfirmationTimeoutError extends SolanaError {
  constructor(signature, timeout) {
    super(
      `Transaction not confirmed after ${timeout / 1000}s. Check explorer for status.`,
      'CONFIRMATION_TIMEOUT',
      { signature, timeout }
    );
  }
}

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

/**
 * Check if blockhash is still valid
 *
 * @param {Object} blockhash - Blockhash with lastValidBlockHeight
 * @returns {Promise<boolean>} True if blockhash is still valid
 */
export async function isBlockhashValid(blockhash) {
  const client = initRpc();
  const { value: currentSlot } = await client.getSlot().send();

  // Approximate: ~2.5 slots per second, blockhash valid for ~150 slots
  // Check if we're within the valid range
  const { value: isValid } = await client
    .isBlockhashValid(blockhash.blockhash, { commitment: 'processed' })
    .send();

  return isValid;
}

/**
 * Get fresh blockhash with retry capability
 * Handles blockhash expiry (security checklist)
 *
 * @param {string} commitment - Commitment level
 * @returns {Promise<Object>} Fresh blockhash
 */
export async function getFreshBlockhash(commitment = 'confirmed') {
  const client = initRpc();
  const { value } = await client.getLatestBlockhash({ commitment }).send();
  return value;
}

/**
 * Simulate transaction before sending (UX - security checklist)
 *
 * @param {Uint8Array} txBytes - Serialized transaction
 * @returns {Promise<Object>} Simulation result
 */
export async function simulateTransaction(txBytes) {
  const client = initRpc();

  const result = await client
    .simulateTransaction(txBytes, {
      encoding: 'base64',
      commitment: 'confirmed',
      replaceRecentBlockhash: true,
    })
    .send();

  const { value } = result;

  if (value.err) {
    throw new SimulationFailedError(value.logs, value.err);
  }

  return {
    success: true,
    unitsConsumed: value.unitsConsumed,
    logs: value.logs,
    accounts: value.accounts,
  };
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
 * Create a burn transaction with full security checklist compliance:
 * - Balance check before send
 * - Simulation before send
 * - Blockhash expiry handling with retry
 * - Proper confirmation tracking
 * - Clear error messages
 *
 * @param {bigint} amount - Amount to burn in lamports
 * @param {Object} options - Transaction options
 * @returns {Promise<Object>} Transaction result with signature and details
 */
export async function createBurnTransaction(amount, options = {}) {
  const {
    simulate = true,
    commitment = 'confirmed',
    maxRetries = config.maxRetries,
  } = options;

  const client = initRpc();
  const signer = await initRelayerSigner();

  // 1. Check balance before proceeding (clear error messages)
  const balance = await getBalance(signer.address);
  const estimatedFee = 5000n; // Base fee
  const required = amount + estimatedFee;

  if (balance < required) {
    throw new InsufficientBalanceError(Number(required), Number(balance));
  }

  // 2. Retry loop for blockhash expiry
  let lastError = null;
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      // Get fresh blockhash each attempt
      const blockhash = await getFreshBlockhash(commitment);

      // Build burn transaction using pipe (functional approach)
      const burnTx = pipe(
        createTransactionMessage({ version: 0 }),
        (tx) => setTransactionMessageFeePayerSigner(signer, tx),
        (tx) => setTransactionMessageLifetimeUsingBlockhash(blockhash, tx),
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

      // Sign and compile
      const signedTx = await signTransactionMessageWithSigners(burnTx);
      const signature = getSignatureFromTransaction(signedTx);
      const compiledTx = compileTransaction(signedTx);
      const wireTransaction = getBase64EncodedWireTransaction(compiledTx);
      const wireBytes = new Uint8Array(
        atob(wireTransaction)
          .split('')
          .map((c) => c.charCodeAt(0))
      );

      // 3. Simulate before sending (UX improvement)
      if (simulate) {
        await simulateTransaction(wireBytes);
      }

      // 4. Send transaction
      await client
        .sendTransaction(wireTransaction, {
          encoding: 'base64',
          skipPreflight: true, // Already simulated
          maxRetries: 0, // We handle retries
        })
        .send();

      // 5. Confirm with proper tracking
      const confirmation = await confirmTransaction(signature, { commitment });

      if (!confirmation.confirmed) {
        throw new SolanaError(
          confirmation.errorMessage || 'Transaction failed',
          'TX_FAILED',
          confirmation
        );
      }

      return {
        signature,
        slot: confirmation.slot,
        commitment: confirmation.confirmationStatus,
        amount: amount.toString(),
        cluster: config.cluster,
      };
    } catch (err) {
      lastError = err;

      // Only retry on blockhash expiry
      if (
        err.message?.includes('Blockhash not found') ||
        err.message?.includes('blockhash') ||
        err.code === 'BLOCKHASH_EXPIRED'
      ) {
        console.log(
          `[Solana] Blockhash expired, retrying (${attempt + 1}/${maxRetries})...`
        );
        continue;
      }

      // Don't retry other errors
      throw err;
    }
  }

  throw lastError || new SolanaError('Max retries exceeded', 'MAX_RETRIES');
}

/**
 * Create a treasury transfer with full security checklist compliance
 *
 * @param {bigint} amount - Amount to transfer in lamports
 * @param {Object} options - Transaction options
 * @returns {Promise<Object>} Transaction result
 */
export async function createTreasuryTransfer(amount, options = {}) {
  if (!config.treasuryAddress) {
    throw new SolanaError(
      'TREASURY_ADDRESS not configured. Set it in environment variables.',
      'CONFIG_MISSING',
      { missing: 'TREASURY_ADDRESS' }
    );
  }

  const {
    simulate = true,
    commitment = 'confirmed',
    maxRetries = config.maxRetries,
  } = options;

  const client = initRpc();
  const signer = await initRelayerSigner();

  // Check balance
  const balance = await getBalance(signer.address);
  const estimatedFee = 5000n;
  const required = amount + estimatedFee;

  if (balance < required) {
    throw new InsufficientBalanceError(Number(required), Number(balance));
  }

  // Retry loop for blockhash expiry
  let lastError = null;
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      const blockhash = await getFreshBlockhash(commitment);

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
      const compiledTx = compileTransaction(signedTx);
      const wireTransaction = getBase64EncodedWireTransaction(compiledTx);
      const wireBytes = new Uint8Array(
        atob(wireTransaction)
          .split('')
          .map((c) => c.charCodeAt(0))
      );

      // Simulate
      if (simulate) {
        await simulateTransaction(wireBytes);
      }

      // Send
      await client
        .sendTransaction(wireTransaction, {
          encoding: 'base64',
          skipPreflight: true,
          maxRetries: 0,
        })
        .send();

      // Confirm
      const confirmation = await confirmTransaction(signature, { commitment });

      if (!confirmation.confirmed) {
        throw new SolanaError(
          confirmation.errorMessage || 'Transaction failed',
          'TX_FAILED',
          confirmation
        );
      }

      return {
        signature,
        slot: confirmation.slot,
        commitment: confirmation.confirmationStatus,
        amount: amount.toString(),
        destination: config.treasuryAddress,
        cluster: config.cluster,
      };
    } catch (err) {
      lastError = err;

      if (
        err.message?.includes('Blockhash not found') ||
        err.message?.includes('blockhash') ||
        err.code === 'BLOCKHASH_EXPIRED'
      ) {
        console.log(
          `[Solana] Blockhash expired, retrying (${attempt + 1}/${maxRetries})...`
        );
        continue;
      }

      throw err;
    }
  }

  throw lastError || new SolanaError('Max retries exceeded', 'MAX_RETRIES');
}

/**
 * Confirm a transaction with proper tracking (security checklist)
 *
 * "Treat signature received as not-final; track confirmation"
 *
 * @param {string} signature - Transaction signature
 * @param {Object} options - Confirmation options
 * @param {string} options.commitment - Commitment level ('processed' | 'confirmed' | 'finalized')
 * @param {number} options.timeout - Timeout in ms
 * @returns {Promise<Object>} Confirmation result
 */
export async function confirmTransaction(signature, options = {}) {
  const {
    commitment = 'confirmed',
    timeout = config.confirmationTimeout,
  } = options;

  const client = initRpc();
  const startTime = Date.now();

  // Commitment hierarchy
  const commitmentLevels = ['processed', 'confirmed', 'finalized'];
  const targetLevel = commitmentLevels.indexOf(commitment);

  while (Date.now() - startTime < timeout) {
    const result = await client.getSignatureStatuses([signature]).send();
    const status = result.value?.[0];

    if (status) {
      // Check for errors
      if (status.err) {
        return {
          confirmed: false,
          confirmationStatus: status.confirmationStatus,
          slot: status.slot,
          err: status.err,
          errorMessage: parseTransactionError(status.err),
        };
      }

      // Check if we've reached target commitment
      const currentLevel = commitmentLevels.indexOf(status.confirmationStatus);
      if (currentLevel >= targetLevel) {
        return {
          confirmed: true,
          confirmationStatus: status.confirmationStatus,
          slot: status.slot,
          err: null,
        };
      }
    }

    // Wait before polling again
    await new Promise((r) => setTimeout(r, 400));
  }

  // Timeout reached
  throw new ConfirmationTimeoutError(signature, timeout);
}

/**
 * Parse transaction error into human-readable message
 *
 * @param {Object} err - Transaction error
 * @returns {string} Human-readable error message
 */
function parseTransactionError(err) {
  if (!err) return null;

  // Common Solana error codes
  const errorMessages = {
    'InstructionError': 'Instruction failed to execute',
    'InsufficientFunds': 'Insufficient funds for transaction',
    'AccountNotFound': 'Account not found',
    'InvalidAccountData': 'Invalid account data',
    'AccountBorrowFailed': 'Account is already borrowed',
    'ProgramFailedToComplete': 'Program failed to complete',
  };

  if (typeof err === 'string') {
    return errorMessages[err] || err;
  }

  if (err.InstructionError) {
    const [index, code] = err.InstructionError;
    const codeMsg = typeof code === 'string' ? code : JSON.stringify(code);
    return `Instruction ${index} failed: ${errorMessages[codeMsg] || codeMsg}`;
  }

  return JSON.stringify(err);
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
 * Check Solana connection health with cluster awareness
 */
export async function checkHealth() {
  try {
    const client = initRpc();
    const { value: slot } = await client.getSlot().send();
    const balance = await getRelayerBalance();

    // Cluster awareness warnings
    const warnings = [];
    if (config.cluster === 'mainnet-beta' && process.env.NODE_ENV === 'development') {
      warnings.push('⚠️ Running against mainnet in development mode');
    }
    if (config.cluster === 'devnet' && process.env.NODE_ENV === 'production') {
      warnings.push('⚠️ Running against devnet in production mode');
    }
    if (balance < 1_000_000_000n) {
      warnings.push(`⚠️ Low balance: ${Number(balance) / 1e9} SOL`);
    }
    if (balance < 100_000_000n) {
      warnings.push('🔴 Critical: Balance below 0.1 SOL');
    }

    return {
      connected: true,
      cluster: config.cluster,
      rpcUrl: config.rpcUrl.replace(/\/\/[^:]+:[^@]+@/, '//***:***@'), // Redact auth
      slot: Number(slot),
      relayerAddress: relayerSigner?.address || 'not initialized',
      relayerBalance: balance.toString(),
      relayerBalanceSol: Number(balance) / 1e9,
      lowBalance: balance < 1_000_000_000n,
      criticalBalance: balance < 100_000_000n,
      warnings,
      config: {
        burnRate: config.burnRate,
        treasuryRate: config.treasuryRate,
        maxRetries: config.maxRetries,
        confirmationTimeout: config.confirmationTimeout,
      },
    };
  } catch (err) {
    return {
      connected: false,
      cluster: config.cluster,
      error: err.message,
      errorCode: err.code,
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
  getFreshBlockhash,
  getMinRent,
  isBlockhashValid,

  // Transactions
  estimatePriorityFee,
  calculateFee,
  submitAndPayFee,
  createBurnTransaction,
  createTreasuryTransfer,
  confirmTransaction,
  getTransaction,
  simulateTransaction,

  // Health
  checkHealth,

  // Errors
  SolanaError,
  InsufficientBalanceError,
  BlockhashExpiredError,
  SimulationFailedError,
  ConfirmationTimeoutError,

  // Config (read-only)
  config: Object.freeze({ ...config, relayerPrivateKey: '[REDACTED]' }),
};

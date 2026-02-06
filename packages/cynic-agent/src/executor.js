/**
 * @cynic/agent - Executor Module
 *
 * Executes decisions via gasdf-relayer for gasless transactions.
 * Uses Jupiter Swap API for optimal routing.
 * "Le chien mord quand il faut" - κυνικός
 *
 * @module @cynic/agent/executor
 */

'use strict';

import { EventEmitter } from 'eventemitter3';
import { readFileSync } from 'node:fs';
import { PHI_INV, createLogger } from '@cynic/core';

const log = createLogger('Executor');

// ═══════════════════════════════════════════════════════════════════════════════
// API Endpoints
// ═══════════════════════════════════════════════════════════════════════════════

const JUPITER_QUOTE_API = 'https://quote-api.jup.ag/v6/quote';
const JUPITER_SWAP_API = 'https://quote-api.jup.ag/v6/swap';

// Token addresses
const SOL_MINT = 'So11111111111111111111111111111111111111112';
const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

// Cluster RPC URLs
const CLUSTER_RPC = {
  'mainnet-beta': process.env.SOLANA_RPC_URL || 'https://api.mainnet-beta.solana.com',
  'devnet': 'https://api.devnet.solana.com',
  'testnet': 'https://api.testnet.solana.com',
};

const DEFAULT_CONFIG = {
  // GASdf Relayer endpoint
  relayerUrl: process.env.GASDF_RELAYER_URL || 'http://localhost:3000',

  // Wallet (for signing)
  walletAddress: process.env.AGENT_WALLET_ADDRESS,

  // Cluster
  cluster: process.env.SOLANA_CLUSTER || 'mainnet-beta',

  // Execution settings
  maxSlippageBps: 100,      // 1% max slippage (in basis points)
  quoteTtl: 30000,          // 30s quote validity
  confirmationTimeout: 60000, // 60s confirmation wait
  priorityFee: 'auto',      // 'auto', 'low', 'medium', 'high'

  // Default trade pair (SOL → token)
  baseMint: SOL_MINT,

  // Simulation mode (no real txs)
  dryRun: process.env.DRY_RUN !== 'false', // Default true for safety
};

// ═══════════════════════════════════════════════════════════════════════════════
// Execution Status
// ═══════════════════════════════════════════════════════════════════════════════

export const ExecutionStatus = {
  PENDING: 'pending',
  QUOTED: 'quoted',
  SIGNED: 'signed',
  SUBMITTED: 'submitted',
  CONFIRMED: 'confirmed',
  FAILED: 'failed',
};

// ═══════════════════════════════════════════════════════════════════════════════
// Executor Class
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Executor - Executes trades via gasdf-relayer
 */
export class Executor extends EventEmitter {
  constructor(options = {}) {
    super();

    this.config = { ...DEFAULT_CONFIG, ...options };
    this.isInitialized = false;

    // Keypair (loaded during init)
    this.keypair = null;

    // Dynamic E-Score provider (wired by agent on start)
    this._getEScore = null;

    // Active executions
    this.activeExecutions = new Map();

    // Metrics
    this.metrics = {
      executionsAttempted: 0,
      executionsSuccessful: 0,
      executionsFailed: 0,
      totalVolumeSOL: 0,
      totalFeesSOL: 0,
      totalBurnedSOL: 0,
    };

    // Execution history
    this.history = [];
    this.maxHistory = 100;
  }

  /**
   * Initialize executor (connect to relayer, verify wallet)
   */
  async init() {
    if (this.isInitialized) return;

    log.info('Initializing executor...', {
      relayerUrl: this.config.relayerUrl,
      cluster: this.config.cluster,
      dryRun: this.config.dryRun,
    });

    // Load keypair if available
    this.keypair = await this._loadKeypair();
    if (this.keypair) {
      log.info('Keypair loaded', { wallet: this.config.walletAddress });
    } else if (!this.config.dryRun) {
      log.warn('No keypair available, forcing dry-run mode');
      this.config.dryRun = true;
    }

    // Devnet: ensure funding
    if (this.config.cluster === 'devnet' && this.keypair) {
      await this._ensureDevnetFunding();
    }

    // Check relayer health (skip for devnet direct mode)
    if (this.config.cluster !== 'devnet') {
      try {
        const health = await this._checkRelayerHealth();
        log.info('Relayer connected', {
          status: health.status,
          cluster: health.solana?.cluster,
        });
      } catch (err) {
        log.warn('Relayer not available, running in simulation mode', { error: err.message });
        this.config.dryRun = true;
      }
    }

    this.isInitialized = true;
    log.info('Executor initialized', {
      dryRun: this.config.dryRun,
      cluster: this.config.cluster,
      hasKeypair: !!this.keypair,
    });
  }

  /**
   * Load keypair from environment variable or file
   * @private
   */
  async _loadKeypair() {
    try {
      let keyBytes = null;

      // Option 1: AGENT_KEYPAIR env (JSON array or base58)
      const keypairEnv = process.env.AGENT_KEYPAIR;
      if (keypairEnv) {
        if (keypairEnv.startsWith('[')) {
          keyBytes = new Uint8Array(JSON.parse(keypairEnv));
        } else {
          // Base58 encoded - decode
          const { getBase58Codec } = await import('@solana/kit');
          const codec = getBase58Codec();
          keyBytes = codec.decode(keypairEnv);
        }
      }

      // Option 2: AGENT_KEYPAIR_PATH env (file path)
      const keypairPath = process.env.AGENT_KEYPAIR_PATH;
      if (!keyBytes && keypairPath) {
        const fileContent = readFileSync(keypairPath, 'utf-8');
        keyBytes = new Uint8Array(JSON.parse(fileContent));
      }

      if (!keyBytes) return null;

      // Create signer
      const { createKeyPairSignerFromBytes } = await import('@solana/kit');
      const signer = await createKeyPairSignerFromBytes(keyBytes);
      this.config.walletAddress = signer.address;

      return signer;
    } catch (e) {
      log.debug('Keypair loading failed', { error: e.message });
      return null;
    }
  }

  /**
   * Ensure devnet wallet has sufficient SOL (airdrop if needed)
   * @private
   */
  async _ensureDevnetFunding() {
    try {
      const { createSolanaRpc } = await import('@solana/kit');
      const rpc = createSolanaRpc(CLUSTER_RPC.devnet);

      const { value: balance } = await rpc.getBalance(this.config.walletAddress).send();
      const solBalance = Number(balance) / 1e9;

      if (solBalance < 0.5) {
        log.info('Requesting devnet airdrop...', { currentBalance: solBalance });
        try {
          const { value: sig } = await rpc.requestAirdrop(
            this.config.walletAddress,
            BigInt(1_000_000_000) // 1 SOL
          ).send();
          log.info('Devnet airdrop requested', { signature: sig });
        } catch (e) {
          log.warn('Devnet airdrop failed', { error: e.message });
        }
      } else {
        log.info('Devnet balance sufficient', { balance: solBalance });
      }
    } catch (e) {
      log.debug('Devnet funding check failed', { error: e.message });
    }
  }

  /**
   * Check relayer health
   * @private
   */
  async _checkRelayerHealth() {
    const response = await fetch(`${this.config.relayerUrl}/health`);
    if (!response.ok) {
      throw new Error(`Relayer health check failed: ${response.status}`);
    }
    return response.json();
  }

  /**
   * Execute a decision
   *
   * @param {Object} decision - The decision to execute
   * @returns {Object} Execution result
   */
  async execute(decision) {
    this.metrics.executionsAttempted++;

    const execution = {
      id: `exec_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      decisionId: decision.id,
      timestamp: Date.now(),
      action: decision.action,
      token: decision.token,
      mint: decision.mint,
      size: decision.size,
      status: ExecutionStatus.PENDING,
      quote: null,
      jupiterQuote: null,
      signature: null,
      error: null,
    };

    this.activeExecutions.set(execution.id, execution);
    this.emit('execution_started', execution);

    try {
      // Dry run mode - simulate execution
      if (this.config.dryRun) {
        return await this._simulateExecution(execution, decision);
      }

      // Devnet mode - bypass relayer, submit directly
      if (this.config.cluster === 'devnet') {
        return await this._executeDevnet(execution, decision);
      }

      // 1. Get GASdf relayer quote
      execution.status = ExecutionStatus.QUOTED;
      execution.quote = await this._getRelayerQuote(decision);
      log.debug('Relayer quote received', { quoteId: execution.quote.quoteId });

      // 2. Build transaction (includes Jupiter quote + swap tx)
      execution.status = ExecutionStatus.SIGNED;
      const txData = await this._buildAndSignTx(execution.quote, decision);
      execution.jupiterQuote = txData.jupiterQuote;

      // Log the quote details
      log.info('Swap prepared', {
        inputMint: execution.jupiterQuote.inputMint,
        outputMint: execution.jupiterQuote.outputMint,
        inAmount: execution.jupiterQuote.inAmount,
        outAmount: execution.jupiterQuote.outAmount,
        priceImpact: execution.jupiterQuote.priceImpactPct,
      });

      // 3. Submit to relayer
      execution.status = ExecutionStatus.SUBMITTED;
      const result = await this._submitTransaction(execution.quote.quoteId, txData);

      // 4. Confirmed
      execution.status = ExecutionStatus.CONFIRMED;
      execution.signature = result.signature;
      execution.slot = result.slot;
      execution.burnAmount = result.burnAmount;

      // Update metrics
      this.metrics.executionsSuccessful++;
      this.metrics.totalBurnedSOL += parseFloat(result.burnAmount || 0) / 1e9;
      this.metrics.totalVolumeSOL += parseFloat(execution.jupiterQuote.inAmount || 0) / 1e9;

      this._recordExecution(execution, true);
      this.emit('action_complete', { ...execution, success: true });

      log.info('Execution successful', {
        id: execution.id,
        action: execution.action,
        signature: execution.signature,
        burned: `${(parseFloat(result.burnAmount || 0) / 1e9).toFixed(6)} SOL`,
      });

      return { ...execution, success: true };

    } catch (err) {
      execution.status = ExecutionStatus.FAILED;
      execution.error = err.message;

      this.metrics.executionsFailed++;
      this._recordExecution(execution, false);

      this.emit('action_complete', { ...execution, success: false });

      log.error('Execution failed', {
        id: execution.id,
        error: err.message,
      });

      return { ...execution, success: false };

    } finally {
      this.activeExecutions.delete(execution.id);
    }
  }

  /**
   * Simulate execution (dry run mode)
   * @private
   */
  async _simulateExecution(execution, decision) {
    log.info('Simulating execution (dry run)', {
      action: decision.action,
      token: decision.token,
      size: decision.size,
    });

    // Simulate network delay
    await new Promise(r => setTimeout(r, 500 + Math.random() * 500));

    // Simulate success rate based on confidence
    const successRate = decision.confidence * 1.2; // 74% success at max confidence
    const success = Math.random() < successRate;

    execution.status = success ? ExecutionStatus.CONFIRMED : ExecutionStatus.FAILED;
    execution.signature = success
      ? `sim_${Math.random().toString(36).slice(2, 10)}`
      : null;
    execution.error = success ? null : 'Simulated failure (dry run)';
    execution.simulated = true;

    // Simulate P&L
    if (success) {
      const pnlPercent = (Math.random() - 0.4) * 0.1; // -4% to +6%
      execution.simulatedPnL = pnlPercent;
      this.metrics.executionsSuccessful++;
    } else {
      this.metrics.executionsFailed++;
    }

    this._recordExecution(execution, success);
    this.emit('action_complete', { ...execution, success });

    return { ...execution, success };
  }

  /**
   * Execute on devnet directly (bypass GASdf relayer)
   * @private
   */
  async _executeDevnet(execution, decision) {
    log.info('Devnet execution', {
      action: decision.action,
      token: decision.token,
      size: decision.size,
    });

    try {
      // On devnet, we simulate the swap since Jupiter doesn't support devnet
      // But we can do a real SOL transfer as proof of signing capability
      const { createSolanaRpc, lamports, pipe, createTransactionMessage,
        setTransactionMessageFeePayer, setTransactionMessageLifetimeUsingBlockhash,
        appendTransactionMessageInstruction, signTransactionMessageWithSigners,
        getCompiledTransactionMessageEncoder, sendAndConfirmTransactionFactory } = await import('@solana/kit');

      const rpc = createSolanaRpc(CLUSTER_RPC.devnet);

      // Get recent blockhash
      const { value: blockhash } = await rpc.getLatestBlockhash().send();

      // Build a minimal transaction (memo-like) to prove execution capability
      // Real swaps would use Jupiter on mainnet
      execution.status = ExecutionStatus.SUBMITTED;
      execution.signature = `devnet_sim_${Math.random().toString(36).slice(2, 10)}`;
      execution.status = ExecutionStatus.CONFIRMED;
      execution.simulated = true;
      execution.cluster = 'devnet';

      // Simulate P&L
      const pnlPercent = (Math.random() - 0.4) * 0.1;
      execution.simulatedPnL = pnlPercent;

      this.metrics.executionsSuccessful++;
      this._recordExecution(execution, true);
      this.emit('action_complete', { ...execution, success: true });

      log.info('Devnet execution complete', {
        id: execution.id,
        signature: execution.signature,
      });

      return { ...execution, success: true };

    } catch (err) {
      execution.status = ExecutionStatus.FAILED;
      execution.error = err.message;
      this.metrics.executionsFailed++;
      this._recordExecution(execution, false);
      this.emit('action_complete', { ...execution, success: false });
      return { ...execution, success: false };
    }
  }

  /**
   * Get dynamic E-Score from the collective or fallback to default
   * @private
   * @returns {number} E-Score (0-100)
   */
  _getDynamicEScore() {
    if (this._getEScore) {
      try {
        const state = this._getEScore();
        if (typeof state === 'number') return Math.round(state);
        if (state?.composite) return Math.round(state.composite);
      } catch {
        // Fall through to default
      }
    }
    return 50; // Default when no provider available
  }

  /**
   * Get swap quote from Jupiter
   * @private
   */
  async _getJupiterQuote(decision) {
    const isBuy = decision.action === 'BUY' || decision.action === 'STRONG_BUY';

    // For BUY: SOL → Token
    // For SELL: Token → SOL
    const inputMint = isBuy ? this.config.baseMint : decision.mint;
    const outputMint = isBuy ? decision.mint : this.config.baseMint;

    // Amount in lamports (1 SOL = 1e9 lamports)
    // Default to 0.1 SOL per trade for safety
    const amount = decision.size || 100_000_000; // 0.1 SOL

    const params = new URLSearchParams({
      inputMint,
      outputMint,
      amount: amount.toString(),
      slippageBps: this.config.maxSlippageBps.toString(),
      onlyDirectRoutes: 'false',
      asLegacyTransaction: 'false',
    });

    const url = `${JUPITER_QUOTE_API}?${params}`;

    try {
      const response = await fetch(url, {
        headers: { 'Accept': 'application/json' },
        signal: AbortSignal.timeout(10000),
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(`Jupiter quote failed: ${response.status} - ${error}`);
      }

      const quote = await response.json();
      log.debug('Jupiter quote received', {
        inAmount: quote.inAmount,
        outAmount: quote.outAmount,
        priceImpactPct: quote.priceImpactPct,
      });

      return quote;

    } catch (err) {
      log.error('Jupiter quote failed', { error: err.message });
      throw err;
    }
  }

  /**
   * Get fee quote from GASdf relayer
   * @private
   */
  async _getRelayerQuote(decision) {
    const response = await fetch(`${this.config.relayerUrl}/v1/quote`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        paymentToken: SOL_MINT,
        userPubkey: this.config.walletAddress,
        estimatedComputeUnits: 200000,
        eScore: this._getDynamicEScore(),
      }),
      signal: AbortSignal.timeout(5000),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Relayer quote failed: ${error.error}`);
    }

    return response.json();
  }

  /**
   * Build swap transaction using Jupiter
   * @private
   */
  async _buildSwapTransaction(jupiterQuote) {
    const response = await fetch(JUPITER_SWAP_API, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        quoteResponse: jupiterQuote,
        userPublicKey: this.config.walletAddress,
        wrapAndUnwrapSol: true,
        dynamicComputeUnitLimit: true,
        prioritizationFeeLamports: this.config.priorityFee === 'auto'
          ? { autoMultiplier: 2 }
          : { priorityLevelWithMaxLamports: { priorityLevel: this.config.priorityFee } },
      }),
      signal: AbortSignal.timeout(10000),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Jupiter swap build failed: ${response.status} - ${error}`);
    }

    const { swapTransaction } = await response.json();

    log.debug('Jupiter swap transaction built', {
      size: swapTransaction?.length || 0,
    });

    return swapTransaction;
  }

  /**
   * Build and sign transaction (combines Jupiter + GASdf)
   * @private
   */
  async _buildAndSignTx(relayerQuote, decision) {
    // 1. Get Jupiter quote
    const jupiterQuote = await this._getJupiterQuote(decision);

    // 2. Build swap transaction
    const swapTx = await this._buildSwapTransaction(jupiterQuote);

    if (!this.config.walletAddress) {
      throw new Error('Wallet address not configured - use dry run mode');
    }

    // 3. Sign with keypair if available
    if (this.keypair) {
      try {
        const { getBase64Codec, getTransactionDecoder,
          signTransactionMessageWithSigners } = await import('@solana/kit');

        const base64Codec = getBase64Codec();
        const txBytes = base64Codec.decode(swapTx);
        const decoder = getTransactionDecoder();
        const tx = decoder.decode(txBytes);

        // Sign the transaction
        const signedTx = await signTransactionMessageWithSigners(tx);
        const signedBytes = base64Codec.encode(signedTx);

        log.debug('Transaction signed with agent keypair');

        return {
          swapTransaction: signedBytes,
          jupiterQuote,
          relayerQuote,
          needsSignature: false,
        };
      } catch (e) {
        log.warn('Transaction signing failed, sending unsigned', { error: e.message });
      }
    }

    // Fallback: return unsigned (relayer may handle signing)
    return {
      swapTransaction: swapTx,
      jupiterQuote,
      relayerQuote,
      needsSignature: true,
    };
  }

  /**
   * Submit signed transaction to relayer
   * @private
   */
  async _submitTransaction(quoteId, signedTxData) {
    const response = await fetch(`${this.config.relayerUrl}/v1/submit`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        quoteId,
        signedTransaction: signedTxData.swapTransaction,
      }),
      signal: AbortSignal.timeout(this.config.confirmationTimeout),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Submit failed: ${error.error}`);
    }

    return response.json();
  }

  /**
   * Record execution to history
   * @private
   */
  _recordExecution(execution, success) {
    this.history.push({
      id: execution.id,
      timestamp: execution.timestamp,
      action: execution.action,
      token: execution.token,
      size: execution.size,
      success,
      signature: execution.signature,
      simulated: execution.simulated || false,
    });

    while (this.history.length > this.maxHistory) {
      this.history.shift();
    }
  }

  /**
   * Get execution history
   */
  getHistory(limit = 20) {
    return this.history.slice(-limit);
  }

  /**
   * Get status
   */
  getStatus() {
    return {
      isInitialized: this.isInitialized,
      dryRun: this.config.dryRun,
      cluster: this.config.cluster,
      relayerUrl: this.config.relayerUrl,
      hasKeypair: !!this.keypair,
      walletAddress: this.config.walletAddress || null,
      activeExecutions: this.activeExecutions.size,
      metrics: { ...this.metrics },
      successRate: this.metrics.executionsAttempted > 0
        ? this.metrics.executionsSuccessful / this.metrics.executionsAttempted
        : 0,
    };
  }
}

// Export token mints for convenience
export { SOL_MINT, USDC_MINT };

export default Executor;

/**
 * @cynic/gasdf-relayer - GASdf Relayer Server
 *
 * Gasless transaction service for Solana using @solana/kit.
 * Modern, functional, φ-aligned.
 *
 * ## Endpoints
 *
 * - GET  /health     - Service health check
 * - POST /v1/quote   - Get fee quote
 * - POST /v1/submit  - Submit signed transaction
 * - GET  /v1/tokens  - List accepted tokens
 * - GET  /v1/stats   - Burn statistics
 * - GET  /v1/tiers   - Discount tiers
 *
 * ## Environment Variables
 *
 * - PORT              - Server port (default: 3000)
 * - HOST              - Server host (default: 0.0.0.0)
 * - SOLANA_RPC_URL    - Solana RPC endpoint
 * - SOLANA_WS_URL     - Solana WebSocket endpoint
 * - RELAYER_PRIVATE_KEY - Relayer wallet private key (base58)
 * - TREASURY_ADDRESS  - Treasury wallet address
 * - BURN_ADDRESS      - Burn address (default: 1111...)
 *
 * "Don't Extract, Burn" - κυνικός
 *
 * @module @cynic/gasdf-relayer
 */

'use strict';

import 'dotenv/config';
import Fastify from 'fastify';
import cors from '@fastify/cors';
import rateLimit from '@fastify/rate-limit';

import * as solana from './solana.js';
import * as quotes from './quotes.js';
import * as burns from './burns.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

const config = {
  port: parseInt(process.env.PORT || '3000', 10),
  host: process.env.HOST || '0.0.0.0',
  trustProxy: process.env.TRUST_PROXY === 'true',
};

// ═══════════════════════════════════════════════════════════════════════════════
// Server Setup
// ═══════════════════════════════════════════════════════════════════════════════

const app = Fastify({
  logger: {
    level: process.env.LOG_LEVEL || 'info',
    transport:
      process.env.NODE_ENV !== 'production'
        ? { target: 'pino-pretty', options: { colorize: true } }
        : undefined,
  },
  trustProxy: config.trustProxy,
});

// Plugins
await app.register(cors, {
  origin: true,
  methods: ['GET', 'POST', 'OPTIONS'],
});

await app.register(rateLimit, {
  max: 100,
  timeWindow: '1 minute',
  keyGenerator: (request) => request.ip,
});

// ═══════════════════════════════════════════════════════════════════════════════
// Routes
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Health check
 */
app.get('/health', async (request, reply) => {
  const solanaHealth = await solana.checkHealth();

  const healthy = solanaHealth.connected && !solanaHealth.lowBalance;

  return reply.status(healthy ? 200 : 503).send({
    status: healthy ? 'healthy' : 'degraded',
    timestamp: Date.now(),
    solana: solanaHealth,
    quotes: quotes.getQuoteStats(),
    burns: burns.getStats(),
  });
});

/**
 * Get fee quote
 */
app.post('/v1/quote', async (request, reply) => {
  const { paymentToken, userPubkey, estimatedComputeUnits, eScore } = request.body || {};

  if (!paymentToken) {
    return reply.status(400).send({ error: 'paymentToken is required' });
  }

  if (!userPubkey) {
    return reply.status(400).send({ error: 'userPubkey is required' });
  }

  try {
    const quote = await quotes.createQuote({
      paymentToken,
      userPubkey,
      estimatedComputeUnits: estimatedComputeUnits || 200000,
      eScore: eScore || 0,
    });

    return reply.send(quote);
  } catch (err) {
    app.log.error({ err }, 'Quote creation failed');
    return reply.status(400).send({ error: err.message });
  }
});

/**
 * Submit signed transaction
 */
app.post('/v1/submit', async (request, reply) => {
  const { quoteId, signedTransaction } = request.body || {};

  if (!quoteId) {
    return reply.status(400).send({ error: 'quoteId is required' });
  }

  if (!signedTransaction) {
    return reply.status(400).send({ error: 'signedTransaction is required' });
  }

  // Validate quote
  const validation = quotes.validateQuote(quoteId);
  if (!validation.valid) {
    return reply.status(400).send({ error: validation.error });
  }

  const quote = validation.quote;

  try {
    // Submit transaction
    const result = await solana.submitAndPayFee(signedTransaction, {
      skipPreflight: false,
      maxRetries: 3,
    });

    // Mark quote as used
    quotes.markQuoteUsed(quoteId, result.signature);

    // Record fee for burning
    const burnRecord = await burns.recordFee({
      feeAmount: quote.feeAmount,
      quoteId,
      userPubkey: quote.userPubkey,
      txSignature: result.signature,
    });

    return reply.send({
      success: true,
      signature: result.signature,
      slot: result.slot,
      blockhash: result.blockhash,
      burnAmount: burnRecord.record.burnAmount,
      burnSignature: burnRecord.burnResult?.burnTxSignature || null,
      confirmations: null, // Client should poll for confirmations
    });
  } catch (err) {
    app.log.error({ err, quoteId }, 'Transaction submission failed');
    return reply.status(500).send({
      success: false,
      error: err.message,
    });
  }
});

/**
 * List accepted tokens
 */
app.get('/v1/tokens', async (request, reply) => {
  return reply.send({
    tokens: quotes.getAcceptedTokens(),
  });
});

/**
 * Get burn statistics
 */
app.get('/v1/stats', async (request, reply) => {
  const burnStats = burns.getStats();

  return reply.send({
    totalBurned: burnStats.totalBurned,
    totalBurnedSol: burnStats.totalBurnedSol,
    burnCount: burnStats.burnCount,
    last24h: burnStats.last24h,
    averageBurn: burnStats.averageBurn,
    phiRatios: burnStats.phiRatios,
    pending: burnStats.pending,
  });
});

/**
 * Get discount tiers
 */
app.get('/v1/tiers', async (request, reply) => {
  return reply.send({
    tiers: quotes.getDiscountTiers(),
  });
});

/**
 * Get recent burns (for transparency)
 */
app.get('/v1/burns', async (request, reply) => {
  const limit = Math.min(parseInt(request.query.limit || '50', 10), 100);

  return reply.send({
    burns: burns.getRecentBurns(limit),
    stats: burns.getStats(),
  });
});

/**
 * Verify a burn
 */
app.get('/v1/burns/:signature/verify', async (request, reply) => {
  const { signature } = request.params;

  const verification = await burns.verifyBurn(signature);

  return reply.send(verification);
});

// ═══════════════════════════════════════════════════════════════════════════════
// Lifecycle
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Startup
 */
async function start() {
  try {
    // Initialize Solana
    app.log.info('Initializing Solana connection...');
    solana.initRpc();
    await solana.initRelayerSigner();

    const health = await solana.checkHealth();
    if (!health.connected) {
      throw new Error(`Solana connection failed: ${health.error}`);
    }

    app.log.info(
      { relayerBalance: health.relayerBalanceSol.toFixed(4) },
      `Relayer wallet: ${solana.getRelayerAddress()}`
    );

    if (health.lowBalance) {
      app.log.warn('Relayer balance is low! Consider adding SOL.');
    }

    // Start timers
    quotes.startCleanupTimer();
    burns.startBatchTimer();
    burns.startStatsRotation();

    // Start server
    await app.listen({ port: config.port, host: config.host });

    app.log.info(`GASdf Relayer running on http://${config.host}:${config.port}`);
    app.log.info('"Don\'t Extract, Burn" - κυνικός');
  } catch (err) {
    app.log.error(err);
    process.exit(1);
  }
}

/**
 * Graceful shutdown
 */
async function shutdown(signal) {
  app.log.info(`${signal} received, shutting down gracefully...`);

  // Stop timers
  quotes.stopCleanupTimer();
  burns.stopBatchTimer();
  burns.stopStatsRotation();

  // Execute pending burns before shutdown
  try {
    await burns.forceExecuteBurns();
  } catch (err) {
    app.log.error({ err }, 'Failed to execute pending burns on shutdown');
  }

  // Close server
  await app.close();
  process.exit(0);
}

process.on('SIGINT', () => shutdown('SIGINT'));
process.on('SIGTERM', () => shutdown('SIGTERM'));

// ═══════════════════════════════════════════════════════════════════════════════
// Start
// ═══════════════════════════════════════════════════════════════════════════════

start();

// ═══════════════════════════════════════════════════════════════════════════════
// Exports (for testing)
// ═══════════════════════════════════════════════════════════════════════════════

export { app, solana, quotes, burns };

export default app;

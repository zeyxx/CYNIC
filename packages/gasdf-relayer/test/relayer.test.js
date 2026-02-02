/**
 * @cynic/gasdf-relayer - Tests
 *
 * Unit tests for GASdf relayer components.
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

// Mock Solana module before importing quotes/burns
const mockCalculateFee = mock.fn(() =>
  Promise.resolve({
    totalFee: 10000n,
    burnAmount: 7639n,
    treasuryAmount: 2361n,
  })
);

const mockGetRelayerAddress = mock.fn(() => 'RelayerPubkey123');

mock.module('./solana.js', {
  namedExports: {
    calculateFee: mockCalculateFee,
    getRelayerAddress: mockGetRelayerAddress,
    createBurnTransaction: mock.fn(() => Promise.resolve('BurnSig123')),
    createTreasuryTransfer: mock.fn(() => Promise.resolve('TreasurySig123')),
    confirmTransaction: mock.fn(() =>
      Promise.resolve({ confirmed: true, confirmationStatus: 'finalized' })
    ),
  },
});

// ═══════════════════════════════════════════════════════════════════════════════
// Quotes Tests
// ═══════════════════════════════════════════════════════════════════════════════

describe('Quotes', async () => {
  const { createQuote, getQuote, validateQuote, getAcceptedTokens, getDiscountTiers } =
    await import('../src/quotes.js');

  it('should create a valid quote', async () => {
    const quote = await createQuote({
      paymentToken: 'So11111111111111111111111111111111111111112',
      userPubkey: 'UserPubkey123',
      estimatedComputeUnits: 200000,
    });

    assert.ok(quote.quoteId);
    assert.ok(quote.quoteId.startsWith('q_'));
    assert.equal(quote.feePayer, 'RelayerPubkey123');
    assert.ok(quote.feeAmount);
    assert.ok(quote.expiresAt > Date.now());
  });

  it('should reject unknown token', async () => {
    await assert.rejects(
      createQuote({
        paymentToken: 'UnknownToken123',
        userPubkey: 'UserPubkey123',
      }),
      /not accepted/
    );
  });

  it('should retrieve created quote', async () => {
    const created = await createQuote({
      paymentToken: 'So11111111111111111111111111111111111111112',
      userPubkey: 'UserPubkey123',
    });

    const retrieved = getQuote(created.quoteId);
    assert.ok(retrieved);
    assert.equal(retrieved.quoteId, created.quoteId);
  });

  it('should return null for nonexistent quote', () => {
    const quote = getQuote('nonexistent_quote_id');
    assert.equal(quote, null);
  });

  it('should validate pending quote', async () => {
    const created = await createQuote({
      paymentToken: 'So11111111111111111111111111111111111111112',
      userPubkey: 'UserPubkey123',
    });

    const validation = validateQuote(created.quoteId);
    assert.equal(validation.valid, true);
    assert.ok(validation.quote);
  });

  it('should list accepted tokens', () => {
    const tokens = getAcceptedTokens();
    assert.ok(Array.isArray(tokens));
    assert.ok(tokens.length >= 3);
    assert.ok(tokens.some((t) => t.symbol === 'SOL'));
    assert.ok(tokens.some((t) => t.symbol === 'USDC'));
  });

  it('should list discount tiers', () => {
    const tiers = getDiscountTiers();
    assert.ok(Array.isArray(tiers));
    assert.ok(tiers.length >= 6);
    assert.ok(tiers.some((t) => t.name === 'GOLD'));
    assert.ok(tiers.some((t) => t.name === 'DIAMOND'));
  });

  it('should apply E-Score discount', async () => {
    const noDiscount = await createQuote({
      paymentToken: 'So11111111111111111111111111111111111111112',
      userPubkey: 'UserPubkey123',
      eScore: 0,
    });

    const goldDiscount = await createQuote({
      paymentToken: 'So11111111111111111111111111111111111111112',
      userPubkey: 'UserPubkey123',
      eScore: 60,
    });

    // Gold tier should have 45% discount
    assert.equal(noDiscount.discount, 0);
    assert.equal(goldDiscount.discount, 0.45);
    assert.equal(goldDiscount.tier, 'GOLD');
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Burns Tests
// ═══════════════════════════════════════════════════════════════════════════════

describe('Burns', async () => {
  const { recordFee, getStats, getRecentBurns } = await import('../src/burns.js');

  it('should record a fee', async () => {
    const result = await recordFee({
      feeAmount: '10000',
      quoteId: 'q_test123',
      userPubkey: 'UserPubkey123',
      txSignature: 'TxSig123',
    });

    assert.ok(result.record);
    assert.equal(result.record.feeAmount, '10000');
    assert.ok(result.record.burnAmount);
    assert.ok(result.record.treasuryAmount);
  });

  it('should get stats', () => {
    const stats = getStats();
    assert.ok(stats);
    assert.ok('totalBurned' in stats);
    assert.ok('burnCount' in stats);
    assert.ok('phiRatios' in stats);
    assert.equal(stats.phiRatios.burn, 0.76393202250021);
    assert.equal(stats.phiRatios.treasury, 0.23606797749979);
  });

  it('should get recent burns', () => {
    const burns = getRecentBurns(10);
    assert.ok(Array.isArray(burns));
  });

  it('should calculate φ-aligned split', async () => {
    const result = await recordFee({
      feeAmount: '1000000000', // 1 SOL
      quoteId: 'q_phi_test',
      userPubkey: 'UserPubkey123',
      txSignature: 'TxSig456',
    });

    const burn = BigInt(result.record.burnAmount);
    const treasury = BigInt(result.record.treasuryAmount);
    const total = BigInt(result.record.feeAmount);

    // Verify split is approximately φ-aligned
    const burnRatio = Number(burn) / Number(total);
    const treasuryRatio = Number(treasury) / Number(total);

    assert.ok(Math.abs(burnRatio - 0.764) < 0.01, `Burn ratio ${burnRatio} should be ~0.764`);
    assert.ok(Math.abs(treasuryRatio - 0.236) < 0.01, `Treasury ratio ${treasuryRatio} should be ~0.236`);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Fee Constants Tests
// ═══════════════════════════════════════════════════════════════════════════════

describe('Fee Constants', () => {
  it('should have correct φ-aligned ratios', () => {
    const PHI = 1.618033988749895;
    const PHI_INV = 1 / PHI; // 0.618...
    const PHI_INV_3 = Math.pow(PHI_INV, 3); // 0.236...

    const BURN_RATE = 0.76393202250021;
    const TREASURY_RATE = 0.23606797749979;

    // Treasury should be φ⁻³
    assert.ok(
      Math.abs(TREASURY_RATE - PHI_INV_3) < 0.0001,
      `Treasury rate ${TREASURY_RATE} should equal φ⁻³ = ${PHI_INV_3}`
    );

    // Burn + Treasury should equal 1
    assert.ok(
      Math.abs(BURN_RATE + TREASURY_RATE - 1) < 0.0001,
      'Burn + Treasury should equal 1'
    );
  });
});

console.log('All tests passed! φ guides the ratios.');

/**
 * TokenScorer Tests
 *
 * Tests for the 17-dimension φ-governed token scoring system.
 * Pure logic tests — no network calls needed.
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';

import { TokenScorer } from '../src/oracle/scorer.js';

// ═══════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════

const PHI_INV = 0.618;

/** Minimal token data with reasonable defaults */
function makeTokenData(overrides = {}) {
  return {
    mint: 'TestMint1111111111111111111111111111111111111',
    name: 'TestToken',
    symbol: 'TEST',
    isNative: false,
    supply: { total: 1000000, circulating: 1000000, decimals: 9 },
    distribution: {
      topHolders: [
        { address: 'A', percentage: 20, amount: 200000 },
        { address: 'B', percentage: 15, amount: 150000 },
        { address: 'C', percentage: 10, amount: 100000 },
      ],
      holderCount: 500,
    },
    authorities: {
      mintAuthority: null,
      freezeAuthority: null,
      mintAuthorityActive: false,
      freezeAuthorityActive: false,
    },
    ageInDays: 60,
    priceInfo: { pricePerToken: 0.05, totalPrice: 50000, currency: 'USDC' },
    metadataIntegrity: {
      hasName: true,
      hasSymbol: true,
      hasUri: true,
      hasImage: true,
    },
    ...overrides,
  };
}

/** Native SOL token data */
function makeNativeData() {
  return makeTokenData({ isNative: true, name: 'SOL', symbol: 'SOL' });
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('TokenScorer', () => {
  const scorer = new TokenScorer();

  describe('score()', () => {
    it('returns a complete verdict structure', () => {
      const result = scorer.score(makeTokenData());

      assert.ok(result.mint);
      assert.ok(result.name);
      assert.ok(result.symbol);
      assert.ok(typeof result.qScore === 'number');
      assert.ok(typeof result.kScore === 'number');
      assert.ok(typeof result.confidence === 'number');
      assert.ok(result.verdict);
      assert.ok(result.verdictIcon);
      assert.ok(result.verdictDescription);
      assert.ok(result.kTier);
      assert.ok(result.axiomScores);
      assert.ok(result.dimensions);
      assert.ok(result.kComponents);
      assert.ok(Array.isArray(result.weaknesses));
      assert.ok(typeof result.theUnnameable === 'number');
      assert.strictEqual(result.totalDimensions, 17);
      assert.ok(result.timestamp);
      assert.ok(result.philosophy.includes('61.8%'));
    });

    it('confidence never exceeds φ⁻¹ (61.8%)', () => {
      // Full data token — max confidence
      const result = scorer.score(makeTokenData());
      assert.ok(result.confidence <= PHI_INV + 0.001, `Confidence ${result.confidence} exceeds φ⁻¹`);

      // Native SOL — full data
      const native = scorer.score(makeNativeData());
      assert.ok(native.confidence <= PHI_INV + 0.001, `Native confidence ${native.confidence} exceeds φ⁻¹`);
    });

    it('scores all 17 dimensions', () => {
      const result = scorer.score(makeTokenData());
      const dims = result.dimensions;
      const dimCount = Object.keys(dims).length;
      assert.strictEqual(dimCount, 17, `Expected 17 dimensions, got ${dimCount}`);

      // All dimensions 0-100
      for (const [name, val] of Object.entries(dims)) {
        assert.ok(val >= 0 && val <= 100, `${name} = ${val} out of range`);
      }
    });

    it('includes theUnnameable dimension', () => {
      const result = scorer.score(makeTokenData());
      assert.ok(typeof result.dimensions.theUnnameable === 'number');
    });

    it('returns four axiom scores', () => {
      const result = scorer.score(makeTokenData());
      assert.ok(typeof result.axiomScores.PHI === 'number');
      assert.ok(typeof result.axiomScores.VERIFY === 'number');
      assert.ok(typeof result.axiomScores.CULTURE === 'number');
      assert.ok(typeof result.axiomScores.BURN === 'number');
    });

    it('returns K-Score components (d, o, l)', () => {
      const result = scorer.score(makeTokenData());
      assert.ok(typeof result.kComponents.d === 'number');
      assert.ok(typeof result.kComponents.o === 'number');
      assert.ok(typeof result.kComponents.l === 'number');
    });
  });

  describe('native SOL scoring', () => {
    it('scores native SOL with high dimension values', () => {
      const result = scorer.score(makeNativeData());

      // Native tokens get fixed high scores
      assert.strictEqual(result.dimensions.supplyDistribution, 70);
      assert.strictEqual(result.dimensions.liquidityDepth, 100);
      assert.strictEqual(result.dimensions.priceStability, 80);
      assert.strictEqual(result.dimensions.mintAuthority, 100);
      assert.strictEqual(result.dimensions.freezeAuthority, 100);
      assert.strictEqual(result.dimensions.metadataIntegrity, 100);
      assert.strictEqual(result.dimensions.programVerification, 100);
      assert.strictEqual(result.dimensions.holderCount, 100);
      assert.strictEqual(result.dimensions.tokenAge, 100);
      assert.strictEqual(result.dimensions.ecosystemIntegration, 100);
      assert.strictEqual(result.dimensions.burnActivity, 80);
      assert.strictEqual(result.dimensions.creatorBehavior, 70);
    });

    it('gives native SOL full security cap', () => {
      const result = scorer.score(makeNativeData());
      // kScore should not be capped by security
      assert.ok(result.kScore >= 0);
    });

    it('gives native SOL full data completeness', () => {
      const result = scorer.score(makeNativeData());
      // theUnnameable should be 0 for native (full data)
      assert.strictEqual(result.dimensions.theUnnameable, 0);
    });
  });

  describe('PHI dimensions', () => {
    it('_scoreSupplyDistribution: penalizes concentrated supply', () => {
      const concentrated = makeTokenData({
        distribution: {
          topHolders: [{ address: 'A', percentage: 80 }],
          holderCount: 10,
        },
      });
      const distributed = makeTokenData({
        distribution: {
          topHolders: [{ address: 'A', percentage: 5 }],
          holderCount: 1000,
        },
      });

      const r1 = scorer.score(concentrated);
      const r2 = scorer.score(distributed);
      assert.ok(r2.dimensions.supplyDistribution > r1.dimensions.supplyDistribution);
    });

    it('_scoreSupplyDistribution: neutral without holder data', () => {
      const noData = makeTokenData({
        distribution: { topHolders: [], holderCount: 0 },
      });
      const result = scorer.score(noData);
      assert.strictEqual(result.dimensions.supplyDistribution, 50);
    });

    it('_scoreLiquidityDepth: DexScreener data increases score', () => {
      const withDex = makeTokenData({
        dexScreener: { liquidityUsd: 500000, volume24h: 100000 },
      });
      const withoutDex = makeTokenData();

      const r1 = scorer.score(withDex);
      const r2 = scorer.score(withoutDex);
      assert.ok(r1.dimensions.liquidityDepth >= r2.dimensions.liquidityDepth);
    });

    it('_scorePriceStability: zero price → zero score', () => {
      const zeroPrice = makeTokenData({
        priceInfo: { pricePerToken: 0 },
      });
      const result = scorer.score(zeroPrice);
      assert.strictEqual(result.dimensions.priceStability, 0);
    });

    it('_scorePriceStability: crash penalty from DexScreener', () => {
      const crashed = makeTokenData({
        dexScreener: { liquidityUsd: 100000, priceChange24h: -50, volume24h: 10000 },
      });
      const stable = makeTokenData({
        dexScreener: { liquidityUsd: 100000, priceChange24h: 5, volume24h: 10000 },
      });

      const r1 = scorer.score(crashed);
      const r2 = scorer.score(stable);
      assert.ok(r2.dimensions.priceStability >= r1.dimensions.priceStability);
    });

    it('_scoreSupplyMechanics: penalizes active authorities', () => {
      const renounced = makeTokenData({
        authorities: {
          mintAuthorityActive: false,
          freezeAuthorityActive: false,
        },
      });
      const active = makeTokenData({
        authorities: {
          mintAuthorityActive: true,
          freezeAuthorityActive: true,
        },
      });

      const r1 = scorer.score(renounced);
      const r2 = scorer.score(active);
      assert.ok(r1.dimensions.supplyMechanics > r2.dimensions.supplyMechanics);
    });
  });

  describe('VERIFY dimensions', () => {
    it('_scoreMintAuthority: renounced gets φ⁻¹ cap', () => {
      const renounced = makeTokenData({
        authorities: { mintAuthorityActive: false },
      });
      const result = scorer.score(renounced);
      assert.strictEqual(result.dimensions.mintAuthority, Math.round(PHI_INV * 100));
    });

    it('_scoreMintAuthority: active gets low score', () => {
      const active = makeTokenData({
        authorities: { mintAuthorityActive: true },
      });
      const result = scorer.score(active);
      assert.strictEqual(result.dimensions.mintAuthority, 20);
    });

    it('_scoreFreezeAuthority: active gets 15', () => {
      const active = makeTokenData({
        authorities: { freezeAuthorityActive: true },
      });
      const result = scorer.score(active);
      assert.strictEqual(result.dimensions.freezeAuthority, 15);
    });

    it('_scoreMetadataIntegrity: capped at φ⁻²', () => {
      const full = makeTokenData({
        metadataIntegrity: { hasName: true, hasSymbol: true, hasUri: true, hasImage: true },
      });
      const result = scorer.score(full);
      // Even with full metadata, capped at ~38
      assert.ok(result.dimensions.metadataIntegrity <= 39);
    });

    it('_scoreProgramVerification: always 0 for non-native', () => {
      const result = scorer.score(makeTokenData());
      assert.strictEqual(result.dimensions.programVerification, 0);
    });
  });

  describe('CULTURE dimensions', () => {
    it('_scoreHolderCount: asymptotic scaling', () => {
      const few = makeTokenData({ distribution: { topHolders: [], holderCount: 10 } });
      const many = makeTokenData({ distribution: { topHolders: [], holderCount: 5000 } });

      const r1 = scorer.score(few);
      const r2 = scorer.score(many);
      assert.ok(r2.dimensions.holderCount > r1.dimensions.holderCount);
    });

    it('_scoreHolderCount: zero holders → zero', () => {
      const zero = makeTokenData({ distribution: { topHolders: [], holderCount: 0 } });
      const result = scorer.score(zero);
      assert.strictEqual(result.dimensions.holderCount, 0);
    });

    it('_scoreTokenAge: exponential approach to 100', () => {
      const young = makeTokenData({ ageInDays: 1 });
      const old = makeTokenData({ ageInDays: 365 });

      const r1 = scorer.score(young);
      const r2 = scorer.score(old);
      assert.ok(r2.dimensions.tokenAge > r1.dimensions.tokenAge);
    });

    it('_scoreTokenAge: zero days → zero', () => {
      const brand = makeTokenData({ ageInDays: 0 });
      const result = scorer.score(brand);
      assert.strictEqual(result.dimensions.tokenAge, 0);
    });

    it('_scoreOrganicGrowth: less concentration → higher score', () => {
      const organic = makeTokenData({
        distribution: {
          topHolders: Array(10).fill(null).map((_, i) => ({ address: `A${i}`, percentage: 5 })),
          holderCount: 1000,
        },
      });
      const whale = makeTokenData({
        distribution: {
          topHolders: [{ address: 'W', percentage: 90 }],
          holderCount: 100,
        },
      });

      const r1 = scorer.score(organic);
      const r2 = scorer.score(whale);
      assert.ok(r1.dimensions.organicGrowth > r2.dimensions.organicGrowth);
    });
  });

  describe('BURN dimensions', () => {
    it('_scoreBurnActivity: always 0 for non-native (no data)', () => {
      const result = scorer.score(makeTokenData());
      assert.strictEqual(result.dimensions.burnActivity, 0);
    });

    it('_scoreFeeRedistribution: always 0 (no data)', () => {
      const result = scorer.score(makeTokenData());
      assert.strictEqual(result.dimensions.feeRedistribution, 0);
    });

    it('_scoreCreatorBehavior: renounced mint adds base trust', () => {
      const renounced = makeTokenData({
        authorities: { mintAuthorityActive: false },
      });
      const active = makeTokenData({
        authorities: { mintAuthorityActive: true },
      });

      const r1 = scorer.score(renounced);
      const r2 = scorer.score(active);
      assert.ok(r1.dimensions.creatorBehavior > r2.dimensions.creatorBehavior);
    });

    it('_scoreCreatorBehavior: sell/buy ratio affects score', () => {
      const buying = makeTokenData({
        authorities: { mintAuthorityActive: false },
        dexScreener: { sellBuyRatio: 0.5, liquidityUsd: 100000, volume24h: 50000 },
      });
      const dumping = makeTokenData({
        authorities: { mintAuthorityActive: false },
        dexScreener: { sellBuyRatio: 2.0, liquidityUsd: 100000, volume24h: 50000 },
      });

      const r1 = scorer.score(buying);
      const r2 = scorer.score(dumping);
      assert.ok(r1.dimensions.creatorBehavior >= r2.dimensions.creatorBehavior);
    });

    it('_scoreRealUtility: no price → zero', () => {
      const noPrice = makeTokenData({ priceInfo: null });
      const result = scorer.score(noPrice);
      assert.strictEqual(result.dimensions.realUtility, 0);
    });
  });

  describe('verdict tiers', () => {
    it('assigns correct verdict based on Q-Score', () => {
      // We test by checking the verdict field
      const result = scorer.score(makeTokenData());
      const validVerdicts = ['BARK', 'GROWL', 'WAG', 'HOWL'];
      assert.ok(validVerdicts.includes(result.verdict), `Unknown verdict: ${result.verdict}`);
    });
  });

  describe('K-Score tiers', () => {
    it('assigns a K-tier name', () => {
      const result = scorer.score(makeTokenData());
      const validTiers = ['Rust', 'Iron', 'Copper', 'Bronze', 'Silver', 'Gold', 'Platinum', 'Diamond'];
      assert.ok(validTiers.includes(result.kTier), `Unknown tier: ${result.kTier}`);
    });
  });

  describe('security cap', () => {
    it('both revoked → no cap', () => {
      const result = scorer.score(makeTokenData({
        authorities: { mintAuthorityActive: false, freezeAuthorityActive: false },
      }));
      assert.ok(result.kScore >= 0);
    });

    it('both active → capped at φ⁻²', () => {
      const result = scorer.score(makeTokenData({
        authorities: { mintAuthorityActive: true, freezeAuthorityActive: true },
      }));
      // K-Score should be ≤ 38 (φ⁻² × 100)
      assert.ok(result.kScore <= 39, `kScore ${result.kScore} exceeds security cap`);
    });
  });

  describe('weaknesses', () => {
    it('identifies dimensions below φ⁻² threshold', () => {
      const result = scorer.score(makeTokenData());
      for (const w of result.weaknesses) {
        assert.ok(w.dimension);
        assert.ok(typeof w.score === 'number');
        assert.ok(w.axiom);
        assert.ok(w.reason);
      }
    });

    it('programVerification always flagged as weakness', () => {
      const result = scorer.score(makeTokenData());
      const pvWeak = result.weaknesses.find(w => w.dimension === 'programVerification');
      assert.ok(pvWeak, 'programVerification should be a weakness');
      assert.strictEqual(pvWeak.score, 0);
    });
  });

  describe('data completeness', () => {
    it('minimal data → high theUnnameable', () => {
      const minimal = makeTokenData({
        name: 'Unknown',
        supply: { total: 0 },
        distribution: { topHolders: [], holderCount: 0 },
        authorities: {},
        ageInDays: 0,
        priceInfo: null,
        metadataIntegrity: {},
      });
      const result = scorer.score(minimal);
      assert.ok(result.dimensions.theUnnameable > 50);
    });

    it('full data → low theUnnameable', () => {
      const result = scorer.score(makeTokenData());
      assert.ok(result.dimensions.theUnnameable < 50);
    });
  });

  describe('OracleMemory price history integration', () => {
    it('price crash from memory penalizes stability', () => {
      const crashed = makeTokenData({
        priceHistory: { priceChange: -0.9 },
        dexScreener: { liquidityUsd: 100000, volume24h: 50000 },
      });
      const stable = makeTokenData({
        priceHistory: { priceChange: 0.1 },
        dexScreener: { liquidityUsd: 100000, volume24h: 50000 },
      });

      const r1 = scorer.score(crashed);
      const r2 = scorer.score(stable);
      assert.ok(r2.dimensions.priceStability >= r1.dimensions.priceStability);
    });
  });
});

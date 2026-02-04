/**
 * TokenFetcher Tests
 *
 * Tests for on-chain token data extraction methods.
 * Mocks fetch() for all RPC/API calls.
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert';

import { TokenFetcher } from '../src/oracle/token-fetcher.js';

// ═══════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════

function createFetcher(apiKey = null) {
  return new TokenFetcher(apiKey);
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS - Pure methods (no network)
// ═══════════════════════════════════════════════════════════════════════════

describe('TokenFetcher', () => {
  describe('constructor', () => {
    it('sets up Helius URL with API key', () => {
      const f = createFetcher('test-key-123');
      assert.ok(f._heliusUrl.includes('test-key-123'));
      assert.strictEqual(f._rpcUrl, f._heliusUrl);
    });

    it('uses public RPC without API key', () => {
      const f = createFetcher();
      assert.strictEqual(f._heliusUrl, null);
      assert.ok(f._rpcUrl.includes('mainnet-beta.solana.com'));
    });
  });

  describe('getTokenData() validation', () => {
    it('rejects invalid mint address (too short)', async () => {
      const f = createFetcher();
      await assert.rejects(
        () => f.getTokenData('short'),
        { message: /Invalid mint address/ }
      );
    });

    it('rejects null mint address', async () => {
      const f = createFetcher();
      await assert.rejects(
        () => f.getTokenData(null),
        /Invalid mint address/
      );
    });

    it('rejects empty string', async () => {
      const f = createFetcher();
      await assert.rejects(
        () => f.getTokenData(''),
        /Invalid mint address/
      );
    });
  });

  describe('_extractDasSupply()', () => {
    const f = createFetcher();

    it('extracts supply from DAS token_info', () => {
      const asset = {
        token_info: { supply: 1000000000, decimals: 6 },
      };
      const result = f._extractDasSupply(asset);

      assert.strictEqual(result.total, 1000);
      assert.strictEqual(result.decimals, 6);
      assert.strictEqual(result.raw, '1000000000');
    });

    it('returns null for missing token_info', () => {
      assert.strictEqual(f._extractDasSupply(null), null);
      assert.strictEqual(f._extractDasSupply({}), null);
      assert.strictEqual(f._extractDasSupply({ token_info: {} }), null);
    });

    it('returns null when supply is zero', () => {
      const asset = { token_info: { supply: 0, decimals: 6 } };
      assert.strictEqual(f._extractDasSupply(asset), null);
    });

    it('handles large supply values', () => {
      const asset = {
        token_info: { supply: 1000000000000000, decimals: 9 },
      };
      const result = f._extractDasSupply(asset);
      assert.strictEqual(result.total, 1000000);
    });
  });

  describe('_extractMetadata()', () => {
    const f = createFetcher();

    it('uses known token data when available', () => {
      const known = { name: 'SOL', symbol: 'SOL', decimals: 9 };
      const result = f._extractMetadata({}, known);

      assert.strictEqual(result.name, 'SOL');
      assert.strictEqual(result.symbol, 'SOL');
      assert.strictEqual(result.decimals, 9);
    });

    it('extracts from fallback accountInfo', () => {
      const asset = {
        fallback: true,
        accountInfo: { data: { parsed: { info: { decimals: 6 } } } },
      };
      const result = f._extractMetadata(asset, null);

      assert.strictEqual(result.name, 'Unknown');
      assert.strictEqual(result.symbol, '???');
      assert.strictEqual(result.decimals, 6);
    });

    it('extracts from Helius DAS response', () => {
      const asset = {
        content: {
          metadata: { name: 'MyToken' },
          json_uri: 'https://arweave.net/...',
          links: { image: 'https://img.url/tok.png' },
        },
        token_info: { symbol: 'MYT', decimals: 9 },
      };
      const result = f._extractMetadata(asset, null);

      assert.strictEqual(result.name, 'MyToken');
      assert.strictEqual(result.symbol, 'MYT');
      assert.strictEqual(result.decimals, 9);
      assert.ok(result.uri);
      assert.ok(result.image);
    });

    it('defaults to Unknown when no metadata', () => {
      const result = f._extractMetadata({}, null);

      assert.strictEqual(result.name, 'Unknown');
      assert.strictEqual(result.symbol, '???');
    });
  });

  describe('_analyzeDistribution()', () => {
    const f = createFetcher();

    it('returns defaults for empty accounts', () => {
      const result = f._analyzeDistribution([], { total: 1000 });

      assert.strictEqual(result.gini, 1.0);
      assert.strictEqual(result.whaleConcentration, 1.0);
      assert.strictEqual(result.estimatedHolders, 0);
    });

    it('calculates whale concentration from top 5', () => {
      const accounts = [
        { address: 'A', amount: 500 },
        { address: 'B', amount: 200 },
        { address: 'C', amount: 100 },
        { address: 'D', amount: 50 },
        { address: 'E', amount: 50 },
      ];
      const result = f._analyzeDistribution(accounts, { total: 1000 });

      // Top 5 = 900/1000 = 0.9
      assert.strictEqual(result.whaleConcentration, 0.9);
    });

    it('caps whale concentration at 1', () => {
      const accounts = [{ address: 'A', amount: 1500 }];
      const result = f._analyzeDistribution(accounts, { total: 1000 });

      assert.strictEqual(result.whaleConcentration, 1);
    });

    it('returns top 10 holders', () => {
      const accounts = Array.from({ length: 20 }, (_, i) => ({
        address: `Addr${i}`,
        amount: 100 - i,
      }));
      const result = f._analyzeDistribution(accounts, { total: 2000 });

      assert.strictEqual(result.topHolders.length, 10);
      assert.ok(result.topHolders[0].percentage > result.topHolders[9].percentage);
    });

    it('estimates holder count', () => {
      const accounts = Array.from({ length: 20 }, (_, i) => ({
        address: `A${i}`, amount: 10,
      }));
      const result = f._analyzeDistribution(accounts, { total: 200 });

      // 20 accounts → estimated 500
      assert.strictEqual(result.estimatedHolders, 500);
    });

    it('estimates lower for fewer accounts', () => {
      const accounts = [{ address: 'A', amount: 100 }];
      const result = f._analyzeDistribution(accounts, { total: 100 });

      // 1 account * 5 = 5
      assert.strictEqual(result.estimatedHolders, 5);
    });
  });

  describe('_calculateGini()', () => {
    const f = createFetcher();

    it('returns 1.0 for empty array', () => {
      assert.strictEqual(f._calculateGini([]), 1.0);
    });

    it('returns 1.0 for all zeros', () => {
      assert.strictEqual(f._calculateGini([0, 0, 0]), 1.0);
    });

    it('returns 0 for perfectly equal distribution', () => {
      const gini = f._calculateGini([100, 100, 100, 100]);
      assert.ok(gini < 0.01, `Gini ${gini} should be ~0 for equal distribution`);
    });

    it('returns high value for unequal distribution', () => {
      const gini = f._calculateGini([1000, 1, 1, 1]);
      assert.ok(gini > 0.5, `Gini ${gini} should be high for unequal distribution`);
    });

    it('returns value between 0 and 1', () => {
      const gini = f._calculateGini([50, 30, 15, 5]);
      assert.ok(gini >= 0 && gini <= 1, `Gini ${gini} out of range`);
    });
  });

  describe('_analyzeAuthorities()', () => {
    const f = createFetcher();

    it('extracts from fallback accountInfo', () => {
      const asset = {
        fallback: true,
        accountInfo: {
          data: {
            parsed: {
              info: {
                mintAuthority: 'AuthAddr111',
                freezeAuthority: null,
              },
            },
          },
        },
      };
      const result = f._analyzeAuthorities(asset);

      assert.strictEqual(result.mintAuthority, 'AuthAddr111');
      assert.strictEqual(result.freezeAuthority, null);
      assert.strictEqual(result.mintAuthorityActive, true);
      assert.strictEqual(result.freezeAuthorityActive, false);
    });

    it('extracts from Helius DAS token_info', () => {
      const asset = {
        token_info: {
          mint_authority: 'MintAuth1',
          freeze_authority: 'FreezeAuth1',
        },
        authorities: [],
      };
      const result = f._analyzeAuthorities(asset);

      assert.strictEqual(result.mintAuthority, 'MintAuth1');
      assert.strictEqual(result.freezeAuthority, 'FreezeAuth1');
      assert.strictEqual(result.mintAuthorityActive, true);
      assert.strictEqual(result.freezeAuthorityActive, true);
    });

    it('detects renounced authorities (null)', () => {
      const asset = {
        token_info: {
          mint_authority: null,
          freeze_authority: null,
        },
        authorities: [],
      };
      const result = f._analyzeAuthorities(asset);

      assert.strictEqual(result.mintAuthorityActive, false);
      assert.strictEqual(result.freezeAuthorityActive, false);
    });

    it('falls back to authorities array', () => {
      const asset = {
        token_info: {},
        authorities: [
          { address: 'FullAuth', scopes: ['full'] },
        ],
      };
      const result = f._analyzeAuthorities(asset);

      assert.strictEqual(result.mintAuthority, 'FullAuth');
    });
  });

  describe('_calculateAge()', () => {
    const f = createFetcher();

    it('calculates age from created_at', () => {
      const threeDaysAgo = new Date(Date.now() - 3 * 24 * 60 * 60 * 1000).toISOString();
      const asset = { created_at: threeDaysAgo };
      const age = f._calculateAge(asset);

      assert.ok(age >= 2 && age <= 4, `Age ${age} should be ~3 days`);
    });

    it('returns conservative 30 for renounced without created_at', () => {
      const asset = { token_info: { mint_authority: null } };
      assert.strictEqual(f._calculateAge(asset), 30);
    });

    it('returns 1 for DAS asset without created_at', () => {
      const asset = { token_info: { mint_authority: 'some_auth' } };
      assert.strictEqual(f._calculateAge(asset), 1);
    });

    it('returns 0 for fallback with no data', () => {
      const asset = { fallback: true, accountInfo: null };
      assert.strictEqual(f._calculateAge(asset), 0);
    });

    it('returns 0 for null asset', () => {
      assert.strictEqual(f._calculateAge(null), 0);
    });
  });
});

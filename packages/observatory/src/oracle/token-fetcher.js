/**
 * TokenFetcher - Fetches on-chain Solana token data
 *
 * Sources: Helius DAS API (getAsset + getTokenAccounts) + Standard Solana RPC
 * "Don't trust, verify" — every data point is on-chain
 *
 * @module @cynic/observatory/oracle/token-fetcher
 */

'use strict';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

const HELIUS_RPC_BASE = 'https://mainnet.helius-rpc.com';
const PUBLIC_RPC = 'https://api.mainnet-beta.solana.com';
const DEXSCREENER_API = 'https://api.dexscreener.com/tokens/v1/solana';
const REQUEST_TIMEOUT_MS = 15000;

// Known DEX program IDs — accounts owned by these are pools, not holders
const DEX_PROGRAMS = new Set([
  '675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8', // Raydium V4
  'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK', // Raydium CLMM
  'CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C', // Raydium CP
  'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc',  // Orca Whirlpool
  'LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo',  // Meteora DLMM
  'srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX',   // OpenBook/Serum
  '6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P',   // PumpFun
  'PSwapMdSai8tjrEXcxFeQth87xC4rRsa4VA5mhGhXkP',   // PumpSwap
  '5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1',  // Raydium Authority V4
]);

// Known infrastructure tokens
const KNOWN_TOKENS = {
  'So11111111111111111111111111111111': { name: 'SOL', symbol: 'SOL', decimals: 9, isNative: true },
  'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v': { name: 'USD Coin', symbol: 'USDC', decimals: 6 },
  'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB': { name: 'Tether USD', symbol: 'USDT', decimals: 6 },
};

// Max pages to paginate for holder count (5 pages = 5000 accounts, covers 99%+ of tokens)
const MAX_HOLDER_PAGES = 5;

// ═══════════════════════════════════════════════════════════════════════════
// TOKEN FETCHER
// ═══════════════════════════════════════════════════════════════════════════

export class TokenFetcher {
  constructor(heliusApiKey) {
    this._heliusApiKey = heliusApiKey;
    this._heliusUrl = heliusApiKey
      ? `${HELIUS_RPC_BASE}/?api-key=${heliusApiKey}`
      : null;
    this._rpcUrl = this._heliusUrl || PUBLIC_RPC;
  }

  /**
   * Fetch all available on-chain data for a token mint
   * @param {string} mintAddress - Solana token mint address
   * @returns {Promise<Object>} Token data with all measurable dimensions
   */
  async getTokenData(mintAddress) {
    const startTime = Date.now();

    if (typeof mintAddress !== 'string' || mintAddress.length < 32 || mintAddress.length > 44) {
      throw new Error(`Invalid mint address: ${mintAddress}`);
    }

    const known = KNOWN_TOKENS[mintAddress];
    const isNative = !!known?.isNative;

    // Fetch data in parallel — all on-chain, all verifiable
    const [asset, supply, largestAccounts, holders] = await Promise.all([
      this._getAsset(mintAddress),
      isNative ? Promise.resolve({ total: 0, circulating: 0, decimals: 9, raw: '0' }) : this._getTokenSupply(mintAddress),
      isNative ? Promise.resolve([]) : this._getTokenLargestAccounts(mintAddress),
      isNative ? Promise.resolve({ count: null, source: 'native' }) : this._getHolderCount(mintAddress),
    ]);

    // Extract metadata (prefer DAS token_info over fallback)
    const metadata = this._extractMetadata(asset, known);

    // Use DAS token_info supply if available (more accurate)
    const dasSupply = this._extractDasSupply(asset);
    const finalSupply = dasSupply || supply;

    // Analyze distribution from largest accounts
    const distribution = this._analyzeDistribution(largestAccounts, finalSupply);

    // Override estimated holder count with real paginated data
    if (holders.count !== null) {
      distribution.estimatedHolders = holders.count;
      distribution.holderCountSource = holders.source;
    }

    // Authorities
    const authorities = this._analyzeAuthorities(asset);

    // Token age
    const ageInDays = this._calculateAge(asset);

    // Price info from DAS (available when showFungible: true)
    const priceInfo = asset?.token_info?.price_info || null;

    const latencyMs = Date.now() - startTime;

    return {
      mint: mintAddress,
      name: metadata.name,
      symbol: metadata.symbol,
      decimals: metadata.decimals,
      uri: metadata.uri,
      isNative,

      supply: {
        total: finalSupply.total,
        circulating: finalSupply.circulating,
        decimals: finalSupply.decimals,
      },

      distribution: {
        topHolders: distribution.topHolders,
        giniCoefficient: distribution.gini,
        whaleConcentration: distribution.whaleConcentration,
        holderCount: distribution.estimatedHolders,
      },

      authorities: {
        mintAuthority: authorities.mintAuthority,
        freezeAuthority: authorities.freezeAuthority,
        mintAuthorityActive: authorities.mintAuthorityActive,
        freezeAuthorityActive: authorities.freezeAuthorityActive,
      },

      ageInDays,

      priceInfo: priceInfo ? {
        pricePerToken: priceInfo.price_per_token,
        totalPrice: priceInfo.total_price,
        currency: priceInfo.currency,
      } : null,

      metadataIntegrity: {
        hasUri: !!metadata.uri,
        hasName: !!metadata.name && metadata.name !== 'Unknown',
        hasSymbol: !!metadata.symbol && metadata.symbol !== '???',
        hasImage: !!metadata.image,
      },

      _raw: {
        latencyMs,
        source: this._heliusApiKey ? 'helius_das' : 'public_rpc',
        holderCountSource: distribution.holderCountSource || 'estimated',
        hasPriceData: !!priceInfo,
        fetchedAt: new Date().toISOString(),
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // RPC CALLS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Helius DAS: getAsset with showFungible: true
   * Returns token_info (supply, decimals, price_info) + metadata + authorities
   */
  async _getAsset(mint) {
    if (this._heliusUrl) {
      try {
        return await this._rpcCall(this._heliusUrl, 'getAsset', {
          id: mint,
          displayOptions: { showFungible: true },
        });
      } catch {
        // Fallback to basic RPC
      }
    }

    // Fallback: basic account info from public RPC
    try {
      const accountInfo = await this._rpcCall(this._rpcUrl, 'getAccountInfo', [
        mint,
        { encoding: 'jsonParsed' },
      ]);
      return { fallback: true, accountInfo: accountInfo?.value || null };
    } catch {
      return { fallback: true, accountInfo: null };
    }
  }

  /**
   * RPC: getTokenSupply — total and circulating supply (fallback if DAS unavailable)
   */
  async _getTokenSupply(mint) {
    try {
      const result = await this._rpcCall(this._rpcUrl, 'getTokenSupply', [mint]);
      const value = result?.value;
      return {
        total: parseFloat(value?.uiAmountString || '0'),
        circulating: parseFloat(value?.uiAmountString || '0'),
        decimals: value?.decimals || 0,
        raw: value?.amount || '0',
      };
    } catch {
      return { total: 0, circulating: 0, decimals: 0, raw: '0' };
    }
  }

  /**
   * RPC: getTokenLargestAccounts — top 20 holder accounts for whale analysis
   */
  async _getTokenLargestAccounts(mint) {
    try {
      const result = await this._rpcCall(this._rpcUrl, 'getTokenLargestAccounts', [mint]);
      return (result?.value || []).map(a => ({
        address: a.address,
        amount: parseFloat(a.uiAmountString || '0'),
        rawAmount: a.amount,
      }));
    } catch {
      return [];
    }
  }

  /**
   * Helius DAS: getTokenAccounts — real holder count via pagination
   * Paginates up to MAX_HOLDER_PAGES (5000 accounts) and deduplicates by owner
   */
  async _getHolderCount(mint) {
    if (!this._heliusUrl) return { count: null, source: 'no_helius' };

    try {
      const owners = new Set();
      let page = 1;

      while (page <= MAX_HOLDER_PAGES) {
        const result = await this._rpcCall(this._heliusUrl, 'getTokenAccounts', {
          mint,
          page,
          limit: 1000,
        });

        const accounts = result?.token_accounts || [];
        if (accounts.length === 0) break;

        for (const acc of accounts) {
          // Filter out DEX pool accounts
          if (acc.owner && !DEX_PROGRAMS.has(acc.owner)) {
            owners.add(acc.owner);
          }
        }

        // If less than 1000 returned, we've seen all accounts
        if (accounts.length < 1000) break;
        page++;
      }

      return {
        count: owners.size,
        source: page > MAX_HOLDER_PAGES ? 'helius_paginated_capped' : 'helius_paginated_complete',
      };
    } catch {
      // Fallback: try single page for at least a total count
      try {
        const result = await this._rpcCall(this._heliusUrl, 'getTokenAccounts', {
          mint,
          page: 1,
          limit: 1,
        });
        if (typeof result?.total === 'number') {
          return { count: result.total, source: 'helius_total_field' };
        }
      } catch { /* ignore */ }
      return { count: null, source: 'helius_error' };
    }
  }

  /**
   * Generic JSON-RPC call with timeout
   */
  async _rpcCall(url, method, params) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

    try {
      const response = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: `cynic-${Date.now()}`,
          method,
          params: Array.isArray(params) ? params : params,
        }),
        signal: controller.signal,
      });

      const json = await response.json();
      if (json.error) {
        throw new Error(`RPC error: ${json.error.message || JSON.stringify(json.error)}`);
      }
      return json.result;
    } finally {
      clearTimeout(timeout);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // DATA EXTRACTION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Extract supply from DAS token_info (more accurate than getTokenSupply for some tokens)
   */
  _extractDasSupply(asset) {
    const ti = asset?.token_info;
    if (!ti || !ti.supply || !ti.decimals) return null;
    const total = ti.supply / Math.pow(10, ti.decimals);
    return {
      total,
      circulating: total, // DAS doesn't distinguish circulating
      decimals: ti.decimals,
      raw: String(ti.supply),
    };
  }

  _extractMetadata(asset, known) {
    if (known) {
      return { name: known.name, symbol: known.symbol, decimals: known.decimals, uri: null, image: null };
    }

    if (asset?.fallback) {
      const parsed = asset.accountInfo?.data?.parsed?.info;
      return {
        name: 'Unknown',
        symbol: '???',
        decimals: parsed?.decimals || 0,
        uri: null,
        image: null,
      };
    }

    // Helius DAS response — prefer token_info for symbol/decimals
    const content = asset?.content || {};
    const metadata = content?.metadata || {};
    const tokenInfo = asset?.token_info || {};

    return {
      name: metadata.name || content.name || 'Unknown',
      symbol: tokenInfo.symbol || metadata.symbol || content.symbol || '???',
      decimals: tokenInfo.decimals || 0,
      uri: content.json_uri || null,
      image: content.links?.image || content.files?.[0]?.uri || null,
    };
  }

  _analyzeDistribution(largestAccounts, supply) {
    if (largestAccounts.length === 0) {
      return {
        topHolders: [],
        gini: 1.0,
        whaleConcentration: 1.0,
        estimatedHolders: 0,
      };
    }

    const totalSupply = supply.total || 1;

    // Filter out likely DEX pool accounts from largest holders
    // (getTokenLargestAccounts returns token account addresses, not owners,
    //  so we can't filter by DEX_PROGRAMS here — but the amounts still work for distribution)
    const topHolders = largestAccounts.slice(0, 10).map(a => ({
      address: a.address,
      amount: a.amount,
      percentage: (a.amount / totalSupply) * 100,
    }));

    // Whale concentration: top 5 holders as % of supply
    const top5Amount = largestAccounts.slice(0, 5).reduce((s, a) => s + a.amount, 0);
    const whaleConcentration = Math.min(1, top5Amount / totalSupply);

    // Gini coefficient from visible holders
    const gini = this._calculateGini(largestAccounts.map(a => a.amount));

    // Fallback holder estimate (overridden by real count from _getHolderCount)
    const estimatedHolders = largestAccounts.length >= 20 ? 500 : largestAccounts.length * 5;

    return { topHolders, gini, whaleConcentration, estimatedHolders, holderCountSource: 'estimated' };
  }

  _calculateGini(amounts) {
    if (amounts.length === 0) return 1.0;
    const sorted = [...amounts].sort((a, b) => a - b);
    const n = sorted.length;
    const total = sorted.reduce((s, v) => s + v, 0);
    if (total === 0) return 1.0;

    let sumOfDifferences = 0;
    for (let i = 0; i < n; i++) {
      sumOfDifferences += (2 * (i + 1) - n - 1) * sorted[i];
    }
    return Math.min(1, Math.max(0, sumOfDifferences / (n * total)));
  }

  _analyzeAuthorities(asset) {
    if (asset?.fallback) {
      const parsed = asset.accountInfo?.data?.parsed?.info;
      return {
        mintAuthority: parsed?.mintAuthority || null,
        freezeAuthority: parsed?.freezeAuthority || null,
        mintAuthorityActive: !!parsed?.mintAuthority,
        freezeAuthorityActive: !!parsed?.freezeAuthority,
      };
    }

    // Helius DAS — token_info has direct mint_authority/freeze_authority fields
    const tokenInfo = asset?.token_info || {};
    const authorities = asset?.authorities || [];

    // Prefer token_info fields (more reliable for SPL tokens)
    const mintAuth = tokenInfo.mint_authority || authorities.find(a => a.scopes?.includes('full'))?.address || null;
    const freezeAuth = tokenInfo.freeze_authority || null;

    return {
      mintAuthority: mintAuth,
      freezeAuthority: freezeAuth,
      mintAuthorityActive: !!mintAuth,
      freezeAuthorityActive: !!freezeAuth,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // DEXSCREENER MARKET DATA (free, no API key)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Fetch market data from DexScreener: real liquidity, volume, 24h change.
   * Completely free, no API key required.
   * Returns the pair with highest liquidity for the given mint.
   * @param {string} mint
   * @returns {Object|null} { priceUsd, priceChange24h, liquidityUsd, volume24h, fdv, marketCap, sellBuyRatio }
   */
  async getDexScreenerData(mint) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
    try {
      const url = `${DEXSCREENER_API}/${mint}`;
      const response = await fetch(url, {
        headers: { 'accept': 'application/json' },
        signal: controller.signal,
      });
      const pairs = await response.json();
      if (!Array.isArray(pairs) || pairs.length === 0) return null;

      // Pick the pair with highest liquidity
      const best = pairs.reduce((a, b) =>
        (b.liquidity?.usd || 0) > (a.liquidity?.usd || 0) ? b : a
      );

      const buys24h = best.txns?.h24?.buys || 0;
      const sells24h = best.txns?.h24?.sells || 0;
      const totalTxns = buys24h + sells24h;

      return {
        priceUsd: parseFloat(best.priceUsd) || null,
        priceChange24h: best.priceChange?.h24 ?? null,
        liquidityUsd: best.liquidity?.usd || 0,
        volume24h: best.volume?.h24 || 0,
        fdv: best.fdv || 0,
        marketCap: best.marketCap || 0,
        buys24h,
        sells24h,
        sellBuyRatio: totalTxns > 0 ? sells24h / Math.max(1, buys24h) : null,
        pairCreatedAt: best.pairCreatedAt || null,
        dexId: best.dexId || null,
        source: 'dexscreener',
      };
    } catch {
      return null;
    } finally {
      clearTimeout(timeout);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // DATA EXTRACTION
  // ═══════════════════════════════════════════════════════════════════════════

  _calculateAge(asset) {
    // 1. DAS created_at (most reliable)
    const created = asset?.created_at || asset?.content?.metadata?.created_at;
    if (created) {
      const ageMs = Date.now() - new Date(created).getTime();
      const days = Math.max(0, Math.floor(ageMs / (24 * 60 * 60 * 1000)));
      if (days > 0) return days;
    }

    // 2. If mint authority is null (renounced) and DAS returned data → likely older token
    const tokenInfo = asset?.token_info || {};
    if (tokenInfo.mint_authority === null || tokenInfo.mint_authority === undefined) {
      if (asset && !asset.fallback) return 30; // Conservative: at least 30 days
    }

    // 3. DAS returned data → token exists on-chain
    if (asset && !asset.fallback) return 1;

    return 0;
  }
}

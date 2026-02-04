/**
 * TokenFetcher - Fetches on-chain Solana token data
 *
 * Sources: Helius DAS API + Standard Solana RPC
 * "Don't trust, verify" — every data point is on-chain
 *
 * @module @cynic/observatory/oracle/token-fetcher
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

const HELIUS_RPC_BASE = 'https://mainnet.helius-rpc.com';
const PUBLIC_RPC = 'https://api.mainnet-beta.solana.com';
const REQUEST_TIMEOUT_MS = 15000;

// Known infrastructure tokens (don't need full analysis)
const KNOWN_TOKENS = {
  'So11111111111111111111111111111111': { name: 'SOL', symbol: 'SOL', decimals: 9, isNative: true },
  'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v': { name: 'USD Coin', symbol: 'USDC', decimals: 6 },
  'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB': { name: 'Tether USD', symbol: 'USDT', decimals: 6 },
};

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

    // Validate mint address format (base58, 32-44 chars)
    if (typeof mintAddress !== 'string' || mintAddress.length < 32 || mintAddress.length > 44) {
      throw new Error(`Invalid mint address: ${mintAddress}`);
    }

    // Check known tokens first
    const known = KNOWN_TOKENS[mintAddress];

    // Fetch data in parallel — all on-chain, all verifiable
    const [asset, supply, largestAccounts] = await Promise.all([
      this._getAsset(mintAddress),
      this._getTokenSupply(mintAddress),
      this._getTokenLargestAccounts(mintAddress),
    ]);

    // Extract metadata
    const metadata = this._extractMetadata(asset, known);

    // Calculate holder distribution metrics
    const distribution = this._analyzeDistribution(largestAccounts, supply);

    // Analyze authorities (mint, freeze)
    const authorities = this._analyzeAuthorities(asset);

    // Calculate token age
    const ageInDays = this._calculateAge(asset);

    const latencyMs = Date.now() - startTime;

    return {
      mint: mintAddress,
      name: metadata.name,
      symbol: metadata.symbol,
      decimals: metadata.decimals,
      uri: metadata.uri,
      isNative: !!known?.isNative,

      // Supply data
      supply: {
        total: supply.total,
        circulating: supply.circulating,
        decimals: supply.decimals,
      },

      // Holder distribution (PHI dimension: harmony)
      distribution: {
        topHolders: distribution.topHolders,
        giniCoefficient: distribution.gini,
        whaleConcentration: distribution.whaleConcentration,
        holderCount: distribution.estimatedHolders,
      },

      // Authorities (VERIFY dimension: truth)
      authorities: {
        mintAuthority: authorities.mintAuthority,
        freezeAuthority: authorities.freezeAuthority,
        mintAuthorityActive: authorities.mintAuthorityActive,
        freezeAuthorityActive: authorities.freezeAuthorityActive,
      },

      // Token age (CULTURE dimension: memory)
      ageInDays,

      // Metadata integrity (VERIFY dimension)
      metadataIntegrity: {
        hasUri: !!metadata.uri,
        hasName: !!metadata.name && metadata.name !== 'Unknown',
        hasSymbol: !!metadata.symbol && metadata.symbol !== '???',
        hasImage: !!metadata.image,
      },

      // Fetch metadata
      _raw: {
        latencyMs,
        source: this._heliusApiKey ? 'helius' : 'public_rpc',
        fetchedAt: new Date().toISOString(),
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // RPC CALLS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Helius DAS: getAsset — token metadata, authorities, ownership
   */
  async _getAsset(mint) {
    if (this._heliusUrl) {
      try {
        return await this._rpcCall(this._heliusUrl, 'getAsset', { id: mint });
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
   * RPC: getTokenSupply — total and circulating supply
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
   * RPC: getTokenLargestAccounts — whale analysis
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
   * Generic RPC call with timeout
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

    // Helius DAS response
    const content = asset?.content || {};
    const metadata = content?.metadata || {};

    return {
      name: metadata.name || content.name || 'Unknown',
      symbol: metadata.symbol || content.symbol || '???',
      decimals: asset?.token_info?.decimals || 0,
      uri: content.json_uri || null,
      image: content.links?.image || content.files?.[0]?.uri || null,
    };
  }

  _analyzeDistribution(largestAccounts, supply) {
    if (largestAccounts.length === 0) {
      return {
        topHolders: [],
        gini: 1.0, // Maximum inequality (no data = assume worst)
        whaleConcentration: 1.0,
        estimatedHolders: 0,
      };
    }

    const totalSupply = supply.total || 1;
    const topHolders = largestAccounts.slice(0, 10).map(a => ({
      address: a.address,
      amount: a.amount,
      percentage: (a.amount / totalSupply) * 100,
    }));

    // Whale concentration: % held by top 5
    const top5Amount = largestAccounts.slice(0, 5).reduce((s, a) => s + a.amount, 0);
    const whaleConcentration = Math.min(1, top5Amount / totalSupply);

    // Simplified Gini coefficient from top holders
    const gini = this._calculateGini(largestAccounts.map(a => a.amount));

    // Estimate total holders (top 20 accounts visible, extrapolate)
    const estimatedHolders = largestAccounts.length >= 20 ? 1000 : largestAccounts.length * 10;

    return { topHolders, gini, whaleConcentration, estimatedHolders };
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

    // Helius DAS
    const authorities = asset?.authorities || [];
    const mintAuth = authorities.find(a => a.scopes?.includes('full') || a.scopes?.includes('mint'));
    const freezeAuth = asset?.token_info?.freeze_authority;

    return {
      mintAuthority: mintAuth?.address || null,
      freezeAuthority: freezeAuth || null,
      mintAuthorityActive: !!mintAuth,
      freezeAuthorityActive: !!freezeAuth,
    };
  }

  _calculateAge(asset) {
    // From DAS metadata if available
    const created = asset?.created_at || asset?.content?.metadata?.created_at;
    if (created) {
      const ageMs = Date.now() - new Date(created).getTime();
      return Math.max(0, Math.floor(ageMs / (24 * 60 * 60 * 1000)));
    }
    // Unknown age — return 0 (will lower CULTURE score)
    return 0;
  }
}

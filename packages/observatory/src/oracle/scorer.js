/**
 * TokenScorer - 17-dimension φ-governed token judgment
 *
 * "φ distrusts φ" — Every score capped at 61.8% confidence
 *
 * 4 Axioms × 4 Dimensions + THE_UNNAMEABLE = 17 honest dimensions
 * Each dimension is traceable to an on-chain data source.
 * No dimension is invented. If we can't measure it, we don't claim it.
 *
 * @module @cynic/observatory/oracle/scorer
 */

'use strict';

import {
  PHI, PHI_INV, PHI_INV_2, PHI_INV_3,
  calculateKScore,
} from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * K-Score Tiers (token quality rating)
 * φ-aligned thresholds
 */
/**
 * Verdict tiers — driven by Q-Score (the unified judgment)
 * K-Score is a supplementary metric, not the verdict driver.
 */
const VERDICT_TIERS = Object.freeze([
  { name: 'BARK', min: 0, threshold: PHI_INV_3 * 100, icon: '🔴', description: 'High risk — proceed with extreme caution' },
  { name: 'GROWL', min: PHI_INV_3 * 100, threshold: PHI_INV_2 * 100, icon: '🟡', description: 'Caution — significant concerns detected' },
  { name: 'WAG', min: PHI_INV_2 * 100, threshold: PHI_INV * 100, icon: '🔵', description: 'Acceptable — some strengths, some gaps' },
  { name: 'HOWL', min: PHI_INV * 100, threshold: 100, icon: '🟢', description: 'Strong — above the golden threshold' },
]);

const K_TIERS = Object.freeze([
  { name: 'Rust', min: 0 },
  { name: 'Iron', min: 20 },
  { name: 'Copper', min: 35 },
  { name: 'Bronze', min: 50 },
  { name: 'Silver', min: 62 },
  { name: 'Gold', min: 70 },
  { name: 'Platinum', min: 80 },
  { name: 'Diamond', min: 90 },
]);

/**
 * Axiom weights for Q-Score calculation
 * Sum = 1.0 (normalized)
 */
const AXIOM_WEIGHTS = Object.freeze({
  PHI: PHI_INV,                 // φ⁻¹ ≈ 0.618 — Harmony is the most important
  VERIFY: PHI_INV_2,            // φ⁻² ≈ 0.382 — Truth is second
  CULTURE: PHI_INV_3,           // φ⁻³ ≈ 0.236 — Memory matters
  BURN: PHI_INV_3 * PHI_INV,   // φ⁻⁴ ≈ 0.146 — Action is the foundation
});

// Normalize weights to sum to 1
const WEIGHT_SUM = Object.values(AXIOM_WEIGHTS).reduce((s, v) => s + v, 0);

/**
 * Holder count asymptotic scaling factor.
 * At HOLDER_SCALE holders, the curve reaches its inflection.
 * Formula: H = 1 - 1/(1 + ln(1 + h/HOLDER_SCALE))
 * At 100 → ~41, at 1000 → ~71, at 10000 → ~82. Continuous, no tier jumps.
 */
const HOLDER_SCALE = 100;

/**
 * Token age time constant τ in days.
 * τ = 21 = F₈ (8th Fibonacci number).
 * At τ days, score = 1 - e⁻¹ ≈ 0.632 (within 2% of φ⁻¹).
 * Formula: A = 1 - e^(-d/AGE_TAU)
 */
const AGE_TAU = 21; // F₈

/**
 * Market cap per holder inflection point in USD.
 * F₆ = 8 ($8/holder). A dead/rugged token has < $1/holder.
 * Formula: S = 1 - 1/(1 + ln(1 + mcapPerHolder/MCAP_PER_HOLDER_SCALE))
 * At $0.30/holder → 4, at $8/holder → 41, at $80/holder → 71, at $800/holder → 82
 */
const MCAP_PER_HOLDER_SCALE = 8; // F₆

/**
 * Market cap liquidity threshold in USD.
 * F₈ × 1000 = $21,000. Pump.fun graduation ≈ $90K mcap.
 * Scores the "depth" of available market from total market cap.
 */
const LIQUIDITY_MCAP_SCALE = 21000; // F₈ × 1000

/**
 * K-Score security caps — authority revocation limits max K-Score.
 * Both renounced = full range. One active = capped at φ⁻¹. Both active = capped at φ⁻².
 * Derives directly from the φ power series: no arbitrary thresholds.
 */
const SECURITY_CAPS = Object.freeze({
  BOTH_REVOKED: 1.0,
  ONE_REVOKED: PHI_INV,    // 61.8%
  BOTH_ACTIVE: PHI_INV_2,  // 38.2%
});

// ═══════════════════════════════════════════════════════════════════════════
// SCORER
// ═══════════════════════════════════════════════════════════════════════════

export class TokenScorer {

  /**
   * Score a token across 17 dimensions
   * @param {Object} tokenData - Output from TokenFetcher.getTokenData()
   * @returns {Object} Full verdict with scores, dimensions, and confidence
   */
  score(tokenData) {
    // ═══════════════════════════════════════════════════════════════════════
    // DIMENSION SCORES (0-100 each)
    // ═══════════════════════════════════════════════════════════════════════

    const dimensions = {
      // PHI: Harmony / Balance
      supplyDistribution: this._scoreSupplyDistribution(tokenData),
      liquidityDepth: this._scoreLiquidityDepth(tokenData),
      priceStability: this._scorePriceStability(tokenData),
      supplyMechanics: this._scoreSupplyMechanics(tokenData),

      // VERIFY: Truth / Verifiability
      mintAuthority: this._scoreMintAuthority(tokenData),
      freezeAuthority: this._scoreFreezeAuthority(tokenData),
      metadataIntegrity: this._scoreMetadataIntegrity(tokenData),
      programVerification: this._scoreProgramVerification(tokenData),

      // CULTURE: Community / Memory
      holderCount: this._scoreHolderCount(tokenData),
      tokenAge: this._scoreTokenAge(tokenData),
      ecosystemIntegration: this._scoreEcosystemIntegration(tokenData),
      organicGrowth: this._scoreOrganicGrowth(tokenData),

      // BURN: Creation vs Extraction
      burnActivity: this._scoreBurnActivity(tokenData),
      creatorBehavior: this._scoreCreatorBehavior(tokenData),
      feeRedistribution: this._scoreFeeRedistribution(tokenData),
      realUtility: this._scoreRealUtility(tokenData),
    };

    // THE_UNNAMEABLE: What we can't measure
    // Higher = more uncertainty = more doubt
    const measuredDimensions = Object.values(dimensions);
    const avgMeasured = measuredDimensions.reduce((s, v) => s + v, 0) / measuredDimensions.length;
    const dataCompleteness = this._calculateDataCompleteness(tokenData);
    dimensions.theUnnameable = Math.round(100 * (1 - dataCompleteness));

    // ═══════════════════════════════════════════════════════════════════════
    // AXIOM SCORES (weighted average of dimensions)
    // ═══════════════════════════════════════════════════════════════════════

    const axiomScores = {
      PHI: this._axiomAverage([
        dimensions.supplyDistribution,
        dimensions.liquidityDepth,
        dimensions.priceStability,
        dimensions.supplyMechanics,
      ]),
      VERIFY: this._axiomAverage([
        dimensions.mintAuthority,
        dimensions.freezeAuthority,
        dimensions.metadataIntegrity,
        dimensions.programVerification,
      ]),
      CULTURE: this._axiomAverage([
        dimensions.holderCount,
        dimensions.tokenAge,
        dimensions.ecosystemIntegration,
        dimensions.organicGrowth,
      ]),
      BURN: this._axiomAverage([
        dimensions.burnActivity,
        dimensions.creatorBehavior,
        dimensions.feeRedistribution,
        dimensions.realUtility,
      ]),
    };

    // ═══════════════════════════════════════════════════════════════════════
    // Q-SCORE: Weighted combination of axiom scores
    // ═══════════════════════════════════════════════════════════════════════

    const qScore = Math.round(
      (axiomScores.PHI * AXIOM_WEIGHTS.PHI +
        axiomScores.VERIFY * AXIOM_WEIGHTS.VERIFY +
        axiomScores.CULTURE * AXIOM_WEIGHTS.CULTURE +
        axiomScores.BURN * AXIOM_WEIGHTS.BURN) / WEIGHT_SUM,
    );

    // ═══════════════════════════════════════════════════════════════════════
    // K-SCORE: Token quality (D × O × L)
    // ═══════════════════════════════════════════════════════════════════════

    const d = this._extractDiamondHands(tokenData, dimensions);
    const o = this._extractOrganicGrowth(tokenData, dimensions);
    const l = this._extractLongevity(tokenData, dimensions);
    const rawKScore = calculateKScore(d, o, l);

    // Security cap: authority revocation limits max K-Score
    const securityCap = this._getSecurityCap(tokenData);
    const kScore = Math.min(rawKScore, Math.round(100 * securityCap));
    const tier = this._getTier(kScore);

    // ═══════════════════════════════════════════════════════════════════════
    // CONFIDENCE: Capped at φ⁻¹ (61.8%) — ALWAYS
    // ═══════════════════════════════════════════════════════════════════════

    const rawConfidence = dataCompleteness * PHI_INV;
    const confidence = Math.min(PHI_INV, rawConfidence);

    // ═══════════════════════════════════════════════════════════════════════
    // VERDICT
    // ═══════════════════════════════════════════════════════════════════════

    const verdictTier = this._getVerdictTier(qScore);

    // Identify weaknesses (dimensions below φ⁻² threshold)
    const weaknesses = [];
    for (const [name, score] of Object.entries(dimensions)) {
      if (name === 'theUnnameable') continue;
      if (score < PHI_INV_2 * 100) {
        weaknesses.push({
          dimension: name,
          score,
          axiom: this._dimensionToAxiom(name),
          reason: this._weaknessReason(name, score, tokenData),
        });
      }
    }

    return {
      // Identity
      mint: tokenData.mint,
      name: tokenData.name,
      symbol: tokenData.symbol,

      // Scores
      qScore,
      kScore,
      confidence: Math.round(confidence * 1000) / 1000,

      // Verdict (driven by Q-Score)
      verdict: verdictTier.name,
      verdictIcon: verdictTier.icon,
      verdictDescription: verdictTier.description,

      // K-Tier (supplementary — token quality metal)
      kTier: tier.name,

      // Dimensions (full transparency)
      axiomScores,
      dimensions,

      // K-Score components
      kComponents: {
        d: Math.round(d * 1000) / 1000,
        o: Math.round(o * 1000) / 1000,
        l: Math.round(l * 1000) / 1000,
      },

      // Issues
      weaknesses,
      theUnnameable: dimensions.theUnnameable,

      // Meta
      totalDimensions: 17,
      timestamp: new Date().toISOString(),
      philosophy: 'φ distrusts φ — max confidence 61.8%',
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PHI DIMENSIONS (Harmony / Balance)
  // ═══════════════════════════════════════════════════════════════════════════

  /** D1: Supply Distribution — top holder dominance (lower = better distributed) */
  _scoreSupplyDistribution(data) {
    if (data.isNative) return 70; // SOL has wide distribution but validators concentrate
    // How much does the single largest account hold?
    // Gini from top-20 is unreliable (biased sample → always ~1.0), so we use direct %
    const topHolder = data.distribution?.topHolders?.[0];
    if (!topHolder) return 50; // No data → neutral
    const dominance = Math.min(100, topHolder.percentage);
    return Math.round(100 - dominance);
  }

  /** D2: Liquidity Depth — real DEX liquidity (DexScreener) + holder breadth */
  _scoreLiquidityDepth(data) {
    if (data.isNative) return 100;
    let score = 0;

    // Prefer DexScreener real liquidity over mcap-based estimate
    const ds = data.dexScreener;
    if (ds && ds.liquidityUsd > 0) {
      // Real on-chain liquidity: asymptotic scaling
      // $21K → ~50%, $210K → ~71%, $2.1M → ~82%
      score += Math.round((1 - 1 / (1 + Math.log(1 + ds.liquidityUsd / LIQUIDITY_MCAP_SCALE))) * 50);
    } else {
      // Fallback: mcap-based estimate (when DexScreener unavailable)
      const price = data.priceInfo?.pricePerToken || 0;
      if (price > 0) {
        const mcap = price * (data.supply?.total || 0);
        if (mcap > 0) {
          score += Math.round((1 - 1 / (1 + Math.log(1 + mcap / LIQUIDITY_MCAP_SCALE))) * 50);
        }
      }
    }

    // Holder breadth: more holders = more potential liquidity
    const h = data.distribution?.holderCount || 0;
    if (h > 0) score += Math.round((1 - 1 / (1 + Math.log(1 + h / HOLDER_SCALE))) * 50);
    return Math.min(100, score);
  }

  /** D3: Price Stability — mcap/holder + liquidity/mcap ratio + 24h change */
  _scorePriceStability(data) {
    if (data.isNative) return 80; // SOL is relatively stable for crypto
    // No price data = unverified = 0
    if (!data.priceInfo?.pricePerToken || data.priceInfo.pricePerToken <= 0) return 0;

    const mcap = data.priceInfo.pricePerToken * (data.supply?.total || 0);
    const holders = data.distribution?.holderCount || 1;
    const mcapPerHolder = mcap / holders;

    // Mcap/holder score: dead tokens ($0.30/holder) → 4, healthy ($80/holder) → 71
    let mcapScore = 0;
    if (mcapPerHolder > 0) {
      mcapScore = Math.round((1 - 1 / (1 + Math.log(1 + mcapPerHolder / MCAP_PER_HOLDER_SCALE))) * 100);
    }

    let finalScore = mcapScore;

    // DexScreener signals (if available)
    const ds = data.dexScreener;
    if (ds) {
      // Liquidity/mcap ratio: devastating signal for post-crash tokens
      // MELANIA: $20K liquidity / $116M mcap = 0.017% → score ≈ 3
      // Healthy: $5M liquidity / $100M mcap = 5% → score ≈ 79
      // Formula: asymptotic on ratio percentage, scale = 1% (F₁ as %)
      if (ds.liquidityUsd > 0 && ds.marketCap > 0) {
        const liqRatio = (ds.liquidityUsd / ds.marketCap) * 100; // as percentage
        const liqRatioScore = Math.round((1 - 1 / (1 + Math.log(1 + liqRatio))) * 100);
        finalScore = Math.min(finalScore, liqRatioScore);
      }

      // 24h price crash: -50% in 24h → halve the score
      if (ds.priceChange24h !== null && ds.priceChange24h < -10) {
        const crashPenalty = Math.max(0, Math.min(100, Math.round(100 * (1 + ds.priceChange24h / 100))));
        finalScore = Math.min(finalScore, crashPenalty);
      }
    }

    // OracleMemory price history (if available — our own judgment-to-judgment tracking)
    const ph = data.priceHistory;
    if (ph && ph.priceChange !== undefined && ph.priceChange !== null) {
      const crashScore = Math.max(0, Math.min(100, Math.round(100 * (1 + ph.priceChange))));
      finalScore = Math.min(finalScore, crashScore);
    }

    return finalScore;
  }

  /** D4: Supply Mechanics — mint authority analysis */
  _scoreSupplyMechanics(data) {
    let score = 50; // Baseline

    // Renounced mint authority = supply is fixed = +30
    if (data.authorities?.mintAuthorityActive === false) {
      score += 30;
    } else {
      score -= 20; // Active mint authority = inflation risk
    }

    // Renounced freeze authority = +20
    if (data.authorities?.freezeAuthorityActive === false) {
      score += 20;
    } else {
      score -= 10;
    }

    return Math.max(0, Math.min(100, score));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // VERIFY DIMENSIONS (Truth / Verifiability)
  // ═══════════════════════════════════════════════════════════════════════════

  /** D5: Mint Authority — is it renounced? */
  _scoreMintAuthority(data) {
    if (data.isNative) return 100; // Native SOL
    if (data.authorities?.mintAuthorityActive === false) return 100; // Renounced
    if (data.authorities?.mintAuthorityActive === true) return 20; // Active = risk
    return 50; // Unknown
  }

  /** D6: Freeze Authority — revoked is baseline (pump.fun default), not merit */
  _scoreFreezeAuthority(data) {
    if (data.isNative) return 100;
    // Pump.fun revokes freeze by default → capped at φ⁻¹ (it's the minimum bar, not merit)
    if (data.authorities?.freezeAuthorityActive === false) return Math.round(PHI_INV * 100);
    if (data.authorities?.freezeAuthorityActive === true) return 15; // Can freeze = major risk
    return 50; // Unknown
  }

  /** D7: Metadata Integrity — capped at φ⁻² (pump.fun gives metadata to ALL tokens) */
  _scoreMetadataIntegrity(data) {
    if (data.isNative) return 100;
    const meta = data.metadataIntegrity || {};
    let score = 0;
    if (meta.hasName) score += 30;
    if (meta.hasSymbol) score += 25;
    if (meta.hasUri) score += 25;
    if (meta.hasImage) score += 20;
    // Cap: having metadata is the bare minimum, not a quality signal
    // Pump.fun auto-generates metadata for ALL tokens → zero discriminative power
    return Math.min(Math.round(PHI_INV_2 * 100), score);
  }

  /** D8: Program Verification — 0 without real verification data */
  _scoreProgramVerification(data) {
    if (data.isNative) return 100;
    // "Don't trust, verify" — we have no program verification data
    // Returning 0, not a proxy. Metadata completeness ≠ program safety.
    return 0;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CULTURE DIMENSIONS (Community / Memory)
  // ═══════════════════════════════════════════════════════════════════════════

  /** D9: Holder Count — asymptotic: H = 1 - 1/(1 + ln(1 + h/HOLDER_SCALE)) */
  _scoreHolderCount(data) {
    if (data.isNative) return 100;
    const h = data.distribution?.holderCount || 0;
    if (h === 0) return 0;
    return Math.round((1 - 1 / (1 + Math.log(1 + h / HOLDER_SCALE))) * 100);
  }

  /** D10: Token Age — exponential: A = 1 - e^(-d/AGE_TAU), τ=F₈=21 days */
  _scoreTokenAge(data) {
    if (data.isNative) return 100;
    const d = data.ageInDays || 0;
    if (d === 0) return 0;
    return Math.round((1 - Math.exp(-d / AGE_TAU)) * 100);
  }

  /** D11: Ecosystem Integration — based on holder count + price availability */
  _scoreEcosystemIntegration(data) {
    if (data.isNative) return 100;
    let score = 30; // Base: token exists on-chain
    // Has price data from Helius = listed on DEX
    if (data.priceInfo?.pricePerToken > 0) score += 35;
    // Many holders = ecosystem presence
    const holders = data.distribution?.holderCount || 0;
    if (holders >= 1000) score += 25;
    else if (holders >= 100) score += 15;
    // Has metadata URI = project cares
    if (data.metadataIntegrity?.hasUri) score += 10;
    return Math.min(100, score);
  }

  /** D12: Organic Growth — top-10 concentration inverted (lower concentration = more organic) */
  _scoreOrganicGrowth(data) {
    if (data.isNative) return 90;
    // What % do the top 10 accounts hold? Less = more organically distributed
    // Combined with holderCount via √ in K-Score extraction — no redundancy
    const topHolders = data.distribution?.topHolders || [];
    if (topHolders.length === 0) return 50; // No data → neutral
    const top10Pct = topHolders.reduce((sum, h) => sum + (h.percentage || 0), 0);
    return Math.round(Math.max(0, 100 - Math.min(100, top10Pct)));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // BURN DIMENSIONS (Creation vs Extraction)
  // ═══════════════════════════════════════════════════════════════════════════

  /** D13: Burn Activity — needs burn tx analysis we don't have */
  _scoreBurnActivity(data) {
    if (data.isNative) return 80; // SOL burns via transaction fees
    // No burn data = no evidence of burn mechanism = 0
    return 0;
  }

  /** D14: Creator Behavior — mint authority as proxy for extraction risk */
  _scoreCreatorBehavior(data) {
    if (data.isNative) return 70;
    // Renounced mint = creator gave up control = φ⁻¹ level trust
    if (data.authorities?.mintAuthorityActive === false) return Math.round(PHI_INV * 100);
    // Active or unknown = unverified = don't trust
    return 0;
  }

  /** D15: Fee Redistribution — needs protocol fee analysis we don't have */
  _scoreFeeRedistribution(_data) {
    // No fee redistribution data for any token = 0
    return 0;
  }

  /** D16: Real Utility — DEX listing + holder breadth as evidence */
  _scoreRealUtility(data) {
    if (data.isNative) return 100;
    // Not listed on any DEX = no utility evidence
    if (!data.priceInfo?.pricePerToken) return 0;
    // Listed: holder breadth as utility proxy (asymptotic, consistent with D9)
    const h = data.distribution?.holderCount || 0;
    if (h === 0) return Math.round(PHI_INV_3 * 100); // ~24: listed but no holders yet
    return Math.round((1 - 1 / (1 + Math.log(1 + h / HOLDER_SCALE))) * 100);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  _axiomAverage(scores) {
    return Math.round(scores.reduce((s, v) => s + v, 0) / scores.length);
  }

  _calculateDataCompleteness(data) {
    let available = 0;
    let total = 9; // 9 key data points

    if (data.isNative) return 1.0; // Native SOL has full data implicitly

    if (data.name && data.name !== 'Unknown') available++;
    if (data.supply?.total > 0) available++;
    if (data.distribution?.topHolders?.length > 0) available++;
    if (data.authorities?.mintAuthority !== undefined) available++;
    if (data.authorities?.freezeAuthority !== undefined) available++;
    if (data.ageInDays > 0) available++;
    if (data.distribution?.holderCount > 0) available++;
    if (data.metadataIntegrity?.hasUri) available++;
    if (data.priceInfo?.pricePerToken > 0) available++;

    return available / total;
  }

  _extractDiamondHands(_data, dimensions) {
    // D = √(distribution × mechanics) — geometric mean: both must be strong
    return Math.sqrt((dimensions.supplyDistribution / 100) * (dimensions.supplyMechanics / 100));
  }

  _extractOrganicGrowth(_data, dimensions) {
    // O = √(holders × growth) — geometric mean: community AND distribution
    return Math.sqrt((dimensions.holderCount / 100) * (dimensions.organicGrowth / 100));
  }

  _extractLongevity(_data, dimensions) {
    // L = √(age × ecosystem) — geometric mean: time AND integration
    return Math.sqrt((dimensions.tokenAge / 100) * (dimensions.ecosystemIntegration / 100));
  }

  _getSecurityCap(data) {
    if (data.isNative) return SECURITY_CAPS.BOTH_REVOKED;
    const mintActive = data.authorities?.mintAuthorityActive !== false;
    const freezeActive = data.authorities?.freezeAuthorityActive !== false;
    if (!mintActive && !freezeActive) return SECURITY_CAPS.BOTH_REVOKED;
    if (!mintActive || !freezeActive) return SECURITY_CAPS.ONE_REVOKED;
    return SECURITY_CAPS.BOTH_ACTIVE;
  }

  _getTier(kScore) {
    for (let i = K_TIERS.length - 1; i >= 0; i--) {
      if (kScore >= K_TIERS[i].min) return K_TIERS[i];
    }
    return K_TIERS[0];
  }

  _getVerdictTier(qScore) {
    for (let i = VERDICT_TIERS.length - 1; i >= 0; i--) {
      if (qScore >= VERDICT_TIERS[i].min) return VERDICT_TIERS[i];
    }
    return VERDICT_TIERS[0];
  }

  _dimensionToAxiom(name) {
    const map = {
      supplyDistribution: 'PHI', liquidityDepth: 'PHI',
      priceStability: 'PHI', supplyMechanics: 'PHI',
      mintAuthority: 'VERIFY', freezeAuthority: 'VERIFY',
      metadataIntegrity: 'VERIFY', programVerification: 'VERIFY',
      holderCount: 'CULTURE', tokenAge: 'CULTURE',
      ecosystemIntegration: 'CULTURE', organicGrowth: 'CULTURE',
      burnActivity: 'BURN', creatorBehavior: 'BURN',
      feeRedistribution: 'BURN', realUtility: 'BURN',
    };
    return map[name] || 'UNKNOWN';
  }

  _weaknessReason(name, score, data) {
    const reasons = {
      supplyDistribution: 'Token supply concentrated in few wallets',
      liquidityDepth: 'Low real DEX liquidity or few holders',
      priceStability: 'Low liquidity, weak mcap/holder, or price crash detected',
      supplyMechanics: 'Mint/freeze authority still active',
      mintAuthority: 'Mint authority not renounced — inflation risk',
      freezeAuthority: 'Freeze authority active — accounts can be frozen',
      metadataIntegrity: 'Incomplete or missing metadata',
      programVerification: 'Program not verified on-chain',
      holderCount: 'Very few token holders',
      tokenAge: 'Token is very new — insufficient history',
      ecosystemIntegration: 'Limited ecosystem integration',
      organicGrowth: 'Distribution appears inorganic',
      burnActivity: 'No burn mechanism detected',
      creatorBehavior: 'Creator wallet behavior unclear',
      feeRedistribution: 'Fee model unclear',
      realUtility: 'Limited real-world utility detected',
    };
    return reasons[name] || 'Below threshold';
  }
}

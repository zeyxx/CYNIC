/**
 * Skeptic Service - The Path Between Binah and Gevurah
 *
 * Implements the Kabbalistic verification process:
 *   TZIMTZUM (צמצום) → BERUR (בירור) → TIKKUN (תיקון)
 *   Contraction    →  Clarification →  Repair
 *
 * "גם זו לטובה" — "This too is for the good"
 * The doubt is not the enemy of truth. The doubt IS the path to truth.
 *
 * @module @cynic/node/services/skeptic
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('Skeptic');

// φ constants - the golden boundaries
const PHI_INV = 0.618033988749895;   // Max confidence
const PHI_SQ_INV = 0.381966011250105; // Min doubt

/**
 * Claim types for classification
 */
export const ClaimType = Object.freeze({
  FACTUAL: 'factual',       // Verifiable against data
  NUMERICAL: 'numerical',   // Contains numbers/stats
  TECHNICAL: 'technical',   // Code/architecture claims
  OPINION: 'opinion',       // Subjective statement
  PREDICTION: 'prediction', // Future-oriented
  REFERENCE: 'reference',   // References external source
});

/**
 * Verification status
 */
export const VerificationStatus = Object.freeze({
  VERIFIED: 'verified',     // Confirmed true
  DISPUTED: 'disputed',     // Found to be false/inaccurate
  PARTIAL: 'partial',       // Partially true
  UNKNOWN: 'unknown',       // Cannot verify
  OVERCLAIMED: 'overclaimed', // True but overstated
});

/**
 * Skeptic Verdict
 */
export const SkepticVerdict = Object.freeze({
  TRUST: 'TRUST',           // All claims verified
  DOUBT: 'DOUBT',           // Significant issues found
  MIXED: 'MIXED',           // Some verified, some not
  VERIFY: 'VERIFY',         // Needs manual verification
});

/**
 * SkepticService - The Verification Engine
 *
 * Processes responses through three Kabbalistic phases:
 * 1. TZIMTZUM - Contract and extract claims
 * 2. BERUR - Clarify and verify each claim
 * 3. TIKKUN - Repair with corrections
 */
export class SkepticService {
  /**
   * @param {Object} options
   * @param {Object} options.factsRepo - FactsRepository for verification
   * @param {Object} options.ollamaClient - Ollama client for LLM calls
   * @param {string} options.extractionModel - Model for claim extraction (default: phi3:mini)
   * @param {string} options.verificationModel - Model for deep verification (default: mixtral)
   * @param {boolean} options.useAirLLM - Whether to use AirLLM for large models
   */
  constructor(options = {}) {
    this.factsRepo = options.factsRepo;
    this.codebaseIndexer = options.codebaseIndexer;
    this.ollamaBaseUrl = options.ollamaBaseUrl || process.env.OLLAMA_HOST || 'http://localhost:11434';
    this.extractionModel = options.extractionModel || 'phi3:mini';
    this.verificationModel = options.verificationModel || 'phi3:mini'; // Start with phi3, upgrade to mixtral later
    this.useAirLLM = options.useAirLLM || false;
    this.timeout = options.timeout || 60000; // 60s for verification

    // Statistics
    this.stats = {
      totalVerifications: 0,
      claimsExtracted: 0,
      claimsVerified: 0,
      claimsDisputed: 0,
      corrections: 0,
    };
  }

  /**
   * Full verification pipeline: TZIMTZUM → BERUR → TIKKUN
   *
   * @param {string} content - The content to verify
   * @param {Object} context - Additional context for verification
   * @returns {Promise<Object>} SkepticResult
   */
  async verify(content, context = {}) {
    const startTime = Date.now();
    log.info('Starting verification (TZIMTZUM → BERUR → TIKKUN)');

    try {
      // ═══════════════════════════════════════════════════════════════════════
      // PHASE 1: TZIMTZUM (צמצום) - Contraction
      // Extract the essential claims from the content
      // ═══════════════════════════════════════════════════════════════════════
      const claims = await this._tzimtzum(content);
      log.info(`TZIMTZUM complete: ${claims.length} claims extracted`);

      if (claims.length === 0) {
        return this._createResult({
          verdict: SkepticVerdict.TRUST,
          message: 'No verifiable claims found',
          claims: [],
          corrections: [],
          confidence: PHI_INV,
          duration: Date.now() - startTime,
        });
      }

      // ═══════════════════════════════════════════════════════════════════════
      // PHASE 2: BERUR (בירור) - Clarification
      // Verify each claim against facts, codebase, and logic
      // ═══════════════════════════════════════════════════════════════════════
      const verifiedClaims = await this._berur(claims, context);
      log.info(`BERUR complete: ${verifiedClaims.filter(c => c.status === VerificationStatus.VERIFIED).length} verified`);

      // ═══════════════════════════════════════════════════════════════════════
      // PHASE 3: TIKKUN (תיקון) - Repair
      // Generate corrections for disputed claims
      // ═══════════════════════════════════════════════════════════════════════
      const result = await this._tikkun(verifiedClaims, content);
      log.info(`TIKKUN complete: ${result.corrections.length} corrections`);

      // Update stats
      this.stats.totalVerifications++;
      this.stats.claimsExtracted += claims.length;
      this.stats.claimsVerified += verifiedClaims.filter(c => c.status === VerificationStatus.VERIFIED).length;
      this.stats.claimsDisputed += verifiedClaims.filter(c => c.status === VerificationStatus.DISPUTED).length;
      this.stats.corrections += result.corrections.length;

      return {
        ...result,
        duration: Date.now() - startTime,
      };

    } catch (error) {
      log.error('Verification failed', { error: error.message });
      return this._createResult({
        verdict: SkepticVerdict.VERIFY,
        message: `Verification error: ${error.message}`,
        claims: [],
        corrections: [],
        confidence: PHI_SQ_INV, // Low confidence due to error
        duration: Date.now() - startTime,
        error: error.message,
      });
    }
  }

  /**
   * PHASE 1: TZIMTZUM (צמצום) - Contraction
   *
   * God contracted to create space for the world.
   * We contract the response to extract essential claims.
   *
   * @param {string} content - Content to analyze
   * @returns {Promise<Object[]>} Extracted claims
   * @private
   */
  async _tzimtzum(content) {
    const claims = [];

    // Step 1: Heuristic extraction (fast, no LLM)
    const heuristicClaims = this._extractClaimsHeuristic(content);
    claims.push(...heuristicClaims);

    // Step 2: LLM-based extraction for complex claims (if available)
    if (await this._isOllamaAvailable()) {
      try {
        const llmClaims = await this._extractClaimsLLM(content);
        // Merge, avoiding duplicates
        for (const claim of llmClaims) {
          if (!claims.some(c => this._claimsSimilar(c, claim))) {
            claims.push(claim);
          }
        }
      } catch (e) {
        log.warn('LLM extraction failed, using heuristics only', { error: e.message });
      }
    }

    return claims;
  }

  /**
   * Heuristic claim extraction (no LLM needed)
   * @private
   */
  _extractClaimsHeuristic(content) {
    const claims = [];

    // Pattern 1: Numerical claims (X%, N items, etc.)
    const numberPatterns = [
      /(\d+(?:\.\d+)?)\s*%/g,                          // Percentages
      /(\d+(?:,\d{3})*(?:\.\d+)?)\s*(MB|GB|KB|bytes?)/gi, // Sizes
      /(\d+)\s*(files?|lines?|functions?|classes?|modules?)/gi, // Counts
      /(?:takes?|requires?|needs?)\s*~?(\d+)\s*(seconds?|minutes?|hours?|ms)/gi, // Time
      /(\d+)\s*dimensions?/gi,                          // Dimensions (CYNIC-specific)
      /confidence\s*(?:of\s*)?(\d+(?:\.\d+)?)/gi,      // Confidence scores
    ];

    for (const pattern of numberPatterns) {
      let match;
      while ((match = pattern.exec(content)) !== null) {
        claims.push({
          text: match[0],
          type: ClaimType.NUMERICAL,
          value: parseFloat(match[1].replace(/,/g, '')),
          context: content.substring(Math.max(0, match.index - 50), match.index + match[0].length + 50),
          source: 'heuristic',
        });
      }
    }

    // Pattern 2: Technical claims about CYNIC
    const cynicPatterns = [
      /(CYNIC|cynic)\s+(uses?|has|implements?|contains?)\s+([^.]+)/gi,
      /(25|twenty-?five)\s+dimensions?/gi,
      /(collective\s+dogs?|11\s+dogs?|sefirot)/gi,
      /(φ|phi|golden\s+ratio|61\.8|0\.618)/gi,
      /(MoE|mixture\s+of\s+experts?)/gi,
    ];

    for (const pattern of cynicPatterns) {
      let match;
      while ((match = pattern.exec(content)) !== null) {
        claims.push({
          text: match[0],
          type: ClaimType.TECHNICAL,
          context: content.substring(Math.max(0, match.index - 50), match.index + match[0].length + 50),
          source: 'heuristic',
        });
      }
    }

    // Pattern 3: Confidence/certainty indicators (potential overclaims)
    const certaintyPatterns = [
      /(?:definitely|certainly|always|never|100%|guaranteed|absolutely)/gi,
      /(?:will\s+(?:definitely|always)|must\s+be|cannot\s+fail)/gi,
    ];

    for (const pattern of certaintyPatterns) {
      let match;
      while ((match = pattern.exec(content)) !== null) {
        claims.push({
          text: match[0],
          type: ClaimType.OPINION,
          flag: 'OVERCERTAINTY',
          context: content.substring(Math.max(0, match.index - 50), match.index + match[0].length + 50),
          source: 'heuristic',
        });
      }
    }

    return claims;
  }

  /**
   * LLM-based claim extraction
   * @private
   */
  async _extractClaimsLLM(content) {
    const prompt = `You are a claim extractor. Analyze the following text and extract all verifiable claims.

For each claim, output in this exact format:
CLAIM: [the claim]
TYPE: [factual|numerical|technical|opinion|prediction|reference]
CONFIDENCE_NEEDED: [low|medium|high]

Text to analyze:
"""
${content.substring(0, 2000)}
"""

Extract claims now:`;

    const response = await this._callOllama(this.extractionModel, prompt);
    return this._parseClaimsFromLLM(response);
  }

  /**
   * Parse LLM response into structured claims
   * @private
   */
  _parseClaimsFromLLM(response) {
    const claims = [];
    const claimBlocks = response.split(/CLAIM:/i).slice(1);

    for (const block of claimBlocks) {
      const lines = block.trim().split('\n');
      const claim = {
        text: lines[0]?.trim() || '',
        type: ClaimType.FACTUAL,
        source: 'llm',
      };

      for (const line of lines.slice(1)) {
        if (line.toLowerCase().startsWith('type:')) {
          const typeStr = line.split(':')[1]?.trim().toLowerCase();
          if (Object.values(ClaimType).includes(typeStr)) {
            claim.type = typeStr;
          }
        }
      }

      if (claim.text) {
        claims.push(claim);
      }
    }

    return claims;
  }

  /**
   * PHASE 2: BERUR (בירור) - Clarification
   *
   * Separate truth from falsehood, light from shells.
   *
   * @param {Object[]} claims - Claims to verify
   * @param {Object} context - Verification context
   * @returns {Promise<Object[]>} Verified claims
   * @private
   */
  async _berur(claims, context = {}) {
    const verifiedClaims = [];

    for (const claim of claims) {
      const verification = await this._verifyClaim(claim, context);
      verifiedClaims.push({
        ...claim,
        ...verification,
      });
    }

    return verifiedClaims;
  }

  /**
   * Verify a single claim
   * @private
   */
  async _verifyClaim(claim, context) {
    // Strategy 1: Check against FactsRepository
    if (this.factsRepo && claim.type === ClaimType.TECHNICAL) {
      const factCheck = await this._checkAgainstFacts(claim);
      if (factCheck.found) {
        return factCheck;
      }
    }

    // Strategy 2: Numerical bounds checking (CYNIC-specific)
    if (claim.type === ClaimType.NUMERICAL) {
      const boundsCheck = this._checkNumericalBounds(claim);
      if (boundsCheck.status !== VerificationStatus.UNKNOWN) {
        return boundsCheck;
      }
    }

    // Strategy 3: Overcertainty detection
    if (claim.flag === 'OVERCERTAINTY') {
      return {
        status: VerificationStatus.OVERCLAIMED,
        reason: 'Absolute certainty violates φ axiom (max confidence 61.8%)',
        correction: 'Use hedged language: "likely", "typically", "in most cases"',
        confidence: PHI_SQ_INV,
      };
    }

    // Strategy 4: LLM-based verification for complex claims
    if (await this._isOllamaAvailable() && claim.type === ClaimType.TECHNICAL) {
      try {
        return await this._verifyWithLLM(claim, context);
      } catch (e) {
        log.warn('LLM verification failed', { error: e.message });
      }
    }

    // Default: Unknown (needs human verification)
    return {
      status: VerificationStatus.UNKNOWN,
      reason: 'Could not automatically verify',
      confidence: PHI_SQ_INV,
    };
  }

  /**
   * Check claim against FactsRepository
   * @private
   */
  async _checkAgainstFacts(claim) {
    try {
      const facts = await this.factsRepo.search(claim.text, { limit: 3 });

      if (facts.length > 0) {
        const relevantFact = facts[0];
        // Simple similarity check
        if (relevantFact.confidence >= PHI_SQ_INV) {
          return {
            found: true,
            status: VerificationStatus.VERIFIED,
            reason: `Matches known fact: "${relevantFact.subject}"`,
            source: 'facts_repository',
            confidence: Math.min(PHI_INV, relevantFact.confidence),
          };
        }
      }
    } catch (e) {
      log.debug('Facts lookup failed', { error: e.message });
    }

    return { found: false };
  }

  /**
   * Check numerical bounds (CYNIC-specific rules)
   * @private
   */
  _checkNumericalBounds(claim) {
    const value = claim.value;
    const text = claim.text.toLowerCase();

    // Rule 1: Confidence percentages must be ≤ 61.8%
    if (text.includes('confidence') || text.includes('accuracy') || text.includes('precision')) {
      if (value > 61.8) {
        return {
          status: VerificationStatus.DISPUTED,
          reason: `Value ${value}% exceeds φ⁻¹ maximum (61.8%)`,
          correction: `Maximum allowed is 61.8% per PHI axiom`,
          confidence: PHI_INV,
        };
      }
    }

    // Rule 2: CYNIC has exactly 25 dimensions
    if (text.includes('dimension')) {
      if (value !== 25 && value !== 24) { // 24 named + 1 META = 25
        return {
          status: VerificationStatus.DISPUTED,
          reason: `CYNIC has 25 dimensions (4 axioms × 6 + 1 META), not ${value}`,
          correction: '25 dimensions total',
          confidence: PHI_INV,
        };
      } else {
        return {
          status: VerificationStatus.VERIFIED,
          reason: 'Correct: CYNIC uses 25 dimensions',
          confidence: PHI_INV,
        };
      }
    }

    // Rule 3: Collective Dogs count
    if (text.includes('dog')) {
      if (value !== 11 && value !== 10) { // 10 dogs + CYNIC = 11
        return {
          status: VerificationStatus.DISPUTED,
          reason: `CYNIC has 11 Collective Dogs (10 + CYNIC/Keter), not ${value}`,
          correction: '11 dogs total',
          confidence: PHI_INV,
        };
      }
    }

    return { status: VerificationStatus.UNKNOWN };
  }

  /**
   * LLM-based verification
   * @private
   */
  async _verifyWithLLM(claim, context) {
    const prompt = `You are a fact-checker for CYNIC, a judgment system. Verify this claim:

CLAIM: "${claim.text}"
CONTEXT: ${claim.context || 'None provided'}

CYNIC FACTS:
- Uses 25 dimensions for judgment (4 axioms × 6 dimensions + 1 META)
- Maximum confidence is 61.8% (φ⁻¹, golden ratio inverse)
- Has 11 Collective Dogs based on Kabbalistic Sefirot
- Is NOT a Mixture of Experts (MoE) architecture - it's algorithmic
- Uses heuristic scoring, not neural networks for judgment
- PHI, VERIFY, CULTURE, BURN are the four axioms

Respond in this format:
STATUS: [VERIFIED|DISPUTED|PARTIAL|UNKNOWN]
REASON: [explanation]
CORRECTION: [if disputed, provide correction]
CONFIDENCE: [0.0-0.618]`;

    const response = await this._callOllama(this.verificationModel, prompt);
    return this._parseVerificationFromLLM(response);
  }

  /**
   * Parse verification response from LLM
   * @private
   */
  _parseVerificationFromLLM(response) {
    const result = {
      status: VerificationStatus.UNKNOWN,
      reason: 'LLM verification',
      confidence: PHI_SQ_INV,
    };

    const statusMatch = response.match(/STATUS:\s*(VERIFIED|DISPUTED|PARTIAL|UNKNOWN)/i);
    if (statusMatch) {
      result.status = statusMatch[1].toLowerCase();
    }

    const reasonMatch = response.match(/REASON:\s*(.+?)(?=CORRECTION:|CONFIDENCE:|$)/is);
    if (reasonMatch) {
      result.reason = reasonMatch[1].trim();
    }

    const correctionMatch = response.match(/CORRECTION:\s*(.+?)(?=CONFIDENCE:|$)/is);
    if (correctionMatch) {
      result.correction = correctionMatch[1].trim();
    }

    const confidenceMatch = response.match(/CONFIDENCE:\s*([\d.]+)/i);
    if (confidenceMatch) {
      result.confidence = Math.min(PHI_INV, parseFloat(confidenceMatch[1]));
    }

    return result;
  }

  /**
   * PHASE 3: TIKKUN (תיקון) - Repair
   *
   * Repair the world through correction.
   *
   * @param {Object[]} verifiedClaims - Claims with verification status
   * @param {string} originalContent - Original content
   * @returns {Promise<Object>} Final result with corrections
   * @private
   */
  async _tikkun(verifiedClaims, originalContent) {
    const corrections = [];
    let verified = 0;
    let disputed = 0;
    let unknown = 0;

    for (const claim of verifiedClaims) {
      switch (claim.status) {
        case VerificationStatus.VERIFIED:
          verified++;
          break;
        case VerificationStatus.DISPUTED:
        case VerificationStatus.OVERCLAIMED:
          disputed++;
          if (claim.correction) {
            corrections.push({
              original: claim.text,
              correction: claim.correction,
              reason: claim.reason,
              type: claim.status,
            });
          }
          break;
        default:
          unknown++;
      }
    }

    // Determine overall verdict
    let verdict;
    let message;

    if (disputed === 0 && verified > 0) {
      verdict = SkepticVerdict.TRUST;
      message = `All ${verified} verifiable claims confirmed`;
    } else if (disputed > verified) {
      verdict = SkepticVerdict.DOUBT;
      message = `${disputed} claims disputed, ${corrections.length} corrections suggested`;
    } else if (disputed > 0) {
      verdict = SkepticVerdict.MIXED;
      message = `${verified} verified, ${disputed} disputed, ${unknown} unknown`;
    } else {
      verdict = SkepticVerdict.VERIFY;
      message = `${unknown} claims could not be automatically verified`;
    }

    // Calculate adjusted confidence
    const total = verified + disputed + unknown;
    const adjustedConfidence = total > 0
      ? Math.min(PHI_INV, (verified / total) * PHI_INV)
      : PHI_SQ_INV;

    return this._createResult({
      verdict,
      message,
      claims: verifiedClaims,
      corrections,
      confidence: adjustedConfidence,
      stats: { verified, disputed, unknown, total },
    });
  }

  /**
   * Create standardized result object
   * @private
   */
  _createResult(data) {
    return {
      verdict: data.verdict || SkepticVerdict.VERIFY,
      message: data.message || '',
      claims: data.claims || [],
      corrections: data.corrections || [],
      confidence: Math.min(PHI_INV, data.confidence || PHI_SQ_INV),
      stats: data.stats || {},
      duration: data.duration || 0,
      error: data.error || null,
      timestamp: Date.now(),
      _meta: {
        service: 'SkepticService',
        process: 'TZIMTZUM → BERUR → TIKKUN',
        maxConfidence: PHI_INV,
      },
    };
  }

  /**
   * Check if two claims are similar (deduplication)
   * @private
   */
  _claimsSimilar(a, b) {
    if (a.type !== b.type) return false;
    if (a.value && b.value && a.value === b.value) return true;

    // Simple text similarity
    const aWords = new Set(a.text.toLowerCase().split(/\s+/));
    const bWords = new Set(b.text.toLowerCase().split(/\s+/));
    const intersection = [...aWords].filter(w => bWords.has(w));
    const union = new Set([...aWords, ...bWords]);

    return intersection.length / union.size > 0.7;
  }

  /**
   * Check if Ollama is available
   * @private
   */
  async _isOllamaAvailable() {
    try {
      const response = await fetch(`${this.ollamaBaseUrl}/api/tags`, {
        signal: AbortSignal.timeout(3000),
      });
      return response.ok;
    } catch {
      return false;
    }
  }

  /**
   * Call Ollama API
   * @private
   */
  async _callOllama(model, prompt) {
    const response = await fetch(`${this.ollamaBaseUrl}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model,
        prompt,
        stream: false,
        options: {
          temperature: 0.3, // Low temperature for factual tasks
          num_predict: 500,
        },
      }),
      signal: AbortSignal.timeout(this.timeout),
    });

    if (!response.ok) {
      throw new Error(`Ollama error: ${response.status}`);
    }

    const data = await response.json();
    return data.response || '';
  }

  /**
   * Get service statistics
   */
  getStats() {
    return {
      ...this.stats,
      model: {
        extraction: this.extractionModel,
        verification: this.verificationModel,
      },
    };
  }
}

/**
 * Create SkepticService instance
 * @param {Object} options
 * @returns {SkepticService}
 */
export function createSkepticService(options = {}) {
  return new SkepticService(options);
}

export default SkepticService;

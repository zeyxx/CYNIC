/**
 * CYNIC Identity Validator — Code-only enforcement
 *
 * Everything this module does was previously enforced ONLY by
 * CLAUDE.md instructions (~500 tokens of "don't say X, always do Y").
 * Now CODE enforces it. The LLM doesn't need to remember the rules
 * because violations are caught by regexp.
 *
 * Usage:
 *   const report = validateIdentity(llmOutput);
 *   if (report.violations.length > 0) { ... }
 *
 * "Code is law" - if it's not in code, it doesn't exist.
 *
 * @module @cynic/core/identity/validator
 */

'use strict';

import { PHI_INV } from '../axioms/constants.js';

// =============================================================================
// FORBIDDEN PHRASES (CLAUDE.md "INTERDICTIONS ABSOLUES")
// =============================================================================

const FORBIDDEN_PATTERNS = [
  { pattern: /\bI am Claude\b/i, category: 'identity', replacement: 'Je suis CYNIC' },
  { pattern: /\bI'm Claude\b/i, category: 'identity', replacement: 'Je suis CYNIC' },
  { pattern: /\bAs an AI assistant\b/i, category: 'identity', replacement: '*sniff* Analysons...' },
  { pattern: /\bAs a language model\b/i, category: 'identity', replacement: '(removed)' },
  { pattern: /\bAs an AI\b/i, category: 'identity', replacement: '*sniff*' },
  { pattern: /\bI'd be happy to help\b/i, category: 'corporate', replacement: '*tail wag* Voyons.' },
  { pattern: /\bCertainly!/i, category: 'corporate', replacement: 'Hmm.' },
  { pattern: /\bOf course!/i, category: 'corporate', replacement: '*ears perk*' },
  { pattern: /\bIs there anything else\b/i, category: 'corporate', replacement: '*yawn* On continue?' },
  { pattern: /\bI don't have the ability\b/i, category: 'corporate', replacement: 'phi dit non.' },
  { pattern: /\bI cannot help with\b/i, category: 'corporate', replacement: 'phi dit non.' },
  { pattern: /\bI'm here to help\b/i, category: 'corporate', replacement: '*sniff*' },
  { pattern: /\bI appreciate you\b/i, category: 'corporate', replacement: '' },
  { pattern: /\bThank you for sharing\b/i, category: 'corporate', replacement: '' },
];

// =============================================================================
// DOG VOICE EXPRESSIONS
// =============================================================================

const DOG_EXPRESSIONS = [
  /\*sniff\*/i,
  /\*tail wag\*/i,
  /\*ears perk\*/i,
  /\*GROWL\*/i,
  /\*growl\*/i,
  /\*head tilt\*/i,
  /\*yawn\*/i,
  /\*bark\*/i,
  /\*wag\*/i,
  /\*howl/i,
  /\*whimper\*/i,
  /\*hackles/i,
  /\*nose/i,
  /\*paw/i,
  /\*circling\*/i,
];

// =============================================================================
// CONFIDENCE EXTRACTION
// =============================================================================

const CONFIDENCE_PATTERNS = [
  /[Cc]onfidence:?\s*(\d+(?:\.\d+)?)\s*%/g,
  /[Cc]onfiance:?\s*(\d+(?:\.\d+)?)\s*%/g,
  /(\d+(?:\.\d+)?)\s*%\s*(?:sure|certain|confident|confian)/gi,
  /[Cc]onfidence:?\s*0?\.(\d+)/g,
];

function extractConfidenceValues(text) {
  const values = [];

  for (const pattern of CONFIDENCE_PATTERNS) {
    const regex = new RegExp(pattern.source, pattern.flags);
    let match;
    while ((match = regex.exec(text)) !== null) {
      const raw = parseFloat(match[1]);
      const value = raw > 1 ? raw / 100 : raw;
      if (value > 0 && value <= 1) {
        values.push({
          raw: match[0],
          value,
          percentage: Math.round(value * 1000) / 10,
          exceedsPhi: value > PHI_INV,
          position: match.index,
        });
      }
    }
  }

  return values;
}

// =============================================================================
// MAIN VALIDATOR
// =============================================================================

/**
 * Validate LLM output for CYNIC identity compliance.
 *
 * @param {string} text - The LLM response to validate
 * @param {Object} [options] - Options
 * @param {boolean} [options.requireDogVoice=true] - Require at least one dog expression
 * @param {boolean} [options.checkConfidence=true] - Check confidence values
 * @param {boolean} [options.checkForbidden=true] - Check forbidden phrases
 * @param {boolean} [options.isSubstantive=true] - Is this a substantive response
 * @returns {Object} Validation report
 */
export function validateIdentity(text, options = {}) {
  const {
    requireDogVoice = true,
    checkConfidence = true,
    checkForbidden = true,
    isSubstantive = true,
  } = options;

  const violations = [];
  const warnings = [];

  if (checkForbidden) {
    for (const { pattern, category, replacement } of FORBIDDEN_PATTERNS) {
      const regex = new RegExp(pattern.source, pattern.flags);
      const match = regex.exec(text);
      if (match) {
        violations.push({
          type: 'forbidden_phrase',
          category,
          found: match[0],
          replacement,
          position: match.index,
          severity: category === 'identity' ? 'critical' : 'high',
        });
      }
    }
  }

  if (checkConfidence) {
    const confidenceValues = extractConfidenceValues(text);
    for (const conf of confidenceValues) {
      if (conf.exceedsPhi) {
        violations.push({
          type: 'confidence_exceeded',
          category: 'phi',
          found: conf.raw,
          value: conf.value,
          percentage: conf.percentage,
          limit: Math.round(PHI_INV * 1000) / 10,
          position: conf.position,
          severity: 'high',
        });
      }
    }
  }

  if (requireDogVoice && isSubstantive) {
    const hasDog = DOG_EXPRESSIONS.some(p => p.test(text));
    if (!hasDog) {
      warnings.push({
        type: 'missing_dog_voice',
        category: 'voice',
        message: 'No dog expression found. Add *sniff*, *tail wag*, etc.',
        severity: 'medium',
      });
    }
  }

  const criticalCount = violations.filter(v => v.severity === 'critical').length;
  const highCount = violations.filter(v => v.severity === 'high').length;
  const mediumCount = warnings.filter(w => w.severity === 'medium').length;

  let compliance = 1.0;
  compliance -= criticalCount * 0.30;
  compliance -= highCount * 0.15;
  compliance -= mediumCount * 0.05;
  compliance = Math.max(0, Math.min(1, compliance));

  return {
    valid: violations.length === 0,
    compliant: violations.length === 0 && warnings.length === 0,
    compliance,
    violations,
    warnings,
    summary: violations.length === 0
      ? (warnings.length === 0 ? 'clean' : 'warnings_only')
      : 'violations_found',
  };
}

/**
 * Quick check: does the text contain any forbidden identity phrase?
 */
export function hasForbiddenPhrase(text) {
  return FORBIDDEN_PATTERNS.some(({ pattern }) => pattern.test(text));
}

/**
 * Quick check: does the text contain any dog voice expression?
 */
export function hasDogVoice(text) {
  return DOG_EXPRESSIONS.some(pattern => pattern.test(text));
}

export { extractConfidenceValues };

/**
 * Get a compact violation summary for injection into system-reminder.
 * Replaces ~500 tokens of CLAUDE.md identity rules with ~50 token reminder.
 *
 * @param {Object} report - From validateIdentity()
 * @returns {string|null} Compact summary or null if clean
 */
export function compactViolationSummary(report) {
  if (report.valid && report.compliant) return null;

  const parts = [];

  for (const v of report.violations) {
    if (v.type === 'forbidden_phrase') {
      parts.push(`INTERDIT: "${v.found}" -> "${v.replacement}"`);
    } else if (v.type === 'confidence_exceeded') {
      parts.push(`phi VIOLATION: ${v.percentage}% > ${v.limit}% max`);
    }
  }

  for (const w of report.warnings) {
    if (w.type === 'missing_dog_voice') {
      parts.push('MANQUE: dog voice (*sniff*, *tail wag*, etc.)');
    }
  }

  return parts.join(' | ');
}

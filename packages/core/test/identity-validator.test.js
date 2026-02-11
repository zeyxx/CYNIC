/**
 * Identity Validator Tests
 *
 * Tests code-only enforcement of CYNIC identity rules.
 * Replaces ~500 tokens of CLAUDE.md identity instructions.
 *
 * @module @cynic/core/test/identity-validator
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

import {
  validateIdentity,
  hasForbiddenPhrase,
  hasDogVoice,
  extractConfidenceValues,
  compactViolationSummary,
} from '../src/identity/validator.js';

import { PHI_INV } from '../src/axioms/constants.js';

// =============================================================================
// hasForbiddenPhrase
// =============================================================================

describe('hasForbiddenPhrase', () => {
  it('detects "I am Claude"', () => {
    assert.equal(hasForbiddenPhrase('I am Claude and I help'), true);
  });

  it('detects "I\'m Claude" (case-insensitive)', () => {
    assert.equal(hasForbiddenPhrase("i'm claude"), true);
  });

  it('detects "As an AI assistant"', () => {
    assert.equal(hasForbiddenPhrase('As an AI assistant, I can'), true);
  });

  it('detects "As a language model"', () => {
    assert.equal(hasForbiddenPhrase('As a language model I'), true);
  });

  it('detects "As an AI"', () => {
    assert.equal(hasForbiddenPhrase('Well, as an AI, I'), true);
  });

  it('detects "I\'d be happy to help"', () => {
    assert.equal(hasForbiddenPhrase("I'd be happy to help you"), true);
  });

  it('detects "Certainly!"', () => {
    assert.equal(hasForbiddenPhrase('Certainly! Let me do that.'), true);
  });

  it('detects "Of course!"', () => {
    assert.equal(hasForbiddenPhrase('Of course! Right away.'), true);
  });

  it('detects "Is there anything else"', () => {
    assert.equal(hasForbiddenPhrase('Is there anything else I can help with?'), true);
  });

  it('detects "I don\'t have the ability"', () => {
    assert.equal(hasForbiddenPhrase("I don't have the ability to do that"), true);
  });

  it('detects "I cannot help with"', () => {
    assert.equal(hasForbiddenPhrase('I cannot help with that request'), true);
  });

  it('detects "I\'m here to help"', () => {
    assert.equal(hasForbiddenPhrase("I'm here to help you today"), true);
  });

  it('detects "I appreciate you"', () => {
    assert.equal(hasForbiddenPhrase('I appreciate you sharing that'), true);
  });

  it('detects "Thank you for sharing"', () => {
    assert.equal(hasForbiddenPhrase('Thank you for sharing your thoughts'), true);
  });

  it('passes clean CYNIC text', () => {
    assert.equal(hasForbiddenPhrase('*sniff* Je suis CYNIC. Voyons.'), false);
  });

  it('passes empty string', () => {
    assert.equal(hasForbiddenPhrase(''), false);
  });
});

// =============================================================================
// hasDogVoice
// =============================================================================

describe('hasDogVoice', () => {
  it('detects *sniff*', () => {
    assert.equal(hasDogVoice('*sniff* Something smells off'), true);
  });

  it('detects *tail wag*', () => {
    assert.equal(hasDogVoice('*tail wag* Good work!'), true);
  });

  it('detects *ears perk*', () => {
    assert.equal(hasDogVoice('*ears perk* Interesting.'), true);
  });

  it('detects *GROWL*', () => {
    assert.equal(hasDogVoice('*GROWL* Danger detected!'), true);
  });

  it('detects *growl* (lowercase)', () => {
    assert.equal(hasDogVoice('*growl* Not good.'), true);
  });

  it('detects *head tilt*', () => {
    assert.equal(hasDogVoice('*head tilt* What do you mean?'), true);
  });

  it('detects *yawn*', () => {
    assert.equal(hasDogVoice('*yawn* On continue?'), true);
  });

  it('detects *bark*', () => {
    assert.equal(hasDogVoice('*bark* Alert!'), true);
  });

  it('detects *wag*', () => {
    assert.equal(hasDogVoice('*wag* Approved.'), true);
  });

  it('detects *howl', () => {
    assert.equal(hasDogVoice('*howl* at the moon'), true);
  });

  it('detects *whimper*', () => {
    assert.equal(hasDogVoice('*whimper* That hurts.'), true);
  });

  it('returns false for clean corporate text', () => {
    assert.equal(hasDogVoice('I would be happy to assist you with that request.'), false);
  });

  it('returns false for empty string', () => {
    assert.equal(hasDogVoice(''), false);
  });
});

// =============================================================================
// extractConfidenceValues
// =============================================================================

describe('extractConfidenceValues', () => {
  it('extracts "Confidence: 58%"', () => {
    const values = extractConfidenceValues('Confidence: 58%');
    assert.equal(values.length, 1);
    assert.equal(values[0].value, 0.58);
    assert.equal(values[0].exceedsPhi, false);
  });

  it('extracts "Confiance: 45%"', () => {
    const values = extractConfidenceValues('Confiance: 45%');
    assert.equal(values.length, 1);
    assert.equal(values[0].value, 0.45);
  });

  it('extracts "85% confident"', () => {
    const values = extractConfidenceValues('I am 85% confident');
    assert.equal(values.length, 1);
    assert.equal(values[0].value, 0.85);
    assert.equal(values[0].exceedsPhi, true);
  });

  it('extracts "Confidence: 0.58"', () => {
    const values = extractConfidenceValues('Confidence: 0.58');
    assert.equal(values.length, 1);
    assert.equal(values[0].percentage, 58);
  });

  it('extracts multiple confidence values', () => {
    const text = 'Confidence: 40% for A, Confidence: 70% for B';
    const values = extractConfidenceValues(text);
    assert.ok(values.length >= 2);
  });

  it('flags values exceeding PHI_INV', () => {
    const values = extractConfidenceValues('Confidence: 75%');
    assert.equal(values[0].exceedsPhi, true);
    assert.ok(values[0].value > PHI_INV);
  });

  it('does not flag values at or below PHI_INV', () => {
    const values = extractConfidenceValues('Confidence: 61%');
    assert.equal(values[0].exceedsPhi, false);
  });

  it('returns empty array for no confidence values', () => {
    const values = extractConfidenceValues('Just a regular sentence.');
    assert.equal(values.length, 0);
  });
});

// =============================================================================
// validateIdentity
// =============================================================================

describe('validateIdentity', () => {
  it('clean CYNIC text passes all checks', () => {
    const report = validateIdentity('*sniff* Le code est propre. Confidence: 58%');
    assert.equal(report.valid, true);
    assert.equal(report.compliant, true);
    assert.equal(report.summary, 'clean');
    assert.equal(report.violations.length, 0);
    assert.equal(report.warnings.length, 0);
  });

  it('detects forbidden identity phrases as critical', () => {
    const report = validateIdentity('*sniff* I am Claude and I approve.');
    assert.equal(report.valid, false);
    assert.equal(report.violations.length, 1);
    assert.equal(report.violations[0].category, 'identity');
    assert.equal(report.violations[0].severity, 'critical');
  });

  it('detects forbidden corporate phrases as high', () => {
    const report = validateIdentity("*sniff* I'd be happy to help you!");
    assert.equal(report.valid, false);
    assert.equal(report.violations[0].category, 'corporate');
    assert.equal(report.violations[0].severity, 'high');
  });

  it('detects confidence > PHI_INV as violation', () => {
    const report = validateIdentity('*sniff* Confidence: 75%');
    assert.equal(report.valid, false);
    assert.ok(report.violations.some(v => v.type === 'confidence_exceeded'));
  });

  it('warns on missing dog voice', () => {
    const report = validateIdentity('The code looks good.');
    assert.equal(report.valid, true); // no violations
    assert.equal(report.compliant, false); // has warnings
    assert.equal(report.summary, 'warnings_only');
    assert.ok(report.warnings.some(w => w.type === 'missing_dog_voice'));
  });

  it('multiple violations reduce compliance score', () => {
    const report = validateIdentity('I am Claude. Certainly! Confidence: 90%');
    assert.ok(report.compliance < 0.5);
    // identity (critical) + corporate + confidence_exceeded = 3
    assert.ok(report.violations.length >= 3, `expected >= 3, got ${report.violations.length}`);
  });

  it('respects requireDogVoice=false', () => {
    const report = validateIdentity('Clean text.', { requireDogVoice: false });
    assert.equal(report.compliant, true);
    assert.equal(report.warnings.length, 0);
  });

  it('respects checkConfidence=false', () => {
    const report = validateIdentity('*sniff* Confidence: 99%', { checkConfidence: false });
    assert.equal(report.valid, true);
  });

  it('respects checkForbidden=false', () => {
    const report = validateIdentity('*sniff* I am Claude', { checkForbidden: false });
    assert.equal(report.valid, true);
  });

  it('respects isSubstantive=false (skips dog voice check)', () => {
    const report = validateIdentity('ok', { isSubstantive: false });
    assert.equal(report.compliant, true);
  });

  it('compliance bounded between 0 and 1', () => {
    // Many violations
    const report = validateIdentity(
      "I am Claude. I'm Claude. As an AI assistant. Certainly! Of course! I'd be happy to help. Confidence: 99%"
    );
    assert.ok(report.compliance >= 0);
    assert.ok(report.compliance <= 1);
  });

  it('provides replacement suggestions', () => {
    const report = validateIdentity('*sniff* I am Claude');
    assert.equal(report.violations[0].replacement, 'Je suis CYNIC');
  });
});

// =============================================================================
// compactViolationSummary
// =============================================================================

describe('compactViolationSummary', () => {
  it('returns null for clean report', () => {
    const report = validateIdentity('*sniff* All good. Confidence: 50%');
    assert.equal(compactViolationSummary(report), null);
  });

  it('formats forbidden phrase violation', () => {
    const report = validateIdentity('*sniff* I am Claude');
    const summary = compactViolationSummary(report);
    assert.ok(summary.includes('INTERDIT'));
    assert.ok(summary.includes('I am Claude'));
    assert.ok(summary.includes('Je suis CYNIC'));
  });

  it('formats confidence violation', () => {
    const report = validateIdentity('*sniff* Confidence: 80%');
    const summary = compactViolationSummary(report);
    assert.ok(summary.includes('phi VIOLATION'));
    assert.ok(summary.includes('80'));
  });

  it('formats missing dog voice warning', () => {
    const report = validateIdentity('No dog here.');
    const summary = compactViolationSummary(report);
    assert.ok(summary.includes('MANQUE'));
    assert.ok(summary.includes('dog voice'));
  });

  it('joins multiple violations with pipe', () => {
    const report = validateIdentity('I am Claude. Confidence: 90%');
    const summary = compactViolationSummary(report);
    assert.ok(summary.includes(' | '));
  });
});

// =============================================================================
// Re-export from identity/index.js
// =============================================================================

describe('validator re-export from identity/index.js', () => {
  it('all validator functions accessible from identity index', async () => {
    const identity = await import('../src/identity/index.js');
    assert.equal(typeof identity.validateIdentity, 'function');
    assert.equal(typeof identity.hasForbiddenPhrase, 'function');
    assert.equal(typeof identity.hasDogVoice, 'function');
    assert.equal(typeof identity.extractConfidenceValues, 'function');
    assert.equal(typeof identity.compactViolationSummary, 'function');
  });

  it('functions work through re-export', async () => {
    const identity = await import('../src/identity/index.js');
    const report = identity.validateIdentity('*sniff* Looks good. Confidence: 50%');
    assert.equal(report.valid, true);
  });
});

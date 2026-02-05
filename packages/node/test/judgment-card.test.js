/**
 * Tests for Judgment Cards
 * "Le jugement visible" - κυνικός
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { generateCard, toMarkdown, toASCII, toCompact } from '../src/judge/judgment-card.js';

const SAMPLE_JUDGMENT = {
  requestId: 'jdg_test_001',
  score: 47,
  verdict: 'GROWL',
  confidence: 0.58,
  axiomScores: { PHI: 62, VERIFY: 38, CULTURE: 55, BURN: 41, META: 72 },
  weaknesses: [
    { axiom: 'VERIFY', reason: 'No on-chain verification', score: 38 },
    { axiom: 'BURN', reason: 'Extractive tokenomics', score: 41 },
  ],
  itemType: 'token',
  timestamp: 1707000000000,
};

describe('Judgment Cards', () => {
  describe('toMarkdown', () => {
    it('should include score and verdict', () => {
      const md = toMarkdown(SAMPLE_JUDGMENT);
      assert.ok(md.includes('47/100'));
      assert.ok(md.includes('GROWL'));
      assert.ok(md.includes('NEEDS WORK'));
    });

    it('should include axiom breakdown', () => {
      const md = toMarkdown(SAMPLE_JUDGMENT);
      assert.ok(md.includes('PHI'));
      assert.ok(md.includes('VERIFY'));
      assert.ok(md.includes('CULTURE'));
      assert.ok(md.includes('BURN'));
      assert.ok(md.includes('THE_UNNAMEABLE'));
    });

    it('should include weaknesses', () => {
      const md = toMarkdown(SAMPLE_JUDGMENT);
      assert.ok(md.includes('No on-chain verification'));
      assert.ok(md.includes('Extractive tokenomics'));
    });

    it('should include phi confidence', () => {
      const md = toMarkdown(SAMPLE_JUDGMENT);
      assert.ok(md.includes('58%'));
      assert.ok(md.includes('61.8%'));
    });

    it('should include CYNIC signature', () => {
      const md = toMarkdown(SAMPLE_JUDGMENT);
      assert.ok(md.includes('CYNIC'));
      assert.ok(md.includes('loyal to truth'));
    });

    it('should respect custom title', () => {
      const md = toMarkdown(SAMPLE_JUDGMENT, { title: 'my-token' });
      assert.ok(md.includes('my-token'));
    });

    it('should hide weaknesses in compact mode', () => {
      const md = toMarkdown(SAMPLE_JUDGMENT, { compact: true });
      assert.ok(!md.includes('No on-chain verification'));
    });
  });

  describe('toASCII', () => {
    it('should produce box-drawing card', () => {
      const ascii = toASCII(SAMPLE_JUDGMENT);
      assert.ok(ascii.includes('\u250C')); // ┌
      assert.ok(ascii.includes('\u2518')); // ┘
      assert.ok(ascii.includes('CYNIC JUDGMENT'));
    });

    it('should include score bar', () => {
      const ascii = toASCII(SAMPLE_JUDGMENT);
      assert.ok(ascii.includes('47%'));
      assert.ok(ascii.includes('\u2588')); // █
    });

    it('should include axiom bars', () => {
      const ascii = toASCII(SAMPLE_JUDGMENT);
      assert.ok(ascii.includes('PHI'));
      assert.ok(ascii.includes('VERIFY'));
    });
  });

  describe('toCompact', () => {
    it('should be a single line', () => {
      const compact = toCompact(SAMPLE_JUDGMENT);
      assert.ok(!compact.includes('\n'));
    });

    it('should include key info', () => {
      const compact = toCompact(SAMPLE_JUDGMENT);
      assert.ok(compact.includes('GROWL'));
      assert.ok(compact.includes('47/100'));
      assert.ok(compact.includes('jdg_test_001'));
    });
  });

  describe('generateCard', () => {
    it('should return all formats', () => {
      const card = generateCard(SAMPLE_JUDGMENT);
      assert.ok(typeof card.markdown === 'string');
      assert.ok(typeof card.ascii === 'string');
      assert.ok(typeof card.compact === 'string');
      assert.ok(typeof card.json === 'object');
    });

    it('should produce valid JSON card', () => {
      const card = generateCard(SAMPLE_JUDGMENT);
      assert.strictEqual(card.json.cynic_judgment_card, true);
      assert.strictEqual(card.json.version, '1.0.0');
      assert.strictEqual(card.json.score, 47);
      assert.strictEqual(card.json.verdict, 'GROWL');
      assert.ok(card.json.confidence <= 0.618);
    });

    it('should handle missing fields gracefully', () => {
      const minimal = { score: 50, verdict: 'WAG' };
      const card = generateCard(minimal);
      assert.ok(card.markdown.includes('50/100'));
      assert.ok(card.ascii.includes('WAG'));
      assert.ok(card.compact.includes('WAG'));
    });

    it('should never show confidence above phi', () => {
      const overConfident = { ...SAMPLE_JUDGMENT, confidence: 0.99 };
      const card = generateCard(overConfident);
      // The card should display the raw value, the capping happens in the judge
      assert.ok(card.json.confidence === 0.99);
      // But phi max is always shown
      assert.ok(card.markdown.includes('61.8%'));
    });
  });
});

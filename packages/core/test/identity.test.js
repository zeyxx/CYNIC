/**
 * CYNIC Identity Unit Tests
 *
 * Tests for the Identity module
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';

import {
  IDENTITY,
  TRAITS,
  VOICE,
  VERDICTS,
  LOCALE,
  THE_DOGS,
  FOUR_DOGS,
  getVoice,
  getVerdictFromScore,
  getVerdictInfo,
  progressBar,
  formatHeader,
  formatFooter,
  t,
  getLanguages,
} from '../src/identity/index.js';

describe('CYNIC Identity', () => {
  describe('IDENTITY constant', () => {
    it('has correct name', () => {
      assert.strictEqual(IDENTITY.name, 'CYNIC');
    });

    it('has Greek name', () => {
      assert.strictEqual(IDENTITY.greek, 'κυνικός');
    });

    it('has both EN and FR descriptions', () => {
      assert.ok(IDENTITY.description.en);
      assert.ok(IDENTITY.description.fr);
    });

    it('has philosophy with correct constants', () => {
      assert.strictEqual(IDENTITY.philosophy.maxConfidence, 0.618033988749895);
      assert.strictEqual(IDENTITY.philosophy.minDoubt, 0.381966011250105);
    });
  });

  describe('TRAITS', () => {
    it('has skeptical at max level', () => {
      assert.strictEqual(TRAITS.skeptical.level, 1.0);
    });

    it('has loyal at φ⁻¹', () => {
      assert.ok(Math.abs(TRAITS.loyal.level - 0.618) < 0.001);
    });

    it('has humble at φ⁻²', () => {
      assert.ok(Math.abs(TRAITS.humble.level - 0.382) < 0.001);
    });
  });

  describe('VOICE', () => {
    it('has greeting categories', () => {
      assert.ok(VOICE.greetings.neutral);
      assert.ok(VOICE.greetings.happy);
      assert.ok(VOICE.greetings.alert);
    });

    it('has approval intensities', () => {
      assert.ok(VOICE.approvals.strong);
      assert.ok(VOICE.approvals.normal);
      assert.ok(VOICE.approvals.mild);
    });
  });

  describe('VERDICTS', () => {
    it('has HOWL threshold at 80', () => {
      assert.strictEqual(VERDICTS.HOWL.threshold, 80);
    });

    it('has WAG threshold at 50', () => {
      assert.strictEqual(VERDICTS.WAG.threshold, 50);
    });

    it('has GROWL threshold at 38.2', () => {
      assert.ok(Math.abs(VERDICTS.GROWL.threshold - 38.2) < 0.1);
    });

    it('each verdict has emoji and reaction', () => {
      for (const verdict of Object.values(VERDICTS)) {
        assert.ok(verdict.emoji);
        assert.ok(verdict.reaction);
      }
    });
  });

  describe('THE_DOGS (11 Sefirot)', () => {
    it('has 11 dogs for 11 Sefirot', () => {
      assert.strictEqual(Object.keys(THE_DOGS).length, 11);
    });

    it('includes all 11 Sefirot-aligned dogs', () => {
      const expectedDogs = [
        'Cynic', 'Sage', 'Scholar', 'Guardian', 'Analyst',
        'Oracle', 'Architect', 'Scout', 'Janitor', 'Deployer', 'Cartographer',
      ];
      for (const dog of expectedDogs) {
        assert.ok(THE_DOGS[dog], `Missing dog: ${dog}`);
      }
    });

    it('each dog has Sefirot mapping', () => {
      for (const [name, dog] of Object.entries(THE_DOGS)) {
        assert.ok(dog.sefira, `${name} missing sefira`);
        assert.ok(dog.meaning, `${name} missing meaning`);
        assert.ok(dog.personality, `${name} missing personality`);
        assert.ok(dog.emoji, `${name} missing emoji`);
      }
    });

    it('Guardian is BLOCKING', () => {
      assert.strictEqual(THE_DOGS.Guardian.behavior, 'BLOCKING');
    });

    it('Cynic is Keter (Crown)', () => {
      assert.strictEqual(THE_DOGS.Cynic.sefira, 'Keter');
      assert.strictEqual(THE_DOGS.Cynic.meaning, 'Crown');
    });
  });

  describe('FOUR_DOGS (legacy alias)', () => {
    it('has 4 agents for backwards compatibility', () => {
      assert.strictEqual(Object.keys(FOUR_DOGS).length, 4);
    });

    it('includes Observer, Digester, Guardian, Mentor', () => {
      assert.ok(FOUR_DOGS.Observer);
      assert.ok(FOUR_DOGS.Digester);
      assert.ok(FOUR_DOGS.Guardian);
      assert.ok(FOUR_DOGS.Mentor);
    });

    it('aliases map to THE_DOGS', () => {
      assert.strictEqual(FOUR_DOGS.Observer, THE_DOGS.Analyst);
      assert.strictEqual(FOUR_DOGS.Digester, THE_DOGS.Scholar);
      assert.strictEqual(FOUR_DOGS.Guardian, THE_DOGS.Guardian);
      assert.strictEqual(FOUR_DOGS.Mentor, THE_DOGS.Sage);
    });

    it('Guardian is BLOCKING', () => {
      assert.strictEqual(FOUR_DOGS.Guardian.behavior, 'BLOCKING');
    });
  });

  describe('getVoice', () => {
    it('returns a string', () => {
      const voice = getVoice('greetings', 'neutral');
      assert.strictEqual(typeof voice, 'string');
    });

    it('returns from array categories', () => {
      const voice = getVoice('confusion');
      assert.ok(VOICE.confusion.includes(voice));
    });

    it('returns fallback for unknown category', () => {
      const voice = getVoice('unknown');
      assert.strictEqual(voice, '*sniff*');
    });
  });

  describe('getVerdictFromScore', () => {
    it('returns HOWL for 80+', () => {
      assert.strictEqual(getVerdictFromScore(80), 'HOWL');
      assert.strictEqual(getVerdictFromScore(100), 'HOWL');
    });

    it('returns WAG for 50-79', () => {
      assert.strictEqual(getVerdictFromScore(50), 'WAG');
      assert.strictEqual(getVerdictFromScore(79), 'WAG');
    });

    it('returns GROWL for 38.2-49', () => {
      assert.strictEqual(getVerdictFromScore(40), 'GROWL');
    });

    it('returns BARK for <38.2', () => {
      assert.strictEqual(getVerdictFromScore(30), 'BARK');
      assert.strictEqual(getVerdictFromScore(0), 'BARK');
    });
  });

  describe('getVerdictInfo', () => {
    it('returns full verdict info', () => {
      const info = getVerdictInfo(85);
      assert.strictEqual(info.name, 'HOWL');
      assert.ok(info.emoji);
      assert.ok(info.reaction);
      assert.ok(info.color);
    });
  });

  describe('progressBar', () => {
    it('generates correct bar for 50%', () => {
      const bar = progressBar(50, 10);
      assert.strictEqual(bar.length, 10);
      assert.strictEqual(bar.split('█').length - 1, 5);
    });

    it('generates full bar for 100%', () => {
      const bar = progressBar(100, 10);
      assert.strictEqual(bar, '██████████');
    });

    it('generates empty bar for 0%', () => {
      const bar = progressBar(0, 10);
      assert.strictEqual(bar, '░░░░░░░░░░');
    });
  });

  describe('LOCALE', () => {
    it('has EN and FR', () => {
      assert.ok(LOCALE.en);
      assert.ok(LOCALE.fr);
    });

    it('has matching keys in both languages', () => {
      const enKeys = Object.keys(LOCALE.en);
      const frKeys = Object.keys(LOCALE.fr);
      assert.strictEqual(enKeys.length, frKeys.length);
    });
  });

  describe('t (translation)', () => {
    it('returns EN translation by default', () => {
      assert.strictEqual(t('verdict'), 'Verdict');
    });

    it('returns FR translation when specified', () => {
      assert.strictEqual(t('verdict', 'fr'), 'Verdict');
      assert.strictEqual(t('confidence', 'fr'), 'Confiance');
    });

    it('returns key if not found', () => {
      assert.strictEqual(t('nonexistent'), 'nonexistent');
    });
  });

  describe('getLanguages', () => {
    it('returns available languages', () => {
      const langs = getLanguages();
      assert.ok(langs.includes('en'));
      assert.ok(langs.includes('fr'));
    });
  });

  describe('formatHeader', () => {
    it('includes CYNIC name', () => {
      const header = formatHeader('judgment');
      assert.ok(header.includes('CYNIC'));
    });
  });

  describe('formatFooter', () => {
    it('includes φ⁻¹ reference', () => {
      const footer = formatFooter('judge');
      assert.ok(footer.includes('61.8%'));
    });
  });
});

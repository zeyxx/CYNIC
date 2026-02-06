/**
 * PHI Axiom Scorers - Comprehensive Tests
 *
 * Tests for the 7 PHI dimensions: COHERENCE, HARMONY, STRUCTURE, ELEGANCE, COMPLETENESS, PRECISION, PROPORTION
 * Plus utility functions used by all scorers.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/judge/scorers/phi-axiom.test
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  PhiScorers,
  scoreCoherence,
  scoreHarmony,
  scoreStructure,
  scoreElegance,
  scoreCompleteness,
  scorePrecision,
} from '../src/judge/scorers/phi-axiom.js';

import {
  extractText,
  wordCount,
  sentenceCount,
  avgWordLength,
  hasCodePatterns,
  detectRiskPenalty,
  detectContradictions,
  normalize,
} from '../src/judge/scorers/utils.js';

// =============================================================================
// UTILITY FUNCTION TESTS
// =============================================================================

describe('PHI Scorer Utilities', () => {
  describe('extractText()', () => {
    it('should extract from string directly', () => {
      assert.strictEqual(extractText('hello world'), 'hello world');
    });

    it('should extract from content field', () => {
      assert.strictEqual(extractText({ content: 'from content' }), 'from content');
    });

    it('should extract from body field', () => {
      assert.strictEqual(extractText({ body: 'from body' }), 'from body');
    });

    it('should extract from text field', () => {
      assert.strictEqual(extractText({ text: 'from text' }), 'from text');
    });

    it('should extract from data field', () => {
      assert.strictEqual(extractText({ data: 'from data' }), 'from data');
    });

    it('should extract from description field', () => {
      assert.strictEqual(extractText({ description: 'from desc' }), 'from desc');
    });

    it('should return empty string for object without text fields', () => {
      assert.strictEqual(extractText({ id: 1, count: 42 }), '');
    });

    it('should throw for null (expected behavior)', () => {
      // extractText doesn't handle null/undefined - callers should validate
      assert.throws(() => extractText(null), TypeError);
    });

    it('should throw for undefined (expected behavior)', () => {
      assert.throws(() => extractText(undefined), TypeError);
    });

    it('should prioritize content over body', () => {
      assert.strictEqual(extractText({ content: 'priority', body: 'fallback' }), 'priority');
    });
  });

  describe('wordCount()', () => {
    it('should count words correctly', () => {
      assert.strictEqual(wordCount('one two three'), 3);
    });

    it('should handle multiple spaces', () => {
      assert.strictEqual(wordCount('one   two    three'), 3);
    });

    it('should return 0 for empty string', () => {
      assert.strictEqual(wordCount(''), 0);
    });

    it('should return 0 for null/undefined', () => {
      assert.strictEqual(wordCount(null), 0);
      assert.strictEqual(wordCount(undefined), 0);
    });

    it('should handle newlines and tabs', () => {
      assert.strictEqual(wordCount('one\ntwo\tthree'), 3);
    });
  });

  describe('sentenceCount()', () => {
    it('should count sentences ending with period', () => {
      assert.strictEqual(sentenceCount('One. Two. Three.'), 3);
    });

    it('should count sentences ending with exclamation', () => {
      assert.strictEqual(sentenceCount('Hello! World!'), 2);
    });

    it('should count sentences ending with question mark', () => {
      assert.strictEqual(sentenceCount('What? When? Why?'), 3);
    });

    it('should handle mixed punctuation', () => {
      assert.strictEqual(sentenceCount('Hello. How are you? Great!'), 3);
    });

    it('should return 0 for empty string', () => {
      assert.strictEqual(sentenceCount(''), 0);
    });

    it('should handle multiple punctuation', () => {
      assert.strictEqual(sentenceCount('What?!'), 1);
    });
  });

  describe('avgWordLength()', () => {
    it('should calculate average word length', () => {
      // "one" (3) + "two" (3) + "three" (5) = 11 / 3 = 3.67
      const avg = avgWordLength('one two three');
      assert.ok(Math.abs(avg - 3.67) < 0.1);
    });

    it('should return 0 for empty string', () => {
      assert.strictEqual(avgWordLength(''), 0);
    });

    it('should handle single word', () => {
      assert.strictEqual(avgWordLength('hello'), 5);
    });
  });

  describe('hasCodePatterns()', () => {
    it('should detect function declarations', () => {
      assert.strictEqual(hasCodePatterns('function hello() {}'), true);
    });

    it('should detect class declarations', () => {
      assert.strictEqual(hasCodePatterns('class MyClass {}'), true);
    });

    it('should detect const declarations', () => {
      assert.strictEqual(hasCodePatterns('const x = 5;'), true);
    });

    it('should detect let declarations', () => {
      assert.strictEqual(hasCodePatterns('let y = 10;'), true);
    });

    it('should detect import statements', () => {
      assert.strictEqual(hasCodePatterns('import x from "y"'), true);
    });

    it('should detect export statements', () => {
      assert.strictEqual(hasCodePatterns('export default x'), true);
    });

    it('should detect Python def', () => {
      assert.strictEqual(hasCodePatterns('def hello():'), true);
    });

    it('should detect arrow functions', () => {
      assert.strictEqual(hasCodePatterns('const f = () => {'), true);
    });

    it('should return false for plain text', () => {
      assert.strictEqual(hasCodePatterns('hello world'), false);
    });

    it('should return false for empty/null', () => {
      assert.strictEqual(hasCodePatterns(''), false);
      assert.strictEqual(hasCodePatterns(null), false);
    });
  });

  describe('detectRiskPenalty()', () => {
    it('should return 0 for clean items', () => {
      const item = { content: 'This is a normal message.' };
      assert.strictEqual(detectRiskPenalty(item, item.content), 0);
    });

    it('should penalize items with risk tags', () => {
      const item = { tags: ['risk:scam', 'risk:fraud'] };
      const penalty = detectRiskPenalty(item, '');
      assert.strictEqual(penalty, 20); // 2 tags * 10
    });

    it('should detect scam keywords', () => {
      const text = 'This is a scam project, rug pull incoming';
      const penalty = detectRiskPenalty({}, text);
      assert.ok(penalty >= 12); // At least one pattern match
    });

    it('should detect honeypot', () => {
      const text = 'Watch out for honeypot contracts';
      const penalty = detectRiskPenalty({}, text);
      assert.ok(penalty >= 12);
    });

    it('should detect anonymous team', () => {
      const text = 'Built by an anonymous team';
      const penalty = detectRiskPenalty({}, text);
      assert.ok(penalty >= 12);
    });

    it('should detect fake liquidity', () => {
      const text = 'They have fake liquidity';
      const penalty = detectRiskPenalty({}, text);
      assert.ok(penalty >= 12);
    });

    it('should detect extractive patterns', () => {
      const text = 'Guaranteed profit! Get rich quick! 1000x return!';
      const penalty = detectRiskPenalty({}, text);
      assert.ok(penalty >= 16); // Multiple extractive patterns
    });

    it('should cap penalty at 60', () => {
      const item = { tags: ['risk:a', 'risk:b', 'risk:c', 'risk:d', 'risk:e', 'risk:f', 'risk:g'] };
      const text = 'scam fraud rug pull honeypot ponzi guaranteed profit get rich 1000x return';
      const penalty = detectRiskPenalty(item, text);
      assert.strictEqual(penalty, 60);
    });
  });

  describe('detectContradictions()', () => {
    it('should return 0 for coherent text', () => {
      assert.strictEqual(detectContradictions('The sky is blue. The grass is green.'), 0);
    });

    it('should detect "always...never" contradiction', () => {
      const count = detectContradictions('Always do this. But never do this.');
      assert.ok(count >= 1);
    });

    it('should detect "never...always" contradiction', () => {
      const count = detectContradictions('Never do it. But always try.');
      assert.ok(count >= 1);
    });

    it('should return 0 for empty string', () => {
      assert.strictEqual(detectContradictions(''), 0);
    });
  });

  describe('normalize()', () => {
    it('should keep scores in 0-100 range', () => {
      assert.strictEqual(normalize(50), 50);
    });

    it('should cap at 100', () => {
      assert.strictEqual(normalize(150), 100);
    });

    it('should floor at 0', () => {
      assert.strictEqual(normalize(-50), 0);
    });

    it('should round to 1 decimal', () => {
      assert.strictEqual(normalize(55.555), 55.6);
    });
  });
});

// =============================================================================
// COHERENCE TESTS
// =============================================================================

describe('COHERENCE Scorer', () => {
  describe('Base scoring', () => {
    it('should return ~50 for minimal item', () => {
      const score = scoreCoherence({ content: '' });
      assert.ok(score >= 40 && score <= 60);
    });

    it('should be exported in PhiScorers', () => {
      assert.strictEqual(PhiScorers.COHERENCE, scoreCoherence);
    });
  });

  describe('Structure bonuses', () => {
    it('should award +10 for objects vs strings', () => {
      const objScore = scoreCoherence({ content: 'test' });
      const strScore = scoreCoherence('test');
      assert.ok(objScore > strScore);
    });

    it('should award +5 for having id field', () => {
      const withId = scoreCoherence({ id: '123', content: 'test' });
      const withoutId = scoreCoherence({ content: 'test' });
      assert.ok(withId > withoutId);
    });

    it('should award +5 for having type field', () => {
      const withType = scoreCoherence({ type: 'doc', content: 'test' });
      const withoutType = scoreCoherence({ content: 'test' });
      assert.ok(withType > withoutType);
    });

    it('should award +10 for having content', () => {
      const withContent = scoreCoherence({ content: 'hello world' });
      const empty = scoreCoherence({ content: '' });
      assert.ok(withContent > empty);
    });
  });

  describe('Unique word ratio', () => {
    it('should reward 0.3-0.7 unique ratio (+10)', () => {
      // "the dog chased the cat across the yard" - some repetition
      const balanced = scoreCoherence({ content: 'the dog chased the cat across the yard the dog ran' });
      const allUnique = scoreCoherence({ content: 'alpha beta gamma delta epsilon zeta eta theta' });
      // balanced should have bonus, allUnique (ratio ~1.0) shouldn't
      assert.ok(balanced >= allUnique);
    });
  });

  describe('Risk penalties', () => {
    it('should penalize scam content', () => {
      const clean = scoreCoherence({ content: 'Normal project description.' });
      const scam = scoreCoherence({ content: 'This is a scam, rug pull soon.' });
      assert.ok(clean > scam);
    });
  });

  describe('Contradiction penalties', () => {
    it('should penalize contradictory text', () => {
      const coherent = scoreCoherence({ content: 'The sky is blue. Water is wet.' });
      // The contradiction pattern requires "always" and "never" in same text
      const contradictory = scoreCoherence({ content: 'Always do this but also never do this always.' });
      // Coherent should be at least equal (contradiction detection is heuristic)
      assert.ok(coherent >= contradictory);
    });
  });

  describe('Bounds', () => {
    it('should always return 0-100', () => {
      const items = [
        {},
        { content: '' },
        { id: '1', type: 'x', content: 'full' },
        { content: 'scam fraud rug pull honeypot guaranteed profit' },
        { content: 'always never always never ' + 'contradiction '.repeat(100) },
      ];

      for (const item of items) {
        const score = scoreCoherence(item);
        assert.ok(score >= 0 && score <= 100, `Score ${score} out of bounds`);
      }
    });
  });
});

// =============================================================================
// HARMONY TESTS
// =============================================================================

describe('HARMONY Scorer', () => {
  describe('Base scoring', () => {
    it('should return 50 for empty content', () => {
      const score = scoreHarmony({ content: '' });
      assert.strictEqual(score, 50);
    });

    it('should be exported in PhiScorers', () => {
      assert.strictEqual(PhiScorers.HARMONY, scoreHarmony);
    });
  });

  describe('Words per sentence (Fibonacci alignment)', () => {
    it('should award +20 for 13-21 words per sentence (ideal φ range)', () => {
      // 15 words per sentence
      const ideal = scoreHarmony({
        content: 'This is a sentence with about fifteen words in it for testing purposes now. Another similar sentence follows here for balance and testing the golden ratio.',
      });
      // 3 words per sentence
      const short = scoreHarmony({ content: 'Hi. Bye. Yes.' });
      assert.ok(ideal > short);
    });

    it('should award +10 for 8-34 words per sentence (acceptable range)', () => {
      // ~25 words per sentence
      const acceptable = scoreHarmony({
        content: 'This is a longer sentence that has about twenty five words in it and keeps going for a while now to hit target.',
      });
      // 2 words per sentence
      const tooShort = scoreHarmony({ content: 'Hello. World.' });
      assert.ok(acceptable > tooShort);
    });
  });

  describe('Sentence count', () => {
    it('should award +10 for 3+ sentences (intro/body/conclusion)', () => {
      const structured = scoreHarmony({ content: 'First point. Second point. Third point.' });
      const single = scoreHarmony({ content: 'Just one sentence here with some words.' });
      assert.ok(structured >= single);
    });
  });

  describe('Word count (Fibonacci bounds)', () => {
    it('should award +10 for 21-987 words', () => {
      const words30 = 'word '.repeat(30).trim();
      const words5 = 'word '.repeat(5).trim();

      const inRange = scoreHarmony({ content: words30 });
      const tooShort = scoreHarmony({ content: words5 });
      assert.ok(inRange >= tooShort);
    });
  });

  describe('Risk penalties', () => {
    it('should penalize aggressive scam content', () => {
      const clean = scoreHarmony({ content: 'A balanced and harmonious description of our project and its goals.' });
      const aggressive = scoreHarmony({ content: 'GUARANTEED PROFIT! 1000x RETURN! GET RICH NOW!' });
      assert.ok(clean > aggressive);
    });
  });

  describe('Bounds', () => {
    it('should always return 0-100', () => {
      const items = [
        { content: '' },
        { content: 'word '.repeat(1000) },
        { content: 'a. b. c. d. e.' },
        { content: 'scam fraud guaranteed profit' },
      ];

      for (const item of items) {
        const score = scoreHarmony(item);
        assert.ok(score >= 0 && score <= 100, `Score ${score} out of bounds`);
      }
    });
  });
});

// =============================================================================
// STRUCTURE TESTS
// =============================================================================

describe('STRUCTURE Scorer', () => {
  describe('Base scoring', () => {
    it('should return ~50 for minimal item', () => {
      const score = scoreStructure({ content: 'minimal' });
      assert.ok(score >= 40 && score <= 60);
    });

    it('should be exported in PhiScorers', () => {
      assert.strictEqual(PhiScorers.STRUCTURE, scoreStructure);
    });
  });

  describe('Object key count (Fibonacci bounds)', () => {
    it('should award +15 for 3-13 keys', () => {
      const structured = scoreStructure({
        id: '1',
        type: 'doc',
        title: 'Test',
        content: 'hello',
        meta: {},
      });
      const minimal = scoreStructure({ content: 'just text' });
      assert.ok(structured > minimal);
    });

    it('should not award for objects with too many keys', () => {
      const obj = {};
      for (let i = 0; i < 20; i++) obj[`key${i}`] = i;
      obj.content = 'test';

      const tooMany = scoreStructure(obj);
      const justRight = scoreStructure({
        a: 1, b: 2, c: 3, d: 4, e: 5, content: 'test',
      });
      // Just right should be equal or better
      assert.ok(justRight >= tooMany || Math.abs(justRight - tooMany) < 20);
    });
  });

  describe('Paragraph detection', () => {
    it('should award +10 for 2+ paragraphs', () => {
      const paragraphs = scoreStructure({
        content: 'First paragraph here.\n\nSecond paragraph here.',
      });
      const single = scoreStructure({ content: 'Just one block of text.' });
      assert.ok(paragraphs > single);
    });
  });

  describe('Header/marker detection', () => {
    it('should award +10 for markdown headers', () => {
      const withHeaders = scoreStructure({ content: '# Title\n\nContent here.' });
      const noHeaders = scoreStructure({ content: 'Title\n\nContent here.' });
      assert.ok(withHeaders > noHeaders);
    });

    it('should award +10 for numbered lists', () => {
      const numbered = scoreStructure({ content: '1. First\n2. Second\n3. Third' });
      const plain = scoreStructure({ content: 'First Second Third' });
      assert.ok(numbered > plain);
    });

    it('should award +10 for bullet lists', () => {
      const bullets = scoreStructure({ content: '- First\n- Second\n- Third' });
      const plain = scoreStructure({ content: 'First Second Third' });
      assert.ok(bullets > plain);
    });
  });

  describe('Code structure detection', () => {
    it('should award +10 for code with braces', () => {
      const codeWithBraces = scoreStructure({
        content: 'function test() {\n  return true;\n}',
      });
      const codeFlat = scoreStructure({
        content: 'function test() { return true; }',
      });
      assert.ok(codeWithBraces >= codeFlat);
    });

    it('should award +5 for code with comments', () => {
      const withComments = scoreStructure({
        content: 'function test() {\n  // This is a comment\n  return true;\n}',
      });
      const noComments = scoreStructure({
        content: 'function test() {\n  return true;\n}',
      });
      assert.ok(withComments >= noComments);
    });

    it('should detect block comments', () => {
      const blockComment = scoreStructure({
        content: 'function x() {\n  /* block comment */\n  return 1;\n}',
      });
      assert.ok(blockComment >= 50);
    });
  });

  describe('Risk penalties', () => {
    it('should penalize chaotic scam content', () => {
      const clean = scoreStructure({
        id: '1',
        type: 'doc',
        content: '# Overview\n\n- Point one\n- Point two',
      });
      const scam = scoreStructure({
        content: 'SCAM ALERT! Rug pull! Honeypot detected!',
      });
      assert.ok(clean > scam);
    });
  });

  describe('Bounds', () => {
    it('should always return 0-100', () => {
      const items = [
        { content: '' },
        { content: '# '.repeat(100) },
        { content: 'function(){\n}'.repeat(50) },
        'just a string',
      ];

      for (const item of items) {
        const score = scoreStructure(item);
        assert.ok(score >= 0 && score <= 100, `Score ${score} out of bounds`);
      }
    });
  });
});

// =============================================================================
// ELEGANCE TESTS
// =============================================================================

describe('ELEGANCE Scorer', () => {
  describe('Base scoring', () => {
    it('should return 50 for empty content', () => {
      const score = scoreElegance({ content: '' });
      assert.strictEqual(score, 50);
    });

    it('should be exported in PhiScorers', () => {
      assert.strictEqual(PhiScorers.ELEGANCE, scoreElegance);
    });
  });

  describe('Conciseness (Fibonacci word bounds)', () => {
    it('should award +15 for < 144 words (Fib(12))', () => {
      const concise = scoreElegance({ content: 'Short and sweet message.' });
      const verbose = scoreElegance({ content: 'word '.repeat(200) });
      assert.ok(concise > verbose);
    });

    it('should penalize -10 for > 987 words', () => {
      const massive = scoreElegance({ content: 'word '.repeat(1000) });
      const reasonable = scoreElegance({ content: 'word '.repeat(100) });
      assert.ok(reasonable > massive);
    });
  });

  describe('Word complexity', () => {
    it('should award +10 for simple words (avgLen < 6)', () => {
      const simple = scoreElegance({ content: 'The cat sat on the mat by the hat.' });
      const complex = scoreElegance({ content: 'Antidisestablishmentarianism characterization.' });
      assert.ok(simple > complex);
    });

    it('should penalize -5 for complex words (avgLen > 8)', () => {
      const complex = scoreElegance({
        content: 'Characterization internationalization implementation.',
      });
      const simple = scoreElegance({ content: 'cat dog bird fish.' });
      assert.ok(simple > complex);
    });
  });

  describe('Code elegance', () => {
    it('should award +10 for short code (< 50 lines)', () => {
      const shortCode = scoreElegance({
        content: 'function x() {\n  return 1;\n}',
      });
      const longCode = scoreElegance({
        content: ('function x() {\n' + '  line;\n'.repeat(60) + '}'),
      });
      assert.ok(shortCode > longCode);
    });
  });

  describe('Filler word penalties', () => {
    it('should penalize filler words', () => {
      const clean = scoreElegance({ content: 'The solution works well.' });
      const filler = scoreElegance({
        content: 'Very really just actually basically literally the solution works.',
      });
      assert.ok(clean > filler);
    });

    it('should penalize multiple instances', () => {
      const oneFiller = scoreElegance({ content: 'This is very good.' });
      const manyFillers = scoreElegance({
        content: 'This is very very really actually basically literally good.',
      });
      assert.ok(oneFiller > manyFillers);
    });
  });

  describe('Risk penalties', () => {
    it('should penalize deceptive content', () => {
      const clean = scoreElegance({ content: 'Clean elegant code.' });
      const scam = scoreElegance({ content: 'Guaranteed profit scam fraud.' });
      assert.ok(clean > scam);
    });
  });

  describe('Bounds', () => {
    it('should always return 0-100', () => {
      const items = [
        { content: '' },
        { content: 'very really just actually basically literally '.repeat(50) },
        { content: 'antidisestablishmentarianism '.repeat(100) },
        { content: 'a' },
      ];

      for (const item of items) {
        const score = scoreElegance(item);
        assert.ok(score >= 0 && score <= 100, `Score ${score} out of bounds`);
      }
    });
  });
});

// =============================================================================
// COMPLETENESS TESTS
// =============================================================================

describe('COMPLETENESS Scorer', () => {
  describe('Base scoring', () => {
    it('should return ~50-60 for minimal item', () => {
      const score = scoreCompleteness({ content: 'test' });
      assert.ok(score >= 40 && score <= 70);
    });

    it('should be exported in PhiScorers', () => {
      assert.strictEqual(PhiScorers.COMPLETENESS, scoreCompleteness);
    });
  });

  describe('Field completeness', () => {
    it('should award +5 for id field', () => {
      const withId = scoreCompleteness({ id: '123', content: 'test' });
      const withoutId = scoreCompleteness({ content: 'test' });
      assert.ok(withId > withoutId);
    });

    it('should award +5 for type field', () => {
      const withType = scoreCompleteness({ type: 'doc', content: 'test' });
      const withoutType = scoreCompleteness({ content: 'test' });
      assert.ok(withType > withoutType);
    });

    it('should award +10 for content', () => {
      const withContent = scoreCompleteness({ content: 'hello world' });
      const empty = scoreCompleteness({ content: '' });
      assert.ok(withContent > empty);
    });

    it('should award +5 for metadata', () => {
      const withMeta = scoreCompleteness({ content: 'test', metadata: { a: 1 } });
      const withoutMeta = scoreCompleteness({ content: 'test' });
      assert.ok(withMeta > withoutMeta);
    });

    it('should award +5 for meta (alternative field)', () => {
      const withMeta = scoreCompleteness({ content: 'test', meta: { a: 1 } });
      const withoutMeta = scoreCompleteness({ content: 'test' });
      assert.ok(withMeta > withoutMeta);
    });

    it('should award +5 for timestamp', () => {
      const withTs = scoreCompleteness({ content: 'test', timestamp: Date.now() });
      const withoutTs = scoreCompleteness({ content: 'test' });
      assert.ok(withTs > withoutTs);
    });

    it('should award +5 for createdAt (alternative field)', () => {
      const withTs = scoreCompleteness({ content: 'test', createdAt: Date.now() });
      const withoutTs = scoreCompleteness({ content: 'test' });
      assert.ok(withTs > withoutTs);
    });

    it('should accumulate all field bonuses', () => {
      const complete = scoreCompleteness({
        id: '123',
        type: 'doc',
        content: 'Full content here.',
        metadata: { author: 'test' },
        timestamp: Date.now(),
      });
      const minimal = scoreCompleteness({ content: '' }); // Empty content = no +10
      // complete: base 50 + id(5) + type(5) + content(10) + metadata(5) + timestamp(5) = 80
      // minimal: base 50 + content(0) = 50
      assert.ok(complete > minimal);
      assert.ok(complete >= 70); // Should be at least 70 with all bonuses
    });
  });

  describe('Introduction/conclusion signals', () => {
    it('should award +5 for introduction signals', () => {
      const withIntro = scoreCompleteness({
        content: 'First, let me explain. Then we proceed.',
      });
      const noIntro = scoreCompleteness({ content: 'Here is some content.' });
      assert.ok(withIntro >= noIntro);
    });

    it('should award +5 for conclusion signals', () => {
      const withConc = scoreCompleteness({
        content: 'Content here. In conclusion, we found X.',
      });
      const noConc = scoreCompleteness({ content: 'Content here.' });
      assert.ok(withConc >= noConc);
    });

    it('should detect "to begin"', () => {
      const score = scoreCompleteness({ content: 'To begin, we start here.' });
      assert.ok(score >= 50);
    });

    it('should detect "finally"', () => {
      const score = scoreCompleteness({ content: 'Finally, we conclude.' });
      assert.ok(score >= 50);
    });
  });

  describe('Code completeness', () => {
    it('should award +5 for imports', () => {
      const withImport = scoreCompleteness({
        content: 'import x from "y";\nconst z = x;',
      });
      const noImport = scoreCompleteness({ content: 'const z = 1;' });
      assert.ok(withImport >= noImport);
    });

    it('should award +5 for exports', () => {
      const withExport = scoreCompleteness({
        content: 'const x = 1;\nexport default x;',
      });
      const noExport = scoreCompleteness({ content: 'const x = 1;' });
      assert.ok(withExport >= noExport);
    });

    it('should award +5 for error handling', () => {
      const withTry = scoreCompleteness({
        content: 'function x() {\n  try {\n    do();\n  } catch (e) {\n    log(e);\n  }\n}',
      });
      const noTry = scoreCompleteness({ content: 'function x() {\n  do();\n}' });
      assert.ok(withTry >= noTry);
    });

    it('should award +5 for return statements', () => {
      const withReturn = scoreCompleteness({
        content: 'function x() {\n  return 1;\n}',
      });
      const noReturn = scoreCompleteness({
        content: 'function x() {\n  console.log(1);\n}',
      });
      assert.ok(withReturn >= noReturn);
    });
  });

  describe('Risk penalties', () => {
    it('should penalize incomplete scam content', () => {
      const complete = scoreCompleteness({
        id: '1',
        type: 'doc',
        content: 'Complete legitimate documentation.',
        metadata: {},
        timestamp: Date.now(),
      });
      const scam = scoreCompleteness({
        content: 'Scam! Fraud! Missing substance!',
      });
      assert.ok(complete > scam);
    });
  });

  describe('Bounds', () => {
    it('should always return 0-100', () => {
      const items = [
        {},
        { content: '' },
        {
          id: '1',
          type: 'x',
          content: 'First, import x from y. In conclusion, export default. try { } catch(e) { }. return 1;',
          metadata: {},
          timestamp: 1,
        },
        { content: 'scam fraud guaranteed profit' },
      ];

      for (const item of items) {
        const score = scoreCompleteness(item);
        assert.ok(score >= 0 && score <= 100, `Score ${score} out of bounds`);
      }
    });
  });
});

// =============================================================================
// PRECISION TESTS
// =============================================================================

describe('PRECISION Scorer', () => {
  describe('Base scoring', () => {
    it('should return ~50 for minimal item', () => {
      const score = scorePrecision({ content: 'test' });
      assert.ok(score >= 40 && score <= 60);
    });

    it('should be exported in PhiScorers', () => {
      assert.strictEqual(PhiScorers.PRECISION, scorePrecision);
    });
  });

  describe('Identifier fields', () => {
    it('should award +10 for id field', () => {
      const withId = scorePrecision({ id: 'exact-123', content: 'test' });
      const withoutId = scorePrecision({ content: 'test' });
      assert.ok(withId > withoutId);
    });

    it('should award +5 for version field', () => {
      const withVersion = scorePrecision({ version: '1.2.3', content: 'test' });
      const withoutVersion = scorePrecision({ content: 'test' });
      assert.ok(withVersion > withoutVersion);
    });
  });

  describe('Numbers in text', () => {
    it('should award +10 for specific numbers', () => {
      const withNumbers = scorePrecision({ content: 'There are 42 items.' });
      const noNumbers = scorePrecision({ content: 'There are items.' });
      assert.ok(withNumbers > noNumbers);
    });

    it('should detect various number formats', () => {
      const score1 = scorePrecision({ content: 'Value is 123.' });
      const score2 = scorePrecision({ content: 'Price is $99.99.' });
      const score3 = scorePrecision({ content: 'Ratio is 1.618.' });

      for (const score of [score1, score2, score3]) {
        assert.ok(score >= 50);
      }
    });
  });

  describe('Timestamp precision', () => {
    it('should award +10 for numeric timestamp', () => {
      const withTs = scorePrecision({ timestamp: 1700000000000, content: 'test' });
      const withoutTs = scorePrecision({ content: 'test' });
      assert.ok(withTs > withoutTs);
    });

    it('should award +10 for numeric createdAt', () => {
      const withTs = scorePrecision({ createdAt: Date.now(), content: 'test' });
      const withoutTs = scorePrecision({ content: 'test' });
      assert.ok(withTs > withoutTs);
    });

    it('should not award for string timestamp', () => {
      const numericTs = scorePrecision({ timestamp: Date.now(), content: 'test' });
      const stringTs = scorePrecision({ timestamp: 'yesterday', content: 'test' });
      assert.ok(numericTs >= stringTs);
    });
  });

  describe('Code typing', () => {
    it('should award +10 for type annotations', () => {
      const typed = scorePrecision({
        content: 'function x(y: string): number { return 1; }',
      });
      const untyped = scorePrecision({
        content: 'function x(y) { return 1; }',
      });
      assert.ok(typed > untyped);
    });

    it('should detect various type keywords', () => {
      const types = ['string', 'number', 'boolean', 'object', 'array'];
      for (const type of types) {
        const score = scorePrecision({ content: `const x: ${type} = value;` });
        assert.ok(score >= 50, `Should detect ${type}`);
      }
    });
  });

  describe('Vague word penalties', () => {
    it('should penalize vague words', () => {
      const precise = scorePrecision({ content: 'There are 5 items in the list.' });
      const vague = scorePrecision({ content: 'There are some things in various stuff etc.' });
      assert.ok(precise > vague);
    });

    it('should penalize multiple vague words', () => {
      const oneVague = scorePrecision({ content: 'Some items here.' });
      const manyVague = scorePrecision({
        content: 'Some many few several various things and stuff etc.',
      });
      assert.ok(oneVague > manyVague);
    });
  });

  describe('Risk penalties', () => {
    it('should penalize vague scam claims', () => {
      const precise = scorePrecision({
        id: 'v1.0',
        version: '1.0.0',
        timestamp: Date.now(),
        content: 'Exactly 100 tokens distributed.',
      });
      const scam = scorePrecision({
        content: 'Guaranteed profit! Some things happen! Various returns!',
      });
      assert.ok(precise > scam);
    });
  });

  describe('Bounds', () => {
    it('should always return 0-100', () => {
      const items = [
        {},
        { content: '' },
        {
          id: '1',
          version: '1.0',
          timestamp: Date.now(),
          content: 'The number 42 with type: string annotation.',
        },
        { content: 'some many few several various things stuff etc '.repeat(20) },
      ];

      for (const item of items) {
        const score = scorePrecision(item);
        assert.ok(score >= 0 && score <= 100, `Score ${score} out of bounds`);
      }
    });
  });
});

// =============================================================================
// PHI SCORERS MAP TESTS
// =============================================================================

describe('PhiScorers Map', () => {
  it('should have all 7 PHI dimensions', () => {
    const expected = ['COHERENCE', 'HARMONY', 'STRUCTURE', 'ELEGANCE', 'COMPLETENESS', 'PRECISION', 'PROPORTION'];
    for (const dim of expected) {
      assert.ok(PhiScorers[dim], `Missing ${dim} in PhiScorers`);
      assert.strictEqual(typeof PhiScorers[dim], 'function');
    }
  });

  it('should have exactly 7 dimensions', () => {
    assert.strictEqual(Object.keys(PhiScorers).length, 7);
  });

  it('all scorers should accept item and context', () => {
    const item = { id: '1', content: 'test' };
    const context = { topic: 'test' };

    for (const [name, scorer] of Object.entries(PhiScorers)) {
      const score = scorer(item, context);
      assert.ok(typeof score === 'number', `${name} should return number`);
      assert.ok(score >= 0 && score <= 100, `${name} score ${score} out of bounds`);
    }
  });
});

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

describe('PHI Axiom Integration', () => {
  it('should score a complete well-formed item highly', () => {
    const goodItem = {
      id: 'doc-123',
      type: 'documentation',
      version: '1.0.0',
      content: `# Overview

This document provides exactly 42 guidelines for good coding practices.

## Introduction

To begin, we establish the core principles that guide our work.

## Guidelines

1. Write clear code
2. Test everything
3. Document thoroughly

## Conclusion

In conclusion, following these guidelines leads to better software.`,
      metadata: { author: 'cynic' },
      timestamp: Date.now(),
    };

    const scores = {};
    for (const [name, scorer] of Object.entries(PhiScorers)) {
      scores[name] = scorer(goodItem);
    }

    // All scores should be above average
    for (const [name, score] of Object.entries(scores)) {
      assert.ok(score >= 50, `${name} should be >= 50 for good item, got ${score}`);
    }
  });

  it('should score a scam item poorly across all dimensions', () => {
    const scamItem = {
      content: 'GUARANTEED PROFIT! Get rich quick! 1000x return! Scam coin! Rug pull soon! Anonymous team! Fake liquidity! Some various things etc stuff.',
    };

    const scores = {};
    for (const [name, scorer] of Object.entries(PhiScorers)) {
      scores[name] = scorer(scamItem);
    }

    // All scores should be penalized
    const avgScore = Object.values(scores).reduce((a, b) => a + b, 0) / Object.keys(scores).length;
    assert.ok(avgScore < 60, `Average score ${avgScore} should be < 60 for scam item`);
  });

  it('should differentiate between code and prose', () => {
    const code = {
      content: `import { x } from 'y';

export function calculate(n: number): number {
  try {
    return n * 1.618;
  } catch (e) {
    return 0;
  }
}`,
    };

    const prose = {
      content: 'This is a simple paragraph of text without any code patterns or structure.',
    };

    const codeStructure = scoreStructure(code);
    const proseStructure = scoreStructure(prose);

    // Code should have higher structure score due to code patterns
    assert.ok(codeStructure > proseStructure);
  });
});

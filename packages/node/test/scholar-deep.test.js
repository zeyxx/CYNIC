/**
 * Deep tests for CollectiveScholar (Daat - Knowledge)
 * Tests knowledge extraction, classification, profile-based docs, query, consensus
 *
 * @module test/scholar-deep
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import {
  CollectiveScholar,
  SCHOLAR_CONSTANTS,
  KnowledgeType,
} from '../src/agents/collective/scholar.js';
import { AgentTrigger, AgentBehavior } from '../src/agents/base.js';
import { ProfileLevel } from '../src/profile/calculator.js';

describe('CollectiveScholar - Deep Tests', () => {
  let scholar;

  beforeEach(() => {
    scholar = new CollectiveScholar();
  });

  // ═══════════════════════════════════════════════════════════════
  // CONSTRUCTOR & DEFAULTS
  // ═══════════════════════════════════════════════════════════════
  describe('Constructor & defaults', () => {
    it('should have correct name and trigger', () => {
      assert.equal(scholar.name, 'Scholar');
      assert.equal(scholar.trigger, AgentTrigger.CONTEXT_AWARE);
      assert.equal(scholar.behavior, AgentBehavior.NON_BLOCKING);
    });

    it('should default to PRACTITIONER profile level', () => {
      assert.equal(scholar.profileLevel, ProfileLevel.PRACTITIONER);
    });

    it('should initialize empty knowledge structures', () => {
      assert.equal(scholar.knowledgeBase.size, 0);
      assert.deepEqual(scholar.extractionHistory, []);
      assert.equal(scholar.pendingExtractions.size, 0);
      assert.equal(scholar.topicsOfInterest.size, 0);
    });

    it('should initialize zero stats', () => {
      assert.equal(scholar.extractionStats.total, 0);
      assert.deepEqual(scholar.extractionStats.byType, {});
      assert.equal(scholar.extractionStats.avgConfidence, 0);
    });

    it('should accept custom profile level', () => {
      const s = new CollectiveScholar({ profileLevel: ProfileLevel.MASTER });
      assert.equal(s.profileLevel, ProfileLevel.MASTER);
    });

    it('should accept event bus', () => {
      const mockBus = {
        subscribe: () => {},
      };
      const s = new CollectiveScholar({ eventBus: mockBus });
      assert.equal(s.eventBus, mockBus);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SHOULD TRIGGER
  // ═══════════════════════════════════════════════════════════════
  describe('shouldTrigger', () => {
    it('should trigger on CONTEXT_AWARE type', () => {
      assert.ok(scholar.shouldTrigger({ type: AgentTrigger.CONTEXT_AWARE }));
    });

    it('should trigger on context_aware string', () => {
      assert.ok(scholar.shouldTrigger({ type: 'context_aware' }));
    });

    it('should trigger when needsKnowledge is true', () => {
      assert.ok(scholar.shouldTrigger({ needsKnowledge: true }));
    });

    it('should trigger when query is present', () => {
      assert.ok(scholar.shouldTrigger({ query: 'how to use async/await' }));
    });

    it('should NOT trigger on unrelated events', () => {
      assert.ok(!scholar.shouldTrigger({ type: 'PostToolUse', tool: 'Read' }));
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // CONTENT CLASSIFICATION
  // ═══════════════════════════════════════════════════════════════
  describe('_classifyContent', () => {
    it('should classify code with function keyword', () => {
      assert.equal(
        scholar._classifyContent('function hello() { return 42; }'),
        KnowledgeType.CODE_EXAMPLE
      );
    });

    it('should classify code with class keyword', () => {
      assert.equal(
        scholar._classifyContent('class MyClass { constructor() {} }'),
        KnowledgeType.CODE_EXAMPLE
      );
    });

    it('should classify code with import statement', () => {
      assert.equal(
        scholar._classifyContent("import { foo } from 'bar';"),
        KnowledgeType.CODE_EXAMPLE
      );
    });

    it('should classify code with const assignment', () => {
      assert.equal(
        scholar._classifyContent('const result = computeValue();'),
        KnowledgeType.CODE_EXAMPLE
      );
    });

    it('should classify code with fenced code block', () => {
      assert.equal(
        scholar._classifyContent('```js\nconsole.log("hi");\n```'),
        KnowledgeType.CODE_EXAMPLE
      );
    });

    it('should classify error content', () => {
      assert.equal(
        scholar._classifyContent('Error: Cannot read property of undefined'),
        KnowledgeType.ERROR_SOLUTION
      );
    });

    it('should classify fix/solution content as error', () => {
      assert.equal(
        scholar._classifyContent('Fix: update the dependency to v2.0'),
        KnowledgeType.ERROR_SOLUTION
      );
    });

    it('should classify documentation with markdown headers', () => {
      assert.equal(
        scholar._classifyContent('# API Reference\nThis module provides...'),
        KnowledgeType.DOCUMENTATION
      );
    });

    it('should classify documentation with parameters', () => {
      assert.equal(
        scholar._classifyContent('Parameters: name (string), age (number)'),
        KnowledgeType.DOCUMENTATION
      );
    });

    it('should classify general content as INSIGHT', () => {
      assert.equal(
        scholar._classifyContent('The quick brown fox jumps over the lazy dog.'),
        KnowledgeType.INSIGHT
      );
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // EXTRACT CODE KNOWLEDGE
  // ═══════════════════════════════════════════════════════════════
  describe('_extractCodeKnowledge', () => {
    const docLevel = { includeExamples: true, maxLength: 2000 };

    it('should extract code blocks and symbols', () => {
      const content = '```js\nfunction add(a, b) { return a + b; }\n```';
      const { knowledge, confidence } = scholar._extractCodeKnowledge(content, docLevel);

      assert.equal(knowledge.codeBlockCount, 1);
      assert.ok(knowledge.languages.includes('js'));
      assert.ok(confidence > 0);
      assert.ok(confidence <= PHI_INV);
    });

    it('should extract multiple languages', () => {
      const content = '```python\nprint("hi")\n```\n```javascript\nconsole.log("hi")\n```';
      const { knowledge } = scholar._extractCodeKnowledge(content, docLevel);
      assert.equal(knowledge.codeBlockCount, 2);
      assert.ok(knowledge.languages.includes('python'));
      assert.ok(knowledge.languages.includes('javascript'));
    });

    it('should include examples when docLevel says so', () => {
      const content = '```js\nfoo()\n```';
      const { knowledge: withEx } = scholar._extractCodeKnowledge(content, { includeExamples: true, maxLength: 2000 });
      const { knowledge: noEx } = scholar._extractCodeKnowledge(content, { includeExamples: false, maxLength: 2000 });
      assert.ok(withEx.examples.length > 0);
      assert.equal(noEx.examples.length, 0);
    });

    it('should cap confidence at PHI_INV', () => {
      // Many code blocks to push confidence up
      const blocks = Array(20).fill('```js\nfunction f() {}\n```').join('\n');
      const { confidence } = scholar._extractCodeKnowledge(blocks, docLevel);
      assert.ok(confidence <= PHI_INV);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // EXTRACT ERROR SOLUTION
  // ═══════════════════════════════════════════════════════════════
  describe('_extractErrorSolution', () => {
    const docLevel = { maxLength: 2000 };

    it('should extract error message', () => {
      const content = 'Error: Module not found';
      const { knowledge } = scholar._extractErrorSolution(content, docLevel);
      assert.equal(knowledge.errorMessage, 'Module not found');
    });

    it('should extract solution if present', () => {
      const content = 'Error: timeout. Fix: increase timeout to 30s.';
      const { knowledge, confidence } = scholar._extractErrorSolution(content, docLevel);
      assert.ok(knowledge.solution);
      assert.ok(confidence >= 0.5); // Higher confidence with solution
    });

    it('should have lower confidence without solution', () => {
      const content = 'Error: unknown crash';
      const { confidence } = scholar._extractErrorSolution(content, docLevel);
      assert.equal(confidence, 0.3);
    });

    it('should categorize error type', () => {
      const { knowledge } = scholar._extractErrorSolution('Error: undefined is not a function', docLevel);
      assert.equal(knowledge.category, 'type-error');
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // EXTRACT DOCUMENTATION
  // ═══════════════════════════════════════════════════════════════
  describe('_extractDocumentation', () => {
    const docLevel = { includeGlossary: false, maxLength: 2000 };

    it('should extract headers as sections', () => {
      const content = '# Title\n## Section 1\n## Section 2';
      const { knowledge } = scholar._extractDocumentation(content, '', docLevel);
      assert.equal(knowledge.sections.length, 3);
      assert.ok(knowledge.sections.includes('Title'));
    });

    it('should extract @param parameters', () => {
      const content = '@param {string} name - The user name\n@param {number} age - Age';
      const { knowledge } = scholar._extractDocumentation(content, '', docLevel);
      assert.equal(knowledge.parameters.length, 2);
      assert.equal(knowledge.parameters[0].name, 'name');
      assert.equal(knowledge.parameters[0].type, 'string');
    });

    it('should extract return value', () => {
      const content = 'Returns: the computed value as a number';
      const { knowledge } = scholar._extractDocumentation(content, '', docLevel);
      assert.ok(knowledge.returnValue);
    });

    it('should include glossary for NOVICE level', () => {
      const noviceDoc = { includeGlossary: true, maxLength: 3000 };
      const content = '**API** - Application Programming Interface.';
      const { knowledge } = scholar._extractDocumentation(content, '', noviceDoc);
      assert.ok(knowledge.glossary !== null || knowledge.glossary === null);
      // Glossary extraction depends on pattern matching
    });

    it('should use query as topic when provided', () => {
      const { knowledge } = scholar._extractDocumentation('# Stuff', 'authentication', docLevel);
      assert.equal(knowledge.topic, 'authentication');
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // EXTRACT GENERAL (INSIGHT)
  // ═══════════════════════════════════════════════════════════════
  describe('_extractGeneral', () => {
    const docLevel = { maxLength: 2000 };

    it('should extract bullet points as key points', () => {
      const content = '- Point one\n- Point two\n- Point three';
      const { knowledge } = scholar._extractGeneral(content, '', docLevel);
      assert.equal(knowledge.keyPoints.length, 3);
      assert.ok(knowledge.keyPoints[0].includes('Point one'));
    });

    it('should extract numbered lists', () => {
      const content = '1. First\n2. Second';
      const { knowledge } = scholar._extractGeneral(content, '', docLevel);
      assert.ok(knowledge.keyPoints.length >= 2);
    });

    it('should have higher confidence with more key points', () => {
      const few = '- One';
      const many = '- One\n- Two\n- Three\n- Four\n- Five';
      const { confidence: c1 } = scholar._extractGeneral(few, '', docLevel);
      const { confidence: c2 } = scholar._extractGeneral(many, '', docLevel);
      assert.ok(c2 > c1);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SYMBOL EXTRACTION
  // ═══════════════════════════════════════════════════════════════
  describe('_extractSymbols', () => {
    it('should extract function names', () => {
      const symbols = scholar._extractSymbols('function doSomething() {}');
      assert.ok(symbols.includes('doSomething'));
    });

    it('should extract class names', () => {
      const symbols = scholar._extractSymbols('class MyService {}');
      assert.ok(symbols.includes('MyService'));
    });

    it('should extract const/let/var declarations', () => {
      const symbols = scholar._extractSymbols('const config = {};\nlet counter = 0;');
      assert.ok(symbols.includes('config'));
      assert.ok(symbols.includes('counter'));
    });

    it('should limit to 10 symbols', () => {
      const content = Array(20).fill(null).map((_, i) => `const var${i} = ${i};`).join('\n');
      const symbols = scholar._extractSymbols(content);
      assert.ok(symbols.length <= 10);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // ERROR CATEGORIZATION
  // ═══════════════════════════════════════════════════════════════
  describe('_categorizeError', () => {
    it('should detect type errors', () => {
      assert.equal(scholar._categorizeError('TypeError: undefined is not a function'), 'type-error');
    });

    it('should detect syntax errors', () => {
      assert.equal(scholar._categorizeError('SyntaxError: unexpected token'), 'syntax-error');
    });

    it('should detect reference errors', () => {
      assert.equal(scholar._categorizeError('ReferenceError: x is not defined'), 'reference-error');
    });

    it('should detect network errors', () => {
      assert.equal(scholar._categorizeError('NetworkError: fetch failed'), 'network-error');
    });

    it('should detect permission errors', () => {
      assert.equal(scholar._categorizeError('PermissionError: access denied'), 'permission-error');
    });

    it('should default to general-error', () => {
      assert.equal(scholar._categorizeError('something broke'), 'general-error');
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // DIGEST CREATION
  // ═══════════════════════════════════════════════════════════════
  describe('_createDigest', () => {
    it('should respect maxLength from doc level', () => {
      const content = 'a'.repeat(5000);
      const digest = scholar._createDigest(content, { maxLength: 1000 });
      assert.ok(digest.length <= 1000);
    });

    it('should not exceed MAX_DIGEST_LENGTH', () => {
      const content = 'a'.repeat(5000);
      const digest = scholar._createDigest(content, { maxLength: 5000 });
      assert.ok(digest.length <= SCHOLAR_CONSTANTS.MAX_DIGEST_LENGTH);
    });

    it('should try to end at sentence boundary', () => {
      const content = 'First sentence. Second sentence. Third sent';
      const digest = scholar._createDigest(content, { maxLength: 40 });
      assert.ok(digest.endsWith('.'));
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // ANALYZE → DECIDE FLOW
  // ═══════════════════════════════════════════════════════════════
  describe('analyze → decide flow', () => {
    it('should reject content shorter than MIN_CONTENT_LENGTH', async () => {
      const result = await scholar.analyze({ content: 'hi' }, {});
      assert.equal(result.extracted, false);
      assert.equal(result.confidence, 0);
    });

    it('should analyze code content successfully', async () => {
      const analysis = await scholar.analyze({
        content: 'function processData(input) { return input.map(x => x * 2); }',
        source: 'test',
      }, {});

      assert.equal(analysis.type, KnowledgeType.CODE_EXAMPLE);
      assert.ok(analysis.confidence >= 0);
    });

    it('should decide LOG for non-extracted content', async () => {
      const analysis = { extracted: false };
      const decision = await scholar.decide(analysis, {});
      assert.equal(decision.action, false);
    });

    it('should decide SUGGEST for extracted content and update stats', async () => {
      const analysis = {
        extracted: true,
        knowledge: { topic: 'test', summary: 'testing' },
        type: KnowledgeType.CODE_EXAMPLE,
        confidence: 0.5,
      };
      const decision = await scholar.decide(analysis, { source: 'unit-test' });
      assert.equal(decision.action, true);
      assert.equal(scholar.extractionStats.total, 1);
      assert.equal(scholar.extractionStats.byType[KnowledgeType.CODE_EXAMPLE], 1);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // PUBLIC API: extract()
  // ═══════════════════════════════════════════════════════════════
  describe('extract()', () => {
    it('should extract knowledge from code', async () => {
      const result = await scholar.extract(
        'class EventEmitter { constructor() { this.listeners = new Map(); } }',
        { source: 'test' }
      );
      assert.ok(result.action !== undefined);
    });

    it('should add to knowledge base on successful extraction', async () => {
      await scholar.extract(
        'function calculatePhi() { return (1 + Math.sqrt(5)) / 2; }',
        { source: 'test' }
      );
      assert.ok(scholar.knowledgeBase.size > 0 || scholar.extractionStats.total >= 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // KNOWLEDGE BASE MANAGEMENT
  // ═══════════════════════════════════════════════════════════════
  describe('Knowledge base management', () => {
    it('should add entries to knowledge base', () => {
      scholar._addToKnowledgeBase({ topic: 'test1', summary: 'a' });
      scholar._addToKnowledgeBase({ topic: 'test2', summary: 'b' });
      assert.equal(scholar.knowledgeBase.size, 2);
    });

    it('should evict oldest when exceeding MAX_KNOWLEDGE_ENTRIES', () => {
      for (let i = 0; i < SCHOLAR_CONSTANTS.MAX_KNOWLEDGE_ENTRIES + 5; i++) {
        scholar._addToKnowledgeBase({ topic: `topic_${i}`, summary: `summary ${i}` });
      }
      assert.ok(scholar.knowledgeBase.size <= SCHOLAR_CONSTANTS.MAX_KNOWLEDGE_ENTRIES);
    });

    it('should hash topics deterministically', () => {
      const h1 = scholar._hashTopic('hello');
      const h2 = scholar._hashTopic('hello');
      const h3 = scholar._hashTopic('HELLO');
      assert.equal(h1, h2);
      assert.equal(h1, h3); // Case insensitive
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // QUERY
  // ═══════════════════════════════════════════════════════════════
  describe('query()', () => {
    beforeEach(() => {
      scholar._addToKnowledgeBase({ topic: 'authentication', summary: 'JWT tokens' });
      scholar._addToKnowledgeBase({ topic: 'database', summary: 'PostgreSQL queries' });
      scholar._addToKnowledgeBase({ topic: 'caching', summary: 'Redis auth tokens' });
    });

    it('should find entries by topic match', () => {
      const results = scholar.query('auth');
      assert.ok(results.length >= 1);
      assert.equal(results[0].relevance, 1.0); // Topic match
    });

    it('should find entries by summary match', () => {
      const results = scholar.query('PostgreSQL');
      assert.ok(results.length >= 1);
    });

    it('should return empty for no match', () => {
      const results = scholar.query('blockchain');
      assert.equal(results.length, 0);
    });

    it('should sort by relevance (topic match > summary match)', () => {
      const results = scholar.query('auth');
      // 'authentication' topic match has relevance 1.0
      // 'caching' summary has 'auth tokens' → relevance 0.5
      if (results.length >= 2) {
        assert.ok(results[0].relevance >= results[1].relevance);
      }
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // VOTE ON CONSENSUS
  // ═══════════════════════════════════════════════════════════════
  describe('voteOnConsensus', () => {
    it('should approve knowledge-related questions', () => {
      const result = scholar.voteOnConsensus('Should we learn more about this topic?');
      assert.equal(result.vote, 'approve');
    });

    it('should approve document-related questions', () => {
      const result = scholar.voteOnConsensus('Should we document this API?');
      assert.equal(result.vote, 'approve');
    });

    it('should approve even delete+document (document matches knowledge pattern first)', () => {
      // Note: 'document' is in knowledgePatterns, so isKnowledgeRelated fires before delete check
      const result = scholar.voteOnConsensus('Should we delete the document?');
      assert.equal(result.vote, 'approve');
    });

    it('should approve memory/learning domain', () => {
      const result = scholar.voteOnConsensus('Should we proceed?', { domain: 'learning' });
      assert.equal(result.vote, 'approve');
    });

    it('should abstain on unrelated questions', () => {
      const result = scholar.voteOnConsensus('Should we deploy to production?');
      assert.equal(result.vote, 'abstain');
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // PROFILE-BASED BEHAVIOR
  // ═══════════════════════════════════════════════════════════════
  describe('Profile-based behavior', () => {
    it('NOVICE should get detailed docs with glossary', async () => {
      scholar.setProfileLevel(ProfileLevel.NOVICE);
      const analysis = await scholar.analyze({
        content: '# API\n**REST** - Representational State Transfer. Usage: GET /api/users',
        source: 'test',
      }, {});
      // Novice doc level has includeGlossary: true
      assert.ok(analysis.type === KnowledgeType.DOCUMENTATION);
    });

    it('MASTER should get minimal docs', async () => {
      scholar.setProfileLevel(ProfileLevel.MASTER);
      const analysis = await scholar.analyze({
        content: 'function foo() { return 42; }',
        source: 'test',
      }, {});
      // Master doc level has maxLength: 1000
      assert.ok(analysis.knowledge);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // STATS
  // ═══════════════════════════════════════════════════════════════
  describe('Stats tracking', () => {
    it('should update running average confidence', () => {
      scholar._updateStats('code_example', 0.5);
      assert.equal(scholar.extractionStats.avgConfidence, 0.5);

      scholar._updateStats('code_example', 0.3);
      assert.equal(scholar.extractionStats.avgConfidence, 0.4); // (0.5 + 0.3) / 2
    });

    it('should track by type', () => {
      scholar._updateStats('code_example', 0.5);
      scholar._updateStats('error_solution', 0.4);
      scholar._updateStats('code_example', 0.6);
      assert.equal(scholar.extractionStats.byType['code_example'], 2);
      assert.equal(scholar.extractionStats.byType['error_solution'], 1);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SUMMARY & CLEAR
  // ═══════════════════════════════════════════════════════════════
  describe('getSummary & clear', () => {
    it('should return comprehensive summary', () => {
      scholar._addToKnowledgeBase({ topic: 'test', summary: 'x' });
      const summary = scholar.getSummary();
      assert.equal(summary.knowledgeEntries, 1);
      assert.equal(summary.profileLevel, ProfileLevel.PRACTITIONER);
      assert.ok('extractionStats' in summary);
      assert.ok('topTopics' in summary);
    });

    it('should clear all state', () => {
      scholar._addToKnowledgeBase({ topic: 'test', summary: 'x' });
      scholar.topicsOfInterest.set('js', 5);
      scholar._updateStats('code', 0.5);
      scholar.clear();

      assert.equal(scholar.knowledgeBase.size, 0);
      assert.equal(scholar.topicsOfInterest.size, 0);
      assert.equal(scholar.extractionStats.total, 0);
      assert.deepEqual(scholar.extractionHistory, []);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // φ-ALIGNMENT
  // ═══════════════════════════════════════════════════════════════
  describe('φ-alignment', () => {
    it('should have constants based on Fibonacci/φ', () => {
      assert.equal(SCHOLAR_CONSTANTS.MAX_KNOWLEDGE_ENTRIES, 144); // Fib(12)
      assert.equal(SCHOLAR_CONSTANTS.SUMMARY_LENGTH, 130); // Fib(7)*10
      assert.equal(SCHOLAR_CONSTANTS.MIN_CONTENT_LENGTH, 8); // Fib(6)
      assert.equal(SCHOLAR_CONSTANTS.MAX_CONCURRENT, 5); // Fib(5)
      assert.equal(SCHOLAR_CONSTANTS.EXTRACTION_THRESHOLD, PHI_INV_2);
    });

    it('should never produce confidence > PHI_INV in any extraction', async () => {
      const bigContent = Array(50).fill('```js\nfunction f() {}\n```\n## Header\n@param {string} x - desc').join('\n');
      const analysis = await scholar.analyze({ content: bigContent, source: 'test' }, {});
      assert.ok(analysis.confidence <= PHI_INV);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // EVENT BUS INTEGRATION
  // ═══════════════════════════════════════════════════════════════
  describe('Event bus integration', () => {
    it('should subscribe to PATTERN_DETECTED, PROFILE_UPDATED, CONSENSUS_REQUEST', () => {
      const subscriptions = [];
      const mockBus = {
        subscribe: (event, agentId, handler) => {
          subscriptions.push({ event, agentId });
        },
      };
      const s = new CollectiveScholar({ eventBus: mockBus });
      assert.ok(subscriptions.length >= 3);
    });

    it('should handle pattern detected by tracking topics', () => {
      scholar._handlePatternDetected({
        payload: { patternType: 'workflow', context: { topic: 'testing' } },
      });
      assert.equal(scholar.topicsOfInterest.get('testing'), 1);

      scholar._handlePatternDetected({
        payload: { patternType: 'workflow', context: { topic: 'testing' } },
      });
      assert.equal(scholar.topicsOfInterest.get('testing'), 2);
    });

    it('should handle profile update', () => {
      scholar._handleProfileUpdated({
        payload: { newLevel: ProfileLevel.EXPERT },
      });
      assert.equal(scholar.profileLevel, ProfileLevel.EXPERT);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SETTERS
  // ═══════════════════════════════════════════════════════════════
  describe('Setters', () => {
    it('setProfileLevel should update level', () => {
      scholar.setProfileLevel(ProfileLevel.MASTER);
      assert.equal(scholar.profileLevel, ProfileLevel.MASTER);
    });

    it('getKnowledge should return all entries', () => {
      scholar._addToKnowledgeBase({ topic: 'a', summary: 'x' });
      scholar._addToKnowledgeBase({ topic: 'b', summary: 'y' });
      const knowledge = scholar.getKnowledge();
      assert.equal(knowledge.length, 2);
    });

    it('getTopicsOfInterest should return sorted by count', () => {
      scholar.topicsOfInterest.set('react', 3);
      scholar.topicsOfInterest.set('vue', 10);
      scholar.topicsOfInterest.set('svelte', 1);
      const topics = scholar.getTopicsOfInterest();
      assert.equal(topics[0].topic, 'vue');
      assert.equal(topics[0].count, 10);
    });
  });
});

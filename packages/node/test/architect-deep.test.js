/**
 * Deep tests for CollectiveArchitect (Chesed - Kindness)
 * Tests design review, pattern detection, constructive feedback, consensus
 *
 * @module test/architect-deep
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import {
  CollectiveArchitect,
  ARCHITECT_CONSTANTS,
  ReviewCategory,
  FeedbackType,
} from '../src/agents/collective/architect.js';
import { AgentTrigger, AgentBehavior } from '../src/agents/base.js';
import { ProfileLevel } from '../src/profile/calculator.js';

describe('CollectiveArchitect - Deep Tests', () => {
  let architect;

  beforeEach(() => {
    architect = new CollectiveArchitect();
  });

  // ═══════════════════════════════════════════════════════════════
  // CONSTRUCTOR & DEFAULTS
  // ═══════════════════════════════════════════════════════════════
  describe('Constructor & defaults', () => {
    it('should have correct name and trigger', () => {
      assert.equal(architect.name, 'Architect');
      assert.equal(architect.trigger, AgentTrigger.CONTEXT_AWARE);
      assert.equal(architect.behavior, AgentBehavior.NON_BLOCKING);
    });

    it('should default to PRACTITIONER profile', () => {
      assert.equal(architect.profileLevel, ProfileLevel.PRACTITIONER);
    });

    it('should initialize empty structures', () => {
      assert.deepEqual(architect.reviewHistory, []);
      assert.equal(architect.patternCounts.size, 0);
      assert.equal(architect.pendingConsensus.size, 0);
    });

    it('should initialize zero stats', () => {
      assert.equal(architect.reviewStats.total, 0);
      assert.equal(architect.reviewStats.patternDetections, 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SHOULD TRIGGER
  // ═══════════════════════════════════════════════════════════════
  describe('shouldTrigger', () => {
    it('should trigger on CONTEXT_AWARE', () => {
      assert.ok(architect.shouldTrigger({ type: AgentTrigger.CONTEXT_AWARE }));
    });

    it('should trigger on context_aware string', () => {
      assert.ok(architect.shouldTrigger({ type: 'context_aware' }));
    });

    it('should trigger when needsReview is true', () => {
      assert.ok(architect.shouldTrigger({ needsReview: true }));
    });

    it('should trigger when code is present', () => {
      assert.ok(architect.shouldTrigger({ code: 'const x = 1;' }));
    });

    it('should NOT trigger on unrelated events', () => {
      assert.ok(!architect.shouldTrigger({ type: 'PostToolUse' }));
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // DESIGN PATTERN DETECTION
  // ═══════════════════════════════════════════════════════════════
  describe('Design pattern detection', () => {
    it('should detect singleton pattern', async () => {
      const code = `
        class Database {
          static getInstance() { return this.instance; }
          private constructor() {}
        }
      `;
      const analysis = await architect.analyze({ code }, {});
      assert.ok(analysis.patterns.some(p => p.name === 'singleton'));
    });

    it('should detect factory pattern', async () => {
      const code = `
        class UserFactory {
          createUser(type) { return new User(type); }
        }
      `;
      const analysis = await architect.analyze({ code }, {});
      assert.ok(analysis.patterns.some(p => p.name === 'factory'));
    });

    it('should detect observer pattern', async () => {
      const code = `
        class EventEmitter {
          subscribe(event, handler) {}
          emit(event, data) {}
        }
      `;
      const analysis = await architect.analyze({ code }, {});
      assert.ok(analysis.patterns.some(p => p.name === 'observer'));
    });

    it('should detect strategy pattern', async () => {
      const code = `
        class PaymentStrategy {
          execute(amount) {}
          setStrategy(strategy) { this.strategy = strategy; }
        }
      `;
      const analysis = await architect.analyze({ code }, {});
      assert.ok(analysis.patterns.some(p => p.name === 'strategy'));
    });

    it('should detect builder pattern', async () => {
      const code = `
        class QueryBuilder {
          withFilter(f) { return this; }
          build() { return this.query; }
        }
      `;
      const analysis = await architect.analyze({ code }, {});
      assert.ok(analysis.patterns.some(p => p.name === 'builder'));
    });

    it('should detect middleware pattern', async () => {
      const code = `
        app.use(middleware);
        function handler(req, res) { next(); }
      `;
      const analysis = await architect.analyze({ code }, {});
      assert.ok(analysis.patterns.some(p => p.name === 'middleware'));
    });

    it('should detect multiple patterns in same code', async () => {
      const code = `
        class ServiceFactory {
          createService() {}
          subscribe(event) {}
          emit(data) {}
        }
      `;
      const analysis = await architect.analyze({ code }, {});
      assert.ok(analysis.patterns.length >= 2);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // ARCHITECTURE REVIEW
  // ═══════════════════════════════════════════════════════════════
  describe('_reviewArchitecture', () => {
    const style = { verbosity: 'balanced', includeExamples: false };

    it('should praise good module organization', () => {
      const code = "import { foo } from 'bar';\nexport default function main() {}";
      const feedback = architect._reviewArchitecture(code, style);
      assert.ok(feedback.some(f => f.type === FeedbackType.PRAISE));
    });

    it('should question mixed classes and functions', () => {
      const code = 'class Foo {}\nfunction bar() {}\nexport { Foo, bar };';
      const feedback = architect._reviewArchitecture(code, style);
      assert.ok(feedback.some(f => f.type === FeedbackType.QUESTION));
    });

    it('should flag deep nesting', () => {
      const code = 'if (a) { if (b) { if (c) { if (d) { if (e) { deep(); } } } } }';
      const feedback = architect._reviewArchitecture(code, style);
      assert.ok(feedback.some(f =>
        f.type === FeedbackType.SUGGESTION &&
        f.category === ReviewCategory.ARCHITECTURE
      ));
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // NAMING REVIEW
  // ═══════════════════════════════════════════════════════════════
  describe('_reviewNaming', () => {
    const style = { verbosity: 'balanced' };

    it('should flag many single-letter variables', () => {
      const code = 'let a = 1;\nlet b = 2;\nlet c = 3;\nlet d = 4;';
      const feedback = architect._reviewNaming(code, style);
      assert.ok(feedback.some(f => f.category === ReviewCategory.NAMING));
    });

    it('should flag mixed naming conventions', () => {
      const code = 'const camelCase = 1;\nconst snake_case = 2;';
      const feedback = architect._reviewNaming(code, style);
      assert.ok(feedback.some(f =>
        f.type === FeedbackType.QUESTION &&
        f.category === ReviewCategory.NAMING
      ));
    });

    it('should praise descriptive verb prefixes', () => {
      const code = 'function getUserById(id) {}\nfunction validateInput(data) {}';
      const feedback = architect._reviewNaming(code, style);
      assert.ok(feedback.some(f => f.type === FeedbackType.PRAISE));
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // COMPLEXITY REVIEW
  // ═══════════════════════════════════════════════════════════════
  describe('_reviewComplexity', () => {
    const style = { verbosity: 'balanced', includeExamples: false };

    it('should praise small files', () => {
      const code = 'const x = 1;\nconst y = 2;';
      const feedback = architect._reviewComplexity(code, style);
      assert.ok(feedback.some(f =>
        f.type === FeedbackType.PRAISE &&
        f.category === ReviewCategory.COMPLEXITY
      ));
    });

    it('should flag complex conditionals', () => {
      const longCondition = 'if (' + 'condition && another_condition && yet_another_check && more_stuff && final_check'.repeat(2) + ') {}';
      const feedback = architect._reviewComplexity(longCondition, style);
      assert.ok(feedback.some(f => f.category === ReviewCategory.COMPLEXITY));
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // TESTABILITY REVIEW
  // ═══════════════════════════════════════════════════════════════
  describe('_reviewTestability', () => {
    const style = { verbosity: 'balanced', includeExamples: false };

    it('should praise dependency injection', () => {
      const code = 'class Service { constructor(database, cache) { this.db = database; } }';
      const feedback = architect._reviewTestability(code, style);
      assert.ok(feedback.some(f =>
        f.type === FeedbackType.PRAISE &&
        f.category === ReviewCategory.TESTABILITY
      ));
    });

    it('should flag hardcoded URLs', () => {
      const code = `
        const api1 = "https://api.example.com/v1";
        const api2 = "https://api.example.com/v2";
        const api3 = "https://api.other.com/data";
      `;
      const feedback = architect._reviewTestability(code, style);
      assert.ok(feedback.some(f =>
        f.type === FeedbackType.SUGGESTION &&
        f.category === ReviewCategory.TESTABILITY
      ));
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // CONSTRUCTIVE BALANCE
  // ═══════════════════════════════════════════════════════════════
  describe('_ensureConstructiveBalance', () => {
    it('should add generic praises when not enough positives', () => {
      const style = { maxSuggestions: 8 };
      const feedback = [
        { type: FeedbackType.SUGGESTION, category: 'x', message: 'fix1', confidence: 0.5 },
        { type: FeedbackType.SUGGESTION, category: 'x', message: 'fix2', confidence: 0.5 },
        { type: FeedbackType.WARNING, category: 'x', message: 'warn1', confidence: 0.5 },
      ];
      const balanced = architect._ensureConstructiveBalance(feedback, style);
      const praises = balanced.filter(f => f.type === FeedbackType.PRAISE);
      const critiques = balanced.filter(f =>
        f.type === FeedbackType.SUGGESTION || f.type === FeedbackType.WARNING
      );
      // Need at least ceil(critiques * PHI_INV) praises
      assert.ok(praises.length >= Math.ceil(critiques.length * ARCHITECT_CONSTANTS.POSITIVE_RATIO));
    });

    it('should sort praises first in feedback', () => {
      const style = { maxSuggestions: 8 };
      const feedback = [
        { type: FeedbackType.WARNING, category: 'x', message: 'w', confidence: 0.5 },
        { type: FeedbackType.PRAISE, category: 'x', message: 'p', confidence: 0.5 },
        { type: FeedbackType.SUGGESTION, category: 'x', message: 's', confidence: 0.5 },
      ];
      const balanced = architect._ensureConstructiveBalance(feedback, style);
      assert.equal(balanced[0].type, FeedbackType.PRAISE);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SCORE CALCULATION
  // ═══════════════════════════════════════════════════════════════
  describe('_calculateScore', () => {
    it('should start at base 70', () => {
      const score = architect._calculateScore([]);
      assert.equal(score, 70);
    });

    it('should increase score for praises', () => {
      const score = architect._calculateScore([
        { type: FeedbackType.PRAISE },
        { type: FeedbackType.PRAISE },
      ]);
      assert.equal(score, 80); // 70 + 5 + 5
    });

    it('should decrease score for warnings', () => {
      const score = architect._calculateScore([
        { type: FeedbackType.WARNING },
        { type: FeedbackType.WARNING },
      ]);
      assert.equal(score, 60); // 70 - 5 - 5
    });

    it('should clamp between 0 and 100', () => {
      const manyWarnings = Array(20).fill({ type: FeedbackType.WARNING });
      const score = architect._calculateScore(manyWarnings);
      assert.equal(score, 0);

      const manyPraises = Array(20).fill({ type: FeedbackType.PRAISE });
      const highScore = architect._calculateScore(manyPraises);
      assert.equal(highScore, 100);
    });

    it('should give +2 for patterns, -3 for suggestions, -1 for questions', () => {
      assert.equal(architect._calculateScore([{ type: FeedbackType.PATTERN }]), 72);
      assert.equal(architect._calculateScore([{ type: FeedbackType.SUGGESTION }]), 67);
      assert.equal(architect._calculateScore([{ type: FeedbackType.QUESTION }]), 69);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // ANALYZE → DECIDE FLOW
  // ═══════════════════════════════════════════════════════════════
  describe('analyze → decide flow', () => {
    it('should reject insufficient content', async () => {
      const analysis = await architect.analyze({ code: 'x' }, {});
      assert.equal(analysis.reviewed, false);
    });

    it('should review valid code', async () => {
      const code = `
        import { foo } from 'bar';
        export class MyService {
          constructor(dep) { this.dep = dep; }
          function getData() { return this.dep.fetch(); }
        }
      `;
      const analysis = await architect.analyze({ code, filename: 'service.js' }, {});
      assert.ok(analysis.reviewed);
      assert.ok(analysis.feedback.length > 0);
      assert.ok(analysis.score >= 0 && analysis.score <= 100);
      assert.ok(analysis.confidence <= PHI_INV);
    });

    it('should decide LOG for non-reviewed content', async () => {
      const decision = await architect.decide({ reviewed: false }, {});
      assert.equal(decision.action, false);
    });

    it('should decide SUGGEST for reviewed content', async () => {
      const analysis = {
        reviewed: true,
        filename: 'test.js',
        feedback: [{ type: FeedbackType.PRAISE, category: 'arch', message: 'good', confidence: 0.5 }],
        patterns: [],
        score: 75,
        confidence: 0.5,
      };
      const decision = await architect.decide(analysis, {});
      assert.ok(decision.action);
      assert.ok(decision.summary);
      assert.equal(architect.reviewStats.total, 1);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // PUBLIC API: review()
  // ═══════════════════════════════════════════════════════════════
  describe('review()', () => {
    it('should review code through public API', async () => {
      const result = await architect.review(
        'class Config { constructor(options) { this.opts = options; } }',
        { filename: 'config.js' }
      );
      assert.ok(result.action !== undefined);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // REVIEW HISTORY
  // ═══════════════════════════════════════════════════════════════
  describe('Review history', () => {
    it('should record reviews in history', async () => {
      await architect.review('class A { constructor(x) {} }', { filename: 'a.js' });
      assert.ok(architect.reviewHistory.length > 0);
    });

    it('should enforce MAX_REVIEW_HISTORY', () => {
      for (let i = 0; i < ARCHITECT_CONSTANTS.MAX_REVIEW_HISTORY + 5; i++) {
        architect._recordReview({
          filename: `file${i}.js`,
          feedback: [],
          patterns: [],
          score: 70,
        });
      }
      assert.ok(architect.reviewHistory.length <= ARCHITECT_CONSTANTS.MAX_REVIEW_HISTORY);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // VOTE ON CONSENSUS
  // ═══════════════════════════════════════════════════════════════
  describe('voteOnConsensus', () => {
    it('should approve simplification', () => {
      const result = architect.voteOnConsensus('Should we refactor and simplify this module?');
      assert.equal(result.vote, 'approve');
    });

    it('should reject added complexity (needs design keyword)', () => {
      // Must contain a designPattern keyword AND 'complex'/'add layer'
      const result = architect.voteOnConsensus('Should we design a complex layer of abstraction?');
      assert.equal(result.vote, 'reject');
    });

    it('should approve deployment domain', () => {
      const result = architect.voteOnConsensus('Should we update infrastructure?', { domain: 'deployment' });
      assert.equal(result.vote, 'approve');
    });

    it('should abstain on unrelated', () => {
      const result = architect.voteOnConsensus('Should we buy more coffee?');
      assert.equal(result.vote, 'abstain');
    });

    it('should approve clean design questions', () => {
      const result = architect.voteOnConsensus('Should we clean up the design pattern?');
      assert.equal(result.vote, 'approve');
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // PROFILE-BASED FEEDBACK
  // ═══════════════════════════════════════════════════════════════
  describe('Profile-based feedback', () => {
    it('NOVICE should limit to 3 suggestions', async () => {
      architect.setProfileLevel(ProfileLevel.NOVICE);
      const code = `
        let a = 1; let b = 2; let c = 3; let d = 4;
        const snake_case = true; const camelCase = true;
        if (a) { if (b) { if (c) { if (d) { if (a) { nested(); } } } } }
        const url1 = "https://a.com"; const url2 = "https://b.com"; const url3 = "https://c.com";
      `;
      const analysis = await architect.analyze({ code }, {});
      // Novice style maxSuggestions = 3
      assert.ok(analysis.feedback.length <= 3);
    });

    it('EXPERT should use direct tone', async () => {
      architect.setProfileLevel(ProfileLevel.EXPERT);
      const analysis = await architect.analyze({ code: 'class X { constructor(a) {} }' }, {});
      assert.equal(analysis.feedbackStyle, 'direct');
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SUMMARY & CLEAR
  // ═══════════════════════════════════════════════════════════════
  describe('getSummary & clear', () => {
    it('should return summary with patterns known', () => {
      const summary = architect.getSummary();
      assert.equal(summary.patternsKnown, 6); // 6 design patterns
      assert.equal(summary.profileLevel, ProfileLevel.PRACTITIONER);
    });

    it('should clear all state', () => {
      architect.reviewHistory.push({ filename: 'a', score: 70 });
      architect.patternCounts.set('singleton', 3);
      architect.reviewStats.total = 5;
      architect.clear();

      assert.deepEqual(architect.reviewHistory, []);
      assert.equal(architect.patternCounts.size, 0);
      assert.equal(architect.reviewStats.total, 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // φ-ALIGNMENT
  // ═══════════════════════════════════════════════════════════════
  describe('φ-alignment', () => {
    it('should have Fibonacci-based constants', () => {
      assert.equal(ARCHITECT_CONSTANTS.MAX_REVIEW_HISTORY, 21); // Fib(8)
      assert.equal(ARCHITECT_CONSTANTS.CONSENSUS_TIMEOUT_MS, 21000);
      assert.equal(ARCHITECT_CONSTANTS.PATTERN_THRESHOLD, 5); // Fib(5)
      assert.equal(ARCHITECT_CONSTANTS.MAX_SUGGESTIONS, 8); // Fib(6)
      assert.equal(ARCHITECT_CONSTANTS.POSITIVE_RATIO, PHI_INV);
      assert.equal(ARCHITECT_CONSTANTS.MIN_SUGGESTION_CONFIDENCE, PHI_INV_2);
    });

    it('confidence should never exceed PHI_INV in analysis', async () => {
      const code = `
        import { a } from 'b'; export default class X {}
        function getUserById() {} function validateInput() {}
        class Factory { createUser() {} subscribe() {} emit() {} }
      `;
      const analysis = await architect.analyze({ code }, {});
      assert.ok(analysis.confidence <= PHI_INV);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // EVENT BUS INTEGRATION
  // ═══════════════════════════════════════════════════════════════
  describe('Event bus integration', () => {
    it('should subscribe to 4 events', () => {
      const subscriptions = [];
      const mockBus = {
        subscribe: (event, agentId, handler) => {
          subscriptions.push({ event, agentId });
        },
      };
      const a = new CollectiveArchitect({ eventBus: mockBus });
      assert.equal(subscriptions.length, 4);
    });

    it('should handle profile update', () => {
      architect._handleProfileUpdated({ payload: { newLevel: ProfileLevel.MASTER } });
      assert.equal(architect.profileLevel, ProfileLevel.MASTER);
    });

    it('should learn from knowledge extraction', () => {
      architect._handleKnowledgeExtracted({
        payload: { knowledgeType: 'pattern', topic: 'singleton' },
      });
      assert.equal(architect.patternCounts.get('singleton'), 1);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SUMMARY CREATION
  // ═══════════════════════════════════════════════════════════════
  describe('_createSummary', () => {
    it('should include counts in summary string', () => {
      const summary = architect._createSummary({
        feedback: [
          { type: FeedbackType.PRAISE },
          { type: FeedbackType.SUGGESTION },
          { type: FeedbackType.WARNING },
        ],
        patterns: [{ name: 'singleton' }],
        score: 72,
      });
      assert.ok(summary.includes('72'));
      assert.ok(summary.includes('1 strength'));
      assert.ok(summary.includes('1 suggestion'));
      assert.ok(summary.includes('1 concern'));
      assert.ok(summary.includes('1 pattern'));
    });
  });
});

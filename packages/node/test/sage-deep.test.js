/**
 * Deep tests for CollectiveSage (Chochmah - Wisdom)
 * Tests wisdom sharing, teaching styles, milestones, relevance, consensus
 *
 * @module test/sage-deep
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import {
  CollectiveSage,
  SAGE_CONSTANTS,
  WisdomType,
} from '../src/agents/collective/sage.js';
import { AgentTrigger, AgentBehavior } from '../src/agents/base.js';
import { ProfileLevel } from '../src/profile/calculator.js';

describe('CollectiveSage - Deep Tests', () => {
  let sage;

  beforeEach(() => {
    sage = new CollectiveSage();
  });

  // ═══════════════════════════════════════════════════════════════
  // CONSTRUCTOR & DEFAULTS
  // ═══════════════════════════════════════════════════════════════
  describe('Constructor & defaults', () => {
    it('should have correct name and trigger', () => {
      assert.equal(sage.name, 'Sage');
      assert.equal(sage.trigger, AgentTrigger.CONTEXT_AWARE);
      assert.equal(sage.behavior, AgentBehavior.NON_BLOCKING);
    });

    it('should default to PRACTITIONER profile', () => {
      assert.equal(sage.profileLevel, ProfileLevel.PRACTITIONER);
    });

    it('should initialize empty collections', () => {
      assert.equal(sage.wisdomBase.size, 0);
      assert.deepEqual(sage.insights, []);
      assert.deepEqual(sage.learnedWarnings, []);
      assert.deepEqual(sage.teachingHistory, []);
    });

    it('should initialize zero progress tracker', () => {
      assert.equal(sage.progressTracker.interactionCount, 0);
      assert.equal(sage.progressTracker.lessonsGiven, 0);
      assert.equal(sage.progressTracker.challengesPresented, 0);
      assert.deepEqual(sage.progressTracker.milestonesReached, []);
    });

    it('should initialize zero wisdom stats', () => {
      assert.equal(sage.wisdomStats.shared, 0);
      assert.equal(sage.wisdomStats.avgRelevance, 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SHOULD TRIGGER
  // ═══════════════════════════════════════════════════════════════
  describe('shouldTrigger', () => {
    it('should trigger on CONTEXT_AWARE', () => {
      assert.ok(sage.shouldTrigger({ type: AgentTrigger.CONTEXT_AWARE }));
    });

    it('should trigger on context_aware string', () => {
      assert.ok(sage.shouldTrigger({ type: 'context_aware' }));
    });

    it('should trigger when needsGuidance is true', () => {
      assert.ok(sage.shouldTrigger({ needsGuidance: true }));
    });

    it('should trigger when question is present', () => {
      assert.ok(sage.shouldTrigger({ question: 'How do I use promises?' }));
    });

    it('should NOT trigger on unrelated events', () => {
      assert.ok(!sage.shouldTrigger({ type: 'PostToolUse' }));
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // TEACHING STYLES
  // ═══════════════════════════════════════════════════════════════
  describe('Teaching styles by profile', () => {
    it('NOVICE should get nurturing approach', async () => {
      sage.setProfileLevel(ProfileLevel.NOVICE);
      const analysis = await sage.analyze({ topic: 'testing' }, {});
      assert.equal(analysis.teachingStyle, 'nurturing');
    });

    it('PRACTITIONER should get collaborative approach', async () => {
      sage.setProfileLevel(ProfileLevel.PRACTITIONER);
      const analysis = await sage.analyze({ topic: 'testing' }, {});
      assert.equal(analysis.teachingStyle, 'collaborative');
    });

    it('MASTER should get dialectic approach', async () => {
      sage.setProfileLevel(ProfileLevel.MASTER);
      const analysis = await sage.analyze({ topic: 'testing' }, {});
      assert.equal(analysis.teachingStyle, 'dialectic');
    });

    it('EXPERT should get peer approach', async () => {
      sage.setProfileLevel(ProfileLevel.EXPERT);
      const analysis = await sage.analyze({ topic: 'testing' }, {});
      assert.equal(analysis.teachingStyle, 'peer');
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // MILESTONE DETECTION
  // ═══════════════════════════════════════════════════════════════
  describe('_checkMilestone', () => {
    it('should detect milestone at MILESTONE_INTERVAL (21)', () => {
      sage.progressTracker.interactionCount = 21;
      const milestone = sage._checkMilestone();
      assert.ok(milestone !== null);
      assert.equal(milestone.number, 1);
      assert.equal(milestone.interactions, 21);
    });

    it('should NOT detect milestone between intervals', () => {
      sage.progressTracker.interactionCount = 15;
      assert.equal(sage._checkMilestone(), null);
    });

    it('should detect multiple milestones', () => {
      sage.progressTracker.interactionCount = 42;
      const milestone = sage._checkMilestone();
      assert.ok(milestone !== null);
      assert.equal(milestone.number, 2);
    });

    it('should cap milestone messages at array length', () => {
      sage.progressTracker.interactionCount = 21 * 10; // 210
      const milestone = sage._checkMilestone();
      assert.ok(milestone.message.length > 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // WISDOM TYPE DETERMINATION
  // ═══════════════════════════════════════════════════════════════
  describe('_determineWisdomType', () => {
    const style = {
      approach: 'collaborative',
      encouragementFrequency: 'occasional',
      challengeLevel: 'moderate',
    };

    it('should return ENCOURAGEMENT on milestone', () => {
      const type = sage._determineWisdomType([], { number: 1 }, '', style);
      assert.equal(type, WisdomType.ENCOURAGEMENT);
    });

    it('should return ENCOURAGEMENT when user is struggling', () => {
      const type = sage._determineWisdomType([], null, 'I am stuck and confused', style);
      assert.equal(type, WisdomType.ENCOURAGEMENT);
    });

    it('should return LESSON for MASTER struggling (no encouragement)', () => {
      const masterStyle = { ...style, encouragementFrequency: 'none' };
      const type = sage._determineWisdomType([], null, 'help me', masterStyle);
      assert.equal(type, WisdomType.LESSON);
    });

    it('should return WARNING when relevant warnings exist', () => {
      const wisdom = [{ type: WisdomType.WARNING, relevance: 0.5 }];
      const type = sage._determineWisdomType(wisdom, null, '', style);
      assert.equal(type, WisdomType.WARNING);
    });

    it('should return INSIGHT when relevant insights exist', () => {
      const wisdom = [{ type: WisdomType.INSIGHT, relevance: 0.5 }];
      const type = sage._determineWisdomType(wisdom, null, '', style);
      assert.equal(type, WisdomType.INSIGHT);
    });

    it('should default to LESSON', () => {
      const type = sage._determineWisdomType([], null, '', style);
      assert.equal(type, WisdomType.LESSON);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // WISDOM CREATION
  // ═══════════════════════════════════════════════════════════════
  describe('_createWisdom (all types)', () => {
    const style = {
      approach: 'collaborative',
      stepByStep: false,
      useAnalogies: false,
      challengeLevel: 'moderate',
    };

    it('should create LESSON with topic', () => {
      const wisdom = sage._createWisdom(WisdomType.LESSON, 'testing', '', style);
      assert.equal(wisdom.type, WisdomType.LESSON);
      assert.ok(wisdom.content.includes('testing'));
      assert.ok(wisdom.confidence > 0);
    });

    it('should create ENCOURAGEMENT', () => {
      const wisdom = sage._createWisdom(WisdomType.ENCOURAGEMENT, 'testing', '', style);
      assert.equal(wisdom.type, WisdomType.ENCOURAGEMENT);
      assert.ok(wisdom.content.length > 0);
    });

    it('should create CHALLENGE', () => {
      const wisdom = sage._createWisdom(WisdomType.CHALLENGE, 'testing', '', style);
      assert.equal(wisdom.type, WisdomType.CHALLENGE);
    });

    it('should create WARNING (generic)', () => {
      const wisdom = sage._createWisdom(WisdomType.WARNING, 'testing', '', style);
      assert.equal(wisdom.type, WisdomType.WARNING);
      assert.ok(wisdom.content.includes('caution'));
    });

    it('should create INSIGHT (generic)', () => {
      const wisdom = sage._createWisdom(WisdomType.INSIGHT, 'testing', '', style);
      assert.equal(wisdom.type, WisdomType.INSIGHT);
    });

    it('should create REFLECTION', () => {
      const wisdom = sage._createWisdom(WisdomType.REFLECTION, 'architecture', 'how?', style);
      assert.equal(wisdom.type, WisdomType.REFLECTION);
    });

    it('NOVICE step-by-step lesson', () => {
      const noviceStyle = { ...style, stepByStep: true, useAnalogies: true };
      const wisdom = sage._createWisdom(WisdomType.LESSON, 'arrays', '', noviceStyle);
      assert.ok(wisdom.content.includes('1.'));
    });

    it('should respect MAX_LESSON_LENGTH per profile', () => {
      sage.setProfileLevel(ProfileLevel.MASTER);
      const wisdom = sage._createWisdom(WisdomType.LESSON, 'x', '', style);
      assert.ok(wisdom.content.length <= SAGE_CONSTANTS.MAX_LESSON_LENGTH[ProfileLevel.MASTER]);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // RELEVANCE CALCULATION
  // ═══════════════════════════════════════════════════════════════
  describe('_calculateRelevance', () => {
    it('should return higher relevance for matching terms', () => {
      const wisdom = { topic: 'authentication', content: 'JWT tokens and OAuth' };
      const r1 = sage._calculateRelevance(wisdom, ['authentication']);
      const r2 = sage._calculateRelevance(wisdom, ['unrelated']);
      assert.ok(r1 > r2);
    });

    it('should apply time decay', () => {
      const recent = {
        topic: 'test',
        content: 'testing',
        learnedAt: Date.now(),
      };
      const old = {
        topic: 'test',
        content: 'testing',
        learnedAt: Date.now() - (90 * 24 * 60 * 60 * 1000), // 90 days ago
      };
      const r1 = sage._calculateRelevance(recent, ['test']);
      const r2 = sage._calculateRelevance(old, ['test']);
      assert.ok(r1 > r2);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // INSIGHTS & WARNINGS MANAGEMENT
  // ═══════════════════════════════════════════════════════════════
  describe('Insights & warnings management', () => {
    it('should add insights', () => {
      sage._addInsight({ type: 'insight', topic: 'a', content: 'x', confidence: 0.5 });
      assert.equal(sage.insights.length, 1);
    });

    it('should cap insights at MAX_INSIGHTS', () => {
      for (let i = 0; i < SAGE_CONSTANTS.MAX_INSIGHTS + 5; i++) {
        sage._addInsight({ type: 'insight', topic: `t${i}`, content: `c${i}` });
      }
      assert.ok(sage.insights.length <= SAGE_CONSTANTS.MAX_INSIGHTS);
    });

    it('should add warnings', () => {
      sage._addWarning({ type: 'warning', threatType: 'destructive', lesson: 'be careful' });
      assert.equal(sage.learnedWarnings.length, 1);
    });

    it('should cap warnings at MAX_INSIGHTS', () => {
      for (let i = 0; i < SAGE_CONSTANTS.MAX_INSIGHTS + 5; i++) {
        sage._addWarning({ type: 'warning', threatType: `t${i}`, lesson: `l${i}` });
      }
      assert.ok(sage.learnedWarnings.length <= SAGE_CONSTANTS.MAX_INSIGHTS);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // ANALYZE → DECIDE FLOW
  // ═══════════════════════════════════════════════════════════════
  describe('analyze → decide flow', () => {
    it('should increment interaction count', async () => {
      await sage.analyze({ topic: 'test' }, {});
      assert.equal(sage.progressTracker.interactionCount, 1);
    });

    it('should produce wisdom with confidence', async () => {
      const analysis = await sage.analyze({ topic: 'testing' }, {});
      assert.ok(analysis.wisdom);
      assert.ok(analysis.confidence >= 0);
      assert.ok(analysis.confidence <= PHI_INV);
    });

    it('should decide SUGGEST when confidence >= MIN_SHARE_CONFIDENCE', async () => {
      const analysis = {
        wisdom: { type: WisdomType.LESSON, topic: 'test', content: 'lesson content', confidence: 0.5 },
        teachingStyle: 'collaborative',
        milestone: null,
        confidence: 0.5,
      };
      const decision = await sage.decide(analysis, {});
      assert.ok(decision.action);
      assert.equal(sage.wisdomStats.shared, 1);
    });

    it('should decide LOG when confidence < MIN_SHARE_CONFIDENCE', async () => {
      const analysis = {
        wisdom: { type: WisdomType.LESSON, content: 'x', confidence: 0.1 },
        milestone: null,
        confidence: 0.1, // Below PHI_INV_2
      };
      const decision = await sage.decide(analysis, {});
      assert.equal(decision.action, false);
    });

    it('should track milestones in decide', async () => {
      const analysis = {
        wisdom: { type: WisdomType.ENCOURAGEMENT, topic: 'test', content: 'great!', confidence: 0.5 },
        teachingStyle: 'collaborative',
        milestone: { number: 1, interactions: 21, message: 'First steps' },
        confidence: 0.5,
      };
      await sage.decide(analysis, {});
      assert.equal(sage.progressTracker.milestonesReached.length, 1);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // PUBLIC API: shareWisdom()
  // ═══════════════════════════════════════════════════════════════
  describe('shareWisdom()', () => {
    it('should share wisdom through public API', async () => {
      const result = await sage.shareWisdom('error handling');
      assert.ok(result !== undefined);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // EVENT HANDLERS
  // ═══════════════════════════════════════════════════════════════
  describe('Event handlers', () => {
    it('should convert knowledge to insight on KNOWLEDGE_EXTRACTED', () => {
      sage._handleKnowledgeExtracted({
        payload: {
          knowledgeType: 'code_example',
          topic: 'async patterns',
          summary: 'Use async/await for cleaner code',
          confidence: 0.5,
        },
      });
      assert.equal(sage.insights.length, 1);
      assert.equal(sage.insights[0].topic, 'async patterns');
    });

    it('should NOT add insight below MIN_SHARE_CONFIDENCE', () => {
      sage._handleKnowledgeExtracted({
        payload: {
          knowledgeType: 'code_example',
          topic: 'test',
          summary: 'test',
          confidence: 0.1, // Below PHI_INV_2
        },
      });
      assert.equal(sage.insights.length, 0);
    });

    it('should convert threat to warning on THREAT_BLOCKED', () => {
      sage._handleThreatBlocked({
        payload: {
          threatType: 'destructive',
          riskLevel: 'CRITICAL',
          reason: 'rm -rf detected',
        },
      });
      assert.equal(sage.learnedWarnings.length, 1);
      assert.ok(sage.learnedWarnings[0].lesson.includes('Destructive'));
    });

    it('should accumulate insights from PATTERN_DETECTED', () => {
      sage._handlePatternDetected({
        payload: {
          patternType: 'workflow',
          patternCategory: 'testing',
          context: {},
          confidence: 0.5,
        },
      });
      assert.equal(sage.insights.length, 1);
    });

    it('should handle profile update', () => {
      sage._handleProfileUpdated({
        payload: { previousLevel: 3, newLevel: 4, reason: 'good work' },
      });
      assert.equal(sage.profileLevel, 4);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // WARNING LESSON CREATION
  // ═══════════════════════════════════════════════════════════════
  describe('_createWarningLesson', () => {
    it('should create lesson for destructive threats', () => {
      const lesson = sage._createWarningLesson('destructive', 'rm -rf');
      assert.ok(lesson.includes('Destructive'));
    });

    it('should create lesson for network threats', () => {
      const lesson = sage._createWarningLesson('network', 'curl');
      assert.ok(lesson.includes('Network'));
    });

    it('should create lesson for sensitive threats', () => {
      const lesson = sage._createWarningLesson('sensitive', '.env');
      assert.ok(lesson.includes('Sensitive'));
    });

    it('should fallback for unknown threat types', () => {
      const lesson = sage._createWarningLesson('unknown', 'reason');
      assert.ok(lesson.includes('reason'));
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // WISDOM MESSAGE FORMATTING
  // ═══════════════════════════════════════════════════════════════
  describe('_formatWisdomMessage', () => {
    it('should prefix LESSON with teaching pose', () => {
      const msg = sage._formatWisdomMessage(
        { type: WisdomType.LESSON, content: 'Learn this.' },
        'collaborative'
      );
      assert.ok(msg.includes('*settles into teaching pose*'));
    });

    it('should prefix ENCOURAGEMENT with tail wag', () => {
      const msg = sage._formatWisdomMessage(
        { type: WisdomType.ENCOURAGEMENT, content: 'Great job!' },
        'collaborative'
      );
      assert.ok(msg.includes('*tail wag*'));
    });

    it('should prefix WARNING with growl', () => {
      const msg = sage._formatWisdomMessage(
        { type: WisdomType.WARNING, content: 'Be careful.' },
        'collaborative'
      );
      assert.ok(msg.includes('*low growl*'));
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // VOTE ON CONSENSUS
  // ═══════════════════════════════════════════════════════════════
  describe('voteOnConsensus', () => {
    it('should reject rushing', () => {
      const result = sage.voteOnConsensus('Should we rush this deployment?');
      assert.equal(result.vote, 'reject');
    });

    it('should reject quick shortcuts', () => {
      const result = sage.voteOnConsensus('Can we do a quick fix?');
      assert.equal(result.vote, 'reject');
    });

    it('should approve proceeding', () => {
      const result = sage.voteOnConsensus('Should we proceed with the plan?');
      assert.equal(result.vote, 'approve');
    });

    it('should approve low risk', () => {
      const result = sage.voteOnConsensus('Should we update the docs?', { risk: 'low' });
      assert.equal(result.vote, 'approve');
    });

    it('should reject critical risk', () => {
      const result = sage.voteOnConsensus('Should we try this?', { risk: 'critical' });
      assert.equal(result.vote, 'reject');
    });

    it('should abstain on unrelated questions', () => {
      const result = sage.voteOnConsensus('What color should the button be?');
      assert.equal(result.vote, 'abstain');
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // PROGRESS TRACKING
  // ═══════════════════════════════════════════════════════════════
  describe('Progress tracking', () => {
    it('should track lessons given', async () => {
      const analysis = {
        wisdom: { type: WisdomType.LESSON, topic: 'test', content: 'lesson', confidence: 0.5 },
        teachingStyle: 'collaborative',
        milestone: null,
        confidence: 0.5,
      };
      await sage.decide(analysis, {});
      assert.equal(sage.progressTracker.lessonsGiven, 1);
    });

    it('should track challenges presented', async () => {
      const analysis = {
        wisdom: { type: WisdomType.CHALLENGE, topic: 'test', content: 'challenge', confidence: 0.5 },
        teachingStyle: 'collaborative',
        milestone: null,
        confidence: 0.5,
      };
      await sage.decide(analysis, {});
      assert.equal(sage.progressTracker.challengesPresented, 1);
    });

    it('getProgress should return copy of tracker', () => {
      sage.progressTracker.interactionCount = 42;
      const progress = sage.getProgress();
      assert.equal(progress.interactionCount, 42);
      progress.interactionCount = 0; // Should not affect original
      assert.equal(sage.progressTracker.interactionCount, 42);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SUMMARY & CLEAR
  // ═══════════════════════════════════════════════════════════════
  describe('getSummary & clear', () => {
    it('should return comprehensive summary', () => {
      sage._addInsight({ topic: 'a', content: 'b', confidence: 0.5 });
      sage._addWarning({ topic: 'c', lesson: 'd' });
      const summary = sage.getSummary();
      assert.equal(summary.insightsCount, 1);
      assert.equal(summary.warningsCount, 1);
      assert.ok('teachingStyle' in summary);
      assert.ok('progress' in summary);
    });

    it('should clear all state', () => {
      sage._addInsight({ topic: 'a' });
      sage._addWarning({ topic: 'b' });
      sage.teachingHistory.push({ type: 'lesson' });
      sage.progressTracker.interactionCount = 50;
      sage.wisdomStats.shared = 10;
      sage.clear();

      assert.equal(sage.wisdomBase.size, 0);
      assert.deepEqual(sage.insights, []);
      assert.deepEqual(sage.learnedWarnings, []);
      assert.deepEqual(sage.teachingHistory, []);
      assert.equal(sage.progressTracker.interactionCount, 0);
      assert.equal(sage.wisdomStats.shared, 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // φ-ALIGNMENT
  // ═══════════════════════════════════════════════════════════════
  describe('φ-alignment', () => {
    it('should have Fibonacci/φ constants', () => {
      assert.equal(SAGE_CONSTANTS.MAX_WISDOM_ENTRIES, 144); // Fib(12)
      assert.equal(SAGE_CONSTANTS.MAX_INSIGHTS, 21); // Fib(8)
      assert.equal(SAGE_CONSTANTS.RELEVANCE_DECAY, PHI_INV);
      assert.equal(SAGE_CONSTANTS.MIN_SHARE_CONFIDENCE, PHI_INV_2);
      assert.equal(SAGE_CONSTANTS.MILESTONE_INTERVAL, 21); // Fib(8)
    });

    it('confidence should never exceed PHI_INV', async () => {
      // Add many insights to potentially inflate confidence
      for (let i = 0; i < 20; i++) {
        sage._addInsight({
          type: 'insight',
          topic: 'testing',
          content: 'Test insight ' + i,
          confidence: 0.6,
          source: 'analyst',
          learnedAt: Date.now(),
        });
      }
      const analysis = await sage.analyze({ topic: 'testing' }, {});
      assert.ok(analysis.confidence <= PHI_INV);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // EVENT BUS SUBSCRIPTIONS
  // ═══════════════════════════════════════════════════════════════
  describe('Event bus subscriptions', () => {
    it('should subscribe to 5 event types', () => {
      const subscriptions = [];
      const mockBus = {
        subscribe: (event, agentId, handler) => {
          subscriptions.push(event);
        },
      };
      const s = new CollectiveSage({ eventBus: mockBus });
      assert.equal(subscriptions.length, 5);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // GETTERS
  // ═══════════════════════════════════════════════════════════════
  describe('Getters', () => {
    it('getInsights should return copy', () => {
      sage._addInsight({ topic: 'a' });
      const insights = sage.getInsights();
      assert.equal(insights.length, 1);
      insights.push({ topic: 'b' }); // Mutate copy
      assert.equal(sage.insights.length, 1); // Original unchanged
    });

    it('getLearnedWarnings should return copy', () => {
      sage._addWarning({ topic: 'a' });
      const warnings = sage.getLearnedWarnings();
      assert.equal(warnings.length, 1);
      warnings.push({ topic: 'b' });
      assert.equal(sage.learnedWarnings.length, 1);
    });
  });
});

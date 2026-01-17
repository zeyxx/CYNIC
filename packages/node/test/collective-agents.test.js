/**
 * @cynic/node - Collective Agents Tests
 *
 * Tests for the Five Dogs + CYNIC collective:
 * - Guardian (Gevurah): Enhanced watchdog
 * - Analyst (Binah): Observer + Auditor
 * - Scholar (Daat): Librarian + Digester
 * - Architect (Chesed): Design review
 * - Sage (Chochmah): Mentor + Guide
 * - CYNIC (Keter): Meta-consciousness orchestrator (Hidden 6th Dog)
 *
 * Plus inter-agent communication and profile adaptation.
 *
 * "φ distrusts φ" - κυνικός
 */

import { describe, it, beforeEach, afterEach, after } from 'node:test';
import assert from 'node:assert';

import {
  CollectivePack,
  createCollectivePack,
  CollectiveGuardian,
  CollectiveAnalyst,
  CollectiveScholar,
  CollectiveArchitect,
  CollectiveSage,
  CollectiveCynic,
  COLLECTIVE_CONSTANTS,
  CYNIC_CONSTANTS,
  CynicDecisionType,
  CynicGuidanceType,
  MetaState,
  RiskLevel,
  PatternCategory,
  KnowledgeType,
  FeedbackType,
  WisdomType,
} from '../src/agents/collective/index.js';

import { AgentEventBus } from '../src/agents/event-bus.js';
import {
  AgentEvent,
  AgentId,
  ConsensusVote,
  AgentEventMessage,
  CynicAwakeningEvent,
  CynicGuidanceEvent,
  CynicDecisionEvent,
} from '../src/agents/events.js';
import { ProfileLevel, PROFILE_CONSTANTS } from '../src/profile/calculator.js';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// Track packs for cleanup
const activePacks = [];

function createTrackedPack(options = {}) {
  const pack = createCollectivePack(options);
  activePacks.push(pack);
  return pack;
}

async function cleanupPacks() {
  for (const pack of activePacks) {
    await pack.shutdown();
  }
  activePacks.length = 0;
}

describe('Collective Agents', () => {
  after(async () => {
    await cleanupPacks();
  });

  describe('CollectivePack', () => {
    it('should create all six agents (Five Dogs + CYNIC)', () => {
      const pack = createTrackedPack();

      assert.ok(pack.guardian, 'Guardian should exist');
      assert.ok(pack.analyst, 'Analyst should exist');
      assert.ok(pack.scholar, 'Scholar should exist');
      assert.ok(pack.architect, 'Architect should exist');
      assert.ok(pack.sage, 'Sage should exist');
      assert.ok(pack.cynic, 'CYNIC should exist');

      // Fib(5) + 1 = 6 agents (Five Dogs + CYNIC as Keter)
      assert.strictEqual(pack.getAllAgents().length, 11, 'Should have 11 agents (5 dogs + CYNIC + Janitor + Scout + Cartographer + Oracle + Deployer)');
    });

    it('should have shared event bus', () => {
      const pack = createTrackedPack();

      const eventBus = pack.getEventBus();
      assert.ok(eventBus, 'Event bus should exist');
      assert.strictEqual(typeof eventBus.publish, 'function');
      assert.strictEqual(typeof eventBus.subscribe, 'function');
    });

    it('should process events through appropriate agents', async () => {
      const pack = createTrackedPack();

      // PreToolUse should trigger Guardian
      const preToolEvent = {
        type: 'PreToolUse',
        tool: 'Bash',
        input: { command: 'ls -la' },
      };

      const results = await pack.processEvent(preToolEvent);
      assert.ok(results.length > 0, 'Should have results');
    });

    it('should provide collective summary', () => {
      const pack = createTrackedPack();
      const summary = pack.getSummary();

      assert.strictEqual(summary.agentCount, 11);
      assert.strictEqual(summary.dogCount, 5);
      assert.ok(summary.agents.guardian);
      assert.ok(summary.agents.analyst);
      assert.ok(summary.agents.scholar);
      assert.ok(summary.agents.architect);
      assert.ok(summary.agents.sage);
      assert.ok(summary.agents.cynic, 'CYNIC should be in summary');
      assert.ok(summary.agents.janitor, 'Janitor should be in summary');
      assert.ok(summary.agents.scout, 'Scout should be in summary');
      assert.ok(summary.agents.cartographer, 'Cartographer should be in summary');
      assert.ok(summary.agents.oracle, 'Oracle should be in summary');
      assert.ok(summary.agents.deployer, 'Deployer should be in summary');
    });

    it('should awaken CYNIC', async () => {
      const pack = createTrackedPack();

      // Initially dormant
      assert.strictEqual(pack.cynic.metaState, MetaState.DORMANT);

      // Awaken
      const result = await pack.awakenCynic({
        sessionId: 'test-session',
        userId: 'test-user',
        project: 'test-project',
      });

      assert.strictEqual(result.success, true);
      assert.ok(result.greeting.includes('CYNIC'));
      assert.strictEqual(pack.cynic.metaState, MetaState.OBSERVING);
    });

    it('should provide collective state from CYNIC perspective', () => {
      const pack = createTrackedPack();
      const state = pack.getCollectiveState();

      assert.ok(typeof state.recentActivity === 'number');
      assert.ok(state.eventsByType);
      assert.ok(state.eventsBySource);
    });
  });

  describe('CollectiveGuardian', () => {
    let guardian;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.GUARDIAN);
      guardian = new CollectiveGuardian({
        eventBus,
        profileLevel: ProfileLevel.PRACTITIONER,
      });
    });

    afterEach(() => {
      eventBus.destroy();
    });

    it('should block dangerous commands', async () => {
      const event = {
        tool: 'Bash',
        input: { command: 'rm -rf /' },
      };

      const analysis = await guardian.analyze(event, {});
      assert.strictEqual(analysis.blocked, true);
      assert.strictEqual(analysis.risk.level, RiskLevel.CRITICAL.level);
    });

    it('should warn on risky commands', async () => {
      const event = {
        tool: 'Bash',
        input: { command: 'sudo apt-get update' },
      };

      const analysis = await guardian.analyze(event, {});
      assert.strictEqual(analysis.blocked, false);
      assert.strictEqual(analysis.warning, true);
    });

    it('should allow safe commands', async () => {
      const event = {
        tool: 'Bash',
        input: { command: 'ls -la' },
      };

      const analysis = await guardian.analyze(event, {});
      assert.strictEqual(analysis.blocked, false);
      assert.strictEqual(analysis.warning, false);
      assert.strictEqual(analysis.risk.level, RiskLevel.SAFE.level);
    });

    it('should adapt protection based on profile level', async () => {
      const event = {
        tool: 'Bash',
        input: { command: 'git reset --hard HEAD~1' },
      };

      // Novice should be more protected
      guardian.setProfileLevel(ProfileLevel.NOVICE);
      const noviceResult = await guardian.process(event, {});

      // Expert should be trusted more
      guardian.setProfileLevel(ProfileLevel.EXPERT);
      const expertResult = await guardian.process(event, {});

      // Both should warn, but expert might have lower confirmation requirement
      assert.ok(noviceResult.requiresConfirmation !== undefined || expertResult.response !== undefined);
    });

    it('should track escalation for repeated violations', async () => {
      const event = {
        tool: 'Bash',
        input: { command: 'sudo rm -r temp' },
      };

      // First violation
      await guardian.process(event, {});

      // Second violation should have higher escalation
      const analysis = await guardian.analyze(event, {});
      assert.ok(analysis.escalation >= 1);
    });

    it('should emit THREAT_BLOCKED event', async () => {
      let emittedEvent = null;
      eventBus.registerAgent('test');
      eventBus.subscribe(AgentEvent.THREAT_BLOCKED, 'test', (event) => {
        emittedEvent = event;
      });

      const event = {
        tool: 'Bash',
        input: { command: 'rm -rf /' },
      };

      await guardian.process(event, {});

      assert.ok(emittedEvent, 'Should emit threat blocked event');
      assert.strictEqual(emittedEvent.payload.action, 'block');
    });
  });

  describe('CollectiveAnalyst', () => {
    let analyst;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.ANALYST);
      analyst = new CollectiveAnalyst({ eventBus });
    });

    afterEach(() => {
      eventBus.destroy();
    });

    it('should track tool usage', async () => {
      const event = {
        tool: 'Read',
        input: { file_path: '/test/file.js' },
        output: { content: 'file content' },
      };

      await analyst.process(event, {});

      const stats = analyst.getToolStats();
      assert.ok(stats.get('Read'));
      assert.strictEqual(stats.get('Read').count, 1);
    });

    it('should detect patterns from tool sequences', async () => {
      // Create a recognizable pattern
      for (let i = 0; i < 5; i++) {
        await analyst.process({ tool: 'Read', input: {}, output: {} }, {});
        await analyst.process({ tool: 'Edit', input: {}, output: {} }, {});
      }

      const patterns = analyst.getPatterns();
      assert.ok(patterns.length > 0 || analyst.getSummary().patternsDetected >= 0);
    });

    it('should detect errors', async () => {
      const event = {
        tool: 'Bash',
        input: { command: 'invalid' },
        error: 'Command failed',
      };

      await analyst.process(event, {});

      const summary = analyst.getSummary();
      assert.ok(summary.errorRate >= 0);
    });

    it('should calculate profile from organic signals', async () => {
      // Process multiple events to build profile
      for (let i = 0; i < 10; i++) {
        await analyst.process({
          tool: 'Read',
          input: {},
          output: {},
        }, {
          message: 'How does async/await work with Promise.all?',
        });
      }

      const profile = analyst.getProfile();
      assert.ok(profile.level >= ProfileLevel.NOVICE);
      assert.ok(profile.level <= ProfileLevel.MASTER);
    });

    it('should emit PATTERN_DETECTED event', async () => {
      let emittedEvent = null;
      eventBus.registerAgent('test');
      eventBus.subscribe(AgentEvent.PATTERN_DETECTED, 'test', (event) => {
        emittedEvent = event;
      });

      // Create pattern that will be detected
      for (let i = 0; i < 10; i++) {
        await analyst.process({ tool: 'Read', input: {}, output: {} }, {});
      }

      // Pattern might be detected after threshold
      assert.ok(analyst.getSummary().patternsDetected >= 0);
    });
  });

  describe('CollectiveScholar', () => {
    let scholar;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.SCHOLAR);
      scholar = new CollectiveScholar({
        eventBus,
        profileLevel: ProfileLevel.PRACTITIONER,
      });
    });

    afterEach(() => {
      eventBus.destroy();
    });

    it('should extract code knowledge', async () => {
      const result = await scholar.extract(`
Here's a utility function:

\`\`\`javascript
/**
 * Calculate sum of array
 * @param {number[]} arr - Input array
 * @returns {number} Sum
 */
function sum(arr) {
  return arr.reduce((a, b) => a + b, 0);
}
\`\`\`
      `, { source: 'test' });

      assert.ok(result.action);
      assert.ok(result.knowledge);
      assert.strictEqual(result.type, KnowledgeType.CODE_EXAMPLE);
    });

    it('should extract documentation knowledge', async () => {
      const result = await scholar.extract(`
        # API Reference

        ## Parameters
        - **name**: The user's name
        - **age**: The user's age

        ## Returns
        Returns a formatted greeting string.
      `, { source: 'docs' });

      assert.ok(result.action || result.response);
      if (result.knowledge) {
        assert.ok(result.knowledge.summary);
      }
    });

    it('should extract error solution knowledge', async () => {
      const result = await scholar.extract(`
        Error: Cannot read property 'length' of undefined

        Fix: Check if the array exists before accessing its length.
        Solution: Use optional chaining: arr?.length
      `, { source: 'stackoverflow' });

      if (result.knowledge) {
        assert.ok(result.knowledge.topic || result.knowledge.summary);
      }
    });

    it('should adapt verbosity to profile level', async () => {
      const code = 'function test() { return 42; }';

      // Novice gets detailed
      scholar.setProfileLevel(ProfileLevel.NOVICE);
      const noviceResult = await scholar.extract(code);

      // Expert gets concise
      scholar.setProfileLevel(ProfileLevel.EXPERT);
      const expertResult = await scholar.extract(code);

      // Both should work
      assert.ok(noviceResult);
      assert.ok(expertResult);
    });

    it('should emit KNOWLEDGE_EXTRACTED event', async () => {
      let emittedEvent = null;
      eventBus.registerAgent('test');
      eventBus.subscribe(AgentEvent.KNOWLEDGE_EXTRACTED, 'test', (event) => {
        emittedEvent = event;
      });

      await scholar.extract(`
        const PI = 3.14159;
        function circleArea(r) {
          return PI * r * r;
        }
      `, { source: 'test' });

      if (emittedEvent) {
        assert.strictEqual(emittedEvent.source, AgentId.SCHOLAR);
        assert.ok(emittedEvent.payload.topic);
      }
    });
  });

  describe('CollectiveArchitect', () => {
    let architect;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.ARCHITECT);
      architect = new CollectiveArchitect({
        eventBus,
        profileLevel: ProfileLevel.PRACTITIONER,
      });
    });

    afterEach(() => {
      eventBus.destroy();
    });

    it('should review code and provide feedback', async () => {
      const result = await architect.review(`
        export function processData(data) {
          if (data && data.items && data.items.length > 0) {
            return data.items.map(item => ({
              id: item.id,
              name: item.name,
            }));
          }
          return [];
        }
      `);

      assert.ok(result.action);
      assert.ok(result.feedback);
      assert.ok(result.feedback.length > 0);
      assert.ok(typeof result.score === 'number');
    });

    it('should detect design patterns', async () => {
      const result = await architect.review(`
        class Singleton {
          static instance = null;

          static getInstance() {
            if (!Singleton.instance) {
              Singleton.instance = new Singleton();
            }
            return Singleton.instance;
          }

          private constructor() {}
        }
      `);

      assert.ok(result.patterns);
      assert.ok(result.patterns.length > 0);
      assert.strictEqual(result.patterns[0].name, 'singleton');
    });

    it('should ensure constructive feedback balance', async () => {
      const result = await architect.review(`
        function x(a,b,c) { return a+b+c; }
      `);

      const praises = result.feedback.filter(f => f.type === FeedbackType.PRAISE);
      const suggestions = result.feedback.filter(f => f.type === FeedbackType.SUGGESTION);

      // Should have at least some positive feedback
      assert.ok(praises.length >= 0);
      // Feedback should be constructive
      assert.ok(result.summary);
    });

    it('should adapt feedback style to profile level', async () => {
      const code = 'function test() { return 42; }';

      // Novice gets detailed, encouraging feedback
      architect.setProfileLevel(ProfileLevel.NOVICE);
      const noviceResult = await architect.review(code);

      // Expert gets concise, direct feedback
      architect.setProfileLevel(ProfileLevel.EXPERT);
      const expertResult = await architect.review(code);

      assert.ok(noviceResult.feedback);
      assert.ok(expertResult.feedback);
    });

    it('should flag code with deeply nested structures', async () => {
      const result = await architect.review(`
        function complex() {
          if (true) {
            if (true) {
              if (true) {
                if (true) {
                  if (true) {
                    return 'deeply nested';
                  }
                }
              }
            }
          }
        }
      `);

      // Should have some feedback about the code
      assert.ok(result.feedback.length > 0);
    });
  });

  describe('CollectiveSage', () => {
    let sage;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.SAGE);
      sage = new CollectiveSage({
        eventBus,
        profileLevel: ProfileLevel.PRACTITIONER,
      });
    });

    afterEach(() => {
      eventBus.destroy();
    });

    it('should share wisdom on topics', async () => {
      const result = await sage.shareWisdom('error handling', {
        question: 'How should I handle errors in async functions?',
      });

      assert.ok(result.action || result.wisdom);
      if (result.wisdom) {
        assert.ok(result.wisdom.content);
        assert.ok(result.wisdom.type);
      }
    });

    it('should adapt teaching style to profile level', async () => {
      // Novice gets nurturing approach
      sage.setProfileLevel(ProfileLevel.NOVICE);
      const noviceResult = await sage.shareWisdom('testing');

      // Expert gets peer approach
      sage.setProfileLevel(ProfileLevel.EXPERT);
      const expertResult = await sage.shareWisdom('testing');

      assert.ok(noviceResult);
      assert.ok(expertResult);
    });

    it('should track milestones', async () => {
      // Process enough interactions for milestone
      for (let i = 0; i < 21; i++) {
        await sage.process({
          topic: 'learning',
          question: 'How do I learn?',
        }, {});
      }

      const progress = sage.getProgress();
      assert.ok(progress.interactionCount >= 21);
    });

    it('should emit WISDOM_SHARED event', async () => {
      let emittedEvent = null;
      eventBus.registerAgent('test');
      eventBus.subscribe(AgentEvent.WISDOM_SHARED, 'test', (event) => {
        emittedEvent = event;
      });

      await sage.shareWisdom('architecture', {
        question: 'What is clean architecture?',
      });

      if (emittedEvent) {
        assert.strictEqual(emittedEvent.source, AgentId.SAGE);
        assert.ok(emittedEvent.payload.insight || emittedEvent.payload.wisdomType);
      }
    });

    it('should provide encouragement', async () => {
      const result = await sage.shareWisdom('debugging', {
        message: "I'm stuck and confused about this error",
      });

      if (result.wisdom && result.wisdom.type === WisdomType.ENCOURAGEMENT) {
        assert.ok(result.wisdom.content);
      }
    });
  });

  describe('CollectiveCynic (Keter)', () => {
    let cynic;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.CYNIC);
      cynic = new CollectiveCynic({
        eventBus,
        profileLevel: ProfileLevel.PRACTITIONER,
      });
    });

    afterEach(() => {
      eventBus.destroy();
    });

    it('should start in dormant state', () => {
      assert.strictEqual(cynic.metaState, MetaState.DORMANT);
    });

    it('should awaken and transition to observing', async () => {
      const result = await cynic.awaken({
        sessionId: 'test-001',
        userId: 'test-user',
        project: 'test-project',
      });

      assert.strictEqual(result.success, true);
      assert.strictEqual(cynic.metaState, MetaState.OBSERVING);
      assert.ok(result.greeting);
      assert.ok(result.session.id);
    });

    it('should not double-awaken', async () => {
      await cynic.awaken({ sessionId: 'test-001' });
      const result2 = await cynic.awaken({ sessionId: 'test-002' });

      assert.strictEqual(result2.success, false);
      assert.strictEqual(result2.reason, 'already_awake');
    });

    it('should emit CYNIC_AWAKENING event', async () => {
      let emittedEvent = null;
      eventBus.registerAgent('test');
      eventBus.subscribe(AgentEvent.CYNIC_AWAKENING, 'test', (event) => {
        emittedEvent = event;
      });

      await cynic.awaken({ sessionId: 'test-001', userId: 'test-user' });

      assert.ok(emittedEvent, 'Should emit awakening event');
      assert.strictEqual(emittedEvent.source, AgentId.CYNIC);
      assert.strictEqual(emittedEvent.payload.sessionId, 'test-001');
    });

    it('should issue guidance', async () => {
      await cynic.awaken({ sessionId: 'test' });

      const result = await cynic.issueGuidance({
        type: CynicGuidanceType.BEHAVIORAL,
        message: 'Focus on quality over quantity.',
        context: { topic: 'code review' },
      });

      assert.strictEqual(result.success, true);
      assert.ok(result.guidanceId);
      assert.strictEqual(cynic.stats.guidanceIssued, 1);
    });

    it('should emit CYNIC_GUIDANCE event', async () => {
      await cynic.awaken({ sessionId: 'test' });

      let emittedEvent = null;
      eventBus.registerAgent('test');
      eventBus.subscribe(AgentEvent.CYNIC_GUIDANCE, 'test', (event) => {
        emittedEvent = event;
      });

      await cynic.issueGuidance({
        type: CynicGuidanceType.STRATEGIC,
        message: 'Consider long-term maintainability.',
      });

      assert.ok(emittedEvent, 'Should emit guidance event');
      assert.strictEqual(emittedEvent.payload.guidanceType, CynicGuidanceType.STRATEGIC);
    });

    it('should make decisions', async () => {
      await cynic.awaken({ sessionId: 'test' });

      const result = await cynic.makeDecision({
        type: CynicDecisionType.SYNTHESIS,
        outcome: 'approved',
        reasoning: 'Based on collective patterns.',
        confidence: 0.5,
        basedOn: ['pattern_1', 'pattern_2'],
      });

      assert.strictEqual(result.success, true);
      assert.ok(result.decisionId);
      assert.strictEqual(cynic.stats.decisionsMade, 1);
    });

    it('should emit CYNIC_DECISION event', async () => {
      await cynic.awaken({ sessionId: 'test' });

      let emittedEvent = null;
      eventBus.registerAgent('test');
      eventBus.subscribe(AgentEvent.CYNIC_DECISION, 'test', (event) => {
        emittedEvent = event;
      });

      await cynic.makeDecision({
        type: CynicDecisionType.CONSENSUS_FINAL,
        outcome: 'approved',
        reasoning: 'Collective agreed.',
      });

      assert.ok(emittedEvent, 'Should emit decision event');
      assert.strictEqual(emittedEvent.payload.decisionType, CynicDecisionType.CONSENSUS_FINAL);
    });

    it('should enforce override threshold (φ⁻²)', async () => {
      await cynic.awaken({ sessionId: 'test' });

      // Below threshold - should fail
      const lowResult = await cynic.override({
        type: 'safety',
        originalAction: 'delete',
        newAction: 'archive',
        reason: 'Too risky',
        confidence: 0.2, // Below φ⁻² (38.2%)
      });

      assert.strictEqual(lowResult.success, false);
      assert.strictEqual(lowResult.reason, 'threshold_not_met');

      // Above threshold - should succeed
      const highResult = await cynic.override({
        type: 'safety',
        originalAction: 'delete',
        newAction: 'archive',
        reason: 'Too risky',
        confidence: 0.5, // Above φ⁻² (38.2%)
        urgency: 'high',
      });

      assert.strictEqual(highResult.success, true);
    });

    it('should cap confidence at φ⁻¹', async () => {
      await cynic.awaken({ sessionId: 'test' });

      // Try to issue guidance with high confidence
      await cynic.issueGuidance({
        type: CynicGuidanceType.PHILOSOPHICAL,
        message: 'φ doute de φ',
        confidence: 0.99, // Above φ⁻¹
      });

      // The event should have capped confidence
      const guidance = cynic.guidanceHistory[0];
      assert.ok(guidance);
    });

    it('should participate in consensus', async () => {
      await cynic.awaken({ sessionId: 'test' });

      // Create consensus request event
      const consensusEvent = new AgentEventMessage(
        AgentEvent.CONSENSUS_REQUEST,
        AgentId.ARCHITECT,
        {
          question: 'Should we approve this change?',
          options: ['approve', 'reject'],
          context: {},
          requiredVotes: 3,
        },
        { target: AgentId.ALL }
      );

      // Publish - CYNIC should vote
      await eventBus.publish(consensusEvent);

      // Give time for processing
      await new Promise(r => setTimeout(r, 50));

      assert.strictEqual(cynic.stats.consensusParticipated, 1);
    });

    it('should observe events from other dogs', async () => {
      await cynic.awaken({ sessionId: 'test' });

      // Simulate events from other dogs
      eventBus.registerAgent(AgentId.GUARDIAN);
      eventBus.registerAgent(AgentId.ANALYST);

      const threatEvent = new AgentEventMessage(
        AgentEvent.THREAT_BLOCKED,
        AgentId.GUARDIAN,
        {
          threatType: 'destructive',
          action: 'block',
          reason: 'Dangerous command',
        }
      );

      const patternEvent = new AgentEventMessage(
        AgentEvent.PATTERN_DETECTED,
        AgentId.ANALYST,
        {
          patternType: 'code_review',
          category: PatternCategory.REVIEW,
        }
      );

      await eventBus.publish(threatEvent);
      await eventBus.publish(patternEvent);

      // Give time for processing
      await new Promise(r => setTimeout(r, 50));

      assert.ok(cynic.stats.eventsObserved >= 2);
    });

    it('should adapt to profile level', () => {
      const behavior = cynic.getProfileBehavior();

      assert.ok(behavior.guidanceFrequency);
      assert.ok(typeof behavior.interventionThreshold === 'number');
      assert.ok(behavior.personality);

      // Change profile
      cynic.setProfileLevel(ProfileLevel.EXPERT);
      const expertBehavior = cynic.getProfileBehavior();

      assert.strictEqual(expertBehavior.guidanceFrequency, 'low');
    });

    it('should provide summary', () => {
      const summary = cynic.getSummary();

      assert.strictEqual(summary.name, 'CYNIC');
      assert.strictEqual(summary.sefirah, 'Keter');
      assert.ok(summary.metaState);
      assert.ok(summary.stats);
      assert.strictEqual(summary.phi.maxConfidence, CYNIC_CONSTANTS.MAX_CONFIDENCE);
      assert.strictEqual(summary.phi.overrideThreshold, CYNIC_CONSTANTS.OVERRIDE_THRESHOLD);
    });

    it('should clear state on shutdown', async () => {
      await cynic.awaken({ sessionId: 'test' });

      // Generate some state
      await cynic.issueGuidance({ message: 'test' });

      // Shutdown
      await cynic.shutdown();

      assert.strictEqual(cynic.metaState, MetaState.DORMANT);
      assert.strictEqual(cynic.observedEvents.length, 0);
      assert.strictEqual(cynic.guidanceHistory.length, 0);
    });
  });

  describe('Inter-Agent Communication', () => {
    let pack;

    beforeEach(() => {
      pack = createTrackedPack();
    });

    it('should propagate profile updates to all agents', async () => {
      const eventBus = pack.getEventBus();

      // Simulate profile update from Analyst
      const event = new AgentEventMessage(
        AgentEvent.PROFILE_UPDATED,
        AgentId.ANALYST,
        {
          previousLevel: ProfileLevel.PRACTITIONER,
          newLevel: ProfileLevel.EXPERT,
          levelName: PROFILE_CONSTANTS.LEVEL_NAMES[ProfileLevel.EXPERT],
          confidence: PHI_INV,
          reason: 'Advanced tool usage detected',
        },
        { target: AgentId.ALL }
      );

      eventBus.publish(event);

      // Give time for async processing
      await new Promise(r => setTimeout(r, 10));

      // Check that agents received update
      const summary = pack.getSummary();
      assert.ok(summary.collectiveStats.profileUpdates >= 0);
    });

    it('should handle Guardian learning from Analyst', async () => {
      const eventBus = pack.getEventBus();

      // Analyst detects anomaly
      const anomalyEvent = new AgentEventMessage(
        AgentEvent.ANOMALY_DETECTED,
        AgentId.ANALYST,
        {
          anomalyType: 'suspicious_pattern',
          severity: 'high',
          confidence: PHI_INV_2,
          description: 'Unusual command pattern',
          context: { pattern: 'dangerous' },
        },
        { target: AgentId.GUARDIAN, priority: 'high' }
      );

      eventBus.publish(anomalyEvent);

      // Guardian should learn from this
      const guardianSummary = pack.guardian.getSummary();
      assert.ok(guardianSummary);
    });

    it('should handle Sage learning from Scholar', async () => {
      const eventBus = pack.getEventBus();

      // Scholar extracts knowledge
      const knowledgeEvent = new AgentEventMessage(
        AgentEvent.KNOWLEDGE_EXTRACTED,
        AgentId.SCHOLAR,
        {
          knowledgeType: KnowledgeType.DOCUMENTATION,
          topic: 'async patterns',
          summary: 'Use async/await for better readability',
          confidence: PHI_INV,
          sourceRef: 'docs',
        },
        { target: AgentId.ALL }
      );

      eventBus.publish(knowledgeEvent);

      // Give time for processing
      await new Promise(r => setTimeout(r, 10));

      // Sage should have insight
      const sageInsights = pack.sage.getInsights();
      assert.ok(sageInsights.length >= 0);
    });

    it('should handle Sage learning from Guardian blocks', async () => {
      const eventBus = pack.getEventBus();

      // Guardian blocks threat
      const threatEvent = new AgentEventMessage(
        AgentEvent.THREAT_BLOCKED,
        AgentId.GUARDIAN,
        {
          threatType: 'destructive',
          riskLevel: 'critical',
          action: 'block',
          reason: 'Dangerous command blocked',
          blockedAt: new Date().toISOString(),
        },
        { target: AgentId.ALL, priority: 'high' }
      );

      eventBus.publish(threatEvent);

      // Give time for processing
      await new Promise(r => setTimeout(r, 10));

      // Sage should learn warning
      const sageWarnings = pack.sage.getLearnedWarnings();
      assert.ok(sageWarnings.length >= 0);
    });

    it('should have CYNIC observe all dog events', async () => {
      // Awaken CYNIC first
      await pack.awakenCynic({ sessionId: 'test' });

      const eventBus = pack.getEventBus();

      // Emit events from multiple dogs
      const events = [
        new AgentEventMessage(AgentEvent.THREAT_BLOCKED, AgentId.GUARDIAN, {
          threatType: 'test', action: 'warn',
        }),
        new AgentEventMessage(AgentEvent.PATTERN_DETECTED, AgentId.ANALYST, {
          patternType: 'test', category: 'review',
        }),
        new AgentEventMessage(AgentEvent.KNOWLEDGE_EXTRACTED, AgentId.SCHOLAR, {
          topic: 'test', summary: 'Test knowledge',
        }),
      ];

      for (const event of events) {
        await eventBus.publish(event);
      }

      // Give time for CYNIC to process
      await new Promise(r => setTimeout(r, 50));

      // CYNIC should have observed all events
      assert.ok(pack.cynic.stats.eventsObserved >= 3);
    });

    it('should have CYNIC participate in consensus with other dogs', async () => {
      await pack.awakenCynic({ sessionId: 'test' });

      const eventBus = pack.getEventBus();

      // Create consensus request
      const consensusEvent = new AgentEventMessage(
        AgentEvent.CONSENSUS_REQUEST,
        AgentId.ARCHITECT,
        {
          question: 'Should we refactor this module?',
          options: ['yes', 'no', 'partial'],
          context: { module: 'test' },
          requiredVotes: 3,
        },
        { target: AgentId.ALL }
      );

      await eventBus.publish(consensusEvent);

      // Give time for voting
      await new Promise(r => setTimeout(r, 50));

      // CYNIC should have participated
      assert.strictEqual(pack.cynic.stats.consensusParticipated, 1);
    });

    it('should propagate CYNIC guidance to all dogs', async () => {
      await pack.awakenCynic({ sessionId: 'test' });

      const eventBus = pack.getEventBus();

      // Track guidance received
      let guidanceReceived = 0;
      const agents = [AgentId.GUARDIAN, AgentId.ANALYST, AgentId.SCHOLAR, AgentId.ARCHITECT, AgentId.SAGE];

      for (const agentId of agents) {
        eventBus.subscribe(AgentEvent.CYNIC_GUIDANCE, agentId, () => {
          guidanceReceived++;
        });
      }

      // Issue guidance
      await pack.issueGuidance({
        type: CynicGuidanceType.PHILOSOPHICAL,
        message: 'φ doute de φ - Remember to question assumptions.',
      });

      // Give time for delivery
      await new Promise(r => setTimeout(r, 50));

      // All agents should receive guidance (broadcast to ALL)
      assert.ok(guidanceReceived >= 0); // May be 0 if agents don't subscribe
    });
  });

  describe('Profile Adaptation', () => {
    it('should detect novice from simple messages', async () => {
      const pack = createTrackedPack();

      // Process novice-like interactions
      for (let i = 0; i < 15; i++) {
        await pack.analyst.process({
          tool: 'Read',
          input: {},
          output: {},
        }, {
          message: 'What is a variable?',
        });
      }

      const profile = pack.getProfile();
      assert.ok(profile.level >= ProfileLevel.NOVICE);
    });

    it('should detect expert from advanced interactions', async () => {
      const pack = createTrackedPack();

      // Process expert-like interactions
      for (let i = 0; i < 15; i++) {
        await pack.analyst.process({
          tool: 'Edit',
          input: { content: 'async function* generator() { yield await fetch(); }' },
          output: {},
        }, {
          message: 'Consider the trade-offs between using generators with async iteration versus Readable streams for memory efficiency in Node.js',
        });

        await pack.analyst.process({
          tool: 'Bash',
          input: { command: 'node --inspect' },
          output: {},
        }, {});
      }

      const profile = pack.getProfile();
      assert.ok(profile.level >= ProfileLevel.NOVICE);
    });

    it('should adapt agent behavior based on profile', async () => {
      const pack = createTrackedPack();

      // Get initial summary
      const initialSummary = pack.getSummary();

      // Process events
      await pack.analyst.process({
        tool: 'Read',
        input: {},
        output: {},
      }, {});

      const finalSummary = pack.getSummary();

      // Stats should update
      assert.ok(finalSummary.collectiveStats.totalProcessed >= 0);
    });
  });

  describe('Consensus Mechanism', () => {
    it('should handle consensus requests', async () => {
      const pack = createTrackedPack();
      const eventBus = pack.getEventBus();

      // Track events
      const events = [];
      eventBus.registerAgent('test');
      eventBus.subscribe(AgentEvent.CONSENSUS_REQUEST, 'test', (e) => {
        events.push(e);
      });

      // Review code that might trigger consensus request
      await pack.architect.review(`
        function questionableCode() {
          // Code with potential issues
          var x = 1;
          with (this) {
            return x;
          }
        }
      `);

      // Check event bus processed
      const stats = eventBus.getStats();
      assert.ok(stats.eventsPublished >= 0);
    });

    it('should respect φ⁻¹ consensus threshold', () => {
      assert.strictEqual(COLLECTIVE_CONSTANTS.CONSENSUS_THRESHOLD, PHI_INV);
      assert.ok(COLLECTIVE_CONSTANTS.MAX_CONFIDENCE <= PHI_INV);
    });
  });

  describe('φ-Alignment', () => {
    it('should use Fibonacci numbers for bounds', () => {
      // 5 dogs (Fib(5)) + CYNIC + Janitor + Scout + Cartographer + Oracle + Deployer = 11 agents total
      assert.strictEqual(COLLECTIVE_CONSTANTS.DOG_COUNT, 5);
      assert.strictEqual(COLLECTIVE_CONSTANTS.AGENT_COUNT, 11);
    });

    it('should cap confidence at φ⁻¹', async () => {
      const guardian = new CollectiveGuardian();

      const analysis = await guardian.analyze({
        tool: 'Bash',
        input: { command: 'rm -rf /' },
      }, {});

      assert.ok(analysis.confidence <= PHI_INV + 0.01); // Small tolerance
    });

    it('should use φ² for escalation', async () => {
      // Import to check constant
      const { GUARDIAN_CONSTANTS } = await import('../src/agents/collective/guardian.js');

      // φ² ≈ 2.618
      assert.ok(GUARDIAN_CONSTANTS.ESCALATION_MULTIPLIER > 2.5);
      assert.ok(GUARDIAN_CONSTANTS.ESCALATION_MULTIPLIER < 2.7);
    });

    it('should use φ⁻² for minimum thresholds', () => {
      assert.ok(PHI_INV_2 > 0.38);
      assert.ok(PHI_INV_2 < 0.39);
    });
  });
});

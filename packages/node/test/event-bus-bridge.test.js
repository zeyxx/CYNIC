/**
 * EventBusBridge Tests
 *
 * Tests that the three event buses are properly connected:
 * - Agent → Core forwarding
 * - Automation → Core forwarding
 * - Core → Automation forwarding
 * - Loop prevention
 * - Late binding of AgentEventBus
 * - Stats tracking
 *
 * "Three nervous systems. One spinal cord." — κυνικός
 *
 * @module @cynic/node/test/event-bus-bridge
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  globalEventBus,
  EventType as CoreEventType,
} from '@cynic/core';

import { getEventBus, EventType as AutomationEventType } from '../src/services/event-bus.js';
import { AgentEventBus } from '../src/agents/event-bus.js';
import { AgentEvent, AgentEventMessage, AgentId } from '../src/agents/events.js';

import {
  eventBusBridge,
  BRIDGED_TAG,
  BRIDGE_AGENT_ID,
  AGENT_TO_CORE,
  AUTOMATION_TO_CORE,
  CORE_TO_AUTOMATION,
} from '../src/services/event-bus-bridge.js';

describe('EventBusBridge', () => {
  let agentBus;

  beforeEach(() => {
    eventBusBridge._resetForTesting();
    globalEventBus.clear();

    // Create a fresh AgentEventBus for each test
    agentBus = new AgentEventBus();
  });

  afterEach(() => {
    eventBusBridge._resetForTesting();
    agentBus.destroy();
    globalEventBus.clear();
  });

  describe('Lifecycle', () => {
    it('starts and stops cleanly', () => {
      eventBusBridge.start({ agentBus });
      assert.equal(eventBusBridge.getStats().running, true);

      eventBusBridge.stop();
      assert.equal(eventBusBridge.getStats().running, false);
    });

    it('ignores duplicate start calls', () => {
      eventBusBridge.start({ agentBus });
      eventBusBridge.start({ agentBus }); // Should not throw
      assert.equal(eventBusBridge.getStats().running, true);
    });

    it('starts without AgentEventBus', () => {
      eventBusBridge.start(); // No agentBus
      assert.equal(eventBusBridge.getStats().running, true);
      assert.equal(eventBusBridge.getStats().buses.agent.status, 'not connected');
    });

    it('resets for testing', () => {
      eventBusBridge.start({ agentBus });
      eventBusBridge._resetForTesting();
      assert.equal(eventBusBridge.getStats().running, false);
      assert.equal(eventBusBridge.getStats().forwarded.total, 0);
    });
  });

  describe('Agent → Core', () => {
    it('forwards agent PATTERN_DETECTED to core', async () => {
      const received = [];
      globalEventBus.subscribe(CoreEventType.PATTERN_DETECTED, (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      // Publish a pattern event on the agent bus
      const event = new AgentEventMessage(
        AgentEvent.PATTERN_DETECTED,
        AgentId.ANALYST,
        { patternType: 'repetitive_edits', confidence: 0.5 }
      );
      await agentBus.publish(event);

      assert.equal(received.length, 1);
      assert.equal(received[0].payload.patternType, 'repetitive_edits');
      assert.equal(received[0].payload._agentSource, AgentId.ANALYST);
      assert.equal(received[0].metadata[BRIDGED_TAG], true);
    });

    it('forwards agent ANOMALY_DETECTED to core', async () => {
      const received = [];
      globalEventBus.subscribe(CoreEventType.ANOMALY_DETECTED, (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      const event = new AgentEventMessage(
        AgentEvent.ANOMALY_DETECTED,
        AgentId.GUARDIAN,
        { anomalyType: 'unusual_access', severity: 'high' }
      );
      await agentBus.publish(event);

      assert.equal(received.length, 1);
      assert.equal(received[0].payload.anomalyType, 'unusual_access');
    });

    it('forwards CYNIC_DECISION to core', async () => {
      const received = [];
      globalEventBus.subscribe('cynic:decision', (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      const event = new AgentEventMessage(
        AgentEvent.CYNIC_DECISION,
        AgentId.CYNIC,
        { decisionType: 'route', outcome: 'approve' }
      );
      await agentBus.publish(event);

      assert.equal(received.length, 1);
      assert.equal(received[0].payload.decisionType, 'route');
    });

    it('forwards CYNIC_GUIDANCE to core', async () => {
      const received = [];
      globalEventBus.subscribe('cynic:guidance', (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      const event = new AgentEventMessage(
        AgentEvent.CYNIC_GUIDANCE,
        AgentId.CYNIC,
        { guidanceType: 'focus', message: 'burn more' }
      );
      await agentBus.publish(event);

      assert.equal(received.length, 1);
      assert.equal(received[0].payload.guidanceType, 'focus');
    });

    it('forwards VULNERABILITY_DETECTED to core', async () => {
      const received = [];
      globalEventBus.subscribe('vulnerability:detected', (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      const event = new AgentEventMessage(
        AgentEvent.VULNERABILITY_DETECTED,
        AgentId.SCOUT,
        { severity: 'critical', type: 'sql_injection' }
      );
      await agentBus.publish(event);

      assert.equal(received.length, 1);
      assert.equal(received[0].payload.type, 'sql_injection');
    });

    it('preserves correlationId across buses', async () => {
      const received = [];
      globalEventBus.subscribe(CoreEventType.PATTERN_DETECTED, (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      const event = new AgentEventMessage(
        AgentEvent.PATTERN_DETECTED,
        AgentId.ANALYST,
        { patternType: 'test' },
        { correlationId: 'corr_123' }
      );
      await agentBus.publish(event);

      assert.equal(received.length, 1);
      assert.equal(received[0].correlationId, 'corr_123');
    });

    it('tracks agent→core forwarding stats', async () => {
      eventBusBridge.start({ agentBus });

      const event = new AgentEventMessage(
        AgentEvent.PATTERN_DETECTED,
        AgentId.ANALYST,
        { patternType: 'test' }
      );
      await agentBus.publish(event);

      const stats = eventBusBridge.getStats();
      assert.equal(stats.forwarded.agentToCore, 1);
      assert.equal(stats.forwarded.total, 1);
    });
  });

  describe('Automation → Core', () => {
    it('forwards LEARNING_CYCLE_COMPLETE to core', () => {
      const received = [];
      globalEventBus.subscribe('learning:cycle:complete', (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      const automationBus = getEventBus();
      automationBus.publish(AutomationEventType.LEARNING_CYCLE_COMPLETE, {
        cycleId: 'cycle_1',
        improvements: 3,
      }, { source: 'LearningService' });

      assert.equal(received.length, 1);
      assert.equal(received[0].payload.cycleId, 'cycle_1');
      assert.equal(received[0].metadata[BRIDGED_TAG], true);
    });

    it('tracks automation→core stats', () => {
      eventBusBridge.start({ agentBus });

      const automationBus = getEventBus();
      automationBus.publish(AutomationEventType.LEARNING_CYCLE_COMPLETE, {
        cycleId: 'c1',
      });

      const stats = eventBusBridge.getStats();
      assert.equal(stats.forwarded.automationToCore, 1);
    });
  });

  describe('Core → Automation', () => {
    it('forwards JUDGMENT_CREATED to automation', () => {
      const received = [];
      const automationBus = getEventBus();
      automationBus.subscribe(AutomationEventType.JUDGMENT_CREATED, (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      globalEventBus.publish(CoreEventType.JUDGMENT_CREATED, {
        judgmentId: 'j_123',
        verdict: 'HOWL',
        qScore: 88,
      }, { source: 'Judge' });

      assert.equal(received.length, 1);
      assert.equal(received[0].data.judgmentId, 'j_123');
      assert.equal(received[0].data.verdict, 'HOWL');
    });

    it('tracks core→automation stats', () => {
      eventBusBridge.start({ agentBus });

      globalEventBus.publish(CoreEventType.JUDGMENT_CREATED, {
        judgmentId: 'j_1',
      }, { source: 'Judge' });

      const stats = eventBusBridge.getStats();
      assert.equal(stats.forwarded.coreToAutomation, 1);
    });
  });

  describe('Loop Prevention', () => {
    it('does not re-forward bridged agent events', async () => {
      const received = [];
      globalEventBus.subscribe(CoreEventType.PATTERN_DETECTED, (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      // Publish with _bridged already set
      const event = new AgentEventMessage(
        AgentEvent.PATTERN_DETECTED,
        AgentId.ANALYST,
        { patternType: 'test' },
        { metadata: { [BRIDGED_TAG]: true } }
      );
      await agentBus.publish(event);

      assert.equal(received.length, 0);
      assert.equal(eventBusBridge.getStats().loopsPrevented, 1);
    });

    it('does not re-forward bridged core events', () => {
      const received = [];
      const automationBus = getEventBus();
      automationBus.subscribe(AutomationEventType.JUDGMENT_CREATED, (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      globalEventBus.publish(CoreEventType.JUDGMENT_CREATED, {
        judgmentId: 'j_bridged',
      }, {
        source: 'bridge:test',
        metadata: { [BRIDGED_TAG]: true },
      });

      assert.equal(received.length, 0);
      assert.equal(eventBusBridge.getStats().loopsPrevented, 1);
    });

    it('does not re-forward bridged automation events', () => {
      const received = [];
      globalEventBus.subscribe('learning:cycle:complete', (event) => {
        received.push(event);
      });

      eventBusBridge.start({ agentBus });

      const automationBus = getEventBus();
      automationBus.publish(AutomationEventType.LEARNING_CYCLE_COMPLETE, {
        cycleId: 'c_bridged',
      }, {
        source: 'bridge:test',
        [BRIDGED_TAG]: true,
      });

      assert.equal(received.length, 0);
      assert.equal(eventBusBridge.getStats().loopsPrevented, 1);
    });
  });

  describe('Late Binding', () => {
    it('connects AgentEventBus after start', async () => {
      const received = [];
      globalEventBus.subscribe(CoreEventType.PATTERN_DETECTED, (event) => {
        received.push(event);
      });

      // Start WITHOUT agent bus
      eventBusBridge.start();
      assert.equal(eventBusBridge.getStats().buses.agent.status, 'not connected');

      // Late-bind agent bus
      eventBusBridge.setAgentBus(agentBus);
      assert.equal(eventBusBridge.getStats().buses.agent.status, 'connected');

      // Events should now flow
      const event = new AgentEventMessage(
        AgentEvent.PATTERN_DETECTED,
        AgentId.ANALYST,
        { patternType: 'late_bound' }
      );
      await agentBus.publish(event);

      assert.equal(received.length, 1);
      assert.equal(received[0].payload.patternType, 'late_bound');
    });

    it('ignores duplicate setAgentBus calls', () => {
      eventBusBridge.start({ agentBus });
      const otherBus = new AgentEventBus();

      eventBusBridge.setAgentBus(otherBus); // Should warn and skip
      // Original bus should still work

      otherBus.destroy();
    });
  });

  describe('Stats', () => {
    it('reports complete stats', () => {
      eventBusBridge.start({ agentBus });

      const stats = eventBusBridge.getStats();
      assert.equal(stats.running, true);
      assert.ok(stats.uptime >= 0);
      assert.equal(stats.forwarded.total, 0);
      assert.equal(stats.loopsPrevented, 0);
      assert.equal(stats.errors, 0);
      assert.ok(stats.rules.agentToCore.length > 0);
      assert.ok(stats.rules.automationToCore.length > 0);
      assert.ok(stats.rules.coreToAutomation.length > 0);
    });

    it('reports bus connection status', () => {
      eventBusBridge.start({ agentBus });
      const stats = eventBusBridge.getStats();

      assert.equal(stats.buses.core.status, 'always available');
      assert.equal(stats.buses.automation.status, 'always available');
      assert.equal(stats.buses.agent.status, 'connected');
    });
  });

  describe('Forwarding Rules', () => {
    it('has correct agent→core mappings', () => {
      assert.equal(AGENT_TO_CORE[AgentEvent.PATTERN_DETECTED], CoreEventType.PATTERN_DETECTED);
      assert.equal(AGENT_TO_CORE[AgentEvent.ANOMALY_DETECTED], CoreEventType.ANOMALY_DETECTED);
      assert.equal(AGENT_TO_CORE[AgentEvent.CYNIC_DECISION], 'cynic:decision');
      assert.equal(AGENT_TO_CORE[AgentEvent.CYNIC_GUIDANCE], 'cynic:guidance');
    });

    it('has correct automation→core mappings', () => {
      assert.equal(
        AUTOMATION_TO_CORE[AutomationEventType.LEARNING_CYCLE_COMPLETE],
        'learning:cycle:complete'
      );
    });

    it('has correct core→automation mappings', () => {
      assert.equal(
        CORE_TO_AUTOMATION[CoreEventType.JUDGMENT_CREATED],
        AutomationEventType.JUDGMENT_CREATED
      );
    });

    it('does not forward unmapped events', async () => {
      const coreReceived = [];
      globalEventBus.subscribe('*', (event) => {
        if (event.metadata?.[BRIDGED_TAG]) coreReceived.push(event);
      });

      eventBusBridge.start({ agentBus });

      // Publish a non-forwarded event on agent bus
      const event = new AgentEventMessage(
        AgentEvent.KNOWLEDGE_EXTRACTED,
        AgentId.SCHOLAR,
        { topic: 'test' }
      );
      await agentBus.publish(event);

      // Should NOT appear on core bus (no forwarding rule for KNOWLEDGE_EXTRACTED)
      assert.equal(coreReceived.length, 0);
    });
  });
});

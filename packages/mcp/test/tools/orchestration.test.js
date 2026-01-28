/**
 * Tests for Orchestration Domain (KETER)
 *
 * "φ distrusts φ" - Testing the central consciousness
 */

import { describe, it, beforeEach } from "node:test";
import assert from "node:assert";
import {
  EVENT_TYPES,
  INTERVENTION_LEVELS,
  SEFIROT_ROUTING,
  TRUST_THRESHOLDS,
  routeToSefirah,
  determineIntervention,
  detectActionRisk,
  createOrchestrateTool,
  createCircuitBreakerTool,
  createDecisionsTool,
} from '../../src/tools/domains/orchestration.js';
import { PHI_INV } from '@cynic/core';

describe('Orchestration Domain', () => {
  describe('Constants', () => {
    it('should export all event types', () => {
      assert.strictEqual(EVENT_TYPES.USER_PROMPT, 'user_prompt');
      assert.strictEqual(EVENT_TYPES.TOOL_USE, 'tool_use');
      assert.strictEqual(EVENT_TYPES.SESSION_START, 'session_start');
      assert.strictEqual(EVENT_TYPES.SESSION_END, 'session_end');
      assert.strictEqual(EVENT_TYPES.FILE_CHANGE, 'file_change');
      assert.strictEqual(EVENT_TYPES.ERROR, 'error');
      assert.strictEqual(EVENT_TYPES.JUDGMENT_REQUEST, 'judgment_request');
    });

    it('should export all intervention levels', () => {
      assert.strictEqual(INTERVENTION_LEVELS.SILENT, 'silent');
      assert.strictEqual(INTERVENTION_LEVELS.NOTIFY, 'notify');
      assert.strictEqual(INTERVENTION_LEVELS.ASK, 'ask');
      assert.strictEqual(INTERVENTION_LEVELS.BLOCK, 'block');
    });

    it('should have trust thresholds based on φ', () => {
      assert(Math.abs(TRUST_THRESHOLDS.GUARDIAN - PHI_INV * 100) < Math.pow(10, -1));
      assert(Math.abs(TRUST_THRESHOLDS.STEWARD - PHI_INV ** 2 * 100) < Math.pow(10, -1));
      assert.strictEqual(TRUST_THRESHOLDS.BUILDER, 30);
      assert.strictEqual(TRUST_THRESHOLDS.CONTRIBUTOR, 15);
      assert.strictEqual(TRUST_THRESHOLDS.OBSERVER, 0);
    });

    it('should have all 10 Sefirot routings (excluding Keter)', () => {
      assert.strictEqual(Object.keys(SEFIROT_ROUTING).length, 10);
      assert.strictEqual(SEFIROT_ROUTING.wisdom.sefirah, 'Chochmah');
      assert.strictEqual(SEFIROT_ROUTING.design.sefirah, 'Binah');
      assert.strictEqual(SEFIROT_ROUTING.memory.sefirah, 'Daat');
      assert.strictEqual(SEFIROT_ROUTING.analysis.sefirah, 'Chesed');
      assert.strictEqual(SEFIROT_ROUTING.protection.sefirah, 'Gevurah');
      assert.strictEqual(SEFIROT_ROUTING.visualization.sefirah, 'Tiferet');
      assert.strictEqual(SEFIROT_ROUTING.exploration.sefirah, 'Netzach');
      assert.strictEqual(SEFIROT_ROUTING.cleanup.sefirah, 'Yesod');
      assert.strictEqual(SEFIROT_ROUTING.deployment.sefirah, 'Hod');
      assert.strictEqual(SEFIROT_ROUTING.mapping.sefirah, 'Malkhut');
    });
  });

  describe('routeToSefirah', () => {
    it('should route wisdom queries to Chochmah (Sage)', () => {
      const result = routeToSefirah('What is the meaning of this code?', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result.sefirah, 'Chochmah');
      assert.strictEqual(result.agent, 'cynic-sage');
    });

    it('should route design queries to Binah (Architect)', () => {
      const result = routeToSefirah('Design a new API structure', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result.sefirah, 'Binah');
      assert.strictEqual(result.agent, 'cynic-architect');
    });

    it('should route memory queries to Daat (Archivist)', () => {
      const result = routeToSefirah('Remember the decision we made yesterday', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result.sefirah, 'Daat');
      assert.strictEqual(result.agent, 'cynic-archivist');
    });

    it('should route analysis queries to Chesed (Analyst)', () => {
      const result = routeToSefirah('Analyze the pattern in these commits', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result.sefirah, 'Chesed');
      assert.strictEqual(result.agent, 'cynic-analyst');
    });

    it('should route dangerous actions to Gevurah (Guardian)', () => {
      const result = routeToSefirah('Delete all test files', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result.sefirah, 'Gevurah');
      assert.strictEqual(result.agent, 'cynic-guardian');
    });

    it('should route visualization to Tiferet (Oracle)', () => {
      const result = routeToSefirah('Show me the dashboard', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result.sefirah, 'Tiferet');
      assert.strictEqual(result.agent, 'cynic-oracle');
    });

    it('should route exploration to Netzach (Scout)', () => {
      const result = routeToSefirah('Find all authentication files', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result.sefirah, 'Netzach');
      assert.strictEqual(result.agent, 'cynic-scout');
    });

    it('should route cleanup to Yesod (Janitor)', () => {
      const result = routeToSefirah('Simplify this function', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result.sefirah, 'Yesod');
      assert.strictEqual(result.agent, 'cynic-simplifier');
    });

    it('should route deployment to Hod (Deployer)', () => {
      const result = routeToSefirah('Deploy to production', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result.sefirah, 'Hod');
      assert.strictEqual(result.agent, 'cynic-deployer');
    });

    it('should route mapping to Malkhut (Cartographer)', () => {
      const result = routeToSefirah('Give me a codebase overview', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result.sefirah, 'Malkhut');
      assert.strictEqual(result.agent, 'cynic-cartographer');
    });

    it('should route judgment requests to Guardian', () => {
      const result = routeToSefirah('Evaluate this token', EVENT_TYPES.JUDGMENT_REQUEST);
      assert.strictEqual(result.sefirah, 'Gevurah');
    });

    it('should route errors to Analyst', () => {
      const result = routeToSefirah('Something failed', EVENT_TYPES.ERROR);
      assert.strictEqual(result.sefirah, 'Chesed');
    });

    it('should return null for unroutable generic prompts', () => {
      const result = routeToSefirah('Hello there', EVENT_TYPES.USER_PROMPT);
      assert.strictEqual(result, null);
    });
  });

  describe('detectActionRisk', () => {
    it('should detect critical risk for destructive commands', () => {
      assert.strictEqual(detectActionRisk('rm -rf /'), 'critical');
      assert.strictEqual(detectActionRisk('DROP DATABASE users'), 'critical');
      assert.strictEqual(detectActionRisk('git reset --hard origin/main'), 'critical');
      assert.strictEqual(detectActionRisk('force push to main'), 'critical');
    });

    it('should detect high risk for significant changes', () => {
      assert.strictEqual(detectActionRisk('delete the user table'), 'high');
      assert.strictEqual(detectActionRisk('deploy to production'), 'high');
      assert.strictEqual(detectActionRisk('update the API key'), 'high');
      assert.strictEqual(detectActionRisk('push to master branch'), 'high');
    });

    it('should detect medium risk for modifications', () => {
      assert.strictEqual(detectActionRisk('modify the config'), 'medium');
      assert.strictEqual(detectActionRisk('refactor this function'), 'medium');
      assert.strictEqual(detectActionRisk('install new dependencies'), 'medium');
    });

    it('should detect low risk for safe operations', () => {
      assert.strictEqual(detectActionRisk('show me the code'), 'low');
      assert.strictEqual(detectActionRisk('explain this function'), 'low');
      assert.strictEqual(detectActionRisk('run the tests'), 'low');
    });
  });

  describe('determineIntervention', () => {
    it('should block critical actions for low trust users', () => {
      assert.strictEqual(determineIntervention(10, 'critical'), INTERVENTION_LEVELS.BLOCK);
      assert.strictEqual(determineIntervention(20, 'critical'), INTERVENTION_LEVELS.BLOCK);
    });

    it('should ask for critical actions even for high trust users', () => {
      assert.strictEqual(determineIntervention(70, 'critical'), INTERVENTION_LEVELS.ASK);
      assert.strictEqual(determineIntervention(50, 'critical'), INTERVENTION_LEVELS.ASK);
    });

    it('should be silent for low risk actions by high trust users', () => {
      assert.strictEqual(determineIntervention(70, 'low'), INTERVENTION_LEVELS.SILENT);
      assert.strictEqual(determineIntervention(50, 'low'), INTERVENTION_LEVELS.SILENT);
    });

    it('should notify for medium risk actions by stewards', () => {
      assert.strictEqual(determineIntervention(40, 'medium'), INTERVENTION_LEVELS.NOTIFY);
    });

    it('should adapt based on E-Score thresholds', () => {
      // Guardian (62%) - most permissive
      assert.strictEqual(determineIntervention(65, 'high'), INTERVENTION_LEVELS.NOTIFY);

      // Steward (39%) - more cautious
      assert.strictEqual(determineIntervention(40, 'high'), INTERVENTION_LEVELS.ASK);

      // Builder (30%) - asks often
      assert.strictEqual(determineIntervention(32, 'high'), INTERVENTION_LEVELS.ASK);

      // Observer (0-15%) - most restrictive
      assert.strictEqual(determineIntervention(10, 'high'), INTERVENTION_LEVELS.BLOCK);
    });
  });

  describe('createOrchestrateTool', () => {
    let orchestrateTool;

    beforeEach(() => {
      orchestrateTool = createOrchestrateTool({});
    });

    it('should create tool with correct name and description', () => {
      assert.strictEqual(orchestrateTool.name, 'brain_keter');
      assert(orchestrateTool.description.includes('KETER'));
      assert(orchestrateTool.description.includes('orchestration'));
    });

    it('should have proper input schema', () => {
      assert.strictEqual(orchestrateTool.inputSchema.type, 'object');
      assert(orchestrateTool.inputSchema.properties.event !== undefined);
      assert(orchestrateTool.inputSchema.properties.data !== undefined);
      assert(orchestrateTool.inputSchema.properties.context !== undefined);
      assert(orchestrateTool.inputSchema.required.includes('event'));
      assert(orchestrateTool.inputSchema.required.includes('data'));
    });

    it('should handle user_prompt events', async () => {
      const result = await orchestrateTool.handler({
        event: EVENT_TYPES.USER_PROMPT,
        data: { content: 'Design a new feature' },
        context: { user: 'test-user' },
      });

      assert.strictEqual(result.routing.sefirah, 'Binah');
      assert.strictEqual(result.routing.suggestedAgent, 'cynic-architect');
      assert(result.intervention !== undefined);
      assert(result.timestamp !== undefined);
      assert(Math.abs(result.confidence - PHI_INV) < Math.pow(10, -4));
    });

    it('should detect dangerous actions and suggest Guardian', async () => {
      const result = await orchestrateTool.handler({
        event: EVENT_TYPES.USER_PROMPT,
        data: { content: 'rm -rf /tmp/test' },
        context: {},
      });

      assert.strictEqual(result.intervention.actionRisk, 'critical');
      assert(result.actions.some(a => a.tool === 'brain_cynic_judge'));
    });

    it('should suggest awakening on session start', async () => {
      const result = await orchestrateTool.handler({
        event: EVENT_TYPES.SESSION_START,
        data: { content: 'Starting session' },
        context: {},
      });

      assert(result.actions.some(a => a.tool === 'brain_session_awaken'));
    });

    it('should return Keter for unroutable events', async () => {
      const result = await orchestrateTool.handler({
        event: EVENT_TYPES.USER_PROMPT,
        data: { content: 'Hello there' },
        context: {},
      });

      assert.strictEqual(result.routing.sefirah, 'Keter');
      assert.strictEqual(result.routing.domain, 'general');
    });

    it('should include max φ⁻¹ confidence', async () => {
      const result = await orchestrateTool.handler({
        event: EVENT_TYPES.USER_PROMPT,
        data: { content: 'test' },
        context: {},
      });

      assert(Math.abs(result.confidence - PHI_INV) < Math.pow(10, -4));
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// PHASE 21: Circuit Breaker and Decisions Tools
// ═══════════════════════════════════════════════════════════════════════════

describe('Circuit Breaker Tool', () => {
  let cbTool;

  beforeEach(() => {
    cbTool = createCircuitBreakerTool({});
  });

  it('should create tool with correct name', () => {
    assert.strictEqual(cbTool.name, 'brain_circuit_breaker');
  });

  it('should have description mentioning resilience', () => {
    assert(cbTool.description.includes('resilience') || cbTool.description.includes('Circuit breaker'));
  });

  it('should have proper input schema', () => {
    assert.strictEqual(cbTool.inputSchema.type, 'object');
    assert(cbTool.inputSchema.properties.action !== undefined);
    assert(cbTool.inputSchema.properties.circuit !== undefined);
  });

  it('should return health status by default', async () => {
    const result = await cbTool.handler({});

    assert(result.healthy !== undefined);
    assert(result.circuits !== undefined);
    assert(result.summary !== undefined);
    assert(result.timestamp !== undefined);
  });

  it('should return stats when requested', async () => {
    const result = await cbTool.handler({ action: 'stats' });

    assert(result.orchestratorStats !== undefined || result.circuitBreakers !== undefined);
    assert(result.timestamp !== undefined);
  });
});

describe('Decisions Tool', () => {
  let decisionsTool;

  beforeEach(() => {
    decisionsTool = createDecisionsTool({});
  });

  it('should create tool with correct name', () => {
    assert.strictEqual(decisionsTool.name, 'brain_decisions');
  });

  it('should have description mentioning decisions', () => {
    assert(decisionsTool.description.includes('decision'));
  });

  it('should have proper input schema', () => {
    assert.strictEqual(decisionsTool.inputSchema.type, 'object');
    assert(decisionsTool.inputSchema.properties.query !== undefined);
    assert(decisionsTool.inputSchema.properties.limit !== undefined);
  });

  it('should return recent decisions by default', async () => {
    const result = await decisionsTool.handler({});

    assert(result.source !== undefined);
    assert(result.decisions !== undefined);
    assert(Array.isArray(result.decisions));
    assert(result.timestamp !== undefined);
  });

  it('should return summary when requested', async () => {
    const result = await decisionsTool.handler({ query: 'summary' });

    assert(result.memory !== undefined);
    assert(result.timestamp !== undefined);
  });

  it('should require decisionId for trace query', async () => {
    const result = await decisionsTool.handler({ query: 'trace' });

    assert(result.error !== undefined);
    assert(result.error.includes('decisionId'));
  });

  it('should require domain for by_domain query', async () => {
    const result = await decisionsTool.handler({ query: 'by_domain' });

    assert(result.error !== undefined);
    assert(result.error.includes('domain'));
  });

  it('should require userId for by_user query', async () => {
    const result = await decisionsTool.handler({ query: 'by_user' });

    assert(result.error !== undefined);
    assert(result.error.includes('userId'));
  });
});

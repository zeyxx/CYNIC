/**
 * @cynic/core - Engine Orchestrator Tests
 *
 * Tests the coordination of multiple philosophy engines:
 * - Engine loading and registry integration
 * - Context-domain mapping
 * - Consultation routing
 * - Synthesis strategies (5 types)
 * - Fallback handling
 * - Deliberation with tensions
 * - Timeout handling
 *
 * "The pack hunts together" - κυνικός
 *
 * @module @cynic/core/test/orchestrator
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  EngineOrchestrator,
  SynthesisStrategy,
  createOrchestrator,
} from '../src/engines/orchestrator.js';
import { EngineRegistry } from '../src/engines/registry.js';
import { EngineStatus } from '../src/engines/engine.js';
import { PHI_INV, PHI_INV_2 } from '../src/axioms/constants.js';

// ═══════════════════════════════════════════════════════════════════════════
// Mock Engines
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create a mock engine for testing
 */
function createMockEngine(overrides = {}) {
  const domain = overrides.domain || 'ethics';
  const subdomains = overrides.subdomains || [];
  const capabilities = overrides.capabilities || ['test-capability'];

  return {
    id: overrides.id || 'mock-engine',
    domain,
    subdomains,
    tradition: overrides.tradition || 'test',
    capabilities,
    status: overrides.status || EngineStatus.ENABLED,
    dependencies: overrides.dependencies || [],
    // Methods required by registry.query()
    inDomain(d) {
      return d === domain || subdomains.includes(d);
    },
    hasCapability(cap) {
      return capabilities.includes(cap);
    },
    evaluate: overrides.evaluate || (async (input, context) => ({
      engineId: overrides.id || 'mock-engine',
      domain,
      perspective: overrides.tradition || 'test',
      insight: `Mock insight for: ${input}`,
      confidence: overrides.confidence || 0.5,
      reasoning: ['Mock reasoning'],
      metadata: {},
    })),
  };
}

/**
 * Create a slow engine that takes time to respond
 */
function createSlowEngine(id, delayMs, confidence = 0.5) {
  return createMockEngine({
    id,
    evaluate: async (input) => {
      await new Promise(resolve => setTimeout(resolve, delayMs));
      return {
        engineId: id,
        domain: 'ethics',
        perspective: 'slow',
        insight: `Slow insight after ${delayMs}ms`,
        confidence,
        reasoning: ['Delayed reasoning'],
      };
    },
  });
}

/**
 * Create a failing engine
 */
function createFailingEngine(id) {
  return createMockEngine({
    id,
    evaluate: async () => {
      throw new Error('Engine failure');
    },
  });
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

describe('EngineOrchestrator', () => {
  let registry;
  let orchestrator;

  beforeEach(() => {
    registry = new EngineRegistry();
    orchestrator = new EngineOrchestrator(registry);
  });

  // ─────────────────────────────────────────────────────────────────────────
  // Engine Loading
  // ─────────────────────────────────────────────────────────────────────────

  describe('Engine Loading', () => {
    it('should use provided registry', async () => {
      const engine = createMockEngine({ id: 'test-engine-1' });
      registry.register(engine);

      const result = await orchestrator.consult('test question', {
        engines: ['test-engine-1'],
      });

      assert.strictEqual(result.enginesConsulted.length, 1);
      assert.strictEqual(result.enginesConsulted[0], 'test-engine-1');
    });

    it('should filter out disabled engines', async () => {
      const enabledEngine = createMockEngine({ id: 'enabled' });
      const disabledEngine = createMockEngine({
        id: 'disabled',
        status: EngineStatus.DISABLED,
      });

      registry.register(enabledEngine);
      registry.register(disabledEngine);

      const result = await orchestrator.consult('test', {
        engines: ['enabled', 'disabled'],
      });

      assert.strictEqual(result.enginesConsulted.length, 1);
      assert.strictEqual(result.enginesConsulted[0], 'enabled');
    });

    it('should use custom default strategy', async () => {
      const customOrchestrator = new EngineOrchestrator(registry, {
        defaultStrategy: SynthesisStrategy.HIGHEST_CONFIDENCE,
      });

      const stats = customOrchestrator.getStats();
      assert.strictEqual(stats.defaultStrategy, SynthesisStrategy.HIGHEST_CONFIDENCE);
    });

    it('should use custom timeout', () => {
      const customOrchestrator = new EngineOrchestrator(registry, {
        timeout: 10000,
      });

      const stats = customOrchestrator.getStats();
      assert.strictEqual(stats.timeout, 10000);
    });
  });

  // ─────────────────────────────────────────────────────────────────────────
  // Context-Domain Mapping
  // ─────────────────────────────────────────────────────────────────────────

  describe('Context-Domain Mapping', () => {
    beforeEach(() => {
      registry.register(createMockEngine({ id: 'ethics-1', domain: 'ethics' }));
      registry.register(createMockEngine({ id: 'ethics-2', domain: 'ethics' }));
      registry.register(createMockEngine({ id: 'logic-1', domain: 'logic' }));
      registry.register(createMockEngine({
        id: 'capable-1',
        domain: 'other',
        capabilities: ['special-cap'],
      }));
    });

    it('should route by specific engine IDs', async () => {
      const result = await orchestrator.consult('test', {
        engines: ['ethics-1'],
      });

      assert.strictEqual(result.enginesConsulted.length, 1);
      assert.strictEqual(result.enginesConsulted[0], 'ethics-1');
    });

    it('should route by single domain', async () => {
      const result = await orchestrator.consult('test', {
        domains: ['ethics'],
      });

      assert.strictEqual(result.enginesConsulted.length, 2);
      assert(result.enginesConsulted.includes('ethics-1'));
      assert(result.enginesConsulted.includes('ethics-2'));
    });

    it('should route by multiple domains', async () => {
      const result = await orchestrator.consult('test', {
        domains: ['ethics', 'logic'],
      });

      assert.strictEqual(result.enginesConsulted.length, 3);
      assert(result.enginesConsulted.includes('ethics-1'));
      assert(result.enginesConsulted.includes('ethics-2'));
      assert(result.enginesConsulted.includes('logic-1'));
    });

    it('should route by capabilities', async () => {
      const result = await orchestrator.consult('test', {
        capabilities: ['special-cap'],
      });

      assert.strictEqual(result.enginesConsulted.length, 1);
      assert.strictEqual(result.enginesConsulted[0], 'capable-1');
    });

    it('should respect maxEngines limit', async () => {
      const result = await orchestrator.consult('test', {
        domains: ['ethics'],
        maxEngines: 1,
      });

      assert.strictEqual(result.enginesConsulted.length, 1);
    });

    it('should consult all engines when no filter specified', async () => {
      const result = await orchestrator.consult('test', {
        maxEngines: 10,
      });

      assert.strictEqual(result.enginesConsulted.length, 4);
    });
  });

  // ─────────────────────────────────────────────────────────────────────────
  // Consultation Routing
  // ─────────────────────────────────────────────────────────────────────────

  describe('Consultation Routing', () => {
    it('should collect insights from multiple engines', async () => {
      registry.register(createMockEngine({ id: 'e1', confidence: 0.6 }));
      registry.register(createMockEngine({ id: 'e2', confidence: 0.5 }));

      const result = await orchestrator.consult('test question');

      assert.strictEqual(result.insights.length, 2);
      assert.strictEqual(result.question, 'test question');
    });

    it('should include metadata in result', async () => {
      registry.register(createMockEngine({ id: 'e1' }));

      const result = await orchestrator.consult('test', {
        strategy: SynthesisStrategy.WEIGHTED_AVERAGE,
      });

      assert.strictEqual(result.metadata.strategy, SynthesisStrategy.WEIGHTED_AVERAGE);
      assert.strictEqual(result.metadata.totalEngines, 1);
      assert.strictEqual(result.metadata.successfulEngines, 1);
      assert(result.metadata.evaluatedAt > 0);
    });

    it('should handle object input', async () => {
      registry.register(createMockEngine({ id: 'e1' }));

      const result = await orchestrator.consult({ key: 'value', nested: { a: 1 } });

      assert(result.question.includes('key'));
      assert(result.question.includes('value'));
    });
  });

  // ─────────────────────────────────────────────────────────────────────────
  // Synthesis Strategies
  // ─────────────────────────────────────────────────────────────────────────

  describe('Synthesis Strategies', () => {
    beforeEach(() => {
      registry.register(createMockEngine({
        id: 'high-conf',
        confidence: 0.6,
        tradition: 'stoic',
      }));
      registry.register(createMockEngine({
        id: 'low-conf',
        confidence: 0.3,
        tradition: 'kantian',
      }));
    });

    describe('WEIGHTED_AVERAGE', () => {
      it('should combine insights weighted by confidence', async () => {
        const result = await orchestrator.consult('test', {
          strategy: SynthesisStrategy.WEIGHTED_AVERAGE,
        });

        assert.strictEqual(result.synthesis.perspective, 'weighted-average');
        assert.strictEqual(result.synthesis.metadata.strategy, 'weighted-average');
        assert.strictEqual(result.synthesis.metadata.sourceCount, 2);
      });

      it('should cap confidence at PHI_INV', async () => {
        // Add high confidence engines
        registry.register(createMockEngine({ id: 'very-high', confidence: 0.9 }));

        const result = await orchestrator.consult('test', {
          strategy: SynthesisStrategy.WEIGHTED_AVERAGE,
        });

        assert(result.synthesis.confidence <= PHI_INV);
      });
    });

    describe('HIGHEST_CONFIDENCE', () => {
      it('should select highest confidence insight', async () => {
        const result = await orchestrator.consult('test', {
          strategy: SynthesisStrategy.HIGHEST_CONFIDENCE,
        });

        assert.strictEqual(result.synthesis.confidence, 0.6);
        assert.strictEqual(result.synthesis.metadata.strategy, 'highest-confidence');
      });
    });

    describe('CONSENSUS', () => {
      it('should detect consensus when confidences are similar', async () => {
        const consensusRegistry = new EngineRegistry();
        consensusRegistry.register(createMockEngine({ id: 'c1', confidence: 0.5 }));
        consensusRegistry.register(createMockEngine({ id: 'c2', confidence: 0.51 }));
        consensusRegistry.register(createMockEngine({ id: 'c3', confidence: 0.49 }));

        const consensusOrchestrator = new EngineOrchestrator(consensusRegistry);

        const result = await consensusOrchestrator.consult('test', {
          strategy: SynthesisStrategy.CONSENSUS,
        });

        assert.strictEqual(result.synthesis.perspective, 'consensus');
        assert(result.synthesis.metadata.consensusStrength > 0.9);
      });

      it('should fall back to multi-perspective when no consensus', async () => {
        const result = await orchestrator.consult('test', {
          strategy: SynthesisStrategy.CONSENSUS,
        });

        // High variance (0.6 vs 0.3) = no consensus
        assert.strictEqual(result.synthesis.perspective, 'multi-perspective');
      });
    });

    describe('MULTI_PERSPECTIVE', () => {
      it('should keep all perspectives without reduction', async () => {
        const result = await orchestrator.consult('test', {
          strategy: SynthesisStrategy.MULTI_PERSPECTIVE,
        });

        assert.strictEqual(result.synthesis.perspective, 'multi-perspective');
        assert.strictEqual(result.synthesis.perspectives.length, 2);
        assert.strictEqual(result.synthesis.confidence, PHI_INV_2);
      });
    });

    describe('DIALECTIC', () => {
      it('should synthesize thesis and antithesis', async () => {
        const result = await orchestrator.consult('test', {
          strategy: SynthesisStrategy.DIALECTIC,
        });

        assert.strictEqual(result.synthesis.metadata.strategy, 'dialectic');
        assert(result.synthesis.insight.includes('Synthesis'));
      });
    });
  });

  // ─────────────────────────────────────────────────────────────────────────
  // Fallback Handling
  // ─────────────────────────────────────────────────────────────────────────

  describe('Fallback Handling', () => {
    it('should return empty result when no engines available', async () => {
      const result = await orchestrator.consult('test');

      assert.strictEqual(result.insights.length, 0);
      assert.strictEqual(result.synthesis, null);
      assert.strictEqual(result.enginesConsulted.length, 0);
      assert.strictEqual(result.overallConfidence, 0);
      assert(result.metadata.error.includes('No engines available'));
    });

    it('should return empty result when all engines disabled', async () => {
      registry.register(createMockEngine({
        id: 'disabled-1',
        status: EngineStatus.DISABLED,
      }));

      const result = await orchestrator.consult('test');

      assert.strictEqual(result.insights.length, 0);
      assert(result.metadata.error.includes('No engines available'));
    });

    it('should handle partial engine failures gracefully', async () => {
      registry.register(createMockEngine({ id: 'working' }));
      registry.register(createFailingEngine('failing'));

      const result = await orchestrator.consult('test');

      assert.strictEqual(result.enginesConsulted.length, 1);
      assert.strictEqual(result.enginesConsulted[0], 'working');
      assert.strictEqual(result.metadata.totalEngines, 2);
      assert.strictEqual(result.metadata.successfulEngines, 1);
    });

    it('should handle all engines failing', async () => {
      registry.register(createFailingEngine('fail-1'));
      registry.register(createFailingEngine('fail-2'));

      const result = await orchestrator.consult('test');

      assert.strictEqual(result.insights.length, 0);
      assert.strictEqual(result.enginesConsulted.length, 0);
      assert.strictEqual(result.synthesis, null);
    });

    it('should return single insight without synthesis for one engine', async () => {
      registry.register(createMockEngine({ id: 'single' }));

      const result = await orchestrator.consult('test');

      assert.strictEqual(result.insights.length, 1);
      assert.strictEqual(result.synthesis.engineId, 'single');
    });
  });

  // ─────────────────────────────────────────────────────────────────────────
  // Timeout Handling
  // ─────────────────────────────────────────────────────────────────────────

  describe('Timeout Handling', () => {
    it('should timeout slow engines', async () => {
      registry.register(createMockEngine({ id: 'fast' }));
      registry.register(createSlowEngine('slow', 2000));

      const fastOrchestrator = new EngineOrchestrator(registry, {
        timeout: 100,
      });

      const result = await fastOrchestrator.consult('test');

      // Only fast engine should succeed
      assert.strictEqual(result.enginesConsulted.length, 1);
      assert.strictEqual(result.enginesConsulted[0], 'fast');
    });

    it('should allow override timeout per consultation', async () => {
      registry.register(createSlowEngine('medium', 200));

      const result = await orchestrator.consult('test', {
        timeout: 500, // Enough time
      });

      assert.strictEqual(result.enginesConsulted.length, 1);
    });
  });

  // ─────────────────────────────────────────────────────────────────────────
  // Deliberation
  // ─────────────────────────────────────────────────────────────────────────

  describe('Deliberation', () => {
    beforeEach(() => {
      registry.register(createMockEngine({
        id: 'stoic-1',
        domain: 'ethics',
        tradition: 'stoic',
        confidence: 0.6,
      }));
      registry.register(createMockEngine({
        id: 'kantian-1',
        domain: 'ethics',
        tradition: 'kantian',
        confidence: 0.5,
      }));
      registry.register(createMockEngine({
        id: 'utilitarian-1',
        domain: 'ethics',
        tradition: 'utilitarian',
        confidence: 0.4,
      }));
    });

    it('should collect positions from ethics engines', async () => {
      const result = await orchestrator.deliberate('Is lying ever justified?');

      assert.strictEqual(result.positions.length, 3);
      assert(result.positions.some(p => p.tradition === 'stoic'));
      assert(result.positions.some(p => p.tradition === 'kantian'));
    });

    it('should identify tensions between traditions', async () => {
      const result = await orchestrator.deliberate('test dilemma');

      assert(result.tensions.length > 0);
      assert(result.tensions[0].traditions.length === 2);
      assert(result.tensions[0].between.length === 2);
    });

    it('should filter by specific traditions', async () => {
      const result = await orchestrator.deliberate('test', {
        traditions: ['stoic', 'kantian'],
      });

      assert.strictEqual(result.positions.length, 2);
      assert(result.positions.every(p =>
        p.tradition === 'stoic' || p.tradition === 'kantian'
      ));
    });

    it('should produce dialectic synthesis', async () => {
      const result = await orchestrator.deliberate('test');

      assert(result.recommendation !== null);
      assert(result.recommendation.thesis !== undefined);
      assert(result.recommendation.antithesis !== undefined);
    });

    it('should return empty result when no ethics engines', async () => {
      const emptyRegistry = new EngineRegistry();
      emptyRegistry.register(createMockEngine({ id: 'logic', domain: 'logic' }));
      const emptyOrchestrator = new EngineOrchestrator(emptyRegistry);

      const result = await emptyOrchestrator.deliberate('test');

      assert.strictEqual(result.positions.length, 0);
      assert.strictEqual(result.tensions.length, 0);
      assert.strictEqual(result.recommendation, null);
    });

    it('should include metadata', async () => {
      const result = await orchestrator.deliberate('test');

      assert(result.metadata.enginesConsulted.length > 0);
      assert(result.metadata.tensionCount >= 0);
      assert(result.metadata.evaluatedAt > 0);
    });
  });

  // ─────────────────────────────────────────────────────────────────────────
  // evaluateWith (Single Engine)
  // ─────────────────────────────────────────────────────────────────────────

  describe('evaluateWith', () => {
    it('should evaluate with specific engine', async () => {
      registry.register(createMockEngine({ id: 'specific' }));

      const result = await orchestrator.evaluateWith('specific', 'test input');

      assert.strictEqual(result.engineId, 'specific');
      assert(result.insight.includes('test input'));
    });

    it('should throw for unknown engine', async () => {
      await assert.rejects(
        () => orchestrator.evaluateWith('nonexistent', 'test'),
        /Engine not found: nonexistent/
      );
    });
  });

  // ─────────────────────────────────────────────────────────────────────────
  // Statistics
  // ─────────────────────────────────────────────────────────────────────────

  describe('getStats', () => {
    it('should return orchestrator statistics', () => {
      registry.register(createMockEngine({ id: 'e1' }));
      registry.register(createMockEngine({ id: 'e2' }));

      const stats = orchestrator.getStats();

      assert.strictEqual(stats.registry.totalEngines, 2);
      assert.strictEqual(stats.defaultStrategy, SynthesisStrategy.WEIGHTED_AVERAGE);
      assert.strictEqual(stats.timeout, 5000);
    });
  });

  // ─────────────────────────────────────────────────────────────────────────
  // Factory Function
  // ─────────────────────────────────────────────────────────────────────────

  describe('createOrchestrator', () => {
    it('should create orchestrator with global registry', () => {
      const orch = createOrchestrator();
      assert(orch instanceof EngineOrchestrator);
    });

    it('should accept custom options', () => {
      const orch = createOrchestrator({
        defaultStrategy: SynthesisStrategy.DIALECTIC,
        timeout: 3000,
      });

      const stats = orch.getStats();
      assert.strictEqual(stats.defaultStrategy, SynthesisStrategy.DIALECTIC);
      assert.strictEqual(stats.timeout, 3000);
    });
  });
});

describe('SynthesisStrategy', () => {
  it('should export all strategy constants', () => {
    assert.strictEqual(SynthesisStrategy.WEIGHTED_AVERAGE, 'weighted-average');
    assert.strictEqual(SynthesisStrategy.HIGHEST_CONFIDENCE, 'highest-confidence');
    assert.strictEqual(SynthesisStrategy.CONSENSUS, 'consensus');
    assert.strictEqual(SynthesisStrategy.MULTI_PERSPECTIVE, 'multi-perspective');
    assert.strictEqual(SynthesisStrategy.DIALECTIC, 'dialectic');
  });
});

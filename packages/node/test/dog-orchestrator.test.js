/**
 * CYNIC Dog Orchestrator & DogChain Tests
 *
 * Tests for the 11 Dogs orchestration system and chain execution.
 *
 * "φ distrusts φ" - κυνικός
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  DogOrchestrator,
  DogMode,
  DogModel,
  DOG_CONFIG,
  DogChain,
  DOG_CHAINS,
} from '../src/agents/orchestrator.js';

import { PHI_INV } from '@cynic/core';

// =============================================================================
// DOG_CONFIG Tests
// =============================================================================

describe('DOG_CONFIG', () => {
  it('should define all 11 dogs', () => {
    const dogs = Object.keys(DOG_CONFIG);
    assert.strictEqual(dogs.length, 11);
    assert.ok(dogs.includes('SAGE'));
    assert.ok(dogs.includes('ANALYST'));
    assert.ok(dogs.includes('GUARDIAN'));
    assert.ok(dogs.includes('SCHOLAR'));
    assert.ok(dogs.includes('ARCHITECT'));
    assert.ok(dogs.includes('JANITOR'));
    assert.ok(dogs.includes('SCOUT'));
    assert.ok(dogs.includes('CARTOGRAPHER'));
    assert.ok(dogs.includes('ORACLE'));
    assert.ok(dogs.includes('DEPLOYER'));
    assert.ok(dogs.includes('CYNIC'));
  });

  it('should have blocking dogs: GUARDIAN, DEPLOYER', () => {
    assert.strictEqual(DOG_CONFIG.GUARDIAN.blocking, true);
    assert.strictEqual(DOG_CONFIG.DEPLOYER.blocking, true);
  });

  it('should have non-blocking dogs', () => {
    assert.strictEqual(DOG_CONFIG.SAGE.blocking, false);
    assert.strictEqual(DOG_CONFIG.ANALYST.blocking, false);
    assert.strictEqual(DOG_CONFIG.SCOUT.blocking, false);
  });

  it('should assign correct models', () => {
    assert.strictEqual(DOG_CONFIG.CYNIC.model, DogModel.OPUS);
    assert.strictEqual(DOG_CONFIG.GUARDIAN.model, DogModel.SONNET);
    assert.strictEqual(DOG_CONFIG.SAGE.model, DogModel.HAIKU);
  });
});

// =============================================================================
// DogMode Tests
// =============================================================================

describe('DogMode', () => {
  it('should define execution modes', () => {
    assert.strictEqual(DogMode.PARALLEL, 'parallel');
    assert.strictEqual(DogMode.SEQUENTIAL, 'sequential');
    assert.strictEqual(DogMode.CRITICAL_ONLY, 'critical');
    assert.strictEqual(DogMode.FAST, 'fast');
  });
});

// =============================================================================
// DogOrchestrator Tests
// =============================================================================

describe('DogOrchestrator', () => {
  let orchestrator;

  beforeEach(() => {
    orchestrator = new DogOrchestrator({
      mode: DogMode.PARALLEL,
    });
  });

  describe('constructor', () => {
    it('should initialize with default mode', () => {
      const orch = new DogOrchestrator();
      assert.strictEqual(orch.mode, DogMode.PARALLEL);
    });

    it('should initialize with custom mode', () => {
      const orch = new DogOrchestrator({ mode: DogMode.SEQUENTIAL });
      assert.strictEqual(orch.mode, DogMode.SEQUENTIAL);
    });

    it('should have φ⁻¹ consensus threshold by default', () => {
      assert.ok(Math.abs(orchestrator.consensusThreshold - PHI_INV) < 0.001);
    });

    it('should initialize stats for all dogs', () => {
      const dogStats = orchestrator.stats.dogVotes;
      assert.strictEqual(Object.keys(dogStats).length, 11);
      for (const dog of Object.keys(DOG_CONFIG)) {
        assert.ok(dogStats[dog]);
        assert.strictEqual(dogStats[dog].total, 0);
      }
    });
  });

  describe('getStats', () => {
    it('should return stats object', () => {
      const stats = orchestrator.getStats();
      assert.ok(stats.dogVotes);
      assert.strictEqual(stats.mode, DogMode.PARALLEL);
      assert.ok(Math.abs(stats.consensusThreshold - PHI_INV) < 0.001);
    });
  });
});

// =============================================================================
// DOG_CHAINS Tests
// =============================================================================

describe('DOG_CHAINS', () => {
  it('should define preset chains', () => {
    assert.ok(DOG_CHAINS.SECURITY_REVIEW);
    assert.ok(DOG_CHAINS.ARCHITECTURE);
    assert.ok(DOG_CHAINS.CODE_QUALITY);
    assert.ok(DOG_CHAINS.RESEARCH);
    assert.ok(DOG_CHAINS.DEPLOYMENT);
    assert.ok(DOG_CHAINS.FULL_REVIEW);
  });

  it('SECURITY_REVIEW should include GUARDIAN', () => {
    assert.ok(DOG_CHAINS.SECURITY_REVIEW.includes('GUARDIAN'));
  });

  it('ARCHITECTURE should include SCOUT, ARCHITECT, GUARDIAN', () => {
    const chain = DOG_CHAINS.ARCHITECTURE;
    assert.ok(chain.includes('SCOUT'));
    assert.ok(chain.includes('ARCHITECT'));
    assert.ok(chain.includes('GUARDIAN'));
  });

  it('FULL_REVIEW should end with CYNIC', () => {
    const chain = DOG_CHAINS.FULL_REVIEW;
    assert.strictEqual(chain[chain.length - 1], 'CYNIC');
  });

  it('All chains should contain valid dog names', () => {
    const validDogs = Object.keys(DOG_CONFIG);
    for (const [chainName, chain] of Object.entries(DOG_CHAINS)) {
      for (const dog of chain) {
        assert.ok(validDogs.includes(dog), `${chainName} contains invalid dog: ${dog}`);
      }
    }
  });
});

// =============================================================================
// DogChain Tests
// =============================================================================

describe('DogChain', () => {
  let orchestrator;
  let chain;

  beforeEach(() => {
    orchestrator = new DogOrchestrator();
    chain = new DogChain({
      orchestrator,
      chain: ['SCOUT', 'ANALYST', 'GUARDIAN'],
    });
  });

  describe('constructor', () => {
    it('should use custom chain', () => {
      assert.deepStrictEqual(chain.chain, ['SCOUT', 'ANALYST', 'GUARDIAN']);
    });

    it('should use preset chain', () => {
      const presetChain = new DogChain({
        orchestrator,
        preset: 'SECURITY_REVIEW',
      });
      assert.deepStrictEqual(presetChain.chain, DOG_CHAINS.SECURITY_REVIEW);
    });

    it('should default to SCOUT, ANALYST, GUARDIAN', () => {
      const defaultChain = new DogChain({ orchestrator });
      assert.deepStrictEqual(defaultChain.chain, ['SCOUT', 'ANALYST', 'GUARDIAN']);
    });

    it('should initialize stats', () => {
      assert.strictEqual(chain.stats.runs, 0);
      assert.strictEqual(chain.stats.completed, 0);
      assert.strictEqual(chain.stats.aborted, 0);
    });
  });

  describe('execute', async () => {
    it('should execute chain and return result', async () => {
      const result = await chain.execute({ type: 'test', content: 'test content' });

      assert.ok(result.chainId);
      assert.deepStrictEqual(result.chain, ['SCOUT', 'ANALYST', 'GUARDIAN']);
      assert.ok(Array.isArray(result.results));
      assert.ok(result.timestamp);
    });

    it('should track chain progress', async () => {
      await chain.execute({ type: 'test' });
      const stats = chain.getStats();

      assert.strictEqual(stats.runs, 1);
    });

    it('should accumulate insights across dogs', async () => {
      const result = await chain.execute({ type: 'test' });

      // Each dog should have run
      assert.ok(result.results.length >= 1);
    });

    it('should include chainStep in each result', async () => {
      const result = await chain.execute({ type: 'test' });

      for (let i = 0; i < result.results.length; i++) {
        const r = result.results[i];
        assert.strictEqual(r.chainStep, i + 1);
        assert.strictEqual(r.totalSteps, 3);
      }
    });
  });

  describe('context accumulation', () => {
    it('should pass accumulated context between dogs', async () => {
      let contextAtGuardian = null;

      // Custom chain with context inspection
      const inspectChain = new DogChain({
        orchestrator: new DogOrchestrator({
          spawner: async (params) => {
            if (params.dog === 'GUARDIAN') {
              contextAtGuardian = params.context;
            }
            return { score: 70, verdict: 'WAG', response: 'allow' };
          },
        }),
        chain: ['SCOUT', 'ANALYST', 'GUARDIAN'],
      });

      await inspectChain.execute({ type: 'test' });

      // Guardian should receive context with previous results
      assert.ok(contextAtGuardian);
      assert.strictEqual(contextAtGuardian.chainPosition, 3);
      assert.ok(contextAtGuardian.previousDogs.includes('SCOUT'));
      assert.ok(contextAtGuardian.previousDogs.includes('ANALYST'));
    });
  });

  describe('early exit on block', () => {
    it('should stop chain if Guardian blocks', async () => {
      const blockChain = new DogChain({
        orchestrator: new DogOrchestrator({
          spawner: async (params) => {
            if (params.dog === 'GUARDIAN') {
              return { score: 20, verdict: 'BARK', response: 'block', reason: 'Dangerous operation' };
            }
            return { score: 70, verdict: 'WAG', response: 'allow' };
          },
        }),
        chain: ['SCOUT', 'GUARDIAN', 'CYNIC'],
      });

      const result = await blockChain.execute({ type: 'test' });

      assert.strictEqual(result.completed, false);
      assert.strictEqual(result.abortedBy, 'GUARDIAN');
      assert.ok(result.abortReason);
      // CYNIC should not have run
      assert.ok(!result.results.find(r => r.dog === 'CYNIC'));
    });
  });

  describe('getStats', () => {
    it('should track completed and aborted chains', async () => {
      // Run a successful chain
      await chain.execute({ type: 'test' });

      const stats = chain.getStats();
      assert.strictEqual(stats.runs, 1);
      assert.ok(stats.averageChainLength > 0);
    });
  });
});

// =============================================================================
// Integration Tests
// =============================================================================

describe('DogOrchestrator + DogChain Integration', () => {
  it('should work together for complex workflows', async () => {
    const orchestrator = new DogOrchestrator();
    const securityChain = new DogChain({
      orchestrator,
      preset: 'SECURITY_REVIEW',
    });

    const result = await securityChain.execute({
      type: 'code_change',
      content: 'DELETE FROM users WHERE id = 1',
    });

    assert.ok(result.chainId);
    assert.ok(result.results.length >= 1);
  });

  it('should use different presets for different tasks', async () => {
    const orchestrator = new DogOrchestrator();

    const archResult = await new DogChain({
      orchestrator,
      preset: 'ARCHITECTURE',
    }).execute({ type: 'design' });

    const codeResult = await new DogChain({
      orchestrator,
      preset: 'CODE_QUALITY',
    }).execute({ type: 'review' });

    assert.deepStrictEqual(archResult.chain, DOG_CHAINS.ARCHITECTURE);
    assert.deepStrictEqual(codeResult.chain, DOG_CHAINS.CODE_QUALITY);
  });
});

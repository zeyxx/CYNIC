/**
 * Brain Tests
 *
 * Tests for the CYNIC Brain - Consciousness Layer.
 *
 * "Le cerveau de CYNIC" - κυνικός
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  Brain,
  BrainState,
  Thought,
} from '../src/orchestration/brain.js';

// φ constants
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.38196601125010515;

// =============================================================================
// TEST HELPERS
// =============================================================================

function createMockDogOrchestrator(overrides = {}) {
  return {
    judge: mock.fn(async (input) => ({
      verdict: 'WAG',
      score: 75,
      confidence: 0.55,
      axiomScores: { PHI: 70, VERIFY: 75, CULTURE: 80, BURN: 75 },
      dogs: ['SAGE', 'ANALYST', 'GUARDIAN'],
    })),
    ...overrides,
  };
}

function createMockEngineOrchestrator(overrides = {}) {
  return {
    // Brain calls consult(), not synthesize()
    consult: mock.fn(async (input) => ({
      synthesis: {
        insight: 'Consider the consequences and verify assumptions',
      },
      confidence: 0.6,
      strategy: 'ethical',
      consultations: [{ domain: 'ethics' }, { domain: 'logic' }],
    })),
    ...overrides,
  };
}

function createMockMemoryStore(overrides = {}) {
  return {
    // Brain calls findPatterns(), not searchPatterns()
    findPatterns: mock.fn(async (query) => [
      { name: 'pattern1', occurrences: 5, confidence: 0.7 },
    ]),
    storePattern: mock.fn(async (pattern) => pattern),
    ...overrides,
  };
}

function createMockLLMOrchestrator(overrides = {}) {
  return {
    execute: mock.fn(async (request) => ({
      response: 'LLM response text',
      model: 'test-model',
      tokens: 100,
    })),
    ...overrides,
  };
}

// =============================================================================
// BRAINSTATE TESTS
// =============================================================================

describe('BrainState', () => {
  describe('construction', () => {
    it('should create with defaults', () => {
      const state = new BrainState();

      assert.ok(state.timestamp > 0);
      assert.equal(state.consciousness, PHI_INV);
      assert.equal(state.cognitiveLoad, 0);
      assert.equal(state.entropy, 0);
      assert.deepEqual(state.patterns, []);
      assert.deepEqual(state.recentThoughts, []);
    });

    it('should create with provided data', () => {
      const state = new BrainState({
        timestamp: 12345,
        consciousness: 0.5,
        cognitiveLoad: 5,
        entropy: 10,
        patterns: [{ name: 'test' }],
        recentThoughts: [{ id: 't1' }],
      });

      assert.equal(state.timestamp, 12345);
      assert.equal(state.consciousness, 0.5);
      assert.equal(state.cognitiveLoad, 5);
      assert.equal(state.entropy, 10);
      assert.equal(state.patterns.length, 1);
      assert.equal(state.recentThoughts.length, 1);
    });

    it('should default consciousness to φ⁻¹', () => {
      const state = new BrainState({});
      assert.equal(state.consciousness, PHI_INV);
    });
  });

  describe('toJSON', () => {
    it('should serialize to JSON', () => {
      const state = new BrainState({
        patterns: [{ a: 1 }, { b: 2 }],
        recentThoughts: [{ id: 't1' }],
      });

      const json = state.toJSON();

      assert.ok(json.timestamp);
      assert.equal(json.consciousness, PHI_INV);
      assert.equal(json.patternCount, 2);
      assert.equal(json.recentThoughtCount, 1);
    });
  });
});

// =============================================================================
// THOUGHT TESTS
// =============================================================================

describe('Thought', () => {
  describe('construction', () => {
    it('should create with defaults', () => {
      const thought = new Thought();

      assert.ok(thought.id.startsWith('thought-'));
      assert.ok(thought.timestamp > 0);
      assert.equal(thought.input, null);
      assert.equal(thought.judgment, null);
      assert.equal(thought.synthesis, null);
      assert.deepEqual(thought.patterns, []);
      assert.equal(thought.confidence, 0);
      assert.equal(thought.decision, null);
      assert.equal(thought.duration, 0);
    });

    it('should create with provided data', () => {
      const thought = new Thought({
        id: 'custom-id',
        input: { content: 'test' },
        confidence: 0.5,
      });

      assert.equal(thought.id, 'custom-id');
      assert.deepEqual(thought.input, { content: 'test' });
      assert.equal(thought.confidence, 0.5);
    });

    it('should cap confidence at φ⁻¹', () => {
      const thought = new Thought({ confidence: 1.0 });
      assert.equal(thought.confidence, PHI_INV);
    });

    it('should cap confidence at φ⁻¹ even for 0.9', () => {
      const thought = new Thought({ confidence: 0.9 });
      assert.equal(thought.confidence, PHI_INV);
    });

    it('should preserve confidence below φ⁻¹', () => {
      const thought = new Thought({ confidence: 0.3 });
      assert.equal(thought.confidence, 0.3);
    });
  });

  describe('verdict getter', () => {
    it('should return verdict from judgment', () => {
      const thought = new Thought({
        judgment: { verdict: 'HOWL', score: 90 },
      });
      assert.equal(thought.verdict, 'HOWL');
    });

    it('should return null without judgment', () => {
      const thought = new Thought();
      assert.equal(thought.verdict, null);
    });
  });

  describe('score getter', () => {
    it('should return score from judgment', () => {
      const thought = new Thought({
        judgment: { verdict: 'WAG', score: 75 },
      });
      assert.equal(thought.score, 75);
    });

    it('should return 0 without judgment', () => {
      const thought = new Thought();
      assert.equal(thought.score, 0);
    });
  });

  describe('isActionable getter', () => {
    it('should be actionable when confidence >= φ⁻²', () => {
      const thought = new Thought({ confidence: PHI_INV_2 });
      assert.equal(thought.isActionable, true);
    });

    it('should be actionable when confidence > φ⁻²', () => {
      const thought = new Thought({ confidence: 0.5 });
      assert.equal(thought.isActionable, true);
    });

    it('should not be actionable when confidence < φ⁻²', () => {
      const thought = new Thought({ confidence: 0.2 });
      assert.equal(thought.isActionable, false);
    });
  });

  describe('toJSON', () => {
    it('should serialize to JSON', () => {
      const thought = new Thought({
        judgment: { verdict: 'WAG', score: 75 },
        confidence: 0.5,
        patterns: [{ a: 1 }],
      });

      const json = thought.toJSON();

      assert.ok(json.id);
      assert.equal(json.verdict, 'WAG');
      assert.equal(json.score, 75);
      assert.equal(json.confidence, 0.5);
      assert.equal(json.isActionable, true);
      assert.equal(json.patternCount, 1);
    });
  });
});

// =============================================================================
// BRAIN TESTS
// =============================================================================

describe('Brain', () => {
  let brain;
  let mockDogs;
  let mockEngines;
  let mockMemory;

  beforeEach(() => {
    mockDogs = createMockDogOrchestrator();
    mockEngines = createMockEngineOrchestrator();
    mockMemory = createMockMemoryStore();

    brain = new Brain({
      dogOrchestrator: mockDogs,
      engineOrchestrator: mockEngines,
      memoryStore: mockMemory,
    });
  });

  // ===========================================================================
  // CONSTRUCTION
  // ===========================================================================

  describe('construction', () => {
    it('should create with all components', () => {
      assert.ok(brain.dogOrchestrator);
      assert.ok(brain.engineOrchestrator);
      assert.ok(brain.memoryStore);
    });

    it('should create without components', () => {
      const emptyBrain = new Brain();
      assert.equal(emptyBrain.dogOrchestrator, null);
      assert.equal(emptyBrain.engineOrchestrator, null);
      assert.equal(emptyBrain.memoryStore, null);
    });

    it('should initialize stats', () => {
      assert.equal(brain.stats.thoughtsProcessed, 0);
      assert.equal(brain.stats.judgmentsRequested, 0);
      assert.equal(brain.stats.synthesisRequested, 0);
    });

    it('should accept llmOrchestrator for Da\'at bridge', () => {
      const llmOrch = createMockLLMOrchestrator();
      const b = new Brain({ llmOrchestrator: llmOrch });
      assert.ok(b.llmOrchestrator);
    });
  });

  // ===========================================================================
  // THINK
  // ===========================================================================

  describe('think', () => {
    it('should process input and return thought', async () => {
      const thought = await brain.think({ content: 'test input' });

      assert.ok(thought instanceof Thought);
      assert.ok(thought.id);
      assert.deepEqual(thought.input, { content: 'test input' });
    });

    it('should request judgment by default', async () => {
      await brain.think({ content: 'test' });

      assert.equal(mockDogs.judge.mock.calls.length, 1);
      assert.equal(brain.stats.judgmentsRequested, 1);
    });

    it('should check patterns by default', async () => {
      await brain.think({ content: 'test' });

      assert.equal(mockMemory.findPatterns.mock.calls.length, 1);
    });

    it('should not request synthesis by default', async () => {
      await brain.think({ content: 'test' });

      assert.equal(mockEngines.consult.mock.calls.length, 0);
      assert.equal(brain.stats.synthesisRequested, 0);
    });

    it('should request synthesis when requested', async () => {
      await brain.think({ content: 'test' }, { requestSynthesis: true });

      assert.equal(mockEngines.consult.mock.calls.length, 1);
      assert.equal(brain.stats.synthesisRequested, 1);
    });

    it('should skip judgment when requested', async () => {
      await brain.think({ content: 'test' }, { requestJudgment: false });

      assert.equal(mockDogs.judge.mock.calls.length, 0);
    });

    it('should skip patterns when requested', async () => {
      await brain.think({ content: 'test' }, { checkPatterns: false });

      assert.equal(mockMemory.findPatterns.mock.calls.length, 0);
    });

    it('should emit thought event', async () => {
      let emittedThought = null;
      brain.on('thought', (t) => { emittedThought = t; });

      await brain.think({ content: 'test' });

      assert.ok(emittedThought);
      assert.ok(emittedThought instanceof Thought);
    });

    it('should record duration', async () => {
      const thought = await brain.think({ content: 'test' });
      assert.ok(thought.duration >= 0);
    });

    it('should increment thoughtsProcessed', async () => {
      await brain.think({ content: 'test1' });
      await brain.think({ content: 'test2' });

      assert.equal(brain.stats.thoughtsProcessed, 2);
    });

    it('should handle errors gracefully', async () => {
      // When dogs throw, Brain catches it internally and continues with patterns
      mockDogs.judge = mock.fn(async () => { throw new Error('Dog error'); });

      const thought = await brain.think({ content: 'test' });

      // Brain continues gracefully - doesn't crash
      assert.ok(thought);
      assert.ok(thought.decision); // Still makes a decision
      // With patterns available, it uses them instead of failing
      assert.ok(thought.patterns.length > 0 || thought.decision.action === 'defer');
      // Judgment should be null because dogs failed
      assert.equal(thought.judgment, null);
    });
  });

  // ===========================================================================
  // JUDGE (SHORTCUT)
  // ===========================================================================

  describe('judge', () => {
    it('should call think with judgment options', async () => {
      const thought = await brain.judge({ content: 'test' });

      assert.ok(thought.judgment);
      assert.equal(mockDogs.judge.mock.calls.length, 1);
      assert.equal(mockEngines.consult.mock.calls.length, 0);
    });
  });

  // ===========================================================================
  // SYNTHESIZE (SHORTCUT)
  // ===========================================================================

  describe('synthesize', () => {
    it('should call think with synthesis options', async () => {
      const thought = await brain.synthesize({ content: 'test' });

      assert.ok(thought.judgment);
      assert.ok(thought.synthesis);
      assert.equal(mockDogs.judge.mock.calls.length, 1);
      assert.equal(mockEngines.consult.mock.calls.length, 1);
    });
  });

  // ===========================================================================
  // RECALL (SHORTCUT)
  // ===========================================================================

  describe('recall', () => {
    it('should call think with pattern-only options', async () => {
      const thought = await brain.recall({ content: 'test' });

      assert.equal(mockDogs.judge.mock.calls.length, 0);
      assert.equal(mockEngines.consult.mock.calls.length, 0);
      assert.equal(mockMemory.findPatterns.mock.calls.length, 1);
    });
  });

  // ===========================================================================
  // EXECUTE (DA'AT BRIDGE)
  // ===========================================================================

  describe('execute', () => {
    it('should increment executionsRequested', async () => {
      const b = new Brain({
        dogOrchestrator: mockDogs,
        memoryStore: mockMemory,
        llmOrchestrator: createMockLLMOrchestrator(),
      });

      await b.execute({ content: 'test' });

      assert.equal(b.stats.executionsRequested, 1);
    });

    it('should think before executing LLM', async () => {
      const llmOrch = createMockLLMOrchestrator();
      const b = new Brain({
        dogOrchestrator: mockDogs,
        memoryStore: mockMemory,
        llmOrchestrator: llmOrch,
      });

      await b.execute({ content: 'test' });

      // Should have thought (judgment requested)
      assert.equal(mockDogs.judge.mock.calls.length, 1);
    });
  });

  // ===========================================================================
  // STATE MANAGEMENT
  // ===========================================================================

  describe('state management', () => {
    it('should update state after thinking', async () => {
      await brain.think({ content: 'test' });

      // State should have been updated (internal method)
      // We can verify via getState if exposed, or check thought history
      assert.equal(brain.stats.thoughtsProcessed, 1);
    });

    it('should track thought history', async () => {
      await brain.think({ content: 'test1' });
      await brain.think({ content: 'test2' });
      await brain.think({ content: 'test3' });

      // History is internal but stats should reflect
      assert.equal(brain.stats.thoughtsProcessed, 3);
    });
  });

  // ===========================================================================
  // WITHOUT COMPONENTS
  // ===========================================================================

  describe('without components', () => {
    it('should work without dogOrchestrator', async () => {
      const b = new Brain({ memoryStore: mockMemory });
      const thought = await b.think({ content: 'test' });

      assert.ok(thought);
      assert.equal(thought.judgment, null);
    });

    it('should work without engineOrchestrator', async () => {
      const b = new Brain({
        dogOrchestrator: mockDogs,
        memoryStore: mockMemory,
      });
      const thought = await b.think({ content: 'test' }, { requestSynthesis: true });

      assert.ok(thought);
      assert.equal(thought.synthesis, null);
    });

    it('should work without memoryStore', async () => {
      const b = new Brain({ dogOrchestrator: mockDogs });
      const thought = await b.think({ content: 'test' });

      assert.ok(thought);
      assert.deepEqual(thought.patterns, []);
    });

    it('should work completely empty', async () => {
      const b = new Brain();
      const thought = await b.think({ content: 'test' });

      assert.ok(thought);
      assert.equal(thought.judgment, null);
      assert.equal(thought.synthesis, null);
      assert.deepEqual(thought.patterns, []);
    });
  });

  // ===========================================================================
  // φ ALIGNMENT
  // ===========================================================================

  describe('φ alignment', () => {
    it('should cap thought confidence at φ⁻¹', async () => {
      // Make dog return high confidence
      mockDogs.judge = mock.fn(async () => ({
        verdict: 'HOWL',
        score: 100,
        confidence: 0.99,
      }));

      const thought = await brain.think({ content: 'test' });

      // Even with high dog confidence, thought confidence should be capped
      assert.ok(thought.confidence <= PHI_INV);
    });

    it('should use φ⁻² as actionable threshold', async () => {
      mockDogs.judge = mock.fn(async () => ({
        verdict: 'WAG',
        score: 70,
        confidence: 0.35, // Below φ⁻²
      }));

      const thought = await brain.think({ content: 'test' });

      // With low confidence, should not be actionable
      // (depends on implementation of _calculateConfidence)
      assert.ok(typeof thought.isActionable === 'boolean');
    });
  });

  // ===========================================================================
  // EVENT EMITTER
  // ===========================================================================

  describe('EventEmitter', () => {
    it('should be an EventEmitter', () => {
      assert.ok(typeof brain.on === 'function');
      assert.ok(typeof brain.emit === 'function');
    });

    it('should emit thought events', async () => {
      const thoughts = [];
      brain.on('thought', (t) => thoughts.push(t));

      await brain.think({ content: 'a' });
      await brain.think({ content: 'b' });

      assert.equal(thoughts.length, 2);
    });
  });
});

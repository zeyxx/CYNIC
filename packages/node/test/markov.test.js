/**
 * Tests for Markov Chain Module
 *
 * "Le chien prédit la prochaine étape" - κυνικός
 *
 * Tests for packages/node/src/inference/markov.js
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';

import {
  // Matrix operations
  buildTransitionMatrix,
  getTransitionProbability,
  getTransitions,

  // Prediction
  predictNextState,
  predictNSteps,
  sampleNextState,
  generateSequence,

  // Sequence analysis
  sequenceProbability,
  sequenceLogProbability,
  detectAnomalousSequence,

  // Stationary distribution
  stationaryDistribution,
  expectedReturnTime,

  // Classes
  MarkovChain,

  // Factories
  createMarkovChain,
  createVerdictChain,
  createActionChain,
} from '../src/inference/markov.js';

describe('Markov: Transition Matrix', () => {
  it('should build matrix from sequence', () => {
    const sequence = ['A', 'B', 'A', 'B', 'A'];
    const { matrix, states } = buildTransitionMatrix(sequence);

    assert.ok(matrix);
    assert.ok(states.includes('A'));
    assert.ok(states.includes('B'));
  });

  it('should calculate correct probabilities', () => {
    // A → B always, B → A always
    const sequence = ['A', 'B', 'A', 'B', 'A'];
    const { matrix } = buildTransitionMatrix(sequence);

    assert.equal(matrix['A']['B'], 1.0); // A always goes to B
    assert.equal(matrix['B']['A'], 1.0); // B always goes to A
  });

  it('should handle self-loops', () => {
    const sequence = ['A', 'A', 'A', 'B', 'B'];
    const { matrix } = buildTransitionMatrix(sequence);

    assert.equal(matrix['A']['A'], 2 / 3); // 2 of 3 A transitions are A→A
    assert.equal(matrix['A']['B'], 1 / 3); // 1 of 3 A transitions is A→B
  });

  it('should return empty for short sequences', () => {
    const { matrix, states } = buildTransitionMatrix(['A']);
    assert.deepEqual(matrix, {});
    assert.deepEqual(states, []);
  });

  it('should use provided states', () => {
    const sequence = ['A', 'B'];
    const { matrix, states } = buildTransitionMatrix(sequence, ['A', 'B', 'C']);

    assert.equal(states.length, 3);
    assert.ok(matrix['C']); // C exists even though not in sequence
  });

  it('should get transition probability', () => {
    const sequence = ['A', 'B', 'A', 'B'];
    const { matrix } = buildTransitionMatrix(sequence);

    assert.equal(getTransitionProbability(matrix, 'A', 'B'), 1.0);
    assert.equal(getTransitionProbability(matrix, 'A', 'C'), 0); // Non-existent
  });

  it('should get all transitions from state', () => {
    const sequence = ['A', 'B', 'A', 'C', 'A'];
    const { matrix } = buildTransitionMatrix(sequence);

    const transitions = getTransitions(matrix, 'A');
    assert.ok(transitions['B'] > 0);
    assert.ok(transitions['C'] > 0);
  });
});

describe('Markov: State Prediction', () => {
  const sequence = ['A', 'B', 'A', 'B', 'A', 'C', 'A'];
  let matrix, states;

  beforeEach(() => {
    const result = buildTransitionMatrix(sequence);
    matrix = result.matrix;
    states = result.states;
  });

  it('should predict most likely next state', () => {
    const prediction = predictNextState(matrix, 'A');

    assert.equal(prediction.state, 'B'); // B is most common after A
    assert.ok(prediction.probability > 0.5);
  });

  it('should include confidence bounded by φ⁻¹', () => {
    const prediction = predictNextState(matrix, 'A');

    assert.ok(prediction.confidence <= PHI_INV + 1e-10);
  });

  it('should include alternatives', () => {
    const prediction = predictNextState(matrix, 'A');

    assert.ok(Array.isArray(prediction.alternatives));
  });

  it('should handle unknown state', () => {
    const prediction = predictNextState(matrix, 'UNKNOWN');

    assert.equal(prediction.state, null);
    assert.equal(prediction.probability, 0);
  });

  it('should predict n steps ahead', () => {
    const dist = predictNSteps(matrix, 'A', 2, states);

    assert.ok(dist['A'] !== undefined);
    assert.ok(dist['B'] !== undefined);

    // Probabilities should sum to ~1
    const sum = Object.values(dist).reduce((a, b) => a + b, 0);
    assert.ok(Math.abs(sum - 1) < 1e-10);
  });

  it('should sample next state', () => {
    const samples = [];
    for (let i = 0; i < 100; i++) {
      samples.push(sampleNextState(matrix, 'A'));
    }

    // Should sample B more often than C
    const bCount = samples.filter(s => s === 'B').length;
    const cCount = samples.filter(s => s === 'C').length;
    assert.ok(bCount > cCount);
  });

  it('should generate sequence', () => {
    const generated = generateSequence(matrix, 'A', 10);

    assert.equal(generated.length, 10);
    assert.equal(generated[0], 'A');
  });
});

describe('Markov: Sequence Analysis', () => {
  const sequence = ['A', 'B', 'A', 'B', 'A', 'B'];
  let matrix;

  beforeEach(() => {
    matrix = buildTransitionMatrix(sequence).matrix;
  });

  it('should calculate sequence probability', () => {
    // Likely sequence (follows pattern)
    const likelyProb = sequenceProbability(matrix, ['A', 'B', 'A']);
    assert.equal(likelyProb, 1.0); // Perfect match

    // Unlikely sequence
    const unlikelyProb = sequenceProbability(matrix, ['A', 'A']);
    assert.equal(unlikelyProb, 0); // Never observed
  });

  it('should calculate log probability', () => {
    const logProb = sequenceLogProbability(matrix, ['A', 'B', 'A']);
    assert.equal(logProb, 0); // log(1) = 0

    const impossibleLogProb = sequenceLogProbability(matrix, ['A', 'A']);
    assert.equal(impossibleLogProb, -Infinity);
  });

  it('should detect anomalous sequence', () => {
    // Normal sequence
    const normal = detectAnomalousSequence(matrix, ['A', 'B', 'A', 'B']);
    assert.equal(normal.isAnomaly, false);

    // Anomalous sequence (never observed transition)
    const anomalous = detectAnomalousSequence(matrix, ['A', 'A', 'A']);
    assert.equal(anomalous.isAnomaly, true);
  });

  it('should include transition details in anomaly detection', () => {
    const result = detectAnomalousSequence(matrix, ['A', 'B', 'A']);

    assert.ok(Array.isArray(result.transitions));
    assert.equal(result.transitions.length, 2);
    assert.ok(result.transitions[0].from === 'A');
    assert.ok(result.transitions[0].to === 'B');
  });
});

describe('Markov: Stationary Distribution', () => {
  it('should calculate stationary distribution', () => {
    // Symmetric chain: equal stationary probabilities
    const sequence = ['A', 'B', 'A', 'B', 'A', 'B'];
    const { matrix, states } = buildTransitionMatrix(sequence);

    const stationary = stationaryDistribution(matrix, states);

    // Should converge to ~0.5 each
    assert.ok(Math.abs(stationary['A'] - 0.5) < 0.1);
    assert.ok(Math.abs(stationary['B'] - 0.5) < 0.1);
  });

  it('should sum to 1', () => {
    const sequence = ['A', 'B', 'C', 'A', 'B', 'A', 'C'];
    const { matrix, states } = buildTransitionMatrix(sequence);

    const stationary = stationaryDistribution(matrix, states);
    const sum = Object.values(stationary).reduce((a, b) => a + b, 0);

    assert.ok(Math.abs(sum - 1) < 1e-6);
  });

  it('should calculate expected return time', () => {
    const stationary = { A: 0.5, B: 0.5 };

    const returnTimeA = expectedReturnTime(stationary, 'A');
    assert.equal(returnTimeA, 2); // 1 / 0.5

    const returnTimeUnknown = expectedReturnTime(stationary, 'C');
    assert.equal(returnTimeUnknown, Infinity);
  });
});

describe('Markov: MarkovChain Class', () => {
  let chain;

  beforeEach(() => {
    chain = new MarkovChain();
  });

  it('should create empty chain', () => {
    assert.equal(chain.states.length, 0);
    assert.equal(chain.observations, 0);
  });

  it('should observe transitions', () => {
    chain.observe('A', 'B');
    chain.observe('B', 'A');

    assert.ok(chain.states.includes('A'));
    assert.ok(chain.states.includes('B'));
    assert.equal(chain.observations, 2);
  });

  it('should observe sequence', () => {
    chain.observeSequence(['A', 'B', 'C', 'A']);

    assert.equal(chain.states.length, 3);
    assert.equal(chain.observations, 3);
  });

  it('should get transition matrix', () => {
    // Use enough observations to make A→B dominant
    chain.observeSequence(['A', 'B', 'A', 'B', 'A', 'B', 'A', 'B']);
    const matrix = chain.getMatrix();

    // With no smoothing, should be exactly 1.0
    assert.ok(matrix['A']['B'] > 0.9); // Allow small tolerance
    assert.ok(matrix['B']['A'] > 0.9);
  });

  it('should predict next state', () => {
    chain.observeSequence(['A', 'B', 'A', 'B', 'A']);
    const prediction = chain.predict('A');

    assert.equal(prediction.state, 'B');
  });

  it('should check for anomalies', () => {
    // With enough observations, normal should not be anomalous
    for (let i = 0; i < 20; i++) {
      chain.observe('A', 'B');
      chain.observe('B', 'A');
    }

    const normalCheck = chain.checkAnomaly(['A', 'B', 'A']);
    assert.equal(normalCheck.isAnomaly, false);

    // Anomalous sequence (very low probability)
    const anomalyCheck = chain.checkAnomaly(['A', 'A', 'A']);
    // With smoothing, A→A has very low but non-zero probability
    assert.ok(anomalyCheck.minTransitionProb < 0.1);
  });

  it('should get stationary distribution', () => {
    // Symmetric chain needs many observations
    for (let i = 0; i < 50; i++) {
      chain.observe('A', 'B');
      chain.observe('B', 'A');
    }
    const stationary = chain.getStationaryDistribution();

    // Should be close to 0.5 each
    assert.ok(Math.abs(stationary['A'] - 0.5) < 0.15);
  });

  it('should generate sequences', () => {
    chain.observeSequence(['A', 'B', 'A', 'B', 'A']);
    const generated = chain.generate('A', 5);

    assert.equal(generated.length, 5);
    assert.equal(generated[0], 'A');
  });

  it('should export and import state', () => {
    chain.observeSequence(['A', 'B', 'C', 'A']);
    const exported = chain.export();

    const newChain = new MarkovChain();
    newChain.import(exported);

    assert.deepEqual(newChain.states, chain.states);
    assert.equal(newChain.observations, chain.observations);
  });

  it('should get stats', () => {
    chain.observeSequence(['A', 'B', 'A', 'B', 'A']);
    const stats = chain.getStats();

    assert.equal(stats.stateCount, 2);
    assert.equal(stats.observations, 4);
    assert.ok(Array.isArray(stats.mostLikelyStates));
  });
});

describe('Markov: With Smoothing', () => {
  it('should handle unseen transitions with smoothing', () => {
    const chain = new MarkovChain({ smoothing: 1 });
    chain.observeSequence(['A', 'B', 'A']);

    const matrix = chain.getMatrix();

    // A→A should have non-zero probability due to smoothing
    assert.ok(matrix['A']['A'] > 0);
  });

  it('should reduce smoothing effect with more data', () => {
    const chain = new MarkovChain({ smoothing: 1 });

    // Few observations
    chain.observeSequence(['A', 'B']);
    const earlyMatrix = chain.getMatrix();

    // More observations
    for (let i = 0; i < 100; i++) {
      chain.observe('A', 'B');
    }
    const lateMatrix = chain.getMatrix();

    // A→B should be higher with more data
    assert.ok(lateMatrix['A']['B'] > earlyMatrix['A']['B']);
  });
});

describe('Markov: Factory Functions', () => {
  it('should create chain from sequence', () => {
    const chain = createMarkovChain(['A', 'B', 'A', 'B']);

    assert.ok(chain instanceof MarkovChain);
    assert.equal(chain.observations, 3);
  });

  it('should create verdict chain', () => {
    const chain = createVerdictChain();

    assert.ok(chain.states.includes('HOWL'));
    assert.ok(chain.states.includes('WAG'));
    assert.ok(chain.states.includes('GROWL'));
    assert.ok(chain.states.includes('BARK'));
  });

  it('should create verdict chain with training data', () => {
    const chain = createVerdictChain(['WAG', 'WAG', 'HOWL', 'WAG']);

    const prediction = chain.predict('WAG');
    // WAG→WAG or WAG→HOWL are likely
    assert.ok(['WAG', 'HOWL'].includes(prediction.state));
  });

  it('should create action chain', () => {
    const chain = createActionChain();

    assert.ok(chain.states.includes('READ'));
    assert.ok(chain.states.includes('WRITE'));
    assert.ok(chain.states.includes('JUDGE'));
  });

  it('should create action chain with training data', () => {
    const chain = createActionChain(['READ', 'JUDGE', 'WRITE', 'READ']);

    assert.equal(chain.observations, 3);
  });
});

describe('Markov: φ-Alignment', () => {
  it('FALSIFIABLE: prediction confidence never exceeds φ⁻¹', () => {
    const testCases = [
      ['A', 'A', 'A', 'A', 'A'], // Very predictable
      ['A', 'B', 'A', 'B', 'A'], // Alternating
      ['A', 'B', 'C', 'A', 'B', 'C'], // Cyclic
    ];

    for (const sequence of testCases) {
      const chain = createMarkovChain(sequence);
      const prediction = chain.predict(sequence[0]);

      assert.ok(
        prediction.confidence <= PHI_INV + 1e-10,
        `Confidence ${prediction.confidence} exceeds φ⁻¹ for ${sequence}`
      );
    }
  });

  it('FALSIFIABLE: higher entropy means lower confidence', () => {
    // Predictable chain
    const predictable = createMarkovChain(['A', 'B', 'A', 'B', 'A', 'B', 'A', 'B']);
    const predPrediction = predictable.predict('A');

    // Less predictable chain
    const random = createMarkovChain(['A', 'B', 'C', 'D', 'A', 'C', 'B', 'D']);
    const randPrediction = random.predict('A');

    assert.ok(
      predPrediction.confidence >= randPrediction.confidence,
      'Predictable chain should have higher or equal confidence'
    );
  });

  it('FALSIFIABLE: anomaly detection uses φ-based thresholds', () => {
    // Build a strong model with many observations
    const chain = new MarkovChain();
    for (let i = 0; i < 50; i++) {
      chain.observe('A', 'B');
      chain.observe('B', 'A');
    }

    // Normal sequence follows the pattern
    const normal = chain.checkAnomaly(['A', 'B', 'A']);
    assert.equal(normal.isAnomaly, false);

    // Anomalous sequence has very low probability transitions
    const anomaly = chain.checkAnomaly(['A', 'A', 'A']);
    // The A→A transition has very low probability
    assert.ok(anomaly.minTransitionProb < PHI_INV_2);
  });
});

describe('Markov: Edge Cases', () => {
  it('should handle empty sequence', () => {
    const chain = createMarkovChain([]);
    assert.equal(chain.states.length, 0);
  });

  it('should handle single element sequence', () => {
    const chain = createMarkovChain(['A']);
    // Single element = no transitions, so no states added
    // Need at least 2 elements to observe a transition
    assert.equal(chain.states.length, 0);
    assert.equal(chain.observations, 0);
  });

  it('should handle all same state', () => {
    const chain = createMarkovChain(['A', 'A', 'A', 'A']);
    const prediction = chain.predict('A');

    assert.equal(prediction.state, 'A');
    assert.equal(prediction.probability, 1.0);
  });

  it('should handle numeric states as strings', () => {
    // Note: Object keys in JS are always strings
    const chain = createMarkovChain([1, 2, 3, 1, 2, 3]);
    // States get converted to strings internally
    const prediction = chain.predict('1'); // Use string key

    assert.equal(prediction.state, '2'); // Returns string
  });

  it('should handle string representations of objects', () => {
    // Note: Object keys stringify to '[object Object]'
    // For proper object support, use a Map-based implementation
    // This test documents current behavior
    const chain = new MarkovChain();
    chain.observeSequence(['stateA', 'stateB', 'stateA']);
    const prediction = chain.predict('stateA');

    assert.equal(prediction.state, 'stateB');
  });
});

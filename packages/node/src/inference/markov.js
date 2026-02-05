/**
 * Markov Chains - State transition modeling
 *
 * "Le chien prédit la prochaine étape" - κυνικός
 *
 * Discrete-time Markov chains for sequence prediction,
 * pattern detection, and state transition modeling.
 *
 * @module @cynic/node/inference/markov
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// TRANSITION MATRIX OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Build transition matrix from sequence of observations
 *
 * @param {Array} sequence - Array of state observations
 * @param {Array} [states] - Optional list of all possible states
 * @returns {Object} {matrix, states, counts}
 */
export function buildTransitionMatrix(sequence, states = null) {
  if (!sequence || sequence.length < 2) {
    return { matrix: {}, states: [], counts: {} };
  }

  // Extract unique states if not provided
  const allStates = states || [...new Set(sequence)];
  const stateIndex = new Map(allStates.map((s, i) => [s, i]));

  // Count transitions
  const counts = {};
  for (const state of allStates) {
    counts[state] = {};
    for (const nextState of allStates) {
      counts[state][nextState] = 0;
    }
  }

  // Count observed transitions
  for (let i = 0; i < sequence.length - 1; i++) {
    const from = sequence[i];
    const to = sequence[i + 1];
    if (counts[from] && counts[from][to] !== undefined) {
      counts[from][to]++;
    }
  }

  // Normalize to probabilities
  const matrix = {};
  for (const from of allStates) {
    matrix[from] = {};
    const total = Object.values(counts[from]).reduce((a, b) => a + b, 0);

    for (const to of allStates) {
      if (total > 0) {
        matrix[from][to] = counts[from][to] / total;
      } else {
        // Uniform distribution for unseen states
        matrix[from][to] = 1 / allStates.length;
      }
    }
  }

  return { matrix, states: allStates, counts };
}

/**
 * Get transition probability P(to | from)
 *
 * @param {Object} matrix - Transition matrix
 * @param {*} from - Source state
 * @param {*} to - Target state
 * @returns {number} Transition probability [0, 1]
 */
export function getTransitionProbability(matrix, from, to) {
  if (!matrix || !matrix[from]) return 0;
  return matrix[from][to] || 0;
}

/**
 * Get all transition probabilities from a state
 *
 * @param {Object} matrix - Transition matrix
 * @param {*} from - Source state
 * @returns {Object} Map of state → probability
 */
export function getTransitions(matrix, from) {
  if (!matrix || !matrix[from]) return {};
  return { ...matrix[from] };
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE PREDICTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Predict next state with confidence
 *
 * @param {Object} matrix - Transition matrix
 * @param {*} currentState - Current state
 * @returns {Object} {state, probability, confidence, alternatives}
 */
export function predictNextState(matrix, currentState) {
  const transitions = getTransitions(matrix, currentState);
  const entries = Object.entries(transitions);

  if (entries.length === 0) {
    return {
      state: null,
      probability: 0,
      confidence: 0,
      alternatives: [],
    };
  }

  // Sort by probability
  entries.sort((a, b) => b[1] - a[1]);

  const [bestState, bestProb] = entries[0];

  // Calculate entropy-based confidence
  // Low entropy (clear winner) → high confidence
  // High entropy (uniform) → low confidence
  let entropy = 0;
  for (const [, p] of entries) {
    if (p > 0) {
      entropy -= p * Math.log2(p);
    }
  }
  const maxEntropy = Math.log2(entries.length);
  const normalizedEntropy = maxEntropy > 0 ? entropy / maxEntropy : 0;

  // Confidence = probability × (1 - entropy), capped at φ⁻¹
  const rawConfidence = bestProb * (1 - normalizedEntropy);
  const confidence = Math.min(PHI_INV, rawConfidence);

  return {
    state: bestState,
    probability: bestProb,
    confidence,
    entropy: normalizedEntropy,
    alternatives: entries.slice(1, 4).map(([s, p]) => ({ state: s, probability: p })),
  };
}

/**
 * Predict state after n steps
 *
 * @param {Object} matrix - Transition matrix
 * @param {*} startState - Starting state
 * @param {number} steps - Number of steps ahead
 * @param {Array} states - List of all states
 * @returns {Object} Distribution over states after n steps
 */
export function predictNSteps(matrix, startState, steps, states) {
  if (steps <= 0 || !states || states.length === 0) {
    return { [startState]: 1.0 };
  }

  // Initialize distribution
  let dist = {};
  for (const s of states) {
    dist[s] = s === startState ? 1.0 : 0.0;
  }

  // Matrix multiplication n times
  for (let step = 0; step < steps; step++) {
    const newDist = {};
    for (const s of states) {
      newDist[s] = 0;
      for (const from of states) {
        newDist[s] += dist[from] * (matrix[from]?.[s] || 0);
      }
    }
    dist = newDist;
  }

  return dist;
}

/**
 * Sample next state according to transition probabilities
 *
 * @param {Object} matrix - Transition matrix
 * @param {*} currentState - Current state
 * @returns {*} Sampled next state
 */
export function sampleNextState(matrix, currentState) {
  const transitions = getTransitions(matrix, currentState);
  const entries = Object.entries(transitions);

  if (entries.length === 0) return currentState;

  const r = Math.random();
  let cumulative = 0;

  for (const [state, prob] of entries) {
    cumulative += prob;
    if (r < cumulative) {
      return state;
    }
  }

  // Fallback to last state (handles floating point errors)
  return entries[entries.length - 1][0];
}

/**
 * Generate sequence of n states
 *
 * @param {Object} matrix - Transition matrix
 * @param {*} startState - Starting state
 * @param {number} length - Length of sequence to generate
 * @returns {Array} Generated sequence
 */
export function generateSequence(matrix, startState, length) {
  const sequence = [startState];
  let current = startState;

  for (let i = 1; i < length; i++) {
    current = sampleNextState(matrix, current);
    sequence.push(current);
  }

  return sequence;
}

// ═══════════════════════════════════════════════════════════════════════════
// SEQUENCE ANALYSIS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate probability of observing a sequence
 *
 * @param {Object} matrix - Transition matrix
 * @param {Array} sequence - Sequence of states
 * @returns {number} Probability of sequence (can be very small)
 */
export function sequenceProbability(matrix, sequence) {
  if (!sequence || sequence.length < 2) return 1.0;

  let prob = 1.0;
  for (let i = 0; i < sequence.length - 1; i++) {
    const p = getTransitionProbability(matrix, sequence[i], sequence[i + 1]);
    prob *= p;
    if (prob === 0) break;
  }

  return prob;
}

/**
 * Calculate log probability of sequence (for numerical stability)
 *
 * @param {Object} matrix - Transition matrix
 * @param {Array} sequence - Sequence of states
 * @returns {number} Log probability (negative, closer to 0 = more likely)
 */
export function sequenceLogProbability(matrix, sequence) {
  if (!sequence || sequence.length < 2) return 0;

  let logProb = 0;
  for (let i = 0; i < sequence.length - 1; i++) {
    const p = getTransitionProbability(matrix, sequence[i], sequence[i + 1]);
    if (p === 0) return -Infinity;
    logProb += Math.log(p);
  }

  return logProb;
}

/**
 * Detect anomalous sequence (low probability given model)
 *
 * @param {Object} matrix - Transition matrix
 * @param {Array} sequence - Sequence to check
 * @param {number} [threshold] - Anomaly threshold (default: φ⁻³)
 * @returns {Object} {isAnomaly, score, probability, transitions}
 */
export function detectAnomalousSequence(matrix, sequence, threshold = PHI_INV_3) {
  if (!sequence || sequence.length < 2) {
    return { isAnomaly: false, score: 0, probability: 1, transitions: [] };
  }

  const transitions = [];
  let minProb = 1.0;
  let anomalousCount = 0;

  for (let i = 0; i < sequence.length - 1; i++) {
    const from = sequence[i];
    const to = sequence[i + 1];
    const prob = getTransitionProbability(matrix, from, to);

    transitions.push({ from, to, probability: prob });

    if (prob < threshold) {
      anomalousCount++;
    }
    if (prob < minProb) {
      minProb = prob;
    }
  }

  // Anomaly score = fraction of low-probability transitions
  const score = anomalousCount / (sequence.length - 1);

  return {
    isAnomaly: score > PHI_INV_2 || minProb < threshold * threshold,
    score,
    probability: sequenceProbability(matrix, sequence),
    minTransitionProb: minProb,
    anomalousTransitions: anomalousCount,
    transitions,
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// STATIONARY DISTRIBUTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate stationary distribution (long-run probabilities)
 *
 * Uses power iteration method.
 *
 * @param {Object} matrix - Transition matrix
 * @param {Array} states - List of states
 * @param {number} [maxIter=100] - Maximum iterations
 * @param {number} [tolerance=1e-8] - Convergence tolerance
 * @returns {Object} Stationary distribution {state → probability}
 */
export function stationaryDistribution(matrix, states, maxIter = 100, tolerance = 1e-8) {
  if (!states || states.length === 0) return {};

  const n = states.length;

  // Initialize uniform distribution
  let pi = {};
  for (const s of states) {
    pi[s] = 1 / n;
  }

  // Power iteration
  for (let iter = 0; iter < maxIter; iter++) {
    const newPi = {};
    let maxDiff = 0;

    for (const s of states) {
      newPi[s] = 0;
      for (const from of states) {
        newPi[s] += pi[from] * (matrix[from]?.[s] || 0);
      }
    }

    // Check convergence
    for (const s of states) {
      const diff = Math.abs(newPi[s] - pi[s]);
      if (diff > maxDiff) maxDiff = diff;
    }

    pi = newPi;

    if (maxDiff < tolerance) {
      break;
    }
  }

  return pi;
}

/**
 * Calculate expected return time to a state
 *
 * @param {Object} stationaryDist - Stationary distribution
 * @param {*} state - Target state
 * @returns {number} Expected steps to return (Infinity if not reachable)
 */
export function expectedReturnTime(stationaryDist, state) {
  const pi = stationaryDist[state];
  if (!pi || pi === 0) return Infinity;
  return 1 / pi;
}

// ═══════════════════════════════════════════════════════════════════════════
// MARKOV CHAIN CLASS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Markov Chain class for stateful sequence modeling
 */
export class MarkovChain {
  /**
   * Create a Markov chain
   * @param {Object} [options] - Options
   * @param {Array} [options.states] - Predefined states
   * @param {number} [options.smoothing=0] - Laplace smoothing parameter
   */
  constructor(options = {}) {
    this.states = options.states || [];
    this.smoothing = options.smoothing || 0;
    this.transitionCounts = {};
    this.totalCounts = {};
    this.observations = 0;

    // Initialize if states provided
    if (this.states.length > 0) {
      this._initializeStates(this.states);
    }
  }

  /**
   * Initialize transition counts for states
   * @private
   */
  _initializeStates(states) {
    for (const from of states) {
      if (!this.transitionCounts[from]) {
        this.transitionCounts[from] = {};
        this.totalCounts[from] = 0;
      }
      for (const to of states) {
        if (this.transitionCounts[from][to] === undefined) {
          this.transitionCounts[from][to] = this.smoothing;
          this.totalCounts[from] += this.smoothing;
        }
      }
    }
  }

  /**
   * Add a state to the chain
   * @param {*} state - State to add
   */
  addState(state) {
    if (!this.states.includes(state)) {
      this.states.push(state);

      // Initialize counts for new state
      if (!this.transitionCounts[state]) {
        this.transitionCounts[state] = {};
        this.totalCounts[state] = 0;
      }

      // Add transitions FROM new state TO all existing states (including itself)
      for (const to of this.states) {
        if (this.transitionCounts[state][to] === undefined) {
          this.transitionCounts[state][to] = this.smoothing;
          this.totalCounts[state] += this.smoothing;
        }
      }

      // Add transitions FROM all existing states TO new state
      for (const from of this.states) {
        if (this.transitionCounts[from][state] === undefined) {
          this.transitionCounts[from][state] = this.smoothing;
          this.totalCounts[from] += this.smoothing;
        }
      }
    }
  }

  /**
   * Observe a transition
   * @param {*} from - Source state
   * @param {*} to - Target state
   */
  observe(from, to) {
    // Add states if new
    this.addState(from);
    this.addState(to);

    // Increment count
    this.transitionCounts[from][to]++;
    this.totalCounts[from]++;
    this.observations++;
  }

  /**
   * Observe a sequence of states
   * @param {Array} sequence - Sequence of states
   */
  observeSequence(sequence) {
    if (!sequence || sequence.length < 2) return;

    for (let i = 0; i < sequence.length - 1; i++) {
      this.observe(sequence[i], sequence[i + 1]);
    }
  }

  /**
   * Get transition matrix
   * @returns {Object} Normalized transition matrix
   */
  getMatrix() {
    const matrix = {};

    for (const from of this.states) {
      matrix[from] = {};
      const total = this.totalCounts[from] || 1;

      for (const to of this.states) {
        const count = this.transitionCounts[from]?.[to] || 0;
        matrix[from][to] = count / total;
      }
    }

    return matrix;
  }

  /**
   * Predict next state
   * @param {*} current - Current state
   * @returns {Object} Prediction with confidence
   */
  predict(current) {
    return predictNextState(this.getMatrix(), current);
  }

  /**
   * Check if sequence is anomalous
   * @param {Array} sequence - Sequence to check
   * @returns {Object} Anomaly analysis
   */
  checkAnomaly(sequence) {
    return detectAnomalousSequence(this.getMatrix(), sequence);
  }

  /**
   * Get stationary distribution
   * @returns {Object} Long-run state probabilities
   */
  getStationaryDistribution() {
    return stationaryDistribution(this.getMatrix(), this.states);
  }

  /**
   * Generate a sequence
   * @param {*} start - Starting state
   * @param {number} length - Sequence length
   * @returns {Array} Generated sequence
   */
  generate(start, length) {
    return generateSequence(this.getMatrix(), start, length);
  }

  /**
   * Get statistics
   * @returns {Object} Chain statistics
   */
  getStats() {
    const stationary = this.getStationaryDistribution();
    const mostLikely = Object.entries(stationary)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 3)
      .map(([state, prob]) => ({ state, probability: prob }));

    return {
      stateCount: this.states.length,
      observations: this.observations,
      mostLikelyStates: mostLikely,
    };
  }

  /**
   * Export chain state
   * @returns {Object} Serializable state
   */
  export() {
    return {
      states: [...this.states],
      transitionCounts: JSON.parse(JSON.stringify(this.transitionCounts)),
      totalCounts: { ...this.totalCounts },
      observations: this.observations,
      smoothing: this.smoothing,
    };
  }

  /**
   * Import chain state
   * @param {Object} state - Previously exported state
   */
  import(state) {
    this.states = state.states || [];
    this.transitionCounts = state.transitionCounts || {};
    this.totalCounts = state.totalCounts || {};
    this.observations = state.observations || 0;
    this.smoothing = state.smoothing || 0;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create a Markov chain from a sequence
 *
 * @param {Array} sequence - Training sequence
 * @param {Object} [options] - Chain options
 * @returns {MarkovChain} Trained chain
 */
export function createMarkovChain(sequence, options = {}) {
  const chain = new MarkovChain(options);
  if (sequence && sequence.length > 0) {
    chain.observeSequence(sequence);
  }
  return chain;
}

/**
 * Create a Markov chain for verdict sequences
 * Pre-configured with CYNIC verdict states
 *
 * @param {Array} [sequence] - Optional training sequence
 * @returns {MarkovChain} Verdict chain
 */
export function createVerdictChain(sequence = null) {
  const chain = new MarkovChain({
    states: ['HOWL', 'WAG', 'GROWL', 'BARK'],
    smoothing: 1, // Laplace smoothing for unseen transitions
  });

  if (sequence) {
    chain.observeSequence(sequence);
  }

  return chain;
}

/**
 * Create a Markov chain for action sequences
 * Pre-configured with common action types
 *
 * @param {Array} [sequence] - Optional training sequence
 * @returns {MarkovChain} Action chain
 */
export function createActionChain(sequence = null) {
  const chain = new MarkovChain({
    states: ['READ', 'WRITE', 'EDIT', 'SEARCH', 'EXECUTE', 'JUDGE', 'LEARN'],
    smoothing: 0.5,
  });

  if (sequence) {
    chain.observeSequence(sequence);
  }

  return chain;
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

export default {
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
};

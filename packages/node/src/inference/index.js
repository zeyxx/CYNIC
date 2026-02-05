/**
 * Inference Module
 *
 * Probabilistic reasoning and belief updating for CYNIC.
 *
 * @module @cynic/node/inference
 */

'use strict';

// Bayesian Inference
export {
  // Core Bayes
  bayesTheorem,
  computeMarginal,
  updateBelief,
  batchUpdateBelief,

  // Multi-hypothesis
  Hypothesis,
  HypothesisSet,
  createHypothesisSet,

  // Beta-Binomial
  BetaDistribution,
  createBetaTracker,

  // Naive Bayes
  NaiveBayesClassifier,
  createClassifier,

  // Belief Networks
  BeliefNode,
  BeliefNetwork,
  createBeliefNetwork,

  // Utilities
  likelihoodRatio,
  probabilityToOdds,
  oddsToProbability,
  logOdds,
  sigmoid,
  updateOdds,

  // Config
  BAYES_CONFIG,
} from './bayes.js';

// Poisson Distribution for Rare Events
export {
  // Core functions
  factorial,
  logFactorial,
  poissonPMF,
  poissonCDF,
  poissonSurvival,
  poissonQuantile,
  poissonMean,
  poissonVariance,
  poissonStdDev,

  // Rate estimation
  estimateRate,
  rateConfidenceInterval,

  // Anomaly detection
  detectAnomaly,
  anomalyScore,

  // Process classes
  PoissonProcess,
  EventRateTracker,

  // Utilities
  poissonGoodnessOfFit,
  waitingTimeCDF,
  timeToNEvents,

  // Factories
  createPoissonProcess,
  createEventTracker,

  // Config
  POISSON_CONFIG,
} from './poisson.js';

// Gaussian Distribution for Noise and Statistics
export {
  // Core functions
  gaussianPDF,
  gaussianLogPDF,
  gaussianCDF,
  gaussianSurvival,
  gaussianQuantile,
  erf,
  erfc,

  // Z-scores and standardization
  zScore,
  fromZScore,
  standardize,
  zScoreToPValue,
  pValueToZScore,

  // Confidence intervals
  confidenceInterval,
  phiConfidenceInterval,
  confidenceToZScore,

  // Random generation
  randomStandardNormal,
  randomNormal,
  randomNormalArray,
  randomCorrelatedNormal,

  // Noise
  addNoise,
  addNoiseArray,
  GaussianNoiseGenerator,
  createNoiseGenerator,

  // Kernels and KDE
  gaussianKernel,
  gaussianKernelScaled,
  silvermanBandwidth,
  kernelDensityEstimate,
  kde,

  // Distribution classes
  GaussianDistribution,
  DiagonalGaussian,
  createGaussian,
  standardNormal,

  // Statistics
  computeStats,
  skewness,
  kurtosis,
  jarqueBeraTest,

  // Config
  GAUSSIAN_CONSTANTS,
  GAUSSIAN_CONFIG,
} from './gaussian.js';

// Entropy and Information Theory
export {
  // Shannon entropy
  shannonEntropy,
  normalizedEntropy,

  // Divergences
  crossEntropy,
  klDivergence,
  jsDivergence,

  // Utilities
  scoresToProbabilities,
  entropyConfidence,

  // Tracker
  EntropyTracker,
  getEntropyTracker,
  resetEntropyTracker,
} from './entropy.js';

// Markov Chains for Sequence Modeling
export {
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
} from './markov.js';

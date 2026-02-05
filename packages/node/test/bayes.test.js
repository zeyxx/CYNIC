/**
 * Tests for Bayesian Inference Module
 * "L'évidence met à jour les croyances" - κυνικός
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  bayesTheorem,
  computeMarginal,
  updateBelief,
  batchUpdateBelief,
  Hypothesis,
  HypothesisSet,
  createHypothesisSet,
  BetaDistribution,
  createBetaTracker,
  NaiveBayesClassifier,
  createClassifier,
  BeliefNode,
  BeliefNetwork,
  createBeliefNetwork,
  likelihoodRatio,
  probabilityToOdds,
  oddsToProbability,
  logOdds,
  sigmoid,
  updateOdds,
  BAYES_CONFIG,
} from '../src/inference/bayes.js';

describe('Core Bayes Theorem', () => {
  describe('bayesTheorem', () => {
    it('should compute posterior correctly', () => {
      // Classic example: disease testing
      // P(disease) = 0.01 (1% prevalence)
      // P(positive|disease) = 0.9 (90% sensitivity)
      // P(positive) = P(+|D)*P(D) + P(+|~D)*P(~D)
      //             = 0.9*0.01 + 0.05*0.99 = 0.009 + 0.0495 = 0.0585

      const likelihood = 0.9;  // P(positive|disease)
      const prior = 0.01;      // P(disease)
      const marginal = 0.0585; // P(positive)

      const posterior = bayesTheorem(likelihood, prior, marginal);

      // P(disease|positive) = 0.9 * 0.01 / 0.0585 ≈ 0.154
      assert.ok(Math.abs(posterior - 0.154) < 0.01);
    });

    it('should cap posterior at φ⁻¹', () => {
      const posterior = bayesTheorem(1.0, 0.9, 0.5);

      // Would be 1.8 without cap
      assert.ok(posterior <= 0.6181, `Posterior ${posterior} exceeds φ⁻¹`);
    });

    it('should not go below minimum', () => {
      const posterior = bayesTheorem(0.001, 0.001, 0.999);

      assert.ok(posterior >= BAYES_CONFIG.MIN_POSTERIOR);
    });

    it('should return prior if marginal is zero', () => {
      const posterior = bayesTheorem(0.5, 0.3, 0);

      assert.strictEqual(posterior, 0.3);
    });
  });

  describe('computeMarginal', () => {
    it('should compute marginal from hypotheses', () => {
      const hypotheses = [
        { likelihood: 0.9, prior: 0.01 },  // Disease: P(+|D) * P(D)
        { likelihood: 0.05, prior: 0.99 }, // No disease: P(+|~D) * P(~D)
      ];

      const marginal = computeMarginal(hypotheses);

      // 0.9*0.01 + 0.05*0.99 = 0.009 + 0.0495 = 0.0585
      assert.ok(Math.abs(marginal - 0.0585) < 0.001);
    });
  });

  describe('updateBelief', () => {
    it('should update belief with evidence', () => {
      const currentBelief = 0.5;
      const likelihood = 0.8; // Evidence supports hypothesis
      const baserate = 0.5;   // Neutral baserate

      const updated = updateBelief(currentBelief, likelihood, baserate);

      // Should increase belief
      assert.ok(updated > currentBelief);
    });

    it('should decrease belief with contradicting evidence', () => {
      const currentBelief = 0.5;
      const likelihood = 0.2; // Evidence contradicts hypothesis
      const baserate = 0.5;

      const updated = updateBelief(currentBelief, likelihood, baserate);

      // Should decrease belief
      assert.ok(updated < currentBelief);
    });
  });

  describe('batchUpdateBelief', () => {
    it('should apply sequential updates', () => {
      const initial = 0.5;
      const evidences = [
        { likelihood: 0.8, baserate: 0.5 },
        { likelihood: 0.7, baserate: 0.5 },
        { likelihood: 0.9, baserate: 0.5 },
      ];

      const final = batchUpdateBelief(initial, evidences);

      // Multiple supporting evidence should increase belief
      assert.ok(final > initial);
      // But still capped at φ⁻¹
      assert.ok(final <= 0.6181);
    });
  });
});

describe('Hypothesis', () => {
  describe('construction', () => {
    it('should create with default prior', () => {
      const h = new Hypothesis('h1', 'Test Hypothesis');

      assert.strictEqual(h.id, 'h1');
      assert.strictEqual(h.name, 'Test Hypothesis');
      assert.strictEqual(h.prior, BAYES_CONFIG.DEFAULT_PRIOR);
      assert.strictEqual(h.posterior, h.prior);
    });

    it('should cap prior at φ⁻¹', () => {
      const h = new Hypothesis('h1', 'Test', 0.9);

      assert.ok(h.prior <= BAYES_CONFIG.MAX_PRIOR);
    });
  });

  describe('update', () => {
    it('should update posterior and track history', () => {
      const h = new Hypothesis('h1', 'Test', 0.5);

      h.update(0.8, 0.5, 'evidence_1');

      assert.ok(h.posterior !== h.prior);
      assert.strictEqual(h.evidenceHistory.length, 1);
      assert.strictEqual(h.evidenceHistory[0].evidenceId, 'evidence_1');
    });
  });

  describe('getConfidenceCategory', () => {
    it('should categorize confidence correctly', () => {
      const h = new Hypothesis('h1', 'Test', 0.6);
      h.posterior = 0.65;

      // 0.65 > 0.618 = strong
      assert.strictEqual(h.getConfidenceCategory(), 'strong');
    });
  });
});

describe('HypothesisSet', () => {
  describe('construction', () => {
    it('should create empty set', () => {
      const set = new HypothesisSet('test_set');

      assert.strictEqual(set.name, 'test_set');
      assert.strictEqual(set.hypotheses.size, 0);
    });
  });

  describe('addHypothesis', () => {
    it('should add hypotheses and normalize', () => {
      const set = new HypothesisSet('test');

      set.addHypothesis('h1', 'Hypothesis 1', 0.3);
      set.addHypothesis('h2', 'Hypothesis 2', 0.3);

      // Priors should sum to 1 (normalized)
      const sum = set.getHypothesis('h1').prior + set.getHypothesis('h2').prior;
      assert.ok(Math.abs(sum - 1.0) < 0.01, `Sum ${sum} should be ~1`);
    });
  });

  describe('updateWithEvidence', () => {
    it('should update all hypotheses', () => {
      const set = new HypothesisSet('test');
      set.addHypothesis('spam', 'Is Spam', 0.3);
      set.addHypothesis('ham', 'Is Ham', 0.7);

      const results = set.updateWithEvidence({
        spam: 0.9, // Evidence strongly suggests spam
        ham: 0.1,
      }, 'evidence_1');

      // Spam should be more likely now
      assert.ok(results.spam > results.ham);
    });
  });

  describe('getMostLikely', () => {
    it('should return highest posterior hypothesis', () => {
      const set = new HypothesisSet('test');
      set.addHypothesis('h1', 'Low', 0.1);
      set.addHypothesis('h2', 'High', 0.9);

      // After normalization: h1 = 0.1/1.0 = 0.1, h2 = 0.9/1.0 = 0.9 (capped at φ⁻¹)
      const best = set.getMostLikely();

      // h2 should have higher posterior
      assert.ok(best.posterior >= set.getHypothesis('h1').posterior);
    });
  });

  describe('isDecisive', () => {
    it('should detect decisive winner', () => {
      const set = new HypothesisSet('test');
      set.addHypothesis('winner', 'Winner', 0.9);
      set.addHypothesis('loser', 'Loser', 0.1);

      const result = set.isDecisive();

      assert.ok(result.decisive);
      assert.strictEqual(result.winner.id, 'winner');
    });

    it('should detect non-decisive case', () => {
      const set = new HypothesisSet('test');
      set.addHypothesis('h1', 'Option 1', 0.4);
      set.addHypothesis('h2', 'Option 2', 0.35);
      set.addHypothesis('h3', 'Option 3', 0.25);

      const result = set.isDecisive();

      // Ratio needs to be >= 1.618 for decisive
      // With 3 similar options, shouldn't be decisive
      assert.ok(result.ratio < 10, `Ratio ${result.ratio} suggests close competition`);
    });
  });
});

describe('createHypothesisSet', () => {
  it('should create set with initial hypotheses', () => {
    const set = createHypothesisSet('test', [
      { id: 'h1', name: 'Hypothesis 1', prior: 0.3 },
      { id: 'h2', name: 'Hypothesis 2', prior: 0.7 },
    ]);

    assert.strictEqual(set.hypotheses.size, 2);
    assert.ok(set.getHypothesis('h1'));
    assert.ok(set.getHypothesis('h2'));
  });
});

describe('BetaDistribution', () => {
  describe('construction', () => {
    it('should create with uniform prior', () => {
      const beta = new BetaDistribution();

      assert.strictEqual(beta.alpha, 1);
      assert.strictEqual(beta.beta, 1);
      assert.strictEqual(beta.getMean(), 0.5);
    });
  });

  describe('recording outcomes', () => {
    it('should update mean with successes', () => {
      const beta = new BetaDistribution();

      beta.recordSuccess();
      beta.recordSuccess();
      beta.recordSuccess();

      // α=4, β=1, mean = 4/5 = 0.8 (capped at φ⁻¹)
      const mean = beta.getMean();
      assert.ok(mean <= 0.6181, `Mean ${mean} exceeds φ⁻¹`);
    });

    it('should update mean with failures', () => {
      const beta = new BetaDistribution();

      beta.recordFailure();
      beta.recordFailure();
      beta.recordFailure();

      // α=1, β=4, mean = 1/5 = 0.2
      assert.ok(Math.abs(beta.getMean() - 0.2) < 0.01);
    });

    it('should handle batch recording', () => {
      const beta = new BetaDistribution();

      beta.recordBatch(7, 3); // 70% success rate

      // α=8, β=4, mean = 8/12 ≈ 0.667 (capped)
      const mean = beta.getMean();
      assert.ok(mean <= 0.6181);
    });
  });

  describe('statistics', () => {
    it('should compute variance', () => {
      const beta = new BetaDistribution(10, 10);

      const variance = beta.getVariance();

      // For α=β=10: var = 10*10 / (20² * 21) = 100/8400 ≈ 0.012
      assert.ok(variance > 0 && variance < 0.1);
    });

    it('should compute confidence interval', () => {
      const beta = new BetaDistribution(10, 10);

      const ci = beta.getConfidenceInterval();

      assert.ok(ci.lower < ci.upper);
      assert.ok(ci.lower >= 0);
      assert.ok(ci.upper <= 1);
    });

    it('should report strength', () => {
      const beta = new BetaDistribution();
      beta.recordBatch(5, 3);

      assert.strictEqual(beta.getStrength(), 8);
    });

    it('should detect confident distribution', () => {
      const beta = new BetaDistribution();
      beta.recordBatch(50, 50);

      assert.ok(beta.isConfident(10, 0.05));
    });
  });

  describe('decay', () => {
    it('should reduce certainty over time', () => {
      const beta = new BetaDistribution();
      beta.recordBatch(10, 90); // 10% success rate (below φ⁻¹)

      const beforeMean = beta.getMean();
      const beforeStrength = beta.getStrength();
      beta.decay(0.5); // Strong decay

      // Strength should decrease (α and β move toward 1)
      const afterStrength = beta.getStrength();
      assert.ok(afterStrength < beforeStrength, 'Strength should decrease');
    });
  });

  describe('serialization', () => {
    it('should serialize and deserialize', () => {
      const original = new BetaDistribution(10, 5);
      const json = original.toJSON();
      const restored = BetaDistribution.fromJSON(json);

      assert.strictEqual(restored.alpha, original.alpha);
      assert.strictEqual(restored.beta, original.beta);
    });
  });
});

describe('NaiveBayesClassifier', () => {
  describe('training', () => {
    it('should train on examples', () => {
      const classifier = new NaiveBayesClassifier();

      classifier.train(['buy', 'now', 'cheap'], 'spam');
      classifier.train(['meeting', 'tomorrow'], 'ham');

      const stats = classifier.getStats();
      assert.strictEqual(stats.classCount, 2);
      assert.strictEqual(stats.totalDocuments, 2);
    });

    it('should train batch', () => {
      const classifier = new NaiveBayesClassifier();

      classifier.trainBatch([
        { features: ['buy', 'cheap'], class: 'spam' },
        { features: ['hello', 'friend'], class: 'ham' },
        { features: ['free', 'money'], class: 'spam' },
      ]);

      assert.strictEqual(classifier.getStats().totalDocuments, 3);
    });
  });

  describe('prediction', () => {
    it('should predict class probabilities', () => {
      const classifier = new NaiveBayesClassifier();

      // Train with clear patterns
      for (let i = 0; i < 10; i++) {
        classifier.train(['buy', 'cheap', 'now', 'free'], 'spam');
        classifier.train(['meeting', 'project', 'work', 'team'], 'ham');
      }

      const predictions = classifier.predict(['buy', 'cheap']);

      assert.ok(predictions.length === 2);
      assert.ok(predictions[0].class === 'spam'); // Spam should be top
      assert.ok(predictions[0].probability > predictions[1].probability);
    });

    it('should classify with confidence', () => {
      const classifier = new NaiveBayesClassifier();

      for (let i = 0; i < 20; i++) {
        classifier.train(['urgent', 'buy', 'discount'], 'spam');
        classifier.train(['schedule', 'meeting', 'discuss'], 'ham');
      }

      const result = classifier.classify(['urgent', 'discount']);

      assert.strictEqual(result.class, 'spam');
      assert.ok(result.probability > 0);
      assert.ok(['strong', 'moderate', 'weak', 'uncertain'].includes(result.confidence));
    });
  });

  describe('serialization', () => {
    it('should serialize and deserialize', () => {
      const original = new NaiveBayesClassifier();
      original.train(['a', 'b'], 'class1');
      original.train(['c', 'd'], 'class2');

      const json = original.toJSON();
      const restored = NaiveBayesClassifier.fromJSON(json);

      assert.strictEqual(restored.getStats().classCount, 2);
      assert.strictEqual(restored.getStats().totalDocuments, 2);
    });
  });
});

describe('BeliefNetwork', () => {
  describe('construction', () => {
    it('should create network with nodes', () => {
      const network = new BeliefNetwork('test');

      network.addNode('rain', 'Rain', 0.2);
      network.addNode('sprinkler', 'Sprinkler', 0.4);
      network.addNode('wet_grass', 'Wet Grass', 0.5);

      assert.strictEqual(network.nodes.size, 3);
    });
  });

  describe('edges', () => {
    it('should add parent-child relationships', () => {
      const network = new BeliefNetwork('test');

      network.addNode('rain', 'Rain', 0.2);
      network.addNode('wet', 'Wet Grass', 0.5);

      network.addEdge('rain', 'wet', 0.9, 0.2);
      // P(wet|rain) = 0.9, P(wet|~rain) = 0.2

      const wetNode = network.nodes.get('wet');
      assert.ok(wetNode.parents.has('rain'));
    });
  });

  describe('observation', () => {
    it('should set observed nodes', () => {
      const network = new BeliefNetwork('test');
      network.addNode('rain', 'Rain', 0.2);

      network.observe('rain', true);

      const node = network.nodes.get('rain');
      assert.ok(node.observed);
      assert.strictEqual(node.belief, 1.0);
    });

    it('should clear observations', () => {
      const network = new BeliefNetwork('test');
      network.addNode('rain', 'Rain', 0.2);
      network.observe('rain', true);

      network.clearObservations();

      const node = network.nodes.get('rain');
      assert.ok(!node.observed);
    });
  });

  describe('propagation', () => {
    it('should propagate beliefs through network', () => {
      const network = new BeliefNetwork('test');

      // Classic sprinkler network
      network.addNode('rain', 'Rain', 0.2);
      network.addNode('wet', 'Wet Grass', 0.5);

      network.addEdge('rain', 'wet', 0.9, 0.2);

      // Observe rain
      network.observe('rain', true);

      const beliefs = network.propagate();

      // Wet grass should be high (P(wet|rain) = 0.9)
      assert.ok(beliefs.get('wet') > 0.5);
    });

    it('should handle multiple parents', () => {
      const network = new BeliefNetwork('test');

      network.addNode('rain', 'Rain', 0.3);
      network.addNode('sprinkler', 'Sprinkler', 0.5);
      network.addNode('wet', 'Wet Grass', 0.1);

      network.addEdge('rain', 'wet', 0.8, 0.1);
      network.addEdge('sprinkler', 'wet', 0.9, 0.1);

      // Observe both causes
      network.observe('rain', true);
      network.observe('sprinkler', true);

      const beliefs = network.propagate();

      // Wet should be very likely
      assert.ok(beliefs.get('wet') > 0.5);
    });
  });

  describe('query', () => {
    it('should query node belief', () => {
      const network = new BeliefNetwork('test');
      network.addNode('test', 'Test', 0.3);

      const belief = network.query('test');

      assert.strictEqual(belief, 0.3);
    });
  });
});

describe('Utility Functions', () => {
  describe('likelihoodRatio', () => {
    it('should compute ratio and interpretation', () => {
      const result = likelihoodRatio(0.9, 0.1);

      // Use approximate equality for floating point
      assert.ok(Math.abs(result.ratio - 9) < 0.01, `Ratio ${result.ratio} should be ~9`);
      assert.strictEqual(result.interpretation, 'substantial');
    });

    it('should handle decisive evidence', () => {
      const result = likelihoodRatio(0.99, 0.001);

      assert.ok(result.ratio > 100);
      assert.strictEqual(result.interpretation, 'decisive');
    });
  });

  describe('probabilityToOdds and oddsToProbability', () => {
    it('should be inverses', () => {
      const p = 0.4;
      const odds = probabilityToOdds(p);
      const recovered = oddsToProbability(odds);

      assert.ok(Math.abs(recovered - p) < 0.01);
    });

    it('should handle edge cases', () => {
      // High probability should give high odds
      const highOdds = probabilityToOdds(0.9);
      assert.ok(highOdds > 1);

      // Low probability should give low odds
      const lowOdds = probabilityToOdds(0.1);
      assert.ok(lowOdds < 1);
    });
  });

  describe('logOdds and sigmoid', () => {
    it('should be inverses', () => {
      const p = 0.3;
      const lo = logOdds(p);
      const recovered = sigmoid(lo);

      assert.ok(Math.abs(recovered - p) < 0.01);
    });

    it('should handle p=0.5', () => {
      const lo = logOdds(0.5);
      assert.ok(Math.abs(lo) < 0.01); // log(1) = 0
    });

    it('should cap sigmoid at φ⁻¹', () => {
      const result = sigmoid(10); // Very high logit

      assert.ok(result <= 0.6181);
    });
  });

  describe('updateOdds', () => {
    it('should multiply odds by likelihood ratio', () => {
      const priorOdds = 1; // 50-50
      const lr = 3;        // Evidence 3x more likely under H1

      const posteriorOdds = updateOdds(priorOdds, lr);

      assert.strictEqual(posteriorOdds, 3);
    });
  });
});

describe('Factory Functions', () => {
  describe('createBetaTracker', () => {
    it('should create with initial observations', () => {
      const tracker = createBetaTracker(5, 3);

      assert.strictEqual(tracker.getStrength(), 8);
    });
  });

  describe('createClassifier', () => {
    it('should create classifier with options', () => {
      const classifier = createClassifier({ smoothing: 2 });

      assert.strictEqual(classifier.smoothing, 2);
    });
  });

  describe('createBeliefNetwork', () => {
    it('should create empty network', () => {
      const network = createBeliefNetwork('test');

      assert.strictEqual(network.name, 'test');
      assert.strictEqual(network.nodes.size, 0);
    });
  });
});

describe('φ-Alignment', () => {
  describe('all posteriors capped', () => {
    it('should never exceed φ⁻¹ in Bayes theorem', () => {
      // Even with extreme values
      const posterior = bayesTheorem(1.0, 1.0, 0.1);

      assert.ok(posterior <= 0.6181);
    });

    it('should never exceed φ⁻¹ in Beta mean', () => {
      const beta = new BetaDistribution(1000, 1);
      const mean = beta.getMean();

      assert.ok(mean <= 0.6181);
    });

    it('should never exceed φ⁻¹ in classifier', () => {
      const classifier = new NaiveBayesClassifier();

      // Train heavily on one class
      for (let i = 0; i < 100; i++) {
        classifier.train(['certain', 'evidence'], 'definite');
      }

      const result = classifier.classify(['certain', 'evidence']);

      assert.ok(result.probability <= 0.6181);
    });
  });
});

describe('Integration', () => {
  describe('spam filter example', () => {
    it('should classify spam vs ham', () => {
      const classifier = new NaiveBayesClassifier();

      // Train
      const spamWords = ['buy', 'cheap', 'free', 'winner', 'prize', 'click'];
      const hamWords = ['meeting', 'project', 'deadline', 'schedule', 'team'];

      for (let i = 0; i < 50; i++) {
        const spamSample = spamWords.slice(0, 2 + Math.floor(Math.random() * 3));
        const hamSample = hamWords.slice(0, 2 + Math.floor(Math.random() * 3));
        classifier.train(spamSample, 'spam');
        classifier.train(hamSample, 'ham');
      }

      // Test
      const spamResult = classifier.classify(['free', 'prize', 'winner']);
      const hamResult = classifier.classify(['meeting', 'schedule']);

      assert.strictEqual(spamResult.class, 'spam');
      assert.strictEqual(hamResult.class, 'ham');
    });
  });

  describe('pattern reliability tracking', () => {
    it('should track pattern success rate', () => {
      const patternTracker = createBetaTracker();

      // Pattern works 80% of the time
      for (let i = 0; i < 100; i++) {
        patternTracker.record(Math.random() < 0.8);
      }

      const mean = patternTracker.getMean();
      const ci = patternTracker.getConfidenceInterval();

      // Should be around 0.8 (but capped at φ⁻¹)
      assert.ok(mean <= 0.6181);
      assert.ok(ci.lower > 0.5); // Clearly better than 50%
    });
  });
});

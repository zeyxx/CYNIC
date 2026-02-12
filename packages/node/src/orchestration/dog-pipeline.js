/**
 * Dog Pipeline - Parallel Dog Voting Execution
 *
 * Executes N Dogs in parallel for judgment consensus.
 * Unlike sequential Tree of Life routing, this is democratic voting.
 *
 * La meute vote ensemble - κυνικός
 *
 * @module @cynic/node/orchestration/dog-pipeline
 */

'use strict';

import { PHI_INV, createLogger, globalEventBus, EventType } from '@cynic/core';

const log = createLogger('DogPipeline');

/**
 * DogPipeline - Parallel Dog execution for voting
 */
export class DogPipeline {
  constructor(options = {}) {
    this.collectivePack = options.collectivePack;
    this.eventBus = options.eventBus || globalEventBus;
    this.timeout = options.timeout || 30000;
    this.minQuorum = options.minQuorum || 3;
    this.stats = {
      votesExecuted: 0,
      dogsInvoked: 0,
      timeouts: 0,
      quorumFailures: 0,
      avgConsensusTime: 0,
    };
  }

  async executeParallel(dogNames, item, context = {}) {
    const startTime = Date.now();
    this.stats.votesExecuted++;

    if (dogNames.length < this.minQuorum) {
      this.stats.quorumFailures++;
      throw new Error(`Quorum not met: ${dogNames.length} Dogs < ${this.minQuorum} required`);
    }

    log.debug('Starting parallel Dog execution', { dogs: dogNames, timeout: this.timeout });

    this.eventBus?.publish(EventType.DOG_VOTE_START, {
      dogs: dogNames,
      mode: 'parallel',
      timestamp: Date.now(),
    }, { source: 'DogPipeline' });

    const votePromises = dogNames.map(dogName =>
      this._executeDogWithTimeout(dogName, item, context)
    );

    const results = await Promise.allSettled(votePromises);
    const votes = [];
    let successCount = 0;
    let timeoutCount = 0;

    for (let i = 0; i < results.length; i++) {
      const result = results[i];
      const dogName = dogNames[i];

      if (result.status === 'fulfilled') {
        votes.push(result.value);
        if (result.value.success) successCount++;
      } else {
        const error = result.reason?.message || 'Unknown error';
        const isTimeout = error.includes('timeout');
        if (isTimeout) { timeoutCount++; this.stats.timeouts++; }
        votes.push({ dog: dogName, success: false, error, isTimeout, score: null, confidence: 0 });
      }

      this.eventBus?.publish(EventType.DOG_VOTE_CAST, {
        dogId: dogName.toLowerCase(),
        score: votes[i].score,
        confidence: votes[i].confidence,
        success: votes[i].success,
      }, { source: 'DogPipeline' });
    }

    if (successCount < this.minQuorum) {
      this.stats.quorumFailures++;
      throw new Error(`Quorum not met after execution: ${successCount} < ${this.minQuorum}`);
    }

    const consensus = this._calculateConsensus(votes.filter(v => v.success));
    const durationMs = Date.now() - startTime;
    this.stats.avgConsensusTime = (this.stats.avgConsensusTime * (this.stats.votesExecuted - 1) + durationMs) / this.stats.votesExecuted;

    this.eventBus?.publish(EventType.CONSENSUS_REACHED, {
      score: consensus.score,
      verdict: consensus.verdict,
      agreementRatio: consensus.agreementRatio,
      confidence: consensus.confidence,
    }, { source: 'DogPipeline' });

    log.debug('Parallel Dog execution complete', {
      successCount,
      timeoutCount,
      verdict: consensus.verdict,
      durationMs,
    });

    return { votes, consensus, successCount, timeoutCount, durationMs };
  }

  async _executeDogWithTimeout(dogName, item, context) {
    this.stats.dogsInvoked++;
    const timeoutPromise = new Promise((_, reject) =>
      setTimeout(() => reject(new Error(`Dog ${dogName} timeout after ${this.timeout}ms`)), this.timeout)
    );
    const executionPromise = this._invokeDog(dogName, item, context);
    const result = await Promise.race([executionPromise, timeoutPromise]);
    return { dog: dogName, ...result, success: true };
  }

  async _invokeDog(dogName, item, context) {
    const dog = this.collectivePack.agents?.[dogName] || this.collectivePack[dogName];
    if (!dog) throw new Error(`Dog ${dogName} not found in CollectivePack`);
    if (typeof dog.voteOnConsensus !== 'function') {
      throw new Error(`Dog ${dogName} does not implement voteOnConsensus method`);
    }
    const voteResult = await dog.voteOnConsensus(item, context);
    return {
      score: voteResult.score,
      confidence: voteResult.confidence,
      response: voteResult.response,
      reasoning: voteResult.reasoning,
    };
  }

  _calculateConsensus(votes) {
    if (votes.length === 0) {
      return { score: 0, confidence: 0, verdict: 'ABSTAIN', agreementRatio: 0, votes: [] };
    }
    const scores = votes.map(v => v.score).filter(s => s != null);
    const confidences = votes.map(v => v.confidence).filter(c => c != null && c > 0);
    if (scores.length === 0) {
      return { score: 0, confidence: 0, verdict: 'ABSTAIN', agreementRatio: 0, votes };
    }
    const geometricMean = Math.exp(
      confidences.reduce((sum, c) => sum + Math.log(Math.max(c, 0.01)), 0) / confidences.length
    );
    const confidence = Math.min(geometricMean, PHI_INV);
    const avgScore = scores.reduce((sum, s) => sum + s, 0) / scores.length;
    const approveCount = scores.filter(s => s > 0.5).length;
    const rejectCount = scores.filter(s => s < 0.5).length;
    const abstainCount = scores.filter(s => s === 0.5).length;
    const agreementRatio = Math.max(approveCount, rejectCount, abstainCount) / scores.length;
    let verdict;
    if (avgScore > 0.6) verdict = 'APPROVE';
    else if (avgScore < 0.4) verdict = 'REJECT';
    else verdict = 'ABSTAIN';
    return { score: avgScore, confidence, verdict, agreementRatio, votes, approveCount, rejectCount, abstainCount };
  }

  getStats() { return { ...this.stats }; }

  async recordVotes(votes, consensus, item) {
    try {
      const { getPool } = await import('@cynic/persistence');
      const pool = getPool();
      for (const vote of votes) {
        if (!vote.success) continue;
        await pool.query(`
          INSERT INTO dog_votes (
            dog_name, item_type, item_id, vote_score, confidence, reasoning, metadata
          ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        `, [
          vote.dog,
          item?.type || 'unknown',
          item?.id || null,
          vote.score,
          vote.confidence,
          vote.reasoning || vote.response,
          JSON.stringify({ consensus: consensus.verdict })
        ]);
      }
    } catch (err) {
      log.debug('Failed to record Dog votes (non-blocking)', { error: err.message });
    }
  }
}

export function createDogPipeline(options) {
  return new DogPipeline(options);
}

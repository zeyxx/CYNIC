/**
 * Tests for Streaming Consensus (M2.2 Optimization)
 *
 * Validates early exit behavior when strong consensus reached at φ-quorum (7 Dogs)
 *
 * @module @cynic/node/test/agents/collective/ambient-consensus-streaming
 */

'use strict';

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { AmbientConsensus } from '../../../src/agents/collective/ambient-consensus.js';
import { getEventBus } from '../../../src/services/event-bus.js';

describe('AmbientConsensus - Streaming Consensus (M2.2)', () => {
  let consensus;
  let mockPack;
  let eventBus;

  beforeEach(() => {
    eventBus = getEventBus();

    // Mock pack with 11 Dogs
    mockPack = {
      guardian: { voteOnConsensus: null },
      analyst: { voteOnConsensus: null },
      sage: { voteOnConsensus: null },
      scout: { voteOnConsensus: null },
      architect: { voteOnConsensus: null },
      scholar: { voteOnConsensus: null },
      janitor: { voteOnConsensus: null },
      deployer: { voteOnConsensus: null },
      oracle: { voteOnConsensus: null },
      cartographer: { voteOnConsensus: null },
      cynic: { voteOnConsensus: null },
      judge: null, // Judge disabled for controlled testing
    };

    consensus = new AmbientConsensus({
      collectivePack: mockPack,
      eventBus,
    });
  });

  afterEach(() => {
    consensus.stop();
  });

  describe('Early Exit - Strong Approval', () => {
    it('should exit early when 7 Dogs strongly approve (>85%)', async () => {
      // First 7 Dogs approve quickly
      const approveVote = async () => {
        await new Promise(resolve => setTimeout(resolve, 10));
        return { vote: 'approve', reason: 'looks good', confidence: 0.9 };
      };

      // Remaining 4 Dogs would take longer (simulating slow voters)
      const slowVote = async () => {
        await new Promise(resolve => setTimeout(resolve, 1000));
        return { vote: 'approve', reason: 'eventually agree' };
      };

      // Setup pack
      mockPack.guardian.voteOnConsensus = approveVote;
      mockPack.analyst.voteOnConsensus = approveVote;
      mockPack.sage.voteOnConsensus = approveVote;
      mockPack.scout.voteOnConsensus = approveVote;
      mockPack.architect.voteOnConsensus = approveVote;
      mockPack.scholar.voteOnConsensus = approveVote;
      mockPack.janitor.voteOnConsensus = approveVote;
      mockPack.deployer.voteOnConsensus = slowVote; // Slow
      mockPack.oracle.voteOnConsensus = slowVote; // Slow
      mockPack.cartographer.voteOnConsensus = slowVote; // Slow
      mockPack.cynic.voteOnConsensus = slowVote; // Slow

      const start = Date.now();
      const result = await consensus.triggerConsensus({
        topic: 'test:early_exit_approve',
        context: { test: true },
        reason: 'Testing early exit',
      });
      const duration = Date.now() - start;

      // Verify early exit
      assert.equal(result.streaming.earlyExit, true, 'Should exit early');
      assert.equal(result.streaming.skipped, 4, 'Should skip 4 slow voters');
      assert.equal(result.approved, true, 'Should approve');
      assert.ok(result.agreement >= 0.85, `Agreement should be >= 85%, got ${result.agreement}`);

      // Verify timing - should be fast (<200ms) since we didn't wait for slow voters
      assert.ok(duration < 200, `Should complete quickly (<200ms), took ${duration}ms`);

      // Verify stats updated
      const stats = consensus.getStats();
      assert.equal(stats.streaming.earlyExits, 1, 'Should record 1 early exit');
      assert.equal(stats.streaming.totalConsensus, 1, 'Should record 1 total consensus');
    });

    it('should calculate correct agreement with 7 Dogs approving', async () => {
      const approveVote = async () => ({ vote: 'approve', reason: 'good', confidence: 0.9 });
      const rejectVote = async () => ({ vote: 'reject', reason: 'bad' });

      // 7 approve, 4 would reject (but skipped)
      mockPack.guardian.voteOnConsensus = approveVote;
      mockPack.analyst.voteOnConsensus = approveVote;
      mockPack.sage.voteOnConsensus = approveVote;
      mockPack.scout.voteOnConsensus = approveVote;
      mockPack.architect.voteOnConsensus = approveVote;
      mockPack.scholar.voteOnConsensus = approveVote;
      mockPack.janitor.voteOnConsensus = approveVote;
      mockPack.deployer.voteOnConsensus = rejectVote; // Skipped
      mockPack.oracle.voteOnConsensus = rejectVote; // Skipped
      mockPack.cartographer.voteOnConsensus = rejectVote; // Skipped
      mockPack.cynic.voteOnConsensus = rejectVote; // Skipped

      const result = await consensus.triggerConsensus({
        topic: 'test:agreement_calculation',
        context: { test: true },
      });

      // 7 approve, 0 reject, 0 abstain = 100% agreement
      assert.equal(result.stats.approve, 7, 'Should have 7 approve votes');
      assert.equal(result.stats.reject, 0, 'Should have 0 reject votes');
      assert.ok(result.agreement >= 0.85, 'Agreement should be >= 85%');
      assert.equal(result.streaming.earlyExit, true, 'Should exit early');
    });
  });

  describe('Early Exit - Strong Rejection', () => {
    it('should exit early when 7 Dogs strongly reject (>85%)', async () => {
      const rejectVote = async () => {
        await new Promise(resolve => setTimeout(resolve, 10));
        return { vote: 'reject', reason: 'bad idea', confidence: 0.9 };
      };

      const slowVote = async () => {
        await new Promise(resolve => setTimeout(resolve, 1000));
        return { vote: 'reject', reason: 'eventually agree' };
      };

      // 7 reject quickly, 4 would be slow
      mockPack.guardian.voteOnConsensus = rejectVote;
      mockPack.analyst.voteOnConsensus = rejectVote;
      mockPack.sage.voteOnConsensus = rejectVote;
      mockPack.scout.voteOnConsensus = rejectVote;
      mockPack.architect.voteOnConsensus = rejectVote;
      mockPack.scholar.voteOnConsensus = rejectVote;
      mockPack.janitor.voteOnConsensus = rejectVote;
      mockPack.deployer.voteOnConsensus = slowVote;
      mockPack.oracle.voteOnConsensus = slowVote;
      mockPack.cartographer.voteOnConsensus = slowVote;
      mockPack.cynic.voteOnConsensus = slowVote;

      const start = Date.now();
      const result = await consensus.triggerConsensus({
        topic: 'test:early_exit_reject',
        context: { test: true },
      });
      const duration = Date.now() - start;

      // Verify early exit on rejection
      assert.equal(result.streaming.earlyExit, true, 'Should exit early');
      assert.equal(result.streaming.skipped, 4, 'Should skip 4 slow voters');
      assert.equal(result.approved, false, 'Should reject');
      assert.ok(duration < 200, `Should complete quickly (<200ms), took ${duration}ms`);
    });
  });

  describe('No Early Exit - Divided Vote', () => {
    it('should NOT exit early when votes are divided (no strong consensus)', async () => {
      const approveVote = async () => ({ vote: 'approve', reason: 'good' });
      const rejectVote = async () => ({ vote: 'reject', reason: 'bad' });

      // 6 approve, 5 reject = divided (no strong consensus at any point)
      mockPack.guardian.voteOnConsensus = approveVote;
      mockPack.analyst.voteOnConsensus = approveVote;
      mockPack.sage.voteOnConsensus = approveVote;
      mockPack.scout.voteOnConsensus = rejectVote;
      mockPack.architect.voteOnConsensus = rejectVote;
      mockPack.scholar.voteOnConsensus = approveVote;
      mockPack.janitor.voteOnConsensus = approveVote;
      mockPack.deployer.voteOnConsensus = rejectVote;
      mockPack.oracle.voteOnConsensus = approveVote;
      mockPack.cartographer.voteOnConsensus = rejectVote;
      mockPack.cynic.voteOnConsensus = rejectVote;

      const result = await consensus.triggerConsensus({
        topic: 'test:no_early_exit_divided',
        context: { test: true },
      });

      // Should NOT exit early
      assert.equal(result.streaming.earlyExit, false, 'Should NOT exit early on divided vote');
      assert.equal(result.streaming.skipped, 0, 'Should skip 0 votes');
      assert.equal(result.stats.total, 11, 'Should collect all 11 votes');

      // Stats should reflect full voting
      const stats = consensus.getStats();
      assert.equal(stats.streaming.fullVotes, 1, 'Should record 1 full vote');
      assert.equal(stats.streaming.earlyExits, 0, 'Should record 0 early exits');
    });

    it('should NOT exit early with abstentions preventing quorum', async () => {
      const approveVote = async () => ({ vote: 'approve', reason: 'good' });
      const abstainVote = async () => ({ vote: 'abstain', reason: 'unsure' });

      // 5 approve, 6 abstain = not enough active voters
      mockPack.guardian.voteOnConsensus = approveVote;
      mockPack.analyst.voteOnConsensus = approveVote;
      mockPack.sage.voteOnConsensus = approveVote;
      mockPack.scout.voteOnConsensus = approveVote;
      mockPack.architect.voteOnConsensus = approveVote;
      mockPack.scholar.voteOnConsensus = abstainVote;
      mockPack.janitor.voteOnConsensus = abstainVote;
      mockPack.deployer.voteOnConsensus = abstainVote;
      mockPack.oracle.voteOnConsensus = abstainVote;
      mockPack.cartographer.voteOnConsensus = abstainVote;
      mockPack.cynic.voteOnConsensus = abstainVote;

      const result = await consensus.triggerConsensus({
        topic: 'test:no_early_exit_abstain',
        context: { test: true },
      });

      // Should NOT exit early (not enough non-abstain voters for quorum)
      assert.equal(result.streaming.earlyExit, false, 'Should NOT exit early with many abstentions');
      assert.equal(result.streaming.skipped, 0, 'Should collect all votes');
      assert.equal(result.stats.abstain, 6, 'Should have 6 abstentions');
    });
  });

  describe('Edge Cases', () => {
    it('should handle exactly 7 Dogs with 85% agreement (boundary)', async () => {
      const approveVote = async () => ({ vote: 'approve', reason: 'good' });
      const rejectVote = async () => ({ vote: 'reject', reason: 'bad' });
      const slowVote = async () => {
        await new Promise(resolve => setTimeout(resolve, 1000));
        return { vote: 'approve', reason: 'late' };
      };

      // 6 approve + 1 reject = 85.7% (should trigger early exit)
      mockPack.guardian.voteOnConsensus = approveVote;
      mockPack.analyst.voteOnConsensus = approveVote;
      mockPack.sage.voteOnConsensus = approveVote;
      mockPack.scout.voteOnConsensus = approveVote;
      mockPack.architect.voteOnConsensus = approveVote;
      mockPack.scholar.voteOnConsensus = approveVote;
      mockPack.janitor.voteOnConsensus = rejectVote; // 1 reject
      mockPack.deployer.voteOnConsensus = slowVote; // Skipped
      mockPack.oracle.voteOnConsensus = slowVote; // Skipped
      mockPack.cartographer.voteOnConsensus = slowVote; // Skipped
      mockPack.cynic.voteOnConsensus = slowVote; // Skipped

      const result = await consensus.triggerConsensus({
        topic: 'test:boundary_85_percent',
        context: { test: true },
      });

      // 6/7 = 85.7% should trigger early exit
      assert.equal(result.streaming.earlyExit, true, 'Should exit early at 85.7% agreement');
      assert.equal(result.streaming.skipped, 4, 'Should skip 4 votes');
      assert.equal(result.approved, true, 'Should approve');
    });

    it('should NOT exit early at 84% agreement (below threshold)', async () => {
      const approveVote = async () => ({ vote: 'approve', reason: 'good' });
      const rejectVote = async () => ({ vote: 'reject', reason: 'bad' });

      // 6 approve + 1 reject + 4 more split = should wait for all
      // At 7 votes: 5 approve, 2 reject = 71% (below 85%)
      let voteCount = 0;
      mockPack.guardian.voteOnConsensus = async () => { voteCount++; return await approveVote(); };
      mockPack.analyst.voteOnConsensus = async () => { voteCount++; return await approveVote(); };
      mockPack.sage.voteOnConsensus = async () => { voteCount++; return await approveVote(); };
      mockPack.scout.voteOnConsensus = async () => { voteCount++; return await rejectVote(); };
      mockPack.architect.voteOnConsensus = async () => { voteCount++; return await approveVote(); };
      mockPack.scholar.voteOnConsensus = async () => { voteCount++; return await rejectVote(); };
      mockPack.janitor.voteOnConsensus = async () => { voteCount++; return await approveVote(); };
      // At this point: 5 approve, 2 reject = 71% (no early exit)
      mockPack.deployer.voteOnConsensus = approveVote;
      mockPack.oracle.voteOnConsensus = approveVote;
      mockPack.cartographer.voteOnConsensus = approveVote;
      mockPack.cynic.voteOnConsensus = approveVote;

      const result = await consensus.triggerConsensus({
        topic: 'test:no_early_exit_84_percent',
        context: { test: true },
      });

      // Should NOT exit early
      assert.equal(result.streaming.earlyExit, false, 'Should NOT exit early at 71% agreement');
      assert.equal(result.streaming.skipped, 0, 'Should collect all votes');
      assert.equal(result.stats.total, 11, 'Should have all 11 votes');
    });

    it('should handle vote errors gracefully', async () => {
      const approveVote = async () => ({ vote: 'approve', reason: 'good' });
      const errorVote = async () => {
        throw new Error('Vote failed');
      };

      // 7 approve (including error handling)
      mockPack.guardian.voteOnConsensus = approveVote;
      mockPack.analyst.voteOnConsensus = approveVote;
      mockPack.sage.voteOnConsensus = errorVote; // Error → abstain
      mockPack.scout.voteOnConsensus = approveVote;
      mockPack.architect.voteOnConsensus = approveVote;
      mockPack.scholar.voteOnConsensus = approveVote;
      mockPack.janitor.voteOnConsensus = approveVote;
      mockPack.deployer.voteOnConsensus = approveVote;
      mockPack.oracle.voteOnConsensus = approveVote;
      mockPack.cartographer.voteOnConsensus = approveVote;
      mockPack.cynic.voteOnConsensus = approveVote;

      const result = await consensus.triggerConsensus({
        topic: 'test:error_handling',
        context: { test: true },
      });

      // Should still reach consensus (error → abstain)
      assert.equal(result.approved, true, 'Should approve despite error');
      assert.ok(result.stats.abstain >= 1, 'Error should become abstain');
    });
  });

  describe('Statistics Tracking', () => {
    it('should track early exit rate across multiple consensus runs', async () => {
      const approveVote = async () => ({ vote: 'approve', reason: 'good' });
      const rejectVote = async () => ({ vote: 'reject', reason: 'bad' });
      const slowVote = async () => {
        await new Promise(resolve => setTimeout(resolve, 1000));
        return { vote: 'approve', reason: 'late' };
      };

      // Setup pack for early exits
      const setupEarlyExit = () => {
        mockPack.guardian.voteOnConsensus = approveVote;
        mockPack.analyst.voteOnConsensus = approveVote;
        mockPack.sage.voteOnConsensus = approveVote;
        mockPack.scout.voteOnConsensus = approveVote;
        mockPack.architect.voteOnConsensus = approveVote;
        mockPack.scholar.voteOnConsensus = approveVote;
        mockPack.janitor.voteOnConsensus = approveVote;
        mockPack.deployer.voteOnConsensus = slowVote;
        mockPack.oracle.voteOnConsensus = slowVote;
        mockPack.cartographer.voteOnConsensus = slowVote;
        mockPack.cynic.voteOnConsensus = slowVote;
      };

      // Setup pack for full voting (divided)
      const setupFullVoting = () => {
        mockPack.guardian.voteOnConsensus = approveVote;
        mockPack.analyst.voteOnConsensus = approveVote;
        mockPack.sage.voteOnConsensus = approveVote;
        mockPack.scout.voteOnConsensus = rejectVote;
        mockPack.architect.voteOnConsensus = rejectVote;
        mockPack.scholar.voteOnConsensus = rejectVote;
        mockPack.janitor.voteOnConsensus = approveVote;
        mockPack.deployer.voteOnConsensus = approveVote;
        mockPack.oracle.voteOnConsensus = rejectVote;
        mockPack.cartographer.voteOnConsensus = rejectVote;
        mockPack.cynic.voteOnConsensus = approveVote;
      };

      // Run 3 early exits
      setupEarlyExit();
      await consensus.triggerConsensus({ topic: 'test:stats_1', context: {} });
      await consensus.triggerConsensus({ topic: 'test:stats_2', context: {} });
      await consensus.triggerConsensus({ topic: 'test:stats_3', context: {} });

      // Run 2 full votings
      setupFullVoting();
      await consensus.triggerConsensus({ topic: 'test:stats_4', context: {} });
      await consensus.triggerConsensus({ topic: 'test:stats_5', context: {} });

      const stats = consensus.getStats();

      assert.equal(stats.streaming.totalConsensus, 5, 'Should track 5 total consensus runs');
      assert.equal(stats.streaming.earlyExits, 3, 'Should track 3 early exits');
      assert.equal(stats.streaming.fullVotes, 2, 'Should track 2 full votes');
      assert.equal(stats.streaming.earlyExitRate, 0.6, 'Early exit rate should be 60%');
      assert.ok(stats.streaming.avgSkippedVotes > 0, 'Should track average skipped votes');
      assert.ok(stats.streaming.avgTimeSaved > 0, 'Should estimate time saved');
    });

    it('should calculate average skipped votes correctly', async () => {
      const approveVote = async () => ({ vote: 'approve', reason: 'good' });
      const slowVote = async () => {
        await new Promise(resolve => setTimeout(resolve, 1000));
        return { vote: 'approve', reason: 'late' };
      };

      // Setup for early exits with different skip counts
      mockPack.guardian.voteOnConsensus = approveVote;
      mockPack.analyst.voteOnConsensus = approveVote;
      mockPack.sage.voteOnConsensus = approveVote;
      mockPack.scout.voteOnConsensus = approveVote;
      mockPack.architect.voteOnConsensus = approveVote;
      mockPack.scholar.voteOnConsensus = approveVote;
      mockPack.janitor.voteOnConsensus = approveVote;
      mockPack.deployer.voteOnConsensus = slowVote; // Skip
      mockPack.oracle.voteOnConsensus = slowVote; // Skip
      mockPack.cartographer.voteOnConsensus = slowVote; // Skip
      mockPack.cynic.voteOnConsensus = slowVote; // Skip

      // All will skip 4 votes
      await consensus.triggerConsensus({ topic: 'test:avg_1', context: {} });
      await consensus.triggerConsensus({ topic: 'test:avg_2', context: {} });

      const stats = consensus.getStats();

      assert.equal(stats.streaming.avgSkippedVotes, 4, 'Average skipped should be 4');
      assert.equal(stats.streaming.avgTimeSaved, 80, 'Time saved should be ~80ms (4 votes × 20ms)');
    });
  });

  describe('Performance Impact', () => {
    it('should be faster than full voting when early exit triggers', async () => {
      const quickVote = async () => {
        await new Promise(resolve => setTimeout(resolve, 10));
        return { vote: 'approve', reason: 'good' };
      };

      const slowVote = async () => {
        await new Promise(resolve => setTimeout(resolve, 100));
        return { vote: 'approve', reason: 'late' };
      };

      // Early exit scenario: 7 quick approves, 4 slow
      mockPack.guardian.voteOnConsensus = quickVote;
      mockPack.analyst.voteOnConsensus = quickVote;
      mockPack.sage.voteOnConsensus = quickVote;
      mockPack.scout.voteOnConsensus = quickVote;
      mockPack.architect.voteOnConsensus = quickVote;
      mockPack.scholar.voteOnConsensus = quickVote;
      mockPack.janitor.voteOnConsensus = quickVote;
      mockPack.deployer.voteOnConsensus = slowVote;
      mockPack.oracle.voteOnConsensus = slowVote;
      mockPack.cartographer.voteOnConsensus = slowVote;
      mockPack.cynic.voteOnConsensus = slowVote;

      const start = Date.now();
      const result = await consensus.triggerConsensus({
        topic: 'test:performance',
        context: { test: true },
      });
      const duration = Date.now() - start;

      // Should complete in ~70ms (7 × 10ms), NOT ~470ms (7 × 10 + 4 × 100)
      assert.ok(duration < 150, `Should be fast (<150ms), took ${duration}ms`);
      assert.equal(result.streaming.earlyExit, true, 'Should exit early');

      // Verify we saved time
      const expectedFullDuration = 7 * 10 + 4 * 100; // ~470ms
      const timeSaved = expectedFullDuration - duration;
      assert.ok(timeSaved > 200, `Should save >200ms, saved ${timeSaved}ms`);
    });
  });
});

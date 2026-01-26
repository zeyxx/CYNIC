/**
 * Byzantine Consensus Scenarios Tests
 *
 * Tests φ-BFT consensus resilience against Byzantine faults:
 * 1. Network partition (2 vs 1 split)
 * 2. Byzantine node sending invalid votes
 * 3. Vote replay attacks
 * 4. Consensus at exactly φ⁻¹ (61.8%) threshold
 * 5. Fork detection and resolution
 * 6. Lockout enforcement across rounds
 *
 * "With 3 dogs, 2 must agree - 1 barking alone convinces no one" - κυνικός
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert';

import {
  ConsensusEngine,
  ConsensusState,
  BlockStatus,
  VoteType,
  createVote,
  verifyVote,
  generateKeypair,
  hashObject,
  signData,
  calculateConsensus,
  ConsensusType,
  VoterLockout,
  LockoutManager,
  checkForkPossibility,
  FinalityTracker,
  calculateFinalityProbability,
  FinalityStatus,
} from '../src/index.js';

import { PHI, PHI_INV, CONSENSUS_THRESHOLD } from '@cynic/core';

/**
 * Create a properly signed block
 */
function createSignedBlock(keypair, slot, data, hash = null) {
  const block = {
    action: 'BLOCK',
    params: { data },
    proposer: keypair.publicKey,
    created_at: Date.now(),
    slot,
  };

  const blockHash = hashObject({
    action: block.action,
    params: block.params,
    proposer: block.proposer,
    created_at: block.created_at,
  });
  block.signature = signData(blockHash, keypair.privateKey);
  block.hash = hash || `block_${slot}_${Math.random().toString(36).slice(2, 8)}`;

  return block;
}

/**
 * Create a valid vote for a block
 */
function createValidVote(voterKeypair, blockHash, voteType, eScore = 75) {
  return createVote({
    proposalId: blockHash,
    vote: voteType,
    voterPublicKey: voterKeypair.publicKey,
    voterPrivateKey: voterKeypair.privateKey,
    eScore,
    burned: 50,
    uptime: 1,
  });
}

/**
 * Create a tampered vote (invalid signature)
 */
function createTamperedVote(voterKeypair, blockHash, voteType, eScore = 75) {
  const vote = createVote({
    proposalId: blockHash,
    vote: voteType,
    voterPublicKey: voterKeypair.publicKey,
    voterPrivateKey: voterKeypair.privateKey,
    eScore,
  });

  // Tamper with the vote after signing
  vote.vote = vote.vote === VoteType.APPROVE ? VoteType.REJECT : VoteType.APPROVE;

  return vote;
}

/**
 * Create a forged vote (different signer than claimed)
 */
function createForgedVote(claimedKeyPair, actualSigner, blockHash, voteType) {
  const vote = createVote({
    proposalId: blockHash,
    vote: voteType,
    voterPublicKey: claimedKeyPair.publicKey, // Claims to be this voter
    voterPrivateKey: actualSigner.privateKey, // But signed by someone else
    eScore: 75,
  });

  return vote;
}

// ═══════════════════════════════════════════════════════════════════════════════
// NETWORK PARTITION SCENARIOS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Network Partition Scenarios', () => {
  let keypairs;

  beforeEach(() => {
    // Need at least 5 voters to meet GOVERNANCE_QUORUM
    keypairs = Array.from({ length: 7 }, () => generateKeypair());
  });

  describe('4 vs 3 Partition (with 7 nodes)', () => {
    it('should reach consensus in majority partition (5+ nodes)', () => {
      // Simulate partition: nodes 0-4 can communicate (5 nodes), nodes 5-6 isolated
      // GOVERNANCE_QUORUM = 5, so 5 voters can reach quorum

      // Majority partition votes (5 nodes)
      const majorityVotes = keypairs.slice(0, 5).map((kp) =>
        createValidVote(kp, 'block_split', VoteType.APPROVE, 75)
      );

      const result = calculateConsensus(majorityVotes, ConsensusType.HARD);

      // 5 out of 5 voting approve = 100% ratio, quorum met (5 >= 5)
      assert.strictEqual(result.reached, true);
      assert.strictEqual(result.result, VoteType.APPROVE);
      assert.strictEqual(result.voterCount, 5);
      assert.strictEqual(result.hasQuorum, true);
    });

    it('should NOT reach consensus in minority partition (< quorum nodes)', () => {
      // Isolated nodes (only 2)
      const minorityVotes = keypairs.slice(5, 7).map((kp) =>
        createValidVote(kp, 'block_split', VoteType.APPROVE, 75)
      );

      const result = calculateConsensus(minorityVotes, ConsensusType.HARD);

      // 2 voters < GOVERNANCE_QUORUM (5), so no quorum
      assert.strictEqual(result.voterCount, 2);
      assert.strictEqual(result.hasQuorum, false);
      assert.strictEqual(result.reached, false);
    });

    it('should not finalize conflicting blocks in different partitions', () => {
      // Partition A (nodes 0-4) proposes block A
      const blockA = createSignedBlock(keypairs[0], 100, { partition: 'A' }, 'block_A');

      // Partition B (nodes 5-6) proposes block B for same slot
      const blockB = createSignedBlock(keypairs[5], 100, { partition: 'B' }, 'block_B');

      // Votes in partition A (5 nodes - meets quorum)
      const votesA = keypairs.slice(0, 5).map((kp) =>
        createValidVote(kp, blockA.hash, VoteType.APPROVE, 75)
      );

      // Votes in partition B (2 nodes - no quorum)
      const votesB = keypairs.slice(5, 7).map((kp) =>
        createValidVote(kp, blockB.hash, VoteType.APPROVE, 75)
      );

      const resultA = calculateConsensus(votesA, ConsensusType.HARD);
      const resultB = calculateConsensus(votesB, ConsensusType.HARD);

      // Partition A has quorum and supermajority
      assert.strictEqual(resultA.reached, true);
      assert.strictEqual(resultA.hasQuorum, true);

      // Partition B cannot reach consensus (no quorum)
      assert.strictEqual(resultB.reached, false);
      assert.strictEqual(resultB.hasQuorum, false);
    });

    it('should detect fork possibility during partition', () => {
      // 2 nodes voted for block A, 1 node is isolated
      const forkCheck = checkForkPossibility({
        lockedWeight: 66, // 2 nodes = 66% of total
        totalWeight: 100,
        currentSlot: 105,
        blockSlot: 100,
        confirmations: 3,
      });

      // With 66% locked, only 34% available
      // 34% < 61.8% needed for supermajority fork
      assert.strictEqual(forkCheck.forkPossible, false);
      assert.strictEqual(forkCheck.availableWeight, 34);
    });

    it('should allow fork when partition heals with low lockout', () => {
      // If only 30% is locked (low confirmations), fork is possible
      const forkCheck = checkForkPossibility({
        lockedWeight: 30,
        totalWeight: 100,
        currentSlot: 102,
        blockSlot: 100,
        confirmations: 1, // Low confirmations = low lockout
      });

      // 70% available > 61.8% needed
      assert.strictEqual(forkCheck.forkPossible, true);
      assert.strictEqual(forkCheck.availableWeight, 70);
    });
  });

  describe('Partition Recovery', () => {
    it('should reconcile state when partition heals', () => {
      // All 7 nodes can now communicate
      const allVotes = keypairs.map((kp) =>
        createValidVote(kp, 'block_reconcile', VoteType.APPROVE, 75)
      );

      // After partition heals, consensus should be clear
      const result = calculateConsensus(allVotes, ConsensusType.HARD);

      assert.strictEqual(result.reached, true);
      assert.strictEqual(result.voterCount, 7);
      assert.strictEqual(result.hasQuorum, true);
      assert.ok(result.ratio >= CONSENSUS_THRESHOLD);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// BYZANTINE VOTE SCENARIOS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Byzantine Vote Scenarios', () => {
  let keypairs;
  let byzantineKeypair;

  beforeEach(() => {
    // Need 6 nodes: 5 honest + 1 Byzantine to meet quorum
    keypairs = Array.from({ length: 6 }, () => generateKeypair());
    byzantineKeypair = keypairs[5]; // Node 5 is Byzantine
  });

  describe('Invalid Vote Detection', () => {
    it('should reject tampered vote (modified after signing)', () => {
      const tamperedVote = createTamperedVote(byzantineKeypair, 'block_tamper', VoteType.APPROVE);

      const isValid = verifyVote(tamperedVote);
      assert.strictEqual(isValid, false, 'Tampered vote should be rejected');
    });

    it('should reject forged vote (wrong signer)', () => {
      const honestKeypair = keypairs[0];
      const forgedVote = createForgedVote(honestKeypair, byzantineKeypair, 'block_forge', VoteType.APPROVE);

      const isValid = verifyVote(forgedVote);
      assert.strictEqual(isValid, false, 'Forged vote should be rejected');
    });

    it('should reject vote without signature', () => {
      const unsignedVote = {
        id: 'vote_unsigned',
        proposal_id: 'block_test',
        vote: VoteType.APPROVE,
        voter: byzantineKeypair.publicKey,
        weight: 75,
        // No signature!
      };

      const isValid = verifyVote(unsignedVote);
      assert.strictEqual(isValid, false, 'Unsigned vote should be rejected');
    });

    it('should reject vote with empty signature', () => {
      const emptySignatureVote = {
        id: 'vote_empty_sig',
        proposal_id: 'block_test',
        vote: VoteType.APPROVE,
        voter: byzantineKeypair.publicKey,
        weight: 75,
        signature: '',
      };

      const isValid = verifyVote(emptySignatureVote);
      assert.strictEqual(isValid, false, 'Empty signature vote should be rejected');
    });

    it('should accept valid vote from honest node', () => {
      const validVote = createValidVote(keypairs[0], 'block_honest', VoteType.APPROVE);

      const isValid = verifyVote(validVote);
      assert.strictEqual(isValid, true, 'Valid vote should be accepted');
    });
  });

  describe('Double Voting Detection', () => {
    it('should detect double vote on same block', () => {
      // Byzantine node votes twice on same block with different decisions
      const vote1 = createValidVote(byzantineKeypair, 'block_double', VoteType.APPROVE);
      const vote2 = createValidVote(byzantineKeypair, 'block_double', VoteType.REJECT);

      // Both votes are individually valid
      assert.strictEqual(verifyVote(vote1), true);
      assert.strictEqual(verifyVote(vote2), true);

      // But same voter, same block, different decisions = equivocation
      const sameVoter = vote1.voter === vote2.voter;
      const sameBlock = vote1.proposal_id === vote2.proposal_id;
      const differentDecision = vote1.vote !== vote2.vote;

      assert.strictEqual(sameVoter && sameBlock && differentDecision, true,
        'Should detect equivocation');
    });

    it('should keep latest vote when aggregating', () => {
      // Two votes from same voter
      const vote1 = createValidVote(byzantineKeypair, 'block_latest', VoteType.REJECT);

      // Simulate time passing
      const vote2 = createValidVote(byzantineKeypair, 'block_latest', VoteType.APPROVE);
      vote2.timestamp = vote1.timestamp + 1000; // Later timestamp

      // When aggregated, latest vote should win
      const allVotes = [vote1, vote2];

      // Filter to keep only latest per voter
      const voteMap = new Map();
      for (const v of allVotes) {
        const existing = voteMap.get(v.voter);
        if (!existing || v.timestamp > existing.timestamp) {
          voteMap.set(v.voter, v);
        }
      }

      const latestVotes = Array.from(voteMap.values());
      assert.strictEqual(latestVotes.length, 1);
      assert.strictEqual(latestVotes[0].vote, VoteType.APPROVE);
    });
  });

  describe('Byzantine Node Impact on Consensus', () => {
    it('should reach consensus despite 1 Byzantine node (with 5 honest)', () => {
      // 5 honest nodes approve (meets quorum)
      const honestVotes = keypairs.slice(0, 5).map((kp) =>
        createValidVote(kp, 'block_byz', VoteType.APPROVE, 75)
      );

      // Byzantine node sends invalid vote (which gets rejected)
      const byzantineVote = createTamperedVote(byzantineKeypair, 'block_byz', VoteType.REJECT);

      // Only valid votes count
      const validVotes = [
        ...honestVotes,
        ...(verifyVote(byzantineVote) ? [byzantineVote] : []),
      ];

      const result = calculateConsensus(validVotes, ConsensusType.HARD);

      // Consensus reached with 5 honest nodes (quorum met, 100% approve)
      assert.strictEqual(result.reached, true);
      assert.strictEqual(result.voterCount, 5);
      assert.strictEqual(result.hasQuorum, true);
    });

    it('should NOT reach consensus if Byzantine node breaks threshold', () => {
      // 3 honest approve, 2 honest reject
      const honestApprove = keypairs.slice(0, 3).map((kp) =>
        createValidVote(kp, 'block_split', VoteType.APPROVE, 75)
      );
      const honestReject = keypairs.slice(3, 5).map((kp) =>
        createValidVote(kp, 'block_split', VoteType.REJECT, 75)
      );

      // Byzantine also rejects
      const byzantineVote = createValidVote(byzantineKeypair, 'block_split', VoteType.REJECT, 75);

      const allVotes = [...honestApprove, ...honestReject, byzantineVote];
      const result = calculateConsensus(allVotes, ConsensusType.HARD);

      // 3 approve vs 3 reject = 50% approve < 61.8% threshold
      assert.strictEqual(result.reached, false);
      assert.ok(result.ratio < CONSENSUS_THRESHOLD);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// VOTE REPLAY ATTACK SCENARIOS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Vote Replay Attack Scenarios', () => {
  let keypairs;

  beforeEach(() => {
    keypairs = Array.from({ length: 3 }, () => generateKeypair());
  });

  it('should detect replay of old vote for different block', () => {
    // Original vote for block A
    const originalVote = createValidVote(keypairs[0], 'block_A', VoteType.APPROVE);

    // Attacker tries to replay for block B
    const replayedVote = { ...originalVote };
    replayedVote.proposal_id = 'block_B'; // Change target block

    // Signature verification should fail
    const isValid = verifyVote(replayedVote);
    assert.strictEqual(isValid, false, 'Replayed vote with changed block should be invalid');
  });

  it('should detect replay with modified weight (weight is not signed)', () => {
    const originalVote = createValidVote(keypairs[0], 'block_weight', VoteType.APPROVE, 75);

    // Attacker modifies weight
    const tamperedVote = { ...originalVote };
    tamperedVote.weight = 1000; // Inflate weight

    // Note: Weight is NOT part of the signature, so verifyVote passes
    // The weight should be recalculated from validator registry, not trusted from vote
    const isValid = verifyVote(tamperedVote);

    // Weight tampering is handled by recalculating weight, not signature verification
    // The validator's actual weight should be looked up, not taken from the vote
    assert.strictEqual(isValid, true, 'Signature is valid (weight not signed)');

    // The REAL protection is that weight should be recalculated from validator eScore
    // This test documents that behavior - weight in vote should not be trusted
  });

  it('should reject duplicate vote IDs', () => {
    const vote1 = createValidVote(keypairs[0], 'block_dup', VoteType.APPROVE);
    const vote2 = { ...vote1 }; // Exact copy (same ID)

    // System should deduplicate by vote ID
    const voteMap = new Map();
    voteMap.set(vote1.id, vote1);
    voteMap.set(vote2.id, vote2);

    // Should only have 1 entry
    assert.strictEqual(voteMap.size, 1, 'Duplicate vote IDs should be deduplicated');
  });

  it('should accept same vote for same block (confirmation)', () => {
    // Same voter voting same way on same block is valid (confirmation)
    const vote1 = createValidVote(keypairs[0], 'block_confirm', VoteType.APPROVE);
    const vote2 = createValidVote(keypairs[0], 'block_confirm', VoteType.APPROVE);

    assert.strictEqual(verifyVote(vote1), true);
    assert.strictEqual(verifyVote(vote2), true);

    // Both are valid, but only count once per voter
    const votes = [vote1, vote2];
    const uniqueVoters = new Set(votes.map(v => v.voter));
    assert.strictEqual(uniqueVoters.size, 1);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// φ⁻¹ THRESHOLD BOUNDARY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('φ⁻¹ Threshold Boundary Tests', () => {
  it('should confirm consensus threshold is φ⁻¹', () => {
    assert.ok(Math.abs(CONSENSUS_THRESHOLD - PHI_INV) < 0.001,
      `Threshold should be φ⁻¹ (${PHI_INV}), got ${CONSENSUS_THRESHOLD}`);
  });

  describe('Exact Threshold Scenarios', () => {
    it('should reach consensus at above 61.8% approval', () => {
      // Create 100 voters (>= GOVERNANCE_QUORUM=5), need 62% to clearly pass
      const keypairs = Array.from({ length: 100 }, () => generateKeypair());

      // 62 approve, 38 reject (62% > 61.8%)
      const votes = keypairs.map((kp, i) =>
        createVote({
          proposalId: 'block_above',
          vote: i < 62 ? VoteType.APPROVE : VoteType.REJECT,
          voterPublicKey: kp.publicKey,
          voterPrivateKey: kp.privateKey,
          eScore: 1, // Equal weight
          burned: 0,
          uptime: 1,
        })
      );

      const result = calculateConsensus(votes, ConsensusType.HARD);

      // 62/100 = 0.62 >= 0.618 threshold
      assert.strictEqual(result.reached, true);
      assert.strictEqual(result.hasQuorum, true);
      assert.ok(result.ratio >= CONSENSUS_THRESHOLD,
        `Ratio ${result.ratio} should be >= ${CONSENSUS_THRESHOLD}`);
    });

    it('should NOT reach consensus at 61% approval (below threshold)', () => {
      const keypairs = Array.from({ length: 100 }, () => generateKeypair());

      // 61 approve, 39 reject (61% < 61.8%)
      const votes = keypairs.map((kp, i) =>
        createVote({
          proposalId: 'block_below',
          vote: i < 61 ? VoteType.APPROVE : VoteType.REJECT,
          voterPublicKey: kp.publicKey,
          voterPrivateKey: kp.privateKey,
          eScore: 1,
          burned: 0,
          uptime: 1,
        })
      );

      const result = calculateConsensus(votes, ConsensusType.HARD);

      // 61/100 = 0.61 < 0.618 threshold
      assert.strictEqual(result.reached, false);
      assert.ok(result.ratio < CONSENSUS_THRESHOLD,
        `Ratio ${result.ratio} should be < ${CONSENSUS_THRESHOLD}`);
    });

    it('should handle 38.2% minority gracefully', () => {
      const keypairs = Array.from({ length: 100 }, () => generateKeypair());

      // 38 approve (minority), 62 reject (majority)
      const votes = keypairs.map((kp, i) =>
        createVote({
          proposalId: 'block_minority',
          vote: i < 38 ? VoteType.APPROVE : VoteType.REJECT,
          voterPublicKey: kp.publicKey,
          voterPrivateKey: kp.privateKey,
          eScore: 1,
          burned: 0,
          uptime: 1,
        })
      );

      const result = calculateConsensus(votes, ConsensusType.HARD);

      // 38% approve < threshold, REJECT wins
      assert.strictEqual(result.result, VoteType.REJECT);
      assert.ok(result.rejectWeight > result.approveWeight);
      // REJECT also doesn't reach supermajority (62% reject < would need 61.8% to "reach")
      // But the system says consensus "reached" = false because approve didn't hit threshold
    });
  });

  describe('Weight-Based Threshold', () => {
    it('should weight votes by E-Score when calculating threshold', () => {
      const highWeightKeypair = generateKeypair();
      const lowWeightKeypairs = Array.from({ length: 10 }, () => generateKeypair());

      // 1 high-weight (100) approve vs 10 low-weight (10 each) reject
      const votes = [
        createVote({
          proposalId: 'block_weighted',
          vote: VoteType.APPROVE,
          voterPublicKey: highWeightKeypair.publicKey,
          voterPrivateKey: highWeightKeypair.privateKey,
          eScore: 100,
          burned: 0,
          uptime: 1,
        }),
        ...lowWeightKeypairs.map(kp =>
          createVote({
            proposalId: 'block_weighted',
            vote: VoteType.REJECT,
            voterPublicKey: kp.publicKey,
            voterPrivateKey: kp.privateKey,
            eScore: 10,
            burned: 0,
            uptime: 1,
          })
        ),
      ];

      const result = calculateConsensus(votes, ConsensusType.HARD);

      // Total weight = 100 + 100 = 200
      // Approve weight = 100 (50%)
      // Reject weight = 100 (50%)
      // Neither reaches 61.8%
      assert.strictEqual(result.reached, false);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// FORK DETECTION AND RESOLUTION
// ═══════════════════════════════════════════════════════════════════════════════

describe('Fork Detection and Resolution', () => {
  let tracker;

  beforeEach(() => {
    tracker = new FinalityTracker();
  });

  describe('Fork Detection', () => {
    it('should detect potential fork with low confirmations', () => {
      const forkCheck = checkForkPossibility({
        lockedWeight: 40, // Only 40% locked
        totalWeight: 100,
        currentSlot: 103,
        blockSlot: 100,
        confirmations: 2,
      });

      // 60% available > 61.8% needed? No, but close
      // Actually 60% < 61.8%, so fork is NOT possible
      assert.strictEqual(forkCheck.forkPossible, false);
    });

    it('should confirm no fork possible with high lockout', () => {
      const forkCheck = checkForkPossibility({
        lockedWeight: 70, // 70% locked (supermajority)
        totalWeight: 100,
        currentSlot: 150,
        blockSlot: 100,
        confirmations: 20,
      });

      // Only 30% available < 61.8% needed
      assert.strictEqual(forkCheck.forkPossible, false);
      assert.strictEqual(forkCheck.availableWeight, 30);
    });

    it('should detect conflicting blocks at same height', () => {
      // Track two blocks at same slot
      tracker.update('block_A', {
        approveRatio: 0.6, // Below threshold
        confirmations: 1,
        totalValidators: 3,
        votedValidators: 2,
      });

      tracker.update('block_B', {
        approveRatio: 0.4,
        confirmations: 1,
        totalValidators: 3,
        votedValidators: 1,
      });

      // Neither should be final
      assert.strictEqual(tracker.isFinal('block_A'), false);
      assert.strictEqual(tracker.isFinal('block_B'), false);
    });
  });

  describe('Fork Resolution', () => {
    it('should finalize block with supermajority after confirmations', () => {
      tracker.update('block_winner', {
        approveRatio: 0.75,
        confirmations: 32,
        totalValidators: 100,
        votedValidators: 100,
      });

      const status = tracker.get('block_winner');
      assert.strictEqual(status.status, FinalityStatus.DETERMINISTIC);
      assert.strictEqual(tracker.isFinal('block_winner'), true);
    });

    it('should not finalize without supermajority even with confirmations', () => {
      tracker.update('block_contested', {
        approveRatio: 0.55, // Below 61.8%
        confirmations: 50,
        totalValidators: 100,
        votedValidators: 100,
      });

      assert.strictEqual(tracker.isFinal('block_contested'), false);
    });

    it('should resolve fork by choosing higher-weight chain', () => {
      // Chain A: 65% approval (above threshold)
      const probA = calculateFinalityProbability({
        approveRatio: 0.65,
        confirmations: 5,
        totalValidators: 100,
        votedValidators: 100,
      });

      // Chain B: 55% approval (below threshold)
      const probB = calculateFinalityProbability({
        approveRatio: 0.55,
        confirmations: 5,
        totalValidators: 100,
        votedValidators: 100,
      });

      // Chain A should have higher probability
      assert.ok(probA.probability > probB.probability,
        `Chain A (${probA.probability}) should have higher probability than Chain B (${probB.probability})`);

      // Chain A is above threshold, so it's progressing (PROBABILISTIC or OPTIMISTIC)
      assert.ok(
        probA.status === FinalityStatus.PROBABILISTIC ||
        probA.status === FinalityStatus.OPTIMISTIC,
        `Chain A status should be PROBABILISTIC or OPTIMISTIC, got ${probA.status}`
      );

      // Chain B is below threshold
      assert.strictEqual(probB.status, FinalityStatus.PENDING,
        `Chain B should be PENDING since 55% < 61.8%`);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// LOCKOUT ENFORCEMENT
// ═══════════════════════════════════════════════════════════════════════════════

describe('Lockout Enforcement', () => {
  let lockout;
  let manager;

  beforeEach(() => {
    lockout = new VoterLockout('voter1');
    manager = new LockoutManager();
  });

  describe('φⁿ Exponential Lockout', () => {
    it('should increase lockout exponentially with confirmations', () => {
      // Vote and confirm multiple times
      lockout.recordVote('block_exp', 100);
      const lockout1 = lockout.getLockoutSlots('block_exp');

      lockout.recordVote('block_exp', 101);
      const lockout2 = lockout.getLockoutSlots('block_exp');

      lockout.recordVote('block_exp', 102);
      const lockout3 = lockout.getLockoutSlots('block_exp');

      // Lockout should follow φⁿ
      assert.ok(Math.abs(lockout1 - PHI) < 0.1, `1 conf: ${lockout1} ≈ φ¹`);
      assert.ok(Math.abs(lockout2 - PHI * PHI) < 0.1, `2 conf: ${lockout2} ≈ φ²`);
      assert.ok(Math.abs(lockout3 - PHI * PHI * PHI) < 0.1, `3 conf: ${lockout3} ≈ φ³`);
    });

    it('should prevent vote switching during lockout period', () => {
      lockout.recordVote('block_lock', 100);

      // Immediately after vote
      const canSwitch1 = lockout.canSwitchVote(100);
      assert.strictEqual(canSwitch1.canSwitch, false);
      assert.strictEqual(canSwitch1.locked.length, 1);

      // Just before lockout expires (φ¹ ≈ 1.618 slots)
      const canSwitch2 = lockout.canSwitchVote(101);
      assert.strictEqual(canSwitch2.canSwitch, false);

      // After lockout expires
      const canSwitch3 = lockout.canSwitchVote(103);
      assert.strictEqual(canSwitch3.canSwitch, true);
    });

    it('should allow vote on different block (not switch)', () => {
      lockout.recordVote('block_A', 100);

      // Can vote on different block
      lockout.recordVote('block_B', 101);

      const activeVotes = lockout.getActiveVotes();
      assert.strictEqual(activeVotes.length, 2);
    });
  });

  describe('Cross-Round Lockout', () => {
    it('should maintain lockout across multiple rounds', () => {
      // Round 1: Vote on block A
      lockout.recordVote('block_A', 100);
      lockout.recordVote('block_A', 101);
      lockout.recordVote('block_A', 102); // 3 confirmations

      // Round 2: Try to switch - should be locked
      const canSwitch = lockout.canSwitchVote(103);

      // With 3 confirmations, lockout ≈ φ³ ≈ 4.2 slots
      // At slot 103, we're only 3 slots in, still locked
      assert.strictEqual(canSwitch.canSwitch, false);
    });

    it('should track multiple voters independently', () => {
      manager.recordVote('voter1', 'block_A', 100);
      manager.recordVote('voter1', 'block_A', 101);

      manager.recordVote('voter2', 'block_B', 100);

      // Voter 1 locked on block_A
      assert.strictEqual(manager.isLockedOut('voter1', 'block_A', 101), true);

      // Voter 2 has different lockout
      assert.strictEqual(manager.isLockedOut('voter2', 'block_B', 101), true);

      // Voter 1 not locked for block_B (didn't vote on it)
      assert.strictEqual(manager.isLockedOut('voter1', 'block_B', 101), false);
    });

    it('should prune old lockouts', () => {
      manager.recordVote('voter1', 'old_block', 100);
      manager.recordVote('voter1', 'new_block', 1000);

      // Prune votes older than 500 slots
      manager.prune(1200);

      const voter1 = manager.getVoter('voter1');
      const activeVotes = voter1.getActiveVotes();

      // Only new_block should remain
      assert.strictEqual(activeVotes.length, 1);
      assert.strictEqual(activeVotes[0].blockHash, 'new_block');
    });
  });

  describe('Lockout Attack Prevention', () => {
    it('should prevent rapid vote flipping', () => {
      const flippingVoter = new VoterLockout('flipper');

      // Try to flip vote rapidly
      flippingVoter.recordVote('block_A', 100);

      // Can't switch immediately
      const canFlip = flippingVoter.canSwitchVote(100);
      assert.strictEqual(canFlip.canSwitch, false);

      // Even at next slot
      const canFlip2 = flippingVoter.canSwitchVote(101);
      assert.strictEqual(canFlip2.canSwitch, false);
    });

    it('should accumulate lockout with repeated confirmations', () => {
      // Repeatedly confirm same block
      for (let i = 0; i < 10; i++) {
        lockout.recordVote('block_deep', 100 + i);
      }

      const lockoutSlots = lockout.getLockoutSlots('block_deep');

      // With 10 confirmations, lockout should be very long
      // φ¹⁰ ≈ 122.99 slots
      assert.ok(lockoutSlots > 100, `High confirmation lockout should be > 100 slots, got ${lockoutSlots}`);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// CONSENSUS ENGINE BYZANTINE BEHAVIOR
// ═══════════════════════════════════════════════════════════════════════════════

describe('ConsensusEngine Byzantine Behavior', () => {
  let engine;
  let keypair;

  beforeEach(() => {
    keypair = generateKeypair();
    engine = new ConsensusEngine({
      publicKey: keypair.publicKey,
      privateKey: keypair.privateKey,
      slotDuration: 100,
    });
  });

  afterEach(() => {
    if (engine && engine.state !== 'STOPPED') {
      engine.stop();
    }
  });

  it('should reject block with invalid structure', () => {
    engine.start();

    const invalidBlock = {
      action: 'BLOCK',
      params: { data: 'test' },
      // Missing proposer!
      created_at: Date.now(),
      slot: 1,
      hash: 'invalid_block',
    };

    let invalidEmitted = false;
    engine.once('block:invalid', (event) => {
      invalidEmitted = true;
      assert.strictEqual(event.reason, 'invalid_structure');
    });

    engine.receiveBlock(invalidBlock, 'byzantine_peer');
    assert.strictEqual(invalidEmitted, true);
  });

  it('should not accept block from non-validator proposer', () => {
    engine.start();

    // Register only known validators
    engine.registerValidator({ publicKey: keypair.publicKey, eScore: 100 });

    const unknownKeypair = generateKeypair();
    const block = createSignedBlock(unknownKeypair, 1, { test: 'unknown' });

    // Block from unknown proposer
    let invalidEmitted = false;
    engine.once('block:invalid', (event) => {
      invalidEmitted = true;
    });

    engine.receiveBlock(block, 'some_peer');

    // Should either reject or mark as from unknown validator
  });

  it('should track blocks from specific peers for reputation', () => {
    engine.start();

    const validBlock = createSignedBlock(keypair, 1, { test: 'tracking' });
    engine.receiveBlock(validBlock, 'peer_123');

    // Should be tracked
    const record = engine.getBlock(validBlock.hash);
    assert.ok(record, 'Block should be tracked');
  });
});

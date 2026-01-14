/**
 * Layer 4: φ-BFT Consensus Tests
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  // Voting
  VoteType,
  ConsensusType,
  calculateVoteWeight,
  generateVoteId,
  createVote,
  verifyVote,
  calculateConsensus,
  checkSoftConsensus,
  aggregateVoteRounds,
  // Lockout
  VoterLockout,
  LockoutManager,
  calculateTotalLockout,
  confirmationsForLockout,
  // Proposal
  ProposalAction,
  ProposalStatus,
  generateProposalId,
  createProposal,
  verifyProposal,
  validateProposal,
  addVoteToProposal,
  finalizeProposal,
  createAddDimensionProposal,
  createParameterChangeProposal,
  generateKeypair,
} from '../src/index.js';

import { PHI, PHI_INV, CONSENSUS_THRESHOLD, GOVERNANCE_QUORUM } from '@cynic/core';

describe('Vote Weight Calculation', () => {
  it('should calculate base vote weight', () => {
    const weight = calculateVoteWeight({
      eScore: 100,
      burned: 0,
      uptime: 1,
    });

    assert.strictEqual(weight, 100); // No burn multiplier
  });

  it('should apply burn multiplier', () => {
    const withoutBurn = calculateVoteWeight({
      eScore: 100,
      burned: 0,
      uptime: 1,
    });

    const withBurn = calculateVoteWeight({
      eScore: 100,
      burned: 100,
      uptime: 1,
    });

    assert.ok(withBurn > withoutBurn);
  });

  it('should apply uptime factor', () => {
    const fullUptime = calculateVoteWeight({
      eScore: 100,
      burned: 10,
      uptime: 1,
    });

    const halfUptime = calculateVoteWeight({
      eScore: 100,
      burned: 10,
      uptime: 0.5,
    });

    assert.ok(fullUptime > halfUptime);
    assert.ok(Math.abs(halfUptime - fullUptime * 0.5) < 0.01);
  });

  it('should handle zero eScore', () => {
    const weight = calculateVoteWeight({
      eScore: 0,
      burned: 100,
      uptime: 1,
    });

    assert.strictEqual(weight, 0);
  });
});

describe('Vote Creation', () => {
  let voterKeypair;

  beforeEach(() => {
    voterKeypair = generateKeypair();
  });

  it('should generate unique vote IDs', () => {
    const id1 = generateVoteId();
    const id2 = generateVoteId();
    assert.ok(id1.startsWith('vote_'));
    assert.notStrictEqual(id1, id2);
  });

  it('should create vote with signature', () => {
    const vote = createVote({
      proposalId: 'prop_123',
      vote: VoteType.APPROVE,
      voterPublicKey: voterKeypair.publicKey,
      voterPrivateKey: voterKeypair.privateKey,
      eScore: 80,
      burned: 50,
      uptime: 0.95,
    });

    assert.ok(vote.id.startsWith('vote_'));
    assert.strictEqual(vote.proposal_id, 'prop_123');
    assert.strictEqual(vote.vote, VoteType.APPROVE);
    assert.strictEqual(vote.e_score, 80);
    assert.ok(vote.weight > 0);
    assert.ok(vote.signature);
  });

  it('should verify vote signature', () => {
    const vote = createVote({
      proposalId: 'prop_123',
      vote: VoteType.REJECT,
      voterPublicKey: voterKeypair.publicKey,
      voterPrivateKey: voterKeypair.privateKey,
      eScore: 75,
    });

    const valid = verifyVote(vote);
    assert.strictEqual(valid, true);
  });

  it('should reject tampered vote', () => {
    const vote = createVote({
      proposalId: 'prop_123',
      vote: VoteType.APPROVE,
      voterPublicKey: voterKeypair.publicKey,
      voterPrivateKey: voterKeypair.privateKey,
      eScore: 75,
    });

    // Tamper with vote
    vote.vote = VoteType.REJECT;

    const valid = verifyVote(vote);
    assert.strictEqual(valid, false);
  });

  it('should reject vote without signature', () => {
    const vote = {
      id: 'vote_123',
      proposal_id: 'prop_123',
      vote: VoteType.APPROVE,
      voter: 'ed25519:abc',
    };

    const valid = verifyVote(vote);
    assert.strictEqual(valid, false);
  });
});

describe('Consensus Calculation', () => {
  let keypairs;

  beforeEach(() => {
    keypairs = Array.from({ length: 10 }, () => generateKeypair());
  });

  it('should calculate hard consensus (φ⁻¹ threshold)', () => {
    // Create votes where 70% approve (above 61.8% threshold)
    const votes = keypairs.map((kp, i) =>
      createVote({
        proposalId: 'prop_123',
        vote: i < 7 ? VoteType.APPROVE : VoteType.REJECT,
        voterPublicKey: kp.publicKey,
        voterPrivateKey: kp.privateKey,
        eScore: 75,
      })
    );

    const result = calculateConsensus(votes, ConsensusType.HARD);
    assert.strictEqual(result.reached, true);
    assert.strictEqual(result.result, VoteType.APPROVE);
    assert.ok(result.ratio >= CONSENSUS_THRESHOLD);
  });

  it('should reject below threshold', () => {
    // Create votes where 50% approve (below 61.8% threshold)
    const votes = keypairs.map((kp, i) =>
      createVote({
        proposalId: 'prop_123',
        vote: i < 5 ? VoteType.APPROVE : VoteType.REJECT,
        voterPublicKey: kp.publicKey,
        voterPrivateKey: kp.privateKey,
        eScore: 75,
      })
    );

    const result = calculateConsensus(votes, ConsensusType.HARD);
    assert.strictEqual(result.reached, false);
    assert.strictEqual(result.result, VoteType.REJECT);
  });

  it('should use simple majority for soft consensus', () => {
    // 60% approve (below 61.8% but above 50%)
    const votes = keypairs.slice(0, 5).map((kp, i) =>
      createVote({
        proposalId: 'prop_123',
        vote: i < 3 ? VoteType.APPROVE : VoteType.REJECT,
        voterPublicKey: kp.publicKey,
        voterPrivateKey: kp.privateKey,
        eScore: 75,
      })
    );

    const result = calculateConsensus(votes, ConsensusType.SOFT);
    assert.strictEqual(result.reached, true);
    assert.strictEqual(result.threshold, 0.5);
  });

  it('should weight votes by E-Score and burn', () => {
    // Two voters: one high-weight, one low-weight
    const highWeight = createVote({
      proposalId: 'prop_123',
      vote: VoteType.APPROVE,
      voterPublicKey: keypairs[0].publicKey,
      voterPrivateKey: keypairs[0].privateKey,
      eScore: 100,
      burned: 1000,
    });

    const lowWeight = createVote({
      proposalId: 'prop_123',
      vote: VoteType.REJECT,
      voterPublicKey: keypairs[1].publicKey,
      voterPrivateKey: keypairs[1].privateKey,
      eScore: 10,
      burned: 0,
    });

    const result = calculateConsensus([highWeight, lowWeight]);
    assert.ok(result.approveWeight > result.rejectWeight);
  });

  it('should ignore abstain votes', () => {
    const votes = [
      createVote({
        proposalId: 'prop_123',
        vote: VoteType.APPROVE,
        voterPublicKey: keypairs[0].publicKey,
        voterPrivateKey: keypairs[0].privateKey,
        eScore: 75,
      }),
      createVote({
        proposalId: 'prop_123',
        vote: VoteType.ABSTAIN,
        voterPublicKey: keypairs[1].publicKey,
        voterPrivateKey: keypairs[1].privateKey,
        eScore: 75,
      }),
    ];

    const result = calculateConsensus(votes);
    assert.strictEqual(result.voterCount, 1);
  });

  it('should handle empty votes', () => {
    const result = calculateConsensus([]);
    assert.strictEqual(result.reached, false);
    assert.strictEqual(result.totalWeight, 0);
  });

  it('should check quorum for hard consensus', () => {
    // Less than quorum
    const votes = keypairs.slice(0, 2).map((kp) =>
      createVote({
        proposalId: 'prop_123',
        vote: VoteType.APPROVE,
        voterPublicKey: kp.publicKey,
        voterPrivateKey: kp.privateKey,
        eScore: 75,
      })
    );

    const result = calculateConsensus(votes, ConsensusType.HARD);
    if (GOVERNANCE_QUORUM > 2) {
      assert.strictEqual(result.hasQuorum, false);
      assert.strictEqual(result.reached, false);
    }
  });
});

describe('Soft Consensus', () => {
  it('should require minimum independent sources', () => {
    const observations = [
      { source: 'node1', value: 'x' },
      { source: 'node2', value: 'x' },
      { source: 'node3', value: 'x' },
    ];

    const result = checkSoftConsensus(observations, 3);
    assert.strictEqual(result.reached, true);
    assert.strictEqual(result.sources, 3);
  });

  it('should count unique sources only', () => {
    const observations = [
      { source: 'node1', value: 'x' },
      { source: 'node1', value: 'y' }, // Same source
      { source: 'node2', value: 'x' },
    ];

    const result = checkSoftConsensus(observations, 3);
    assert.strictEqual(result.reached, false);
    assert.strictEqual(result.sources, 2);
  });

  it('should use operator as source fallback', () => {
    const observations = [
      { operator: 'op1' },
      { operator: 'op2' },
      { operator: 'op3' },
    ];

    const result = checkSoftConsensus(observations);
    assert.strictEqual(result.sources, 3);
  });
});

describe('Vote Aggregation', () => {
  let keypairs;

  beforeEach(() => {
    keypairs = Array.from({ length: 5 }, () => generateKeypair());
  });

  it('should aggregate multiple voting rounds', () => {
    const round1 = [
      createVote({
        proposalId: 'prop_123',
        vote: VoteType.APPROVE,
        voterPublicKey: keypairs[0].publicKey,
        voterPrivateKey: keypairs[0].privateKey,
        eScore: 75,
      }),
    ];

    const round2 = [
      createVote({
        proposalId: 'prop_123',
        vote: VoteType.APPROVE,
        voterPublicKey: keypairs[1].publicKey,
        voterPrivateKey: keypairs[1].privateKey,
        eScore: 75,
      }),
    ];

    const result = aggregateVoteRounds([round1, round2]);
    assert.strictEqual(result.voterCount, 2);
  });

  it('should keep latest vote per voter', async () => {
    // First vote: REJECT
    const vote1 = createVote({
      proposalId: 'prop_123',
      vote: VoteType.REJECT,
      voterPublicKey: keypairs[0].publicKey,
      voterPrivateKey: keypairs[0].privateKey,
      eScore: 75,
    });

    // Wait a bit for different timestamp
    await new Promise((resolve) => setTimeout(resolve, 10));

    // Second vote: APPROVE (same voter)
    const vote2 = createVote({
      proposalId: 'prop_123',
      vote: VoteType.APPROVE,
      voterPublicKey: keypairs[0].publicKey,
      voterPrivateKey: keypairs[0].privateKey,
      eScore: 75,
    });

    const result = aggregateVoteRounds([[vote1], [vote2]]);
    assert.strictEqual(result.voterCount, 1);
    assert.strictEqual(result.result, VoteType.APPROVE); // Latest vote wins
  });
});

describe('Voter Lockout', () => {
  let lockout;

  beforeEach(() => {
    lockout = new VoterLockout('voter123');
  });

  it('should record votes', () => {
    const record = lockout.recordVote('block_abc', 100);

    assert.strictEqual(record.blockHash, 'block_abc');
    assert.strictEqual(record.initialSlot, 100);
    assert.strictEqual(record.confirmations, 1);
  });

  it('should increment confirmations on re-vote', () => {
    lockout.recordVote('block_abc', 100);
    lockout.recordVote('block_abc', 101);
    lockout.recordVote('block_abc', 102);

    const votes = lockout.getActiveVotes();
    const record = votes.find((v) => v.blockHash === 'block_abc');

    assert.strictEqual(record.confirmations, 3);
  });

  it('should calculate φⁿ lockout', () => {
    lockout.recordVote('block_abc', 100);
    const slots1 = lockout.getLockoutSlots('block_abc');
    assert.ok(Math.abs(slots1 - PHI) < 0.01);

    lockout.recordVote('block_abc', 101);
    const slots2 = lockout.getLockoutSlots('block_abc');
    assert.ok(Math.abs(slots2 - PHI * PHI) < 0.01);
  });

  it('should check lockout status', () => {
    lockout.recordVote('block_abc', 100);

    // Immediately after vote
    assert.strictEqual(lockout.isLockedOut('block_abc', 100), true);
    assert.strictEqual(lockout.isLockedOut('block_abc', 101), true);

    // After lockout expires (φ¹ ≈ 1.618 slots)
    assert.strictEqual(lockout.isLockedOut('block_abc', 103), false);
  });

  it('should check switch eligibility', () => {
    lockout.recordVote('block_abc', 100);

    const status1 = lockout.canSwitchVote(100);
    assert.strictEqual(status1.canSwitch, false);
    assert.strictEqual(status1.locked.length, 1);

    const status2 = lockout.canSwitchVote(200);
    assert.strictEqual(status2.canSwitch, true);
    assert.strictEqual(status2.unlocked.length, 1);
  });

  it('should prune old votes', () => {
    lockout.recordVote('block_abc', 100);
    lockout.recordVote('block_def', 500);

    lockout.pruneOldVotes(1200, 1000);

    const votes = lockout.getActiveVotes();
    assert.strictEqual(votes.length, 1);
    assert.strictEqual(votes[0].blockHash, 'block_def');
  });
});

describe('Lockout Manager', () => {
  let manager;

  beforeEach(() => {
    manager = new LockoutManager();
  });

  it('should track multiple voters', () => {
    manager.recordVote('voter1', 'block_abc', 100);
    manager.recordVote('voter2', 'block_abc', 100);
    manager.recordVote('voter3', 'block_def', 100);

    const stats = manager.getStats(100);
    assert.strictEqual(stats.totalVoters, 3);
  });

  it('should check lockout across voters', () => {
    manager.recordVote('voter1', 'block_abc', 100);

    assert.strictEqual(manager.isLockedOut('voter1', 'block_abc', 100), true);
    assert.strictEqual(manager.isLockedOut('voter2', 'block_abc', 100), false);
  });

  it('should check switch eligibility', () => {
    manager.recordVote('voter1', 'block_abc', 100);

    const status = manager.canSwitchVote('voter1', 100);
    assert.strictEqual(status.canSwitch, false);

    const status2 = manager.canSwitchVote('voter2', 100);
    assert.strictEqual(status2.canSwitch, true);
  });

  it('should prune all voters', () => {
    manager.recordVote('voter1', 'block_abc', 100);
    manager.recordVote('voter2', 'block_def', 200);

    manager.prune(1500);

    const voter1 = manager.getVoter('voter1');
    assert.strictEqual(voter1.getActiveVotes().length, 0);
  });
});

describe('Lockout Calculations', () => {
  it('should calculate total lockout', () => {
    // Sum of φ¹ + φ² + φ³
    const total = calculateTotalLockout(3);
    const expected = PHI + PHI * PHI + PHI * PHI * PHI;
    assert.ok(Math.abs(total - expected) < 0.01);
  });

  it('should estimate confirmations for lockout', () => {
    const confs = confirmationsForLockout(100);
    const actual = calculateTotalLockout(confs);
    assert.ok(actual >= 100);
  });
});

describe('Proposal Management', () => {
  let keypair;

  beforeEach(() => {
    keypair = generateKeypair();
  });

  it('should generate unique proposal IDs', () => {
    const id1 = generateProposalId();
    const id2 = generateProposalId();
    assert.ok(id1.startsWith('prop_'));
    assert.notStrictEqual(id1, id2);
  });

  it('should create proposal with signature', () => {
    const proposal = createProposal({
      action: ProposalAction.ADD_DIMENSION,
      params: { name: 'INTEGRITY', axiom: 'VERIFY', threshold: 50 },
      description: 'Add INTEGRITY dimension',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    assert.ok(proposal.id.startsWith('prop_'));
    assert.strictEqual(proposal.action, ProposalAction.ADD_DIMENSION);
    assert.strictEqual(proposal.status, ProposalStatus.VOTING);
    assert.ok(proposal.signature);
  });

  it('should verify proposal signature', () => {
    const proposal = createProposal({
      action: ProposalAction.BAN_PEER,
      params: { peerId: 'peer123' },
      description: 'Ban malicious peer',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    const valid = verifyProposal(proposal);
    assert.strictEqual(valid, true);
  });

  it('should reject tampered proposal', () => {
    const proposal = createProposal({
      action: ProposalAction.BAN_PEER,
      params: { peerId: 'peer123' },
      description: 'Ban malicious peer',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    // Tamper
    proposal.params.peerId = 'peer456';

    const valid = verifyProposal(proposal);
    assert.strictEqual(valid, false);
  });

  it('should reject invalid action', () => {
    assert.throws(() => {
      createProposal({
        action: 'INVALID_ACTION',
        params: {},
        description: 'Invalid',
        proposerPublicKey: keypair.publicKey,
        proposerPrivateKey: keypair.privateKey,
      });
    });
  });

  it('should validate ADD_DIMENSION proposal', () => {
    const valid = createProposal({
      action: ProposalAction.ADD_DIMENSION,
      params: { name: 'TEST', axiom: 'PHI', threshold: 50 },
      description: 'Add test dimension',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    const result = validateProposal(valid);
    assert.strictEqual(result.valid, true);

    const invalid = createProposal({
      action: ProposalAction.ADD_DIMENSION,
      params: { axiom: 'INVALID' }, // Missing name, invalid axiom
      description: 'Invalid',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    const result2 = validateProposal(invalid);
    assert.strictEqual(result2.valid, false);
    assert.ok(result2.errors.length > 0);
  });

  it('should validate MODIFY_THRESHOLD proposal', () => {
    const valid = createProposal({
      action: ProposalAction.MODIFY_THRESHOLD,
      params: { threshold: 75 },
      description: 'Modify threshold',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    const result = validateProposal(valid);
    assert.strictEqual(result.valid, true);

    const invalid = createProposal({
      action: ProposalAction.MODIFY_THRESHOLD,
      params: { threshold: 150 }, // Out of range
      description: 'Invalid',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    const result2 = validateProposal(invalid);
    assert.strictEqual(result2.valid, false);
  });

  it('should add votes to proposal', () => {
    const proposal = createProposal({
      action: ProposalAction.PARAMETER_CHANGE,
      params: { parameter: 'X', oldValue: 1, newValue: 2 },
      description: 'Change X',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    const voterKeypair = generateKeypair();
    const vote = createVote({
      proposalId: proposal.id,
      vote: VoteType.APPROVE,
      voterPublicKey: voterKeypair.publicKey,
      voterPrivateKey: voterKeypair.privateKey,
      eScore: 75,
    });

    addVoteToProposal(proposal, vote);
    assert.strictEqual(proposal.votes.length, 1);
  });

  it('should update existing vote', () => {
    const proposal = createProposal({
      action: ProposalAction.PARAMETER_CHANGE,
      params: { parameter: 'X', oldValue: 1, newValue: 2 },
      description: 'Change X',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    const voterKeypair = generateKeypair();

    const vote1 = createVote({
      proposalId: proposal.id,
      vote: VoteType.APPROVE,
      voterPublicKey: voterKeypair.publicKey,
      voterPrivateKey: voterKeypair.privateKey,
      eScore: 75,
    });

    const vote2 = createVote({
      proposalId: proposal.id,
      vote: VoteType.REJECT,
      voterPublicKey: voterKeypair.publicKey,
      voterPrivateKey: voterKeypair.privateKey,
      eScore: 75,
    });

    addVoteToProposal(proposal, vote1);
    addVoteToProposal(proposal, vote2);

    assert.strictEqual(proposal.votes.length, 1);
    assert.strictEqual(proposal.votes[0].vote, VoteType.REJECT);
  });

  it('should finalize passed proposal', () => {
    const proposal = createProposal({
      action: ProposalAction.ADD_PATTERN,
      params: { patternId: 'pat_123' },
      description: 'Force add pattern',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
      votingDurationMs: 1, // Expire immediately
    });

    // Add enough votes to pass
    for (let i = 0; i < GOVERNANCE_QUORUM + 1; i++) {
      const vKp = generateKeypair();
      const vote = createVote({
        proposalId: proposal.id,
        vote: VoteType.APPROVE,
        voterPublicKey: vKp.publicKey,
        voterPrivateKey: vKp.privateKey,
        eScore: 75,
      });
      proposal.votes.push(vote);
    }

    // Wait for voting to end
    setTimeout(() => {
      finalizeProposal(proposal);
      assert.strictEqual(proposal.status, ProposalStatus.PASSED);
    }, 10);
  });

  it('should expire proposal without quorum', () => {
    const proposal = createProposal({
      action: ProposalAction.ADD_PATTERN,
      params: { patternId: 'pat_123' },
      description: 'Force add pattern',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
      votingDurationMs: 1, // Expire immediately
    });

    // Only 1 vote (below quorum)
    const vKp = generateKeypair();
    const vote = createVote({
      proposalId: proposal.id,
      vote: VoteType.APPROVE,
      voterPublicKey: vKp.publicKey,
      voterPrivateKey: vKp.privateKey,
      eScore: 75,
    });
    proposal.votes.push(vote);

    setTimeout(() => {
      finalizeProposal(proposal);
      if (GOVERNANCE_QUORUM > 1) {
        assert.strictEqual(proposal.status, ProposalStatus.EXPIRED);
        assert.strictEqual(proposal.result.reason, 'quorum_not_met');
      }
    }, 10);
  });

  it('should create ADD_DIMENSION helper', () => {
    const proposal = createAddDimensionProposal({
      name: 'CLARITY',
      axiom: 'VERIFY',
      threshold: 60,
      weight: 0.618,
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    assert.strictEqual(proposal.action, ProposalAction.ADD_DIMENSION);
    assert.strictEqual(proposal.params.name, 'CLARITY');
    assert.strictEqual(proposal.params.axiom, 'VERIFY');
  });

  it('should create PARAMETER_CHANGE helper', () => {
    const proposal = createParameterChangeProposal({
      parameter: 'CONSENSUS_THRESHOLD',
      oldValue: 0.618,
      newValue: 0.7,
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });

    assert.strictEqual(proposal.action, ProposalAction.PARAMETER_CHANGE);
    assert.strictEqual(proposal.params.parameter, 'CONSENSUS_THRESHOLD');
  });
});

/**
 * Consensus Engine — Comprehensive Tests
 *
 * Tests for ConsensusEngine, voting mechanics, finality, lockout,
 * slot management, proposals, and phi-aligned thresholds.
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert';

import {
  // Engine
  ConsensusEngine,
  ConsensusState,
  BlockStatus,
  // Voting
  VoteType,
  ConsensusType,
  calculateVoteWeight,
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
  // Finality
  FinalityStatus,
  calculateFinalityProbability,
  estimateTimeToFinality,
  checkForkPossibility,
  FinalityTracker,
  // Slot
  getCurrentSlot,
  getEpochForSlot,
  getSlotIndexInEpoch,
  selectLeader,
  SlotManager,
  SLOT_DURATION_MS,
  SLOTS_PER_EPOCH,
  // Proposal
  ProposalAction,
  ProposalStatus,
  createProposal,
  validateProposal,
  addVoteToProposal,
  finalizeProposal,
  // Crypto
  generateKeypair,
} from '../src/index.js';

import { PHI, PHI_INV, CONSENSUS_THRESHOLD, GOVERNANCE_QUORUM } from '@cynic/core';

// ─────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────

function makeEngine(overrides = {}) {
  const kp = generateKeypair();
  const engine = new ConsensusEngine({
    publicKey: kp.publicKey,
    privateKey: kp.privateKey,
    slotDuration: 400,
    confirmationsForFinality: 3, // low for testing
    ...overrides,
  });
  return { engine, keypair: kp };
}

function makeValidators(count) {
  return Array.from({ length: count }, () => {
    const kp = generateKeypair();
    return { keypair: kp, publicKey: kp.publicKey, eScore: 80, burned: 10, uptime: 1 };
  });
}

// ─────────────────────────────────────────────────────────────────
// 1. ConsensusEngine — lifecycle and state transitions
// ─────────────────────────────────────────────────────────────────

describe('ConsensusEngine — Lifecycle', () => {
  let engine, keypair;

  beforeEach(() => {
    ({ engine, keypair } = makeEngine());
  });

  afterEach(() => {
    if (engine.state !== ConsensusState.STOPPED) engine.stop();
  });

  it('should initialize in INITIALIZING state', () => {
    assert.strictEqual(engine.state, ConsensusState.INITIALIZING);
    assert.strictEqual(engine.currentSlot, 0);
    assert.strictEqual(engine.lastFinalizedSlot, 0);
  });

  it('should transition to PARTICIPATING on start', () => {
    engine.start();
    assert.strictEqual(engine.state, ConsensusState.PARTICIPATING);
    assert.ok(engine.genesisTime > 0);
  });

  it('should transition to STOPPED on stop', () => {
    engine.start();
    engine.stop();
    assert.strictEqual(engine.state, ConsensusState.STOPPED);
    assert.strictEqual(engine.slotTimer, null);
  });

  it('should not start if already PARTICIPATING', () => {
    engine.start();
    const genesisTime = engine.genesisTime;
    engine.start(); // second call should be no-op
    assert.strictEqual(engine.genesisTime, genesisTime);
  });

  it('should emit consensus:started event', () => {
    let emitted = null;
    engine.on('consensus:started', (data) => { emitted = data; });
    engine.start();
    assert.ok(emitted);
    assert.ok(typeof emitted.slot === 'number');
    assert.ok(typeof emitted.genesisTime === 'number');
  });

  it('should emit consensus:stopped event', () => {
    engine.start();
    let stopped = false;
    engine.on('consensus:stopped', () => { stopped = true; });
    engine.stop();
    assert.strictEqual(stopped, true);
  });
});

// ─────────────────────────────────────────────────────────────────
// 2. ConsensusEngine — validator management
// ─────────────────────────────────────────────────────────────────

describe('ConsensusEngine — Validators', () => {
  let engine;

  beforeEach(() => {
    ({ engine } = makeEngine());
  });

  afterEach(() => {
    if (engine.state !== ConsensusState.STOPPED) engine.stop();
  });

  it('should register validators and compute weight', () => {
    engine.registerValidator({ publicKey: 'v1', eScore: 100, burned: 50, uptime: 1 });
    const v = engine.validators.get('v1');
    assert.ok(v);
    assert.ok(v.weight > 0);
    assert.strictEqual(v.eScore, 100);
  });

  it('should emit validator:registered event', () => {
    let eventData = null;
    engine.on('validator:registered', (d) => { eventData = d; });
    engine.registerValidator({ publicKey: 'v1', eScore: 80 });
    assert.ok(eventData);
    assert.strictEqual(eventData.publicKey, 'v1');
  });

  it('should remove validators', () => {
    engine.registerValidator({ publicKey: 'v1', eScore: 80 });
    engine.removeValidator('v1');
    assert.strictEqual(engine.validators.has('v1'), false);
  });

  it('should emit validator:removed event', () => {
    engine.registerValidator({ publicKey: 'v1', eScore: 80 });
    let removed = null;
    engine.on('validator:removed', (d) => { removed = d; });
    engine.removeValidator('v1');
    assert.strictEqual(removed.publicKey, 'v1');
  });
});

// ─────────────────────────────────────────────────────────────────
// 3. ConsensusEngine — block proposal
// ─────────────────────────────────────────────────────────────────

describe('ConsensusEngine — Block Proposal', () => {
  let engine, keypair;

  beforeEach(() => {
    ({ engine, keypair } = makeEngine());
    engine.start();
    engine.registerValidator({ publicKey: keypair.publicKey, eScore: 100 });
  });

  afterEach(() => {
    if (engine.state !== ConsensusState.STOPPED) engine.stop();
  });

  it('should propose a block and track it', () => {
    const block = { hash: 'block_A', slot: 1, proposer: keypair.publicKey, data: 'x' };
    const record = engine.proposeBlock(block);
    assert.strictEqual(record.hash, 'block_A');
    assert.strictEqual(record.status, BlockStatus.PROPOSED);
    assert.strictEqual(engine.blocks.size, 1);
  });

  it('should emit block:proposed event', () => {
    let eventData = null;
    engine.on('block:proposed', (d) => { eventData = d; });
    engine.proposeBlock({ hash: 'block_B', slot: 1, proposer: keypair.publicKey });
    assert.ok(eventData);
    assert.strictEqual(eventData.blockHash, 'block_B');
  });

  it('should self-vote APPROVE on own proposed block', () => {
    const block = { hash: 'block_C', slot: 1, proposer: keypair.publicKey };
    engine.proposeBlock(block);
    const record = engine.getBlock('block_C');
    // should have at least the self-vote
    assert.ok(record.votes.size >= 1);
    assert.ok(record.approveWeight > 0);
  });

  it('should throw when proposing without starting', () => {
    const { engine: fresh } = makeEngine();
    assert.throws(
      () => fresh.proposeBlock({ hash: 'fail', slot: 0, proposer: 'x' }),
      /Not participating/
    );
  });

  it('should increment blocksProposed stat', () => {
    engine.proposeBlock({ hash: 'stat1', slot: 1, proposer: keypair.publicKey });
    engine.proposeBlock({ hash: 'stat2', slot: 2, proposer: keypair.publicKey });
    assert.strictEqual(engine.stats.blocksProposed, 2);
  });

  it('should track blocks by slot', () => {
    engine.proposeBlock({ hash: 'slot_blk', slot: 5, proposer: keypair.publicKey });
    const blocks = engine.getBlocksAtSlot(5);
    assert.strictEqual(blocks.length, 1);
    assert.strictEqual(blocks[0].hash, 'slot_blk');
  });
});

// ─────────────────────────────────────────────────────────────────
// 4. ConsensusEngine — receiving blocks + equivocation
// ─────────────────────────────────────────────────────────────────

describe('ConsensusEngine — Receive Block & Equivocation', () => {
  let engine, keypair;

  beforeEach(() => {
    ({ engine, keypair } = makeEngine());
    engine.start();
    engine.registerValidator({ publicKey: keypair.publicKey, eScore: 100 });
  });

  afterEach(() => {
    if (engine.state !== ConsensusState.STOPPED) engine.stop();
  });

  it('should receive a valid block from network', () => {
    const block = { hash: 'net_blk', slot: 2, proposer: 'other_node' };
    engine.receiveBlock(block, 'peerX');
    const record = engine.getBlock('net_blk');
    assert.ok(record);
    assert.strictEqual(record.status, BlockStatus.VOTING);
    assert.strictEqual(record.receivedFrom, 'peerX');
  });

  it('should ignore duplicate blocks', () => {
    const block = { hash: 'dup_blk', slot: 3, proposer: 'other' };
    engine.receiveBlock(block, 'peer1');
    engine.receiveBlock(block, 'peer2'); // same hash
    assert.strictEqual(engine.blocks.size, 1);
  });

  it('should reject invalid structure blocks (no proposer)', () => {
    let invalidEvt = null;
    engine.on('block:invalid', (e) => { invalidEvt = e; });
    engine.receiveBlock({ hash: 'bad', slot: 1 }, 'peer_bad');
    assert.ok(invalidEvt);
    assert.strictEqual(invalidEvt.reason, 'invalid_structure');
  });

  it('should reject invalid structure blocks (non-numeric slot)', () => {
    let invalidEvt = null;
    engine.on('block:invalid', (e) => { invalidEvt = e; });
    engine.receiveBlock({ hash: 'bad2', slot: 'not_num', proposer: 'p' }, 'peer');
    assert.ok(invalidEvt);
  });

  it('should ignore blocks older than lastFinalizedSlot', () => {
    engine.lastFinalizedSlot = 100;
    engine.receiveBlock({ hash: 'old', slot: 50, proposer: 'p' }, 'peer');
    assert.strictEqual(engine.getBlock('old'), null);
  });

  it('should detect equivocation: same proposer, same slot, different block', () => {
    let equivocationEvt = null;
    engine.on('block:equivocation', (e) => { equivocationEvt = e; });

    const proposer = 'equivocator';
    engine.receiveBlock({ hash: 'eq_A', slot: 10, proposer }, 'peer1');
    engine.receiveBlock({ hash: 'eq_B', slot: 10, proposer }, 'peer2');

    assert.ok(equivocationEvt);
    assert.strictEqual(equivocationEvt.proposer, proposer);
    assert.strictEqual(equivocationEvt.slot, 10);
    // second block should NOT be stored
    assert.strictEqual(engine.getBlock('eq_B'), null);
  });
});

// ─────────────────────────────────────────────────────────────────
// 5. ConsensusEngine — vote processing & consensus checking
// ─────────────────────────────────────────────────────────────────

describe('ConsensusEngine — Vote Processing & Consensus', () => {
  let engine, keypair;

  beforeEach(() => {
    ({ engine, keypair } = makeEngine());
    engine.start();
  });

  afterEach(() => {
    if (engine.state !== ConsensusState.STOPPED) engine.stop();
  });

  it('should reach CONFIRMED status when supermajority approves', () => {
    // Register 3 validators with equal weight
    const validators = makeValidators(3);
    validators.forEach((v) => engine.registerValidator(v));
    engine.registerValidator({ publicKey: keypair.publicKey, eScore: 80, burned: 10, uptime: 1 });

    // Propose a block
    const block = { hash: 'consensus_blk', slot: 1, proposer: keypair.publicKey };
    engine.proposeBlock(block);

    // Create approve votes from all validators (simulating supermajority)
    for (const v of validators) {
      const vote = createVote({
        proposalId: 'consensus_blk',
        vote: VoteType.APPROVE,
        voterPublicKey: v.keypair.publicKey,
        voterPrivateKey: v.keypair.privateKey,
        eScore: v.eScore,
        burned: v.burned,
        uptime: v.uptime,
      });
      vote.block_hash = 'consensus_blk';
      vote.slot = 0;
      engine.receiveVote(vote, 'some_peer');
    }

    const record = engine.getBlock('consensus_blk');
    // With all validators approving, should be CONFIRMED
    assert.strictEqual(record.status, BlockStatus.CONFIRMED);
  });

  it('should reject block when reject weight exceeds (1 - threshold)', () => {
    // 4 validators: 1 approves, 3 reject
    const validators = makeValidators(3);
    validators.forEach((v) => engine.registerValidator(v));
    engine.registerValidator({ publicKey: keypair.publicKey, eScore: 80, burned: 10, uptime: 1 });

    const block = { hash: 'rej_blk', slot: 1, proposer: keypair.publicKey };
    engine.proposeBlock(block); // self-vote approve

    for (const v of validators) {
      const vote = createVote({
        proposalId: 'rej_blk',
        vote: VoteType.REJECT,
        voterPublicKey: v.keypair.publicKey,
        voterPrivateKey: v.keypair.privateKey,
        eScore: v.eScore,
        burned: v.burned,
        uptime: v.uptime,
      });
      vote.block_hash = 'rej_blk';
      vote.slot = 0;
      engine.receiveVote(vote, 'peer');
    }

    const record = engine.getBlock('rej_blk');
    assert.strictEqual(record.status, BlockStatus.REJECTED);
  });

  it('should silently ignore duplicate same-type votes from same voter', () => {
    engine.registerValidator({ publicKey: keypair.publicKey, eScore: 100 });
    const v = makeValidators(1)[0];
    engine.registerValidator(v);

    const block = { hash: 'dup_vote_blk', slot: 1, proposer: keypair.publicKey };
    engine.proposeBlock(block);

    const vote1 = createVote({
      proposalId: 'dup_vote_blk',
      vote: VoteType.APPROVE,
      voterPublicKey: v.keypair.publicKey,
      voterPrivateKey: v.keypair.privateKey,
      eScore: v.eScore,
    });
    vote1.block_hash = 'dup_vote_blk';
    vote1.slot = 0;

    const vote2 = createVote({
      proposalId: 'dup_vote_blk',
      vote: VoteType.APPROVE,
      voterPublicKey: v.keypair.publicKey,
      voterPrivateKey: v.keypair.privateKey,
      eScore: v.eScore,
    });
    vote2.block_hash = 'dup_vote_blk';
    vote2.slot = 0;

    let addedCount = 0;
    engine.on('vote:added', () => { addedCount++; });

    engine.receiveVote(vote1, 'p1');
    const weightAfterFirst = engine.getBlock('dup_vote_blk').approveWeight;

    engine.receiveVote(vote2, 'p2');
    const weightAfterSecond = engine.getBlock('dup_vote_blk').approveWeight;

    // Weight should not have doubled — duplicate vote silently ignored
    assert.strictEqual(weightAfterFirst, weightAfterSecond);
    // Only 1 vote:added event from receiveVote (the self-vote in proposeBlock also adds one)
    assert.strictEqual(addedCount, 1);
  });

  it('should queue votes for unknown blocks', () => {
    engine.registerValidator({ publicKey: keypair.publicKey, eScore: 100 });
    const v = makeValidators(1)[0];
    engine.registerValidator(v);

    const vote = createVote({
      proposalId: 'future_blk',
      vote: VoteType.APPROVE,
      voterPublicKey: v.keypair.publicKey,
      voterPrivateKey: v.keypair.privateKey,
      eScore: v.eScore,
    });
    vote.block_hash = 'future_blk';
    vote.slot = 0;

    engine.receiveVote(vote, 'p');
    assert.strictEqual(engine.pendingVotes.length, 1);
  });

  it('should enforce maxPendingVotes limit', () => {
    const { engine: eng } = makeEngine({ maxPendingVotes: 2 });
    eng.start();

    for (let i = 0; i < 5; i++) {
      const v = makeValidators(1)[0];
      const vote = createVote({
        proposalId: `unknown_${i}`,
        vote: VoteType.APPROVE,
        voterPublicKey: v.keypair.publicKey,
        voterPrivateKey: v.keypair.privateKey,
        eScore: v.eScore,
      });
      vote.block_hash = `unknown_${i}`;
      vote.slot = 0;
      eng.receiveVote(vote, 'p');
    }

    assert.ok(eng.pendingVotes.length <= 2);
    eng.stop();
  });
});

// ─────────────────────────────────────────────────────────────────
// 6. ConsensusEngine — state sync (import/export)
// ─────────────────────────────────────────────────────────────────

describe('ConsensusEngine — State Sync', () => {
  let engine, keypair;

  beforeEach(() => {
    ({ engine, keypair } = makeEngine());
    engine.start();
  });

  afterEach(() => {
    if (engine.state !== ConsensusState.STOPPED) engine.stop();
  });

  it('should export finalized blocks', () => {
    // Manually insert a finalized block
    engine.blocks.set('fin_blk', {
      hash: 'fin_blk',
      slot: 5,
      proposer: 'p',
      block: { data: 'x' },
      status: BlockStatus.FINALIZED,
      approveWeight: 100,
      confirmations: 32,
      finalizedAt: Date.now(),
    });
    engine.lastFinalizedSlot = 5;

    const exported = engine.exportState(0, 50);
    assert.strictEqual(exported.blocks.length, 1);
    assert.strictEqual(exported.blocks[0].hash, 'fin_blk');
    assert.strictEqual(exported.latestSlot, 5);
  });

  it('should import state from a peer', () => {
    const state = {
      blocks: [
        { hash: 'sync_A', slot: 10, proposer: 'p', block: { data: 'a' }, approveWeight: 80 },
        { hash: 'sync_B', slot: 20, proposer: 'p', block: { data: 'b' }, approveWeight: 90 },
      ],
      latestSlot: 20,
      genesisTime: Date.now() - 100000,
    };

    const result = engine.importState(state);
    assert.strictEqual(result.imported, 2);
    assert.strictEqual(result.total, 2);
    assert.strictEqual(engine.lastFinalizedSlot, 20);

    // Imported blocks should be FINALIZED
    const blk = engine.getBlock('sync_A');
    assert.strictEqual(blk.status, BlockStatus.FINALIZED);
  });

  it('should skip duplicate blocks on import', () => {
    engine.blocks.set('existing', {
      hash: 'existing', slot: 5, status: BlockStatus.FINALIZED,
      votes: new Map(), proposer: 'p', block: {},
    });

    const state = {
      blocks: [{ hash: 'existing', slot: 5, proposer: 'p', block: {} }],
      latestSlot: 5,
    };

    const result = engine.importState(state);
    assert.strictEqual(result.imported, 0);
  });

  it('should handle invalid import state gracefully', () => {
    const result = engine.importState({ blocks: null });
    assert.strictEqual(result.imported, 0);
    assert.ok(result.error);
  });

  it('should detect sync need', () => {
    engine.lastFinalizedSlot = 10;
    assert.strictEqual(engine.needsSync(20), true);
    assert.strictEqual(engine.needsSync(5), false);
    assert.strictEqual(engine.needsSync(10), false);
  });
});

// ─────────────────────────────────────────────────────────────────
// 7. ConsensusEngine — getState / getStats
// ─────────────────────────────────────────────────────────────────

describe('ConsensusEngine — State & Stats', () => {
  let engine, keypair;

  beforeEach(() => {
    ({ engine, keypair } = makeEngine());
    engine.start();
    engine.registerValidator({ publicKey: keypair.publicKey, eScore: 100 });
  });

  afterEach(() => {
    if (engine.state !== ConsensusState.STOPPED) engine.stop();
  });

  it('should return correct state summary', () => {
    engine.proposeBlock({ hash: 'st_blk', slot: 1, proposer: keypair.publicKey });
    const state = engine.getState();
    assert.strictEqual(state.state, ConsensusState.PARTICIPATING);
    assert.strictEqual(state.validators, 1);
    assert.ok(state.pendingBlocks >= 0);
  });

  it('should return stats with lockout info', () => {
    const stats = engine.getStats();
    assert.ok('totalVoters' in stats);
    assert.ok('lockedVoters' in stats);
    assert.ok('freeVoters' in stats);
    assert.strictEqual(stats.state, ConsensusState.PARTICIPATING);
  });
});

// ─────────────────────────────────────────────────────────────────
// 8. Vote weight — phi-aligned thresholds
// ─────────────────────────────────────────────────────────────────

describe('Vote Weight — phi-aligned', () => {
  it('should use burn multiplier = log_phi(burned + 1)', () => {
    const w0 = calculateVoteWeight({ eScore: 100, burned: 0, uptime: 1 });
    const w1 = calculateVoteWeight({ eScore: 100, burned: 100, uptime: 1 });
    const w2 = calculateVoteWeight({ eScore: 100, burned: 1000, uptime: 1 });
    assert.ok(w1 > w0, 'burning tokens should increase weight');
    assert.ok(w2 > w1, 'more burn = more weight');
  });

  it('should cap burn multiplier at minimum 1', () => {
    // burned = 0 -> log_phi(1) = 0, but max(0, 1) = 1
    const w = calculateVoteWeight({ eScore: 50, burned: 0, uptime: 1 });
    assert.strictEqual(w, 50); // eScore * 1 * 1
  });

  it('should scale linearly with uptime', () => {
    const full = calculateVoteWeight({ eScore: 100, burned: 50, uptime: 1 });
    const half = calculateVoteWeight({ eScore: 100, burned: 50, uptime: 0.5 });
    assert.ok(Math.abs(half - full * 0.5) < 0.01);
  });
});

// ─────────────────────────────────────────────────────────────────
// 9. Consensus calculation — threshold = phi^-1
// ─────────────────────────────────────────────────────────────────

describe('Consensus Calculation — phi thresholds', () => {
  it('should require 61.8% for hard consensus (phi^-1)', () => {
    assert.ok(Math.abs(CONSENSUS_THRESHOLD - PHI_INV) < 0.001);
  });

  it('should reach consensus at exactly threshold ratio', () => {
    // Generate enough voters to satisfy quorum
    const kps = Array.from({ length: Math.max(GOVERNANCE_QUORUM + 1, 10) }, () => generateKeypair());
    const approveCount = Math.ceil(kps.length * 0.62); // just above 61.8%
    const votes = kps.map((kp, i) =>
      createVote({
        proposalId: 'p',
        vote: i < approveCount ? VoteType.APPROVE : VoteType.REJECT,
        voterPublicKey: kp.publicKey,
        voterPrivateKey: kp.privateKey,
        eScore: 50, // equal weight
      })
    );

    const result = calculateConsensus(votes, ConsensusType.HARD);
    assert.strictEqual(result.reached, true);
  });

  it('should NOT reach consensus at 60% (below phi^-1)', () => {
    const kps = Array.from({ length: 10 }, () => generateKeypair());
    const votes = kps.map((kp, i) =>
      createVote({
        proposalId: 'p',
        vote: i < 6 ? VoteType.APPROVE : VoteType.REJECT,
        voterPublicKey: kp.publicKey,
        voterPrivateKey: kp.privateKey,
        eScore: 50,
      })
    );

    const result = calculateConsensus(votes, ConsensusType.HARD);
    // 60% < 61.8% -> not reached
    assert.strictEqual(result.reached, false);
  });

  it('should use 50% threshold for soft consensus', () => {
    const kps = Array.from({ length: 4 }, () => generateKeypair());
    const votes = kps.map((kp, i) =>
      createVote({
        proposalId: 'p',
        vote: i < 3 ? VoteType.APPROVE : VoteType.REJECT,
        voterPublicKey: kp.publicKey,
        voterPrivateKey: kp.privateKey,
        eScore: 50,
      })
    );
    const result = calculateConsensus(votes, ConsensusType.SOFT);
    assert.strictEqual(result.threshold, 0.5);
    assert.strictEqual(result.reached, true);
  });
});

// ─────────────────────────────────────────────────────────────────
// 10. Lockout — phi exponential lockout
// ─────────────────────────────────────────────────────────────────

describe('Lockout — phi exponential', () => {
  it('should compute lockout as phi^n', () => {
    const vl = new VoterLockout('voter');
    vl.recordVote('b1', 100);
    assert.ok(Math.abs(vl.getLockoutSlots('b1') - PHI) < 0.001);

    vl.recordVote('b1', 101); // 2 confirmations
    assert.ok(Math.abs(vl.getLockoutSlots('b1') - Math.pow(PHI, 2)) < 0.001);

    vl.recordVote('b1', 102); // 3 confirmations
    assert.ok(Math.abs(vl.getLockoutSlots('b1') - Math.pow(PHI, 3)) < 0.001);
  });

  it('should be locked out within lockout window', () => {
    const vl = new VoterLockout('voter');
    vl.recordVote('b1', 100);
    // phi^1 ~= 1.618 => locked at slot 100 and 101
    assert.strictEqual(vl.isLockedOut('b1', 100), true);
    assert.strictEqual(vl.isLockedOut('b1', 101), true);
  });

  it('should be unlocked after lockout expires', () => {
    const vl = new VoterLockout('voter');
    vl.recordVote('b1', 100);
    // phi^1 ~= 1.618 => at slot 102 (2 slots later), unlocked
    assert.strictEqual(vl.isLockedOut('b1', 102), false);
  });

  it('should track independent lockouts per block', () => {
    const vl = new VoterLockout('voter');
    vl.recordVote('b1', 100);
    vl.recordVote('b2', 105);

    assert.strictEqual(vl.isLockedOut('b1', 102), false); // expired
    assert.strictEqual(vl.isLockedOut('b2', 106), true);  // still locked
  });

  it('calculateTotalLockout should sum phi^i from 1 to n', () => {
    const total = calculateTotalLockout(4);
    const expected = PHI + Math.pow(PHI, 2) + Math.pow(PHI, 3) + Math.pow(PHI, 4);
    assert.ok(Math.abs(total - expected) < 0.01);
  });

  it('confirmationsForLockout should find correct confirmations', () => {
    const target = 50;
    const confs = confirmationsForLockout(target);
    const actual = calculateTotalLockout(confs);
    assert.ok(actual >= target);
    // And one less should be below target
    if (confs > 1) {
      assert.ok(calculateTotalLockout(confs - 1) < target);
    }
  });
});

// ─────────────────────────────────────────────────────────────────
// 11. LockoutManager
// ─────────────────────────────────────────────────────────────────

describe('LockoutManager', () => {
  let mgr;

  beforeEach(() => {
    mgr = new LockoutManager();
  });

  it('should track lockout across multiple voters', () => {
    mgr.recordVote('voter1', 'blk', 100);
    mgr.recordVote('voter2', 'blk', 100);

    assert.strictEqual(mgr.isLockedOut('voter1', 'blk', 100), true);
    assert.strictEqual(mgr.isLockedOut('voter2', 'blk', 100), true);
    assert.strictEqual(mgr.isLockedOut('voter3', 'blk', 100), false);
  });

  it('should report canSwitchVote correctly', () => {
    mgr.recordVote('v1', 'blk', 100);
    const locked = mgr.canSwitchVote('v1', 100);
    assert.strictEqual(locked.canSwitch, false);
    assert.strictEqual(locked.locked.length, 1);

    const unlocked = mgr.canSwitchVote('v1', 200);
    assert.strictEqual(unlocked.canSwitch, true);
  });

  it('should report stats', () => {
    mgr.recordVote('v1', 'blk', 100);
    mgr.recordVote('v2', 'blk', 100);
    const stats = mgr.getStats(100);
    assert.strictEqual(stats.totalVoters, 2);
    assert.strictEqual(stats.lockedVoters, 2);
    assert.strictEqual(stats.freeVoters, 0);
  });
});

// ─────────────────────────────────────────────────────────────────
// 12. Finality — probability and status transitions
// ─────────────────────────────────────────────────────────────────

describe('Finality — Probability', () => {
  it('should return PENDING when below consensus threshold', () => {
    const r = calculateFinalityProbability({
      approveRatio: 0.3,
      confirmations: 10,
      totalValidators: 100,
      votedValidators: 100,
    });
    assert.strictEqual(r.status, FinalityStatus.PENDING);
  });

  it('should return OPTIMISTIC just above threshold', () => {
    const r = calculateFinalityProbability({
      approveRatio: 0.62,
      confirmations: 1,
      totalValidators: 100,
      votedValidators: 100,
    });
    assert.strictEqual(r.status, FinalityStatus.OPTIMISTIC);
    assert.ok(r.probability > 0.9);
  });

  it('should return DETERMINISTIC at full approval + high confirmations', () => {
    const r = calculateFinalityProbability({
      approveRatio: 1.0,
      confirmations: 32,
      totalValidators: 100,
      votedValidators: 100,
    });
    assert.strictEqual(r.status, FinalityStatus.DETERMINISTIC);
    assert.ok(r.probability >= 0.9999);
  });

  it('should penalize low participation', () => {
    const full = calculateFinalityProbability({
      approveRatio: 0.7,
      confirmations: 5,
      totalValidators: 100,
      votedValidators: 100,
    });
    const half = calculateFinalityProbability({
      approveRatio: 0.7,
      confirmations: 5,
      totalValidators: 100,
      votedValidators: 50,
    });
    assert.ok(half.probability < full.probability);
  });

  it('should include lockout slots in result', () => {
    const r = calculateFinalityProbability({
      approveRatio: 0.7,
      confirmations: 5,
      totalValidators: 50,
      votedValidators: 50,
    });
    const expectedLockout = calculateTotalLockout(5);
    assert.ok(Math.abs(r.lockoutSlots - expectedLockout) < 0.01);
  });
});

// ─────────────────────────────────────────────────────────────────
// 13. Finality — time estimation & fork possibility
// ─────────────────────────────────────────────────────────────────

describe('Finality — Estimation & Forks', () => {
  it('should estimate time to finality at 99% target', () => {
    const est = estimateTimeToFinality({
      currentConfirmations: 5,
      targetProbability: 0.99,
      slotDurationMs: 400,
    });
    assert.strictEqual(est.targetConfirmations, 13);
    assert.strictEqual(est.remainingConfirmations, 8);
    assert.strictEqual(est.estimatedMs, 8 * 400);
  });

  it('should return 0 remaining when already past target', () => {
    const est = estimateTimeToFinality({
      currentConfirmations: 40,
      targetProbability: 0.9999,
    });
    assert.strictEqual(est.remainingConfirmations, 0);
    assert.strictEqual(est.estimatedMs, 0);
  });

  it('should detect fork possibility when available weight is sufficient', () => {
    const r = checkForkPossibility({
      lockedWeight: 30,
      totalWeight: 100,
      currentSlot: 10,
      blockSlot: 5,
      confirmations: 2,
    });
    // available = 70 >= 61.8 required
    assert.strictEqual(r.forkPossible, true);
    assert.strictEqual(r.availableWeight, 70);
  });

  it('should detect fork impossibility when too much weight is locked', () => {
    const r = checkForkPossibility({
      lockedWeight: 50,
      totalWeight: 100,
      currentSlot: 100,
      blockSlot: 5,
      confirmations: 20,
    });
    // available = 50 < 61.8
    assert.strictEqual(r.forkPossible, false);
  });
});

// ─────────────────────────────────────────────────────────────────
// 14. FinalityTracker
// ─────────────────────────────────────────────────────────────────

describe('FinalityTracker', () => {
  let tracker;

  beforeEach(() => {
    tracker = new FinalityTracker();
  });

  it('should track and update finality for blocks', () => {
    const result = tracker.update('blk1', {
      approveRatio: 0.7,
      confirmations: 5,
      totalValidators: 100,
      votedValidators: 100,
    });
    assert.ok(result.probability > 0);
    assert.ok(tracker.get('blk1'));
  });

  it('should track deterministically final blocks', () => {
    tracker.update('blk2', {
      approveRatio: 1.0,
      confirmations: 32,
      totalValidators: 100,
      votedValidators: 100,
    });
    assert.strictEqual(tracker.isFinal('blk2'), true);
    assert.strictEqual(tracker.isFinal('unknown'), false);
  });

  it('should accumulate history entries', () => {
    tracker.update('blk3', { approveRatio: 0.5, confirmations: 1, totalValidators: 10, votedValidators: 10 });
    tracker.update('blk3', { approveRatio: 0.6, confirmations: 2, totalValidators: 10, votedValidators: 10 });
    tracker.update('blk3', { approveRatio: 0.7, confirmations: 3, totalValidators: 10, votedValidators: 10 });

    const info = tracker.blocks.get('blk3');
    assert.strictEqual(info.history.length, 3);
  });

  it('should prune non-finalized old entries', () => {
    tracker.update('old_blk', { approveRatio: 0.5, confirmations: 1, totalValidators: 10, votedValidators: 10 });
    // Force the lastUpdated to be old
    tracker.blocks.get('old_blk').lastUpdated = Date.now() - 7200000; // 2 hours ago

    tracker.prune(3600000); // max age 1 hour
    assert.strictEqual(tracker.get('old_blk'), null);
  });

  it('should return stats', () => {
    tracker.update('s1', { approveRatio: 0.5, confirmations: 1, totalValidators: 10, votedValidators: 10 });
    tracker.update('s2', { approveRatio: 1.0, confirmations: 32, totalValidators: 10, votedValidators: 10 });

    const stats = tracker.getStats();
    assert.strictEqual(stats.total, 2);
    assert.strictEqual(stats.finalized, 1);
    assert.ok(stats.avgProbability > 0);
  });

  it('should enforce maxFinalizedBlocks with FIFO eviction', () => {
    const small = new FinalityTracker({ maxFinalizedBlocks: 2 });
    small.update('f1', { approveRatio: 1.0, confirmations: 32, totalValidators: 10, votedValidators: 10 });
    small.update('f2', { approveRatio: 1.0, confirmations: 32, totalValidators: 10, votedValidators: 10 });
    small.update('f3', { approveRatio: 1.0, confirmations: 32, totalValidators: 10, votedValidators: 10 });

    assert.strictEqual(small.finalizedBlocks.size, 2);
    // f1 should be evicted (FIFO)
    assert.strictEqual(small.isFinal('f1'), false);
    assert.strictEqual(small.isFinal('f3'), true);
  });
});

// ─────────────────────────────────────────────────────────────────
// 15. Slot Management
// ─────────────────────────────────────────────────────────────────

describe('Slot Management', () => {
  const genesis = 1700000000000;

  it('should calculate slot from timestamp', () => {
    assert.strictEqual(getCurrentSlot(genesis, genesis + 0), 0);
    assert.strictEqual(getCurrentSlot(genesis, genesis + 399), 0);
    assert.strictEqual(getCurrentSlot(genesis, genesis + 400), 1);
    assert.strictEqual(getCurrentSlot(genesis, genesis + 800), 2);
  });

  it('should return 0 for time before genesis', () => {
    assert.strictEqual(getCurrentSlot(genesis, genesis - 1000), 0);
  });

  it('should compute epoch from slot', () => {
    assert.strictEqual(getEpochForSlot(0), 0);
    assert.strictEqual(getEpochForSlot(SLOTS_PER_EPOCH - 1), 0);
    assert.strictEqual(getEpochForSlot(SLOTS_PER_EPOCH), 1);
  });

  it('should compute slot index within epoch', () => {
    assert.strictEqual(getSlotIndexInEpoch(0), 0);
    assert.strictEqual(getSlotIndexInEpoch(SLOTS_PER_EPOCH), 0);
    assert.strictEqual(getSlotIndexInEpoch(SLOTS_PER_EPOCH + 5), 5);
  });

  it('should select leader deterministically for same slot', () => {
    const validators = [
      { id: 'v1', weight: 50 },
      { id: 'v2', weight: 50 },
      { id: 'v3', weight: 50 },
    ];
    const leader1 = selectLeader(42, validators);
    const leader2 = selectLeader(42, validators);
    assert.strictEqual(leader1, leader2);
  });

  it('should return null for empty validator set', () => {
    assert.strictEqual(selectLeader(0, []), null);
    assert.strictEqual(selectLeader(0, null), null);
  });

  it('should create SlotManager and get slot info', () => {
    const mgr = new SlotManager({ genesisTime: Date.now() - 2000 });
    mgr.setValidators([{ id: 'v1', weight: 100 }]);
    const info = mgr.getSlotInfo(0);
    assert.strictEqual(info.slot, 0);
    assert.strictEqual(info.epoch, 0);
    assert.strictEqual(info.isEpochStart, true);
    mgr.stop();
  });
});

// ─────────────────────────────────────────────────────────────────
// 16. Proposal validation
// ─────────────────────────────────────────────────────────────────

describe('Proposal — Validation', () => {
  let keypair;

  beforeEach(() => {
    keypair = generateKeypair();
  });

  it('should reject ADD_DIMENSION without name', () => {
    const p = createProposal({
      action: ProposalAction.ADD_DIMENSION,
      params: { axiom: 'PHI' },
      description: 'test',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });
    const result = validateProposal(p);
    assert.strictEqual(result.valid, false);
    assert.ok(result.errors.some((e) => e.includes('name')));
  });

  it('should reject BAN_PEER without peerId', () => {
    const p = createProposal({
      action: ProposalAction.BAN_PEER,
      params: {},
      description: 'ban',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });
    const result = validateProposal(p);
    assert.strictEqual(result.valid, false);
    assert.ok(result.errors.some((e) => e.includes('peerId')));
  });

  it('should reject MODIFY_THRESHOLD out of range', () => {
    const p = createProposal({
      action: ProposalAction.MODIFY_THRESHOLD,
      params: { threshold: 200 },
      description: 'bad',
      proposerPublicKey: keypair.publicKey,
      proposerPrivateKey: keypair.privateKey,
    });
    const result = validateProposal(p);
    assert.strictEqual(result.valid, false);
  });

  it('should reject invalid action', () => {
    assert.throws(() => {
      createProposal({
        action: 'NONEXISTENT',
        params: {},
        description: 'fail',
        proposerPublicKey: keypair.publicKey,
        proposerPrivateKey: keypair.privateKey,
      });
    });
  });
});

// ─────────────────────────────────────────────────────────────────
// 17. ConsensusEngine — block pruning
// ─────────────────────────────────────────────────────────────────

describe('ConsensusEngine — Block Pruning', () => {
  it('should prune old blocks after finalization', () => {
    const { engine, keypair } = makeEngine({ maxBlockHistory: 5 });
    engine.start();

    // Insert blocks at various slots
    for (let i = 0; i < 10; i++) {
      engine.blocks.set(`blk_${i}`, {
        hash: `blk_${i}`, slot: i, proposer: 'p',
        status: BlockStatus.FINALIZED, votes: new Map(),
        block: { data: i },
      });
      if (!engine.slotBlocks.has(i)) engine.slotBlocks.set(i, new Set());
      engine.slotBlocks.get(i).add(`blk_${i}`);
      engine.votes.set(`blk_${i}`, new Map());
    }

    engine.lastFinalizedSlot = 9;

    // Trigger pruning manually
    engine._pruneOldData();

    // Blocks with slot < (9 - 5) = 4 should be pruned
    assert.strictEqual(engine.getBlock('blk_0'), null);
    assert.strictEqual(engine.getBlock('blk_3'), null);
    assert.ok(engine.getBlock('blk_4') !== null);
    assert.ok(engine.getBlock('blk_9') !== null);

    engine.stop();
  });
});

// ─────────────────────────────────────────────────────────────────
// 18. Soft consensus — independent sources
// ─────────────────────────────────────────────────────────────────

describe('Soft Consensus — Independent Sources', () => {
  it('should require minimum unique sources', () => {
    const obs = [
      { source: 'a', val: 1 },
      { source: 'b', val: 1 },
      { source: 'c', val: 1 },
    ];
    const r = checkSoftConsensus(obs, 3);
    assert.strictEqual(r.reached, true);
    assert.strictEqual(r.sources, 3);
  });

  it('should deduplicate same source', () => {
    const obs = [
      { source: 'a', val: 1 },
      { source: 'a', val: 2 },
      { source: 'b', val: 1 },
    ];
    const r = checkSoftConsensus(obs, 3);
    assert.strictEqual(r.reached, false);
    assert.strictEqual(r.sources, 2);
  });

  it('should fall back to operator field', () => {
    const obs = [
      { operator: 'op1' },
      { operator: 'op2' },
      { operator: 'op3' },
    ];
    const r = checkSoftConsensus(obs);
    assert.strictEqual(r.reached, true);
    assert.strictEqual(r.sources, 3);
  });
});

// ─────────────────────────────────────────────────────────────────
// 19. Vote aggregation across rounds
// ─────────────────────────────────────────────────────────────────

describe('Vote Aggregation', () => {
  it('should aggregate rounds keeping latest vote per voter', async () => {
    const kp = generateKeypair();
    const vote1 = createVote({
      proposalId: 'p', vote: VoteType.REJECT,
      voterPublicKey: kp.publicKey, voterPrivateKey: kp.privateKey,
      eScore: 50,
    });

    await new Promise((r) => setTimeout(r, 5));

    const vote2 = createVote({
      proposalId: 'p', vote: VoteType.APPROVE,
      voterPublicKey: kp.publicKey, voterPrivateKey: kp.privateKey,
      eScore: 50,
    });

    const result = aggregateVoteRounds([[vote1], [vote2]]);
    assert.strictEqual(result.voterCount, 1);
    assert.strictEqual(result.result, VoteType.APPROVE);
  });

  it('should combine votes from different voters across rounds', () => {
    const kp1 = generateKeypair();
    const kp2 = generateKeypair();

    const v1 = createVote({
      proposalId: 'p', vote: VoteType.APPROVE,
      voterPublicKey: kp1.publicKey, voterPrivateKey: kp1.privateKey,
      eScore: 50,
    });
    const v2 = createVote({
      proposalId: 'p', vote: VoteType.APPROVE,
      voterPublicKey: kp2.publicKey, voterPrivateKey: kp2.privateKey,
      eScore: 50,
    });

    const result = aggregateVoteRounds([[v1], [v2]]);
    assert.strictEqual(result.voterCount, 2);
  });
});

// ─────────────────────────────────────────────────────────────────
// 20. Vote creation and verification
// ─────────────────────────────────────────────────────────────────

describe('Vote Creation & Verification', () => {
  let kp;

  beforeEach(() => {
    kp = generateKeypair();
  });

  it('should create vote with computed weight', () => {
    const vote = createVote({
      proposalId: 'test',
      vote: VoteType.APPROVE,
      voterPublicKey: kp.publicKey,
      voterPrivateKey: kp.privateKey,
      eScore: 80,
      burned: 100,
      uptime: 0.9,
    });
    assert.ok(vote.weight > 0);
    assert.strictEqual(vote.vote, VoteType.APPROVE);
    assert.ok(vote.signature);
  });

  it('should verify valid vote', () => {
    const vote = createVote({
      proposalId: 'x',
      vote: VoteType.REJECT,
      voterPublicKey: kp.publicKey,
      voterPrivateKey: kp.privateKey,
      eScore: 50,
    });
    assert.strictEqual(verifyVote(vote), true);
  });

  it('should reject tampered vote', () => {
    const vote = createVote({
      proposalId: 'x',
      vote: VoteType.APPROVE,
      voterPublicKey: kp.publicKey,
      voterPrivateKey: kp.privateKey,
      eScore: 50,
    });
    vote.vote = VoteType.REJECT;
    assert.strictEqual(verifyVote(vote), false);
  });

  it('should reject vote without signature', () => {
    assert.strictEqual(verifyVote({ voter: 'someone' }), false);
  });

  it('should reject vote without voter', () => {
    assert.strictEqual(verifyVote({ signature: 'sig' }), false);
  });
});

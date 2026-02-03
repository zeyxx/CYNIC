/**
 * DimensionDiscovery Tests
 *
 * "New eyes see new truths" - κυνικός
 *
 * @module @cynic/emergence/test/dimension-discovery
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  DimensionDiscovery,
  createDimensionDiscovery,
  KNOWN_AXIOMS,
  ProposalStatus,
  ACCEPTANCE_THRESHOLDS,
} from '../src/dimension-discovery.js';
import { PHI_INV } from '@cynic/core';

// =============================================================================
// CONSTANTS
// =============================================================================

describe('KNOWN_AXIOMS', () => {
  it('should have 4 axioms', () => {
    assert.ok('PHI' in KNOWN_AXIOMS);
    assert.ok('VERIFY' in KNOWN_AXIOMS);
    assert.ok('CULTURE' in KNOWN_AXIOMS);
    assert.ok('BURN' in KNOWN_AXIOMS);
  });

  it('should have 6 dimensions per axiom', () => {
    assert.strictEqual(KNOWN_AXIOMS.PHI.length, 6);
    assert.strictEqual(KNOWN_AXIOMS.VERIFY.length, 6);
    assert.strictEqual(KNOWN_AXIOMS.CULTURE.length, 6);
    assert.strictEqual(KNOWN_AXIOMS.BURN.length, 6);
  });
});

describe('ProposalStatus', () => {
  it('should have all status values', () => {
    assert.strictEqual(ProposalStatus.DRAFT, 'DRAFT');
    assert.strictEqual(ProposalStatus.UNDER_REVIEW, 'UNDER_REVIEW');
    assert.strictEqual(ProposalStatus.VOTING, 'VOTING');
    assert.strictEqual(ProposalStatus.ACCEPTED, 'ACCEPTED');
    assert.strictEqual(ProposalStatus.REJECTED, 'REJECTED');
    assert.strictEqual(ProposalStatus.DEPRECATED, 'DEPRECATED');
  });
});

describe('ACCEPTANCE_THRESHOLDS', () => {
  it('should have φ-aligned approval ratio', () => {
    assert.ok(Math.abs(ACCEPTANCE_THRESHOLDS.APPROVAL_RATIO - PHI_INV) < 0.001);
  });

  it('should require 7 votes (like Sefirot)', () => {
    assert.strictEqual(ACCEPTANCE_THRESHOLDS.MIN_VOTES, 7);
  });

  it('should require minimum evidence', () => {
    assert.strictEqual(ACCEPTANCE_THRESHOLDS.MIN_EVIDENCE, 10);
  });
});

// =============================================================================
// DIMENSION DISCOVERY
// =============================================================================

describe('DimensionDiscovery', () => {
  let discovery;

  beforeEach(() => {
    discovery = createDimensionDiscovery({ nodeId: 'test_node' });
  });

  describe('Construction', () => {
    it('should create with factory', () => {
      assert.ok(discovery instanceof DimensionDiscovery);
    });

    it('should have node ID', () => {
      assert.strictEqual(discovery.nodeId, 'test_node');
    });

    it('should start with no candidates', () => {
      assert.strictEqual(discovery.candidates.size, 0);
    });

    it('should start with no proposals', () => {
      assert.strictEqual(discovery.proposals.size, 0);
    });
  });

  describe('analyzeJudgment()', () => {
    it('should store judgment pattern', () => {
      discovery.analyzeJudgment({
        scores: { PHI: 60, VERIFY: 70 },
      });
      assert.strictEqual(discovery.judgmentPatterns.length, 1);
    });

    it('should track axiom gaps for low scores', () => {
      discovery.analyzeJudgment({
        scores: { PHI: 30, VERIFY: 70 },
      });
      assert.ok(discovery.axiomGaps.has('PHI'));
    });

    it('should extract terms from raw assessment', () => {
      // Terms must end with: ity, ness, ment, tion, ance, ence, ism
      discovery.analyzeJudgment({
        rawAssessment: 'The implementation lacks transparency and verifiability with poor alignment',
      });
      // 'transparency' ends with 'ency' which matches 'ence' pattern
      // 'verifiability' ends with 'ity'
      // 'implementation' ends with 'tion'
      // 'alignment' ends with 'ment'
      assert.ok(
        discovery.termFrequency.has('transparency') ||
        discovery.termFrequency.has('verifiability') ||
        discovery.termFrequency.has('implementation') ||
        discovery.termFrequency.has('alignment'),
        'Should extract at least one dimension-like term'
      );
    });
  });

  describe('getCandidates()', () => {
    beforeEach(() => {
      // Generate enough patterns to create candidates
      for (let i = 0; i < 15; i++) {
        discovery.analyzeJudgment({
          scores: { PHI: 40, VERIFY: 30 },
          rawAssessment: 'The implementation lacks consistency and reliability',
        });
      }
    });

    it('should return candidates sorted by evidence', () => {
      const candidates = discovery.getCandidates();
      for (let i = 1; i < candidates.length; i++) {
        assert.ok(candidates[i - 1].evidence >= candidates[i].evidence);
      }
    });

    it('should filter by axiom', () => {
      const candidates = discovery.getCandidates('VERIFY');
      for (const c of candidates) {
        assert.strictEqual(c.axiom, 'VERIFY');
      }
    });
  });

  describe('propose()', () => {
    it('should create proposal', () => {
      const proposal = discovery.propose(
        'METHODOLOGY',
        'VERIFY',
        'Measures clarity of process',
        'Recurring pattern in judgments'
      );

      assert.strictEqual(proposal.name, 'METHODOLOGY');
      assert.strictEqual(proposal.axiom, 'VERIFY');
      assert.strictEqual(proposal.status, ProposalStatus.DRAFT);
    });

    it('should normalize name to UPPERCASE_SNAKE', () => {
      const proposal = discovery.propose(
        'some dimension',
        'PHI',
        'Test',
        'Test'
      );
      assert.strictEqual(proposal.name, 'SOME_DIMENSION');
    });

    it('should throw if dimension already exists', () => {
      assert.throws(() => {
        discovery.propose('COHERENCE', 'PHI', 'Test', 'Test');
      }, /already exists/);
    });

    it('should throw if already proposed', () => {
      discovery.propose('NEW_DIM', 'PHI', 'Test', 'Test');
      assert.throws(() => {
        discovery.propose('NEW_DIM', 'PHI', 'Test', 'Test');
      }, /already proposed/);
    });
  });

  describe('Proposal workflow', () => {
    let proposal;

    beforeEach(() => {
      proposal = discovery.propose(
        'TEST_DIM',
        'VERIFY',
        'Test dimension',
        'Testing'
      );
    });

    it('should submit for review', () => {
      const updated = discovery.submitForReview(proposal.id);
      assert.strictEqual(updated.status, ProposalStatus.UNDER_REVIEW);
    });

    it('should throw if submitting non-draft', () => {
      discovery.submitForReview(proposal.id);
      assert.throws(() => {
        discovery.submitForReview(proposal.id);
      }, /must be in DRAFT/);
    });

    it('should open voting', () => {
      discovery.submitForReview(proposal.id);
      const updated = discovery.openVoting(proposal.id);
      assert.strictEqual(updated.status, ProposalStatus.VOTING);
    });

    it('should throw if opening voting on non-reviewed', () => {
      assert.throws(() => {
        discovery.openVoting(proposal.id);
      }, /must be UNDER_REVIEW/);
    });
  });

  describe('vote()', () => {
    let proposal;

    beforeEach(() => {
      proposal = discovery.propose('VOTE_DIM', 'VERIFY', 'Test', 'Test');
      discovery.submitForReview(proposal.id);
      discovery.openVoting(proposal.id);
    });

    it('should record vote', () => {
      const result = discovery.vote(proposal.id, 'node1', true, 'Good idea');

      assert.strictEqual(result.proposal.votes.for.length, 1);
      assert.strictEqual(result.vote.approve, true);
    });

    it('should not allow double voting', () => {
      discovery.vote(proposal.id, 'node1', true);
      assert.throws(() => {
        discovery.vote(proposal.id, 'node1', false);
      }, /already voted/);
    });

    it('should accept proposal with enough approval', () => {
      // Need 7 votes with 61.8% approval (5 for, 2 against minimum)
      discovery.vote(proposal.id, 'node1', true);
      discovery.vote(proposal.id, 'node2', true);
      discovery.vote(proposal.id, 'node3', true);
      discovery.vote(proposal.id, 'node4', true);
      discovery.vote(proposal.id, 'node5', true);
      discovery.vote(proposal.id, 'node6', true);
      const result = discovery.vote(proposal.id, 'node7', false);

      // 6/7 = 85.7% > 61.8%
      assert.strictEqual(result.result.concluded, true);
      assert.strictEqual(result.result.status, 'ACCEPTED');
      assert.strictEqual(proposal.status, ProposalStatus.ACCEPTED);
    });

    it('should add accepted dimension to adopted', () => {
      for (let i = 1; i <= 7; i++) {
        discovery.vote(proposal.id, `node${i}`, true);
      }

      assert.ok(discovery.adopted.has(proposal.id));
    });

    it('should reject proposal with insufficient approval', () => {
      // Create a new proposal for this test
      const rejectProposal = discovery.propose('REJECT_DIM', 'VERIFY', 'Test', 'Test');
      discovery.submitForReview(rejectProposal.id);
      discovery.openVoting(rejectProposal.id);

      // Vote against first to prevent early acceptance
      // Need enough against votes so approval ratio < 61.8% when we hit 14 votes
      for (let i = 1; i <= 9; i++) {
        discovery.vote(rejectProposal.id, `against${i}`, false);
      }
      for (let i = 1; i <= 5; i++) {
        discovery.vote(rejectProposal.id, `for${i}`, true);
      }

      // 5/14 = 35.7% < 61.8%, and >= 14 votes (2x minimum = 14)
      const p = discovery.proposals.get(rejectProposal.id);
      assert.strictEqual(p.status, ProposalStatus.REJECTED);
    });
  });

  describe('getProposals()', () => {
    beforeEach(() => {
      discovery.propose('DIM1', 'PHI', 'Test1', 'Test');
      discovery.propose('DIM2', 'VERIFY', 'Test2', 'Test');
      discovery.submitForReview('VERIFY:DIM2');
    });

    it('should return all proposals', () => {
      const all = discovery.getProposals();
      assert.strictEqual(all.length, 2);
    });

    it('should filter by status', () => {
      const drafts = discovery.getProposals(ProposalStatus.DRAFT);
      assert.strictEqual(drafts.length, 1);
      assert.strictEqual(drafts[0].name, 'DIM1');
    });
  });

  describe('getDimensionsForAxiom()', () => {
    it('should return base dimensions', () => {
      const dims = discovery.getDimensionsForAxiom('PHI');
      assert.ok(dims.includes('COHERENCE'));
      assert.ok(dims.includes('HARMONY'));
    });

    it('should include adopted dimensions', () => {
      // Create and accept a proposal
      const proposal = discovery.propose('NEW_PHI_DIM', 'PHI', 'Test', 'Test');
      discovery.submitForReview(proposal.id);
      discovery.openVoting(proposal.id);
      for (let i = 1; i <= 7; i++) {
        discovery.vote(proposal.id, `node${i}`, true);
      }

      const dims = discovery.getDimensionsForAxiom('PHI');
      assert.ok(dims.includes('NEW_PHI_DIM'));
    });
  });

  describe('export/import', () => {
    it('should export state', () => {
      discovery.propose('TEST', 'PHI', 'Test', 'Test');
      const exported = discovery.export();

      assert.ok('proposals' in exported);
      assert.ok('candidates' in exported);
      assert.ok('adopted' in exported);
      assert.ok('exportedAt' in exported);
    });

    it('should import state', () => {
      discovery.propose('TEST', 'PHI', 'Test', 'Test');
      const exported = discovery.export();

      const newDiscovery = createDimensionDiscovery();
      newDiscovery.import(exported);

      assert.ok(newDiscovery.proposals.has('PHI:TEST'));
    });
  });

  describe('getStats()', () => {
    it('should return statistics', () => {
      discovery.propose('TEST', 'PHI', 'Test', 'Test');

      const stats = discovery.getStats();

      assert.ok('termsTracked' in stats);
      assert.ok('candidates' in stats);
      assert.ok('proposals' in stats);
      assert.ok('adopted' in stats);
      assert.strictEqual(stats.proposals.draft, 1);
    });
  });
});

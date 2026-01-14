/**
 * φ-BFT Voting
 *
 * Vote weighting and consensus threshold calculations
 *
 * @module @cynic/protocol/consensus/voting
 */

'use strict';

import { PHI, PHI_INV, CONSENSUS_THRESHOLD, GOVERNANCE_QUORUM } from '@cynic/core';
import { signData, verifySignature } from '../crypto/signature.js';
import { hashObject, randomHex } from '../crypto/hash.js';

/**
 * Vote types
 */
export const VoteType = {
  APPROVE: 'APPROVE',
  REJECT: 'REJECT',
  ABSTAIN: 'ABSTAIN',
};

/**
 * Consensus type
 */
export const ConsensusType = {
  SOFT: 'SOFT', // Judgments, patterns (daily)
  HARD: 'HARD', // Governance, dimensions (requires supermajority)
};

/**
 * Calculate vote weight
 *
 * Weight = E-Score × Burn-Multiplier × Uptime
 *
 * Burn-Multiplier = log_φ(total_burned + 1)
 *
 * @param {Object} params - Weight parameters
 * @param {number} params.eScore - E-Score (0-100)
 * @param {number} params.burned - Total tokens burned
 * @param {number} params.uptime - Uptime ratio (0-1)
 * @returns {number} Vote weight
 */
export function calculateVoteWeight({ eScore, burned = 0, uptime = 1 }) {
  // Burn multiplier: log base φ of (burned + 1)
  // This rewards contribution without extraction
  const burnMultiplier = Math.log(burned + 1) / Math.log(PHI);

  return eScore * Math.max(burnMultiplier, 1) * uptime;
}

/**
 * Generate vote ID
 * @returns {string} Unique vote ID
 */
export function generateVoteId() {
  return `vote_${randomHex(16)}`;
}

/**
 * Create a vote
 * @param {Object} params - Vote parameters
 * @param {string} params.proposalId - Proposal being voted on
 * @param {string} params.vote - Vote type (APPROVE, REJECT, ABSTAIN)
 * @param {string} params.voterPublicKey - Voter public key
 * @param {string} params.voterPrivateKey - Voter private key
 * @param {number} params.eScore - Voter E-Score
 * @param {number} [params.burned=0] - Voter burned tokens
 * @param {number} [params.uptime=1] - Voter uptime
 * @returns {Object} Vote object
 */
export function createVote({
  proposalId,
  vote,
  voterPublicKey,
  voterPrivateKey,
  eScore,
  burned = 0,
  uptime = 1,
}) {
  const voteObj = {
    id: generateVoteId(),
    proposal_id: proposalId,
    vote,
    voter: voterPublicKey,
    e_score: eScore,
    burned,
    uptime,
    weight: calculateVoteWeight({ eScore, burned, uptime }),
    timestamp: Date.now(),
  };

  // Sign vote
  const voteHash = hashObject({
    proposal_id: proposalId,
    vote,
    voter: voterPublicKey,
    timestamp: voteObj.timestamp,
  });
  voteObj.signature = signData(voteHash, voterPrivateKey);

  return voteObj;
}

/**
 * Verify vote signature
 * @param {Object} vote - Vote to verify
 * @returns {boolean} True if valid
 */
export function verifyVote(vote) {
  if (!vote.signature || !vote.voter) {
    return false;
  }

  const publicKey = vote.voter.startsWith('ed25519:')
    ? vote.voter.slice(8)
    : vote.voter;

  const voteHash = hashObject({
    proposal_id: vote.proposal_id,
    vote: vote.vote,
    voter: vote.voter,
    timestamp: vote.timestamp,
  });

  return verifySignature(voteHash, vote.signature, publicKey);
}

/**
 * Calculate consensus result
 * @param {Object[]} votes - Array of votes
 * @param {string} [type=ConsensusType.HARD] - Consensus type
 * @returns {Object} Consensus result
 */
export function calculateConsensus(votes, type = ConsensusType.HARD) {
  const validVotes = votes.filter((v) => v.vote !== VoteType.ABSTAIN);

  if (validVotes.length === 0) {
    return {
      reached: false,
      result: null,
      approveWeight: 0,
      rejectWeight: 0,
      totalWeight: 0,
      ratio: 0,
      threshold: CONSENSUS_THRESHOLD,
    };
  }

  const approveVotes = validVotes.filter((v) => v.vote === VoteType.APPROVE);
  const rejectVotes = validVotes.filter((v) => v.vote === VoteType.REJECT);

  const approveWeight = approveVotes.reduce((sum, v) => sum + (v.weight || 1), 0);
  const rejectWeight = rejectVotes.reduce((sum, v) => sum + (v.weight || 1), 0);
  const totalWeight = approveWeight + rejectWeight;

  const ratio = approveWeight / totalWeight;
  const threshold = type === ConsensusType.HARD ? CONSENSUS_THRESHOLD : 0.5;

  // Check quorum for hard consensus
  const hasQuorum = type === ConsensusType.HARD
    ? validVotes.length >= GOVERNANCE_QUORUM
    : true;

  return {
    reached: hasQuorum && ratio >= threshold,
    result: ratio >= threshold ? VoteType.APPROVE : VoteType.REJECT,
    approveWeight,
    rejectWeight,
    totalWeight,
    ratio: Math.round(ratio * 1000) / 1000,
    threshold,
    hasQuorum,
    voterCount: validVotes.length,
  };
}

/**
 * Check if soft consensus is reached (≥3 independent sources)
 * @param {Object[]} observations - Array of observations
 * @param {number} [minSources=3] - Minimum sources required
 * @returns {Object} Soft consensus result
 */
export function checkSoftConsensus(observations, minSources = 3) {
  const uniqueSources = new Set(observations.map((o) => o.source || o.operator));
  const sources = uniqueSources.size;

  return {
    reached: sources >= minSources,
    sources,
    minRequired: minSources,
  };
}

/**
 * Aggregate votes from multiple rounds
 * @param {Object[][]} rounds - Array of vote arrays per round
 * @returns {Object} Aggregated consensus
 */
export function aggregateVoteRounds(rounds) {
  // Flatten and dedupe by voter (keep latest vote)
  const latestByVoter = new Map();

  for (const round of rounds) {
    for (const vote of round) {
      const existing = latestByVoter.get(vote.voter);
      if (!existing || vote.timestamp > existing.timestamp) {
        latestByVoter.set(vote.voter, vote);
      }
    }
  }

  return calculateConsensus(Array.from(latestByVoter.values()));
}

export default {
  VoteType,
  ConsensusType,
  calculateVoteWeight,
  generateVoteId,
  createVote,
  verifyVote,
  calculateConsensus,
  checkSoftConsensus,
  aggregateVoteRounds,
};

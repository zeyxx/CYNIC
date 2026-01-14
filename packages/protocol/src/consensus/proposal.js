/**
 * Governance Proposals
 *
 * Proposals for hard consensus decisions
 *
 * @module @cynic/protocol/consensus/proposal
 */

'use strict';

import { GOVERNANCE_QUORUM, CYCLE_MS, AXIOMS } from '@cynic/core';
import { randomHex, hashObject, sha256Prefixed } from '../crypto/hash.js';
import { signData, verifySignature } from '../crypto/signature.js';
import { calculateConsensus, ConsensusType, VoteType } from './voting.js';

/**
 * Proposal action types
 */
export const ProposalAction = {
  ADD_DIMENSION: 'ADD_DIMENSION',
  REMOVE_DIMENSION: 'REMOVE_DIMENSION',
  MODIFY_THRESHOLD: 'MODIFY_THRESHOLD',
  ADD_PATTERN: 'ADD_PATTERN', // Force-add pattern
  BAN_PEER: 'BAN_PEER',
  PARAMETER_CHANGE: 'PARAMETER_CHANGE',
};

/**
 * Proposal status
 */
export const ProposalStatus = {
  PENDING: 'PENDING',
  VOTING: 'VOTING',
  PASSED: 'PASSED',
  REJECTED: 'REJECTED',
  EXPIRED: 'EXPIRED',
};

/**
 * Generate proposal ID
 * @returns {string} Unique proposal ID
 */
export function generateProposalId() {
  return `prop_${randomHex(16)}`;
}

/**
 * Create a governance proposal
 * @param {Object} params - Proposal parameters
 * @param {string} params.action - Proposal action type
 * @param {Object} params.params - Action parameters
 * @param {string} params.description - Human-readable description
 * @param {string} params.proposerPublicKey - Proposer public key
 * @param {string} params.proposerPrivateKey - Proposer private key
 * @param {number} [params.votingDurationMs] - Voting period (default: 1 cycle)
 * @returns {Object} Proposal object
 */
export function createProposal({
  action,
  params,
  description,
  proposerPublicKey,
  proposerPrivateKey,
  votingDurationMs = CYCLE_MS * 100, // 100 cycles
}) {
  if (!Object.values(ProposalAction).includes(action)) {
    throw new Error(`Invalid proposal action: ${action}`);
  }

  const proposal = {
    id: generateProposalId(),
    action,
    params,
    description,
    proposer: proposerPublicKey,
    created_at: Date.now(),
    voting_ends_at: Date.now() + votingDurationMs,
    status: ProposalStatus.VOTING,
    votes: [],
    result: null,
  };

  // Sign proposal
  const proposalHash = hashObject({
    action,
    params,
    proposer: proposerPublicKey,
    created_at: proposal.created_at,
  });
  proposal.signature = signData(proposalHash, proposerPrivateKey);

  return proposal;
}

/**
 * Verify proposal signature
 * @param {Object} proposal - Proposal to verify
 * @returns {boolean} True if valid
 */
export function verifyProposal(proposal) {
  if (!proposal.signature || !proposal.proposer) {
    return false;
  }

  const publicKey = proposal.proposer.startsWith('ed25519:')
    ? proposal.proposer.slice(8)
    : proposal.proposer;

  const proposalHash = hashObject({
    action: proposal.action,
    params: proposal.params,
    proposer: proposal.proposer,
    created_at: proposal.created_at,
  });

  return verifySignature(proposalHash, proposal.signature, publicKey);
}

/**
 * Validate proposal parameters
 * @param {Object} proposal - Proposal to validate
 * @returns {{ valid: boolean, errors: string[] }} Validation result
 */
export function validateProposal(proposal) {
  const errors = [];

  // Required fields
  const required = ['id', 'action', 'params', 'proposer', 'created_at', 'voting_ends_at'];
  for (const field of required) {
    if (proposal[field] === undefined) {
      errors.push(`Missing required field: ${field}`);
    }
  }

  // Action-specific validation
  switch (proposal.action) {
    case ProposalAction.ADD_DIMENSION:
      if (!proposal.params?.name) {
        errors.push('ADD_DIMENSION requires params.name');
      }
      if (!proposal.params?.axiom || !AXIOMS[proposal.params.axiom]) {
        errors.push('ADD_DIMENSION requires valid params.axiom');
      }
      break;

    case ProposalAction.MODIFY_THRESHOLD:
      if (typeof proposal.params?.threshold !== 'number') {
        errors.push('MODIFY_THRESHOLD requires params.threshold');
      }
      if (proposal.params.threshold < 0 || proposal.params.threshold > 100) {
        errors.push('threshold must be between 0 and 100');
      }
      break;

    case ProposalAction.BAN_PEER:
      if (!proposal.params?.peerId) {
        errors.push('BAN_PEER requires params.peerId');
      }
      break;
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Add vote to proposal
 * @param {Object} proposal - Proposal to vote on
 * @param {Object} vote - Vote to add
 * @returns {Object} Updated proposal
 */
export function addVoteToProposal(proposal, vote) {
  // Check if voting is still open
  if (Date.now() > proposal.voting_ends_at) {
    throw new Error('Voting period has ended');
  }

  // Check if voter already voted
  const existingVote = proposal.votes.find((v) => v.voter === vote.voter);
  if (existingVote) {
    // Update existing vote
    Object.assign(existingVote, vote);
  } else {
    proposal.votes.push(vote);
  }

  return proposal;
}

/**
 * Finalize proposal (calculate result)
 * @param {Object} proposal - Proposal to finalize
 * @returns {Object} Finalized proposal
 */
export function finalizeProposal(proposal) {
  if (proposal.status !== ProposalStatus.VOTING) {
    return proposal;
  }

  const now = Date.now();

  // Check if expired without votes
  if (now > proposal.voting_ends_at && proposal.votes.length < GOVERNANCE_QUORUM) {
    proposal.status = ProposalStatus.EXPIRED;
    proposal.result = {
      reached: false,
      reason: 'quorum_not_met',
      voterCount: proposal.votes.length,
      required: GOVERNANCE_QUORUM,
    };
    return proposal;
  }

  // Calculate consensus
  const consensus = calculateConsensus(proposal.votes, ConsensusType.HARD);
  proposal.result = consensus;

  if (now > proposal.voting_ends_at || consensus.reached) {
    proposal.status = consensus.reached ? ProposalStatus.PASSED : ProposalStatus.REJECTED;
  }

  return proposal;
}

/**
 * Create ADD_DIMENSION proposal
 * @param {Object} params - Dimension parameters
 * @param {string} params.name - Dimension name
 * @param {string} params.axiom - Associated axiom
 * @param {number} params.threshold - Score threshold
 * @param {number} params.weight - φ-weight
 * @param {string} params.proposerPublicKey - Proposer public key
 * @param {string} params.proposerPrivateKey - Proposer private key
 * @returns {Object} Proposal
 */
export function createAddDimensionProposal({
  name,
  axiom,
  threshold = 50,
  weight = 0.618,
  proposerPublicKey,
  proposerPrivateKey,
}) {
  return createProposal({
    action: ProposalAction.ADD_DIMENSION,
    params: { name, axiom, threshold, weight },
    description: `Add new dimension "${name}" to ${axiom} axiom`,
    proposerPublicKey,
    proposerPrivateKey,
  });
}

/**
 * Create PARAMETER_CHANGE proposal
 * @param {Object} params - Change parameters
 * @param {string} params.parameter - Parameter name
 * @param {*} params.oldValue - Current value
 * @param {*} params.newValue - Proposed value
 * @param {string} params.proposerPublicKey - Proposer public key
 * @param {string} params.proposerPrivateKey - Proposer private key
 * @returns {Object} Proposal
 */
export function createParameterChangeProposal({
  parameter,
  oldValue,
  newValue,
  proposerPublicKey,
  proposerPrivateKey,
}) {
  return createProposal({
    action: ProposalAction.PARAMETER_CHANGE,
    params: { parameter, oldValue, newValue },
    description: `Change ${parameter} from ${oldValue} to ${newValue}`,
    proposerPublicKey,
    proposerPrivateKey,
  });
}

export default {
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
};

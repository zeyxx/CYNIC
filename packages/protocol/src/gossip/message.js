/**
 * Gossip Message Types
 *
 * Protocol messages for gossip propagation
 *
 * @module @cynic/protocol/gossip/message
 */

'use strict';

import { randomHex, sha256Prefixed, hashObject } from '../crypto/hash.js';
import { signData, verifySignature } from '../crypto/signature.js';

/**
 * Message types
 */
export const MessageType = {
  // Push messages
  BLOCK: 'BLOCK', // New block announcement
  JUDGMENT: 'JUDGMENT', // New judgment
  PATTERN: 'PATTERN', // New pattern emerged
  LEARNING: 'LEARNING', // New learning

  // Pull messages
  SYNC_REQUEST: 'SYNC_REQUEST', // Request blocks since slot
  SYNC_RESPONSE: 'SYNC_RESPONSE', // Response with blocks
  STATE_REQUEST: 'STATE_REQUEST', // Request merkle state
  STATE_RESPONSE: 'STATE_RESPONSE', // Response with state
  JUDGMENT_SYNC_REQUEST: 'JUDGMENT_SYNC_REQUEST', // Request judgments since timestamp
  JUDGMENT_SYNC_RESPONSE: 'JUDGMENT_SYNC_RESPONSE', // Response with judgments

  // Control messages
  HEARTBEAT: 'HEARTBEAT', // Peer alive signal
  PEER_ANNOUNCE: 'PEER_ANNOUNCE', // New peer announcement

  // Consensus messages (Layer 4)
  CONSENSUS_BLOCK_PROPOSAL: 'CONSENSUS_BLOCK_PROPOSAL', // Block proposal
  CONSENSUS_VOTE: 'CONSENSUS_VOTE', // Vote on block
  CONSENSUS_VOTE_AGGREGATE: 'CONSENSUS_VOTE_AGGREGATE', // Aggregated votes
  CONSENSUS_FINALITY: 'CONSENSUS_FINALITY', // Finality notification
  CONSENSUS_SLOT_STATUS: 'CONSENSUS_SLOT_STATUS', // Slot status update

  // State sync messages (for late joiners)
  CONSENSUS_STATE_REQUEST: 'CONSENSUS_STATE_REQUEST', // Request finalized state
  CONSENSUS_STATE_RESPONSE: 'CONSENSUS_STATE_RESPONSE', // Response with finalized blocks

  // Merkle diff sync messages (O(log n) efficiency)
  CONSENSUS_MERKLE_ROOT: 'CONSENSUS_MERKLE_ROOT', // Announce Merkle root of state
  CONSENSUS_MERKLE_DIFF_REQUEST: 'CONSENSUS_MERKLE_DIFF_REQUEST', // Request diff from root
  CONSENSUS_MERKLE_DIFF_RESPONSE: 'CONSENSUS_MERKLE_DIFF_RESPONSE', // Response with diff only
};

/**
 * Generate message ID
 * @returns {string} Unique message ID
 */
export function generateMessageId() {
  return `msg_${randomHex(16)}`;
}

/**
 * Create a gossip message
 * @param {Object} params - Message parameters
 * @param {string} params.type - Message type
 * @param {Object} params.payload - Message payload
 * @param {string} params.sender - Sender public key
 * @param {string} [params.privateKey] - Sender private key (for signing)
 * @param {number} [params.ttl=3] - Time-to-live (hops)
 * @returns {Object} Gossip message
 */
export function createMessage({ type, payload, sender, privateKey, ttl = 3 }) {
  const message = {
    id: generateMessageId(),
    type,
    payload,
    sender,
    timestamp: Date.now(),
    ttl,
    hops: 0,
  };

  // Sign message if private key provided
  if (privateKey) {
    const contentHash = hashObject({ type, payload, sender, timestamp: message.timestamp });
    message.signature = signData(contentHash, privateKey);
  }

  return message;
}

/**
 * Verify message signature
 * @param {Object} message - Message to verify
 * @returns {boolean} True if valid
 */
export function verifyMessage(message) {
  if (!message.signature || !message.sender) {
    return false;
  }

  const publicKey = message.sender.startsWith('ed25519:')
    ? message.sender.slice(8)
    : message.sender;

  const contentHash = hashObject({
    type: message.type,
    payload: message.payload,
    sender: message.sender,
    timestamp: message.timestamp,
  });

  return verifySignature(contentHash, message.signature, publicKey);
}

/**
 * Check if message should be relayed
 * @param {Object} message - Message to check
 * @returns {boolean} True if should relay
 */
export function shouldRelay(message) {
  // Don't relay if TTL exhausted
  if (message.ttl <= 0) {
    return false;
  }

  // Don't relay control messages
  if (message.type === MessageType.HEARTBEAT) {
    return false;
  }

  return true;
}

/**
 * Prepare message for relay
 * @param {Object} message - Original message
 * @returns {Object} Message ready for relay
 */
export function prepareRelay(message) {
  return {
    ...message,
    ttl: message.ttl - 1,
    hops: message.hops + 1,
  };
}

/**
 * Create block announcement message
 * @param {Object} block - Block to announce
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Block message
 */
export function createBlockMessage(block, sender, privateKey) {
  return createMessage({
    type: MessageType.BLOCK,
    payload: block,
    sender,
    privateKey,
  });
}

/**
 * Create sync request message
 * @param {number} sinceSlot - Request blocks since this slot
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Sync request message
 */
export function createSyncRequest(sinceSlot, sender, privateKey) {
  return createMessage({
    type: MessageType.SYNC_REQUEST,
    payload: { since_slot: sinceSlot },
    sender,
    privateKey,
    ttl: 1, // Direct request, no relay
  });
}

/**
 * Create sync response message
 * @param {Object[]} blocks - Blocks to send
 * @param {string} requestId - Original request ID
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Sync response message
 */
export function createSyncResponse(blocks, requestId, sender, privateKey) {
  return createMessage({
    type: MessageType.SYNC_RESPONSE,
    payload: { blocks, request_id: requestId },
    sender,
    privateKey,
    ttl: 1, // Direct response, no relay
  });
}

/**
 * Create heartbeat message
 * @param {Object} status - Node status
 * @param {string} sender - Sender public key
 * @returns {Object} Heartbeat message
 */
export function createHeartbeat(status, sender) {
  return createMessage({
    type: MessageType.HEARTBEAT,
    payload: status,
    sender,
    ttl: 1, // No relay for heartbeats
  });
}

/**
 * Create peer announcement message
 * @param {Object} peerInfo - Peer information
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Peer announcement message
 */
export function createPeerAnnounce(peerInfo, sender, privateKey) {
  return createMessage({
    type: MessageType.PEER_ANNOUNCE,
    payload: peerInfo,
    sender,
    privateKey,
    ttl: 2, // Limited relay for peer discovery
  });
}

// ===========================================
// Consensus Message Helpers (Layer 4)
// ===========================================

/**
 * Create consensus block proposal message
 * @param {Object} proposal - Block proposal
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Consensus block proposal message
 */
export function createConsensusBlockProposal(proposal, sender, privateKey) {
  return createMessage({
    type: MessageType.CONSENSUS_BLOCK_PROPOSAL,
    payload: proposal,
    sender,
    privateKey,
    ttl: 3, // Propagate widely
  });
}

/**
 * Create consensus vote message
 * @param {Object} vote - Vote data
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Consensus vote message
 */
export function createConsensusVote(vote, sender, privateKey) {
  return createMessage({
    type: MessageType.CONSENSUS_VOTE,
    payload: vote,
    sender,
    privateKey,
    ttl: 3, // Propagate widely
  });
}

/**
 * Create consensus vote aggregate message
 * @param {Object} aggregate - Vote aggregate
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Consensus vote aggregate message
 */
export function createConsensusVoteAggregate(aggregate, sender, privateKey) {
  return createMessage({
    type: MessageType.CONSENSUS_VOTE_AGGREGATE,
    payload: aggregate,
    sender,
    privateKey,
    ttl: 2, // Limited relay
  });
}

/**
 * Create consensus finality notification message
 * @param {Object} finality - Finality data
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Consensus finality message
 */
export function createConsensusFinality(finality, sender, privateKey) {
  return createMessage({
    type: MessageType.CONSENSUS_FINALITY,
    payload: finality,
    sender,
    privateKey,
    ttl: 3, // Propagate widely
  });
}

/**
 * Create consensus slot status message
 * @param {Object} status - Slot status
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Consensus slot status message
 */
export function createConsensusSlotStatus(status, sender, privateKey) {
  return createMessage({
    type: MessageType.CONSENSUS_SLOT_STATUS,
    payload: status,
    sender,
    privateKey,
    ttl: 2, // Limited relay
  });
}

/**
 * Create consensus state request message (for late joiners)
 * @param {Object} request - State request
 * @param {number} request.sinceSlot - Request state since this slot
 * @param {number} [request.maxBlocks=50] - Maximum blocks to return
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} State request message
 */
export function createConsensusStateRequest(request, sender, privateKey) {
  return createMessage({
    type: MessageType.CONSENSUS_STATE_REQUEST,
    payload: {
      sinceSlot: request.sinceSlot || 0,
      maxBlocks: request.maxBlocks || 50,
      requestId: generateMessageId(),
    },
    sender,
    privateKey,
    ttl: 1, // Direct request, no relay
  });
}

/**
 * Create consensus state response message
 * @param {Object} response - State response
 * @param {string} response.requestId - Original request ID
 * @param {Object[]} response.blocks - Finalized blocks
 * @param {number} response.latestSlot - Latest finalized slot
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} State response message
 */
export function createConsensusStateResponse(response, sender, privateKey) {
  return createMessage({
    type: MessageType.CONSENSUS_STATE_RESPONSE,
    payload: {
      requestId: response.requestId,
      blocks: response.blocks || [],
      latestSlot: response.latestSlot || 0,
      validatorCount: response.validatorCount || 0,
    },
    sender,
    privateKey,
    ttl: 1, // Direct response, no relay
  });
}

/**
 * Check if message is a consensus message
 * @param {string} type - Message type
 * @returns {boolean} True if consensus message
 */
export function isConsensusGossipMessage(type) {
  return type && type.startsWith('CONSENSUS_');
}

// ═══════════════════════════════════════════════════════════════════════════
// Judgment Sync Messages (Gap #9/#10 fix)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create judgment sync request message
 * @param {number} sinceTimestamp - Request judgments since this timestamp
 * @param {number} [limit=100] - Max judgments to return
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Judgment sync request message
 */
export function createJudgmentSyncRequest(sinceTimestamp, limit, sender, privateKey) {
  return createMessage({
    type: MessageType.JUDGMENT_SYNC_REQUEST,
    payload: {
      since_timestamp: sinceTimestamp || 0,
      limit: limit || 100,
    },
    sender,
    privateKey,
    ttl: 1, // Direct request, no relay
  });
}

/**
 * Create judgment sync response message
 * @param {Object[]} judgments - Judgments to send
 * @param {string} requestId - Original request ID
 * @param {boolean} hasMore - Whether more judgments are available
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Judgment sync response message
 */
export function createJudgmentSyncResponse(judgments, requestId, hasMore, sender, privateKey) {
  return createMessage({
    type: MessageType.JUDGMENT_SYNC_RESPONSE,
    payload: {
      judgments: judgments || [],
      request_id: requestId,
      has_more: hasMore || false,
      count: judgments?.length || 0,
    },
    sender,
    privateKey,
    ttl: 1, // Direct response, no relay
  });
}

// ═══════════════════════════════════════════════════════════════════════════
// Merkle Diff Sync Messages (O(log n) efficiency)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create Merkle root announcement message
 * @param {Object} rootInfo - Merkle root info
 * @param {string} rootInfo.root - Merkle root hash
 * @param {number} rootInfo.slot - Latest finalized slot
 * @param {number} rootInfo.blockCount - Number of finalized blocks
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Merkle root message
 */
export function createConsensusMerkleRoot(rootInfo, sender, privateKey) {
  return createMessage({
    type: MessageType.CONSENSUS_MERKLE_ROOT,
    payload: {
      root: rootInfo.root,
      slot: rootInfo.slot || 0,
      blockCount: rootInfo.blockCount || 0,
      timestamp: Date.now(),
    },
    sender,
    privateKey,
    ttl: 2, // Limited relay for handshake
  });
}

/**
 * Create Merkle diff request message
 * @param {Object} request - Diff request
 * @param {string} request.theirRoot - Peer's Merkle root
 * @param {string} request.ourRoot - Our Merkle root
 * @param {number} request.sinceSlot - Request blocks since this slot
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Merkle diff request message
 */
export function createConsensusMerkleDiffRequest(request, sender, privateKey) {
  return createMessage({
    type: MessageType.CONSENSUS_MERKLE_DIFF_REQUEST,
    payload: {
      theirRoot: request.theirRoot,
      ourRoot: request.ourRoot,
      sinceSlot: request.sinceSlot || 0,
      requestId: generateMessageId(),
    },
    sender,
    privateKey,
    ttl: 1, // Direct request, no relay
  });
}

/**
 * Create Merkle diff response message
 * @param {Object} response - Diff response
 * @param {string} response.requestId - Original request ID
 * @param {Object[]} response.blocks - Blocks in diff (only missing/changed)
 * @param {string} response.newRoot - Root after applying diff
 * @param {number} response.latestSlot - Latest finalized slot
 * @param {string} sender - Sender public key
 * @param {string} privateKey - Sender private key
 * @returns {Object} Merkle diff response message
 */
export function createConsensusMerkleDiffResponse(response, sender, privateKey) {
  return createMessage({
    type: MessageType.CONSENSUS_MERKLE_DIFF_RESPONSE,
    payload: {
      requestId: response.requestId,
      blocks: response.blocks || [],
      newRoot: response.newRoot,
      latestSlot: response.latestSlot || 0,
      diffSize: response.blocks?.length || 0,
    },
    sender,
    privateKey,
    ttl: 1, // Direct response, no relay
  });
}

export default {
  MessageType,
  generateMessageId,
  createMessage,
  verifyMessage,
  shouldRelay,
  prepareRelay,
  createBlockMessage,
  createSyncRequest,
  createSyncResponse,
  createHeartbeat,
  createPeerAnnounce,
  // Consensus messages
  createConsensusBlockProposal,
  createConsensusVote,
  createConsensusVoteAggregate,
  createConsensusFinality,
  createConsensusSlotStatus,
  // State sync messages
  createConsensusStateRequest,
  createConsensusStateResponse,
  isConsensusGossipMessage,
  // Judgment sync messages
  createJudgmentSyncRequest,
  createJudgmentSyncResponse,
  // Merkle diff sync messages
  createConsensusMerkleRoot,
  createConsensusMerkleDiffRequest,
  createConsensusMerkleDiffResponse,
};

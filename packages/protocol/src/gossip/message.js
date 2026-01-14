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

  // Control messages
  HEARTBEAT: 'HEARTBEAT', // Peer alive signal
  PEER_ANNOUNCE: 'PEER_ANNOUNCE', // New peer announcement
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
};

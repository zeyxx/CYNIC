/**
 * Network Node Singleton
 *
 * PHASE 2: DECENTRALIZE
 *
 * Creates and manages a singleton CYNICNetworkNode for production use.
 * Controlled by environment variables:
 *   CYNIC_P2P_ENABLED=true     - Enable P2P networking
 *   CYNIC_P2P_PORT=8618        - P2P listen port (φ-aligned default)
 *   CYNIC_SEED_NODES=ws://...  - Comma-separated seed node addresses
 *   CYNIC_PUBLIC_KEY            - Node public key (generated if missing)
 *   CYNIC_PRIVATE_KEY           - Node private key (generated if missing)
 *
 * "The pack hunts together" - κυνικός
 *
 * @module @cynic/node/network-singleton
 */

'use strict';

import crypto from 'crypto';
import { createLogger } from '@cynic/core';
import { CYNICNetworkNode } from './network/network-node.js';

const log = createLogger('NetworkSingleton');

let _networkNode = null;
let _initPromise = null;

/**
 * Check if P2P networking is enabled via env vars
 * @returns {boolean}
 */
export function isP2PEnabled() {
  return process.env.CYNIC_P2P_ENABLED === 'true' ||
         process.env.CYNIC_P2P_ENABLED === '1';
}

/**
 * Generate a random keypair for node identity
 * @returns {{ publicKey: string, privateKey: string }}
 */
function generateNodeKeys() {
  const privateKey = crypto.randomBytes(32).toString('hex');
  const publicKey = crypto.createHash('sha256').update(privateKey).digest('hex');
  return { publicKey, privateKey };
}

/**
 * Get or create the NetworkNode singleton
 *
 * @param {Object} [options] - Override options (only used on first call)
 * @param {Object} [options.persistence] - Persistence layer for block store wiring
 * @param {boolean} [options.autoStart=false] - Start the node immediately
 * @returns {CYNICNetworkNode|null} The node, or null if P2P disabled
 */
export function getNetworkNode(options = {}) {
  if (_networkNode) return _networkNode;

  if (!isP2PEnabled() && !options.forceEnable) {
    log.debug('P2P networking disabled (set CYNIC_P2P_ENABLED=true to enable)');
    return null;
  }

  // Resolve keys from env or generate
  const envKeys = {
    publicKey: process.env.CYNIC_PUBLIC_KEY,
    privateKey: process.env.CYNIC_PRIVATE_KEY,
  };
  const keys = (envKeys.publicKey && envKeys.privateKey)
    ? envKeys
    : generateNodeKeys();

  if (!envKeys.publicKey) {
    log.info('Generated ephemeral node keys (set CYNIC_PUBLIC_KEY/CYNIC_PRIVATE_KEY for persistence)');
  }

  const port = parseInt(process.env.CYNIC_P2P_PORT) || 8618;
  const seedNodes = (process.env.CYNIC_SEED_NODES || '')
    .split(',')
    .map(s => s.trim())
    .filter(Boolean);

  _networkNode = new CYNICNetworkNode({
    publicKey: keys.publicKey,
    privateKey: keys.privateKey,
    port,
    seedNodes,
    eScore: 50,
    enabled: true,
    // Solana anchoring (opt-in)
    anchoringEnabled: process.env.CYNIC_ANCHORING_ENABLED === 'true',
    solanaCluster: process.env.SOLANA_CLUSTER || 'devnet',
    dryRun: process.env.CYNIC_ANCHORING_DRY_RUN !== 'false',
    ...options,
  });

  log.info('NetworkNode singleton created', {
    publicKey: keys.publicKey.slice(0, 16),
    port,
    seedNodes: seedNodes.length,
  });

  return _networkNode;
}

/**
 * Get or create NetworkNode and wire persistence (async version)
 *
 * @param {Object} [options]
 * @param {Object} [options.persistence] - Persistence layer
 * @returns {Promise<CYNICNetworkNode|null>}
 */
export async function getNetworkNodeAsync(options = {}) {
  if (_networkNode) return _networkNode;
  if (_initPromise) return _initPromise;

  const node = getNetworkNode(options);
  if (!node) return null;

  _initPromise = (async () => {
    // Wire block store from persistence
    if (options.persistence) {
      try {
        // Try getRepository pattern (PersistenceManager) first, then direct import
        let blockRepo = options.persistence.getRepository?.('poj-blocks') || null;

        if (!blockRepo) {
          const { PoJBlockRepository } = await import(
            '../../../persistence/src/postgres/repositories/poj-blocks.js'
          );
          blockRepo = new PoJBlockRepository(options.persistence.pool || options.persistence);
        }

        node.wireBlockStore({
          getBlocks: async (fromSlot, toSlot) => {
            const limit = Math.min(toSlot - fromSlot + 1, 500);
            return blockRepo.findSince(fromSlot - 1, limit);
          },
          storeBlock: async (block) => {
            return blockRepo.create(block);
          },
        });

        log.info('Block store wired (PoJBlockRepository)');
      } catch (error) {
        log.warn('Could not wire block store', { error: error.message });
      }
    }

    return node;
  })();

  return _initPromise;
}

/**
 * Start the network node (if it exists and is not already running)
 * @returns {Promise<boolean>} True if started
 */
export async function startNetworkNode() {
  if (!_networkNode) return false;
  try {
    await _networkNode.start();
    return true;
  } catch (error) {
    log.error('Failed to start network node', { error: error.message });
    return false;
  }
}

/**
 * Stop the network node
 * @returns {Promise<void>}
 */
export async function stopNetworkNode() {
  if (!_networkNode) return;
  await _networkNode.stop();
}

/**
 * Get network status (safe to call even if node is null)
 * @returns {Object|null}
 */
export function getNetworkStatus() {
  if (!_networkNode) return null;
  return _networkNode.getStatus();
}

/**
 * Reset for testing
 */
export function _resetNetworkForTesting() {
  if (process.env.NODE_ENV === 'production') {
    throw new Error('Cannot reset network singleton in production');
  }
  _networkNode = null;
  _initPromise = null;
}

export default {
  getNetworkNode,
  getNetworkNodeAsync,
  startNetworkNode,
  stopNetworkNode,
  getNetworkStatus,
  isP2PEnabled,
};

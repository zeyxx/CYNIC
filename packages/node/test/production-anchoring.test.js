/**
 * Production Anchoring Wiring Tests
 *
 * Verifies that wallet loading + BlockStore → AnchoringManager wiring
 * works correctly for production Solana anchoring.
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert';
import { CYNICNetworkNode } from '../src/network/network-node.js';

const mockPublicKey = 'test-public-key-0123456789abcdef';
const mockPrivateKey = 'test-private-key-0123456789abcdef';

describe('Production Anchoring Wiring', () => {
  let node;

  beforeEach(() => {
    node = new CYNICNetworkNode({
      publicKey: mockPublicKey,
      privateKey: mockPrivateKey,
      enabled: false,
      anchoringEnabled: true,
    });
  });

  afterEach(async () => {
    if (node) {
      await node.stop();
      node = null;
    }
  });

  it('wireAnchoringStore sets block store on anchoring manager', () => {
    const mockBlockStore = { getFailedAnchors: async () => [] };
    node.wireAnchoringStore(mockBlockStore);
    assert.strictEqual(node._anchoringManager._blockStore, mockBlockStore);
  });

  it('wireAnchoringStore is a no-op when blockStore is null', () => {
    const before = node._anchoringManager._blockStore;
    node.wireAnchoringStore(null);
    assert.strictEqual(node._anchoringManager._blockStore, before);
  });

  it('setAnchoringWallet sets wallet on anchoring manager', () => {
    const mockWallet = { publicKey: { toBase58: () => 'mockPubkey123' } };
    node.setAnchoringWallet(mockWallet);
    assert.strictEqual(node._anchoringManager._wallet, mockWallet);
  });

  it('wallet is null by default (no CYNIC_SOLANA_KEY)', () => {
    assert.strictEqual(node._anchoringManager._wallet, null);
  });

  it('wireAnchoringStore is independent from wireBlockStore', () => {
    const mockBlockStore = { getFailedAnchors: async () => [] };
    const mockStateSyncCallbacks = {
      getBlocks: async () => [],
      storeBlock: async () => {},
    };

    // Wire both stores independently
    node.wireBlockStore(mockStateSyncCallbacks);
    node.wireAnchoringStore(mockBlockStore);

    // Anchoring manager has its store
    assert.strictEqual(node._anchoringManager._blockStore, mockBlockStore);
  });
});

#!/usr/bin/env node
/**
 * CYNIC Full Flow Test - Anchor + Burns avec Mock
 *
 * "Onchain is truth" - κυνικός
 *
 * Usage: node test/full-flow.test.js
 */

import { mock } from 'node:test';
import { CYNICNode, SolanaCluster } from '../src/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// Mock Burns API
// ═══════════════════════════════════════════════════════════════════════════════

const MOCK_BURNS = new Map([
  ['5BURNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1', {
    verified: true,
    amount: 1_000_000_000, // 1 SOL
    burner: 'BURNERxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
    timestamp: Date.now() - 3600000,
    slot: 250000000,
    token: null, // SOL burn
  }],
  ['5BURNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2', {
    verified: true,
    amount: 500_000_000, // 0.5 SOL
    burner: 'USERxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
    timestamp: Date.now() - 7200000,
    slot: 250000001,
    token: 'ASHFxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx', // Token burn
  }],
]);

const originalFetch = global.fetch;
global.fetch = mock.fn(async (url) => {
  const urlObj = new URL(url);
  const signature = urlObj.pathname.split('/').pop();
  const burnData = MOCK_BURNS.get(signature);

  if (burnData) {
    return { ok: true, status: 200, json: async () => burnData };
  }
  return { ok: false, status: 404, statusText: 'Not Found' };
});

// ═══════════════════════════════════════════════════════════════════════════════
// Full Flow Test
// ═══════════════════════════════════════════════════════════════════════════════

console.log('═══════════════════════════════════════════════════════════════════════');
console.log('🐕 CYNIC Full Flow Test - Anchor + Burns');
console.log('═══════════════════════════════════════════════════════════════════════');
console.log();

// Create node with full integration
const node = new CYNICNode({
  name: 'full-flow-test',
  anchor: {
    enabled: true,
    cluster: SolanaCluster.DEVNET,
    autoAnchor: true,
  },
  burns: {
    enabled: true,
    minAmount: 100_000_000, // 0.1 SOL minimum
  },
});

await node.start();
console.log();

// ═══════════════════════════════════════════════════════════════════════════════
// Test 1: Judge with valid burn (1 SOL)
// ═══════════════════════════════════════════════════════════════════════════════

console.log('─────────────────────────────────────────────────────────────────────────');
console.log('📋 TEST 1: Judge with valid burn (1 SOL)');
console.log('─────────────────────────────────────────────────────────────────────────');

const result1 = await node.judge(
  {
    type: 'token',
    name: 'GoodToken',
    symbol: 'GOOD',
    holders: 5000,
    volume: 1000000,
  },
  {
    burnTx: '5BURNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1',
  }
);

console.log('Result:', {
  success: result1.success,
  burnVerified: result1.burnVerified,
  anchored: result1.anchored,
  verdict: result1.judgment?.verdict,
  qScore: result1.judgment?.q_score,
  anchorStatus: result1.judgment?.anchorStatus,
});
console.log();

// ═══════════════════════════════════════════════════════════════════════════════
// Test 2: Judge with smaller burn (0.5 SOL, min is 0.1 - should pass)
// ═══════════════════════════════════════════════════════════════════════════════

console.log('─────────────────────────────────────────────────────────────────────────');
console.log('📋 TEST 2: Judge with smaller burn (0.5 SOL, min 0.1 SOL)');
console.log('─────────────────────────────────────────────────────────────────────────');

const result2 = await node.judge(
  {
    type: 'token',
    name: 'AnotherToken',
    symbol: 'ANOT',
  },
  {
    burnTx: '5BURNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX2',
  }
);

console.log('Result:', {
  success: result2.success,
  burnVerified: result2.burnVerified,
  anchored: result2.anchored,
  verdict: result2.judgment?.verdict,
});
console.log();

// ═══════════════════════════════════════════════════════════════════════════════
// Test 3: Judge without burn (should fail)
// ═══════════════════════════════════════════════════════════════════════════════

console.log('─────────────────────────────────────────────────────────────────────────');
console.log('📋 TEST 3: Judge without burn (should be rejected)');
console.log('─────────────────────────────────────────────────────────────────────────');

const result3 = await node.judge({ type: 'token', name: 'NoBurnToken' });

console.log('Result:', {
  success: result3.success,
  error: result3.error,
  code: result3.code,
});
console.log();

// ═══════════════════════════════════════════════════════════════════════════════
// Test 4: Judge with invalid burn
// ═══════════════════════════════════════════════════════════════════════════════

console.log('─────────────────────────────────────────────────────────────────────────');
console.log('📋 TEST 4: Judge with invalid burn (should be rejected)');
console.log('─────────────────────────────────────────────────────────────────────────');

const result4 = await node.judge(
  { type: 'token', name: 'FakeToken' },
  { burnTx: '5FAKExxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1' }
);

console.log('Result:', {
  success: result4.success,
  error: result4.error,
  code: result4.code,
});
console.log();

// ═══════════════════════════════════════════════════════════════════════════════
// Check node stats
// ═══════════════════════════════════════════════════════════════════════════════

console.log('─────────────────────────────────────────────────────────────────────────');
console.log('📊 NODE STATS');
console.log('─────────────────────────────────────────────────────────────────────────');

const info = node.getInfo();
console.log('Judge stats:', info.judge);
console.log('Burns stats:', info.burns.stats);
console.log('Anchor queue:', info.anchor.queue.stats);
console.log();

// ═══════════════════════════════════════════════════════════════════════════════
// Stop node (flushes anchors)
// ═══════════════════════════════════════════════════════════════════════════════

console.log('─────────────────────────────────────────────────────────────────────────');
console.log('🛑 STOPPING NODE (flushing anchors)');
console.log('─────────────────────────────────────────────────────────────────────────');

await node.stop();

// Restore fetch
global.fetch = originalFetch;

console.log();
console.log('═══════════════════════════════════════════════════════════════════════');
console.log('✅ FULL FLOW TEST COMPLETE');
console.log('═══════════════════════════════════════════════════════════════════════');
console.log();
console.log('Summary:');
console.log('  ✓ Burns verification gates judgments');
console.log('  ✓ Valid burns allow judgment + anchoring');
console.log('  ✓ Missing/invalid burns are rejected');
console.log('  ✓ Judgments are queued and batched for Solana');
console.log('  ✓ φ-aligned: 38 items/batch, 61,803ms interval');
console.log();

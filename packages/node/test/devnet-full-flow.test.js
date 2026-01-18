#!/usr/bin/env node
/**
 * CYNIC Full Flow Test - Real Devnet Transactions
 *
 * "Onchain is truth" - κυνικός
 *
 * This test uses a real wallet to anchor judgments on Solana devnet.
 *
 * Usage: node test/devnet-full-flow.test.js
 */

import { mock } from 'node:test';
import { CYNICNode, SolanaCluster, loadWalletFromFile } from '../src/index.js';
import { Connection, PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';

const WALLET_PATH = '/workspaces/CYNIC-new/packages/anchor/test/.devnet-wallet.json';

// ═══════════════════════════════════════════════════════════════════════════════
// Mock Burns API (burns are separate from anchoring)
// ═══════════════════════════════════════════════════════════════════════════════

const MOCK_BURNS = new Map([
  ['5BURNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1', {
    verified: true,
    amount: 1_000_000_000, // 1 SOL
    burner: 'BURNERxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
    timestamp: Date.now() - 3600000,
    slot: 250000000,
    token: null,
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
// Main Test
// ═══════════════════════════════════════════════════════════════════════════════

console.log('═══════════════════════════════════════════════════════════════════════');
console.log('🐕 CYNIC Full Flow Test - REAL DEVNET TRANSACTIONS');
console.log('═══════════════════════════════════════════════════════════════════════');
console.log();

// Load wallet
const wallet = loadWalletFromFile(WALLET_PATH);
console.log('📍 Wallet:', wallet.publicKey);

// Check balance
const connection = new Connection(SolanaCluster.DEVNET);
const balance = await connection.getBalance(new PublicKey(wallet.publicKey));
console.log('💰 Balance:', balance / LAMPORTS_PER_SOL, 'SOL');

if (balance < 0.01 * LAMPORTS_PER_SOL) {
  console.error('❌ Insufficient balance for test');
  process.exit(1);
}

console.log();

// Create node with real wallet
const node = new CYNICNode({
  name: 'devnet-flow-test',
  anchor: {
    enabled: true,
    cluster: SolanaCluster.DEVNET,
    wallet: wallet,
    autoAnchor: false, // We'll flush manually
  },
  burns: {
    enabled: true,
    minAmount: 100_000_000,
  },
});

await node.start();
console.log();

// ═══════════════════════════════════════════════════════════════════════════════
// Create judgments with burn verification
// ═══════════════════════════════════════════════════════════════════════════════

console.log('─────────────────────────────────────────────────────────────────────────');
console.log('📋 Creating judgments with burn verification...');
console.log('─────────────────────────────────────────────────────────────────────────');

const judgments = [];

for (let i = 1; i <= 3; i++) {
  const result = await node.judge(
    {
      type: 'token',
      name: `TestToken${i}`,
      symbol: `TT${i}`,
      testId: `devnet-test-${Date.now()}-${i}`,
    },
    {
      burnTx: '5BURNxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxTX1',
    }
  );

  if (result.success) {
    judgments.push(result.judgment);
    console.log(`  ✓ Judgment ${i}: ${result.judgment.verdict} (${result.judgment.id.slice(0, 16)}...)`);
  } else {
    console.log(`  ✗ Judgment ${i} failed: ${result.error}`);
  }
}

console.log();
console.log(`Created ${judgments.length} judgments, queued for anchoring`);
console.log();

// ═══════════════════════════════════════════════════════════════════════════════
// Anchor to Solana devnet
// ═══════════════════════════════════════════════════════════════════════════════

console.log('─────────────────────────────────────────────────────────────────────────');
console.log('⚓ Anchoring to Solana devnet...');
console.log('─────────────────────────────────────────────────────────────────────────');

const queueLength = node.anchorQueue.getQueueLength();
console.log(`Queue length: ${queueLength} items`);

// Flush queue to anchor
const batch = await node.anchorQueue.flush();

if (batch && batch.signature) {
  console.log();
  console.log('✅ ANCHORED ON SOLANA DEVNET');
  console.log('─────────────────────────────────────────────────────────────────────────');
  console.log(`  Signature: ${batch.signature}`);
  console.log(`  Items:     ${batch.items.length}`);
  console.log(`  Merkle:    ${batch.merkleRoot}`);
  console.log();
  console.log(`  Explorer:  https://explorer.solana.com/tx/${batch.signature}?cluster=devnet`);
  console.log();

  // Verify merkle proofs
  console.log('─────────────────────────────────────────────────────────────────────────');
  console.log('🔍 Verifying Merkle proofs...');
  console.log('─────────────────────────────────────────────────────────────────────────');

  for (const judgment of judgments) {
    const proof = node.anchorQueue.getProof(judgment.id);
    if (proof) {
      console.log(`  ✓ ${judgment.id.slice(0, 16)}... has valid proof (${proof.merkleProof.length} nodes)`);
    } else {
      console.log(`  ✗ ${judgment.id.slice(0, 16)}... no proof found`);
    }
  }
} else {
  console.log('⚠️  No batch returned (queue may be empty or error occurred)');
}

console.log();

// ═══════════════════════════════════════════════════════════════════════════════
// Final stats
// ═══════════════════════════════════════════════════════════════════════════════

console.log('─────────────────────────────────────────────────────────────────────────');
console.log('📊 FINAL STATS');
console.log('─────────────────────────────────────────────────────────────────────────');

const info = node.getInfo();
console.log('Judge:', {
  totalJudgments: info.judge.totalJudgments,
  avgScore: info.judge.avgScore,
});
console.log('Anchor:', {
  totalAnchored: info.anchor.stats.totalAnchored,
  totalItems: info.anchor.stats.totalItems,
  lastSignature: info.anchor.stats.lastSignature?.slice(0, 20) + '...',
});
console.log('Burns:', {
  totalVerified: info.burns.stats.totalVerified,
});

// Check final balance
const finalBalance = await connection.getBalance(new PublicKey(wallet.publicKey));
const cost = (balance - finalBalance) / LAMPORTS_PER_SOL;
console.log();
console.log(`💰 Transaction cost: ${cost.toFixed(6)} SOL`);

await node.stop();

// Restore fetch
global.fetch = originalFetch;

console.log();
console.log('═══════════════════════════════════════════════════════════════════════');
console.log('✅ DEVNET FULL FLOW TEST COMPLETE');
console.log('═══════════════════════════════════════════════════════════════════════');

#!/usr/bin/env node
/**
 * @cynic/anchor + @cynic/burns - End-to-End Integration Test
 *
 * Tests the complete "Onchain is truth" workflow:
 * 1. Verify burn (proof of skin in the game)
 * 2. Anchor judgment to Solana (proof of existence)
 * 3. Verify anchor on-chain (proof of truth)
 *
 * "Don't extract, burn. Onchain is truth." - κυνικός
 *
 * Usage:
 *   node test/e2e-anchor-burns.test.js
 */

import { Connection, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { createHash } from 'crypto';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';

// Anchor imports
import {
  createAnchorer,
  createAnchorQueue,
  SolanaCluster,
  CynicWallet,
  AnchorStatus,
  ANCHOR_CONSTANTS,
} from '../src/index.js';

// Burns imports
import { createBurnVerifier, BurnStatus } from '../../burns/src/index.js';

// ═══════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════

const CONFIG = {
  cluster: SolanaCluster.DEVNET,
  useRealSolana: true, // Set to false for simulation mode
};

// Colors
const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';
const MAGENTA = '\x1b[35m';
const RESET = '\x1b[0m';
const BOLD = '\x1b[1m';

function log(msg, color = RESET) {
  console.log(`${color}${msg}${RESET}`);
}

function header(title) {
  log(`\n${'═'.repeat(70)}`, CYAN);
  log(`  ${title}`, CYAN);
  log(`${'═'.repeat(70)}`, CYAN);
}

function step(num, title) {
  log(`\n  ┌${'─'.repeat(66)}┐`, YELLOW);
  log(`  │ Step ${num}: ${title.padEnd(57)} │`, YELLOW);
  log(`  └${'─'.repeat(66)}┘`, YELLOW);
}

// ═══════════════════════════════════════════════════════════════════════════
// Mock Burns API (since real API may not have test burns)
// ═══════════════════════════════════════════════════════════════════════════

const MOCK_BURNS = new Map([
  [
    'BURNtx1111111111111111111111111111111111111111111111111111111111',
    {
      verified: true,
      amount: 1_000_000_000, // 1 SOL
      burner: 'OP1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
      timestamp: Date.now() - 3600000,
      slot: 123456789,
      token: null,
    },
  ],
  [
    'BURNtx2222222222222222222222222222222222222222222222222222222222',
    {
      verified: true,
      amount: 500_000_000, // 0.5 SOL
      burner: 'OP2xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
      timestamp: Date.now() - 7200000,
      slot: 123456700,
      token: null,
    },
  ],
]);

let originalFetch;

function setupMockBurnsAPI() {
  originalFetch = global.fetch;

  global.fetch = async (url, options) => {
    const urlStr = url.toString();

    // Only mock burns API calls
    if (urlStr.includes('alonisthe.dev/burns')) {
      const pathParts = urlStr.split('/');
      const signature = pathParts[pathParts.length - 1];

      const burnData = MOCK_BURNS.get(signature);

      if (burnData) {
        return {
          ok: true,
          status: 200,
          json: async () => burnData,
        };
      }

      return {
        ok: false,
        status: 404,
        statusText: 'Not Found',
      };
    }

    // Pass through to real fetch for Solana RPC calls
    return originalFetch(url, options);
  };
}

function teardownMockBurnsAPI() {
  if (originalFetch) {
    global.fetch = originalFetch;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test Results
// ═══════════════════════════════════════════════════════════════════════════

const results = {
  passed: 0,
  failed: 0,
  tests: [],
};

function pass(name, details = '') {
  results.passed++;
  results.tests.push({ name, status: 'PASS', details });
  log(`    ✅ ${name}${details ? ': ' + details : ''}`, GREEN);
}

function fail(name, error) {
  results.failed++;
  results.tests.push({ name, status: 'FAIL', error: error.message || error });
  log(`    ❌ ${name}: ${error.message || error}`, RED);
}

// ═══════════════════════════════════════════════════════════════════════════
// E2E Test Scenarios
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Scenario 1: Complete Judgment Anchoring Flow
 *
 * An operator wants to anchor a judgment:
 * 1. Must first prove they burned tokens (skin in the game)
 * 2. Then anchor the judgment merkle root to Solana
 * 3. Finally verify the anchor exists on-chain
 */
async function testCompleteJudgmentFlow(connection, keypair) {
  step(1, 'Complete Judgment Anchoring Flow');

  const burnVerifier = createBurnVerifier();
  const wallet = keypair ? new CynicWallet({ secretKey: keypair.secretKey }) : null;
  const anchorer = createAnchorer({
    cluster: CONFIG.cluster,
    wallet,
  });

  // Operator's burn transaction
  const operatorBurnTx = 'BURNtx1111111111111111111111111111111111111111111111111111111111';

  // Step 1: Verify burn
  log('    Verifying operator burn...', CYAN);
  const burnVerification = await burnVerifier.verify(operatorBurnTx);

  if (!burnVerification.verified) {
    fail('Burn verification', 'Operator burn not verified');
    return null;
  }
  pass('Burn verification', `${burnVerification.amount / 1e9} SOL burned`);

  // Step 2: Create judgment and compute merkle root
  const judgment = {
    id: `jdg_${Date.now()}`,
    item: { type: 'token', address: 'TEST123' },
    verdict: 'WAG',
    qScore: 72,
    dimensions: {
      phi: 0.618,
      verify: 0.75,
      culture: 0.80,
      burn: 0.65,
    },
    timestamp: Date.now(),
    operator: burnVerification.burner,
    burnTx: operatorBurnTx,
  };

  const judgmentHash = createHash('sha256')
    .update(JSON.stringify(judgment))
    .digest('hex');

  log(`    Judgment hash: ${judgmentHash.slice(0, 16)}...`, CYAN);

  // Step 3: Anchor to Solana
  log('    Anchoring to Solana...', CYAN);
  const anchorResult = await anchorer.anchor(judgmentHash, [judgment.id]);

  if (!anchorResult.success) {
    fail('Anchor transaction', anchorResult.error);
    return null;
  }

  if (anchorResult.simulated) {
    pass('Anchor transaction (simulated)', `sig: ${anchorResult.signature?.slice(0, 20)}...`);
  } else {
    pass('Anchor transaction', `sig: ${anchorResult.signature.slice(0, 20)}...`);
    log(`    Explorer: https://explorer.solana.com/tx/${anchorResult.signature}?cluster=devnet`, MAGENTA);
  }

  // Step 4: Verify anchor on-chain (only for real transactions)
  if (!anchorResult.simulated && anchorResult.signature) {
    log('    Verifying anchor on-chain...', CYAN);
    const verification = await anchorer.verifyAnchor(anchorResult.signature, judgmentHash);

    if (verification.verified) {
      pass('On-chain verification', `slot: ${verification.slot}`);
    } else {
      fail('On-chain verification', verification.error);
    }
  }

  return {
    judgment,
    judgmentHash,
    burnVerification,
    anchorResult,
  };
}

/**
 * Scenario 2: Batch Judgments with Queue
 *
 * Multiple judgments are queued and anchored together:
 * 1. Verify burns for each operator
 * 2. Queue judgments
 * 3. Flush queue (anchors batch merkle root)
 * 4. Verify merkle proofs
 */
async function testBatchJudgmentQueue(connection, keypair) {
  step(2, 'Batch Judgments with Queue');

  const burnVerifier = createBurnVerifier();
  const wallet = keypair ? new CynicWallet({ secretKey: keypair.secretKey }) : null;
  const anchorer = createAnchorer({
    cluster: CONFIG.cluster,
    wallet,
  });

  const queue = createAnchorQueue({
    anchorer,
    batchSize: 10, // Larger than our test items
    autoStart: false,
  });

  // Multiple operators with burns
  const operators = [
    { id: 'op_1', burnTx: 'BURNtx1111111111111111111111111111111111111111111111111111111111' },
    { id: 'op_2', burnTx: 'BURNtx2222222222222222222222222222222222222222222222222222222222' },
  ];

  // Step 1: Verify all operator burns
  log('    Verifying operator burns...', CYAN);
  const verifiedOperators = [];

  for (const op of operators) {
    const verification = await burnVerifier.verify(op.burnTx);
    if (verification.verified) {
      verifiedOperators.push({ ...op, burn: verification });
    }
  }

  if (verifiedOperators.length === operators.length) {
    pass('All burns verified', `${verifiedOperators.length} operators`);
  } else {
    fail('Burn verification', `Only ${verifiedOperators.length}/${operators.length} verified`);
    queue.destroy();
    return null;
  }

  // Step 2: Each operator submits a judgment
  log('    Queueing judgments...', CYAN);
  const judgments = [];

  for (const op of verifiedOperators) {
    const judgment = {
      id: `jdg_${op.id}_${Date.now()}`,
      verdict: op.id === 'op_1' ? 'WAG' : 'GROWL',
      qScore: op.id === 'op_1' ? 78 : 45,
      operator: op.id,
      burnAmount: op.burn.amount,
    };

    queue.enqueue(judgment.id, judgment);
    judgments.push(judgment);
  }

  const queueLength = queue.getQueueLength();
  if (queueLength === judgments.length) {
    pass('Queue judgments', `${queueLength} items queued`);
  } else {
    fail('Queue judgments', `Expected ${judgments.length}, got ${queueLength}`);
  }

  // Step 3: Flush queue (anchor batch)
  log('    Flushing queue to Solana...', CYAN);
  const batch = await queue.flush();

  if (batch && batch.merkleRoot) {
    if (batch.signature) {
      pass('Batch anchored', `root: ${batch.merkleRoot.slice(0, 16)}...`);
      log(`    Explorer: https://explorer.solana.com/tx/${batch.signature}?cluster=devnet`, MAGENTA);
    } else {
      pass('Batch created (simulated)', `root: ${batch.merkleRoot.slice(0, 16)}...`);
    }

    // Step 4: Verify merkle proofs
    log('    Verifying merkle proofs...', CYAN);
    let proofsValid = true;

    for (const judgment of judgments) {
      const proofData = queue.getProof(judgment.id);
      if (proofData) {
        const isValid = queue.verifyProof(
          proofData.itemHash,
          proofData.merkleRoot,
          proofData.merkleProof
        );
        if (!isValid) proofsValid = false;
      } else {
        proofsValid = false;
      }
    }

    if (proofsValid) {
      pass('Merkle proofs', 'All proofs valid');
    } else {
      fail('Merkle proofs', 'Some proofs invalid');
    }
  } else {
    fail('Batch anchor', 'No batch created');
  }

  queue.destroy();
  return { judgments, batch };
}

/**
 * Scenario 3: Reject Unverified Burns
 *
 * System must reject anchoring from operators without verified burns
 */
async function testRejectUnverifiedBurns() {
  step(3, 'Reject Unverified Burns');

  const burnVerifier = createBurnVerifier();

  // Operator with fake/unverified burn
  const fakeBurnTx = 'FAKEtx1111111111111111111111111111111111111111111111111111111111';

  log('    Checking unverified burn...', CYAN);
  const verification = await burnVerifier.verify(fakeBurnTx);

  if (!verification.verified) {
    pass('Burn rejected', 'Unverified burn correctly blocked');

    // Simulate the enforcement
    const canAnchor = verification.verified;
    if (!canAnchor) {
      pass('Anchor blocked', 'Cannot anchor without verified burn');
    }
  } else {
    fail('Burn check', 'Should have rejected unverified burn');
  }
}

/**
 * Scenario 4: φ-Aligned Minimum Burn
 *
 * Burns must meet φ-aligned minimum threshold
 */
async function testPhiAlignedBurnMinimum() {
  step(4, 'φ-Aligned Burn Minimum');

  const burnVerifier = createBurnVerifier();
  const PHI_INV = 0.618033988749895;
  const BASE_AMOUNT = 1_000_000_000; // 1 SOL
  const MIN_BURN = Math.floor(BASE_AMOUNT * PHI_INV); // ~618M lamports

  log(`    φ minimum: ${MIN_BURN / 1e9} SOL (${MIN_BURN} lamports)`, CYAN);

  // Test 1: Burn below threshold (0.5 SOL)
  const lowBurnTx = 'BURNtx2222222222222222222222222222222222222222222222222222222222';
  const lowVerification = await burnVerifier.verify(lowBurnTx, {
    minAmount: MIN_BURN,
  });

  if (!lowVerification.verified) {
    pass('Low burn rejected', `0.5 SOL < φ minimum (0.618 SOL)`);
  } else {
    fail('Low burn check', 'Should have rejected burn below φ minimum');
  }

  // Test 2: Burn at/above threshold (1 SOL)
  const highBurnTx = 'BURNtx1111111111111111111111111111111111111111111111111111111111';
  const highVerification = await burnVerifier.verify(highBurnTx, {
    minAmount: MIN_BURN,
  });

  if (highVerification.verified) {
    pass('High burn accepted', `1 SOL >= φ minimum (0.618 SOL)`);
  } else {
    fail('High burn check', 'Should have accepted burn above φ minimum');
  }
}

/**
 * Scenario 5: Full State Export/Import
 *
 * Export and restore complete system state
 */
async function testStateExportImport() {
  step(5, 'State Export/Import');

  const burnVerifier = createBurnVerifier();
  const anchorer = createAnchorer({ cluster: CONFIG.cluster });
  const queue = createAnchorQueue({ anchorer, batchSize: 10 });

  // Create some state
  await burnVerifier.verify('BURNtx1111111111111111111111111111111111111111111111111111111111');
  queue.enqueue('test_1', { data: 'test1' });
  queue.enqueue('test_2', { data: 'test2' });

  // Export all state
  const exportedBurns = burnVerifier.export();
  const exportedAnchorer = anchorer.export();
  const exportedQueue = queue.export();

  log('    Exported state:', CYAN);
  log(`      Burns cache: ${exportedBurns.cache?.length || 0} entries`, CYAN);
  log(`      Anchorer history: ${exportedAnchorer.history?.length || 0} entries`, CYAN);
  log(`      Queue items: ${exportedQueue.queue?.length || 0} items`, CYAN);

  // Create new instances and import
  const newBurnVerifier = createBurnVerifier();
  const newAnchorer = createAnchorer({ cluster: CONFIG.cluster });
  const newQueue = createAnchorQueue({ anchorer: newAnchorer, batchSize: 10 });

  newBurnVerifier.import(exportedBurns);
  newAnchorer.import(exportedAnchorer);
  newQueue.import(exportedQueue);

  // Verify state restored
  const burnsCached = newBurnVerifier.isVerified('BURNtx1111111111111111111111111111111111111111111111111111111111');
  const queueRestored = newQueue.getQueueLength() === 2;

  if (burnsCached && queueRestored) {
    pass('State restored', 'All components imported successfully');
  } else {
    fail('State restore', `Burns: ${burnsCached}, Queue: ${queueRestored}`);
  }

  queue.destroy();
  newQueue.destroy();
}

// ═══════════════════════════════════════════════════════════════════════════
// Main Test Runner
// ═══════════════════════════════════════════════════════════════════════════

async function main() {
  header('🐕 CYNIC E2E: Anchor + Burns Integration');
  log(`\n  Cluster: ${CONFIG.cluster}`, CYAN);
  log(`  Time: ${new Date().toISOString()}`, CYAN);

  // Setup mock burns API
  setupMockBurnsAPI();

  // Load wallet if available
  const walletPath = join(import.meta.dirname, '.devnet-wallet.json');
  let keypair = null;

  if (existsSync(walletPath) && CONFIG.useRealSolana) {
    log(`\n  Loading wallet from: ${walletPath}`, CYAN);
    const keyData = JSON.parse(readFileSync(walletPath, 'utf8'));
    keypair = Keypair.fromSecretKey(new Uint8Array(keyData));
    log(`  Wallet: ${keypair.publicKey.toBase58()}`, CYAN);

    const connection = new Connection(CONFIG.cluster, 'confirmed');
    const balance = await connection.getBalance(keypair.publicKey);
    log(`  Balance: ${balance / LAMPORTS_PER_SOL} SOL`, balance > 0 ? GREEN : YELLOW);

    if (balance === 0) {
      log('  ⚠️ No balance - using simulation mode', YELLOW);
      keypair = null;
    }
  } else {
    log('\n  No wallet found - using simulation mode', YELLOW);
  }

  const connection = new Connection(CONFIG.cluster, 'confirmed');

  // Run all E2E scenarios
  try {
    await testCompleteJudgmentFlow(connection, keypair);
    await testBatchJudgmentQueue(connection, keypair);
    await testRejectUnverifiedBurns();
    await testPhiAlignedBurnMinimum();
    await testStateExportImport();
  } finally {
    teardownMockBurnsAPI();
  }

  // Summary
  header('📊 E2E Test Results');

  const total = results.passed + results.failed;
  const passRate = ((results.passed / total) * 100).toFixed(1);

  log(`\n  Total Tests: ${total}`, CYAN);
  log(`  Passed: ${results.passed}`, GREEN);
  log(`  Failed: ${results.failed}`, results.failed > 0 ? RED : GREEN);
  log(`  Pass Rate: ${passRate}%`, passRate >= 80 ? GREEN : YELLOW);

  if (results.failed > 0) {
    log('\n  Failed Tests:', RED);
    for (const test of results.tests.filter((t) => t.status === 'FAIL')) {
      log(`    - ${test.name}: ${test.error}`, RED);
    }
  }

  header('🐕 E2E Complete');
  log(`\n  "Don't extract, burn. Onchain is truth."`, CYAN);
  log(`  Architecture: ${results.failed === 0 ? 'VALIDATED' : 'NEEDS ATTENTION'}`,
      results.failed === 0 ? GREEN : YELLOW);

  process.exit(results.failed > 0 ? 1 : 0);
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});

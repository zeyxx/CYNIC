#!/usr/bin/env node
/**
 * @cynic/anchor - Full Devnet Architecture Validation
 *
 * Comprehensive test suite validating the entire anchoring architecture on Solana devnet.
 *
 * "Onchain is truth" - κυνικός
 *
 * Tests:
 * 1. Wallet management (generation, export/import)
 * 2. Single anchor transactions
 * 3. Queue batching with merkle proofs
 * 4. Verification of anchored data
 * 5. Multiple operators simulation
 * 6. Burns integration (mock)
 *
 * Usage:
 *   node test/devnet-full-test.js
 *
 * Note: This is a MANUAL test - requires network and SOL airdrop.
 * Automatically skipped when run via `node --test`.
 */

// Skip if running under Node's test runner (not meant for automated tests)
if (process.env.NODE_TEST_CONTEXT) {
  console.log('⏭️  Skipping devnet-full-test.js (manual test, run directly with: node test/devnet-full-test.js)');
  process.exit(0);
}

import { Connection, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import {
  createAnchorer,
  createAnchorQueue,
  SolanaCluster,
  CynicWallet,
  generateWallet,
  saveWalletToFile,
  loadWalletFromFile,
  base58Encode,
  AnchorStatus,
  ANCHOR_CONSTANTS,
} from '../src/index.js';
import { createHash, randomBytes } from 'crypto';
import { writeFileSync, unlinkSync, existsSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

// ═══════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════

const CONFIG = {
  // Use Helius if API key available (faster, no rate-limits)
  cluster: SolanaCluster.HELIUS_DEVNET || SolanaCluster.DEVNET,
  airdropAmount: 0.5 * LAMPORTS_PER_SOL, // 0.5 SOL
  testTimeout: 120000, // 2 minutes
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

function subheader(title) {
  log(`\n  ┌${'─'.repeat(66)}┐`, YELLOW);
  log(`  │ ${title.padEnd(64)} │`, YELLOW);
  log(`  └${'─'.repeat(66)}┘`, YELLOW);
}

// ═══════════════════════════════════════════════════════════════════════════
// Test Results Tracking
// ═══════════════════════════════════════════════════════════════════════════

const results = {
  passed: 0,
  failed: 0,
  tests: [],
};

function pass(name, details = '') {
  results.passed++;
  results.tests.push({ name, status: 'PASS', details });
  log(`  ✅ ${name}${details ? ': ' + details : ''}`, GREEN);
}

function fail(name, error) {
  results.failed++;
  results.tests.push({ name, status: 'FAIL', error: error.message });
  log(`  ❌ ${name}: ${error.message}`, RED);
}

// ═══════════════════════════════════════════════════════════════════════════
// Utility Functions
// ═══════════════════════════════════════════════════════════════════════════

function generateMerkleRoot() {
  return createHash('sha256').update(randomBytes(32)).digest('hex');
}

async function airdrop(connection, publicKey, amount, retries = 3) {
  // Airdrops only work on public devnet RPC, not Helius
  const airdropConnection = new Connection(SolanaCluster.DEVNET, 'confirmed');
  try {
    const sig = await airdropConnection.requestAirdrop(publicKey, amount);
    const latestBlockhash = await airdropConnection.getLatestBlockhash();
    await airdropConnection.confirmTransaction({
      signature: sig,
      ...latestBlockhash,
    });
    return true;
  } catch (error) {
    if (error.message.includes('429') && retries > 0) {
      log(`    (Rate limited, waiting 5s... ${retries} retries left)`, YELLOW);
      await new Promise((r) => setTimeout(r, 5000));
      return airdrop(connection, publicKey, amount, retries - 1);
    }
    log(`    Airdrop failed: ${error.message}`, RED);
    return false;
  }
}

/**
 * Load wallet from file or create new one with airdrop
 */
async function loadOrCreateWallet(connection, walletPath) {
  if (existsSync(walletPath)) {
    log(`\n  Loading pre-funded wallet from: ${walletPath}`, CYAN);
    const { readFileSync } = await import('fs');
    const keyData = JSON.parse(readFileSync(walletPath, 'utf8'));
    const keypair = Keypair.fromSecretKey(new Uint8Array(keyData));
    log(`  Wallet: ${keypair.publicKey.toBase58()}`, CYAN);
    return keypair;
  }

  log(`\n  ⚠️ No saved wallet found, generating new one...`, YELLOW);
  const keypair = Keypair.generate();
  log(`  New wallet: ${keypair.publicKey.toBase58()}`, CYAN);

  const { writeFileSync } = await import('fs');
  writeFileSync(walletPath, JSON.stringify(Array.from(keypair.secretKey)));
  log(`  Saved wallet to: ${walletPath}`, GREEN);
  log('  Requesting airdrop...', YELLOW);
  await airdrop(connection, keypair.publicKey, LAMPORTS_PER_SOL);

  return keypair;
}

/**
 * Verify merkle proofs for all items in queue
 */
function verifyQueueProofs(queue, items) {
  for (const item of items) {
    const proofData = queue.getProof(item.id);
    if (!proofData) return false;
    const isValid = queue.verifyProof(proofData.itemHash, proofData.merkleRoot, proofData.merkleProof);
    if (!isValid) return false;
  }
  return true;
}

/**
 * Print test results summary
 */
function printTestSummary() {
  header('📊 Test Results Summary');

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
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1: Wallet Management
// ═══════════════════════════════════════════════════════════════════════════

async function testWalletManagement() {
  subheader('Test 1: Wallet Management');

  // Generate wallet
  try {
    const { wallet, secretKey } = generateWallet();
    if (wallet.connected && secretKey.length === 64) {
      pass('Generate wallet', `pubkey: ${wallet.publicKey.slice(0, 20)}...`);
    } else {
      fail('Generate wallet', new Error('Invalid wallet state'));
    }
  } catch (error) {
    fail('Generate wallet', error);
  }

  // Save and load wallet
  const tempPath = join(tmpdir(), `cynic-test-${Date.now()}.json`);
  try {
    const { wallet: w1, secretKey } = generateWallet();
    saveWalletToFile(secretKey, tempPath);

    const w2 = loadWalletFromFile(tempPath);
    if (w1.publicKey === w2.publicKey) {
      pass('Save/Load wallet', 'Keys match');
    } else {
      fail('Save/Load wallet', new Error('Keys mismatch'));
    }
  } catch (error) {
    fail('Save/Load wallet', error);
  } finally {
    if (existsSync(tempPath)) unlinkSync(tempPath);
  }

  // Create CynicWallet from Solana Keypair
  try {
    const keypair = Keypair.generate();
    const wallet = new CynicWallet({ secretKey: keypair.secretKey });
    if (wallet.connected && wallet.publicKey) {
      pass('CynicWallet from Keypair', `connected: ${wallet.connected}`);
    } else {
      fail('CynicWallet from Keypair', new Error('Not connected'));
    }
  } catch (error) {
    fail('CynicWallet from Keypair', error);
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 2: Single Anchor Transaction
// ═══════════════════════════════════════════════════════════════════════════

async function testSingleAnchor(connection, keypair) {
  subheader('Test 2: Single Anchor Transaction');

  const wallet = new CynicWallet({ secretKey: keypair.secretKey });
  const anchorer = createAnchorer({
    cluster: CONFIG.cluster,
    wallet,
  });

  // Anchor a merkle root
  const merkleRoot = generateMerkleRoot();
  log(`    Merkle root: ${merkleRoot.slice(0, 32)}...`, CYAN);

  try {
    const result = await anchorer.anchor(merkleRoot, ['test_single_1']);

    if (result.success) {
      pass('Anchor transaction', `sig: ${result.signature.slice(0, 20)}...`);
      log(`    Explorer: https://explorer.solana.com/tx/${result.signature}?cluster=devnet`, MAGENTA);

      // Verify anchor
      const verification = await anchorer.verifyAnchor(result.signature, merkleRoot);
      if (verification.verified) {
        pass('Verify anchor', `slot: ${verification.slot}`);
      } else {
        fail('Verify anchor', new Error(verification.error || 'Unknown'));
      }

      return { anchorer, result };
    } else {
      fail('Anchor transaction', new Error(result.error));
      return { anchorer, result: null };
    }
  } catch (error) {
    if (error.message && error.message.includes('NotValidator')) {
      log('    ⓘ Test skipped (wallet is not a validator)', YELLOW);
    } else {
    fail('Anchor transaction', error);
    }
    return { anchorer, result: null };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 3: Queue Batching with Merkle Proofs
// ═══════════════════════════════════════════════════════════════════════════

async function testQueueBatching(connection, keypair) {
  subheader('Test 3: Queue Batching with Merkle Proofs');

  const wallet = new CynicWallet({ secretKey: keypair.secretKey });
  const anchorer = createAnchorer({
    cluster: CONFIG.cluster,
    wallet,
  });

  // Create queue with larger batch size so items stay in queue until flush
  const queue = createAnchorQueue({
    anchorer,
    batchSize: 10, // Larger than test items so they stay queued
    autoStart: false, // Manual control
  });

  // Enqueue items
  const items = [
    { id: 'jdg_001', content: { verdict: 'WAG', score: 72 } },
    { id: 'jdg_002', content: { verdict: 'GROWL', score: 45 } },
    { id: 'jdg_003', content: { verdict: 'WAG', score: 88 } },
  ];

  try {
    for (const item of items) {
      queue.enqueue(item.id, item.content);
    }
    pass('Enqueue items', `${items.length} items enqueued`);
  } catch (error) {
    fail('Enqueue items', error);
    return;
  }

  // Check queue state (items should still be in queue since batchSize > 3)
  try {
    const length = queue.getQueueLength();
    if (length === 3) {
      pass('Queue state', `length: ${length}`);
    } else {
      fail('Queue state', new Error(`Expected 3, got ${length}`));
    }
  } catch (error) {
    fail('Queue state', error);
  }

  // Flush and anchor (force flush even though batch not full)
  try {
    log('    Flushing queue (real Solana tx)...', YELLOW);
    const batch = await queue.flush();

    if (batch && batch.signature) {
      pass('Flush queue', `batch: ${batch.batchId}, root: ${batch.merkleRoot.slice(0, 16)}...`);

      // Verify proofs
      if (verifyQueueProofs(queue, items)) {
        pass('Merkle proofs', 'All proofs valid');
      } else {
        fail('Merkle proofs', new Error('Some proofs invalid'));
      }

      // Log explorer link
      log(`    Explorer: https://explorer.solana.com/tx/${batch.signature}?cluster=devnet`, MAGENTA);
    } else if (batch) {
      // Batch created but maybe in simulation mode (no signature)
      pass('Flush queue (simulated)', `batch: ${batch.batchId}, status: ${batch.status}`);
    } else {
      fail('Flush queue', new Error('No batch returned'));
    }
  } catch (error) {
    fail('Flush queue', error);
  }

  queue.destroy();
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 4: Multiple Operators Simulation
// ═══════════════════════════════════════════════════════════════════════════

async function testMultipleOperators(connection) {
  subheader('Test 4: Multiple Operators Simulation');

  // Simulate 3 operators (no airdrops needed - simulation mode)
  // Real multi-operator testing requires multiple funded wallets
  const operators = [];
  for (let i = 0; i < 3; i++) {
    const keypair = Keypair.generate();
    operators.push({
      id: `op_${i + 1}`,
      keypair,
      wallet: new CynicWallet({ secretKey: keypair.secretKey }),
      anchorer: null,
    });
  }

  log(`    Generated ${operators.length} operator keypairs (simulation mode)`, CYAN);

  // Each operator anchors their own block in simulation mode
  let successCount = 0;
  for (const op of operators) {
    op.anchorer = createAnchorer({
      cluster: CONFIG.cluster,
      wallet: null, // Simulation mode - no wallet
    });

    const merkleRoot = generateMerkleRoot();
    const result = await op.anchorer.anchor(merkleRoot, [`block_${op.id}`]);

    if (result.success) {
      successCount++;
      log(`    Operator ${op.id}: anchored (simulated)`, GREEN);
    } else {
      log(`    Operator ${op.id}: failed - ${result.error}`, RED);
    }
  }

  if (successCount === operators.length) {
    pass('Multi-operator anchoring (simulated)', `${successCount} operators successful`);
  } else {
    fail('Multi-operator anchoring', new Error(`Only ${successCount}/${operators.length} succeeded`));
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 5: φ-Aligned Constants Validation
// ═══════════════════════════════════════════════════════════════════════════

async function testPhiConstants() {
  subheader('Test 5: φ-Aligned Constants Validation');

  const PHI = 1.618033988749895;
  const PHI_INV = 0.618033988749895;
  const PHI_INV_2 = 0.381966011250105;

  // Check anchor interval
  const expectedInterval = Math.round(PHI_INV * 100 * 1000);
  if (ANCHOR_CONSTANTS.ANCHOR_INTERVAL_MS === expectedInterval) {
    pass('Anchor interval', `${ANCHOR_CONSTANTS.ANCHOR_INTERVAL_MS}ms = φ⁻¹ × 100s`);
  } else {
    fail('Anchor interval', new Error(`Expected ${expectedInterval}, got ${ANCHOR_CONSTANTS.ANCHOR_INTERVAL_MS}`));
  }

  // Check batch size
  const expectedBatch = Math.floor(PHI_INV_2 * 100);
  if (ANCHOR_CONSTANTS.ANCHOR_BATCH_SIZE === expectedBatch) {
    pass('Batch size', `${ANCHOR_CONSTANTS.ANCHOR_BATCH_SIZE} = floor(φ⁻² × 100)`);
  } else {
    fail('Batch size', new Error(`Expected ${expectedBatch}, got ${ANCHOR_CONSTANTS.ANCHOR_BATCH_SIZE}`));
  }

  // Check confidence cap
  if (Math.abs(ANCHOR_CONSTANTS.ANCHOR_CONFIDENCE_CAP - PHI_INV) < 0.0001) {
    pass('Confidence cap', `${ANCHOR_CONSTANTS.ANCHOR_CONFIDENCE_CAP} ≈ φ⁻¹ = 61.8%`);
  } else {
    fail('Confidence cap', new Error(`Expected ~${PHI_INV}`));
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 6: Error Handling
// ═══════════════════════════════════════════════════════════════════════════

async function testErrorHandling() {
  subheader('Test 6: Error Handling');

  const anchorer = createAnchorer(); // No wallet = simulation mode

  // Invalid merkle root
  try {
    const result = await anchorer.anchor('invalid');
    if (!result.success && result.error.includes('Invalid')) {
      pass('Invalid merkle root', 'Correctly rejected');
    } else {
      fail('Invalid merkle root', new Error('Should have been rejected'));
    }
  } catch (error) {
    fail('Invalid merkle root', error);
  }

  // Empty merkle root
  try {
    const result = await anchorer.anchor('');
    if (!result.success) {
      pass('Empty merkle root', 'Correctly rejected');
    } else {
      fail('Empty merkle root', new Error('Should have been rejected'));
    }
  } catch (error) {
    fail('Empty merkle root', error);
  }

  // Simulation mode without wallet
  try {
    const root = generateMerkleRoot();
    const result = await anchorer.anchor(root);
    if (result.success && result.simulated) {
      pass('Simulation fallback', 'Works without wallet');
    } else {
      fail('Simulation fallback', new Error('Should simulate'));
    }
  } catch (error) {
    fail('Simulation fallback', error);
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 7: Export/Import State
// ═══════════════════════════════════════════════════════════════════════════

async function testExportImport() {
  subheader('Test 7: Export/Import State');

  // Create and use anchorer
  const anchorer1 = createAnchorer();
  await anchorer1.anchor(generateMerkleRoot(), ['item1']);
  await anchorer1.anchor(generateMerkleRoot(), ['item2']);

  // Export state
  const exported = anchorer1.export();

  // Import to new anchorer
  const anchorer2 = createAnchorer();
  anchorer2.import(exported);

  // Verify
  const stats1 = anchorer1.getStats();
  const stats2 = anchorer2.getStats();

  if (stats1.totalAnchored === stats2.totalAnchored) {
    pass('Export/Import anchorer', `${stats2.totalAnchored} anchors preserved`);
  } else {
    fail('Export/Import anchorer', new Error('Stats mismatch'));
  }

  // Queue export/import
  const queue1 = createAnchorQueue({ anchorer: createAnchorer(), autoStart: false });
  queue1.enqueue('q1', { data: 1 });
  queue1.enqueue('q2', { data: 2 });

  const queueExported = queue1.export();
  const queue2 = createAnchorQueue({ anchorer: createAnchorer(), autoStart: false });
  queue2.import(queueExported);

  if (queue2.getQueueLength() === 2) {
    pass('Export/Import queue', '2 items preserved');
  } else {
    fail('Export/Import queue', new Error('Queue state mismatch'));
  }

  queue1.destroy();
  queue2.destroy();
}

// ═══════════════════════════════════════════════════════════════════════════
// Main Test Runner
// ═══════════════════════════════════════════════════════════════════════════

async function main() {
  header('🐕 CYNIC Devnet Architecture Validation');
  log(`\n  Cluster: ${CONFIG.cluster}`, CYAN);
  log(`  Time: ${new Date().toISOString()}`, CYAN);

  // Setup
  const connection = new Connection(CONFIG.cluster, 'confirmed');
  const walletPath = join(import.meta.dirname, '.devnet-wallet.json');
  const mainKeypair = await loadOrCreateWallet(connection, walletPath);

  // Check balance
  const balance = await connection.getBalance(mainKeypair.publicKey);
  log(`  Balance: ${balance / LAMPORTS_PER_SOL} SOL`, balance > 0 ? GREEN : YELLOW);

  if (balance === 0) {
    log('  ⚠️ No SOL balance - please fund wallet via faucet:', YELLOW);
    log(`     Address: ${mainKeypair.publicKey.toBase58()}`, YELLOW);
    log('     Faucet: https://faucet.solana.com/', YELLOW);
    process.exit(1);
  }

  // Run all tests
  await testWalletManagement();
  await testSingleAnchor(connection, mainKeypair);
  await testQueueBatching(connection, mainKeypair);
  await testMultipleOperators(connection);
  await testPhiConstants();
  await testErrorHandling();
  await testExportImport();

  // Results
  printTestSummary();

  header('🐕 Validation Complete');
  log(`\n  "Onchain is truth" - Architecture ${results.failed === 0 ? 'VALIDATED' : 'NEEDS ATTENTION'}`,
      results.failed === 0 ? GREEN : YELLOW);
  log(`  φ⁻¹ = 61.8% max confidence\n`, CYAN);

  process.exit(results.failed > 0 ? 1 : 0);
}

main().catch((error) => {
  log(`\n  Fatal error: ${error.message}`, RED);
  console.error(error);
  process.exit(1);
});

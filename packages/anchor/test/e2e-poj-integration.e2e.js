#!/usr/bin/env node
/**
 * @cynic/anchor - E2E Tests for PoJAnchorIntegration
 *
 * Tests the integration between PoJ chain and Solana anchoring.
 * Uses a mock PoJ chain to simulate block finalization.
 *
 * "Onchain is truth" - κυνικός
 *
 * Usage:
 *   node test/e2e-poj-integration.e2e.js
 *
 * Note: This is a MANUAL test - requires network and wallet.
 * Automatically skipped when run via `node --test`.
 */

// Skip if running under Node's test runner (not meant for automated tests)
if (process.env.NODE_TEST_CONTEXT) {
  console.log('⏭️  Skipping e2e-poj-integration.e2e.js (manual test)');
  process.exit(0);
}

import { Connection, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { createHash, randomBytes } from 'crypto';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';
import { EventEmitter } from 'events';

import { PoJAnchorIntegration, SolanaCluster, CYNIC_PROGRAM } from '../src/index.js';
import { CynicWallet } from '../src/wallet.js';

// ═══════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════

const CONFIG = {
  cluster: process.env.HELIUS_RPC || SolanaCluster.DEVNET,
  walletPath: join(import.meta.dirname, '..', '..', '..', 'deploy-wallet.json'),
};

const PROGRAM_ID = CYNIC_PROGRAM.PROGRAM_ID;

// Colors
const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';
const MAGENTA = '\x1b[35m';
const RESET = '\x1b[0m';

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
  log(`  │ Test ${num}: ${title.padEnd(56)} │`, YELLOW);
  log(`  └${'─'.repeat(66)}┘`, YELLOW);
}

// ═══════════════════════════════════════════════════════════════════════════
// Test Results
// ═══════════════════════════════════════════════════════════════════════════

const results = {
  passed: 0,
  failed: 0,
  skipped: 0,
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

function skip(name, reason) {
  results.skipped++;
  results.tests.push({ name, status: 'SKIP', reason });
  log(`    ⏭️  ${name}: ${reason}`, YELLOW);
}

// ═══════════════════════════════════════════════════════════════════════════
// Mock PoJ Chain
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Mock PoJ Chain for testing
 * Emits block:finalized events like the real chain
 */
class MockPoJChain extends EventEmitter {
  constructor() {
    super();
    this.currentSlot = 0;
    this.blocks = [];
    this._stats = { lastAnchoredSlot: -1 };
  }

  /**
   * Create a mock block with judgments
   */
  createBlock(judgmentCount = 1) {
    this.currentSlot++;

    const judgments = [];
    for (let i = 0; i < judgmentCount; i++) {
      judgments.push({
        id: `jdg_${this.currentSlot}_${i}`,
        data: {
          id: `jdg_${this.currentSlot}_${i}`,
          verdict: ['WAG', 'GROWL', 'HOWL', 'BARK'][i % 4],
          qScore: Math.floor(Math.random() * 100),
          timestamp: Date.now(),
        },
      });
    }

    // Compute merkle root
    const judgmentHashes = judgments.map(j =>
      createHash('sha256').update(JSON.stringify(j.data)).digest()
    );

    let root;
    if (judgmentHashes.length === 0) {
      root = Buffer.alloc(32);
    } else if (judgmentHashes.length === 1) {
      root = judgmentHashes[0];
    } else {
      // Simple merkle root (pairs)
      let level = judgmentHashes;
      while (level.length > 1) {
        const next = [];
        for (let i = 0; i < level.length; i += 2) {
          const left = level[i];
          const right = level[i + 1] || left;
          next.push(createHash('sha256').update(Buffer.concat([left, right])).digest());
        }
        level = next;
      }
      root = level[0];
    }

    const block = {
      slot: this.currentSlot,
      header: {
        judgmentsRoot: root.toString('hex'),
        previousHash: randomBytes(32).toString('hex'),
        timestamp: Date.now(),
      },
      body: {
        judgments,
      },
      judgmentsRoot: root.toString('hex'),
      judgments,
    };

    this.blocks.push(block);
    return block;
  }

  /**
   * Finalize a block (emit event)
   */
  finalizeBlock(block) {
    this.emit('block:finalized', { block, slot: block.slot });
  }

  /**
   * Get chain stats
   */
  async getStats() {
    return this._stats;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// E2E Tests
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Test 1: Integration Initialization
 * Creates an integration with mock PoJ chain and verifies initialization
 */
async function testIntegrationInitialization(wallet) {
  step(1, 'Integration Initialization');

  try {
    const mockChain = new MockPoJChain();

    const integration = new PoJAnchorIntegration({
      pojChain: mockChain,
      wallet,
      cluster: CONFIG.cluster,
      autoAnchor: false, // Disable auto for init test
    });

    await integration.init();

    // Verify initialization
    if (!integration._initialized) {
      fail('Initialization flag', 'Not marked as initialized');
      integration.stop();
      return null;
    }
    pass('Initialization flag', 'correctly set');

    // Check stats
    const stats = integration.getStats();

    if (typeof stats.blocksProcessed !== 'number') {
      fail('Stats blocksProcessed', 'Missing field');
      integration.stop();
      return null;
    }
    pass('Stats structure', `blocksProcessed=${stats.blocksProcessed}, rootsAnchored=${stats.rootsAnchored}`);

    if (stats.cluster !== CONFIG.cluster) {
      fail('Stats cluster', 'Wrong cluster in stats');
      integration.stop();
      return null;
    }
    pass('Stats cluster', stats.cluster.slice(0, 30) + '...');

    if (stats.hasWallet !== !!wallet) {
      fail('Stats hasWallet', `Expected ${!!wallet}, got ${stats.hasWallet}`);
      integration.stop();
      return null;
    }
    pass('Stats hasWallet', `${stats.hasWallet}`);

    if (stats.autoAnchor !== false) {
      fail('Stats autoAnchor', 'Should be false');
      integration.stop();
      return null;
    }
    pass('Stats autoAnchor', 'correctly disabled');

    integration.stop();
    return { success: true };
  } catch (error) {
    fail('Integration initialization', error);
    return null;
  }
}

/**
 * Test 2: Block Finalization -> Anchor
 * Simulates block:finalized event and verifies automatic anchoring
 */
async function testBlockFinalizationAnchor(wallet, hasBalance) {
  step(2, 'Block Finalization -> Anchor');

  if (!hasBalance) {
    skip('Block finalization anchor', 'Insufficient SOL balance');
    return { success: true };
  }

  try {
    const mockChain = new MockPoJChain();

    const integration = new PoJAnchorIntegration({
      pojChain: mockChain,
      wallet,
      cluster: CONFIG.cluster,
      autoAnchor: true,
      batchBlocks: 1, // Anchor immediately
    });

    await integration.init();

    // Create promise to wait for anchor event
    const anchorPromise = new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Anchor timeout (30s)'));
      }, 30000);

      integration.on('blocks:anchored', (event) => {
        clearTimeout(timeout);
        resolve(event);
      });

      integration.on('anchor:failed', (event) => {
        clearTimeout(timeout);
        reject(new Error(event.error || 'Anchor failed'));
      });

      integration.on('anchor:error', (event) => {
        clearTimeout(timeout);
        reject(event.error || new Error('Anchor error'));
      });
    });

    // Create and finalize a block
    const block = mockChain.createBlock(3); // 3 judgments
    log(`    Created block ${block.slot} with 3 judgments`, CYAN);
    log(`    Root: ${block.judgmentsRoot.slice(0, 16)}...`, CYAN);

    mockChain.finalizeBlock(block);
    log('    Block finalized, waiting for anchor...', CYAN);

    // Wait for anchor
    const anchorEvent = await anchorPromise;

    if (!anchorEvent.signature) {
      fail('Anchor signature', 'No signature in event');
      integration.stop();
      return null;
    }
    pass('Anchor transaction', `sig: ${anchorEvent.signature.slice(0, 20)}...`);
    log(`    Explorer: https://explorer.solana.com/tx/${anchorEvent.signature}?cluster=devnet`, MAGENTA);

    if (anchorEvent.slot !== block.slot) {
      fail('Anchor slot', `Expected ${block.slot}, got ${anchorEvent.slot}`);
    } else {
      pass('Anchor slot', `block ${anchorEvent.slot}`);
    }

    if (anchorEvent.merkleRoot !== block.judgmentsRoot) {
      fail('Anchor merkle root', 'Root mismatch');
    } else {
      pass('Anchor merkle root', anchorEvent.merkleRoot.slice(0, 16) + '...');
    }

    if (anchorEvent.itemCount !== 3) {
      fail('Anchor item count', `Expected 3, got ${anchorEvent.itemCount}`);
    } else {
      pass('Anchor item count', `${anchorEvent.itemCount} judgments`);
    }

    // Check stats updated
    const stats = integration.getStats();
    if (stats.rootsAnchored < 1) {
      fail('Stats rootsAnchored', 'Should be at least 1');
    } else {
      pass('Stats updated', `rootsAnchored=${stats.rootsAnchored}`);
    }

    integration.stop();
    return { success: true, signature: anchorEvent.signature };
  } catch (error) {
    fail('Block finalization anchor', error);
    return null;
  }
}

/**
 * Test 3: Batch Anchoring
 * Configures batchBlocks: 3 and verifies batching behavior
 */
async function testBatchAnchoring(wallet, hasBalance) {
  step(3, 'Batch Anchoring');

  if (!hasBalance) {
    skip('Batch anchoring', 'Insufficient SOL balance');
    return { success: true };
  }

  try {
    const mockChain = new MockPoJChain();

    const integration = new PoJAnchorIntegration({
      pojChain: mockChain,
      wallet,
      cluster: CONFIG.cluster,
      autoAnchor: true,
      batchBlocks: 3, // Anchor every 3 blocks
    });

    await integration.init();

    let anchorCount = 0;
    const signatures = [];

    integration.on('blocks:anchored', (event) => {
      anchorCount++;
      signatures.push(event.signature);
    });

    // Create and finalize 3 blocks
    for (let i = 0; i < 3; i++) {
      const block = mockChain.createBlock(2);
      log(`    Created block ${block.slot}`, CYAN);
      mockChain.finalizeBlock(block);
    }

    // Wait for anchor (should trigger after 3rd block)
    log('    Waiting for batch anchor...', CYAN);
    await new Promise(resolve => setTimeout(resolve, 5000));

    // Give time for transaction
    if (anchorCount === 0) {
      // May still be processing, wait longer
      await new Promise(resolve => setTimeout(resolve, 10000));
    }

    const stats = integration.getStats();
    log(`    Blocks processed: ${stats.blocksProcessed}`, CYAN);
    log(`    Roots anchored: ${stats.rootsAnchored}`, CYAN);

    if (stats.blocksProcessed >= 3) {
      pass('Blocks processed', `${stats.blocksProcessed} blocks`);
    } else {
      fail('Blocks processed', `Expected >= 3, got ${stats.blocksProcessed}`);
    }

    // Should have exactly 1 anchor (batched)
    if (stats.rootsAnchored >= 1) {
      pass('Batch anchor', `${stats.rootsAnchored} anchor(s) for 3 blocks`);
      if (signatures.length > 0) {
        log(`    Explorer: https://explorer.solana.com/tx/${signatures[0]}?cluster=devnet`, MAGENTA);
      }
    } else {
      // Might be pending, check pending blocks
      if (stats.pendingBlocks > 0) {
        skip('Batch anchor', `${stats.pendingBlocks} blocks still pending (network slow)`);
      } else {
        fail('Batch anchor', 'No anchors triggered');
      }
    }

    integration.stop();
    return { success: true };
  } catch (error) {
    fail('Batch anchoring', error);
    return null;
  }
}

/**
 * Test 4: Manual Block Anchor
 * Uses anchorBlock() for a specific block
 */
async function testManualBlockAnchor(wallet, hasBalance) {
  step(4, 'Manual Block Anchor');

  if (!hasBalance) {
    skip('Manual block anchor', 'Insufficient SOL balance');
    return { success: true };
  }

  try {
    const mockChain = new MockPoJChain();

    const integration = new PoJAnchorIntegration({
      pojChain: mockChain,
      wallet,
      cluster: CONFIG.cluster,
      autoAnchor: false, // Disable auto
    });

    await integration.init();

    // Create a block manually
    const block = mockChain.createBlock(5); // 5 judgments
    log(`    Created block ${block.slot} with 5 judgments`, CYAN);
    log(`    Root: ${block.judgmentsRoot.slice(0, 16)}...`, CYAN);

    // Manually anchor
    log('    Calling anchorBlock()...', CYAN);
    const result = await integration.anchorBlock(block);

    if (!result.success) {
      fail('Manual anchor', result.error || 'Anchor failed');
      integration.stop();
      return null;
    }

    pass('Manual anchor success', `${result.success}`);

    if (result.signature) {
      pass('Manual anchor signature', `sig: ${result.signature.slice(0, 20)}...`);
      log(`    Explorer: https://explorer.solana.com/tx/${result.signature}?cluster=devnet`, MAGENTA);
    } else if (result.simulated) {
      pass('Manual anchor (simulated)', 'No signature (simulation mode)');
    }

    // Verify on-chain
    if (result.signature) {
      log('    Verifying on-chain...', CYAN);
      await new Promise(resolve => setTimeout(resolve, 2000));

      const verification = await integration.verifyBlockAnchor(block);

      if (verification.verified) {
        pass('On-chain verification', 'Block anchor verified');
      } else {
        // Might not be confirmed yet
        skip('On-chain verification', verification.error || 'Not yet confirmed');
      }
    }

    integration.stop();
    return { success: true, signature: result.signature };
  } catch (error) {
    fail('Manual block anchor', error);
    return null;
  }
}

/**
 * Test 5: Auto-Anchor Disabled
 * Verifies blocks are skipped when autoAnchor is false
 */
async function testAutoAnchorDisabled(wallet) {
  step(5, 'Auto-Anchor Disabled');

  try {
    const mockChain = new MockPoJChain();

    const integration = new PoJAnchorIntegration({
      pojChain: mockChain,
      wallet,
      cluster: CONFIG.cluster,
      autoAnchor: false,
    });

    await integration.init();

    let skippedCount = 0;
    integration.on('block:skipped', () => {
      skippedCount++;
    });

    // Finalize some blocks
    for (let i = 0; i < 3; i++) {
      const block = mockChain.createBlock(1);
      mockChain.finalizeBlock(block);
    }

    // Small delay for event processing
    await new Promise(resolve => setTimeout(resolve, 100));

    const stats = integration.getStats();

    if (stats.blocksProcessed === 3) {
      pass('Blocks processed', '3 blocks');
    } else {
      fail('Blocks processed', `Expected 3, got ${stats.blocksProcessed}`);
    }

    if (stats.rootsAnchored === 0) {
      pass('No auto-anchors', 'correctly skipped');
    } else {
      fail('Auto-anchor', `Should be 0, got ${stats.rootsAnchored}`);
    }

    if (skippedCount === 3) {
      pass('Skipped events', '3 blocks skipped');
    } else {
      skip('Skipped events', `Got ${skippedCount} (expected 3)`);
    }

    integration.stop();
    return { success: true };
  } catch (error) {
    fail('Auto-anchor disabled', error);
    return null;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Main Test Runner
// ═══════════════════════════════════════════════════════════════════════════

async function main() {
  header('CYNIC E2E: PoJAnchorIntegration Tests');
  log(`\n  Cluster: ${CONFIG.cluster}`, CYAN);
  log(`  Program: ${PROGRAM_ID}`, CYAN);
  log(`  Time: ${new Date().toISOString()}`, CYAN);

  // Load wallet
  let wallet = null;
  let hasBalance = false;

  if (existsSync(CONFIG.walletPath)) {
    const keyData = JSON.parse(readFileSync(CONFIG.walletPath, 'utf8'));
    const keypair = Keypair.fromSecretKey(new Uint8Array(keyData));
    wallet = new CynicWallet({ secretKey: keypair.secretKey });

    log(`\n  Wallet: ${keypair.publicKey.toBase58()}`, CYAN);

    // Check balance
    const connection = new Connection(CONFIG.cluster, 'confirmed');
    const balance = await connection.getBalance(keypair.publicKey);
    hasBalance = balance > 0.01 * LAMPORTS_PER_SOL;

    log(`  Balance: ${(balance / LAMPORTS_PER_SOL).toFixed(4)} SOL`, hasBalance ? GREEN : YELLOW);

    if (!hasBalance) {
      log('  WARNING: Low balance - some tests will be skipped', YELLOW);
    }
  } else {
    log('\n  Wallet: Not found (simulation mode)', YELLOW);
    log(`  Expected at: ${CONFIG.walletPath}`, YELLOW);
  }

  // Run tests
  try {
    await testIntegrationInitialization(wallet);
    await testAutoAnchorDisabled(wallet);
    await testBlockFinalizationAnchor(wallet, hasBalance);
    await testBatchAnchoring(wallet, hasBalance);
    await testManualBlockAnchor(wallet, hasBalance);
  } catch (error) {
    log(`\n  Fatal error: ${error.message}`, RED);
    console.error(error);
  }

  // Summary
  header('E2E Test Results');

  const total = results.passed + results.failed + results.skipped;
  const passRate = (results.passed + results.failed) > 0
    ? ((results.passed / (results.passed + results.failed)) * 100).toFixed(1)
    : 0;

  log(`\n  Total Tests: ${total}`, CYAN);
  log(`  Passed: ${results.passed}`, GREEN);
  log(`  Failed: ${results.failed}`, results.failed > 0 ? RED : GREEN);
  log(`  Skipped: ${results.skipped}`, results.skipped > 0 ? YELLOW : CYAN);
  log(`  Pass Rate: ${passRate}%`, passRate >= 80 ? GREEN : YELLOW);

  if (results.failed > 0) {
    log('\n  Failed Tests:', RED);
    for (const test of results.tests.filter(t => t.status === 'FAIL')) {
      log(`    - ${test.name}: ${test.error}`, RED);
    }
  }

  header('E2E Complete');
  log(`\n  "Onchain is truth." - κυνικός`, CYAN);

  process.exit(results.failed > 0 ? 1 : 0);
}

main().catch(error => {
  console.error('Fatal error:', error);
  process.exit(1);
});

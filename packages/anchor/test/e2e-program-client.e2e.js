#!/usr/bin/env node
/**
 * @cynic/anchor - E2E Tests for CynicProgramClient
 *
 * Tests real interaction with the CYNIC Anchor program on Solana devnet.
 * Requires deploy-wallet.json with SOL balance.
 *
 * "Onchain is truth" - κυνικός
 *
 * Usage:
 *   node test/e2e-program-client.e2e.js
 *
 * Note: This is a MANUAL test - requires network and wallet.
 * Automatically skipped when run via `node --test`.
 */

// Skip if running under Node's test runner (not meant for automated tests)
if (process.env.NODE_TEST_CONTEXT) {
  console.log('⏭️  Skipping e2e-program-client.e2e.js (manual test)');
  process.exit(0);
}

import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey } from '@solana/web3.js';
import { createHash, randomBytes } from 'crypto';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';

import { CynicProgramClient, SolanaCluster, CYNIC_PROGRAM } from '../src/index.js';
import { CynicWallet } from '../src/wallet.js';

// ═══════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════

const CONFIG = {
  // Use Helius RPC for reliability (no rate-limits)
  cluster: process.env.HELIUS_RPC || SolanaCluster.DEVNET,
  walletPath: join(import.meta.dirname, '..', '..', '..', 'deploy-wallet.json'),
};

// Known anchored root from devnet (for verification tests)
const KNOWN_ROOT = '83b4eea777b79645452a395c2eff8950dbe7d97f4cf95763fe0b142661971c47';
const PROGRAM_ID = CYNIC_PROGRAM.PROGRAM_ID;

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
// E2E Tests
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Test 1: Program State Query
 * Verifies getState() returns the deployed program state
 */
async function testProgramStateQuery(client) {
  step(1, 'Program State Query');

  try {
    const state = await client.getState();

    if (!state) {
      fail('State fetch', 'Program not initialized');
      return false;
    }

    // Verify required fields
    if (!state.authority) {
      fail('State authority', 'Missing authority field');
      return false;
    }
    pass('State authority', state.authority.slice(0, 12) + '...');

    if (typeof state.rootCount !== 'number') {
      fail('State rootCount', 'Missing rootCount field');
      return false;
    }
    pass('State rootCount', `${state.rootCount} roots anchored`);

    if (typeof state.validatorCount !== 'number') {
      fail('State validatorCount', 'Missing validatorCount field');
      return false;
    }
    pass('State validatorCount', `${state.validatorCount} validators`);

    if (!Array.isArray(state.validators)) {
      fail('State validators', 'Missing validators array');
      return false;
    }
    pass('State validators', state.validators.map(v => v.slice(0, 8) + '...').join(', ') || '(none)');

    if (typeof state.lastAnchorSlot !== 'number') {
      fail('State lastAnchorSlot', 'Missing lastAnchorSlot field');
      return false;
    }
    pass('State lastAnchorSlot', `slot ${state.lastAnchorSlot}`);

    return true;
  } catch (error) {
    fail('State query', error);
    return false;
  }
}

/**
 * Test 2: Root Verification (existing root)
 * Verifies verifyRoot() for an already anchored root
 */
async function testRootVerification(client) {
  step(2, 'Root Verification (existing root)');

  try {
    log(`    Verifying known root: ${KNOWN_ROOT.slice(0, 16)}...`, CYAN);

    const result = await client.verifyRoot(KNOWN_ROOT);

    if (!result.verified) {
      // Root might not exist on current devnet deployment
      skip('Known root verification', `Root not found: ${result.error || 'unknown'}`);
      return true; // Don't fail the test suite for this
    }

    pass('Root found on-chain', `index ${result.entry?.index}`);

    // Verify entry fields
    if (result.entry) {
      if (result.entry.merkleRoot === KNOWN_ROOT) {
        pass('Merkle root match', 'stored root matches query');
      } else {
        fail('Merkle root match', 'stored root differs from query');
      }

      if (typeof result.entry.itemCount === 'number') {
        pass('Item count', `${result.entry.itemCount} items`);
      }

      if (typeof result.entry.blockHeight === 'number') {
        pass('Block height', `PoJ block ${result.entry.blockHeight}`);
      }

      if (result.entry.validator) {
        pass('Validator', result.entry.validator.slice(0, 12) + '...');
      }

      if (result.entry.slot) {
        pass('Solana slot', `slot ${result.entry.slot}`);
      }
    }

    return true;
  } catch (error) {
    fail('Root verification', error);
    return false;
  }
}

/**
 * Test 3: Anchor New Root (real transaction)
 * Anchors a new root and verifies it on-chain
 */
async function testAnchorNewRoot(client, hasBalance) {
  step(3, 'Anchor New Root (real transaction)');

  if (!hasBalance) {
    skip('Anchor new root', 'Insufficient SOL balance');
    return true;
  }

  try {
    // Generate a unique merkle root
    const testData = {
      timestamp: Date.now(),
      nonce: randomBytes(16).toString('hex'),
      test: 'e2e-program-client',
    };
    const merkleRoot = createHash('sha256')
      .update(JSON.stringify(testData))
      .digest('hex');

    log(`    Anchoring root: ${merkleRoot.slice(0, 16)}...`, CYAN);

    const itemCount = 1;
    const blockHeight = Math.floor(Date.now() / 1000); // Use timestamp as mock block height

    const result = await client.anchorRoot(merkleRoot, itemCount, blockHeight);

    if (!result.signature) {
      fail('Anchor transaction', 'No signature returned');
      return false;
    }

    pass('Transaction sent', `sig: ${result.signature.slice(0, 20)}...`);
    log(`    Explorer: https://explorer.solana.com/tx/${result.signature}?cluster=devnet`, MAGENTA);

    if (result.slot) {
      pass('Transaction slot', `slot ${result.slot}`);
    }

    if (result.rootPda) {
      pass('Root PDA', result.rootPda.slice(0, 12) + '...');
    }

    // Verify the root on-chain
    log('    Verifying anchor on-chain...', CYAN);

    // Wait a moment for confirmation
    await new Promise(resolve => setTimeout(resolve, 2000));

    const verification = await client.verifyRoot(merkleRoot);

    if (verification.verified) {
      pass('On-chain verification', 'Root found on-chain');

      if (verification.entry) {
        if (verification.entry.blockHeight === blockHeight) {
          pass('Block height stored', `${blockHeight}`);
        }
        if (verification.entry.itemCount === itemCount) {
          pass('Item count stored', `${itemCount}`);
        }
      }
    } else {
      fail('On-chain verification', verification.error || 'Root not found');
    }

    return true;
  } catch (error) {
    fail('Anchor new root', error);
    return false;
  }
}

/**
 * Test 4: Validator Check
 * Verifies isValidator() for deploy wallet and random wallet
 */
async function testValidatorCheck(client, deployWalletPubkey) {
  step(4, 'Validator Check');

  try {
    // Check deploy wallet
    log(`    Checking deploy wallet: ${deployWalletPubkey.slice(0, 12)}...`, CYAN);
    const isDeployValidator = await client.isValidator(deployWalletPubkey);

    if (isDeployValidator) {
      pass('Deploy wallet is validator', 'correctly registered');
    } else {
      // This might be expected if not added as validator
      skip('Deploy wallet is validator', 'Not registered as validator (may be expected)');
    }

    // Check random wallet (should NOT be validator)
    const randomWallet = Keypair.generate().publicKey.toBase58();
    log(`    Checking random wallet: ${randomWallet.slice(0, 12)}...`, CYAN);
    const isRandomValidator = await client.isValidator(randomWallet);

    if (!isRandomValidator) {
      pass('Random wallet not validator', 'correctly returns false');
    } else {
      fail('Random wallet check', 'Random wallet should not be a validator');
    }

    return true;
  } catch (error) {
    fail('Validator check', error);
    return false;
  }
}

/**
 * Test 5: Invalid Root Verification
 * Verifies verifyRoot() returns false for non-existent root
 */
async function testInvalidRootVerification(client) {
  step(5, 'Invalid Root Verification');

  try {
    // Generate a random root that definitely doesn't exist
    const fakeRoot = randomBytes(32).toString('hex');
    log(`    Checking fake root: ${fakeRoot.slice(0, 16)}...`, CYAN);

    const result = await client.verifyRoot(fakeRoot);

    if (!result.verified) {
      pass('Fake root rejected', result.error || 'Not found');
    } else {
      fail('Fake root check', 'Should not have found random root');
    }

    return true;
  } catch (error) {
    fail('Invalid root verification', error);
    return false;
  }
}

/**
 * Test 6: Program Initialization Check
 */
async function testProgramInitialization(client) {
  step(6, 'Program Initialization Check');

  try {
    const isInit = await client.isInitialized();

    if (isInit) {
      pass('Program initialized', 'State account exists');
    } else {
      fail('Program initialization', 'Program not initialized');
      return false;
    }

    return true;
  } catch (error) {
    fail('Initialization check', error);
    return false;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Main Test Runner
// ═══════════════════════════════════════════════════════════════════════════

async function main() {
  header('CYNIC E2E: CynicProgramClient Tests');
  log(`\n  Cluster: ${CONFIG.cluster}`, CYAN);
  log(`  Program: ${PROGRAM_ID}`, CYAN);
  log(`  Time: ${new Date().toISOString()}`, CYAN);

  // Load wallet
  if (!existsSync(CONFIG.walletPath)) {
    log('\n  ERROR: deploy-wallet.json not found', RED);
    log(`  Expected at: ${CONFIG.walletPath}`, RED);
    process.exit(1);
  }

  const keyData = JSON.parse(readFileSync(CONFIG.walletPath, 'utf8'));
  const keypair = Keypair.fromSecretKey(new Uint8Array(keyData));
  const walletPubkey = keypair.publicKey.toBase58();

  log(`\n  Wallet: ${walletPubkey}`, CYAN);

  // Check balance
  const connection = new Connection(CONFIG.cluster, 'confirmed');
  const balance = await connection.getBalance(keypair.publicKey);
  const hasBalance = balance > 0.01 * LAMPORTS_PER_SOL;

  log(`  Balance: ${(balance / LAMPORTS_PER_SOL).toFixed(4)} SOL`, hasBalance ? GREEN : YELLOW);

  if (!hasBalance) {
    log('  WARNING: Low balance - some tests will be skipped', YELLOW);
  }

  // Create client
  const wallet = new CynicWallet({ secretKey: keypair.secretKey });
  const client = new CynicProgramClient({
    cluster: CONFIG.cluster,
    wallet,
    programId: PROGRAM_ID,
  });

  // Run tests
  try {
    await testProgramInitialization(client);
    await testProgramStateQuery(client);
    await testRootVerification(client);
    await testAnchorNewRoot(client, hasBalance);
    await testValidatorCheck(client, walletPubkey);
    await testInvalidRootVerification(client);
  } catch (error) {
    log(`\n  Fatal error: ${error.message}`, RED);
    console.error(error);
  }

  // Summary
  header('E2E Test Results');

  const total = results.passed + results.failed + results.skipped;
  const passRate = total > 0 ? ((results.passed / (results.passed + results.failed)) * 100).toFixed(1) : 0;

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

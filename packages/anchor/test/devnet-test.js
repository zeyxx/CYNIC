#!/usr/bin/env node
/**
 * @cynic/anchor - Devnet Integration Test
 *
 * Tests real Solana anchoring on devnet.
 *
 * "Onchain is truth" - κυνικός
 *
 * Usage:
 *   node test/devnet-test.js
 */

import { Connection, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import {
  createAnchorer,
  createAnchorQueue,
  SolanaCluster,
  CynicWallet,
  generateWallet,
} from '../src/index.js';

// Colors for output
const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';
const RESET = '\x1b[0m';

function log(msg, color = RESET) {
  console.log(`${color}${msg}${RESET}`);
}

async function main() {
  log('\n═══════════════════════════════════════════════════════════', CYAN);
  log('  🐕 CYNIC Devnet Anchor Test - "Onchain is truth"', CYAN);
  log('═══════════════════════════════════════════════════════════\n', CYAN);

  // Step 1: Generate keypair
  log('📝 Step 1: Generating devnet keypair...', YELLOW);
  const keypair = Keypair.generate();
  const publicKey = keypair.publicKey.toBase58();
  log(`   Public Key: ${publicKey}`, GREEN);

  // Step 2: Request airdrop (try multiple endpoints)
  log('\n💰 Step 2: Requesting devnet SOL airdrop...', YELLOW);

  // Try Helius devnet RPC if available (more reliable, no rate-limits)
  const rpcEndpoints = [
    SolanaCluster.HELIUS_DEVNET, // Helius (if HELIUS_API_KEY set)
    SolanaCluster.DEVNET,        // Fallback to public devnet
  ].filter(Boolean);

  let connection;
  let airdropSuccess = false;

  for (const endpoint of rpcEndpoints) {
    log(`   Trying endpoint: ${endpoint.slice(0, 40)}...`, YELLOW);
    connection = new Connection(endpoint, 'confirmed');

    try {
      const airdropSig = await connection.requestAirdrop(
        keypair.publicKey,
        LAMPORTS_PER_SOL // 1 SOL
      );
      log(`   Airdrop signature: ${airdropSig.slice(0, 20)}...`, GREEN);

      // Wait for confirmation
      log('   Waiting for airdrop confirmation...', YELLOW);
      const latestBlockhash = await connection.getLatestBlockhash();
      await connection.confirmTransaction({
        signature: airdropSig,
        ...latestBlockhash,
      });

      const balance = await connection.getBalance(keypair.publicKey);
      log(`   Balance: ${balance / LAMPORTS_PER_SOL} SOL ✅`, GREEN);
      airdropSuccess = true;
      break;
    } catch (error) {
      log(`   ⚠️ Airdrop failed on this endpoint: ${error.message.slice(0, 50)}`, RED);
    }
  }

  if (!airdropSuccess) {
    log('   All airdrop attempts failed. Need SOL to continue.', RED);
    log('   Try: solana airdrop 1 --url devnet', YELLOW);

    // Check if we have any balance anyway
    const balance = await connection.getBalance(keypair.publicKey);
    if (balance === 0) {
      log('\n   Cannot proceed without SOL. Exiting.', RED);
      process.exit(1);
    }
  }

  // Step 3: Create CynicWallet
  log('\n🔐 Step 3: Creating CynicWallet...', YELLOW);
  const wallet = new CynicWallet({
    secretKey: keypair.secretKey,
  });
  log(`   Wallet connected: ${wallet.connected}`, GREEN);
  log(`   Wallet public key: ${wallet.publicKey}`, GREEN);

  // Step 4: Create anchorer
  log('\n⚓ Step 4: Creating anchorer with devnet...', YELLOW);
  const anchorer = createAnchorer({
    cluster: SolanaCluster.DEVNET,
    wallet,
    onAnchor: (record) => {
      log(`   📌 Anchor complete: ${record.signature?.slice(0, 20)}...`, GREEN);
    },
    onError: (record, error) => {
      log(`   ❌ Anchor error: ${error.message}`, RED);
    },
  });
  log(`   Anchorer created for cluster: ${anchorer.cluster}`, GREEN);

  // Step 5: Test anchor
  log('\n🚀 Step 5: Sending anchor transaction...', YELLOW);

  // Create a test merkle root (64 hex chars)
  const testMerkleRoot =
    'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2';

  log(`   Merkle root: ${testMerkleRoot.slice(0, 20)}...`, CYAN);

  try {
    const result = await anchorer.anchor(testMerkleRoot, ['test_item_1']);

    if (result.success) {
      log('\n✅ ANCHOR SUCCESS!', GREEN);
      log(`   Signature: ${result.signature}`, GREEN);
      log(`   Slot: ${result.slot}`, GREEN);
      log(`   Timestamp: ${new Date(result.timestamp).toISOString()}`, GREEN);

      // Step 6: Verify anchor
      log('\n🔍 Step 6: Verifying anchor on chain...', YELLOW);
      const verification = await anchorer.verifyAnchor(
        result.signature,
        testMerkleRoot
      );

      if (verification.verified) {
        log('✅ VERIFICATION SUCCESS!', GREEN);
        log(`   Slot: ${verification.slot}`, GREEN);
        log(`   Block time: ${verification.blockTime}`, GREEN);
      } else {
        log(`❌ Verification failed: ${verification.error}`, RED);
      }

      // Explorer link
      log('\n🔗 View on Solana Explorer:', CYAN);
      log(
        `   https://explorer.solana.com/tx/${result.signature}?cluster=devnet`,
        CYAN
      );
    } else {
      log(`\n❌ ANCHOR FAILED: ${result.error}`, RED);
    }
  } catch (error) {
    log(`\n❌ Error: ${error.message}`, RED);
    if (error.message.includes('insufficient lamports')) {
      log('   → Need SOL for transaction fees. Try airdrop again.', YELLOW);
    }
  }

  // Stats
  log('\n📊 Anchorer Stats:', YELLOW);
  const stats = anchorer.getStats();
  log(`   Total anchored: ${stats.totalAnchored}`, CYAN);
  log(`   Total failed: ${stats.totalFailed}`, CYAN);
  log(`   Has wallet: ${stats.hasWallet}`, CYAN);

  log('\n═══════════════════════════════════════════════════════════', CYAN);
  log('  🐕 Test complete - "Onchain is truth"', CYAN);
  log('═══════════════════════════════════════════════════════════\n', CYAN);
}

main().catch(console.error);

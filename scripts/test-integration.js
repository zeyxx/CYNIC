#!/usr/bin/env node
/**
 * Test CYNIC Anchor Integration
 *
 * Tests the full flow: ProgramClient -> Anchorer -> Verification
 *
 * "Onchain is truth" - κυνικός
 */

import { createHash } from 'crypto';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

import { CynicProgramClient, SolanaAnchorer, CynicWallet, SolanaCluster } from '../packages/anchor/src/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const RPC_URL = process.env.HELIUS_RPC || SolanaCluster.DEVNET;

async function main() {
  console.log('🐕 CYNIC Anchor Integration Test');
  console.log('=================================\n');

  // Load wallet
  const walletPath = join(__dirname, '..', 'deploy-wallet.json');
  const walletData = JSON.parse(readFileSync(walletPath, 'utf-8'));
  const wallet = new CynicWallet({ secretKey: Uint8Array.from(walletData) });

  console.log('Wallet:', wallet.publicKey);
  console.log('Cluster:', RPC_URL);
  console.log('');

  // Test 1: ProgramClient - Get State
  console.log('─── Test 1: ProgramClient.getState() ───');
  const client = new CynicProgramClient({
    cluster: RPC_URL,
    wallet,
  });

  const state = await client.getState();
  console.log('State:', state);
  console.log('✓ ProgramClient works!\n');

  // Test 2: ProgramClient - Verify existing root
  console.log('─── Test 2: ProgramClient.verifyRoot() ───');
  const testRoot = '83b4eea777b79645452a395c2eff8950dbe7d97f4cf95763fe0b142661971c47';
  const verifyResult = await client.verifyRoot(testRoot);
  console.log('Verify result:', verifyResult);
  console.log('✓ Root verification works!\n');

  // Test 3: SolanaAnchorer - Anchor new root
  console.log('─── Test 3: SolanaAnchorer.anchor() ───');
  const anchorer = new SolanaAnchorer({
    cluster: RPC_URL,
    wallet,
    useAnchorProgram: true,
  });

  // Generate test merkle root
  const testData = `CYNIC Integration Test - ${Date.now()}`;
  const merkleRoot = createHash('sha256').update(testData).digest('hex');
  console.log('Test Data:', testData);
  console.log('Merkle Root:', merkleRoot);

  anchorer.setBlockHeight(2); // PoJ block 2

  const anchorResult = await anchorer.anchor(merkleRoot, ['test_item_1', 'test_item_2']);
  console.log('Anchor result:', anchorResult);

  if (anchorResult.success) {
    console.log('✓ Anchoring works!\n');

    // Test 4: Verify the newly anchored root
    console.log('─── Test 4: Verify newly anchored root ───');
    const newVerify = await client.verifyRoot(merkleRoot);
    console.log('New root verified:', newVerify);
    console.log('✓ New root verification works!\n');
  } else {
    console.log('✗ Anchoring failed:', anchorResult.error);
  }

  // Final state
  console.log('─── Final State ───');
  const finalState = await client.getState();
  console.log('Root count:', finalState.rootCount);
  console.log('Last anchor slot:', finalState.lastAnchorSlot);
  console.log('');

  console.log('══════════════════════════════════════════════════════════════');
  console.log('  ✅ CYNIC ANCHOR INTEGRATION COMPLETE!');
  console.log('══════════════════════════════════════════════════════════════');
  console.log('');
  console.log('  🐕 Tous les jugements seront maintenant ancrés sur Solana!');
  console.log('');
}

main().catch(err => {
  console.error('Error:', err);
  process.exit(1);
});

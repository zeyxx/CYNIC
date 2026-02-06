#!/usr/bin/env node
/**
 * Setup Devnet Validators
 *
 * Generates 5 Solana keypairs, airdrops SOL, registers on-chain.
 * Idempotent - safe to run multiple times.
 *
 * Usage: node scripts/setup-devnet-validators.js
 *
 * "The pack registers on-chain" - κυνικός
 */

import { existsSync, mkdirSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import {
  generateWallet,
  saveWalletToFile,
  loadWalletFromFile,
  CynicProgramClient,
  SolanaCluster,
} from '@cynic/anchor';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, '..');

const VALIDATOR_COUNT = 5;
const VALIDATORS_DIR = join(ROOT, 'validators');
const AUTHORITY_PATH = join(ROOT, 'deploy-wallet.json');
const AIRDROP_SOL = 1;
const AIRDROP_LAMPORTS = AIRDROP_SOL * 1_000_000_000;
const RPC_URL = process.env.HELIUS_RPC || SolanaCluster.DEVNET;

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function airdropWithRetry(connection, pubkey, lamports, maxRetries = 3) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const sig = await connection.requestAirdrop(pubkey, lamports);
      await connection.confirmTransaction(sig, 'confirmed');
      return sig;
    } catch (err) {
      if (attempt === maxRetries) throw err;
      const delay = 2000 * attempt; // linear backoff
      console.log(`    Airdrop attempt ${attempt} failed, retrying in ${delay / 1000}s...`);
      await sleep(delay);
    }
  }
}

async function main() {
  console.log('═══════════════════════════════════════════════════════════');
  console.log('  CYNIC Multi-Validator Devnet Setup');
  console.log('  "The pack registers on-chain"');
  console.log('═══════════════════════════════════════════════════════════\n');

  // ── Check authority wallet ─────────────────────────────────────────────
  if (!existsSync(AUTHORITY_PATH)) {
    console.error('ERROR: deploy-wallet.json not found at:', AUTHORITY_PATH);
    console.error('');
    console.error('Generate one with:');
    console.error('  solana-keygen new --outfile deploy-wallet.json');
    console.error('  solana airdrop 2 --keypair deploy-wallet.json --url devnet');
    process.exit(1);
  }

  const authorityWallet = loadWalletFromFile(AUTHORITY_PATH);
  console.log('Authority:', authorityWallet.publicKey);
  console.log('RPC:      ', RPC_URL);
  console.log('');

  // ── Create program client ──────────────────────────────────────────────
  const client = new CynicProgramClient({
    cluster: RPC_URL,
    wallet: authorityWallet,
  });

  // Verify program is initialized
  const state = await client.getState();
  if (!state) {
    console.error('ERROR: CYNIC program not initialized on-chain.');
    console.error('Run: node scripts/initialize.js');
    process.exit(1);
  }
  console.log('On-chain state:');
  console.log('  Authority:      ', state.authority);
  console.log('  Validator count:', state.validatorCount);
  console.log('  Root count:     ', state.rootCount);
  console.log('');

  // ── Generate / load validator keypairs ──────────────────────────────────
  if (!existsSync(VALIDATORS_DIR)) {
    mkdirSync(VALIDATORS_DIR, { recursive: true });
  }

  const validators = [];

  console.log('── VALIDATOR KEYPAIRS ──────────────────────────────────────\n');

  for (let i = 0; i < VALIDATOR_COUNT; i++) {
    const path = join(VALIDATORS_DIR, `validator-${i}.json`);

    let wallet;
    if (existsSync(path)) {
      wallet = loadWalletFromFile(path);
      console.log(`  [${i}] Loaded existing:  ${wallet.publicKey}`);
    } else {
      const { wallet: newWallet, secretKey } = generateWallet();
      saveWalletToFile(secretKey, path);
      wallet = newWallet;
      console.log(`  [${i}] Generated new:    ${wallet.publicKey}`);
    }

    validators.push({ index: i, wallet, path });
  }
  console.log('');

  // ── Airdrop SOL ────────────────────────────────────────────────────────
  console.log('── AIRDROP ────────────────────────────────────────────────\n');

  // Lazy-load @solana/web3.js for airdrop (ProgramClient handles its own)
  const { Connection, PublicKey } = await import('@solana/web3.js');
  const connection = new Connection(RPC_URL, 'confirmed');

  for (const { index, wallet } of validators) {
    const pubkey = new PublicKey(wallet.publicKey);
    const balance = await connection.getBalance(pubkey);
    const balanceSol = balance / 1_000_000_000;

    if (balanceSol >= 0.5) {
      console.log(`  [${index}] Balance: ${balanceSol.toFixed(3)} SOL (sufficient)`);
      continue;
    }

    try {
      console.log(`  [${index}] Airdropping ${AIRDROP_SOL} SOL...`);
      await airdropWithRetry(connection, pubkey, AIRDROP_LAMPORTS);
      const newBalance = await connection.getBalance(pubkey);
      console.log(`  [${index}] Balance: ${(newBalance / 1_000_000_000).toFixed(3)} SOL`);
    } catch (err) {
      console.error(`  [${index}] Airdrop failed: ${err.message}`);
      console.error('         (Devnet may be rate-limiting. Try again later.)');
    }

    // Rate limit: 2s between airdrops
    await sleep(2000);
  }
  console.log('');

  // ── Register validators on-chain ──────────────────────────────────────
  console.log('── ON-CHAIN REGISTRATION ──────────────────────────────────\n');

  let registered = 0;
  let skipped = 0;
  let failed = 0;

  for (const { index, wallet } of validators) {
    const pubkey = wallet.publicKey;

    // Idempotent: skip if already registered
    const alreadyRegistered = await client.isValidator(pubkey);
    if (alreadyRegistered) {
      console.log(`  [${index}] Already registered: ${pubkey.slice(0, 20)}...`);
      skipped++;
      continue;
    }

    try {
      console.log(`  [${index}] Registering: ${pubkey.slice(0, 20)}...`);
      const { signature } = await client.addValidator(pubkey);
      console.log(`  [${index}] TX: ${signature.slice(0, 32)}...`);
      registered++;
    } catch (err) {
      console.error(`  [${index}] Registration FAILED: ${err.message}`);
      if (err.logs) {
        err.logs.forEach(log => console.error(`         ${log}`));
      }
      failed++;
    }

    // Small delay between TXs
    await sleep(500);
  }
  console.log('');

  // ── Verify final state ────────────────────────────────────────────────
  console.log('── VERIFICATION ───────────────────────────────────────────\n');

  const finalState = await client.getState();
  console.log('  Validator count:', finalState.validatorCount);
  console.log('  Validators:');
  for (let i = 0; i < finalState.validators.length; i++) {
    console.log(`    [${i}] ${finalState.validators[i]}`);
  }
  console.log('');

  // ── Summary ───────────────────────────────────────────────────────────
  const success = finalState.validatorCount >= VALIDATOR_COUNT;

  console.log('═══════════════════════════════════════════════════════════');
  console.log(`  ${success ? 'SETUP COMPLETE' : 'SETUP PARTIAL'}`);
  console.log(`  Registered: ${registered} new, ${skipped} existed, ${failed} failed`);
  console.log(`  On-chain:   ${finalState.validatorCount} / ${VALIDATOR_COUNT} validators`);
  console.log('═══════════════════════════════════════════════════════════\n');

  if (!success) {
    console.error('WARNING: Not all validators registered. Check errors above.');
    process.exit(1);
  }
}

main().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});

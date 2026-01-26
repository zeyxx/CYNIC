#!/usr/bin/env node
/**
 * Initialize CYNIC Anchor Program on Devnet
 *
 * Usage: node scripts/init-devnet.js
 *
 * "Onchain is truth" - κυνικός
 */

import { Connection, Keypair, PublicKey, Transaction, TransactionInstruction, sendAndConfirmTransaction, SystemProgram } from '@solana/web3.js';
import { createHash } from 'crypto';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PROGRAM_ID = new PublicKey('G3Yana4ukbevyoVNSWrXgRQtQqHYMnPEMi1xvpp9CqBY');
const RPC_URL = process.env.HELIUS_RPC || 'https://api.devnet.solana.com';

/**
 * Compute Anchor instruction discriminator
 * @param {string} name - Instruction name (e.g., "initialize")
 * @returns {Buffer}
 */
function getDiscriminator(name) {
  const preimage = `global:${name}`;
  const hash = createHash('sha256').update(preimage).digest();
  return hash.slice(0, 8);
}

async function main() {
  console.log('🐕 CYNIC Anchor Initialization');
  console.log('================================\n');

  // Load wallet
  const walletPath = join(__dirname, '..', 'deploy-wallet.json');
  const walletData = JSON.parse(readFileSync(walletPath, 'utf-8'));
  const authority = Keypair.fromSecretKey(Uint8Array.from(walletData));

  console.log('Authority:', authority.publicKey.toBase58());
  console.log('Program ID:', PROGRAM_ID.toBase58());
  console.log('RPC:', RPC_URL);
  console.log('');

  // Connect
  const connection = new Connection(RPC_URL, 'confirmed');

  // Derive state PDA
  const [statePda, stateBump] = PublicKey.findProgramAddressSync(
    [Buffer.from('cynic_state')],
    PROGRAM_ID
  );
  console.log('State PDA:', statePda.toBase58());
  console.log('State Bump:', stateBump);
  console.log('');

  // Check if already initialized
  const stateAccount = await connection.getAccountInfo(statePda);
  if (stateAccount) {
    console.log('✓ Program already initialized!');
    console.log('  Owner:', stateAccount.owner.toBase58());
    console.log('  Data length:', stateAccount.data.length, 'bytes');
    console.log('  Lamports:', stateAccount.lamports);
    return;
  }

  console.log('Initializing program...\n');

  // Build initialize instruction
  const discriminator = getDiscriminator('initialize');
  console.log('Discriminator:', discriminator.toString('hex'));

  const initIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: statePda, isSigner: false, isWritable: true },
      { pubkey: authority.publicKey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: discriminator,
  });

  const tx = new Transaction().add(initIx);

  // Get recent blockhash
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;
  tx.feePayer = authority.publicKey;

  try {
    const sig = await sendAndConfirmTransaction(connection, tx, [authority], {
      commitment: 'confirmed',
    });

    console.log('✓ Program initialized!');
    console.log('');
    console.log('  Signature:', sig);
    console.log('  State PDA:', statePda.toBase58());
    console.log('  Explorer: https://explorer.solana.com/tx/' + sig + '?cluster=devnet');
    console.log('');
    console.log('🐕 κυνικός awakens on-chain! *tail wag*');
  } catch (err) {
    console.error('Error:', err.message);
    if (err.logs) {
      console.error('\nProgram logs:');
      err.logs.forEach(log => console.error('  ', log));
    }
    process.exit(1);
  }
}

main().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});

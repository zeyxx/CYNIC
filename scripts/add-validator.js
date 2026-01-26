#!/usr/bin/env node
/**
 * Add a validator to CYNIC Anchor Program
 *
 * Usage: node scripts/add-validator.js [validator_pubkey]
 * If no pubkey provided, uses the deploy wallet
 *
 * "Onchain is truth" - κυνικός
 */

import { Connection, Keypair, PublicKey, Transaction, TransactionInstruction, sendAndConfirmTransaction } from '@solana/web3.js';
import { createHash } from 'crypto';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PROGRAM_ID = new PublicKey('G3Yana4ukbevyoVNSWrXgRQtQqHYMnPEMi1xvpp9CqBY');
const RPC_URL = process.env.HELIUS_RPC || 'https://api.devnet.solana.com';

async function main() {
  // Load authority wallet
  const walletPath = join(__dirname, '..', 'deploy-wallet.json');
  const walletData = JSON.parse(readFileSync(walletPath, 'utf-8'));
  const authority = Keypair.fromSecretKey(Uint8Array.from(walletData));

  // Validator to add (default: authority itself)
  const validatorPubkey = process.argv[2]
    ? new PublicKey(process.argv[2])
    : authority.publicKey;

  console.log('🐕 CYNIC Add Validator');
  console.log('======================\n');
  console.log('Authority:', authority.publicKey.toBase58());
  console.log('Validator:', validatorPubkey.toBase58());
  console.log('');

  const connection = new Connection(RPC_URL, 'confirmed');

  // State PDA
  const [statePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('cynic_state')],
    PROGRAM_ID
  );

  // Discriminator for add_validator
  const discriminator = createHash('sha256').update('global:add_validator').digest().slice(0, 8);

  // Instruction data = discriminator + validator pubkey
  const data = Buffer.concat([discriminator, validatorPubkey.toBuffer()]);

  const addValidatorIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: statePda, isSigner: false, isWritable: true },
      { pubkey: authority.publicKey, isSigner: true, isWritable: false },
    ],
    data: data,
  });

  const tx = new Transaction().add(addValidatorIx);

  try {
    const sig = await sendAndConfirmTransaction(connection, tx, [authority], {
      commitment: 'confirmed'
    });

    console.log('✓ Validator added!');
    console.log('');
    console.log('  Signature:', sig);
    console.log('  Explorer: https://explorer.solana.com/tx/' + sig + '?cluster=devnet');
    console.log('');
    console.log('🐕 *tail wag*');
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

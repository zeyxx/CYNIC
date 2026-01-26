#!/usr/bin/env node
/**
 * Test anchoring a merkle root on CYNIC Anchor Program
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

async function main() {
  console.log('🐕 CYNIC Anchor Root Test');
  console.log('=========================\n');

  // Load validator wallet
  const walletPath = join(__dirname, '..', 'deploy-wallet.json');
  const walletData = JSON.parse(readFileSync(walletPath, 'utf-8'));
  const validator = Keypair.fromSecretKey(Uint8Array.from(walletData));

  const connection = new Connection(RPC_URL, 'confirmed');

  // Create a test merkle root (hash of "CYNIC PoJ Test")
  const testData = `CYNIC PoJ Test - ${Date.now()}`;
  const merkleRoot = createHash('sha256').update(testData).digest();

  console.log('Test Data:', testData);
  console.log('Merkle Root:', merkleRoot.toString('hex'));
  console.log('Validator:', validator.publicKey.toBase58());
  console.log('');

  // State PDA
  const [statePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('cynic_state')],
    PROGRAM_ID
  );

  // Root entry PDA
  const [rootPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('root'), merkleRoot],
    PROGRAM_ID
  );

  console.log('State PDA:', statePda.toBase58());
  console.log('Root PDA:', rootPda.toBase58());
  console.log('');

  // Discriminator for anchor_root
  const discriminator = createHash('sha256').update('global:anchor_root').digest().slice(0, 8);

  // Instruction data:
  // - discriminator (8 bytes)
  // - merkle_root ([u8; 32])
  // - item_count (u32 LE)
  // - block_height (u64 LE)
  const itemCount = 42; // Test: 42 judgments
  const blockHeight = 1n; // PoJ block 1

  const data = Buffer.alloc(8 + 32 + 4 + 8);
  let offset = 0;

  discriminator.copy(data, offset);
  offset += 8;

  merkleRoot.copy(data, offset);
  offset += 32;

  data.writeUInt32LE(itemCount, offset);
  offset += 4;

  data.writeBigUInt64LE(blockHeight, offset);

  const anchorRootIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: statePda, isSigner: false, isWritable: true },
      { pubkey: rootPda, isSigner: false, isWritable: true },
      { pubkey: validator.publicKey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: data,
  });

  const tx = new Transaction().add(anchorRootIx);

  try {
    console.log('Anchoring root...\n');

    const sig = await sendAndConfirmTransaction(connection, tx, [validator], {
      commitment: 'confirmed'
    });

    console.log('✅ ROOT ANCHORED!');
    console.log('');
    console.log('  Merkle Root:', merkleRoot.toString('hex'));
    console.log('  Item Count:', itemCount);
    console.log('  Block Height:', blockHeight.toString());
    console.log('  Root PDA:', rootPda.toBase58());
    console.log('');
    console.log('  Signature:', sig);
    console.log('  Explorer: https://explorer.solana.com/tx/' + sig + '?cluster=devnet');
    console.log('');
    console.log('🐕 *HOWL* First judgment anchored on Solana!');

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

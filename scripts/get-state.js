#!/usr/bin/env node
/**
 * Get CYNIC Anchor Program State
 *
 * "Onchain is truth" - κυνικός
 */

import { Connection, PublicKey } from '@solana/web3.js';

const PROGRAM_ID = new PublicKey('G3Yana4ukbevyoVNSWrXgRQtQqHYMnPEMi1xvpp9CqBY');
const RPC_URL = process.env.HELIUS_RPC || 'https://api.devnet.solana.com';

async function main() {
  const connection = new Connection(RPC_URL, 'confirmed');

  // State PDA
  const [statePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('cynic_state')],
    PROGRAM_ID
  );

  console.log('🐕 CYNIC Anchor State');
  console.log('=====================\n');
  console.log('Program ID:', PROGRAM_ID.toBase58());
  console.log('State PDA:', statePda.toBase58());
  console.log('');

  const account = await connection.getAccountInfo(statePda);
  if (!account) {
    console.log('❌ Program not initialized');
    return;
  }

  // Parse state (Anchor account format: 8-byte discriminator + data)
  const data = account.data;

  // Skip 8-byte discriminator
  let offset = 8;

  // authority: Pubkey (32 bytes)
  const authority = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;

  // initialized_at: i64 (8 bytes)
  const initializedAt = Number(data.readBigInt64LE(offset));
  offset += 8;

  // root_count: u64 (8 bytes)
  const rootCount = Number(data.readBigUInt64LE(offset));
  offset += 8;

  // validator_count: u8 (1 byte)
  const validatorCount = data.readUInt8(offset);
  offset += 1;

  // validators: [Pubkey; 21] (21 * 32 = 672 bytes)
  const validators = [];
  for (let i = 0; i < validatorCount; i++) {
    const validator = new PublicKey(data.slice(offset + i * 32, offset + (i + 1) * 32));
    validators.push(validator.toBase58());
  }
  offset += 21 * 32;

  // last_anchor_slot: u64 (8 bytes)
  const lastAnchorSlot = Number(data.readBigUInt64LE(offset));
  offset += 8;

  // bump: u8 (1 byte)
  const bump = data.readUInt8(offset);

  console.log('══════════════════════════════════════════════════════════════');
  console.log('  CYNIC STATE');
  console.log('══════════════════════════════════════════════════════════════');
  console.log('');
  console.log('  Authority:        ', authority.toBase58());
  console.log('  Initialized:      ', new Date(initializedAt * 1000).toISOString());
  console.log('  Root Count:       ', rootCount);
  console.log('  Validator Count:  ', validatorCount, '/ 21');
  console.log('  Last Anchor Slot: ', lastAnchorSlot || 'none');
  console.log('  Bump:             ', bump);
  console.log('');
  console.log('  Validators:');
  if (validators.length === 0) {
    console.log('    (none)');
  } else {
    validators.forEach((v, i) => console.log(`    ${i + 1}. ${v}`));
  }
  console.log('');
  console.log('══════════════════════════════════════════════════════════════');
}

main().catch(err => {
  console.error('Error:', err);
  process.exit(1);
});

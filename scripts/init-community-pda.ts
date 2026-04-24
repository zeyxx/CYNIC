/**
 * CYNIC Pinocchio — Initialize Community PDA
 *
 * Creates a new Community PDA with the current agent and guardian keys.
 * The program is deployed at A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx on devnet.
 *
 * Usage: npx ts-node scripts/init-community-pda.ts
 *
 * Requires:
 *   - @solana/web3.js (npm install @solana/web3.js)
 *   - ~/.cynic-keys/agent.json (payer + agent authority)
 *   - ~/.cynic-keys/guardian.json (guardian authority)
 */

import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import * as fs from "fs";
import * as path from "path";

const PROGRAM_ID = new PublicKey("A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx");
const DEVNET_URL = "https://api.devnet.solana.com";

// Load keypair from JSON file
function loadKeypair(filePath: string): Keypair {
  const raw = JSON.parse(fs.readFileSync(filePath, "utf-8"));
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

async function main() {
  const connection = new Connection(DEVNET_URL, "confirmed");

  // Load keys
  const agentKp = loadKeypair(path.join(process.env.HOME!, ".cynic-keys/agent.json"));
  const guardianKp = loadKeypair(path.join(process.env.HOME!, ".cynic-keys/guardian.json"));

  console.log(`Agent:    ${agentKp.publicKey.toBase58()}`);
  console.log(`Guardian: ${guardianKp.publicKey.toBase58()}`);
  console.log(`Program:  ${PROGRAM_ID.toBase58()}`);

  // Use the same mint as the original PDA for consistency.
  // Original tx used 8y5MYNuV9DzjGJNBa2aR39bPgBFkvTGyQkxFn3ihKaMd as mint.
  // For a NEW community with new keys, we use a different mint to get a different PDA.
  // Using guardian pubkey as the "mint" seed — any unique pubkey works.
  const communityMint = guardianKp.publicKey;

  // Derive PDA: seeds = ["community", mint.as_ref()]
  const [communityPda, bump] = PublicKey.findProgramAddressSync(
    [Buffer.from("community"), communityMint.toBuffer()],
    PROGRAM_ID
  );
  console.log(`Community PDA: ${communityPda.toBase58()} (bump: ${bump})`);

  // Check if PDA already exists
  const existing = await connection.getAccountInfo(communityPda);
  if (existing) {
    console.log(`PDA already exists (${existing.data.length} bytes). Aborting.`);
    process.exit(0);
  }

  // Build initialize_community instruction
  // Layout (from binary analysis):
  //   discriminator: 0 (u8) = initialize_community
  //   threshold: u16 (little-endian) — Q-score threshold in basis points (5000 = 0.50)
  //
  // Accounts (in order):
  //   0: payer (signer, writable) — pays rent
  //   1: community PDA (writable) — will be created
  //   2: agent pubkey (read-only) — stored in PDA
  //   3: guardian pubkey (read-only) — stored in PDA
  //   4: mint pubkey (read-only) — used as PDA seed
  //   5: system_program (read-only) — for create_account

  const threshold = 5000; // 0.50 in basis points (phi^-2 ≈ 0.382 → 3820, but 5000 is safer)

  // Instruction layout from original tx: Data: [0, 236, 14, 100, 0, 0, 0]
  // = u8 discriminator(0) + u16 threshold(3820=φ⁻²) + u32 unknown(100)
  // The u32 might be a timelock or version. Replicate exactly.
  const data = Buffer.alloc(7);
  data.writeUInt8(0, 0); // discriminator: initialize_community
  data.writeUInt16LE(threshold, 1); // threshold in basis points
  data.writeUInt32LE(100, 3); // unknown param — replicate from original tx

  // Account layout from original tx (6 accounts):
  //   0: payer (signer, writable)
  //   1: community PDA (writable)
  //   2: mint (read-only) — used as PDA seed
  //   3: agent (read-only) — stored in PDA
  //   4: guardian (read-only) — stored in PDA
  //   5: system_program (read-only) — for create_account CPI
  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: agentKp.publicKey, isSigner: true, isWritable: true },
      { pubkey: communityPda, isSigner: false, isWritable: true },
      { pubkey: communityMint, isSigner: false, isWritable: false },
      { pubkey: agentKp.publicKey, isSigner: false, isWritable: false },
      { pubkey: guardianKp.publicKey, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });

  const tx = new Transaction().add(ix);

  console.log("Sending initialize_community...");
  try {
    const sig = await sendAndConfirmTransaction(connection, tx, [agentKp], {
      commitment: "confirmed",
    });
    console.log(`SUCCESS: ${sig}`);
    console.log(`PDA: ${communityPda.toBase58()}`);

    // Verify
    const account = await connection.getAccountInfo(communityPda);
    if (account) {
      console.log(`Verified: PDA exists, ${account.data.length} bytes, owner=${account.owner.toBase58()}`);
    }
  } catch (e: any) {
    console.error(`FAILED: ${e.message}`);
    if (e.logs) {
      console.error("Program logs:");
      e.logs.forEach((l: string) => console.error(`  ${l}`));
    }
    process.exit(1);
  }
}

main();

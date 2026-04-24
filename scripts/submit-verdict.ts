#!/usr/bin/env npx ts-node
/**
 * CYNIC → Solana submit_verdict
 *
 * Flow: content → CYNIC /judge → verdict → submit_verdict ix → devnet PDA
 *
 * Usage:
 *   npx ts-node scripts/submit-verdict.ts "content to judge"
 *   npx ts-node scripts/submit-verdict.ts read
 *   npx ts-node scripts/submit-verdict.ts read-verdict <proposal-hash-hex>
 *
 * Env (source ~/.cynic-env, or export manually):
 *   CYNIC_REST_ADDR  — kernel address (http://host:port or host:port)
 *   CYNIC_API_KEY    — Bearer token
 *
 * Keys:
 *   ~/.cynic-keys/agent.json    — agent keypair (signs tx, pays rent)
 *   ~/.cynic-keys/guardian.json — guardian keypair (pubkey = community PDA seed)
 *
 * Falsify: solana confirm -v <sig> --url devnet
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
import { createHash } from "crypto";
import * as fs from "fs";
import * as path from "path";

// ── Constants ─────────────────────────────────────────────────────────────────

const PROGRAM_ID = new PublicKey("A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx");
const DEVNET_URL = "https://api.devnet.solana.com";
const COMMUNITY_SEED = Buffer.from("community");
const VERDICT_SEED = Buffer.from("verdict");
const KNOWN_COMMUNITY_PDA = "8DVUKmJabj5gzQXE6u6DpnQxsDMGy8Be5aHzjqxttHow";

// Verdict type mapping (matches Pinocchio program enum)
const VERDICT_TYPE_MAP: Record<string, number> = { HOWL: 0, WAG: 1, GROWL: 2, BARK: 3 };
const VERDICT_TYPES = ["HOWL", "WAG", "GROWL", "BARK"];

// ── Env loading ───────────────────────────────────────────────────────────────

function loadCynicEnv(): void {
  const envPath = path.join(process.env.HOME!, ".cynic-env");
  if (!fs.existsSync(envPath)) return;
  const lines = fs.readFileSync(envPath, "utf-8").split("\n");
  for (const line of lines) {
    const m = line.match(/^(?:export\s+)?([A-Z_][A-Z0-9_]*)=["']?([^"'\n]*)["']?/);
    if (m && !process.env[m[1]]) process.env[m[1]] = m[2];
  }
}

function cynicUrl(): string {
  const addr = process.env.CYNIC_REST_ADDR ?? "";
  if (!addr) throw new Error("CYNIC_REST_ADDR not set. Source ~/.cynic-env first.");
  return addr.startsWith("http") ? addr : `http://${addr}`;
}

// ── Key loading ───────────────────────────────────────────────────────────────

function loadKeypair(filePath: string): Keypair {
  const expanded = filePath.replace(/^~/, process.env.HOME!);
  const raw = JSON.parse(fs.readFileSync(expanded, "utf-8"));
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

// ── PDA derivation ────────────────────────────────────────────────────────────

function findCommunityPDA(guardianPubkey: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [COMMUNITY_SEED, guardianPubkey.toBuffer()],
    PROGRAM_ID
  );
}

function findVerdictPDA(community: PublicKey, proposalHash: Buffer): [PublicKey, number] {
  if (proposalHash.length !== 32) throw new Error("proposal_hash must be 32 bytes");
  return PublicKey.findProgramAddressSync(
    [VERDICT_SEED, community.toBuffer(), proposalHash],
    PROGRAM_ID
  );
}

function proposalHash(content: string): Buffer {
  return createHash("sha256").update(content).digest() as unknown as Buffer;
}

// ── Score conversion ──────────────────────────────────────────────────────────

// CYNIC scores: 0.0–1.0 (φ-bounded max ≈ 0.618 = 6180 bp)
// On-chain: basis points (0–10000)
function toBasisPoints(score: number): number {
  return Math.round(Math.min(score, 1.0) * 10000);
}

// ── Instruction builder ───────────────────────────────────────────────────────

interface AxiomScores {
  fidelity: number;
  phi: number;
  verify: number;
  culture: number;
  burn: number;
  sovereignty: number;
}

function buildSubmitVerdictIx(params: {
  agent: PublicKey;
  community: PublicKey;
  proposalHash: Buffer;
  qScore: number;       // basis points
  axiomScores: AxiomScores; // basis points
  dogCount: number;
  verdictType: number;
}): TransactionInstruction {
  const [verdictPDA] = findVerdictPDA(params.community, params.proposalHash);

  // Data layout: 1 (disc) + 32 (hash) + 2 (q) + 6×2 (axioms) + 1 (dogs) + 1 (type) = 49 bytes
  const data = Buffer.alloc(49);
  let offset = 0;

  data.writeUInt8(1, offset); offset += 1;                              // discriminator = submit_verdict
  params.proposalHash.copy(data, offset); offset += 32;                 // proposal_hash
  data.writeUInt16LE(params.qScore, offset); offset += 2;               // q_score
  data.writeUInt16LE(params.axiomScores.fidelity, offset); offset += 2; // fidelity
  data.writeUInt16LE(params.axiomScores.phi, offset); offset += 2;      // phi
  data.writeUInt16LE(params.axiomScores.verify, offset); offset += 2;   // verify
  data.writeUInt16LE(params.axiomScores.culture, offset); offset += 2;  // culture
  data.writeUInt16LE(params.axiomScores.burn, offset); offset += 2;     // burn
  data.writeUInt16LE(params.axiomScores.sovereignty, offset); offset += 2; // sovereignty
  data.writeUInt8(params.dogCount, offset); offset += 1;                // dog_count
  data.writeUInt8(params.verdictType, offset);                          // verdict_type

  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: params.agent, isSigner: true, isWritable: true },
      { pubkey: params.community, isSigner: false, isWritable: true },
      { pubkey: verdictPDA, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

// ── Account deserializers ─────────────────────────────────────────────────────

function deserializeCommunity(data: Buffer) {
  if (data.length < 106 || data[0] !== 0) return null;
  return {
    disc: data.readUInt8(0),
    version: data.readUInt8(1),
    bump: data.readUInt8(2),
    paused: data.readUInt8(3) !== 0,
    mint: new PublicKey(data.subarray(4, 36)).toBase58(),
    agent: new PublicKey(data.subarray(36, 68)).toBase58(),
    guardian: new PublicKey(data.subarray(68, 100)).toBase58(),
    threshold: data.readUInt16LE(100),
    timelockSlots: data.readUInt32LE(102),
    totalVerdicts: data.readUInt32LE(106),
  };
}

function deserializeVerdict(data: Buffer) {
  if (data.length < 99 || data[0] !== 1) return null;
  return {
    disc: data.readUInt8(0),
    verdictType: VERDICT_TYPES[data.readUInt8(3)] ?? "UNKNOWN",
    community: new PublicKey(data.subarray(4, 36)).toBase58(),
    proposalHash: data.subarray(36, 68).toString("hex"),
    qScore: data.readUInt16LE(68),
    fidelity: data.readUInt16LE(70),
    phi: data.readUInt16LE(72),
    verify: data.readUInt16LE(74),
    culture: data.readUInt16LE(76),
    burn: data.readUInt16LE(78),
    sovereignty: data.readUInt16LE(80),
    dogCount: data.readUInt8(82),
    // 7 bytes repr(C) padding at [83..89] to align i64
    timestamp: Number(data.readBigInt64LE(90)),
    executed: data.readUInt8(98) !== 0,
  };
}

// ── CYNIC /judge ──────────────────────────────────────────────────────────────

interface JudgeResponse {
  verdict: string;
  q_score: {
    total: number;
    fidelity: number;
    phi: number;
    verify: number;
    culture: number;
    burn: number;
    sovereignty: number;
  };
  voter_count: number;
}

async function judgeContent(content: string): Promise<JudgeResponse> {
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  const apiKey = process.env.CYNIC_API_KEY ?? "";
  if (apiKey) headers["Authorization"] = `Bearer ${apiKey}`;

  const res = await fetch(`${cynicUrl()}/judge`, {
    method: "POST",
    headers,
    body: JSON.stringify({ content, domain: "token-analysis" }),
  });

  if (!res.ok) {
    throw new Error(`CYNIC /judge failed: HTTP ${res.status}\n${await res.text()}`);
  }
  return res.json() as Promise<JudgeResponse>;
}

// ── Commands ──────────────────────────────────────────────────────────────────

async function cmdJudge(content: string): Promise<void> {
  const connection = new Connection(DEVNET_URL, "confirmed");
  const agentKp = loadKeypair("~/.cynic-keys/agent.json");
  const guardianKp = loadKeypair("~/.cynic-keys/guardian.json");

  // Verify community PDA matches the known address
  const [communityPDA] = findCommunityPDA(guardianKp.publicKey);
  if (communityPDA.toBase58() !== KNOWN_COMMUNITY_PDA) {
    throw new Error(
      `Community PDA mismatch!\n` +
      `  Derived:  ${communityPDA.toBase58()}\n` +
      `  Expected: ${KNOWN_COMMUNITY_PDA}\n` +
      `  Check that ~/.cynic-keys/guardian.json is the correct keypair.`
    );
  }

  console.log(`\n─── CYNIC submit_verdict ─────────────────────────────`);
  console.log(`Content:  "${content.slice(0, 80)}${content.length > 80 ? "…" : ""}"`);
  console.log(`Agent:    ${agentKp.publicKey.toBase58()}`);
  console.log(`Community: ${communityPDA.toBase58()}`);

  // Step 1: Judge
  console.log(`\n[1/4] Calling CYNIC /judge...`);
  const result = await judgeContent(content);

  const verdictLabel = (result.verdict ?? "BARK").toUpperCase();
  const qs = result.q_score ?? { total: 0, fidelity: 0, phi: 0, verify: 0, culture: 0, burn: 0, sovereignty: 0 };
  const dogCount = result.voter_count ?? 0;
  const verdictType = VERDICT_TYPE_MAP[verdictLabel] ?? 3;
  const qScoreBp = toBasisPoints(qs.total ?? 0);
  const axiomScores: AxiomScores = {
    fidelity: toBasisPoints(qs.fidelity ?? 0),
    phi: toBasisPoints(qs.phi ?? 0),
    verify: toBasisPoints(qs.verify ?? 0),
    culture: toBasisPoints(qs.culture ?? 0),
    burn: toBasisPoints(qs.burn ?? 0),
    sovereignty: toBasisPoints(qs.sovereignty ?? 0),
  };

  console.log(`  Verdict:  ${verdictLabel} (type=${verdictType})`);
  console.log(`  Q-Score:  ${(qs.total ?? 0).toFixed(4)} (${qScoreBp} bp)`);
  console.log(`  Dogs:     ${dogCount}`);
  console.log(`  Axioms:`);
  console.log(`    fidelity=${axiomScores.fidelity} phi=${axiomScores.phi} verify=${axiomScores.verify}`);
  console.log(`    culture=${axiomScores.culture} burn=${axiomScores.burn} sovereignty=${axiomScores.sovereignty}`);

  // Step 2: Derive verdict PDA
  const hash = proposalHash(content);
  const hashHex = hash.toString("hex");
  const [verdictPDA] = findVerdictPDA(communityPDA, hash);

  console.log(`\n[2/4] Building transaction...`);
  console.log(`  Proposal hash: ${hashHex}`);
  console.log(`  Verdict PDA:   ${verdictPDA.toBase58()}`);

  const ix = buildSubmitVerdictIx({
    agent: agentKp.publicKey,
    community: communityPDA,
    proposalHash: hash,
    qScore: qScoreBp,
    axiomScores,
    dogCount,
    verdictType,
  });

  // Step 3: Sign and submit
  console.log(`\n[3/4] Signing and submitting to devnet...`);
  const tx = new Transaction().add(ix);
  const sig = await sendAndConfirmTransaction(connection, tx, [agentKp], {
    commitment: "confirmed",
  });
  console.log(`  Signature: ${sig}`);

  // Step 4: Read back and verify
  console.log(`\n[4/4] Verifying on-chain...`);
  const account = await connection.getAccountInfo(verdictPDA);
  if (!account) {
    throw new Error(`Verdict PDA not found after confirmed tx — unexpected state.`);
  }
  const onChain = deserializeVerdict(account.data as Buffer);
  if (!onChain) {
    throw new Error(`Could not deserialize verdict account (${account.data.length} bytes, disc=${account.data[0]})`);
  }

  console.log(`\n─── On-Chain Verdict ─────────────────────────────────`);
  console.log(`  PDA:         ${verdictPDA.toBase58()}`);
  console.log(`  Type:        ${onChain.verdictType}`);
  console.log(`  Q-Score:     ${onChain.qScore} bp (${(onChain.qScore / 100).toFixed(2)}%)`);
  console.log(`  Fidelity:    ${onChain.fidelity} bp`);
  console.log(`  Phi:         ${onChain.phi} bp`);
  console.log(`  Verify:      ${onChain.verify} bp`);
  console.log(`  Culture:     ${onChain.culture} bp`);
  console.log(`  Burn:        ${onChain.burn} bp`);
  console.log(`  Sovereignty: ${onChain.sovereignty} bp`);
  console.log(`  Dogs:        ${onChain.dogCount}`);
  console.log(`  Timestamp:   ${new Date(onChain.timestamp * 1000).toISOString()}`);
  console.log(`  Executed:    ${onChain.executed}`);
  console.log(`\n─── Falsify ──────────────────────────────────────────`);
  console.log(`  solana confirm -v ${sig} --url devnet`);
  console.log(`  solana account ${verdictPDA.toBase58()} --url devnet`);
  console.log(`\nProposal hash (for read-verdict): ${hashHex}`);
}

async function cmdRead(): Promise<void> {
  const connection = new Connection(DEVNET_URL, "confirmed");
  const guardianKp = loadKeypair("~/.cynic-keys/guardian.json");
  const [communityPDA] = findCommunityPDA(guardianKp.publicKey);

  const account = await connection.getAccountInfo(communityPDA);
  if (!account) {
    console.error(`Community PDA not found: ${communityPDA.toBase58()}`);
    console.error(`Run: npx ts-node scripts/init-community-pda.ts`);
    process.exit(1);
  }

  const c = deserializeCommunity(account.data as Buffer);
  console.log(`\n─── Community State ──────────────────────────────────`);
  console.log(`  PDA:            ${communityPDA.toBase58()}`);
  if (c) {
    console.log(`  Mint (seed):    ${c.mint}`);
    console.log(`  Agent:          ${c.agent}`);
    console.log(`  Guardian:       ${c.guardian}`);
    console.log(`  Paused:         ${c.paused}`);
    console.log(`  Threshold:      ${c.threshold} bp`);
    console.log(`  Timelock:       ${c.timelockSlots} slots`);
    console.log(`  Total Verdicts: ${c.totalVerdicts}`);
  } else {
    console.log(`  Raw (${account.data.length} bytes): ${Buffer.from(account.data).toString("hex").slice(0, 64)}…`);
  }
}

async function cmdReadVerdict(hashHex: string): Promise<void> {
  if (!hashHex || hashHex.length !== 64) {
    console.error("Usage: submit-verdict.ts read-verdict <64-char-hex>");
    process.exit(1);
  }

  const connection = new Connection(DEVNET_URL, "confirmed");
  const guardianKp = loadKeypair("~/.cynic-keys/guardian.json");
  const [communityPDA] = findCommunityPDA(guardianKp.publicKey);

  const hashBuf = Buffer.from(hashHex, "hex");
  const [verdictPDA] = findVerdictPDA(communityPDA, hashBuf);

  const account = await connection.getAccountInfo(verdictPDA);
  if (!account) {
    console.error(`Verdict PDA not found: ${verdictPDA.toBase58()}`);
    process.exit(1);
  }

  const v = deserializeVerdict(account.data as Buffer);
  console.log(`\n─── Verdict ──────────────────────────────────────────`);
  console.log(`  PDA:         ${verdictPDA.toBase58()}`);
  if (v) {
    console.log(`  Type:        ${v.verdictType}`);
    console.log(`  Q-Score:     ${v.qScore} bp`);
    console.log(`  Fidelity:    ${v.fidelity} bp`);
    console.log(`  Phi:         ${v.phi} bp`);
    console.log(`  Verify:      ${v.verify} bp`);
    console.log(`  Culture:     ${v.culture} bp`);
    console.log(`  Burn:        ${v.burn} bp`);
    console.log(`  Sovereignty: ${v.sovereignty} bp`);
    console.log(`  Dogs:        ${v.dogCount}`);
    console.log(`  Timestamp:   ${new Date(v.timestamp * 1000).toISOString()}`);
    console.log(`  Executed:    ${v.executed}`);
  } else {
    console.log(`  Raw (${account.data.length} bytes): ${Buffer.from(account.data).toString("hex")}`);
  }
}

// ── Main ──────────────────────────────────────────────────────────────────────

loadCynicEnv();

const [, , cmd, ...rest] = process.argv;

switch (cmd) {
  case "read":
    cmdRead().catch((e) => { console.error(e.message); process.exit(1); });
    break;
  case "read-verdict":
    cmdReadVerdict(rest[0]).catch((e) => { console.error(e.message); process.exit(1); });
    break;
  case undefined:
  case "--help":
  case "-h":
    console.log(`CYNIC submit_verdict

Usage:
  npx ts-node scripts/submit-verdict.ts "content"            Judge + submit on-chain
  npx ts-node scripts/submit-verdict.ts read                 Read community state
  npx ts-node scripts/submit-verdict.ts read-verdict <hex>   Read verdict by proposal hash

Env: source ~/.cynic-env (CYNIC_REST_ADDR, CYNIC_API_KEY)
Keys: ~/.cynic-keys/agent.json, ~/.cynic-keys/guardian.json`);
    break;
  default:
    // Treat all args as content to judge
    cmdJudge([cmd, ...rest].join(" ")).catch((e) => { console.error(e.message); process.exit(1); });
}

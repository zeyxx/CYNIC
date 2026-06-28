import { Connection, PublicKey } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import path from 'path';

// Local research probe only. This is not organ-gasdf runtime code.
dotenv.config({ path: path.resolve(__dirname, '../../../.env') });

const HELIUS_CREDENTIAL_ENV = 'HELIUS_API_KEY';
const rpcCredential = process.env[HELIUS_CREDENTIAL_ENV];
if (!rpcCredential) {
    console.error("No HELIUS_API_KEY found in .env");
    process.exit(1);
}

const connection = new Connection(`https://mainnet.helius-rpc.com/?api-key=${rpcCredential}`);
const PUMP_FUN_PROGRAM = new PublicKey('6EF8rrecthR5Dkzon8Nwu78hRvfX9132uT2WRjS5A5S1');

async function analyzeBottomUp() {
    console.log("Fetching recent signatures for Pump.fun program...");
    const signatures = await connection.getSignaturesForAddress(PUMP_FUN_PROGRAM, { limit: 50 });
    
    console.log(`Fetched ${signatures.length} signatures. Analyzing instructions...`);
    
    const sigList = signatures.map(s => s.signature);
    
    // Fetch parsed transactions
    const txs = await connection.getParsedTransactions(sigList, {
        maxSupportedTransactionVersion: 0,
        commitment: 'confirmed'
    });

    const instructionDiscriminators = new Map<string, number>();

    txs.forEach((tx, index) => {
        if (!tx || tx.meta?.err) return; // Skip failed txs
        
        tx.transaction.message.instructions.forEach(ix => {
            if ('programId' in ix && ix.programId.equals(PUMP_FUN_PROGRAM)) {
                if ('data' in ix) {
                    // Extract the first 8 bytes (discriminator)
                    const dataBuffer = Buffer.from(ix.data, 'base64');
                    if (dataBuffer.length >= 8) {
                        const discriminator = dataBuffer.subarray(0, 8).toString('hex');
                        instructionDiscriminators.set(discriminator, (instructionDiscriminators.get(discriminator) || 0) + 1);
                        
                        // If it's a rare instruction (not buy/sell/create), let's log the accounts
                        // Create: 181ec828051c0777
                        // Buy: 66063d1201daebea
                        // Sell: 33e685a4017f83ad
                        // We will log anything else to find the CTO / Fee routing instruction
                        if (!['181ec828051c0777', '66063d1201daebea', '33e685a4017f83ad'].includes(discriminator)) {
                            console.log(`\nFound potential admin/CTO instruction: ${discriminator}`);
                            console.log(`Tx: https://solscan.io/tx/${sigList[index]}`);
                            console.log(`Accounts involved:`);
                            ix.accounts.forEach((acc, i) => {
                                console.log(`  [${i}]: ${acc.toBase58()}`);
                            });
                        }
                    }
                }
            }
        });
    });

    console.log("\n--- Instruction Breakdown ---");
    instructionDiscriminators.forEach((count, disc) => {
        let label = "Unknown";
        if (disc === '181ec828051c0777') label = "Create";
        if (disc === '66063d1201daebea') label = "Buy";
        if (disc === '33e685a4017f83ad') label = "Sell";
        console.log(`${disc} (${label}): ${count} calls`);
    });
}

analyzeBottomUp().catch(console.error);

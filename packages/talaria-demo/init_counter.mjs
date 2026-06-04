import { Connection, PublicKey, Transaction, TransactionInstruction, Keypair, sendAndConfirmTransaction, SystemProgram } from '@solana/web3.js'
import { readFileSync } from 'fs'

const PROGRAM_ID = new PublicKey('AKjCbxzdjXHcTmTqN37K7eZM2RUsCYTmaXUriTd6csBH')
const conn = new Connection('https://api.devnet.solana.com', 'confirmed')

const payer = Keypair.fromSecretKey(
  Uint8Array.from(JSON.parse(readFileSync('/home/user/.config/solana/id.json', 'utf-8')))
)

const [counterPda] = PublicKey.findProgramAddressSync([Buffer.from('cynic-counter')], PROGRAM_ID)
console.log('Counter PDA:', counterPda.toBase58())

const ix = new TransactionInstruction({
  keys: [
    { pubkey: counterPda, isSigner: false, isWritable: true },
    { pubkey: payer.publicKey, isSigner: true, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ],
  programId: PROGRAM_ID,
  data: Buffer.from([0]), // InitCounter
})

const tx = new Transaction().add(ix)
const sig = await sendAndConfirmTransaction(conn, tx, [payer])
console.log('Counter initialized:', sig)
console.log('Explorer:', `https://explorer.solana.com/tx/${sig}?cluster=devnet`)

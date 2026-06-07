import {
  PublicKey,
  TransactionInstruction,
  SystemProgram,
} from '@solana/web3.js'

export const PROGRAM_ID = new PublicKey('AKjCbxzdjXHcTmTqN37K7eZM2RUsCYTmaXUriTd6csBH')
export const DEVNET_RPC = 'https://api.devnet.solana.com'

const textEncoder = new TextEncoder()

export function findCounterPda() {
  return PublicKey.findProgramAddressSync([textEncoder.encode('cynic-counter')], PROGRAM_ID)
}

export function findVerdictPda(verdictIdBytes) {
  return PublicKey.findProgramAddressSync(
    [textEncoder.encode('verdict'), verdictIdBytes],
    PROGRAM_ID,
  )
}

// verdict_id: string (UUID with dashes) → Uint8Array(16)
export function uuidToBytes(uuid) {
  const hex = uuid.replace(/-/g, '')
  const bytes = new Uint8Array(16)
  for (let i = 0; i < 16; i++) bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16)
  return bytes
}

const KIND = { Bark: 0, Growl: 1, Wag: 2, Howl: 3, Epoche: 4 }

// Build RecordVerdict instruction
export function recordVerdictIx(recorder, verdictId, verdictKind, qScoreTotal) {
  const idBytes = uuidToBytes(verdictId)
  const [verdictPda] = findVerdictPda(idBytes)
  const [counterPda] = findCounterPda()
  const kind = KIND[verdictKind] ?? 1
  const qMillis = Math.round(qScoreTotal * 1000)

  const data = new Uint8Array(22)
  data[0] = 1 // discriminator
  idBytes.forEach((b, i) => { data[i + 1] = b })
  data[17] = kind
  new DataView(data.buffer).setUint32(18, qMillis, true)

  return {
    ix: new TransactionInstruction({
      keys: [
        { pubkey: verdictPda, isSigner: false, isWritable: true },
        { pubkey: counterPda, isSigner: false, isWritable: true },
        { pubkey: recorder, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data,
    }),
    verdictPda,
  }
}

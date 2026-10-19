/**
 * Self-facilitator: the relay verifies and settles its own x402 payments.
 * Canonical pattern from solana-foundation/x402
 * (examples/typescript/facilitator/basic), SVM-only.
 *
 * Env: FACILITATOR_KEY_PATH (JSON secret-key file, 600), SVM_RPC_URL,
 * PORT (default 4021). The key MUST be dedicated: never the Kora signer.
 * Kora keeps sponsoring underneath in all cases.
 */
import express from 'express';
import { readFileSync } from 'node:fs';
import { createKeyPairSignerFromBytes } from '@solana/kit';

/**
 * UPSTREAM WORKAROUND (@x402/svm 2.28.0 + @solana/kit 8.5.0):
 * the SDK's exact-scheme verifier imports the payer pubkey with
 * extractable=false, but Kit 8.5.0's verifySignature throws unless the key
 * is extractable. The SDK swallows the throw as `signature_invalid`, so
 * every payment fails verification. Coerce Ed25519 verify-only imports to
 * extractable. Cryptography is unaffected (extractability only gates key
 * export, not verification). Remove when @x402/svm imports with true.
 */
const rawImport = globalThis.crypto.subtle.importKey.bind(globalThis.crypto.subtle);
async function patchedImportKey(
  format: string,
  keyData: Uint8Array,
  algorithm: string,
  extractable: boolean,
  keyUsages: string[],
): Promise<CryptoKey> {
  if (algorithm === 'Ed25519' || (algorithm as unknown as { name?: string })?.name === 'Ed25519') {
    extractable = true;
  }
  const out = await (rawImport as unknown as (...args: unknown[]) => Promise<CryptoKey>)(
    format,
    keyData,
    algorithm,
    extractable,
    keyUsages,
  );
  return out;
}
globalThis.crypto.subtle.importKey = patchedImportKey as typeof globalThis.crypto.subtle.importKey;
import { x402Facilitator } from '@x402/core/facilitator';
import { toFacilitatorSvmSigner } from '@x402/svm';
import { ExactSvmScheme } from '@x402/svm/exact/facilitator';
import { UptoSvmScheme } from '@x402/svm/upto/facilitator';

const DEVNET = 'solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1';
const MAINNET = 'solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp';

function required(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`missing env ${name}`);
  return value;
}

const secret = new Uint8Array(JSON.parse(readFileSync(required('FACILITATOR_KEY_PATH'), 'utf8')));
const signer = await createKeyPairSignerFromBytes(secret);
console.info(`Facilitator account: ${signer.address}`);

const rpcUrl = process.env['SVM_RPC_URL'];
const svmSigner = toFacilitatorSvmSigner(signer, rpcUrl ? { defaultRpcUrl: rpcUrl } : undefined);

const facilitator = new x402Facilitator()
  .onVerifyFailure(async (context) => console.log('Verify failure', context))
  .onSettleFailure(async (context) => console.log('Settle failure', context));

facilitator.register(DEVNET, new ExactSvmScheme(svmSigner));
facilitator.register(MAINNET, new ExactSvmScheme(svmSigner));
// Upto (channels): client locks a USDC deposit once, then pays per relay
// with off-chain vouchers the facilitator settles. No deposit is opened by
// registering here; opening/closing channels moves real funds (explicit op).
facilitator.register(DEVNET, new UptoSvmScheme(svmSigner));
facilitator.register(MAINNET, new UptoSvmScheme(svmSigner));

const app = express();
app.use(express.json());

/**
 * The @x402/express resource server forwards the legacy payload shape
 * ({scheme, network, payload}) while the @x402/svm verifier reads the
 * current shape (payload.accepted.{scheme, network}). Normalize here so
 * both halves of the same 2.28.0 release interoperate. Remove if either
 * side converges.
 */
function normalizePayload(paymentPayload: any, paymentRequirements: any): any {
  if (paymentPayload?.accepted) return paymentPayload;
  return {
    ...paymentPayload,
    accepted: {
      scheme: paymentPayload?.scheme ?? paymentRequirements?.scheme,
      network: paymentPayload?.network ?? paymentRequirements?.network,
    },
  };
}

app.post('/verify', async (req, res) => {
  try {
    const { paymentPayload, paymentRequirements } = req.body;
    if (!paymentPayload || !paymentRequirements) {
      return res.status(400).json({ error: 'Missing paymentPayload or paymentRequirements' });
    }
    res.json(
      await facilitator.verify(
        normalizePayload(paymentPayload, paymentRequirements),
        paymentRequirements,
      ),
    );
  } catch (error) {
    res.status(500).json({ error: error instanceof Error ? error.message : 'Unknown error' });
  }
});

app.post('/settle', async (req, res) => {
  try {
    const { paymentPayload, paymentRequirements } = req.body;
    if (!paymentPayload || !paymentRequirements) {
      return res.status(400).json({ error: 'Missing paymentPayload or paymentRequirements' });
    }
    res.json(
      await facilitator.settle(
        normalizePayload(paymentPayload, paymentRequirements),
        paymentRequirements,
      ),
    );
  } catch (error) {
    res.status(500).json({ error: error instanceof Error ? error.message : 'Unknown error' });
  }
});

app.get('/supported', async (_req, res) => {
  try {
    res.json(facilitator.getSupported());
  } catch (error) {
    res.status(500).json({ error: error instanceof Error ? error.message : 'Unknown error' });
  }
});

const port = Number(process.env['PORT'] ?? 4021);
app.listen(port, () => console.info(`Facilitator listening on :${port}`));

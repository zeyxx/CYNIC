/**
 * Relay gateway composition root. No secrets here: everything sensitive comes
 * from the environment (Kora HMAC, Jupiter key, signer file loaded by the
 * launcher, never baked into code).
 *
 * Required env: KORA_URL, KORA_HMAC_SECRET, KORA_SIGNER, RPC_URL.
 * Optional env: JUPITER_API_KEY, X402_PAY_TO, X402_NETWORK, PORT,
 * OUTPUT_MINT (default: WSOL; USDC for the dollar leg).
 */
import { address } from '@solana/kit';
import { estimateFee, signAndSend, type KoraConfig } from './kora/client.js';
import { ASDF_MINT, WSOL_MINT, buildSponsoredSell } from './builder/setup-aware.js';

function required(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`missing env ${name}`);
  return value;
}

const kora: KoraConfig = { url: required('KORA_URL'), hmacSecret: required('KORA_HMAC_SECRET') };
const KORA_SIGNER = address(required('KORA_SIGNER'));
const RPC_URL = required('RPC_URL');

export async function sponsoredSellOnce(user: {
  wallet: ReturnType<typeof address>;
  signer: import('@solana/kit').TransactionSigner;
}): Promise<{ signature: string; quoteOutAmount: string; feeInToken: bigint }> {
  const { base64Tx, quoteOutAmount, feeInToken } = await buildSponsoredSell({
    rpcUrl: RPC_URL,
    kora,
    koraSigner: KORA_SIGNER,
    user,
    inputMint: ASDF_MINT,
    outputMint: process.env['OUTPUT_MINT'] ? address(process.env['OUTPUT_MINT']) : WSOL_MINT,
    amount: 1_000_000n,
    feeToken: ASDF_MINT,
    jupiterApiKey: process.env['JUPITER_API_KEY'],
    feeBufferBps: 500n,
  });
  // Pre-flight through estimate so a bad tx never reaches the signer.
  await estimateFee(kora, base64Tx, ASDF_MINT.toString());
  const signature = await signAndSend(kora, base64Tx);
  return { signature, quoteOutAmount, feeInToken };
}

/**
 * HTTP composition: x402-gated relay route wired to the (local by default)
 * self-facilitator. The dev user key signs the sponsored leg; production
 * callers will submit their own partial signatures instead.
 *
 * Env: PORT (default 3000), X402_PRICE (default "$0.005"),
 * X402_PAY_TO (default: facilitator account), X402_NETWORK,
 * X402_FACILITATOR_URL (default "http://localhost:4021"),
 * USER_KEY_PATH (dev user secret-key file).
 */
export async function startServer(): Promise<void> {
  const { default: express } = await import('express');
  const {
    relayPaymentMiddleware,
    uptoRelayPaymentMiddleware,
    setSettlementOverrides,
  } = await import('./facilitator/x402.js');
  const { createKeyPairSignerFromBytes } = await import('@solana/kit');
  const { readFileSync } = await import('node:fs');

  const userSecret = new Uint8Array(
    JSON.parse(readFileSync(required('USER_KEY_PATH'), 'utf8')),
  );
  const userSigner = await createKeyPairSignerFromBytes(userSecret);

  const network = (process.env['X402_NETWORK'] ??
    'solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp') as `${string}:${string}`;
  const facilitatorUrl =
    process.env['X402_FACILITATOR_URL'] ?? 'http://localhost:4021';
  const payTo =
    process.env['X402_PAY_TO'] ?? '4KaDNU2ey2LMpUq4RXeW2WWeXkL44rvKSrei7ZLnyqRt';

  const app = express();
  app.use(express.json());
  app.use(
    relayPaymentMiddleware({
      price: (process.env['X402_PRICE'] ?? '$0.005') as `$${string}`,
      payTo,
      network,
      facilitatorUrl,
    }),
  );
  app.post('/relay/send', async (_req, res) => {
    try {
      const out = await sponsoredSellOnce({ wallet: userSigner.address, signer: userSigner });
      res.json({ signature: out.signature, quoteOutAmount: out.quoteOutAmount });
    } catch (error) {
      res.status(500).json({ error: error instanceof Error ? error.message : 'Unknown error' });
    }
  });

  // Upto route: channel ceiling $0.10, each relay settles its actual $0.005.
  // Disabled unless UPTO_AUTHORIZER_KEY_PATH is set (server hot key).
  const authorizerPath = process.env['UPTO_AUTHORIZER_KEY_PATH'];
  if (authorizerPath) {
    const authorizerSecret = new Uint8Array(JSON.parse(readFileSync(authorizerPath, 'utf8')));
    const receiverAuthorizerSigner = await createKeyPairSignerFromBytes(authorizerSecret);
    app.use(
      uptoRelayPaymentMiddleware({
        price: '$0.005',
        ceiling: '$0.10',
        payTo,
        network,
        facilitatorUrl,
        receiverAuthorizerSigner,
      }),
    );
    app.post('/relay/send-upto', async (_req, res) => {
      try {
        const out = await sponsoredSellOnce({ wallet: userSigner.address, signer: userSigner });
        setSettlementOverrides(res, { amount: '5000' });
        res.json({ signature: out.signature, quoteOutAmount: out.quoteOutAmount });
      } catch (error) {
        res.status(500).json({ error: error instanceof Error ? error.message : 'Unknown error' });
      }
    });
  }
  const port = Number(process.env['PORT'] ?? 3000);
  app.listen(port, () => console.info(`Relay gateway listening on :${port}`));
}

if (process.env['RUN_SERVER'] === '1') {
  await startServer();
}

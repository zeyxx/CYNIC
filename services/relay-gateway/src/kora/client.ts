/**
 * Typed client for the Kora relay RPC (HMAC-authenticated).
 * No secrets live here: HMAC secret is injected via config.
 */
import { createHmac } from 'node:crypto';

export interface KoraConfig {
  url: string;
  hmacSecret: string;
}

export interface FeeEstimate {
  feeInToken: bigint;
  paymentAddress: string;
}

function signedFetch(config: KoraConfig, method: string, params: unknown): Promise<unknown> {
  const body = JSON.stringify({ jsonrpc: '2.0', id: 1, method, params });
  const timestamp = Math.floor(Date.now() / 1000).toString();
  const signature = createHmac('sha256', config.hmacSecret).update(timestamp + body).digest('hex');
  return fetch(config.url, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-timestamp': timestamp,
      'x-hmac-signature': signature,
    },
    body,
  }).then(async (res) => {
    if (res.status === 401) throw new Error(`${method}: 401 unauthorized`);
    const parsed = (await res.json()) as { result?: unknown; error?: { message: string } };
    if (parsed.error) throw new Error(`${method}: ${parsed.error.message.slice(0, 200)}`);
    return parsed.result;
  });
}

export async function estimateFee(
  config: KoraConfig,
  base64Tx: string,
  feeToken: string,
): Promise<FeeEstimate> {
  const result = (await signedFetch(config, 'estimateTransactionFee', {
    transaction: base64Tx,
    fee_token: feeToken,
    sig_verify: false,
  })) as { fee_in_token: string; payment_address: string };
  return { feeInToken: BigInt(result.fee_in_token), paymentAddress: result.payment_address };
}

export async function signAndSend(config: KoraConfig, base64Tx: string): Promise<string> {
  const result = await signedFetch(config, 'signAndSendTransaction', {
    transaction: base64Tx,
    sig_verify: false,
  });
  return typeof result === 'string' ? result : (result as { signature: string }).signature;
}

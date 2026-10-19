/**
 * x402 exact-scheme wiring for the relay, following the official
 * solana-foundation/x402 express-server example:
 *
 *   paymentMiddleware(routes, new x402ResourceServer(facilitatorClient)
 *     .register(network, new ExactSvmScheme()))
 *
 * Route price uses Money strings (e.g. "$0.001"). Networks are CAIP-2
 * ('solana:<genesis>' on mainnet, 'solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1'
 * on devnet).
 *
 * Self-facilitator (relay settles its own payments) lives in
 * ./server.ts with a dedicated facilitator signer (dev: key file,
 * prod: solana-keychain KMS). Until it is deployed, point the relay at
 * an external facilitator URL.
 */
import { paymentMiddleware, setSettlementOverrides, x402ResourceServer } from '@x402/express';
import { ExactSvmScheme } from '@x402/svm/exact/server';
import { UptoSvmScheme } from '@x402/svm/upto/server';
import { HTTPFacilitatorClient } from '@x402/core/server';

export { setSettlementOverrides };

export interface RelayPricing {
  /** Human price per relayed transaction, e.g. "$0.001". */
  price: `$${string}`;
  /** Treasury address receiving x402 payments. */
  payTo: string;
  /** CAIP-2 network id. */
  network: `${string}:${string}`;
  /** URL of the facilitator used until the relay self-facilitates. */
  facilitatorUrl: string;
}

export function relayPaymentMiddleware(pricing: RelayPricing) {
  const facilitatorClient = new HTTPFacilitatorClient({ url: pricing.facilitatorUrl });
  const server = new x402ResourceServer(facilitatorClient).register(
    pricing.network,
    new ExactSvmScheme(),
  );
  return paymentMiddleware(
    {
      'POST /relay/send': {
        accepts: [
          {
            scheme: 'exact',
            price: pricing.price,
            network: pricing.network,
            payTo: pricing.payTo,
          },
        ],
        description: 'Sponsor and submit a user-signed Solana transaction',
      },
    },
    server,
  );
}

export interface UptoRelayPricing extends RelayPricing {
  /** Ceiling the client locks in the channel, e.g. "$0.10". */
  ceiling: `$${string}`;
  /** Server hot key signing settlement vouchers (authorizer file, never Kora). */
  receiverAuthorizerSigner: import('@solana/kit').TransactionSigner;
}

/**
 * Upto variant: one channel deposit covers many relays; each relay settles
 * only its actual cost via setSettlementOverrides in the route handler.
 */
export function uptoRelayPaymentMiddleware(pricing: UptoRelayPricing) {
  const facilitatorClient = new HTTPFacilitatorClient({ url: pricing.facilitatorUrl });
  const server = new x402ResourceServer(facilitatorClient).register(
    pricing.network,
    new UptoSvmScheme({ receiverAuthorizerSigner: pricing.receiverAuthorizerSigner }),
  );
  return paymentMiddleware(
    {
      'POST /relay/send-upto': {
        accepts: [
          {
            scheme: 'upto',
            price: pricing.ceiling,
            network: pricing.network,
            payTo: pricing.payTo,
          },
        ],
        description: 'Sponsored relay billed against an open channel (actual usage settled)',
      },
    },
    server,
  );
}

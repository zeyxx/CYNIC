/**
 * Setup-aware transaction builder (Kit-native port of the proven flow2 script).
 *
 * Pipeline: Jupiter quote/swap (user as wallet) -> decode wire tx -> decompile
 * with lookup tables -> swap fee payer to the Kora signer -> drop ATA creations
 * that already exist on-chain (else re-point funding to the signer) -> append
 * the Kora fee payment -> refresh lifetime -> recompress (LUT-preserving) ->
 * user partially signs. Kora adds the fee-payer signature server-side.
 *
 * Proven on mainnet (Oct 2026): 1 ASDF -> SOL via Pump AMM, fee matched to the
 * unit. Multi-leg ASDF->USDC routes need Phoenix seat accounts: excluded until
 * seat pre-creation setup exists.
 */
import {
  addSignersToTransactionMessage,
  address,
  assertIsInstructionWithAccounts,
  compressTransactionMessageUsingAddressLookupTables,
  createSolanaRpc,
  decompileTransactionMessageFetchingLookupTables,
  getBase64EncodedWireTransaction,
  getBase64Encoder,
  getCompiledTransactionMessageDecoder,
  getTransactionDecoder,
  partiallySignTransactionMessageWithSigners,
  setTransactionMessageFeePayer,
  setTransactionMessageLifetimeUsingBlockhash,
  type Address,
  type Instruction,
  type TransactionSigner,
} from '@solana/kit';
import { fetchAddressLookupTable } from '@solana-program/address-lookup-table';
import {
  ASSOCIATED_TOKEN_PROGRAM_ADDRESS,
  TOKEN_PROGRAM_ADDRESS,
  findAssociatedTokenPda,
  getTransferInstruction,
} from '@solana-program/token';
import { estimateFee, type KoraConfig } from '../kora/client.js';

export const ASDF_MINT = address('9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump');
export const WSOL_MINT = address('So11111111111111111111111111111111111111112');
export const USDC_MINT = address('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v');

export async function jupiterSwapPlan(
  inputMint: Address,
  outputMint: Address,
  amount: bigint,
  userWallet: Address,
  jupiterApiKey?: string,
  wrapAndUnwrapSol = true,
): Promise<{ quoteOutAmount: string; wireTransaction: string }> {
  const headers: Record<string, string> = {};
  if (jupiterApiKey) headers['x-api-key'] = jupiterApiKey;
  const quote = (await fetch(
    `https://api.jup.ag/swap/v1/quote?inputMint=${inputMint}&outputMint=${outputMint}&amount=${amount}&slippageBps=100`,
    { headers },
  ).then((r) => {
    if (!r.ok) throw new Error(`jupiter quote ${r.status}`);
    return r.json();
  })) as { outAmount: string };
  const swap = (await fetch('https://api.jup.ag/swap/v1/swap', {
    method: 'POST',
    headers: { 'content-type': 'application/json', ...headers },
    body: JSON.stringify({ quoteResponse: quote, userPublicKey: userWallet, wrapAndUnwrapSol }),
  }).then((r) => {
    if (!r.ok) throw new Error(`jupiter swap ${r.status}`);
    return r.json();
  })) as { swapTransaction: string };
  return { quoteOutAmount: quote.outAmount, wireTransaction: swap.swapTransaction };
}

/**
 * Setup-aware instruction pass: drop ATA creations for accounts that already
 * exist (the Kora fee model counts static creation outflow even for
 * idempotent no-ops); re-point funding of missing ones to the Kora signer by
 * swapping the funder address in place (role flags preserved, Kora co-signs).
 */
async function setupAwareInstructions(
  instructions: readonly Instruction[],
  userWallet: Address,
  signerWallet: Address,
  rpc: ReturnType<typeof createSolanaRpc>,
): Promise<Instruction[]> {
  const out: Instruction[] = [];
  for (const ix of instructions) {
    if (ix.programAddress === ASSOCIATED_TOKEN_PROGRAM_ADDRESS) {
      assertIsInstructionWithAccounts(ix);
      // Only the funder (accounts[0]) is re-pointed to the Kora signer.
      // Owner (accounts[2]) MUST stay the user: the ATA address is derived
      // from (owner, mint), touching it breaks seed derivation (InvalidSeeds).
      if (ix.accounts[0]?.address === userWallet) {
        const ata = ix.accounts[1].address;
        const existing = await rpc.getAccountInfo(ata).send();
        // NOTE: a missing account is the NORMAL case for first-touch users
        // (or closed ATAs, e.g. WSOL unwrapped by a previous swap).
        if (existing.value !== null) continue;
        out.push({
          ...ix,
          accounts: ix.accounts.map((meta, index) =>
            index === 0 ? { ...meta, address: signerWallet } : meta,
          ),
        });
        continue;
      }
    }
    out.push(ix);
  }
  return out;
}

export interface SponsoredTxContext {
  rpc: ReturnType<typeof createSolanaRpc>;
  decompiled: Awaited<ReturnType<typeof decompileTransactionMessageFetchingLookupTables>>;
  addressesByLookupTableAddress: Record<Address, Address[]>;
  userFeeAta: Address;
  userWallet: Address;
  userSigner: TransactionSigner;
  koraSigner: Address;
  feeToken: Address;
}

export async function prepareSponsoredTx(params: {
  rpcUrl: string;
  koraSigner: Address;
  user: { wallet: Address; signer: TransactionSigner };
  inputMint: Address;
  outputMint: Address;
  amount: bigint;
  feeToken: Address;
  jupiterApiKey?: string;
  wrapAndUnwrapSol?: boolean;
}): Promise<{ context: SponsoredTxContext; quoteOutAmount: string }> {
  const rpc = createSolanaRpc(params.rpcUrl);
  const plan = await jupiterSwapPlan(
    params.inputMint,
    params.outputMint,
    params.amount,
    params.user.wallet,
    params.jupiterApiKey,
    params.wrapAndUnwrapSol,
  );
  const wireBytes = getBase64Encoder().encode(plan.wireTransaction);
  const decoded = getTransactionDecoder().decode(wireBytes);
  const compiled = getCompiledTransactionMessageDecoder().decode(decoded.messageBytes);
  if (compiled.version !== 0) throw new Error('only v0 transactions supported');
  const decompiled = await decompileTransactionMessageFetchingLookupTables(compiled, rpc);

  const lookupAddresses = compiled.addressTableLookups?.map((l) => l.lookupTableAddress) ?? [];
  const addressesByLookupTableAddress: Record<Address, Address[]> = {};
  for (const tableAddress of lookupAddresses) {
    const table = await fetchAddressLookupTable(rpc, tableAddress);
    addressesByLookupTableAddress[tableAddress] = [...table.data.addresses];
  }

  const [userFeeAta] = await findAssociatedTokenPda({
    mint: params.feeToken,
    owner: params.user.wallet,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });
  return {
    context: {
      rpc,
      decompiled,
      addressesByLookupTableAddress,
      userFeeAta,
      userWallet: params.user.wallet,
      userSigner: params.user.signer,
      koraSigner: params.koraSigner,
      feeToken: params.feeToken,
    },
    quoteOutAmount: plan.quoteOutAmount,
  };
}

export async function buildSponsoredTx(
  ctx: SponsoredTxContext,
  feeAmount: bigint,
  paymentOwner: Address,
): Promise<string> {
  const instructions = await setupAwareInstructions(
    ctx.decompiled.instructions,
    ctx.userWallet,
    ctx.koraSigner,
    ctx.rpc,
  );
  if (feeAmount > 0n) {
    const [paymentFeeAta] = await findAssociatedTokenPda({
      mint: ctx.feeToken,
      owner: paymentOwner,
      tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });
    instructions.push(
      getTransferInstruction({
        source: ctx.userFeeAta,
        destination: paymentFeeAta,
        authority: ctx.userSigner,
        amount: feeAmount,
      }),
    );
  }
  const { value: latestBlockhash } = await ctx.rpc.getLatestBlockhash().send();
  if (ctx.decompiled.version !== 0) throw new Error('only v0 transaction messages supported');
  let message = { ...ctx.decompiled, instructions };
  message = setTransactionMessageFeePayer(ctx.koraSigner, message);
  message = setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, message);
  message = addSignersToTransactionMessage([ctx.userSigner], message);
  const compressed = compressTransactionMessageUsingAddressLookupTables(
    message,
    ctx.addressesByLookupTableAddress,
  );
  const partiallySigned = await partiallySignTransactionMessageWithSigners(compressed);
  return getBase64EncodedWireTransaction(partiallySigned);
}

/**
 * Build a Kora-ready partially-signed transaction (base64) for an ASDF sell.
 * Two passes: generous fee to learn the exact requirement, then exact fee.
 */
export async function buildSponsoredSell(params: {
  rpcUrl: string;
  kora: KoraConfig;
  koraSigner: Address;
  user: { wallet: Address; signer: TransactionSigner };
  inputMint: Address;
  outputMint: Address;
  amount: bigint;
  feeToken: Address;
  jupiterApiKey?: string;
  /**
   * False keeps the output as WSOL in the (persistent) ATA instead of
   * unwrapping to native SOL (which closes the ATA and forces a setup
   * re-creation at the next swap). Default true: proven behavior.
   */
  wrapAndUnwrapSol?: boolean;
  /**
   * Volatility buffer on the final fee leg, in basis points (default 0 =
   * exact). The quote-to-send window spans several RPC round trips; on a
   * volatile fee token the oracle price can dip below the quoted fee and
   * Kora rejects the payment. The buffer overpays Kora slightly instead.
   */
  feeBufferBps?: bigint;
}): Promise<{ base64Tx: string; quoteOutAmount: string; feeInToken: bigint }> {
  const { context, quoteOutAmount } = await prepareSponsoredTx(params);
  // Pass 1: generous fee to learn the exact requirement (payer = funded signer).
  const probe = await buildSponsoredTx(context, 30_000n, params.koraSigner);
  const estimate = await estimateFee(params.kora, probe, params.feeToken.toString());
  // Pass 2: exact fee (+ optional volatility buffer).
  const bufferedFee =
    estimate.feeInToken + (estimate.feeInToken * (params.feeBufferBps ?? 0n)) / 10_000n;
  const base64Tx = await buildSponsoredTx(
    context,
    bufferedFee,
    address(estimate.paymentAddress),
  );
  return { base64Tx, quoteOutAmount, feeInToken: estimate.feeInToken };
}

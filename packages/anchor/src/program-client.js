/**
 * @cynic/anchor - Program Client
 *
 * Client for interacting with the CYNIC Anchor program on Solana.
 * Uses raw transactions for maximum reliability.
 *
 * "Onchain is truth" - κυνικός
 *
 * @module @cynic/anchor/program-client
 */

'use strict';

import { createHash } from 'crypto';
import { CYNIC_PROGRAM, SolanaCluster } from './constants.js';

/**
 * Compute Anchor instruction discriminator
 * @param {string} name - Instruction name (e.g., "initialize", "anchor_root")
 * @returns {Buffer} 8-byte discriminator
 */
function getDiscriminator(name) {
  const preimage = `global:${name}`;
  return createHash('sha256').update(preimage).digest().slice(0, 8);
}

/**
 * CYNIC Anchor Program Client
 *
 * Provides methods to interact with the deployed Anchor program:
 * - Initialize program state
 * - Add/remove validators
 * - Anchor merkle roots
 * - Verify anchored roots
 */
export class CynicProgramClient {
  /**
   * @param {Object} config - Configuration
   * @param {string} [config.cluster] - Solana cluster URL
   * @param {Object} config.wallet - Wallet for signing
   * @param {string} [config.programId] - Program ID (defaults to deployed)
   */
  constructor(config = {}) {
    this.cluster = config.cluster || SolanaCluster.DEVNET;
    this.wallet = config.wallet;
    this.programId = config.programId || CYNIC_PROGRAM.PROGRAM_ID;

    // Lazy-loaded Solana dependencies
    this._connection = null;
    this._PublicKey = null;
    this._Keypair = null;
    this._Transaction = null;
    this._TransactionInstruction = null;
    this._SystemProgram = null;
    this._sendAndConfirmTransaction = null;
  }

  /**
   * Initialize Solana connection
   * @private
   */
  async _init() {
    if (this._connection) return;

    const solanaWeb3 = await import('@solana/web3.js');
    const {
      Connection,
      PublicKey,
      Keypair,
      Transaction,
      TransactionInstruction,
      SystemProgram,
      sendAndConfirmTransaction,
    } = solanaWeb3;

    this._connection = new Connection(this.cluster, 'confirmed');
    this._PublicKey = PublicKey;
    this._Keypair = Keypair;
    this._Transaction = Transaction;
    this._TransactionInstruction = TransactionInstruction;
    this._SystemProgram = SystemProgram;
    this._sendAndConfirmTransaction = sendAndConfirmTransaction;
  }

  /**
   * Get keypair from wallet
   * @returns {Keypair}
   * @private
   */
  _getKeypair() {
    if (!this.wallet) {
      throw new Error('Wallet required for signing');
    }

    if (this.wallet._secretKey) {
      return this._Keypair.fromSecretKey(this.wallet._secretKey);
    } else if (this.wallet._keypair) {
      return this.wallet._keypair;
    } else if (this.wallet.secretKey) {
      return this._Keypair.fromSecretKey(
        this.wallet.secretKey instanceof Uint8Array
          ? this.wallet.secretKey
          : Uint8Array.from(this.wallet.secretKey)
      );
    }

    throw new Error('Invalid wallet - no secret key found');
  }

  /**
   * Get the state PDA address
   * @returns {Promise<[PublicKey, number]>}
   */
  async getStatePda() {
    await this._init();
    return this._PublicKey.findProgramAddressSync(
      [Buffer.from(CYNIC_PROGRAM.STATE_SEED)],
      new this._PublicKey(this.programId)
    );
  }

  /**
   * Get a root entry PDA address
   * @param {Buffer|Uint8Array|string} merkleRoot - 32-byte merkle root
   * @returns {Promise<[PublicKey, number]>}
   */
  async getRootPda(merkleRoot) {
    await this._init();

    const rootBytes = typeof merkleRoot === 'string'
      ? Buffer.from(merkleRoot, 'hex')
      : Buffer.from(merkleRoot);

    return this._PublicKey.findProgramAddressSync(
      [Buffer.from(CYNIC_PROGRAM.ROOT_SEED), rootBytes],
      new this._PublicKey(this.programId)
    );
  }

  /**
   * Initialize the program state
   * @returns {Promise<{signature: string, state: string}>}
   */
  async initialize() {
    await this._init();

    const keypair = this._getKeypair();
    const [statePda] = await this.getStatePda();
    const programId = new this._PublicKey(this.programId);

    const discriminator = getDiscriminator('initialize');

    const ix = new this._TransactionInstruction({
      programId,
      keys: [
        { pubkey: statePda, isSigner: false, isWritable: true },
        { pubkey: keypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: this._SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: discriminator,
    });

    const tx = new this._Transaction().add(ix);
    const signature = await this._sendAndConfirmTransaction(
      this._connection,
      tx,
      [keypair],
      { commitment: 'confirmed' }
    );

    return { signature, state: statePda.toBase58() };
  }

  /**
   * Add a validator to the registry
   * @param {string} validatorPubkey - Validator's public key
   * @returns {Promise<{signature: string}>}
   */
  async addValidator(validatorPubkey) {
    await this._init();

    const keypair = this._getKeypair();
    const [statePda] = await this.getStatePda();
    const programId = new this._PublicKey(this.programId);
    const validator = new this._PublicKey(validatorPubkey);

    const discriminator = getDiscriminator('add_validator');
    const data = Buffer.concat([discriminator, validator.toBuffer()]);

    const ix = new this._TransactionInstruction({
      programId,
      keys: [
        { pubkey: statePda, isSigner: false, isWritable: true },
        { pubkey: keypair.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    });

    const tx = new this._Transaction().add(ix);
    const signature = await this._sendAndConfirmTransaction(
      this._connection,
      tx,
      [keypair],
      { commitment: 'confirmed' }
    );

    return { signature };
  }

  /**
   * Remove a validator from the registry
   * @param {string} validatorPubkey - Validator's public key
   * @returns {Promise<{signature: string}>}
   */
  async removeValidator(validatorPubkey) {
    await this._init();

    const keypair = this._getKeypair();
    const [statePda] = await this.getStatePda();
    const programId = new this._PublicKey(this.programId);
    const validator = new this._PublicKey(validatorPubkey);

    const discriminator = getDiscriminator('remove_validator');
    const data = Buffer.concat([discriminator, validator.toBuffer()]);

    const ix = new this._TransactionInstruction({
      programId,
      keys: [
        { pubkey: statePda, isSigner: false, isWritable: true },
        { pubkey: keypair.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    });

    const tx = new this._Transaction().add(ix);
    const signature = await this._sendAndConfirmTransaction(
      this._connection,
      tx,
      [keypair],
      { commitment: 'confirmed' }
    );

    return { signature };
  }

  /**
   * Anchor a merkle root on-chain
   * @param {string|Buffer} merkleRoot - 32-byte merkle root (hex string or Buffer)
   * @param {number} itemCount - Number of items in the root
   * @param {number} blockHeight - PoJ block height
   * @returns {Promise<{signature: string, slot: number, rootPda: string}>}
   */
  async anchorRoot(merkleRoot, itemCount, blockHeight) {
    await this._init();

    const keypair = this._getKeypair();
    const [statePda] = await this.getStatePda();
    const programId = new this._PublicKey(this.programId);

    // Convert hex string to bytes if needed
    const rootBytes = typeof merkleRoot === 'string'
      ? Buffer.from(merkleRoot, 'hex')
      : Buffer.from(merkleRoot);

    if (rootBytes.length !== 32) {
      throw new Error('Merkle root must be 32 bytes');
    }

    const [rootPda] = await this.getRootPda(rootBytes);

    // Build instruction data
    const discriminator = getDiscriminator('anchor_root');
    const data = Buffer.alloc(8 + 32 + 4 + 8);
    let offset = 0;

    discriminator.copy(data, offset);
    offset += 8;

    rootBytes.copy(data, offset);
    offset += 32;

    data.writeUInt32LE(itemCount, offset);
    offset += 4;

    data.writeBigUInt64LE(BigInt(blockHeight), offset);

    const ix = new this._TransactionInstruction({
      programId,
      keys: [
        { pubkey: statePda, isSigner: false, isWritable: true },
        { pubkey: rootPda, isSigner: false, isWritable: true },
        { pubkey: keypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: this._SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data,
    });

    const tx = new this._Transaction().add(ix);
    const signature = await this._sendAndConfirmTransaction(
      this._connection,
      tx,
      [keypair],
      { commitment: 'confirmed' }
    );

    // Get slot from transaction
    const txInfo = await this._connection.getTransaction(signature, {
      commitment: 'confirmed',
    });

    return {
      signature,
      slot: txInfo?.slot || 0,
      rootPda: rootPda.toBase58(),
    };
  }

  /**
   * Verify a merkle root exists on-chain
   * @param {string|Buffer} merkleRoot - 32-byte merkle root
   * @returns {Promise<{verified: boolean, entry?: Object, error?: string}>}
   */
  async verifyRoot(merkleRoot) {
    await this._init();

    const rootBytes = typeof merkleRoot === 'string'
      ? Buffer.from(merkleRoot, 'hex')
      : Buffer.from(merkleRoot);

    if (rootBytes.length !== 32) {
      return { verified: false, error: 'Merkle root must be 32 bytes' };
    }

    try {
      const [rootPda] = await this.getRootPda(rootBytes);

      // Fetch the root entry account
      const accountInfo = await this._connection.getAccountInfo(rootPda);

      if (!accountInfo) {
        return { verified: false, error: 'Root not found on-chain' };
      }

      // Parse account data (skip 8-byte discriminator)
      const data = accountInfo.data;
      let offset = 8;

      const storedRoot = data.slice(offset, offset + 32);
      offset += 32;

      const itemCount = data.readUInt32LE(offset);
      offset += 4;

      const blockHeight = Number(data.readBigUInt64LE(offset));
      offset += 8;

      const validator = new this._PublicKey(data.slice(offset, offset + 32));
      offset += 32;

      const timestamp = Number(data.readBigInt64LE(offset));
      offset += 8;

      const slot = Number(data.readBigUInt64LE(offset));
      offset += 8;

      const index = Number(data.readBigUInt64LE(offset));

      return {
        verified: true,
        entry: {
          merkleRoot: Buffer.from(storedRoot).toString('hex'),
          itemCount,
          blockHeight,
          validator: validator.toBase58(),
          timestamp,
          slot,
          index,
        },
      };
    } catch (error) {
      return { verified: false, error: error.message };
    }
  }

  /**
   * Get the program state
   * @returns {Promise<Object|null>}
   */
  async getState() {
    await this._init();

    const [statePda] = await this.getStatePda();

    try {
      const accountInfo = await this._connection.getAccountInfo(statePda);

      if (!accountInfo) {
        return null; // Not initialized
      }

      // Parse state (skip 8-byte discriminator)
      const data = accountInfo.data;
      let offset = 8;

      const authority = new this._PublicKey(data.slice(offset, offset + 32));
      offset += 32;

      const initializedAt = Number(data.readBigInt64LE(offset));
      offset += 8;

      const rootCount = Number(data.readBigUInt64LE(offset));
      offset += 8;

      const validatorCount = data.readUInt8(offset);
      offset += 1;

      // Read validators (21 * 32 bytes)
      const validators = [];
      for (let i = 0; i < validatorCount; i++) {
        const validator = new this._PublicKey(
          data.slice(offset + i * 32, offset + (i + 1) * 32)
        );
        validators.push(validator.toBase58());
      }
      offset += 21 * 32;

      const lastAnchorSlot = Number(data.readBigUInt64LE(offset));
      offset += 8;

      const bump = data.readUInt8(offset);

      return {
        authority: authority.toBase58(),
        initializedAt,
        rootCount,
        validatorCount,
        validators,
        lastAnchorSlot,
        bump,
      };
    } catch (error) {
      if (error.message.includes('Account does not exist')) {
        return null;
      }
      throw error;
    }
  }

  /**
   * Check if program is initialized
   * @returns {Promise<boolean>}
   */
  async isInitialized() {
    const state = await this.getState();
    return state !== null;
  }

  /**
   * Check if a public key is a validator
   * @param {string} pubkey - Public key to check
   * @returns {Promise<boolean>}
   */
  async isValidator(pubkey) {
    const state = await this.getState();
    if (!state) return false;
    return state.validators.includes(pubkey);
  }

  /**
   * Transfer authority to a new account
   * @param {string} newAuthority - New authority public key
   * @returns {Promise<{signature: string}>}
   */
  async transferAuthority(newAuthority) {
    await this._init();

    const keypair = this._getKeypair();
    const [statePda] = await this.getStatePda();
    const programId = new this._PublicKey(this.programId);
    const newAuth = new this._PublicKey(newAuthority);

    const discriminator = getDiscriminator('transfer_authority');
    const data = Buffer.concat([discriminator, newAuth.toBuffer()]);

    const ix = new this._TransactionInstruction({
      programId,
      keys: [
        { pubkey: statePda, isSigner: false, isWritable: true },
        { pubkey: keypair.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    });

    const tx = new this._Transaction().add(ix);
    const signature = await this._sendAndConfirmTransaction(
      this._connection,
      tx,
      [keypair],
      { commitment: 'confirmed' }
    );

    return { signature };
  }
}

/**
 * Create a CYNIC program client
 * @param {Object} config - Configuration
 * @returns {CynicProgramClient}
 */
export function createProgramClient(config = {}) {
  return new CynicProgramClient(config);
}

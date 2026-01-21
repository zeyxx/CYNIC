/**
 * Key Manager - Persistent Ed25519 Identity
 *
 * "Your keys are your identity" - κυνικός
 *
 * Manages Ed25519 keypairs for CYNIC nodes:
 * - Generate new keypairs
 * - Load/save from file
 * - Sign and verify data
 * - Derive node ID from public key
 *
 * @module @cynic/identity/keys
 */

'use strict';

import { generateKeyPairSync, sign, verify, createPrivateKey, createPublicKey, createHash } from 'crypto';
import fs from 'fs';
import path from 'path';

/**
 * Generate Ed25519 keypair
 * @returns {{ publicKey: string, privateKey: string }} Keypair (hex encoded DER)
 */
export function generateKeypair() {
  const { publicKey, privateKey } = generateKeyPairSync('ed25519', {
    publicKeyEncoding: { type: 'spki', format: 'der' },
    privateKeyEncoding: { type: 'pkcs8', format: 'der' },
  });

  return {
    publicKey: publicKey.toString('hex'),
    privateKey: privateKey.toString('hex'),
  };
}

/**
 * Derive node ID from public key (first 16 bytes of SHA-256)
 * @param {string} publicKeyHex - Public key in hex
 * @returns {string} Node ID (32 hex chars)
 */
export function deriveNodeId(publicKeyHex) {
  const hash = createHash('sha256').update(publicKeyHex).digest('hex');
  return hash.slice(0, 32);
}

/**
 * Key Manager
 *
 * Manages persistent Ed25519 identity for a CYNIC node.
 *
 * @example
 * ```javascript
 * const km = new KeyManager({ keyfile: './node-key.json' });
 * await km.load(); // or km.generate() if new node
 *
 * console.log(km.nodeId); // Derived from public key
 * const sig = km.sign('hello');
 * const valid = km.verify('hello', sig, km.publicKey);
 * ```
 */
export class KeyManager {
  /**
   * @param {Object} options - Configuration
   * @param {string} [options.keyfile] - Path to keyfile (JSON)
   * @param {string} [options.publicKey] - Preloaded public key (hex)
   * @param {string} [options.privateKey] - Preloaded private key (hex)
   */
  constructor(options = {}) {
    this.keyfile = options.keyfile || null;
    this._publicKey = options.publicKey || null;
    this._privateKey = options.privateKey || null;
    this._nodeId = null;

    // If keys provided, derive node ID
    if (this._publicKey) {
      this._nodeId = deriveNodeId(this._publicKey);
    }
  }

  /**
   * Public key (hex encoded)
   * @type {string|null}
   */
  get publicKey() {
    return this._publicKey;
  }

  /**
   * Node ID (derived from public key)
   * @type {string|null}
   */
  get nodeId() {
    return this._nodeId;
  }

  /**
   * Formatted public key with prefix
   * @type {string|null}
   */
  get formattedKey() {
    return this._publicKey ? `ed25519:${this._publicKey}` : null;
  }

  /**
   * Check if keys are loaded
   * @type {boolean}
   */
  get hasKeys() {
    return this._publicKey !== null && this._privateKey !== null;
  }

  /**
   * Generate new keypair
   * @param {boolean} [save=true] - Save to keyfile if configured
   * @returns {Promise<void>}
   */
  async generate(save = true) {
    const keypair = generateKeypair();
    this._publicKey = keypair.publicKey;
    this._privateKey = keypair.privateKey;
    this._nodeId = deriveNodeId(this._publicKey);

    if (save && this.keyfile) {
      await this.save();
    }
  }

  /**
   * Load keypair from keyfile
   * @returns {Promise<boolean>} True if loaded, false if file doesn't exist
   */
  async load() {
    if (!this.keyfile) {
      throw new Error('No keyfile configured');
    }

    try {
      const data = fs.readFileSync(this.keyfile, 'utf-8');
      const keypair = JSON.parse(data);

      if (!keypair.publicKey || !keypair.privateKey) {
        throw new Error('Invalid keyfile format: missing publicKey or privateKey');
      }

      this._publicKey = keypair.publicKey;
      this._privateKey = keypair.privateKey;
      this._nodeId = deriveNodeId(this._publicKey);

      return true;
    } catch (err) {
      if (err.code === 'ENOENT') {
        return false;
      }
      throw err;
    }
  }

  /**
   * Save keypair to keyfile
   * @returns {Promise<void>}
   */
  async save() {
    if (!this.keyfile) {
      throw new Error('No keyfile configured');
    }

    if (!this._publicKey || !this._privateKey) {
      throw new Error('No keypair to save');
    }

    // Ensure directory exists
    const dir = path.dirname(this.keyfile);
    if (dir && dir !== '.') {
      fs.mkdirSync(dir, { recursive: true });
    }

    const data = JSON.stringify({
      publicKey: this._publicKey,
      privateKey: this._privateKey,
      nodeId: this._nodeId,
      createdAt: new Date().toISOString(),
    }, null, 2);

    fs.writeFileSync(this.keyfile, data, { mode: 0o600 }); // Restrictive permissions
  }

  /**
   * Load or generate keypair
   * @returns {Promise<boolean>} True if loaded, false if generated
   */
  async loadOrGenerate() {
    if (this.keyfile) {
      const loaded = await this.load();
      if (loaded) {
        return true;
      }
    }

    await this.generate();
    return false;
  }

  /**
   * Sign data
   * @param {string|Buffer} data - Data to sign
   * @returns {string} Signature (hex encoded)
   */
  sign(data) {
    if (!this._privateKey) {
      throw new Error('No private key loaded');
    }

    const privateKey = createPrivateKey({
      key: Buffer.from(this._privateKey, 'hex'),
      format: 'der',
      type: 'pkcs8',
    });

    const dataBuffer = typeof data === 'string' ? Buffer.from(data) : data;
    const signature = sign(null, dataBuffer, privateKey);
    return signature.toString('hex');
  }

  /**
   * Verify signature
   * @param {string|Buffer} data - Original data
   * @param {string} signatureHex - Signature (hex encoded)
   * @param {string} [publicKeyHex] - Public key (defaults to own key)
   * @returns {boolean} True if valid
   */
  verify(data, signatureHex, publicKeyHex = this._publicKey) {
    if (!publicKeyHex) {
      throw new Error('No public key provided');
    }

    try {
      const publicKey = createPublicKey({
        key: Buffer.from(publicKeyHex, 'hex'),
        format: 'der',
        type: 'spki',
      });

      const dataBuffer = typeof data === 'string' ? Buffer.from(data) : data;
      const signatureBuffer = Buffer.from(signatureHex, 'hex');

      return verify(null, dataBuffer, publicKey, signatureBuffer);
    } catch {
      return false;
    }
  }

  /**
   * Sign an object (JSON stringified)
   * @param {Object} obj - Object to sign
   * @returns {string} Signature
   */
  signObject(obj) {
    const data = JSON.stringify(obj, Object.keys(obj).sort());
    return this.sign(data);
  }

  /**
   * Verify object signature
   * @param {Object} obj - Object to verify
   * @param {string} signatureHex - Signature
   * @param {string} [publicKeyHex] - Public key
   * @returns {boolean} True if valid
   */
  verifyObject(obj, signatureHex, publicKeyHex = this._publicKey) {
    const data = JSON.stringify(obj, Object.keys(obj).sort());
    return this.verify(data, signatureHex, publicKeyHex);
  }

  /**
   * Export public identity (safe to share)
   * @returns {Object}
   */
  exportPublic() {
    return {
      nodeId: this._nodeId,
      publicKey: this._publicKey,
      formattedKey: this.formattedKey,
    };
  }

  /**
   * Clear keys from memory
   */
  clear() {
    this._publicKey = null;
    this._privateKey = null;
    this._nodeId = null;
  }
}

/**
 * Create a KeyManager instance
 * @param {Object} [options] - Configuration
 * @returns {KeyManager}
 */
export function createKeyManager(options = {}) {
  return new KeyManager(options);
}

export default {
  KeyManager,
  createKeyManager,
  generateKeypair,
  deriveNodeId,
};

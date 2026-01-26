/**
 * TransportComponent - P2P Networking Domain
 *
 * Encapsulates WebSocket transport, peer management, and gossip protocol wiring.
 * Part of CYNICNode decomposition (Phase 2, #30).
 *
 * "Nodes talk to nodes" - κυνικός
 *
 * @module @cynic/node/components/transport-component
 */

'use strict';

import { EventEmitter } from 'events';
import { GossipProtocol, MessageType } from '@cynic/protocol';
import { createLogger } from '@cynic/core';
import { WebSocketTransport, ConnectionState } from '../transport/websocket.js';

const log = createLogger('TransportComponent');

/**
 * Transport Component - manages P2P networking
 *
 * Single Responsibility: Network transport, peer connections, message routing
 */
export class TransportComponent extends EventEmitter {
  /**
   * Create transport component
   *
   * @param {Object} options - Component options
   * @param {boolean} [options.enabled=true] - Enable P2P transport
   * @param {number} [options.port=8618] - WebSocket port (φ-aligned)
   * @param {string} [options.host='0.0.0.0'] - Listen host
   * @param {Object} [options.ssl] - SSL config for WSS
   * @param {string} options.publicKey - Node public key
   * @param {string} options.privateKey - Node private key
   * @param {string} [options.address] - Node address for gossip
   * @param {Function} [options.onMessage] - Message handler callback
   */
  constructor(options = {}) {
    super();

    this._config = {
      enabled: options.enabled ?? true,
      port: options.port || 8618,
      host: options.host || '0.0.0.0',
      ssl: options.ssl || null,
    };

    this._publicKey = options.publicKey;
    this._privateKey = options.privateKey;
    this._address = options.address || `${this._config.host}:${this._config.port}`;
    this._onMessage = options.onMessage || (async () => {});

    // Initialize transport (if enabled)
    this._transport = null;
    if (this._config.enabled) {
      this._transport = new WebSocketTransport({
        port: this._config.port,
        host: this._config.host,
        publicKey: this._publicKey,
        privateKey: this._privateKey,
        ssl: this._config.ssl,
      });
    }

    // Initialize gossip protocol
    this._gossip = new GossipProtocol({
      publicKey: this._publicKey,
      privateKey: this._privateKey,
      address: this._address,
      onMessage: this._onMessage,
      sendFn: this._transport ? this._transport.getSendFn() : (async () => {}),
    });

    // Event handlers storage (for cleanup)
    this._handlers = {};
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Lifecycle
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Start the transport server
   * @returns {Promise<void>}
   */
  async start() {
    if (!this._config.enabled || !this._transport) {
      return;
    }

    await this._transport.startServer();

    const protocol = this._config.ssl ? 'wss' : 'ws';
    log.info('P2P listening', { protocol, host: this._config.host, port: this._config.port });
  }

  /**
   * Stop the transport server
   * @returns {Promise<void>}
   */
  async stop() {
    if (!this._config.enabled || !this._transport) {
      return;
    }

    await this._transport.stopServer();
    log.info('P2P transport stopped');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Event Wiring
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Wire transport events to external handlers
   *
   * @param {Object} handlers - Event handlers
   * @param {Function} [handlers.onPeerConnected] - (peerId, publicKey, address) => void
   * @param {Function} [handlers.onPeerDisconnected] - (peerId, code, reason) => void
   * @param {Function} [handlers.onPeerIdentified] - (peerId, publicKey, address) => void
   * @param {Function} [handlers.onPeerError] - (peerId, error) => void
   * @param {Function} [handlers.onServerError] - (error) => void
   */
  wireEvents(handlers = {}) {
    if (!this._transport) return;

    this._handlers = handlers;

    // Peer connected
    this._transport.on('peer:connected', ({ peerId, publicKey, address }) => {
      log.info('Peer connected', { peerId: peerId.slice(0, 16), address });

      // Add to gossip
      this._gossip.addPeer({
        id: peerId,
        publicKey: publicKey || peerId,
        address,
      });

      // Emit for parent
      this.emit('peer:connected', { peerId, publicKey, address });
      handlers.onPeerConnected?.(peerId, publicKey, address);
    });

    // Peer disconnected
    this._transport.on('peer:disconnected', ({ peerId, code, reason }) => {
      log.info('Peer disconnected', { peerId: peerId.slice(0, 16), reason: reason || code });

      this.emit('peer:disconnected', { peerId, code, reason });
      handlers.onPeerDisconnected?.(peerId, code, reason);
    });

    // Peer identified (identity exchange complete)
    this._transport.on('peer:identified', ({ peerId, publicKey, address }) => {
      log.debug('Peer identified', { peerId: peerId.slice(0, 16), address });

      this.emit('peer:identified', { peerId, publicKey, address });
      handlers.onPeerIdentified?.(peerId, publicKey, address);
    });

    // Message received
    this._transport.on('message', ({ message, peerId }) => {
      // Route to gossip protocol
      this._gossip.handleMessage(message, peerId);
    });

    // Server listening
    this._transport.on('server:listening', ({ port, host, secure }) => {
      this.emit('server:listening', { port, host, secure });
    });

    // Errors
    this._transport.on('peer:error', ({ peerId, error }) => {
      log.warn('Peer error', { peerId: peerId?.slice(0, 16), error: error.message });
      handlers.onPeerError?.(peerId, error);
    });

    this._transport.on('server:error', (error) => {
      log.error('Server error', { error: error.message });
      handlers.onServerError?.(error);
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Peer Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Connect to a remote peer
   *
   * @param {Object} peer - Peer info
   * @param {string} peer.address - Peer address (ws://host:port or host:port)
   * @param {string} [peer.id] - Peer ID (optional, discovered via identity exchange)
   * @returns {Promise<void>}
   */
  async connectToPeer(peer) {
    if (!this._transport) {
      throw new Error('P2P transport not enabled');
    }

    if (!peer.address) {
      throw new Error('Peer address required');
    }

    await this._transport.connect({
      id: peer.id || peer.address,
      address: peer.address,
    });
  }

  /**
   * Disconnect from a peer
   * @param {string} peerId - Peer ID
   */
  disconnectPeer(peerId) {
    if (this._transport) {
      this._transport.disconnect(peerId);
    }
  }

  /**
   * Add peer to gossip (without connecting via transport)
   * Used for peers connected via external means
   * @param {Object} peerInfo - Peer info
   */
  addPeer(peerInfo) {
    this._gossip.addPeer(peerInfo);
  }

  /**
   * Get list of connected peer IDs
   * @returns {string[]} Connected peer IDs
   */
  getConnectedPeers() {
    if (!this._transport) return [];
    return this._transport.getConnectedPeers();
  }

  /**
   * Get active peers from gossip
   * @returns {Array} Active peers
   */
  getActivePeers() {
    return this._gossip.getActivePeers?.() || [];
  }

  /**
   * Check if connected to a specific peer
   * @param {string} peerId - Peer ID
   * @returns {boolean} True if connected
   */
  isConnectedToPeer(peerId) {
    if (!this._transport) return false;
    return this._transport.isConnected(peerId);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Messaging
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Broadcast a judgment to all peers
   * @param {Object} judgment - Judgment to broadcast
   * @returns {Promise<void>}
   */
  async broadcastJudgment(judgment) {
    await this._gossip.broadcastJudgment(judgment);
  }

  /**
   * Broadcast a block to all peers
   * @param {Object} block - Block to broadcast
   * @returns {Promise<void>}
   */
  async broadcastBlock(block) {
    await this._gossip.broadcastBlock(block);
  }

  /**
   * Broadcast a pattern to all peers
   * @param {Object} pattern - Pattern to broadcast
   * @returns {Promise<void>}
   */
  async broadcastPattern(pattern) {
    await this._gossip.broadcastPattern(pattern);
  }

  /**
   * Send message to specific peer
   * @param {string} peerId - Target peer ID
   * @param {Object} message - Message to send
   * @returns {Promise<void>}
   */
  async sendTo(peerId, message) {
    await this._gossip.sendTo(peerId, message);
  }

  /**
   * Get send function for external use
   * @returns {Function} Send function
   */
  getSendFn() {
    return this._transport ? this._transport.getSendFn() : (async () => {});
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Stats & Info
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {boolean} Whether transport is enabled */
  get enabled() {
    return this._config.enabled;
  }

  /** @returns {number} Port number */
  get port() {
    return this._config.port;
  }

  /** @returns {boolean} Whether using SSL */
  get isSecure() {
    return !!this._config.ssl;
  }

  /**
   * Get transport stats
   * @returns {Object|null} Stats or null if disabled
   */
  getTransportStats() {
    return this._transport?.getStats() || null;
  }

  /**
   * Get gossip stats
   * @returns {Object} Gossip stats
   */
  getGossipStats() {
    return this._gossip.getStats();
  }

  /**
   * Get combined component info
   * @returns {Object} Component info
   */
  getInfo() {
    return {
      enabled: this._config.enabled,
      port: this._config.port,
      host: this._config.host,
      secure: this.isSecure,
      stats: this.getTransportStats(),
      connectedPeers: this.getConnectedPeers().length,
      gossip: this.getGossipStats(),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Accessors for backward compatibility
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get raw gossip protocol
   * @returns {GossipProtocol} Gossip protocol
   * @deprecated Use component methods instead
   */
  get gossip() {
    return this._gossip;
  }

  /**
   * Get raw transport
   * @returns {WebSocketTransport|null} Transport
   * @deprecated Use component methods instead
   */
  get transport() {
    return this._transport;
  }
}

export { ConnectionState };
export default TransportComponent;

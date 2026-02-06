/**
 * Peer Discovery
 *
 * Simple peer discovery mechanism using:
 * 1. Seed nodes (bootstrap list)
 * 2. Peer exchange (share known peers)
 * 3. Periodic reconnection attempts
 *
 * "The pack finds each other" - κυνικός
 *
 * @module @cynic/node/transport/discovery
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, GOSSIP_FANOUT, createLogger } from '@cynic/core';

const log = createLogger('PeerDiscovery');

/**
 * Default seed nodes (placeholder - should be configured)
 */
export const DEFAULT_SEED_NODES = [
  // No default seeds - must be configured
];

/**
 * Peer discovery states
 */
export const DiscoveryState = {
  IDLE: 'IDLE',
  BOOTSTRAPPING: 'BOOTSTRAPPING',
  DISCOVERING: 'DISCOVERING',
  CONNECTED: 'CONNECTED',
};

/**
 * φ-aligned discovery intervals (Fibonacci-based)
 */
const DISCOVERY_INTERVALS = {
  BOOTSTRAP_RETRY_MS: 5000, // F(5) seconds
  PEER_EXCHANGE_MS: 13000, // F(7) seconds
  RECONNECT_MS: 21000, // F(8) seconds
  HEALTH_CHECK_MS: 34000, // F(9) seconds
};

/**
 * Peer Discovery Manager
 *
 * Handles finding and connecting to peers in the network
 */
export class PeerDiscovery extends EventEmitter {
  /**
   * @param {Object} options - Discovery options
   * @param {Object} options.transport - WebSocket transport instance
   * @param {string[]} [options.seedNodes=[]] - Bootstrap seed node addresses
   * @param {number} [options.minPeers=3] - Minimum peers to maintain
   * @param {number} [options.maxPeers=50] - Maximum peers to track
   * @param {boolean} [options.autoStart=false] - Start discovery on creation
   */
  constructor({
    transport,
    seedNodes = [],
    minPeers = 3,
    maxPeers = 50,
    autoStart = false,
  }) {
    super();
    this.transport = transport;
    this.seedNodes = [...seedNodes];
    this.minPeers = minPeers;
    this.maxPeers = maxPeers;

    /** @type {Map<string, {address: string, lastSeen: number, connectAttempts: number}>} */
    this.knownPeers = new Map();
    this.state = DiscoveryState.IDLE;
    this.isRunning = false;

    // Timers
    this._bootstrapTimer = null;
    this._exchangeTimer = null;
    this._healthTimer = null;

    // Stats
    this.stats = {
      bootstrapAttempts: 0,
      peersDiscovered: 0,
      peersConnected: 0,
      exchangesSent: 0,
      exchangesReceived: 0,
      startedAt: null,
    };

    // Wire transport events
    this._wireTransportEvents();

    if (autoStart) {
      this.start();
    }
  }

  /**
   * Wire transport events for peer discovery
   * @private
   */
  _wireTransportEvents() {
    // When a peer connects, potentially exchange known peers
    this.transport.on('peer:connected', ({ peerId, address }) => {
      this.stats.peersConnected++;

      // Add to known peers
      this._addKnownPeer(peerId, address);

      // If we have enough peers, we're connected
      if (this.transport.getConnectedPeers().length >= this.minPeers) {
        this.state = DiscoveryState.CONNECTED;
      }

      this.emit('peer:discovered', { peerId, address, source: 'connection' });
    });

    // When a peer disconnects, try to maintain minimum (with cooldown to prevent storm)
    this._lastReconnectAttempt = 0;
    this.transport.on('peer:disconnected', ({ peerId, reason }) => {
      // Don't react to duplicate connection cleanup — the other connection is still alive
      if (reason === 'duplicate_connection') return;

      const connectedCount = this.transport.getConnectedPeers().length;
      const now = Date.now();

      if (connectedCount < this.minPeers && now - this._lastReconnectAttempt > 5000) {
        this._lastReconnectAttempt = now;
        this.state = DiscoveryState.DISCOVERING;
        this._tryConnectKnownPeers();
      }
    });

    // Handle peer exchange messages
    this.transport.on('message', ({ message, peerId }) => {
      if (message.type === 'PEER_EXCHANGE') {
        this._handlePeerExchange(message.payload, peerId);
      }
    });
  }

  /**
   * Start peer discovery
   */
  start() {
    if (this.isRunning) return;

    this.isRunning = true;
    this.stats.startedAt = Date.now();
    this.state = DiscoveryState.BOOTSTRAPPING;

    // Start bootstrap process
    this._bootstrap();

    // Start periodic exchange
    this._exchangeTimer = setInterval(() => {
      this._broadcastPeerExchange();
    }, DISCOVERY_INTERVALS.PEER_EXCHANGE_MS);

    // Start health checks
    this._healthTimer = setInterval(() => {
      this._checkHealth();
    }, DISCOVERY_INTERVALS.HEALTH_CHECK_MS);

    this.emit('started');
  }

  /**
   * Stop peer discovery
   */
  stop() {
    if (!this.isRunning) return;

    this.isRunning = false;

    if (this._bootstrapTimer) {
      clearTimeout(this._bootstrapTimer);
      this._bootstrapTimer = null;
    }
    if (this._exchangeTimer) {
      clearInterval(this._exchangeTimer);
      this._exchangeTimer = null;
    }
    if (this._healthTimer) {
      clearInterval(this._healthTimer);
      this._healthTimer = null;
    }

    this.state = DiscoveryState.IDLE;
    this.emit('stopped');
  }

  /**
   * Add a seed node
   * @param {string} address - Seed node address (ws://host:port)
   */
  addSeedNode(address) {
    if (!this.seedNodes.includes(address)) {
      this.seedNodes.push(address);

      // If running, try to connect immediately
      if (this.isRunning) {
        this._connectToAddress(address);
      }
    }
  }

  /**
   * Bootstrap: Connect to seed nodes
   * @private
   */
  async _bootstrap() {
    this.stats.bootstrapAttempts++;

    if (this.seedNodes.length === 0) {
      log.info('No seed nodes configured');
      this.state = DiscoveryState.DISCOVERING;
      return;
    }

    log.info('Bootstrapping', { seedNodes: this.seedNodes.length });

    let connected = 0;
    for (const address of this.seedNodes) {
      try {
        await this._connectToAddress(address);
        connected++;
      } catch (error) {
        log.warn('Failed to connect to seed', { address, error: error.message });
      }
    }

    if (connected === 0 && this.isRunning) {
      // Retry bootstrap after delay
      this._bootstrapTimer = setTimeout(() => {
        this._bootstrap();
      }, DISCOVERY_INTERVALS.BOOTSTRAP_RETRY_MS);
    } else {
      this.state = connected >= this.minPeers
        ? DiscoveryState.CONNECTED
        : DiscoveryState.DISCOVERING;
    }
  }

  /**
   * Connect to a peer address
   * @private
   * @param {string} address
   */
  async _connectToAddress(address) {
    // Normalize address
    if (!address.startsWith('ws://') && !address.startsWith('wss://')) {
      address = `ws://${address}`;
    }

    await this.transport.connect({
      id: address, // Will be replaced by real ID after identity exchange
      address,
    });
  }

  /**
   * Add a known peer
   * @private
   */
  _addKnownPeer(peerId, address) {
    // Don't add self
    if (peerId === this.transport.publicKey) return;

    // Enforce limit
    if (this.knownPeers.size >= this.maxPeers && !this.knownPeers.has(peerId)) {
      // Remove oldest
      let oldest = null;
      let oldestTime = Infinity;
      for (const [id, info] of this.knownPeers) {
        if (info.lastSeen < oldestTime) {
          oldest = id;
          oldestTime = info.lastSeen;
        }
      }
      if (oldest) this.knownPeers.delete(oldest);
    }

    const existing = this.knownPeers.get(peerId);
    if (existing) {
      existing.lastSeen = Date.now();
      existing.address = address || existing.address;
    } else {
      this.knownPeers.set(peerId, {
        address,
        lastSeen: Date.now(),
        connectAttempts: 0,
      });
      this.stats.peersDiscovered++;
    }
  }

  /**
   * Try to connect to known peers
   * @private
   */
  async _tryConnectKnownPeers() {
    const connected = new Set(this.transport.getConnectedPeers());
    const needed = this.minPeers - connected.size;

    if (needed <= 0) return;

    // Build set of addresses we're already connected to (prevents connecting
    // to a peer we already have under a different key, e.g. pubkey vs URL)
    const connectedAddresses = new Set();
    for (const peerId of connected) {
      const conn = this.transport.connections?.get(peerId);
      if (conn?.address) connectedAddresses.add(conn.address.replace(/^wss?:\/\//, ''));
      if (conn?.outboundAddress) connectedAddresses.add(conn.outboundAddress.replace(/^wss?:\/\//, ''));
    }

    // Sort by fewest connect attempts, filter out already-connected by ID or address
    const candidates = Array.from(this.knownPeers.entries())
      .filter(([peerId, info]) => {
        if (connected.has(peerId)) return false;
        // Also check by address to prevent bilateral duplicate connections
        if (info.address) {
          const bare = info.address.replace(/^wss?:\/\//, '');
          if (connectedAddresses.has(bare)) return false;
          // Also check hasConnectionToAddress for CONNECTING/RECONNECTING states
          if (this.transport.hasConnectionToAddress?.(info.address)) return false;
        }
        return true;
      })
      .sort((a, b) => a[1].connectAttempts - b[1].connectAttempts)
      .slice(0, needed);

    for (const [peerId, info] of candidates) {
      if (!info.address) continue;

      info.connectAttempts++;
      try {
        await this._connectToAddress(info.address);
      } catch {
        // Continue trying others
      }
    }
  }

  /**
   * Broadcast peer exchange to connected peers
   * @private
   */
  _broadcastPeerExchange() {
    const connectedPeers = this.transport.getConnectedPeers();
    if (connectedPeers.length === 0) return;

    // Build list of known peers to share (exclude already connected)
    const peersToShare = [];
    for (const [peerId, info] of this.knownPeers) {
      if (!connectedPeers.includes(peerId) && info.address) {
        peersToShare.push({
          id: peerId,
          address: info.address,
          lastSeen: info.lastSeen,
        });
      }
    }

    // Also include currently connected peers
    for (const peerId of connectedPeers) {
      const connInfo = this.transport.connections.get(peerId);
      if (connInfo?.address) {
        peersToShare.push({
          id: peerId,
          address: connInfo.address,
          lastSeen: Date.now(),
        });
      }
    }

    if (peersToShare.length === 0) return;

    // Send to all connected peers
    const message = {
      type: 'PEER_EXCHANGE',
      payload: {
        peers: peersToShare.slice(0, GOSSIP_FANOUT), // Limit shared peers
        timestamp: Date.now(),
      },
    };

    for (const peerId of connectedPeers) {
      this.transport.send({ id: peerId }, message).catch(() => {});
    }

    this.stats.exchangesSent++;
  }

  /**
   * Handle incoming peer exchange
   * @private
   */
  _handlePeerExchange(payload, fromPeerId) {
    if (!payload?.peers || !Array.isArray(payload.peers)) return;

    this.stats.exchangesReceived++;

    for (const peer of payload.peers) {
      if (peer.id && peer.address) {
        this._addKnownPeer(peer.id, peer.address);

        this.emit('peer:discovered', {
          peerId: peer.id,
          address: peer.address,
          source: 'exchange',
          via: fromPeerId,
        });
      }
    }

    // Try to connect to new peers if needed (respect cooldown)
    const connectedCount = this.transport.getConnectedPeers().length;
    const now = Date.now();
    if (connectedCount < this.minPeers && now - this._lastReconnectAttempt > 5000) {
      this._lastReconnectAttempt = now;
      this._tryConnectKnownPeers();
    }
  }

  /**
   * Health check: verify connections and reconnect if needed
   * @private
   */
  _checkHealth() {
    const connectedCount = this.transport.getConnectedPeers().length;

    if (connectedCount < this.minPeers) {
      this.state = DiscoveryState.DISCOVERING;

      // Try seed nodes first
      if (connectedCount === 0 && this.seedNodes.length > 0) {
        this._bootstrap();
      } else {
        this._tryConnectKnownPeers();
      }
    } else {
      this.state = DiscoveryState.CONNECTED;
    }
  }

  /**
   * Get discovery statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      state: this.state,
      knownPeers: this.knownPeers.size,
      connectedPeers: this.transport.getConnectedPeers().length,
      seedNodes: this.seedNodes.length,
      uptime: this.stats.startedAt ? Date.now() - this.stats.startedAt : 0,
    };
  }

  /**
   * Get list of known peers
   * @returns {Array}
   */
  getKnownPeers() {
    return Array.from(this.knownPeers.entries()).map(([id, info]) => ({
      id,
      ...info,
      isConnected: this.transport.isConnected(id),
    }));
  }
}

export default { PeerDiscovery, DiscoveryState, DEFAULT_SEED_NODES };

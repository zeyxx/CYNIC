/**
 * WebSocket Transport
 *
 * Peer-to-peer WebSocket transport for CYNIC gossip protocol
 *
 * Features:
 * - Server mode: Accept incoming connections
 * - Client mode: Connect to peers
 * - Auto-reconnect with exponential backoff
 * - Heartbeat/keepalive
 * - Message queuing during disconnection
 *
 * @module @cynic/node/transport/websocket
 */

'use strict';

import { WebSocketServer, WebSocket } from 'ws';
import { EventEmitter } from 'events';
import { createServer as createHttpServer } from 'http';
import { createServer as createHttpsServer } from 'https';
import { readFileSync } from 'fs';
import { serialize, deserialize } from './serializer.js';
import { PHI, PHI_INV, secureId } from '@cynic/core';
import { createHeartbeat, signData, verifySignature } from '@cynic/protocol';

/**
 * Connection state
 */
export const ConnectionState = {
  CONNECTING: 'CONNECTING',
  CONNECTED: 'CONNECTED',
  DISCONNECTED: 'DISCONNECTED',
  RECONNECTING: 'RECONNECTING',
  CLOSED: 'CLOSED',
};

/**
 * WebSocket Transport for CYNIC gossip
 *
 * SECURITY NOTE: By default uses ws:// (unencrypted). For production deployments
 * outside a VPN/private network, enable TLS by providing ssl options:
 *
 * @example
 * // Production with TLS (wss://)
 * const transport = new WebSocketTransport({
 *   port: 8618,
 *   publicKey: keypair.publicKey,
 *   privateKey: keypair.privateKey,
 *   ssl: {
 *     key: '/path/to/privkey.pem',
 *     cert: '/path/to/fullchain.pem',
 *   },
 * });
 */
export class WebSocketTransport extends EventEmitter {
  /**
   * @param {Object} options - Transport options
   * @param {number} [options.port=8618] - Server port (φ-aligned: 8×φ×1000≈8618)
   * @param {string} [options.host='0.0.0.0'] - Server host
   * @param {string} options.publicKey - Node public key
   * @param {string} options.privateKey - Node private key
   * @param {number} [options.heartbeatInterval=61800] - Heartbeat interval (φ-aligned)
   * @param {number} [options.reconnectBaseMs=1000] - Base reconnect delay
   * @param {number} [options.reconnectMaxMs=30000] - Max reconnect delay
   * @param {number} [options.maxQueueSize=1000] - Max queued messages per peer
   * @param {number} [options.maxPeerIdRemaps=10000] - Max peer ID remappings to track
   * @param {Object} [options.ssl] - SSL/TLS config for WSS (secure WebSocket)
   * @param {string} [options.ssl.key] - Path to private key PEM file
   * @param {string} [options.ssl.cert] - Path to certificate PEM file
   * @param {string} [options.ssl.ca] - Path to CA certificate PEM file (optional)
   * @param {Function} [options.httpHandler] - HTTP request handler for non-WS requests
   */
  constructor(options = {}) {
    super();

    this.port = options.port || 8618;
    this.host = options.host || '0.0.0.0';
    this.publicKey = options.publicKey;
    this.privateKey = options.privateKey;
    this.heartbeatInterval = options.heartbeatInterval || 61800;
    this.reconnectBaseMs = options.reconnectBaseMs || 1000;
    this.reconnectMaxMs = options.reconnectMaxMs || 30000;
    this.maxQueueSize = options.maxQueueSize || 1000;
    this.maxPeerIdRemaps = options.maxPeerIdRemaps || 10000;
    this.ssl = options.ssl || null;
    this.httpHandler = options.httpHandler || null;

    // Server
    this.server = null;
    this.serverRunning = false;

    // Connections: peerId -> { ws, state, queue, reconnectAttempts, address }
    this.connections = new Map();
    this._stopped = false;
    this._reconnectTimers = new Set();

    // Address to peerId mapping
    this.addressToPeer = new Map();

    // Original peerId to real peerId mapping (after identity exchange)
    // Bounded with FIFO eviction
    this.peerIdRemap = new Map();

    // Heartbeat timer
    this.heartbeatTimer = null;

    // Stats
    this.stats = {
      messagesSent: 0,
      messagesReceived: 0,
      bytesIn: 0,
      bytesOut: 0,
      connectionAttempts: 0,
      connectionsEstablished: 0,
      connectionsFailed: 0,
    };
  }

  /**
   * Start WebSocket server
   *
   * If ssl options are provided, starts a secure WSS server.
   * Otherwise, starts a plain WS server.
   *
   * @returns {Promise<void>}
   */
  async startServer() {
    if (this.serverRunning) {
      return;
    }

    return new Promise((resolve, reject) => {
      try {
        if (this.ssl) {
          // Create HTTPS server for WSS (secure WebSocket)
          const httpsOptions = {
            key: readFileSync(this.ssl.key),
            cert: readFileSync(this.ssl.cert),
          };
          if (this.ssl.ca) {
            httpsOptions.ca = readFileSync(this.ssl.ca);
          }

          this.httpsServer = createHttpsServer(httpsOptions);
          this.server = new WebSocketServer({ server: this.httpsServer });

          this.httpsServer.listen(this.port, this.host, () => {
            this.serverRunning = true;
            this.emit('server:listening', { port: this.port, host: this.host, secure: true });
            this._startHeartbeat();
            resolve();
          });

          this.httpsServer.on('error', (err) => {
            this.emit('server:error', err);
            reject(err);
          });
        } else {
          // Create HTTP server first, then attach WS to it
          // This allows handling both HTTP and WS on the same port
          this.httpServer = createHttpServer((req, res) => {
            // Handle HTTP requests (non-WebSocket)
            if (this.httpHandler) {
              this.httpHandler(req, res);
            } else {
              // Default: return WS upgrade required
              res.writeHead(426, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({
                error: 'Upgrade Required',
                message: 'This is a WebSocket endpoint. Use ws:// to connect.',
              }));
            }
          });

          this.server = new WebSocketServer({ server: this.httpServer });

          this.httpServer.listen(this.port, this.host, () => {
            this.serverRunning = true;
            this.emit('server:listening', { port: this.port, host: this.host, secure: false });
            this._startHeartbeat();
            resolve();
          });

          this.httpServer.on('error', (err) => {
            this.emit('server:error', err);
            reject(err);
          });
        }

        this.server.on('connection', (ws, req) => {
          this._handleIncomingConnection(ws, req);
        });
      } catch (err) {
        reject(err);
      }
    });
  }

  /**
   * Stop WebSocket server
   * @returns {Promise<void>}
   */
  async stopServer() {
    this._stopped = true;
    this._stopHeartbeat();

    // Cancel all pending reconnect timers
    for (const timer of this._reconnectTimers) {
      clearTimeout(timer);
    }
    this._reconnectTimers.clear();

    // Close all connections
    for (const [peerId, conn] of this.connections) {
      this._closeConnection(peerId, 'server_shutdown');
    }

    const closePromises = [];

    if (this.server) {
      closePromises.push(
        new Promise((resolve) => {
          this.server.close(() => {
            this.server = null;
            resolve();
          });
        })
      );
    }

    // Close HTTPS server if WSS was used
    if (this.httpsServer) {
      closePromises.push(
        new Promise((resolve) => {
          this.httpsServer.close(() => {
            this.httpsServer = null;
            resolve();
          });
        })
      );
    }

    // Close HTTP server if used
    if (this.httpServer) {
      closePromises.push(
        new Promise((resolve) => {
          this.httpServer.close(() => {
            this.httpServer = null;
            resolve();
          });
        })
      );
    }

    if (closePromises.length > 0) {
      await Promise.all(closePromises);
      this.serverRunning = false;
      this.emit('server:closed');
    }
  }

  /**
   * Connect to a peer
   * @param {Object} peer - Peer info with address
   * @returns {Promise<void>}
   */
  async connect(peer) {
    const { id: peerId, address } = peer;

    if (this.connections.has(peerId)) {
      const conn = this.connections.get(peerId);
      if (conn.state === ConnectionState.CONNECTED || conn.state === ConnectionState.CONNECTING) {
        return; // Already connected or connecting
      }
    }

    // Check if we already have a connection to this address (under a different peerId)
    // Prevents duplicate connections when the same host is referenced by different IDs
    if (address && this.hasConnectionToAddress(address)) {
      return;
    }

    this.stats.connectionAttempts++;

    // Initialize connection state
    const conn = {
      ws: null,
      state: ConnectionState.CONNECTING,
      queue: [],
      reconnectAttempts: 0,
      address,
      outboundAddress: address, // Preserved across identity exchange for reconnect matching
      peerId,
      isOutbound: true,
    };
    this.connections.set(peerId, conn);
    this.addressToPeer.set(address, peerId);

    return this._connectToPeer(conn);
  }

  /**
   * Internal connect to peer
   * @private
   */
  _connectToPeer(conn) {
    if (this._stopped) return Promise.resolve();

    return new Promise((resolve, reject) => {
      const wsUrl = conn.address.startsWith('ws')
        ? conn.address
        : `ws://${conn.address}`;

      try {
        const ws = new WebSocket(wsUrl);
        conn.ws = ws;

        ws.on('open', () => {
          conn.state = ConnectionState.CONNECTED;
          conn.reconnectAttempts = 0;
          this.stats.connectionsEstablished++;

          this.emit('peer:connected', { peerId: conn.peerId, address: conn.address });

          // Send queued messages
          this._flushQueue(conn);

          // Send identity
          this._sendIdentity(conn);

          resolve();
        });

        ws.on('message', (data) => {
          this._handleMessage(data, conn.peerId);
        });

        ws.on('close', (code, reason) => {
          this._handleDisconnect(conn, code, reason?.toString());
        });

        ws.on('error', (err) => {
          this.emit('peer:error', { peerId: conn.peerId, error: err });
          if (conn.state === ConnectionState.CONNECTING) {
            this.stats.connectionsFailed++;
            reject(err);
          }
        });
      } catch (err) {
        this.stats.connectionsFailed++;
        reject(err);
      }
    });
  }

  /**
   * Handle incoming connection
   * @private
   */
  _handleIncomingConnection(ws, req) {
    const remoteAddress = req.socket.remoteAddress;
    const tempId = secureId('temp');

    const conn = {
      ws,
      state: ConnectionState.CONNECTED,
      queue: [],
      reconnectAttempts: 0,
      address: remoteAddress,
      peerId: tempId,
      isOutbound: false,
      pendingIdentity: true, // Don't emit peer:connected until identity is received
    };

    this.connections.set(tempId, conn);
    this.stats.connectionsEstablished++;

    // NOTE: peer:connected is emitted AFTER identity is received for inbound connections
    // This prevents temp IDs from leaking to the application layer

    ws.on('message', (data) => {
      // Use conn.peerId (not tempId) so we get the real ID after identity verification
      this._handleMessage(data, conn.peerId);
    });

    ws.on('close', (code, reason) => {
      this._handleDisconnect(conn, code, reason?.toString());
    });

    ws.on('error', (err) => {
      this.emit('peer:error', { peerId: tempId, error: err });
    });

    // Request identity
    this._sendIdentity(conn);
  }

  /**
   * Handle incoming message
   * @private
   */
  _handleMessage(data, peerId) {
    const dataStr = data.toString();
    this.stats.messagesReceived++;
    this.stats.bytesIn += dataStr.length;

    try {
      const message = deserialize(dataStr);

      // Handle identity message specially
      if (message.type === 'IDENTITY') {
        this._handleIdentity(message, peerId);
        return;
      }

      this.emit('message', { message, peerId });
    } catch (err) {
      this.emit('message:error', { peerId, error: err, raw: dataStr.slice(0, 100) });
    }
  }

  /**
   * Handle identity message with signature verification
   * @private
   */
  _handleIdentity(message, tempPeerId) {
    const { publicKey, address, timestamp, signature } = message.payload;

    const conn = this.connections.get(tempPeerId);
    if (!conn) return;

    // SECURITY: Verify the identity signature to prevent spoofing
    if (!signature || !timestamp) {
      this.emit('peer:identity_invalid', {
        tempPeerId,
        reason: 'missing_signature',
      });
      conn.ws?.close(1008, 'identity_verification_failed');
      this.connections.delete(tempPeerId);
      return;
    }

    // Verify timestamp is recent (within 5 minutes) to prevent replay attacks
    const age = Date.now() - timestamp;
    if (age < 0 || age > 300000) {
      this.emit('peer:identity_invalid', {
        tempPeerId,
        reason: 'timestamp_invalid',
        age,
      });
      conn.ws?.close(1008, 'identity_verification_failed');
      this.connections.delete(tempPeerId);
      return;
    }

    // Verify signature proves ownership of the public key
    const challenge = `${publicKey}:${timestamp}`;
    if (!verifySignature(challenge, signature, publicKey)) {
      this.emit('peer:identity_invalid', {
        tempPeerId,
        reason: 'signature_invalid',
      });
      conn.ws?.close(1008, 'identity_verification_failed');
      this.connections.delete(tempPeerId);
      return;
    }

    // Calculate real peer ID (hash of public key)
    // For now, use publicKey as ID - will be replaced by proper hash
    const realPeerId = publicKey;

    const wasPendingIdentity = conn.pendingIdentity;

    // Mark connection as being remapped to prevent disconnect handler race
    conn._remapping = true;

    // Update connection with real peer ID
    if (tempPeerId !== realPeerId) {
      // Check if we already have a connection to this peer
      const existing = this.connections.get(realPeerId);
      if (existing) {
        // Duplicate connection - keep one deterministically
        // Rule: always keep the connection initiated by the node with the LOWER publicKey
        // Both sides compute the same answer since iAmLower flips AND isOutbound flips
        const iAmLower = this.publicKey < publicKey;
        const keepExisting = iAmLower === existing.isOutbound;
        if (keepExisting) {
          // Close this new connection, mark as not outbound to prevent reconnect
          conn._closing = true; // Prevent disconnect handler from emitting events/reconnecting
          conn.isOutbound = false;
          conn.pendingIdentity = false;
          conn.state = ConnectionState.CLOSED;
          conn._remapping = false;
          // Delete temp entry before closing to prevent disconnect handler confusion
          this.connections.delete(tempPeerId);
          conn.ws.close(1000, 'duplicate_connection');
          return;
        } else {
          // Close existing, keep new - mark existing to prevent its disconnect handler from interfering
          existing._closing = true;
          existing.isOutbound = false;
          existing.pendingIdentity = false;
          existing.state = ConnectionState.CLOSED;
          existing.ws?.close(1000, 'duplicate_connection');
          this.connections.delete(realPeerId);
        }
      }

      // Update connection ID atomically: set new entry first, then delete old
      conn.peerId = realPeerId;
      this.connections.set(realPeerId, conn);
      this.connections.delete(tempPeerId);

      // Record the mapping so send() can find the connection by original ID
      // Enforce bounds with FIFO eviction
      if (this.peerIdRemap.size >= this.maxPeerIdRemaps) {
        const oldest = this.peerIdRemap.keys().next().value;
        if (oldest) this.peerIdRemap.delete(oldest);
      }
      this.peerIdRemap.set(tempPeerId, realPeerId);
    }

    conn._remapping = false;
    conn.pendingIdentity = false;

    if (address) {
      conn.address = address;
      this.addressToPeer.set(address, realPeerId);
    }

    // For inbound connections, emit peer:connected NOW (after identity is confirmed)
    // This ensures the application layer only sees real peer IDs, not temp IDs
    if (wasPendingIdentity) {
      this.emit('peer:connected', { peerId: realPeerId, publicKey, address: conn.address, inbound: true });
    }

    this.emit('peer:identified', { peerId: realPeerId, publicKey, address });
  }

  /**
   * Send identity to peer with cryptographic proof
   * @private
   */
  _sendIdentity(conn) {
    const timestamp = Date.now();
    const challenge = `${this.publicKey}:${timestamp}`;

    // SECURITY: Sign the identity to prove ownership of the public key
    const signature = signData(challenge, this.privateKey);

    const identity = {
      type: 'IDENTITY',
      payload: {
        publicKey: this.publicKey,
        address: `${this.host}:${this.port}`,
        timestamp,
        signature,
      },
    };

    this._sendRaw(conn, serialize(identity));
  }

  /**
   * Handle disconnection
   * @private
   */
  _handleDisconnect(conn, code, reason) {
    // Skip if connection is being remapped (identity swap in progress)
    // or if it's being closed as part of duplicate detection
    if (conn._remapping || conn._closing) {
      conn.ws = null;
      return;
    }

    const wasConnected = conn.state === ConnectionState.CONNECTED;
    const wasIntentionallyClosed = conn.state === ConnectionState.CLOSED;
    conn.state = ConnectionState.DISCONNECTED;
    conn.ws = null;

    // Only emit peer:disconnected if the application layer knew about this connection
    // (i.e., it wasn't a pending inbound connection that never completed identity exchange)
    if (!conn.pendingIdentity) {
      this.emit('peer:disconnected', {
        peerId: conn.peerId,
        code,
        reason,
        wasConnected,
      });
    }

    // Auto-reconnect for outbound connections only if:
    // 1. Server is not stopped
    // 2. It was an outbound connection (we initiated it)
    // 3. Close code wasn't 1000 (normal close)
    // 4. Connection wasn't intentionally closed (duplicate detection, shutdown, etc.)
    if (!this._stopped && conn.isOutbound && code !== 1000 && !wasIntentionallyClosed) {
      this._scheduleReconnect(conn);
    } else {
      // Clean up connection
      this.connections.delete(conn.peerId);
      if (conn.address) {
        this.addressToPeer.delete(conn.address);
      }
      // Clean up any peerIdRemap entries pointing to this peer
      for (const [tempId, realId] of this.peerIdRemap) {
        if (realId === conn.peerId) {
          this.peerIdRemap.delete(tempId);
        }
      }
    }
  }

  /**
   * Schedule reconnection
   * @private
   */
  _scheduleReconnect(conn) {
    // Don't reconnect if server is stopped
    if (this._stopped) return;

    conn.state = ConnectionState.RECONNECTING;
    conn.reconnectAttempts++;

    // Exponential backoff with φ factor
    const delay = Math.min(
      this.reconnectBaseMs * Math.pow(PHI, conn.reconnectAttempts - 1),
      this.reconnectMaxMs
    );

    this.emit('peer:reconnecting', {
      peerId: conn.peerId,
      attempt: conn.reconnectAttempts,
      delayMs: delay,
    });

    const timer = setTimeout(() => {
      this._reconnectTimers.delete(timer);
      if (!this._stopped && conn.state === ConnectionState.RECONNECTING) {
        this._connectToPeer(conn).catch(() => {
          // Will retry via disconnect handler
        });
      }
    }, delay);
    this._reconnectTimers.add(timer);
  }

  /**
   * Send message to peer
   * @param {Object} peer - Peer info (from gossip layer, has id and publicKey)
   * @param {Object} message - Message to send
   * @returns {Promise<boolean>} True if sent or queued
   */
  async send(peer, message) {
    // Use publicKey for connection lookup since that's how connections are keyed
    // after identity exchange. Fall back to id for backwards compatibility.
    let peerId = peer.publicKey || peer.id;

    // Check if this peerId was remapped after identity exchange
    const remappedId = this.peerIdRemap.get(peerId);
    if (remappedId) {
      peerId = remappedId;
    }

    let conn = this.connections.get(peerId);

    if (!conn) {
      // Not connected, try to connect first
      try {
        await this.connect({ ...peer, id: peerId });
      } catch (err) {
        this.emit('send:error', { peerId, error: err, reason: 'connect_failed' });
        return false;
      }

      // Re-fetch connection after connect attempt
      conn = this.connections.get(peerId);
      if (!conn) {
        // Connection failed to establish (e.g., duplicate detection closed it)
        this.emit('send:error', { peerId, error: new Error('Connection not established'), reason: 'no_connection' });
        return false;
      }
    }

    const data = serialize(message);

    if (conn.state === ConnectionState.CONNECTED && conn.ws?.readyState === WebSocket.OPEN) {
      return this._sendRaw(conn, data);
    } else {
      // Queue message
      if (conn.queue.length < this.maxQueueSize) {
        conn.queue.push(data);
        return true;
      }
      this.emit('send:error', { peerId, error: new Error('Queue full'), reason: 'queue_full' });
      return false;
    }
  }

  /**
   * Send raw data
   * @private
   */
  _sendRaw(conn, data) {
    try {
      conn.ws.send(data);
      this.stats.messagesSent++;
      this.stats.bytesOut += data.length;
      return true;
    } catch (err) {
      this.emit('send:error', { peerId: conn.peerId, error: err });
      return false;
    }
  }

  /**
   * Flush queued messages
   * @private
   */
  _flushQueue(conn) {
    while (conn.queue.length > 0 && conn.ws?.readyState === WebSocket.OPEN) {
      const data = conn.queue.shift();
      this._sendRaw(conn, data);
    }
  }

  /**
   * Close connection to peer
   * @param {string} peerId - Peer ID
   * @param {string} [reason] - Close reason
   */
  _closeConnection(peerId, reason = 'normal') {
    const conn = this.connections.get(peerId);
    if (!conn) return;

    conn.state = ConnectionState.CLOSED;

    if (conn.ws) {
      try {
        conn.ws.close(1000, reason);
      } catch {
        // Ignore close errors
      }
    }

    this.connections.delete(peerId);
    if (conn.address) {
      this.addressToPeer.delete(conn.address);
    }
  }

  /**
   * Disconnect from peer
   * @param {string} peerId - Peer ID
   */
  disconnect(peerId) {
    this._closeConnection(peerId, 'requested');
  }

  /**
   * Start heartbeat
   * @private
   */
  _startHeartbeat() {
    if (this.heartbeatTimer) return;

    this.heartbeatTimer = setInterval(() => {
      this._sendHeartbeats();
    }, this.heartbeatInterval);
  }

  /**
   * Stop heartbeat
   * @private
   */
  _stopHeartbeat() {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  /**
   * Send heartbeats to all connected peers
   * @private
   */
  _sendHeartbeats() {
    // SECURITY: Only include timestamp for liveness checks
    // Removed uptime and connection count to prevent fingerprinting
    const heartbeat = createHeartbeat(
      {
        timestamp: Date.now(),
      },
      this.publicKey
    );

    const data = serialize(heartbeat);

    for (const conn of this.connections.values()) {
      if (conn.state === ConnectionState.CONNECTED && conn.ws?.readyState === WebSocket.OPEN) {
        this._sendRaw(conn, data);
      }
    }
  }

  /**
   * Get send function for GossipProtocol
   * @returns {Function} Send function
   */
  getSendFn() {
    return (peer, message) => this.send(peer, message);
  }

  /**
   * Get connected peer IDs
   * @returns {string[]} Peer IDs
   */
  getConnectedPeers() {
    return Array.from(this.connections.entries())
      .filter(([_, conn]) => conn.state === ConnectionState.CONNECTED)
      .map(([peerId]) => peerId);
  }

  /**
   * Check if connected to peer
   * @param {string} peerId - Peer ID
   * @returns {boolean} True if connected
   */
  isConnected(peerId) {
    const conn = this.connections.get(peerId);
    return conn?.state === ConnectionState.CONNECTED;
  }

  /**
   * Check if we have an active outbound connection to a specific address
   * @param {string} address - WebSocket address (e.g. wss://host:port)
   * @returns {boolean} True if connected to that address
   */
  hasConnectionToAddress(address) {
    const bare = address.replace(/^wss?:\/\//, '');
    for (const conn of this.connections.values()) {
      // Consider any active connection state (not just CONNECTED)
      // CONNECTING/RECONNECTING mean we're already trying — don't create duplicates
      if (conn.state === ConnectionState.DISCONNECTED || conn.state === ConnectionState.CLOSED) continue;
      const connBare = (conn.address || '').replace(/^wss?:\/\//, '');
      const outBare = (conn.outboundAddress || '').replace(/^wss?:\/\//, '');
      if (connBare === bare || outBare === bare || conn.address === address) return true;
    }
    return false;
  }

  /**
   * Get transport statistics
   * @returns {Object} Statistics
   */
  getStats() {
    const connected = this.getConnectedPeers().length;
    const connecting = Array.from(this.connections.values()).filter(
      (c) => c.state === ConnectionState.CONNECTING || c.state === ConnectionState.RECONNECTING
    ).length;

    return {
      ...this.stats,
      serverRunning: this.serverRunning,
      port: this.port,
      secure: !!this.ssl,
      connections: {
        total: this.connections.size,
        connected,
        connecting,
      },
    };
  }
}

export default {
  ConnectionState,
  WebSocketTransport,
};

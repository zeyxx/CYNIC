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
import { serialize, deserialize } from './serializer.js';
import { PHI, PHI_INV } from '@cynic/core';
import { createHeartbeat } from '@cynic/protocol';

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

    // Server
    this.server = null;
    this.serverRunning = false;

    // Connections: peerId -> { ws, state, queue, reconnectAttempts, address }
    this.connections = new Map();

    // Address to peerId mapping
    this.addressToPeer = new Map();

    // Original peerId to real peerId mapping (after identity exchange)
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
   * @returns {Promise<void>}
   */
  async startServer() {
    if (this.serverRunning) {
      return;
    }

    return new Promise((resolve, reject) => {
      this.server = new WebSocketServer({
        port: this.port,
        host: this.host,
      });

      this.server.on('listening', () => {
        this.serverRunning = true;
        this.emit('server:listening', { port: this.port, host: this.host });
        this._startHeartbeat();
        resolve();
      });

      this.server.on('error', (err) => {
        this.emit('server:error', err);
        reject(err);
      });

      this.server.on('connection', (ws, req) => {
        this._handleIncomingConnection(ws, req);
      });
    });
  }

  /**
   * Stop WebSocket server
   * @returns {Promise<void>}
   */
  async stopServer() {
    this._stopHeartbeat();

    // Close all connections
    for (const [peerId, conn] of this.connections) {
      this._closeConnection(peerId, 'server_shutdown');
    }

    if (this.server) {
      return new Promise((resolve) => {
        this.server.close(() => {
          this.serverRunning = false;
          this.server = null;
          this.emit('server:closed');
          resolve();
        });
      });
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
      if (conn.state === ConnectionState.CONNECTED) {
        return; // Already connected
      }
    }

    this.stats.connectionAttempts++;

    // Initialize connection state
    const conn = {
      ws: null,
      state: ConnectionState.CONNECTING,
      queue: [],
      reconnectAttempts: 0,
      address,
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
    const tempId = `temp_${Date.now()}_${Math.random().toString(36).slice(2)}`;

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
      this._handleMessage(data, tempId);
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
   * Handle identity message
   * @private
   */
  _handleIdentity(message, tempPeerId) {
    const { publicKey, address } = message.payload;

    // Calculate real peer ID (hash of public key)
    // For now, use publicKey as ID - will be replaced by proper hash
    const realPeerId = publicKey;

    const conn = this.connections.get(tempPeerId);
    if (!conn) return;

    const wasPendingIdentity = conn.pendingIdentity;

    // Update connection with real peer ID
    if (tempPeerId !== realPeerId) {
      this.connections.delete(tempPeerId);

      // Record the mapping so send() can find the connection by original ID
      this.peerIdRemap.set(tempPeerId, realPeerId);

      // Check if we already have a connection to this peer
      const existing = this.connections.get(realPeerId);
      if (existing) {
        // Duplicate connection - keep one deterministically (lower publicKey wins as server)
        const keepExisting = this.publicKey < publicKey;
        if (keepExisting) {
          // Close this new connection, mark as not outbound to prevent reconnect
          conn.isOutbound = false;
          conn.pendingIdentity = false;
          conn.state = ConnectionState.CLOSED;
          conn.ws.close(1000, 'duplicate_connection');
          return;
        } else {
          // Close existing, keep new
          existing.isOutbound = false;
          existing.pendingIdentity = false;
          existing.state = ConnectionState.CLOSED;
          existing.ws?.close(1000, 'duplicate_connection');
          this.connections.delete(realPeerId);
        }
      }

      conn.peerId = realPeerId;
      this.connections.set(realPeerId, conn);
    }

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
   * Send identity to peer
   * @private
   */
  _sendIdentity(conn) {
    const identity = {
      type: 'IDENTITY',
      payload: {
        publicKey: this.publicKey,
        address: `${this.host}:${this.port}`,
      },
      timestamp: Date.now(),
    };

    this._sendRaw(conn, serialize(identity));
  }

  /**
   * Handle disconnection
   * @private
   */
  _handleDisconnect(conn, code, reason) {
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
    // 1. It was an outbound connection (we initiated it)
    // 2. Close code wasn't 1000 (normal close)
    // 3. Connection wasn't intentionally closed (duplicate detection, shutdown, etc.)
    if (conn.isOutbound && code !== 1000 && !wasIntentionallyClosed) {
      this._scheduleReconnect(conn);
    } else {
      // Clean up connection
      this.connections.delete(conn.peerId);
      if (conn.address) {
        this.addressToPeer.delete(conn.address);
      }
    }
  }

  /**
   * Schedule reconnection
   * @private
   */
  _scheduleReconnect(conn) {
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

    setTimeout(() => {
      if (conn.state === ConnectionState.RECONNECTING) {
        this._connectToPeer(conn).catch(() => {
          // Will retry via disconnect handler
        });
      }
    }, delay);
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
    const heartbeat = createHeartbeat(
      {
        uptime: process.uptime(),
        connections: this.connections.size,
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

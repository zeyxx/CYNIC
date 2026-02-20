/**
 * CYNIC Organism Service
 * Handles WebSocket connection and HTTP API communication with CYNIC organism nervous system
 */

class OrganismClient {
  constructor(baseUrl = 'http://localhost:8000') {
    this.baseUrl = baseUrl
    this.wsUrl = baseUrl.replace('http', 'ws')
    this.ws = null
    this.reconnectAttempts = 0
    this.maxReconnectAttempts = 5
    this.reconnectDelay = 1000
    this.callbacks = {
      onConnect: null,
      onDisconnect: null,
      onJudgment: null,
      onCycleChange: null,
      onError: null
    }
  }

  /**
   * Initialize kernel connection
   */
  async init(callbacks = {}) {
    this.callbacks = { ...this.callbacks, ...callbacks }

    try {
      // Test kernel availability
      const response = await fetch(`${this.baseUrl}/health`)
      if (!response.ok) {
        throw new Error('Kernel not responding')
      }

      // Connect WebSocket
      this.connectWebSocket()

      // Fetch initial state
      this.fetchConsciousness()
    } catch (error) {
      console.error('Failed to initialize kernel connection:', error)
      this.callbacks.onError?.(error)
      this.scheduleReconnect()
    }
  }

  /**
   * Connect to WebSocket stream
   */
  connectWebSocket() {
    try {
      const url = `${this.wsUrl}/ws/stream`
      console.log('Connecting to WebSocket:', url)

      this.ws = new WebSocket(url)

      this.ws.onopen = () => {
        console.log('WebSocket connected')
        this.reconnectAttempts = 0
        this.callbacks.onConnect?.()
      }

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data)
          this.handleMessage(data)
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error, event.data)
        }
      }

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error)
        this.callbacks.onError?.(error)
      }

      this.ws.onclose = () => {
        console.log('WebSocket disconnected')
        this.callbacks.onDisconnect?.()
        this.scheduleReconnect()
      }
    } catch (error) {
      console.error('Failed to create WebSocket:', error)
      this.callbacks.onError?.(error)
      this.scheduleReconnect()
    }
  }

  /**
   * Handle incoming WebSocket message
   */
  handleMessage(data) {
    if (!data.event) return

    switch (data.event) {
      case 'judgment.created':
        this.callbacks.onJudgment?.(data.data)
        break
      case 'cycle.changed':
        this.callbacks.onCycleChange?.(data.data.phase)
        break
      case 'dog.awakened':
        // Dog activation event
        console.log('Dog awakened:', data.data.dog)
        break
      case 'learning.update':
        console.log('Learning update:', data.data)
        break
      default:
        console.log('Unknown event:', data.event)
    }
  }

  /**
   * Schedule WebSocket reconnection
   */
  scheduleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('Max reconnection attempts reached')
      return
    }

    this.reconnectAttempts++
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1)
    console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`)

    setTimeout(() => {
      this.connectWebSocket()
    }, delay)
  }

  /**
   * Fetch current consciousness state
   */
  async fetchConsciousness() {
    try {
      const response = await fetch(`${this.baseUrl}/consciousness`)
      if (!response.ok) throw new Error('Failed to fetch consciousness')

      const data = await response.json()
      return data
    } catch (error) {
      console.error('Failed to fetch consciousness:', error)
      this.callbacks.onError?.(error)
      throw error
    }
  }

  /**
   * Fetch health metrics
   */
  async fetchHealth() {
    try {
      const response = await fetch(`${this.baseUrl}/health`)
      if (!response.ok) throw new Error('Failed to fetch health')

      const data = await response.json()
      return data
    } catch (error) {
      console.error('Failed to fetch health:', error)
      throw error
    }
  }

  /**
   * Fetch axioms status
   */
  async fetchAxioms() {
    try {
      const response = await fetch(`${this.baseUrl}/axioms`)
      if (!response.ok) throw new Error('Failed to fetch axioms')

      const data = await response.json()
      return data
    } catch (error) {
      console.error('Failed to fetch axioms:', error)
      throw error
    }
  }

  /**
   * Send judgment request
   */
  async judge(input) {
    try {
      const response = await fetch(`${this.baseUrl}/judge`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ input })
      })

      if (!response.ok) throw new Error('Failed to send judgment')

      const data = await response.json()
      return data
    } catch (error) {
      console.error('Failed to judge:', error)
      throw error
    }
  }

  /**
   * Send feedback
   */
  async feedback(judgmentId, rating) {
    try {
      const response = await fetch(`${this.baseUrl}/feedback`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ judgment_id: judgmentId, rating })
      })

      if (!response.ok) throw new Error('Failed to send feedback')

      const data = await response.json()
      return data
    } catch (error) {
      console.error('Failed to send feedback:', error)
      throw error
    }
  }

  /**
   * Close WebSocket connection
   */
  disconnect() {
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
  }
}

/**
 * Global organism client instance
 */
let organismClient = null

/**
 * Initialize organism connection (nervous system awakening)
 */
export function initOrganismConnection(callbacks = {}) {
  if (!organismClient) {
    organismClient = new OrganismClient()
  }

  organismClient.init(callbacks)
  return organismClient
}

/**
 * Get organism client instance
 */
export function getOrganismClient() {
  if (!organismClient) {
    organismClient = new OrganismClient()
  }
  return organismClient
}

/**
 * Disconnect organism client (nervous system sleep)
 */
export function disconnectOrganism() {
  if (organismClient) {
    organismClient.disconnect()
    organismClient = null
  }
}

export default OrganismClient

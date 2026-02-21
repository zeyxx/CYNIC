import axios from 'axios'

const API_BASE = import.meta.env.VITE_CYNIC_URL || 'http://localhost:8000'
const TIMEOUT = parseInt(import.meta.env.VITE_API_TIMEOUT || '30000', 10)

const client = axios.create({
  baseURL: API_BASE,
  timeout: TIMEOUT,
  headers: {
    'Content-Type': 'application/json',
  }
})

// Add error handling
client.interceptors.response.use(
  response => response,
  error => {
    console.error('API error:', error.message)
    return Promise.reject(error)
  }
)

export default client

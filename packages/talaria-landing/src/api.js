const DEFAULT_API_BASE = 'https://api.talaria.build'

export const API_BASE = (import.meta.env.VITE_TALARIA_API_BASE ?? DEFAULT_API_BASE).replace(/\/+$/, '')

export function apiUrl(path) {
  return `${API_BASE}${path.startsWith('/') ? path : `/${path}`}`
}

export async function fetchJson(path, { timeoutMs = 12000, ...options } = {}) {
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), timeoutMs)
  try {
    const res = await fetch(apiUrl(path), {
      ...options,
      signal: ctrl.signal,
      headers: {
        Accept: 'application/json',
        ...(options.headers ?? {}),
      },
    })
    const text = await res.text()
    const contentType = res.headers.get('content-type') ?? ''
    const data = text && contentType.includes('application/json') ? JSON.parse(text) : null
    if (!res.ok) {
      const message = data?.detail?.error ?? data?.error ?? `HTTP ${res.status}`
      throw new Error(message)
    }
    if (text && !data) throw new Error('Backend returned non-JSON response')
    return data
  } finally {
    clearTimeout(timer)
  }
}

export function asArray(value) {
  return Array.isArray(value) ? value : []
}

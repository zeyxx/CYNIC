/**
 * Global test setup for vitest
 * Mocks fetch and other browser APIs that don't work in Node.js test environment
 */

import { vi } from 'vitest';

// Mock schema that will be returned by fetch for /api/organism/schema
const MOCK_SCHEMA = {
  version: '1.0.0',
  commands: [
    {
      id: 'get-status',
      name: 'Get Status',
      description: 'Fetch organism status',
      params: {
        verbose: { type: 'boolean', required: false, description: 'Include extra detail' },
        format: { type: 'enum', required: true, description: 'Output format', enum: ['json', 'text'] },
      },
      returns: { type: 'object', description: 'Status object' },
    },
    {
      id: 'set-learn-rate',
      name: 'Set Learn Rate',
      description: 'Adjust learning rate',
      params: {
        rate: { type: 'number', required: true, description: 'Learning rate (0-1)', default: 0.618 },
      },
      returns: { type: 'object', description: 'Updated learn rate' },
    },
  ],
  skills: [],
  state: {},
};

// Mock fetch globally to handle relative URLs
global.fetch = vi.fn(async (url: string | Request, options?: RequestInit) => {
  const urlStr = typeof url === 'string' ? url : url.url;

  // Handle /api/organism/schema
  if (urlStr === '/api/organism/schema' || urlStr.endsWith('/api/organism/schema')) {
    return new Response(JSON.stringify(MOCK_SCHEMA), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    });
  }

  // Default: return 404 for unmocked endpoints
  return new Response(JSON.stringify({ error: 'Not mocked' }), {
    status: 404,
    headers: { 'Content-Type': 'application/json' },
  });
});

// Ensure localStorage is available (jsdom provides this, but verify)
if (!global.localStorage) {
  const store: Record<string, string> = {};
  global.localStorage = {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      for (const key in store) {
        delete store[key];
      }
    },
    get length() {
      return Object.keys(store).length;
    },
    key: (index: number) => {
      const keys = Object.keys(store);
      return keys[index] || null;
    },
  } as any;
}

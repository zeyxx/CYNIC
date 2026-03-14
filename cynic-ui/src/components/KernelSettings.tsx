import { useState, useEffect } from 'react';
import { DEFAULT_API_BASE } from '../types';

const LS_KEY = 'cynic_kernel_url';

export function getKernelUrl(): string {
  return localStorage.getItem(LS_KEY) ?? DEFAULT_API_BASE;
}

export function KernelSettings({ onClose }: { onClose: () => void }) {
  const [url, setUrl] = useState(getKernelUrl());
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<string | null>(null);

  const save = () => {
    const clean = url.replace(/\/$/, '');
    localStorage.setItem(LS_KEY, clean);
    window.location.reload();
  };

  const reset = () => {
    localStorage.removeItem(LS_KEY);
    setUrl(DEFAULT_API_BASE);
    setTestResult(null);
  };

  const testConnection = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const res = await fetch(`${url.replace(/\/$/, '')}/health`);
      const data = await res.json();
      setTestResult(`✓ ${data.status} — v${data.version}`);
    } catch (e) {
      setTestResult(`✗ Unreachable: ${e instanceof Error ? e.message : 'unknown error'}`);
    } finally {
      setTesting(false);
    }
  };

  useEffect(() => { setTestResult(null); }, [url]);

  return (
    <div style={{
      position: 'fixed', inset: 0, zIndex: 100,
      background: 'rgba(0,0,0,0.7)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
    }} onClick={onClose}>
      <div style={{
        background: '#111', border: '1px solid #333', borderRadius: 12,
        padding: 28, width: 520, maxWidth: '95vw',
      }} onClick={e => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 20 }}>
          <h3 style={{ margin: 0, color: '#C9A84C', fontFamily: 'monospace', fontSize: 15, letterSpacing: 2 }}>
            ⚙ KERNEL SETTINGS
          </h3>
          <button onClick={onClose} style={{ background: 'none', border: 'none', color: '#666', cursor: 'pointer', fontSize: 18 }}>
            ×
          </button>
        </div>

        <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 8 }}>
          CYNIC Kernel URL (REST API base)
        </label>
        <input
          value={url}
          onChange={e => setUrl(e.target.value)}
          style={{
            width: '100%', padding: '10px 12px',
            background: '#0a0a0a', border: '1px solid #333',
            borderRadius: 8, color: '#e0e0e0',
            fontFamily: 'monospace', fontSize: 13,
            outline: 'none', boxSizing: 'border-box',
          }}
          placeholder="https://... or http://localhost:3030"
        />
        <div style={{ fontSize: 11, color: '#444', marginTop: 6 }}>
          Cloudflare tunnel, Tailscale IP, ou localhost:3030 si le kernel tourne localement.
        </div>

        {testResult && (
          <div style={{
            marginTop: 12, padding: '8px 12px',
            background: testResult.startsWith('✓') ? '#4CAF5011' : '#F4433611',
            border: `1px solid ${testResult.startsWith('✓') ? '#4CAF50' : '#F44336'}`,
            borderRadius: 6, color: testResult.startsWith('✓') ? '#4CAF50' : '#F44336',
            fontFamily: 'monospace', fontSize: 12,
          }}>
            {testResult}
          </div>
        )}

        <div style={{ display: 'flex', gap: 8, marginTop: 20 }}>
          <button onClick={testConnection} disabled={testing} style={{
            flex: 1, padding: '10px', background: '#1a1a1a',
            border: '1px solid #444', borderRadius: 8,
            color: '#aaa', cursor: testing ? 'not-allowed' : 'pointer',
            fontFamily: 'monospace', fontSize: 12,
          }}>
            {testing ? 'Testing...' : 'Test Connection'}
          </button>
          <button onClick={reset} style={{
            padding: '10px 16px', background: '#1a1a1a',
            border: '1px solid #333', borderRadius: 8,
            color: '#666', cursor: 'pointer',
            fontFamily: 'monospace', fontSize: 12,
          }}>
            Reset
          </button>
          <button onClick={save} style={{
            flex: 1, padding: '10px', background: '#C9A84C22',
            border: '1px solid #C9A84C', borderRadius: 8,
            color: '#C9A84C', cursor: 'pointer',
            fontFamily: 'monospace', fontSize: 12, fontWeight: 700,
          }}>
            Save & Reload
          </button>
        </div>
      </div>
    </div>
  );
}

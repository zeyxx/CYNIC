import { useState, useEffect } from 'react';
import { DEFAULT_API_BASE } from '../types';
import { getAvailableDogs } from '../api';
import { getKernelUrl, KERNEL_URL_LS_KEY, SELECTED_DOGS_LS_KEY, getSelectedDogs } from '../utils';

export function KernelSettings({ onClose }: { onClose: () => void }) {
  const [url, setUrl] = useState(getKernelUrl());
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<string | null>(null);
  const [availableDogs, setAvailableDogs] = useState<string[]>([]);
  const [selectedDogs, setSelectedDogs] = useState<string[]>(getSelectedDogs() ?? []);

  useEffect(() => {
    getAvailableDogs().then(setAvailableDogs).catch(console.error);
  }, []);

  const save = () => {
    const clean = url.replace(/\/$/, '');
    localStorage.setItem(KERNEL_URL_LS_KEY, clean);
    localStorage.setItem(SELECTED_DOGS_LS_KEY, JSON.stringify(selectedDogs));
    window.location.reload();
  };

  const toggleDog = (id: string) => {
    setSelectedDogs(prev => {
      const effective = prev.length === 0 ? [...availableDogs] : prev;
      return effective.includes(id)
        ? effective.filter(d => d !== id)
        : [...effective, id];
    });
  };

  const reset = () => {
    localStorage.removeItem(KERNEL_URL_LS_KEY);
    localStorage.removeItem(SELECTED_DOGS_LS_KEY);
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
    <div className="settings-modal" onClick={onClose}>
      <div className="settings-dialog" onClick={e => e.stopPropagation()}>
        <div className="settings-head">
          <div>
            <div className="section-label">KERNEL SETTINGS</div>
            <div className="section-title">Connection and consensus</div>
          </div>
          <button onClick={onClose} className="button button-ghost">CLOSE</button>
        </div>

        <div className="settings-grid">
          <div className="surface-card" style={{ minHeight: 0 }}>
            <div className="section-label">KERNEL URL</div>
            <input
              value={url}
              onChange={e => setUrl(e.target.value)}
              className="form-input"
              placeholder="https://... or http://localhost:3030"
            />
            <div className="helper-text">Cloudflare tunnel, Tailscale IP, or localhost:3030 if the kernel runs locally.</div>
            <div className="helper-text">Protected endpoints require a wallet session.</div>
          </div>

          <div className="surface-card" style={{ minHeight: 0 }}>
            <div className="section-label">ACTIVE DOGS</div>
            <div className="checkbox-grid">
              {availableDogs.length === 0 && <div className="empty-state">Loading dogs...</div>}
              {availableDogs.map(id => (
                <label key={id} className="checkbox-card">
                  <input
                    type="checkbox"
                    checked={selectedDogs.length === 0 || selectedDogs.includes(id)}
                    onChange={() => toggleDog(id)}
                  />
                  <span>{id}</span>
                </label>
              ))}
            </div>
            <div className="helper-text">If none are selected, all dogs are used by default.</div>
          </div>
        </div>

        {testResult ? (
          <div className={`surface-card ${testResult.startsWith('✓') ? '' : ''}`} style={{ marginTop: 12, minHeight: 0 }}>
            <div className={`status-chip ${testResult.startsWith('✓') ? 'is-success' : 'is-danger'}`}>{testResult}</div>
          </div>
        ) : null}

        <div className="settings-actions">
          <button onClick={testConnection} disabled={testing} className="button button-ghost">
            {testing ? 'TESTING...' : 'TEST CONNECTION'}
          </button>
          <button onClick={reset} className="button button-ghost">
            RESET
          </button>
          <button onClick={save} className="button button-primary">
            SAVE & RELOAD
          </button>
        </div>
      </div>
    </div>
  );
}
